mod crypto;
mod manifest;
mod sandbox;
#[cfg(feature = "wasi-preview2")]
pub mod wasi_preview2;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use parking_lot::RwLock;
use thiserror::Error;

pub use crypto::{sign_plugin_wasm, verify_plugin_artifact, TrustedPluginKey, TRUSTED_PLUGIN_KEYS};
pub use manifest::{
    validate_license, validate_manifest, validate_version, PluginAnalyzerInfo, PluginEntry,
    PluginKind, PluginManifest, PluginMetadata, PluginPermissions,
};
pub use sandbox::WasmConfig;
#[cfg(feature = "wasi-preview2")]
pub use wasi_preview2::WasiPlugin;

pub(crate) use crypto::verify_plugin_trust;
pub(crate) use manifest::read_plugin_manifest;

/// Plugin trust-chain verification policy.
///
/// Controls how [`WasmConfig`] treats the manifest's `wasm_hash` /
/// `signature` / `signed_by` trust fields during plugin loading.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PluginVerification {
    /// Require a valid `wasm_hash` plus an ed25519 `signature` made by a
    /// key in [`TRUSTED_PLUGIN_KEYS`]. Missing or invalid trust data
    /// rejects the plugin (fail-closed). This is the default.
    #[default]
    Required,
    /// Permit plugins without trust metadata (logged via `tracing::warn!`),
    /// while still rejecting any *present* hash or signature that fails
    /// verification — bad crypto is always fail-closed.
    AllowUnsigned,
}

/// Plugin errors.
#[derive(Debug, Error)]
pub enum PluginError {
    #[error("plugin not found: {0}")]
    NotFound(String),

    #[error("plugin load failed: {0}")]
    LoadFailed(String),

    #[error("plugin init failed: {0}")]
    InitFailed(String),

    #[error("plugin analysis failed: {0}")]
    AnalysisFailed(String),

    #[error("incompatible API version: {0} (expected 1.0)")]
    IncompatibleApiVersion(String),

    #[error("manifest parse error: {0}")]
    ManifestParse(String),

    #[error("WASM execution error: {0}")]
    WasmExecution(String),

    #[error("invalid manifest: {0}")]
    InvalidManifest(String),
}

/// Errors specific to plugin manifest validation.
#[derive(Debug, Error, PartialEq)]
pub enum ManifestError {
    #[error("name is required and must be non-empty")]
    NameRequired,

    #[error("name must contain only alphanumeric characters and hyphens")]
    NameInvalid,

    #[error("version is required and must be non-empty")]
    VersionRequired,

    #[error("version must be valid semver (X.Y.Z)")]
    VersionInvalid,

    #[error("description is required and must be non-empty")]
    DescriptionRequired,

    #[error("description exceeds maximum length of 500 characters")]
    DescriptionTooLong,

    #[error("author is required and must be non-empty")]
    AuthorRequired,

    #[error("license is required and must be non-empty")]
    LicenseRequired,

    #[error("license must be a valid SPDX identifier")]
    LicenseInvalid,

    #[error("entry_point (wasm) is required and must be non-empty")]
    EntryPointRequired,

    #[error("entry_point must end with .wasm")]
    EntryPointNotWasm,
}

/// Verify a plugin directory's trust chain exactly as the loader does
/// under [`PluginVerification::Required`], without compiling or
/// instantiating the module. Returns the verified metadata on success.
///
/// This is the check exposed to `crawlkit plugin verify`.
pub fn verify_plugin_dir(plugin_dir: &Path) -> Result<PluginMetadata, PluginError> {
    let manifest = read_plugin_manifest(plugin_dir)?;
    validate_manifest(&manifest.plugin).map_err(|e| PluginError::InvalidManifest(e.to_string()))?;

    let wasm_file = manifest
        .plugin
        .entry
        .wasm
        .as_deref()
        .ok_or_else(|| PluginError::LoadFailed("No WASM entry point specified".to_string()))?;
    let wasm_bytes = std::fs::read(plugin_dir.join(wasm_file))
        .map_err(|e| PluginError::LoadFailed(format!("Failed to read WASM file: {e}")))?;

    verify_plugin_trust(
        &manifest.plugin.name,
        manifest.plugin.wasm_hash.as_deref(),
        manifest.plugin.signature.as_deref(),
        manifest.plugin.signed_by.as_deref(),
        &wasm_bytes,
        &PluginVerification::Required,
    )?;
    Ok(manifest.plugin)
}

/// Loaded WASM plugin instance.
/// Per-plugin host state readable by guest `crawlkit_host.get_context`.
#[derive(Debug, Default, Clone)]
pub struct HostState {
    /// JSON blob of the analysis context (url, status, headers, parsed
    /// page summary) set by [`WasmPlugin::analyze_with_context`] before
    /// each analyze call. `None` for plain [`WasmPlugin::analyze`].
    context_json: Option<String>,
}

pub struct WasmPlugin {
    pub manifest: PluginMetadata,
    config: WasmConfig,
    engine: wasmtime::Engine,
    store: wasmtime::Store<HostState>,
    instance: wasmtime::Instance,
    memory: wasmtime::Memory,
}

/// Determine whether `url` is a public HTTP(S) target that the WASM host
/// fetch may follow. Rejects non-HTTP schemes, metadata/internal hostnames,
/// and private/loopback/link-local/multicast IP addresses.
fn is_public_http_url(url: &str) -> bool {
    crate::ssrf::is_public_url(url)
}

/// Dedicated blocking runtime for WASM host fetch calls (leaked static,
/// same pattern as [`PgStorage`](crate::pg_storage::BLOCKING_RUNTIME)).
static FETCH_RUNTIME: std::sync::OnceLock<tokio::runtime::Runtime> = std::sync::OnceLock::new();

pub(crate) fn fetch_runtime() -> &'static tokio::runtime::Runtime {
    #[allow(clippy::panic)]
    FETCH_RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .thread_name("crawlkit-wasm-fetch")
            .build()
            .unwrap_or_else(|e| panic!("WASM fetch runtime failed to build: {e}"))
    })
}

// Error types for plugin loading are already defined above.

impl WasmPlugin {
    /// Load a WASM plugin from a directory with default security configuration.
    pub fn load(plugin_dir: &Path) -> Result<Self, PluginError> {
        Self::load_with_config(plugin_dir, &WasmConfig::default())
    }

    /// Load a WASM plugin from a directory with custom security configuration.
    pub fn load_with_config(plugin_dir: &Path, config: &WasmConfig) -> Result<Self, PluginError> {
        let manifest = read_plugin_manifest(plugin_dir)?;

        if !manifest.plugin.api_version.starts_with("1.") {
            return Err(PluginError::IncompatibleApiVersion(
                manifest.plugin.api_version,
            ));
        }

        // Capability enforcement (fail-closed with grantable network).
        // filesystem and env_vars are always rejected; network is grantable
        // only when BOTH the manifest declares it AND the embedder enables it
        // via WasmConfig.allow_plugin_network.
        if let Some(perms) = &manifest.plugin.permissions {
            let network_requested = perms.network.unwrap_or(false);
            let filesystem_requested = perms.filesystem.unwrap_or(false);
            let env_vars_requested = perms.env_vars.as_ref().is_some_and(|v| !v.is_empty());
            if filesystem_requested || env_vars_requested {
                return Err(PluginError::InvalidManifest(
                    concat!(
                        "plugin requests filesystem/env_vars capabilities ",
                        "that the sandbox cannot grant; only network is grantable ",
                        "via allow_plugin_network config",
                    )
                    .to_string(),
                ));
            }
            if network_requested && !config.allow_plugin_network {
                return Err(PluginError::InvalidManifest(
                    concat!(
                        "plugin requests network capability but allow_plugin_network ",
                        "is false; set WasmConfig.allow_plugin_network = true to grant ",
                        "HTTP access (SSRF-validated, no redirects, 1 MiB cap, 10s timeout)"
                    )
                    .to_string(),
                ));
            }
            if network_requested && config.allow_plugin_network {
                tracing::info!(
                    "Granting network capability to plugin: {}",
                    manifest.plugin.name
                );
            }
        }

        // Validate manifest fields before loading WASM
        validate_manifest(&manifest.plugin).map_err(|e| {
            tracing::warn!(
                "Plugin manifest validation failed for {}: {}",
                manifest.plugin.name,
                e
            );
            PluginError::InvalidManifest(e.to_string())
        })?;

        let wasm_file =
            manifest.plugin.entry.wasm.as_ref().ok_or_else(|| {
                PluginError::LoadFailed("No WASM entry point specified".to_string())
            })?;
        let wasm_path = plugin_dir.join(wasm_file);

        // Trust chain (wasm_hash + ed25519 signature) is verified BEFORE
        // the module is handed to wasmtime, so untrusted bytes never even
        // reach the compiler.
        let wasm_bytes = std::fs::read(&wasm_path)
            .map_err(|e| PluginError::LoadFailed(format!("Failed to read WASM file: {e}")))?;
        verify_plugin_trust(
            &manifest.plugin.name,
            manifest.plugin.wasm_hash.as_deref(),
            manifest.plugin.signature.as_deref(),
            manifest.plugin.signed_by.as_deref(),
            &wasm_bytes,
            &config.plugin_verification,
        )?;

        // Configure wasmtime with fuel limits to prevent infinite loops and
        // epoch interruption to enforce wall-clock timeouts.
        let mut engine_config = wasmtime::Config::new();
        engine_config.consume_fuel(true);
        engine_config.epoch_interruption(true);
        let engine = wasmtime::Engine::new(&engine_config)
            .map_err(|e| PluginError::LoadFailed(format!("Failed to create engine: {}", e)))?;

        let module = wasmtime::Module::from_file(&engine, &wasm_path)
            .map_err(|e| PluginError::LoadFailed(format!("Failed to compile WASM: {}", e)))?;

        let mut store = wasmtime::Store::new(&engine, HostState::default());
        store
            .set_fuel(config.max_fuel)
            .map_err(|e| PluginError::LoadFailed(format!("Failed to set fuel: {}", e)))?;

        // With epoch interruption enabled, every call traps unless a
        // deadline is armed. Load-time calls (init) run under a
        // effectively-infinite deadline; only `analyze` arms the tight
        // per-call timeout.
        store.set_epoch_deadline(u64::MAX);

        let mut linker = wasmtime::Linker::new(&engine);

        // When network capability is granted (manifest declares it AND config
        // enables it), link the host fetch function. Otherwise the sandbox
        // remains pure-compute (no imports linked).
        let network_granted = manifest
            .plugin
            .permissions
            .as_ref()
            .is_some_and(|p| p.network.unwrap_or(false))
            && config.allow_plugin_network;

        if network_granted {
            linker
                .func_wrap(
                    "crawlkit_host",
                    "fetch",
                    |mut caller: wasmtime::Caller<'_, HostState>,
                     url_ptr: i32,
                     url_len: i32|
                     -> i32 {
                        // Read URL bytes from guest memory
                        let url_bytes = {
                            let memory = match caller.get_export("memory") {
                                Some(wasmtime::Extern::Memory(m)) => m,
                                _ => return 0,
                            };
                            let data = memory.data(&caller);
                            let start = url_ptr as usize;
                            let end = start + url_len as usize;
                            if end > data.len() {
                                return 0;
                            }
                            data[start..end].to_vec()
                        };

                        let url = match String::from_utf8(url_bytes) {
                            Ok(s) => s,
                            Err(_) => return 0,
                        };

                        if !is_public_http_url(&url) {
                            tracing::debug!("WASM fetch blocked by SSRF guard: {url}");
                            return 0;
                        }

                        // Fetch via the dedicated blocking runtime (never
                        // panics from within a Tokio worker because the
                        // runtime is separate and leaked).
                        let rt = fetch_runtime();
                        let result = rt.block_on(async {
                            let client = reqwest::Client::builder()
                                .redirect(reqwest::redirect::Policy::none())
                                .timeout(std::time::Duration::from_secs(10))
                                .build()
                                .map_err(|e| e.to_string())?;
                            let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;
                            let status = resp.status().as_u16();
                            let body = resp.bytes().await.map_err(|e| e.to_string())?;
                            // Cap at 1 MiB
                            let body = if body.len() > 1_048_576 {
                                &body[..1_048_576]
                            } else {
                                &body
                            };
                            let body_str = String::from_utf8_lossy(body).into_owned();
                            Ok::<(u16, String), String>((status, body_str))
                        });

                        let json = match result {
                            Ok((status, body)) => serde_json::json!({
                                "status": status,
                                "body": body,
                            })
                            .to_string(),
                            Err(e) => {
                                tracing::debug!("WASM fetch failed: {e}");
                                return 0;
                            }
                        };

                        let json_bytes = json.as_bytes();
                        let alloc_len = json_bytes.len() + 1; // +1 for NUL

                        // Allocate in guest via crawlkit_plugin_alloc
                        let alloc_fn = match caller
                            .get_export("crawlkit_plugin_alloc")
                            .and_then(|e| e.into_func())
                        {
                            Some(f) => f,
                            None => return 0,
                        };
                        let mut alloc_result = [wasmtime::Val::I32(0)];
                        if alloc_fn
                            .call(
                                &mut caller,
                                &[wasmtime::Val::I32(alloc_len as i32)],
                                &mut alloc_result,
                            )
                            .is_err()
                        {
                            return 0;
                        }
                        let result_ptr = match alloc_result[0] {
                            wasmtime::Val::I32(p) => p,
                            _ => return 0,
                        };
                        if result_ptr == 0 {
                            return 0;
                        }

                        // Write JSON + NUL into guest memory at result_ptr
                        if let Some(wasmtime::Extern::Memory(memory)) = caller.get_export("memory")
                        {
                            let data = memory.data_mut(&mut caller);
                            let start = result_ptr as usize;
                            let end = start + alloc_len;
                            if end <= data.len() {
                                data[start..start + json_bytes.len()].copy_from_slice(json_bytes);
                                data[start + json_bytes.len()] = 0; // NUL terminator
                            }
                        }

                        result_ptr
                    },
                )
                .map_err(|e| {
                    PluginError::LoadFailed(format!("Failed to link crawlkit_host.fetch: {e}"))
                })?;
        }

        // Structured context access (B4): always linked — it exposes nothing
        // beyond what the raw HTML input already conveys; it is pure
        // precomputed convenience for guests. Returns 0 (null) when no
        // context was set for this analyze call (plain `analyze`).
        linker
            .func_wrap(
                "crawlkit_host",
                "get_context",
                |mut caller: wasmtime::Caller<'_, HostState>| -> i32 {
                    let Some(json) = caller.data().context_json.clone() else {
                        return 0;
                    };
                    let bytes = json.as_bytes();
                    let alloc_len = bytes.len() + 1; // NUL terminator
                    let Some(alloc_fn) = caller
                        .get_export("crawlkit_plugin_alloc")
                        .and_then(|e| e.into_func())
                    else {
                        return 0;
                    };
                    let mut result = [wasmtime::Val::I32(0)];
                    if alloc_fn
                        .call(
                            &mut caller,
                            &[wasmtime::Val::I32(alloc_len as i32)],
                            &mut result,
                        )
                        .is_err()
                    {
                        return 0;
                    }
                    let wasmtime::Val::I32(ptr) = result[0] else {
                        return 0;
                    };
                    if ptr == 0 {
                        return 0;
                    }
                    if let Some(wasmtime::Extern::Memory(memory)) = caller.get_export("memory") {
                        let data = memory.data_mut(&mut caller);
                        let start = ptr as usize;
                        if start + alloc_len <= data.len() {
                            data[start..start + bytes.len()].copy_from_slice(bytes);
                            data[start + bytes.len()] = 0;
                        } else {
                            return 0;
                        }
                    } else {
                        return 0;
                    }
                    ptr
                },
            )
            .map_err(|e| {
                PluginError::LoadFailed(format!("Failed to link crawlkit_host.get_context: {e}"))
            })?;

        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| PluginError::LoadFailed(format!("Failed to instantiate WASM: {}", e)))?;

        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| PluginError::LoadFailed("No memory export found".to_string()))?;

        // Validate initial memory does not exceed configured limit.
        if memory.data_size(&store) > config.max_memory_bytes {
            return Err(PluginError::LoadFailed(format!(
                "Plugin initial memory ({} bytes) exceeds limit ({} bytes)",
                memory.data_size(&store),
                config.max_memory_bytes,
            )));
        }

        let init_func = instance
            .get_typed_func::<i32, i32>(&mut store, "crawlkit_plugin_init")
            .map_err(|e| PluginError::InitFailed(format!("Init function not found: {}", e)))?;

        let result = init_func
            .call(&mut store, 0)
            .map_err(|e| PluginError::InitFailed(format!("Init failed: {}", e)))?;

        if result != 0 {
            return Err(PluginError::InitFailed(format!(
                "Init returned error code: {}",
                result
            )));
        }

        Ok(Self {
            manifest: manifest.plugin,
            config: config.clone(),
            engine,
            store,
            instance,
            memory,
        })
    }

    /// Analyze HTML content using the plugin.
    ///
    /// Enforces the configured wall-clock timeout via epoch interruption:
    /// a watchdog thread increments the engine epoch when the deadline
    /// passes, trapping execution inside the guest.
    pub fn analyze(&mut self, html: &str, url: &str) -> Result<String, PluginError> {
        self.analyze_with_context(html, url, None)
    }

    /// Analyze with a structured context available to the guest.
    ///
    /// `context_json` (a serialized [`crate::analyzers::AnalysisContext`]
    /// summary) is stored in the host state before the guest runs; guests
    /// that import `crawlkit_host.get_context` receive it as a
    /// NUL-terminated JSON string. Guests that do not import it are
    /// unaffected — the v1 ABI is unchanged.
    ///
    /// # Errors
    ///
    /// Same failure modes as [`WasmPlugin::analyze`].
    pub fn analyze_with_context(
        &mut self,
        html: &str,
        url: &str,
        context_json: Option<&str>,
    ) -> Result<String, PluginError> {
        self.store.data_mut().context_json = context_json.map(str::to_string);

        let analyze_func = self
            .instance
            .get_typed_func::<(i32, i32, i32, i32), i32>(&mut self.store, "crawlkit_plugin_analyze")
            .map_err(|e| {
                PluginError::AnalysisFailed(format!("Analyze function not found: {}", e))
            })?;

        let html_bytes = html.as_bytes();
        let url_bytes = url.as_bytes();

        let alloc_func = self
            .instance
            .get_typed_func::<i32, i32>(&mut self.store, "crawlkit_plugin_alloc")
            .map_err(|e| PluginError::AnalysisFailed(format!("Alloc function not found: {}", e)))?;

        let html_ptr = alloc_func
            .call(&mut self.store, html_bytes.len() as i32)
            .map_err(|e| {
                PluginError::AnalysisFailed(format!("Failed to allocate for HTML: {}", e))
            })?;

        let url_ptr = alloc_func
            .call(&mut self.store, url_bytes.len() as i32)
            .map_err(|e| {
                PluginError::AnalysisFailed(format!("Failed to allocate for URL: {}", e))
            })?;

        // Bounds check: validate allocation results are within memory limits.
        self.validate_wasm_pointer(html_ptr as usize, html_bytes.len())?;
        self.validate_wasm_pointer(url_ptr as usize, url_bytes.len())?;

        self.memory.data_mut(&mut self.store)
            [html_ptr as usize..(html_ptr as usize + html_bytes.len())]
            .copy_from_slice(html_bytes);

        self.memory.data_mut(&mut self.store)
            [url_ptr as usize..(url_ptr as usize + url_bytes.len())]
            .copy_from_slice(url_bytes);

        // Arm the wall-clock deadline for this call, then start the watchdog
        // that bumps the engine epoch if the guest overruns.
        self.store.set_epoch_deadline(1);
        let watchdog =
            EpochWatchdog::spawn(self.engine.clone(), self.config.max_analysis_timeout_ms);

        let analyze_result = analyze_func.call(
            &mut self.store,
            (
                html_ptr,
                html_bytes.len() as i32,
                url_ptr,
                url_bytes.len() as i32,
            ),
        );

        watchdog.cancel();

        let result_ptr = analyze_result.map_err(|e| {
            // Epoch-deadline kills surface as `Trap::Interrupt` (or an
            // "epoch" message on some wasmtime versions); fuel exhaustion
            // reports "all fuel consumed".
            let is_timeout = e
                .downcast_ref::<wasmtime::Trap>()
                .is_some_and(|trap| matches!(trap, wasmtime::Trap::Interrupt))
                || e.to_string().contains("epoch");
            let is_fuel = e.to_string().contains("all fuel consumed");
            if is_timeout {
                PluginError::AnalysisFailed(format!(
                    "Plugin exceeded the {}ms analysis timeout and was terminated",
                    self.config.max_analysis_timeout_ms
                ))
            } else if is_fuel {
                PluginError::AnalysisFailed(format!(
                    "Plugin exhausted its {} instruction fuel budget and was terminated",
                    self.config.max_fuel
                ))
            } else {
                PluginError::AnalysisFailed(format!("Analyze failed: {}", e))
            }
        })?;

        // A null (0) return means the plugin could not produce a result
        // (e.g. allocation failure inside the guest).
        if result_ptr == 0 {
            return Err(PluginError::AnalysisFailed(
                "Plugin analyze returned a null pointer".to_string(),
            ));
        }

        let result = self.read_string(result_ptr as usize)?;

        let free_func = self
            .instance
            .get_typed_func::<i32, ()>(&mut self.store, "crawlkit_plugin_free")
            .map_err(|e| PluginError::AnalysisFailed(format!("Free function not found: {}", e)))?;
        let _ = free_func.call(&mut self.store, html_ptr);
        let _ = free_func.call(&mut self.store, url_ptr);
        let _ = free_func.call(&mut self.store, result_ptr);

        Ok(result)
    }

    /// Validate that a pointer+length region lies within the WASM memory bounds.
    fn validate_wasm_pointer(&self, ptr: usize, len: usize) -> Result<(), PluginError> {
        let mem_size = self.memory.data(&self.store).len();
        if ptr > mem_size || len > mem_size - ptr {
            return Err(PluginError::WasmExecution(format!(
                "WASM pointer out of bounds: ptr={}, len={}, memory_size={}",
                ptr, len, mem_size,
            )));
        }
        // Reject if the write region exceeds the configured memory limit.
        if ptr + len > self.config.max_memory_bytes {
            return Err(PluginError::WasmExecution(format!(
                "WASM memory access exceeds limit: ptr={}, len={}, limit={}",
                ptr, len, self.config.max_memory_bytes,
            )));
        }
        Ok(())
    }

    /// Read a null-terminated string from WASM memory.
    fn read_string(&self, ptr: usize) -> Result<String, PluginError> {
        let data = self.memory.data(&self.store);
        let mem_len = data.len();

        // Bounds check: ptr must be within memory.
        if ptr >= mem_len {
            return Err(PluginError::WasmExecution(format!(
                "String pointer out of bounds: ptr={}, memory_size={}",
                ptr, mem_len,
            )));
        }

        let end = data[ptr..].iter().position(|&b| b == 0).ok_or_else(|| {
            PluginError::WasmExecution(format!(
                "No null terminator found starting at ptr={}, memory_size={}",
                ptr, mem_len,
            ))
        })?;

        let bytes = &data[ptr..ptr + end];
        String::from_utf8(bytes.to_vec())
            .map_err(|e| PluginError::WasmExecution(format!("Invalid UTF-8: {}", e)))
    }

    /// Get plugin metadata.
    pub fn metadata(&self) -> &PluginMetadata {
        &self.manifest
    }
}

/// Wall-clock watchdog for WASM plugin execution.
///
/// Sleeps in small increments until either cancelled (the plugin finished)
/// or the deadline passes, at which point it increments the wasmtime engine
/// epoch — trapping any guest execution armed with an epoch deadline.
struct EpochWatchdog {
    done: Arc<std::sync::atomic::AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl EpochWatchdog {
    fn spawn(engine: wasmtime::Engine, timeout_ms: u64) -> Self {
        let done = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = Arc::clone(&done);
        let handle = std::thread::spawn(move || {
            let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);
            while !flag.load(std::sync::atomic::Ordering::Relaxed) {
                if std::time::Instant::now() >= deadline {
                    engine.increment_epoch();
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        });
        Self {
            done,
            handle: Some(handle),
        }
    }

    /// Signal completion and join the watchdog thread.
    fn cancel(mut self) {
        self.done.store(true, std::sync::atomic::Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for EpochWatchdog {
    fn drop(&mut self) {
        self.done.store(true, std::sync::atomic::Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// A loaded plugin instance — either a core ABI [`WasmPlugin`](WasmPlugin)
/// or, under the `wasi-preview2` feature, a WASI Preview 2 plugin.
///
/// This enum provides a uniform interface for the plugin registry and
/// runtime without requiring dynamic dispatch.
pub enum PluginInstance {
    /// Core WASM ABI plugin (legacy, default).
    Wasm(WasmPlugin),
    /// WASI Preview 2 component plugin.
    #[cfg(feature = "wasi-preview2")]
    Wasi(WasiPlugin),
}

impl PluginInstance {
    /// Get the plugin metadata.
    pub fn metadata(&self) -> &PluginMetadata {
        match self {
            Self::Wasm(p) => p.metadata(),
            #[cfg(feature = "wasi-preview2")]
            Self::Wasi(p) => p.metadata(),
        }
    }

    /// Analyze HTML content using the plugin.
    pub fn analyze(
        &mut self,
        html: &str,
        url: &str,
        context_json: Option<&str>,
    ) -> Result<String, PluginError> {
        match self {
            Self::Wasm(p) => match context_json {
                Some(ctx) => p.analyze_with_context(html, url, Some(ctx)),
                None => p.analyze(html, url),
            },
            #[cfg(feature = "wasi-preview2")]
            Self::Wasi(p) => p.analyze(html, url, context_json),
        }
    }
}

/// Plugin registry managing all loaded plugins.
pub struct PluginRegistry {
    plugins: Arc<RwLock<Vec<PluginInstance>>>,
    search_paths: Vec<PathBuf>,
}

impl PluginRegistry {
    /// Create empty plugin registry.
    pub fn new() -> Self {
        Self {
            plugins: Arc::new(RwLock::new(Vec::new())),
            search_paths: Vec::new(),
        }
    }

    /// Add a plugin search path.
    pub fn add_search_path(&mut self, path: PathBuf) {
        self.search_paths.push(path);
    }

    /// Scan search paths and load all valid plugins.
    pub fn load_all(&mut self) -> Vec<PluginError> {
        self.load_all_with_config(&WasmConfig::default())
    }

    /// Scan search paths and load all valid plugins with custom config.
    pub fn load_all_with_config(&mut self, config: &WasmConfig) -> Vec<PluginError> {
        let mut errors = Vec::new();

        for search_path in &self.search_paths {
            if !search_path.exists() {
                continue;
            }

            if let Ok(entries) = std::fs::read_dir(search_path) {
                for entry in entries.flatten() {
                    let plugin_dir = entry.path();
                    if plugin_dir.is_dir() {
                        match load_plugin_from_dir(&plugin_dir, config) {
                            Ok(instance) => {
                                let meta = instance.metadata();
                                tracing::info!(
                                    "Loaded plugin: {} v{} ({})",
                                    meta.name,
                                    meta.version,
                                    meta.kind.as_deref().unwrap_or("wasm"),
                                );
                                self.plugins.write().push(instance);
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "Failed to load plugin from {}: {}",
                                    plugin_dir.display(),
                                    e
                                );
                                errors.push(e);
                            }
                        }
                    }
                }
            }
        }

        errors
    }

    /// Get list of loaded plugin names.
    pub fn list(&self) -> Vec<String> {
        self.plugins
            .read()
            .iter()
            .map(|p| p.metadata().name.clone())
            .collect()
    }

    /// Get plugin count.
    pub fn count(&self) -> usize {
        self.plugins.read().len()
    }

    /// Run analysis through all loaded plugins.
    pub fn analyze_all(&self, html: &str, url: &str) -> Vec<Result<String, PluginError>> {
        let mut results = Vec::new();
        let mut plugins = self.plugins.write();

        for plugin in plugins.iter_mut() {
            results.push(plugin.analyze(html, url, None));
        }

        results
    }
}

/// Load a single plugin from a directory, dispatching to the correct
/// adapter based on the manifest's `kind` field.
pub fn load_plugin_from_dir(
    plugin_dir: &Path,
    config: &WasmConfig,
) -> Result<PluginInstance, PluginError> {
    // Peek at the manifest to determine the plugin kind before loading.
    let manifest = manifest::read_plugin_manifest(plugin_dir)?;
    let kind = PluginKind::from_manifest(manifest.plugin.kind.as_deref());

    match kind {
        PluginKind::Wasm => {
            WasmPlugin::load_with_config(plugin_dir, config).map(PluginInstance::Wasm)
        }
        #[cfg(feature = "wasi-preview2")]
        PluginKind::WasiComponent => {
            wasi_preview2::WasiPlugin::load(plugin_dir, config).map(PluginInstance::Wasi)
        }
        #[cfg(not(feature = "wasi-preview2"))]
        PluginKind::WasiComponent => Err(PluginError::LoadFailed(
            "WASI Preview 2 support is not enabled (compile with wasi-preview2 feature)"
                .to_string(),
        )),
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // Security defaults contract: any change that weakens the sandbox
    // defaults fails here. Mirrors docs/SECURITY_BOUNDARIES.md.
    #[test]
    fn wasmconfig_defaults_are_fail_closed() {
        let cfg = WasmConfig::default();
        // Fuel must be bounded (no zero / no unbounded compute).
        assert!(cfg.max_fuel > 0, "max_fuel must be bounded");
        // Memory must be bounded.
        assert!(cfg.max_memory_bytes > 0, "max_memory_bytes must be bounded");
        // Wall-clock timeout must be bounded.
        assert!(
            cfg.max_analysis_timeout_ms > 0,
            "max_analysis_timeout_ms must be bounded"
        );
        // Trust verification is required by default (fail-closed).
        assert_eq!(cfg.plugin_verification, PluginVerification::Required);
        // Network is deny-by-default.
        assert!(!cfg.allow_plugin_network, "network must be deny-by-default");
    }

    #[test]
    fn test_plugin_registry_default() {
        let registry = PluginRegistry::new();
        assert_eq!(registry.count(), 0);
        assert!(registry.list().is_empty());
    }

    #[test]
    fn test_plugin_loader_add_search_path() {
        let mut registry = PluginRegistry::new();
        registry.add_search_path(PathBuf::from("/tmp/plugins"));
        assert_eq!(registry.search_paths.len(), 1);
    }

    #[test]
    fn test_plugin_loader_nonexistent_path() {
        let mut registry = PluginRegistry::new();
        registry.add_search_path(PathBuf::from("/nonexistent/path"));
        let errors = registry.load_all();
        assert!(errors.is_empty());
    }

    #[test]
    fn test_validate_version_valid() {
        assert!(validate_version("1.0.0"));
        assert!(validate_version("0.1.0"));
        assert!(validate_version("10.20.30"));
        assert!(validate_version("0.0.1"));
    }

    #[test]
    fn test_validate_version_invalid() {
        assert!(!validate_version("1.0"));
        assert!(!validate_version("1.0.0.0"));
        assert!(!validate_version("1.0.beta"));
        assert!(!validate_version(""));
        assert!(!validate_version("01.0.0"));
        assert!(!validate_version("1.00.0"));
    }

    #[test]
    fn test_validate_license_valid() {
        assert!(validate_license("MIT"));
        assert!(validate_license("Apache-2.0"));
        assert!(validate_license("GPL-3.0-or-later"));
        assert!(validate_license("BSD-3-Clause"));
        assert!(validate_license("ISC"));
        assert!(validate_license("MPL-2.0"));
    }

    #[test]
    fn test_validate_license_invalid() {
        assert!(!validate_license("MIT-style"));
        assert!(!validate_license("Proprietary"));
        assert!(!validate_license(""));
        assert!(!validate_license("Custom-1.0"));
    }

    #[test]
    fn test_validate_manifest_valid() {
        let metadata = PluginMetadata {
            name: "test-plugin".to_string(),
            version: "1.0.0".to_string(),
            api_version: "1.0".to_string(),
            author: "Test Author".to_string(),
            description: "A test plugin".to_string(),
            license: "MIT".to_string(),
            kind: None,
            trust_level: None,
            entry: PluginEntry {
                wasm: Some("plugin.wasm".to_string()),
                native: None,
            },
            permissions: None,
            analyzer: None,
            wasm_hash: None,
            signature: None,
            signed_by: None,
        };
        assert!(validate_manifest(&metadata).is_ok());
    }

    #[test]
    fn test_validate_manifest_empty_name() {
        let metadata = PluginMetadata {
            name: "".to_string(),
            version: "1.0.0".to_string(),
            api_version: "1.0".to_string(),
            author: "Author".to_string(),
            description: "Desc".to_string(),
            license: "MIT".to_string(),
            kind: None,
            trust_level: None,
            entry: PluginEntry {
                wasm: Some("plugin.wasm".to_string()),
                native: None,
            },
            permissions: None,
            analyzer: None,
            wasm_hash: None,
            signature: None,
            signed_by: None,
        };
        assert_eq!(
            validate_manifest(&metadata).unwrap_err(),
            ManifestError::NameRequired
        );
    }

    #[test]
    fn test_validate_manifest_invalid_name() {
        let metadata = PluginMetadata {
            name: "test plugin!".to_string(),
            version: "1.0.0".to_string(),
            api_version: "1.0".to_string(),
            author: "Author".to_string(),
            description: "Desc".to_string(),
            license: "MIT".to_string(),
            kind: None,
            trust_level: None,
            entry: PluginEntry {
                wasm: Some("plugin.wasm".to_string()),
                native: None,
            },
            permissions: None,
            analyzer: None,
            wasm_hash: None,
            signature: None,
            signed_by: None,
        };
        assert_eq!(
            validate_manifest(&metadata).unwrap_err(),
            ManifestError::NameInvalid
        );
    }

    #[test]
    fn test_validate_manifest_invalid_version() {
        let metadata = PluginMetadata {
            name: "my-plugin".to_string(),
            version: "1.0".to_string(),
            api_version: "1.0".to_string(),
            author: "Author".to_string(),
            description: "Desc".to_string(),
            license: "MIT".to_string(),
            kind: None,
            trust_level: None,
            entry: PluginEntry {
                wasm: Some("plugin.wasm".to_string()),
                native: None,
            },
            permissions: None,
            analyzer: None,
            wasm_hash: None,
            signature: None,
            signed_by: None,
        };
        assert_eq!(
            validate_manifest(&metadata).unwrap_err(),
            ManifestError::VersionInvalid
        );
    }

    #[test]
    fn test_validate_manifest_description_too_long() {
        let metadata = PluginMetadata {
            name: "my-plugin".to_string(),
            version: "1.0.0".to_string(),
            api_version: "1.0".to_string(),
            author: "Author".to_string(),
            description: "a".repeat(501),
            license: "MIT".to_string(),
            kind: None,
            trust_level: None,
            entry: PluginEntry {
                wasm: Some("plugin.wasm".to_string()),
                native: None,
            },
            permissions: None,
            analyzer: None,
            wasm_hash: None,
            signature: None,
            signed_by: None,
        };
        assert_eq!(
            validate_manifest(&metadata).unwrap_err(),
            ManifestError::DescriptionTooLong
        );
    }

    #[test]
    fn test_validate_manifest_invalid_license() {
        let metadata = PluginMetadata {
            name: "my-plugin".to_string(),
            version: "1.0.0".to_string(),
            api_version: "1.0".to_string(),
            author: "Author".to_string(),
            description: "Desc".to_string(),
            license: "Proprietary".to_string(),
            kind: None,
            trust_level: None,
            entry: PluginEntry {
                wasm: Some("plugin.wasm".to_string()),
                native: None,
            },
            permissions: None,
            analyzer: None,
            wasm_hash: None,
            signature: None,
            signed_by: None,
        };
        assert_eq!(
            validate_manifest(&metadata).unwrap_err(),
            ManifestError::LicenseInvalid
        );
    }

    #[test]
    fn test_validate_manifest_no_wasm_entry() {
        let metadata = PluginMetadata {
            name: "my-plugin".to_string(),
            version: "1.0.0".to_string(),
            api_version: "1.0".to_string(),
            author: "Author".to_string(),
            description: "Desc".to_string(),
            license: "MIT".to_string(),
            kind: None,
            trust_level: None,
            entry: PluginEntry {
                wasm: None,
                native: None,
            },
            permissions: None,
            analyzer: None,
            wasm_hash: None,
            signature: None,
            signed_by: None,
        };
        assert_eq!(
            validate_manifest(&metadata).unwrap_err(),
            ManifestError::EntryPointRequired
        );
    }

    #[test]
    fn test_validate_manifest_non_wasm_entry() {
        let metadata = PluginMetadata {
            name: "my-plugin".to_string(),
            version: "1.0.0".to_string(),
            api_version: "1.0".to_string(),
            author: "Author".to_string(),
            description: "Desc".to_string(),
            license: "MIT".to_string(),
            kind: None,
            trust_level: None,
            entry: PluginEntry {
                wasm: Some("plugin.js".to_string()),
                native: None,
            },
            permissions: None,
            analyzer: None,
            wasm_hash: None,
            signature: None,
            signed_by: None,
        };
        assert_eq!(
            validate_manifest(&metadata).unwrap_err(),
            ManifestError::EntryPointNotWasm
        );
    }
}
