use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// Encryption configuration for data at rest.
///
/// Controls whether encryption is enabled, where to load the key from,
/// and which algorithm to use. Default is disabled with AES-256-GCM.
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

/// Encryption manager for data at rest.
///
/// Provides AES-256-GCM encryption for sensitive data. Key is loaded from
/// file, environment variable, or system keyring. When encryption is disabled,
/// `encrypt` and `decrypt` are passthrough operations.
///
/// # Examples
///
/// ```rust
/// use crawlkit_engine::{EncryptionManager, EncryptionConfig};
///
/// let manager = EncryptionManager::default();
/// assert!(!manager.is_enabled());
/// // When disabled, encrypt/decrypt are identity operations
/// let data = b"hello";
/// assert_eq!(manager.encrypt(data).unwrap(), data);
/// ```
pub struct EncryptionManager {
    config: EncryptionConfig,
    initialized: Arc<RwLock<bool>>,
    key: Arc<RwLock<Option<Vec<u8>>>>,
}

impl EncryptionManager {
    /// Create new encryption manager.
    #[must_use]
    pub fn new(config: EncryptionConfig) -> Self {
        Self {
            config,
            initialized: Arc::new(RwLock::new(false)),
            key: Arc::new(RwLock::new(None)),
        }
    }

    /// Check if encryption is enabled.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Initialize encryption by loading the key.
    ///
    /// # Errors
    /// Returns error if key cannot be loaded or is invalid.
    pub fn initialize(&self) -> Result<(), EncryptionError> {
        if !self.config.enabled {
            return Ok(());
        }

        let key = self.load_key()?;

        // Validate key length (256 bits = 32 bytes for AES-256)
        if key.len() != 32 {
            return Err(EncryptionError::InvalidKeyFormat(format!(
                "Expected 32 bytes, got {} bytes",
                key.len()
            )));
        }

        *self.key.write() = Some(key);
        *self.initialized.write() = true;
        Ok(())
    }

    /// Load encryption key from configured source.
    fn load_key(&self) -> Result<Vec<u8>, EncryptionError> {
        match &self.config.key_source {
            KeySource::File(path) => std::fs::read(path).map_err(|e| {
                EncryptionError::KeyNotFound(format!(
                    "Failed to read key file {}: {}",
                    path.display(),
                    e
                ))
            }),
            KeySource::EnvVar(var_name) => {
                std::env::var(var_name)
                    .map(|v| v.into_bytes())
                    .map_err(|_| {
                        EncryptionError::KeyNotFound(format!(
                            "Environment variable {} not set",
                            var_name
                        ))
                    })
            }
            KeySource::Keyring(service) => {
                // Try environment variable first as fallback
                let env_key = format!(
                    "CRAWLKIT_KEYRING_{}",
                    service.to_uppercase().replace('-', "_")
                );
                if let Ok(key) = std::env::var(&env_key) {
                    return Ok(key.into_bytes());
                }

                // Try file-based keyring
                let keyring_path = dirs::home_dir()
                    .map(|h| {
                        h.join(".crawlkit")
                            .join("keyrings")
                            .join(format!("{}.key", service))
                    })
                    .ok_or_else(|| {
                        EncryptionError::KeyNotFound("Cannot determine home directory".to_string())
                    })?;

                if keyring_path.exists() {
                    std::fs::read(&keyring_path).map_err(|e| {
                        EncryptionError::KeyNotFound(format!(
                            "Failed to read keyring file {}: {}",
                            keyring_path.display(),
                            e
                        ))
                    })
                } else {
                    Err(EncryptionError::KeyNotFound(format!(
                        "Keyring '{}' not found. Set {} environment variable or create keyring file at {}",
                        service, env_key, keyring_path.display()
                    )))
                }
            }
        }
    }

    /// Encrypt data using AES-256-GCM.
    ///
    /// # Errors
    /// Returns error if encryption fails.
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, EncryptionError> {
        if !self.config.enabled {
            return Ok(plaintext.to_vec());
        }

        let key = self.key.read();
        let key = key.as_ref().ok_or_else(|| {
            EncryptionError::InitializationFailed("Encryption not initialized".to_string())
        })?;

        // Generate random nonce (96 bits for AES-GCM)
        let mut nonce = [0u8; 12];
        getrandom::getrandom(&mut nonce).map_err(|e| {
            EncryptionError::InitializationFailed(format!("Failed to generate nonce: {}", e))
        })?;

        // Create AES-256-GCM cipher

        use aes_gcm::{aead::Aead, Aes256Gcm, KeyInit, Nonce};

        let cipher = Aes256Gcm::new_from_slice(key)
            .map_err(|e| EncryptionError::InvalidKeyFormat(format!("Invalid key: {}", e)))?;

        let nonce = Nonce::from_slice(&nonce);
        let ciphertext = cipher.encrypt(nonce, plaintext).map_err(|e| {
            EncryptionError::InitializationFailed(format!("Encryption failed: {}", e))
        })?;

        // Prepend nonce to ciphertext
        let mut result = nonce.to_vec();
        result.extend_from_slice(&ciphertext);
        Ok(result)
    }

    /// Decrypt data using AES-256-GCM.
    ///
    /// # Errors
    /// Returns error if decryption fails.
    pub fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>, EncryptionError> {
        if !self.config.enabled {
            return Ok(ciphertext.to_vec());
        }

        let key = self.key.read();
        let key = key.as_ref().ok_or_else(|| {
            EncryptionError::InitializationFailed("Encryption not initialized".to_string())
        })?;

        if ciphertext.len() < 12 {
            return Err(EncryptionError::InvalidKeyFormat(
                "Ciphertext too short".to_string(),
            ));
        }

        // Extract nonce from ciphertext
        let (nonce_bytes, actual_ciphertext) = ciphertext.split_at(12);

        use aes_gcm::{aead::Aead, Aes256Gcm, KeyInit, Nonce};

        let cipher = Aes256Gcm::new_from_slice(key)
            .map_err(|e| EncryptionError::InvalidKeyFormat(format!("Invalid key: {}", e)))?;

        let nonce = Nonce::from_slice(nonce_bytes);
        let plaintext = cipher.decrypt(nonce, actual_ciphertext).map_err(|e| {
            EncryptionError::InitializationFailed(format!("Decryption failed: {}", e))
        })?;

        Ok(plaintext)
    }

    /// Check if initialized.
    #[must_use]
    pub fn is_initialized(&self) -> bool {
        *self.initialized.read()
    }

    /// Get configuration.
    #[must_use]
    pub fn config(&self) -> &EncryptionConfig {
        &self.config
    }
}

/// Encryption errors.
#[derive(Debug, thiserror::Error)]
pub enum EncryptionError {
    /// The encryption key could not be found.
    #[error("encryption key not found: {0}")]
    KeyNotFound(String),

    /// The encryption key has invalid format or length.
    #[error("invalid key format: {0}")]
    InvalidKeyFormat(String),

    /// Encryption/decryption initialization failed.
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

    #[test]
    fn test_encryption_decrypt_roundtrip() {
        // Create a valid 32-byte key
        let key = vec![0u8; 32];
        let config = EncryptionConfig {
            enabled: true,
            key_source: KeySource::EnvVar("TEST_KEY".to_string()),
            algorithm: EncryptionAlgorithm::Aes256Gcm,
        };

        let manager = EncryptionManager::new(config);

        // Manually set the key for testing
        *manager.key.write() = Some(key);
        *manager.initialized.write() = true;

        // Test encrypt/decrypt roundtrip
        let plaintext = b"Hello, World!";
        let ciphertext = manager.encrypt(plaintext).unwrap();
        let decrypted = manager.decrypt(&ciphertext).unwrap();

        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn test_encryption_different_plaintexts() {
        let key = vec![42u8; 32];
        let config = EncryptionConfig {
            enabled: true,
            key_source: KeySource::EnvVar("TEST_KEY".to_string()),
            algorithm: EncryptionAlgorithm::Aes256Gcm,
        };

        let manager = EncryptionManager::new(config);
        *manager.key.write() = Some(key);
        *manager.initialized.write() = true;

        // Different plaintexts should produce different ciphertexts
        let ct1 = manager.encrypt(b"message1").unwrap();
        let ct2 = manager.encrypt(b"message2").unwrap();
        assert_ne!(ct1, ct2);
    }

    #[test]
    fn test_encryption_disabled_passthrough() {
        let manager = EncryptionManager::default();
        let data = b"test data";

        // When disabled, encrypt/decrypt should be passthrough
        let encrypted = manager.encrypt(data).unwrap();
        let decrypted = manager.decrypt(&encrypted).unwrap();

        assert_eq!(data.to_vec(), decrypted);
    }
}
