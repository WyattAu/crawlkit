use crawlkit_engine::EncryptionManager;

/// Decrypt an encrypted field value (hex-encoded, prefixed with "enc:").
pub fn decrypt_field(encryption: &EncryptionManager, field: &Option<String>) -> Option<String> {
    match field {
        Some(val) if val.starts_with("enc:") => {
            let hex_str = &val[4..];
            if let Ok(bytes) = hex_decode(hex_str) {
                encryption
                    .decrypt(&bytes)
                    .ok()
                    .and_then(|b| String::from_utf8(b).ok())
            } else {
                Some(val.clone())
            }
        }
        other => other.clone(),
    }
}

/// Decode a hex string to bytes.
pub fn hex_decode(hex: &str) -> Result<Vec<u8>, anyhow::Error> {
    if !hex.len().is_multiple_of(2) {
        return Err(anyhow::anyhow!("Invalid hex string length"));
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|e| anyhow::anyhow!(e)))
        .collect()
}
