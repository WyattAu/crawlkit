use std::path::{Path, PathBuf};
use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::analyzers::Analyzer;

/// Plugin errors.
#[derive(Debug, Error)]
pub enum PluginError {
    #[error("plugin not found: {0}")]
    NotFound(String),

    #[error("plugin load failed: {0}")]
    LoadFailed(String),

    #[error("plugin symbol not found: {0}")]
    SymbolNotFound(String),

    #[error("plugin initialization failed: {0}")]
    InitializationFailed(String),
}

/// Plugin metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// Plugin name.
    pub name: String,
    /// Plugin version.
    pub version: String,
    /// Plugin author.
    pub author: String,
    /// Plugin description.
    pub description: String,
    /// Plugin API version (for compatibility).
    pub api_version: String,
}

/// Plugin trait for analyzers.
///
/// Plugins must implement this trait to be loaded by the plugin system.
pub trait PluginAnalyzer: Analyzer {
    /// Get plugin metadata.
    fn metadata(&self) -> PluginMetadata;

    /// Initialize the plugin.
    ///
    /// # Errors
    /// Returns error if initialization fails.
    fn initialize(&mut self) -> Result<(), PluginError>;
}

/// Plugin loader for native libraries.
pub struct PluginLoader {
    /// Loaded plugins.
    plugins: Arc<RwLock<Vec<Box<dyn PluginAnalyzer>>>>,
    /// Plugin search paths.
    search_paths: Vec<PathBuf>,
}

impl PluginLoader {
    /// Create a new plugin loader.
    ///
    /// Default search paths:
    /// - `./plugins` (current directory)
    /// - `$HOME/.crawlkit/plugins` (user home directory, if available)
    #[must_use]
    pub fn new() -> Self {
        let mut search_paths = vec![PathBuf::from("./plugins")];
        if let Some(home) = dirs::home_dir() {
            search_paths.push(home.join(".crawlkit/plugins"));
        }
        Self {
            plugins: Arc::new(RwLock::new(Vec::new())),
            search_paths,
        }
    }

    /// Add a search path for plugins.
    pub fn add_search_path(&mut self, path: PathBuf) {
        self.search_paths.push(path);
    }

    /// Load a plugin from a file path.
    ///
    /// Validates the plugin manifest and API version compatibility.
    ///
    /// # Note
    /// Dynamic library loading requires the `libloading` crate.
    /// Currently returns metadata only; actual plugin registration
    /// must be done programmatically by adding plugins via the loader.
    ///
    /// # Errors
    /// Returns error if manifest is missing, invalid, or incompatible.
    pub fn load_plugin(&self, path: &Path) -> Result<PluginMetadata, PluginError> {
        // Check if file exists
        if !path.exists() {
            return Err(PluginError::NotFound(path.display().to_string()));
        }

        // Read plugin manifest (JSON file next to the library)
        let manifest_path = path.with_extension("json");
        if !manifest_path.exists() {
            return Err(PluginError::LoadFailed(format!(
                "Plugin manifest not found: {}",
                manifest_path.display()
            )));
        }

        let manifest_content = std::fs::read_to_string(&manifest_path)
            .map_err(|e| PluginError::LoadFailed(format!("Failed to read manifest: {e}")))?;

        let metadata: PluginMetadata = serde_json::from_str(&manifest_content)
            .map_err(|e| PluginError::LoadFailed(format!("Invalid manifest format: {e}")))?;

        // Validate API version
        if metadata.api_version != "1.0" {
            return Err(PluginError::LoadFailed(format!(
                "Incompatible API version: {} (expected 1.0)",
                metadata.api_version
            )));
        }

        // TODO: Integrate `libloading` for dynamic library loading.
        // The loaded PluginAnalyzer would be inserted into self.plugins.

        Ok(metadata)
    }

    /// Load all plugins from search paths.
    ///
    /// # Errors
    /// Returns errors for each failed plugin load.
    pub fn load_all(&self) -> Vec<PluginError> {
        let mut errors = Vec::new();

        for path in &self.search_paths {
            if path.exists() {
                if let Ok(entries) = std::fs::read_dir(path) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path
                            .extension()
                            .is_some_and(|e| e == "so" || e == "dylib" || e == "wasm")
                        {
                            if let Err(e) = self.load_plugin(&path) {
                                errors.push(e);
                            }
                        }
                    }
                }
            }
        }

        errors
    }

    /// Get all loaded plugins.
    #[must_use]
    pub fn plugins(&self) -> Vec<PluginMetadata> {
        self.plugins.read().iter().map(|p| p.metadata()).collect()
    }

    /// Get plugin count.
    #[must_use]
    pub fn count(&self) -> usize {
        self.plugins.read().len()
    }

    /// Get search paths.
    #[must_use]
    pub fn search_paths(&self) -> &[PathBuf] {
        &self.search_paths
    }
}

impl Default for PluginLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// Plugin registry for managing plugins.
pub struct PluginRegistry {
    loader: PluginLoader,
}

impl PluginRegistry {
    /// Create a new plugin registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            loader: PluginLoader::new(),
        }
    }

    /// Load plugins from default paths.
    pub fn load_defaults(&mut self) -> Vec<PluginError> {
        self.loader.load_all()
    }

    /// Get plugin loader.
    #[must_use]
    pub fn loader(&self) -> &PluginLoader {
        &self.loader
    }

    /// Get mutable plugin loader.
    pub fn loader_mut(&mut self) -> &mut PluginLoader {
        &mut self.loader
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
    fn test_plugin_loader_default() {
        let loader = PluginLoader::new();
        assert_eq!(loader.count(), 0);
        assert!(!loader.search_paths().is_empty());
    }

    #[test]
    fn test_plugin_loader_add_search_path() {
        let mut loader = PluginLoader::new();
        loader.add_search_path(PathBuf::from("/tmp/plugins"));
        assert!(loader
            .search_paths()
            .contains(&PathBuf::from("/tmp/plugins")));
    }

    #[test]
    fn test_plugin_loader_nonexistent_path() {
        let loader = PluginLoader::new();
        let result = loader.load_plugin(Path::new("/nonexistent/plugin.so"));
        assert!(result.is_err());
    }

    #[test]
    fn test_plugin_registry_default() {
        let registry = PluginRegistry::new();
        assert_eq!(registry.loader().count(), 0);
    }
}
