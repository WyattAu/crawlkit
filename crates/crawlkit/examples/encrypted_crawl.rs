//! Example: Crawl with encryption at rest.
//!
//! Run with: CRAWLKIT_ENCRYPTION_KEY=$(openssl rand -hex 32) cargo run --example encrypted_crawl

use crawlkit_core::encryption::{EncryptionConfig, EncryptionManager, KeySource};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = EncryptionConfig {
        enabled: true,
        key_source: KeySource::EnvVar("CRAWLKIT_ENCRYPTION_KEY".to_string()),
        ..Default::default()
    };

    let manager = EncryptionManager::new(config);

    match manager.initialize() {
        Ok(()) => println!("Encryption initialized successfully"),
        Err(e) => {
            eprintln!("Failed to initialize encryption: {}", e);
            eprintln!("Set CRAWLKIT_ENCRYPTION_KEY environment variable to a 32-byte hex string.");
            return Ok(());
        }
    }

    let plaintext = b"This is sensitive crawl data that should be encrypted.";
    let ciphertext = manager.encrypt(plaintext)?;
    println!("Plaintext: {} bytes", plaintext.len());
    println!("Ciphertext: {} bytes", ciphertext.len());

    let decrypted = manager.decrypt(&ciphertext)?;
    assert_eq!(plaintext.to_vec(), decrypted);
    println!("Decryption verified successfully.");

    Ok(())
}
