use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// Encryption configuration for data at rest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    /// Enable encryption at rest.
    pub enabled: bool,
    /// Encryption key source.
    pub key_source: KeySource,
    /// Encryption algorithm.
    pub algorithm: EncryptionAlgorithm,
}

/// Source of encryption key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeySource {
    /// Key from file path.
    File(PathBuf),
    /// Key from environment variable.
    EnvVar(String),
    /// Key from system keyring.
    Keyring(String),
}

/// Encryption algorithm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EncryptionAlgorithm {
    Aes256Gcm,
    Aes256Cbc,
}

impl Default for EncryptionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            key_source: KeySource::EnvVar("CRAWLKIT_ENCRYPTION_KEY".to_string()),
            algorithm: EncryptionAlgorithm::Aes256Gcm,
        }
    }
}

/// Encryption manager (placeholder for SQLCipher integration).
pub struct EncryptionManager {
    config: EncryptionConfig,
    initialized: Arc<RwLock<bool>>,
}

impl EncryptionManager {
    /// Create new encryption manager.
    #[must_use]
    pub fn new(config: EncryptionConfig) -> Self {
        Self {
            config,
            initialized: Arc::new(RwLock::new(false)),
        }
    }

    /// Check if encryption is enabled.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Initialize encryption (placeholder).
    ///
    /// # Errors
    /// Returns error if key cannot be loaded.
    pub fn initialize(&self) -> Result<(), EncryptionError> {
        if !self.config.enabled {
            return Ok(());
        }

        // Placeholder: actual implementation would load key
        // and initialize SQLCipher connection
        *self.initialized.write() = true;
        Ok(())
    }

    /// Check if initialized.
    #[must_use]
    pub fn is_initialized(&self) -> bool {
        *self.initialized.read()
    }
}

/// Encryption errors.
#[derive(Debug, thiserror::Error)]
pub enum EncryptionError {
    #[error("encryption key not found: {0}")]
    KeyNotFound(String),

    #[error("invalid key format: {0}")]
    InvalidKeyFormat(String),

    #[error("encryption initialization failed: {0}")]
    InitializationFailed(String),
}

impl Default for EncryptionManager {
    fn default() -> Self {
        Self::new(EncryptionConfig::default())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_config_default() {
        let config = EncryptionConfig::default();
        assert!(!config.enabled);
    }

    #[test]
    fn test_encryption_manager_disabled() {
        let manager = EncryptionManager::default();
        assert!(!manager.is_enabled());
        assert!(manager.initialize().is_ok());
    }
}
