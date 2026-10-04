//! WASI Preview 2 plugin adapter.
//!
//! Enables plugins built as WASI components (vs the existing core WASM ABI).
//! WASI components can use standard I/O, CLI args, and HTTP outcalls through
//! the component model's interface-based linking.
//!
//! # Architecture
//!
//! WASI components are loaded via wasmtime's component model API:
//! - [`wasmtime::component::Component`] replaces [`wasmtime::Module`]
//! - [`wasmtime::component::Linker`] links WASI interfaces
//! - The guest exports a `crawlkit:plugin/analyze` function that the host
//!   calls with the page HTML and URL, receiving findings as JSON. (The
//!   legacy `crawlkit:plugin/analyze#analyze` name is still accepted, but
//!   `#` is not a valid component export character under current validation,
//!   so new plugins must use the interface-style name.)
//!
//! WASI host implementations come from the official `wasmtime-wasi` and
//! `wasmtime-wasi-http` crates (v47, matching the workspace `wasmtime`
//! version):
//! - **Gate 4 (CLI)**: `wasi:cli` is linked via
//!   [`wasmtime_wasi::p2::add_to_linker_sync`] with stdout/stderr redirected
//!   into bounded in-memory pipes captured per `analyze` call. No arguments,
//!   environment variables, preopened directories, or stdin are provided.
//! - **Gate 5 (HTTP)**: `wasi:http/outgoing-handler` is linked via
//!   [`wasmtime_wasi_http::p2::add_only_http_to_linker_sync`], routed through
//!   [`PluginHttpHooks`] which enforces the same fetch policy as the core ABI:
//!   deny-by-default capability checks, SSRF validation, 10 s timeouts, a
//!   1 MiB response cap, and no redirect following.
//!
//! # Security
//!
//! WASI components run under the same fuel/epoch/resource limits as core
//! WASM plugins. The sandbox grants nothing implicitly:
//! - filesystem: no preopens (all path access fails)
//! - sockets: `wasi:sockets` allows no addresses by default
//! - environment/arguments: empty
//! - stdout/stderr: bounded in-memory capture (1 MiB per stream), never the
//!   host's own stdio
//! - HTTP: denied unless the manifest declares `permissions.network` AND the
//!   embedder sets [`WasmConfig::allow_plugin_network`]; SSRF-validated on
//!   every request
//!
//! Interfaces that are linked but not implemented trap via
//! `define_unknown_imports_as_traps` (fail-closed).
//!
//! # Gate Coverage
//!
//! - **Gate 3**: WASI Preview 2 component model support — IMPLEMENTED
//! - **Gate 4**: WASI CLI (stdout/stderr capture) — IMPLEMENTED
//! - **Gate 5**: WASI HTTP outcalls — IMPLEMENTED

use std::future::Future;
use std::path::Path;
use std::task::Poll;
use std::time::Duration;

use bytes::Bytes;
use http_body_util::combinators::UnsyncBoxBody;
use wasmtime_wasi::p2::pipe::MemoryOutputPipe;
use wasmtime_wasi_http::p2::bindings::http::types::ErrorCode;
use wasmtime_wasi_http::{
    Error, RequestOptions, WasiBody, WasiHttpCtxView, WasiHttpHooks, WasiHttpView,
};

use crate::plugin::manifest::PluginMetadata;
use crate::plugin::PluginError;

use super::is_public_http_url;
use super::sandbox::WasmConfig;
use super::EpochWatchdog;

/// Per-stream capacity for captured guest stdout/stderr. A guest that writes
/// beyond this traps instead of growing host memory.
const OUTPUT_CAPTURE_CAPACITY: usize = 1024 * 1024;

/// Maximum time for each phase of a WASI HTTP outcall. Matches the core
/// ABI's `crawlkit_host.fetch` 10-second policy; guests may request larger
/// request options but the host clamps them.
const MAX_OUTCALL_TIMEOUT: Duration = Duration::from_secs(10);

/// Maximum number of response body bytes a WASI HTTP outcall may stream back
/// to the guest. Matches the core ABI's 1 MiB fetch cap.
const MAX_OUTCALL_BODY_BYTES: usize = 1024 * 1024;

/// Export the guest must provide: `analyze: func(html: string, url: string)
/// -> result<string, string>`.
const ANALYZE_EXPORT: &str = "crawlkit:plugin/analyze";

/// Pre-validation spelling of [`ANALYZE_EXPORT`] (`#` is not a valid
/// component export character), kept as a lookup fallback.
const LEGACY_ANALYZE_EXPORT: &str = "crawlkit:plugin/analyze#analyze";

/// WASI Preview 2 plugin instance.
///
/// Loads and executes a WASI component that exports the crawlkit plugin
/// interface. The component must export:
///
/// ```wit
/// package crawlkit:plugin;
///
/// interface analyze {
///     analyze: func(html: string, url: string) -> result<string, string>;
/// }
/// ```
///
/// where the result string is JSON-encoded findings (same schema as the
/// core ABI's `crawlkit_plugin_analyze` return).
pub struct WasiPlugin {
    pub manifest: PluginMetadata,
    config: WasmConfig,
    engine: wasmtime::Engine,
    /// The compiled WASI component.
    component: wasmtime::component::Component,
    /// stdout/stderr captured from the most recent [`WasiPlugin::analyze`]
    /// call (lossily decoded from UTF-8).
    captured_output: (String, String),
}

/// Per-instance host state for WASI plugins.
///
/// Holds the component-model resource table plus the WASI CLI and WASI HTTP
/// contexts linked into each instantiation. This is separate from
/// [`super::HostState`] because the component model uses typed state via
/// [`wasmtime::Store<T>`] rather than raw memory access.
pub(crate) struct WasiPluginState {
    /// Component-model resources (streams, HTTP requests, ...).
    pub(crate) table: wasmtime::component::ResourceTable,
    /// WASI CLI / clocks / random / filesystem / sockets context.
    pub(crate) wasi: wasmtime_wasi::WasiCtx,
    /// WASI HTTP context (header size limits, ...).
    pub(crate) http: wasmtime_wasi_http::WasiHttpCtx,
    /// Hooks enforcing the network capability and SSRF policy for every
    /// outgoing HTTP request.
    pub(crate) hooks: PluginHttpHooks,
    /// JSON blob of the analysis context available to the guest via
    /// `get-context`. Set before each `analyze` call.
    pub(crate) context_json: Option<String>,
    /// Bounded capture sink for guest stdout.
    pub(crate) stdout_pipe: MemoryOutputPipe,
    /// Bounded capture sink for guest stderr.
    pub(crate) stderr_pipe: MemoryOutputPipe,
}

impl WasiPluginState {
    /// Build per-call state with captured stdio and the given network
    /// capability.
    pub(crate) fn new(context_json: Option<String>, allow_network: bool) -> Self {
        let stdout_pipe = MemoryOutputPipe::new(OUTPUT_CAPTURE_CAPACITY);
        let stderr_pipe = MemoryOutputPipe::new(OUTPUT_CAPTURE_CAPACITY);
        let wasi = wasmtime_wasi::WasiCtxBuilder::new()
            .stdout(stdout_pipe.clone())
            .stderr(stderr_pipe.clone())
            .build();
        Self {
            table: wasmtime::component::ResourceTable::new(),
            wasi,
            http: wasmtime_wasi_http::WasiHttpCtx::new(),
            hooks: PluginHttpHooks { allow_network },
            context_json,
            stdout_pipe,
            stderr_pipe,
        }
    }
}

impl wasmtime_wasi::WasiView for WasiPluginState {
    fn ctx(&mut self) -> wasmtime_wasi::WasiCtxView<'_> {
        wasmtime_wasi::WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

impl WasiHttpView for WasiPluginState {
    fn http(&mut self) -> WasiHttpCtxView<'_> {
        WasiHttpCtxView {
            ctx: &mut self.http,
            table: &mut self.table,
            hooks: &mut self.hooks,
        }
    }
}

/// Boxed future returned by [`WasiHttpHooks::send_request`]: the response
/// plus the guest-visible request-processing error future.
type SendRequestFuture = Box<
    dyn Future<
            Output = Result<
                (
                    http::Response<WasiBody>,
                    Box<dyn Future<Output = Result<(), Error>> + Send>,
                ),
                Error,
            >,
        > + Send,
>;

/// Host hooks for `wasi:http/outgoing-handler`.
///
/// Every guest HTTP request flows through [`WasiHttpHooks::send_request`],
/// which is the single enforcement point for:
///
/// 1. capability — requests are denied outright unless network was granted
///    (manifest `permissions.network` AND `WasmConfig::allow_plugin_network`);
/// 2. SSRF — the resolved scheme/authority must pass the host's
///    [`is_public_http_url`] check (no loopback/private/link-local/metadata
///    targets);
/// 3. timeouts — connect/first-byte/between-bytes timeouts are clamped to
///    10 s regardless of what the guest requested;
/// 4. response size — the incoming body is wrapped so streaming beyond
///    1 MiB errors instead of buffering in the guest;
/// 5. redirects — the default handler never follows redirects.
pub(crate) struct PluginHttpHooks {
    pub(crate) allow_network: bool,
}

impl PluginHttpHooks {
    /// Validate the target of an outgoing request against the host's SSRF
    /// policy. Split from [`WasiHttpHooks::send_request`] so tests can
    /// exercise the policy without touching the network.
    fn check_target(uri: &http::Uri) -> Result<(), ErrorCode> {
        let Some(host) = uri.host() else {
            return Err(ErrorCode::HttpRequestUriInvalid);
        };
        let url = uri.to_string();
        if !is_public_http_url(&url) {
            tracing::debug!("WASI HTTP outcall blocked by SSRF guard: host={host} uri={url}");
            return Err(ErrorCode::DestinationIpProhibited);
        }
        Ok(())
    }
}

impl WasiHttpHooks for PluginHttpHooks {
    fn send_request(
        &mut self,
        request: http::Request<WasiBody>,
        options: Option<RequestOptions>,
        fut: Box<dyn Future<Output = Result<(), Error>> + Send>,
    ) -> SendRequestFuture {
        if !self.allow_network {
            tracing::debug!("WASI HTTP outcall denied: network capability not granted");
            return Box::new(async move {
                drop((request, options, fut));
                Err(Error::HttpRequestDenied)
            });
        }
        if let Err(code) = PluginHttpHooks::check_target(request.uri()) {
            return Box::new(async move {
                drop((request, options, fut));
                Err(error_code_to_error(code))
            });
        }
        let options = clamp_outcall_options(options);

        Box::new(async move {
            use http_body_util::BodyExt as _;
            let (response, io) = wasmtime_wasi_http::default_send_request(request, options).await?;
            let mut response = response.map(|b| b.boxed_unsync());
            cap_response_body(&mut response, MAX_OUTCALL_BODY_BYTES);
            Ok((
                response,
                Box::new(io) as Box<dyn Future<Output = Result<(), Error>> + Send>,
            ))
        })
    }
}

/// Map the two rejection codes [`PluginHttpHooks::check_target`] produces
/// onto the crate-root [`Error`] the 48-series hook API expects.
fn error_code_to_error(code: ErrorCode) -> Error {
    match code {
        ErrorCode::HttpRequestUriInvalid => Error::HttpRequestUriInvalid,
        ErrorCode::DestinationIpProhibited => Error::DestinationIpProhibited,
        // check_target only produces the two codes above; keep the mapping
        // total for safety rather than panicking inside a hook.
        other => Error::InternalError(Some(format!("WASI HTTP outcall rejected: {other:?}"))),
    }
}

/// Clamp every outcall timeout phase to [`MAX_OUTCALL_TIMEOUT`].
fn clamp_outcall_options(mut options: Option<RequestOptions>) -> Option<RequestOptions> {
    let clamp = |phase: &mut Option<Duration>| {
        if phase.is_some_and(|d| d > MAX_OUTCALL_TIMEOUT) {
            *phase = Some(MAX_OUTCALL_TIMEOUT);
        }
    };
    if let Some(opts) = &mut options {
        clamp(&mut opts.connect_timeout);
        clamp(&mut opts.first_byte_timeout);
        clamp(&mut opts.between_bytes_timeout);
    }
    options
}
/// Replace a response body with a size-limited wrapper so a guest cannot
/// stream an unbounded amount of data through an outcall.
fn cap_response_body(response: &mut http::Response<WasiBody>, max_bytes: usize) {
    use http_body_util::BodyExt as _;
    let empty = http_body_util::Empty::<Bytes>::new()
        .map_err(|_: std::convert::Infallible| Error::InternalError(None));
    let empty = UnsyncBoxBody::new(empty);
    let body = std::mem::replace(response.body_mut(), empty);
    let limited = UnsyncBoxBody::new(BodySizeLimit {
        inner: body,
        remaining: max_bytes,
    });

    *response.body_mut() = limited;
}

/// [`http_body::Body`] wrapper rejecting bodies that exceed a byte budget.
struct BodySizeLimit {
    inner: WasiBody,
    remaining: usize,
}

impl http_body::Body for BodySizeLimit {
    type Data = Bytes;
    type Error = Error;

    fn poll_frame(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> {
        let this = self.get_mut();
        let inner = std::pin::Pin::new(&mut this.inner);
        match std::task::ready!(inner.poll_frame(cx)) {
            Some(Ok(frame)) => {
                if let Some(data) = frame.data_ref() {
                    if data.len() > this.remaining {
                        return Poll::Ready(Some(Err(Error::InternalError(Some(
                            "WASI HTTP response body exceeds the host's 1 MiB cap".to_string(),
                        )))));
                    }
                    this.remaining -= data.len();
                }
                Poll::Ready(Some(Ok(frame)))
            }
            other => Poll::Ready(other),
        }
    }
}

impl WasiPlugin {
    /// Load a WASI component from a plugin directory.
    ///
    /// The directory must contain `crawlkit-plugin.toml` with
    /// `kind = "wasi-component"` and a `.wasm` entry point that is a
    /// valid WASI Preview 2 component (not a core WASM module).
    pub fn load(plugin_dir: &Path, config: &WasmConfig) -> Result<Self, PluginError> {
        use super::manifest::read_plugin_manifest;

        let manifest = read_plugin_manifest(plugin_dir)?;

        // Validate the plugin kind — this adapter only handles WASI components.
        let kind = manifest.plugin.kind.as_deref().unwrap_or("wasm");
        if kind != "wasi-component" {
            return Err(PluginError::LoadFailed(format!(
                "expected kind 'wasi-component', got '{kind}'"
            )));
        }

        if !manifest.plugin.api_version.starts_with("1.") {
            return Err(PluginError::IncompatibleApiVersion(
                manifest.plugin.api_version,
            ));
        }

        // Capability enforcement — same rules as core ABI plugins.
        if let Some(perms) = &manifest.plugin.permissions {
            let network_requested = perms.network.unwrap_or(false);
            let filesystem_requested = perms.filesystem.unwrap_or(false);
            let env_vars_requested = perms.env_vars.as_ref().is_some_and(|v| !v.is_empty());
            if filesystem_requested || env_vars_requested {
                return Err(PluginError::InvalidManifest(
                    "plugin requests filesystem/env_vars capabilities \
                     that the sandbox cannot grant; only network is grantable"
                        .to_string(),
                ));
            }
            if network_requested && !config.allow_plugin_network {
                return Err(PluginError::InvalidManifest(
                    "plugin requests network capability but allow_plugin_network \
                     is false; set WasmConfig.allow_plugin_network = true to grant \
                     HTTP access (SSRF-validated, no redirects, 1 MiB cap, 10s timeout)"
                        .to_string(),
                ));
            }
        }

        super::manifest::validate_manifest(&manifest.plugin)
            .map_err(|e| PluginError::InvalidManifest(e.to_string()))?;

        let wasm_file =
            manifest.plugin.entry.wasm.as_ref().ok_or_else(|| {
                PluginError::LoadFailed("No WASM entry point specified".to_string())
            })?;
        let wasm_path = plugin_dir.join(wasm_file);

        // Trust chain verification before handing bytes to the compiler.
        let wasm_bytes = std::fs::read(&wasm_path)
            .map_err(|e| PluginError::LoadFailed(format!("Failed to read WASM file: {e}")))?;
        super::crypto::verify_plugin_trust(
            &manifest.plugin.name,
            manifest.plugin.wasm_hash.as_deref(),
            manifest.plugin.signature.as_deref(),
            manifest.plugin.signed_by.as_deref(),
            &wasm_bytes,
            &config.plugin_verification,
        )?;

        // Configure wasmtime engine with component-model support, fuel,
        // and epoch interruption — same resource limits as core ABI.
        let mut engine_config = wasmtime::Config::new();
        engine_config.consume_fuel(true);
        engine_config.epoch_interruption(true);
        let engine = wasmtime::Engine::new(&engine_config)
            .map_err(|e| PluginError::LoadFailed(format!("Failed to create engine: {e}")))?;

        // Load as a component (not a core module).
        let component =
            wasmtime::component::Component::from_file(&engine, &wasm_path).map_err(|e| {
                PluginError::LoadFailed(format!("Failed to compile WASI component: {e}"))
            })?;

        Ok(Self {
            manifest: manifest.plugin,
            config: config.clone(),
            engine,
            component,
            captured_output: (String::new(), String::new()),
        })
    }

    /// Analyze HTML content using the WASI component.
    ///
    /// Instantiates the component with a fully-linked WASI store, calls
    /// its `analyze` export, and returns the JSON findings payload.
    /// Enforces the wall-clock timeout via epoch interruption.
    ///
    /// Guest stdout/stderr written through `wasi:cli` are captured into
    /// bounded buffers and can be retrieved afterwards with
    /// [`WasiPlugin::take_captured_output`].
    pub fn analyze(
        &mut self,
        html: &str,
        url: &str,
        context_json: Option<&str>,
    ) -> Result<String, PluginError> {
        // Build the WASI linker fresh per call (state changes per invocation).
        let mut linker = wasmtime::component::Linker::<WasiPluginState>::new(&self.engine);

        // Gate 4: link wasi:cli (stdout/stderr/args/env), wasi:clocks,
        // wasi:random, wasi:filesystem (no preopens) and wasi:sockets
        // (all addresses denied). stdout/stderr are redirected into the
        // state's bounded memory pipes.
        wasmtime_wasi::p2::add_to_linker_sync(&mut linker)
            .map_err(|e| PluginError::LoadFailed(format!("Failed to link WASI CLI: {e}")))?;

        // Gate 5: link wasi:http/outgoing-handler (routed through
        // PluginHttpHooks for capability + SSRF enforcement).
        wasmtime_wasi_http::p2::add_only_http_to_linker_sync(&mut linker)
            .map_err(|e| PluginError::LoadFailed(format!("Failed to link WASI HTTP: {e}")))?;

        // Link the crawlkit-specific plugin interface.
        link_crawlkit_plugin(&mut linker)?;

        // Anything still unimplemented traps when called rather than
        // failing instantiation (fail-closed; must run after all real
        // definitions so it only covers the remainder).
        linker
            .define_unknown_imports_as_traps(&self.component)
            .map_err(|e| {
                PluginError::LoadFailed(format!("Failed to define unknown imports: {e}"))
            })?;

        let network_granted = self
            .manifest
            .permissions
            .as_ref()
            .is_some_and(|p| p.network.unwrap_or(false))
            && self.config.allow_plugin_network;

        let mut store = wasmtime::Store::new(
            &self.engine,
            WasiPluginState::new(context_json.map(str::to_string), network_granted),
        );

        store
            .set_fuel(self.config.max_fuel)
            .map_err(|e| PluginError::WasmExecution(format!("Failed to set fuel: {e}")))?;

        // Arm the wall-clock deadline and start the watchdog.
        store.set_epoch_deadline(1);
        let watchdog =
            EpochWatchdog::spawn(self.engine.clone(), self.config.max_analysis_timeout_ms);

        // Instantiate the component.
        let instance = linker
            .instantiate(&mut store, &self.component)
            .map_err(|e| {
                PluginError::AnalysisFailed(format!("Component instantiation failed: {e}"))
            })?;

        // Call the crawlkit:plugin/analyze export. `#` is not a valid
        // component export character, so the interface-style name is the
        // contract; the legacy `#analyze` spelling is still accepted.
        let analyze_func = instance
            .get_func(&mut store, ANALYZE_EXPORT)
            .or_else(|| instance.get_func(&mut store, LEGACY_ANALYZE_EXPORT))
            .ok_or_else(|| {
                PluginError::AnalysisFailed(format!("Component does not export {ANALYZE_EXPORT}"))
            })?;

        let mut results = [wasmtime::component::Val::Bool(false)];
        let result = analyze_func.call(
            &mut store,
            &[
                wasmtime::component::Val::String(html.to_string()),
                wasmtime::component::Val::String(url.to_string()),
            ],
            &mut results,
        );

        watchdog.cancel();

        // Gate 4: capture whatever the guest printed, success or failure.
        let stdout = store.data().stdout_pipe.contents();
        let stderr = store.data().stderr_pipe.contents();
        if !stdout.is_empty() {
            tracing::debug!(
                plugin = %self.manifest.name,
                stdout = %String::from_utf8_lossy(&stdout),
                "captured WASI plugin stdout"
            );
        }
        if !stderr.is_empty() {
            tracing::debug!(
                plugin = %self.manifest.name,
                stderr = %String::from_utf8_lossy(&stderr),
                "captured WASI plugin stderr"
            );
        }
        self.captured_output = (
            String::from_utf8_lossy(&stdout).into_owned(),
            String::from_utf8_lossy(&stderr).into_owned(),
        );

        result.map_err(|e| {
            let msg = e.to_string();
            let is_timeout = msg.contains("epoch") || msg.contains("interrupt");
            let is_fuel = msg.contains("all fuel consumed");
            if is_timeout {
                PluginError::AnalysisFailed(format!(
                    "WASI plugin exceeded the {}ms analysis timeout",
                    self.config.max_analysis_timeout_ms
                ))
            } else if is_fuel {
                PluginError::AnalysisFailed(format!(
                    "WASI plugin exhausted its {} instruction fuel budget",
                    self.config.max_fuel
                ))
            } else {
                PluginError::AnalysisFailed(format!("WASI analyze failed: {e}"))
            }
        })?;

        // Extract the string result from the component's return value.
        //
        // The component model's `result<T, E>` WIT type maps to
        // `Val::Result(Result<Option<Box<Val>>, Option<Box<Val>>>)`.
        // Ok(val) → Result::Ok(Some(Box::new(val)))
        // Err(val) → Result::Err(Some(Box::new(val)))
        match &results[0] {
            wasmtime::component::Val::Result(result_val) => match result_val {
                Ok(Some(inner)) => match inner.as_ref() {
                    wasmtime::component::Val::String(s) => Ok(s.clone()),
                    other => Err(PluginError::AnalysisFailed(format!(
                        "Unexpected OK inner type: {other:?}"
                    ))),
                },
                Ok(None) => Err(PluginError::AnalysisFailed(
                    "WASI analyze returned Ok(()) but expected a string".to_string(),
                )),
                Err(Some(inner)) => Err(PluginError::AnalysisFailed(format!(
                    "WASI analyze returned error: {inner:?}"
                ))),
                Err(None) => Err(PluginError::AnalysisFailed(
                    "WASI analyze returned an error".to_string(),
                )),
            },
            wasmtime::component::Val::String(s) => Ok(s.clone()),
            other => Err(PluginError::AnalysisFailed(format!(
                "Unexpected return value: {other:?}"
            ))),
        }
    }

    /// Get plugin metadata.
    pub fn metadata(&self) -> &PluginMetadata {
        &self.manifest
    }

    /// Return the stdout/stderr the guest wrote during the most recent
    /// [`WasiPlugin::analyze`] call, replacing them with empty strings.
    ///
    /// Gate 4: guests cannot write to the host's real stdio; their output
    /// is captured here (each stream capped at 1 MiB, excess traps).
    pub fn take_captured_output(&mut self) -> (String, String) {
        std::mem::take(&mut self.captured_output)
    }
}

// ---------------------------------------------------------------------------
// WASI interface linking
// ---------------------------------------------------------------------------

/// Link the crawlkit-specific plugin interface.
///
/// Exports `crawlkit:plugin/context#get-context` which returns the
/// analysis context JSON blob set by the host before each `analyze` call.
///
/// Uses [`LinkerInstance::func_new`] with raw [`Val`] slices to avoid
/// the [`ComponentNamedList`] constraint of [`func_wrap`](wasmtime::component::LinkerInstance::func_wrap).
fn link_crawlkit_plugin(
    linker: &mut wasmtime::component::Linker<WasiPluginState>,
) -> Result<(), PluginError> {
    let mut ctx_instance = linker.instance("crawlkit:plugin/context").map_err(|e| {
        PluginError::LoadFailed(format!(
            "Failed to get crawlkit:plugin/context instance: {e}"
        ))
    })?;

    // get-context: returns Option<String> (the context JSON blob).
    // WIT signature: get-context: func() -> option<string>;
    //
    // Using func_new with raw vals since ComponentNamedList is only
    // implemented for tuples, not single types.
    ctx_instance
        .func_new(
            "get-context",
            |ctx: wasmtime::StoreContextMut<'_, WasiPluginState>,
             _ty: wasmtime::component::types::ComponentFunc,
             _params: &[wasmtime::component::Val],
             results: &mut [wasmtime::component::Val]| {
                let context = ctx.data().context_json.clone();
                // WIT option<string>: represented as Val::Option(Some(Val::String(...)))
                // or Val::Option(None).
                results[0] = match context {
                    Some(s) => wasmtime::component::Val::Option(Some(Box::new(
                        wasmtime::component::Val::String(s),
                    ))),
                    None => wasmtime::component::Val::Option(None),
                };
                Ok(())
            },
        )
        .map_err(|e| PluginError::LoadFailed(format!("Failed to link get-context: {e}")))?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wasi_plugin_state_wires_stdio_and_capabilities() {
        let state = WasiPluginState::new(Some(r#"{"url":"https://example.com"}"#.into()), true);
        assert_eq!(
            state.context_json.as_deref(),
            Some(r#"{"url":"https://example.com"}"#)
        );
        assert!(state.hooks.allow_network);
        assert!(state.stdout_pipe.contents().is_empty());
        assert!(state.stderr_pipe.contents().is_empty());

        let state = WasiPluginState::new(None, false);
        assert!(state.context_json.is_none());
        assert!(!state.hooks.allow_network);
    }

    #[test]
    fn stdio_pipes_capture_and_trap_beyond_capacity() {
        use wasmtime_wasi::p2::OutputStream as _;

        // Mirrors what the linked wasi:cli stdout implementation does with
        // the pipes wired into WasiCtxBuilder.
        let state = WasiPluginState::new(None, false);
        let mut stdout = state.stdout_pipe.clone();
        stdout
            .write(Bytes::from_static(b"hello "))
            .unwrap_or_else(|e| panic!("captured stdout write failed: {e}"));
        stdout
            .write(Bytes::from_static(b"world"))
            .unwrap_or_else(|e| panic!("captured stdout write failed: {e}"));
        assert_eq!(state.stdout_pipe.contents(), &b"hello world"[..]);

        let tiny = MemoryOutputPipe::new(4);
        let mut tiny_writer = tiny.clone();
        let result = tiny_writer.write(Bytes::from_static(b"12345"));
        assert!(result.is_err(), "write beyond capacity must fail");
        assert!(tiny.contents().is_empty());
    }

    /// Box a payload into the body type WASI HTTP uses, mapping the
    /// infallible error of `Full` onto the crate-root error type.
    fn boxed_body(payload: Bytes) -> WasiBody {
        use http_body_util::BodyExt as _;
        let body = http_body_util::Full::new(payload)
            .map_err(|_: std::convert::Infallible| Error::InternalError(None));
        UnsyncBoxBody::new(body)
    }

    #[test]
    fn http_hooks_deny_when_capability_not_granted() {
        let mut hooks = PluginHttpHooks {
            allow_network: false,
        };
        let request = hyper::Request::builder()
            .uri("https://example.com/")
            .body(boxed_body(Bytes::new()))
            .unwrap_or_else(|e| panic!("valid request rejected: {e}"));
        let mut fut = Box::into_pin(hooks.send_request(
            request,
            None,
            Box::new(async { Ok::<(), Error>(()) }),
        ));
        // The denial is decided synchronously: a single poll must observe it.
        let waker = std::task::Waker::noop();
        let mut cx = std::task::Context::from_waker(waker);
        match fut.as_mut().poll(&mut cx) {
            std::task::Poll::Ready(Err(Error::HttpRequestDenied)) => {}
            std::task::Poll::Ready(Err(err)) => {
                panic!("expected HttpRequestDenied denial, got {err:?}")
            }
            std::task::Poll::Ready(Ok(_)) => {
                panic!("expected capability denial, request unexpectedly succeeded")
            }
            std::task::Poll::Pending => panic!("expected synchronous denial, got Pending"),
        }
    }

    #[test]
    fn http_hooks_ssrf_block_private_targets() {
        for uri in [
            "http://127.0.0.1:8080/x",
            "http://192.168.1.1/x",
            "http://localhost/x",
            "http://169.254.169.254/latest/meta-data",
            "ftp://example.com/file",
        ] {
            let parsed: http::Uri = uri.parse().unwrap_or_else(|e| panic!("bad test uri: {e}"));
            let err = PluginHttpHooks::check_target(&parsed)
                .expect_err("private/disallowed target must be blocked");
            assert!(
                matches!(err, ErrorCode::DestinationIpProhibited),
                "unexpected error for {uri}: {err:?}"
            );
        }
    }

    #[test]
    fn http_hooks_ssrf_allows_public_targets() {
        for uri in ["https://example.com/", "http://example.com:8080/a?b=c"] {
            let parsed: http::Uri = uri.parse().unwrap_or_else(|e| panic!("bad test uri: {e}"));
            PluginHttpHooks::check_target(&parsed)
                .unwrap_or_else(|e| panic!("public target {uri} must pass, got {e:?}"));
        }
    }

    #[test]
    fn http_hooks_reject_uri_without_host() {
        let parsed: http::Uri = "/no-authority"
            .parse()
            .unwrap_or_else(|e| panic!("bad test uri: {e}"));
        assert!(matches!(
            PluginHttpHooks::check_target(&parsed),
            Err(ErrorCode::HttpRequestUriInvalid)
        ));
    }

    #[test]
    fn outcall_timeouts_are_clamped_to_10s() {
        let clamped = clamp_outcall_options(Some(RequestOptions {
            connect_timeout: Some(Duration::from_secs(600)),
            first_byte_timeout: Some(Duration::from_secs(600)),
            between_bytes_timeout: Some(Duration::from_millis(500)),
        }))
        .expect("Some input stays Some");
        assert_eq!(clamped.connect_timeout, Some(MAX_OUTCALL_TIMEOUT));
        assert_eq!(clamped.first_byte_timeout, Some(MAX_OUTCALL_TIMEOUT));
        assert_eq!(
            clamped.between_bytes_timeout,
            Some(Duration::from_millis(500))
        );
        assert!(clamp_outcall_options(None).is_none(), "None stays None");
    }

    #[test]
    fn response_body_cap_enforced() {
        let payload = Bytes::from(vec![0u8; MAX_OUTCALL_BODY_BYTES + 1]);
        let mut response = hyper::Response::new(boxed_body(payload));
        cap_response_body(&mut response, MAX_OUTCALL_BODY_BYTES);

        use http_body::Body as _;
        let waker = std::task::Waker::noop();
        let mut cx = std::task::Context::from_waker(waker);
        let body = response.into_body();
        match std::pin::pin!(body).poll_frame(&mut cx) {
            std::task::Poll::Ready(Some(Err(Error::InternalError(Some(msg))))) => {
                assert!(msg.contains("1 MiB"), "unexpected message: {msg}");
            }
            other => panic!("expected body-cap error, got {other:?}"),
        }
    }

    /// A component that imports `wasi:clocks/monotonic-clock` and exports
    /// `get-now` (host-called) which forwards to the imported `now`.
    ///
    /// Instantiation typechecks the import against the real
    /// `add_to_linker_sync` definitions, and a successful call proves the
    /// *real* clock implementation is linked (a trap stub would trap).
    const CLOCKS_COMPONENT: &str = r#"
        (component
          (import "wasi:clocks/monotonic-clock@0.2.0" (instance $clock
            (export "now" (func (result u64)))
          ))
          (alias export $clock "now" (func $now))
          (core module $m
            (import "clock" "now" (func $clock_now (result i64)))
            (memory (export "memory") 1)
            (func (export "get-now") (result i64) (call $clock_now))
          )
          (core func $now_lower (canon lower (func $now)))
          (core instance $clock_inst (export "now" (func $now_lower)))
          (core instance $i (instantiate $m (with "clock" (instance $clock_inst))))
          (type $now_ty (func (result u64)))
          (func $get_now (export "get-now") (type $now_ty)
            (canon lift (core func $i "get-now")))
        )
    "#;

    /// Build the same linker `analyze` uses, to exercise WASI interface
    /// linkage in isolation.
    fn build_linker(
        engine: &wasmtime::Engine,
    ) -> Result<wasmtime::component::Linker<WasiPluginState>, PluginError> {
        let mut linker = wasmtime::component::Linker::<WasiPluginState>::new(engine);
        wasmtime_wasi::p2::add_to_linker_sync(&mut linker)
            .map_err(|e| PluginError::LoadFailed(format!("Failed to link WASI CLI: {e}")))?;
        wasmtime_wasi_http::p2::add_only_http_to_linker_sync(&mut linker)
            .map_err(|e| PluginError::LoadFailed(format!("Failed to link WASI HTTP: {e}")))?;
        link_crawlkit_plugin(&mut linker)?;
        Ok(linker)
    }

    #[test]
    fn wasi_clock_import_is_really_linked() {
        let mut engine_config = wasmtime::Config::new();
        engine_config.consume_fuel(true);
        engine_config.epoch_interruption(true);
        let engine =
            wasmtime::Engine::new(&engine_config).unwrap_or_else(|e| panic!("engine: {e}"));
        let component = wasmtime::component::Component::new(&engine, CLOCKS_COMPONENT)
            .unwrap_or_else(|e| panic!("wat component: {e:#}"));
        let linker = build_linker(&engine).unwrap_or_else(|e| panic!("linker: {e}"));

        let mut store = wasmtime::Store::new(&engine, WasiPluginState::new(None, false));
        store
            .set_fuel(10_000_000)
            .unwrap_or_else(|e| panic!("fuel: {e}"));
        store.set_epoch_deadline(u64::MAX);

        let instance = linker
            .instantiate(&mut store, &component)
            .unwrap_or_else(|e| panic!("instantiate: {e}"));
        let func = instance
            .get_func(&mut store, "get-now")
            .unwrap_or_else(|| panic!("get-now export missing"));
        let mut results = [wasmtime::component::Val::Bool(false)];
        func.call(&mut store, &[], &mut results)
            .unwrap_or_else(|e| panic!("calling get-now failed (trap stub linked?): {e}"));
        let wasmtime::component::Val::U64(now) = &results[0] else {
            panic!("expected U64 result, got {:?}", results[0]);
        };
        // A real monotonic clock returns a positive nanosecond reading; the
        // trap stub and an unlinked import would have errored above.
        assert!(*now > 0, "monotonic clock returned 0");
    }

    /// Full Gate 3+4 pipeline: a component importing `wasi:clocks` whose
    /// `crawlkit:plugin/analyze` export receives HTML/URL strings
    /// and returns `result<string, string>`.
    const ANALYZE_COMPONENT: &str = r#"
        (component
          (type $analyze_ty (func (param "html" string) (param "url" string)
            (result (result string (error string)))))
          (import "wasi:clocks/monotonic-clock@0.2.0" (instance $clock
            (export "now" (func (result u64)))
          ))
          (alias export $clock "now" (func $now))
          (core module $m
            (import "clock" "now" (func $clock_now (result i64)))
            (memory (export "memory") 1)
            (global $bump (mut i32) (i32.const 8))
            (func $realloc (export "realloc")
              (param $old i32) (param $old_size i32) (param $align i32) (param $new_size i32)
              (result i32)
              (local $ptr i32)
              (local.set $ptr (global.get $bump))
              (global.set $bump (i32.add (global.get $bump)
                (i32.and (i32.add (local.get $new_size) (i32.const 3)) (i32.const -4))))
              (local.get $ptr)
            )
            ;; result<string, string> flattens to a return-area pointer:
            ;; [disc i32][ok ptr i32][ok len i32][err ptr i32][err len i32]
            (func (export "analyze")
              (param $hp i32) (param $hl i32) (param $up i32) (param $ul i32)
              (result i32)
              (local $ret i32)
              (local.set $ret (call $realloc
                (i32.const 0) (i32.const 0) (i32.const 4) (i32.const 20)))
              (i32.store (local.get $ret) (i32.const 0))
              (i32.store (i32.add (local.get $ret) (i32.const 4)) (i32.const 128))
              (i32.store (i32.add (local.get $ret) (i32.const 8)) (i32.const 2))
              (i32.store8 (i32.const 128) (i32.const 111))
              (i32.store8 (i32.const 129) (i32.const 107))
              (local.get $ret)
            )
          )
          (core func $now_lower (canon lower (func $now)))
          (core instance $clock_inst (export "now" (func $now_lower)))
          (core instance $i (instantiate $m (with "clock" (instance $clock_inst))))
          (func $analyze (export "crawlkit:plugin/analyze") (type $analyze_ty)
            (canon lift (core func $i "analyze")
              (memory (core memory $i "memory"))
              (realloc (core func $i "realloc"))))
        )
    "#;

    #[test]
    fn analyze_pipeline_with_wasi_linked() {
        let mut engine_config = wasmtime::Config::new();
        engine_config.consume_fuel(true);
        engine_config.epoch_interruption(true);
        let engine =
            wasmtime::Engine::new(&engine_config).unwrap_or_else(|e| panic!("engine: {e}"));
        let component = wasmtime::component::Component::new(&engine, ANALYZE_COMPONENT)
            .unwrap_or_else(|e| panic!("wat component: {e:#}"));
        let linker = build_linker(&engine).unwrap_or_else(|e| panic!("linker: {e}"));

        let mut store = wasmtime::Store::new(&engine, WasiPluginState::new(None, false));
        store
            .set_fuel(10_000_000)
            .unwrap_or_else(|e| panic!("fuel: {e}"));
        store.set_epoch_deadline(u64::MAX);

        let instance = linker
            .instantiate(&mut store, &component)
            .unwrap_or_else(|e| panic!("instantiate: {e}"));
        let func = instance
            .get_func(&mut store, ANALYZE_EXPORT)
            .unwrap_or_else(|| panic!("analyze export missing"));

        let mut results = [wasmtime::component::Val::Bool(false)];
        func.call(
            &mut store,
            &[
                wasmtime::component::Val::String("<html></html>".to_string()),
                wasmtime::component::Val::String("https://example.com".to_string()),
            ],
            &mut results,
        )
        .unwrap_or_else(|e| panic!("analyze call failed: {e}"));

        let wasmtime::component::Val::Result(inner) = &results[0] else {
            panic!("expected result type, got {:?}", results[0]);
        };
        match inner {
            Ok(Some(value)) => match value.as_ref() {
                wasmtime::component::Val::String(s) => assert_eq!(s, "ok"),
                other => panic!("expected string payload, got {other:?}"),
            },
            other => panic!("expected Ok payload, got {other:?}"),
        }
        assert!(store.data().stdout_pipe.contents().is_empty());
    }
}
