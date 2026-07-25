use std::path::{Path, PathBuf};
use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Security configuration for WASM plugin execution.
#[derive(Clone)]
pub struct WasmConfig {
    /// Maximum fuel (instructions) a WASM plugin may consume before being
    /// killed. Prevents infinite loops / CPU exhaustion.
    pub max_fuel: u64,
    /// Maximum bytes the WASM linear memory may grow to.
    pub max_memory_bytes: usize,
    /// Timeout for a single `analyze` call in milliseconds (reserved for
    /// future use with epoch-based interruption).
    pub max_analysis_timeout_ms: u64,
}

impl Default for WasmConfig {
    fn default() -> Self {
        Self {
            // ~10 billion instructions – generous for legitimate analysis
            // but prevents runaway loops.
            max_fuel: 10_000_000_000,
            // 64 MiB – sufficient for HTML processing without allowing
            // memory-bomb attacks.
            max_memory_bytes: 64 * 1024 * 1024,
            // 30 seconds per analysis call.
            max_analysis_timeout_ms: 30_000,
        }
    }
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
}

/// Plugin manifest (crawlkit-plugin.toml).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub plugin: PluginMetadata,
}

/// Plugin metadata from manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub api_version: String,
    pub author: String,
    pub description: String,
    pub license: String,
    pub trust_level: Option<String>,
    pub entry: PluginEntry,
    pub permissions: Option<PluginPermissions>,
    pub analyzer: Option<PluginAnalyzerInfo>,
}

/// Plugin entry point configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginEntry {
    pub wasm: Option<String>,
    pub native: Option<String>,
}

/// WASM plugin permissions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginPermissions {
    pub network: Option<bool>,
    pub filesystem: Option<bool>,
    pub env_vars: Option<Vec<String>>,
}

/// Plugin analyzer metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginAnalyzerInfo {
    pub name: String,
    pub category: Option<String>,
    pub description: Option<String>,
    pub severity: Option<String>,
}

/// Loaded WASM plugin instance.
pub struct WasmPlugin {
    pub manifest: PluginMetadata,
    config: WasmConfig,
    store: wasmtime::Store<()>,
    instance: wasmtime::Instance,
    memory: wasmtime::Memory,
}

impl WasmPlugin {
    /// Load a WASM plugin from a directory with default security configuration.
    pub fn load(plugin_dir: &Path) -> Result<Self, PluginError> {
        Self::load_with_config(plugin_dir, &WasmConfig::default())
    }

    /// Load a WASM plugin from a directory with custom security configuration.
    pub fn load_with_config(plugin_dir: &Path, config: &WasmConfig) -> Result<Self, PluginError> {
        let manifest_path = plugin_dir.join("crawlkit-plugin.toml");
        let manifest_str = std::fs::read_to_string(&manifest_path)
            .map_err(|e| PluginError::ManifestParse(format!("Failed to read manifest: {}", e)))?;
        let manifest: PluginManifest = toml::from_str(&manifest_str)
            .map_err(|e| PluginError::ManifestParse(format!("Invalid manifest: {}", e)))?;

        if !manifest.plugin.api_version.starts_with("1.") {
            return Err(PluginError::IncompatibleApiVersion(
                manifest.plugin.api_version,
            ));
        }

        let wasm_file =
            manifest.plugin.entry.wasm.as_ref().ok_or_else(|| {
                PluginError::LoadFailed("No WASM entry point specified".to_string())
            })?;
        let wasm_path = plugin_dir.join(wasm_file);

        // Configure wasmtime with fuel limits to prevent infinite loops.
        let mut engine_config = wasmtime::Config::new();
        engine_config.consume_fuel(true);
        let engine = wasmtime::Engine::new(&engine_config)
            .map_err(|e| PluginError::LoadFailed(format!("Failed to create engine: {}", e)))?;

        let module = wasmtime::Module::from_file(&engine, &wasm_path)
            .map_err(|e| PluginError::LoadFailed(format!("Failed to compile WASM: {}", e)))?;

        let mut store = wasmtime::Store::new(&engine, ());
        store
            .set_fuel(config.max_fuel)
            .map_err(|e| PluginError::LoadFailed(format!("Failed to set fuel: {}", e)))?;

        let linker = wasmtime::Linker::new(&engine);

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
            store,
            instance,
            memory,
        })
    }

    /// Analyze HTML content using the plugin.
    pub fn analyze(&mut self, html: &str, url: &str) -> Result<String, PluginError> {
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

        let result_ptr = analyze_func
            .call(
                &mut self.store,
                (
                    html_ptr,
                    html_bytes.len() as i32,
                    url_ptr,
                    url_bytes.len() as i32,
                ),
            )
            .map_err(|e| PluginError::AnalysisFailed(format!("Analyze failed: {}", e)))?;

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

/// Plugin registry managing all loaded plugins.
pub struct PluginRegistry {
    plugins: Arc<RwLock<Vec<WasmPlugin>>>,
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
        let mut errors = Vec::new();

        for search_path in &self.search_paths {
            if !search_path.exists() {
                continue;
            }

            if let Ok(entries) = std::fs::read_dir(search_path) {
                for entry in entries.flatten() {
                    let plugin_dir = entry.path();
                    if plugin_dir.is_dir() {
                        match WasmPlugin::load(&plugin_dir) {
                            Ok(plugin) => {
                                tracing::info!(
                                    "Loaded plugin: {} v{}",
                                    plugin.metadata().name,
                                    plugin.metadata().version
                                );
                                self.plugins.write().push(plugin);
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
            results.push(plugin.analyze(html, url));
        }

        results
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
}
