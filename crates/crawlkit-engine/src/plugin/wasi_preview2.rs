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
//!   calls with the page HTML and URL, receiving findings as JSON
//!
//! # Security
//!
//! WASI components run under the same fuel/epoch/resource limits as core
//! WASM plugins. Filesystem and network access are controlled via the
//! manifest's `permissions` and the host's `WasmConfig` — identical to
//! the core ABI. The component model adds capability-based security:
//! each WASI interface must be explicitly linked by the host.
//!
//! # Gate Coverage
//!
//! - **Gate 3**: WASI Preview 2 component model support (this module)
//! - **Gate 4**: WASI CLI commands — stubbed (trap on call)
//! - **Gate 5**: WASI HTTP outcalls — stubbed (trap on call)
//!
//! Full WASI host implementations for Gates 4 and 5 require the
//! `wasmtime-wasi` crate. The structural scaffolding is in place;
//! link the actual WASI implementations when `wasmtime-wasi` is added
//! as a dependency.

use std::path::Path;

use crate::plugin::manifest::PluginMetadata;
use crate::plugin::PluginError;

use super::sandbox::WasmConfig;

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
}

/// Per-instance host state for WASI plugins.
///
/// Separate from [`super::HostState`] because the component model uses typed
/// state via [`wasmtime::Store<T>`] rather than raw memory access.
#[derive(Debug, Default, Clone)]
pub(crate) struct WasiPluginState {
    /// JSON blob of the analysis context available to the guest via
    /// `get-context`. Set before each `analyze` call.
    pub(crate) context_json: Option<String>,
    /// Whether network capability is granted to this plugin.
    /// Reserved for Gate 5 (WASI HTTP outcalls) when full host
    /// implementations are linked.
    #[allow(dead_code)]
    pub(crate) allow_network: bool,
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

        let wasm_file = manifest
            .plugin
            .entry
            .wasm
            .as_ref()
            .ok_or_else(|| PluginError::LoadFailed("No WASM entry point specified".to_string()))?;
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
        let component = wasmtime::component::Component::from_file(&engine, &wasm_path).map_err(
            |e| PluginError::LoadFailed(format!("Failed to compile WASI component: {e}")),
        )?;

        Ok(Self {
            manifest: manifest.plugin,
            config: config.clone(),
            engine,
            component,
        })
    }

    /// Analyze HTML content using the WASI component.
    ///
    /// Instantiates the component with a fully-linked WASI store, calls
    /// its `analyze` export, and returns the JSON findings payload.
    /// Enforces the wall-clock timeout via epoch interruption.
    pub fn analyze(
        &mut self,
        html: &str,
        url: &str,
        context_json: Option<&str>,
    ) -> Result<String, PluginError> {
        // Build the WASI linker fresh per call (state changes per invocation).
        let mut linker = wasmtime::component::Linker::<WasiPluginState>::new(&self.engine);

        // Trap on any unlinked WASI imports — components that import
        // wasi:cli or wasi:http will trap until the real implementations
        // are linked (requires wasmtime-wasi crate).
        linker
            .define_unknown_imports_as_traps(&self.component)
            .map_err(|e| {
                PluginError::LoadFailed(format!("Failed to define unknown imports: {e}"))
            })?;

        // Link the crawlkit-specific plugin interface.
        link_crawlkit_plugin(&mut linker)?;

        let network_granted = self
            .manifest
            .permissions
            .as_ref()
            .is_some_and(|p| p.network.unwrap_or(false))
            && self.config.allow_plugin_network;

        let mut store = wasmtime::Store::new(
            &self.engine,
            WasiPluginState {
                context_json: context_json.map(str::to_string),
                allow_network: network_granted,
            },
        );

        store
            .set_fuel(self.config.max_fuel)
            .map_err(|e| PluginError::WasmExecution(format!("Failed to set fuel: {e}")))?;

        // Arm the wall-clock deadline and start the watchdog.
        store.set_epoch_deadline(1);
        let watchdog =
            super::EpochWatchdog::spawn(self.engine.clone(), self.config.max_analysis_timeout_ms);

        // Instantiate the component.
        let instance = linker
            .instantiate(&mut store, &self.component)
            .map_err(|e| {
                PluginError::AnalysisFailed(format!("Component instantiation failed: {e}"))
            })?;

        // Call the crawlkit:plugin/analyze#analyze export.
        let analyze_func = instance
            .get_func(&mut store, "crawlkit:plugin/analyze#analyze")
            .ok_or_else(|| {
                PluginError::AnalysisFailed(
                    "Component does not export crawlkit:plugin/analyze#analyze".to_string(),
                )
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
    let mut ctx_instance = linker
        .instance("crawlkit:plugin/context")
        .map_err(|e| {
            PluginError::LoadFailed(format!("Failed to get crawlkit:plugin/context instance: {e}"))
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
    fn wasi_plugin_state_default() {
        let state = WasiPluginState::default();
        assert!(state.context_json.is_none());
        assert!(!state.allow_network);
    }

    #[test]
    fn wasi_plugin_state_clone() {
        let state = WasiPluginState {
            context_json: Some(r#"{"url":"https://example.com"}"#.to_string()),
            allow_network: true,
        };
        let cloned = state.clone();
        assert_eq!(state.context_json, cloned.context_json);
        assert_eq!(state.allow_network, cloned.allow_network);
    }
}
