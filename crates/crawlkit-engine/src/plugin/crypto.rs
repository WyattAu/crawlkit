use std::fmt::Write as _;

use sha2::{Digest, Sha256};

use super::{PluginError, PluginVerification};

/// A public key trusted to sign crawlkit plugins.
#[derive(Debug, Clone, Copy)]
pub struct TrustedPluginKey {
    /// Key id — the first 16 hex characters of the public key.
    pub key_id: &'static str,
    /// Ed25519 verifying key as 64 hex characters.
    pub public_key_hex: &'static str,
}

/// Built-in plugin trust store: the public keys allowed to sign plugins
/// loaded under [`PluginVerification::Required`].
///
/// Key rotation is a deliberate, auditable event: adding, rotating, or
/// removing a key happens via a PR that updates this constant (with the
/// key id and reason documented in the changelog) and ships in a release.
/// Never commit matching secret keys — first-party signing secrets live
/// only in the release signing environment.
pub const TRUSTED_PLUGIN_KEYS: &[TrustedPluginKey] = &[
    // Primary trust anchor — generated for v5.0.0.
    TrustedPluginKey {
        key_id: "12a7a8db5aabb20b",
        public_key_hex: "12a7a8db5aabb20b6ac20bd18b09d7179246e5757ca6f3f368429016c724240a",
    },
];

/// Encode bytes as lowercase hex.
pub(crate) fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// Decode a lowercase-or-uppercase hex string; `None` on malformed input.
pub(crate) fn hex_decode(s: &str) -> Option<Vec<u8>> {
    if s.len() & 1 != 0 {
        return None;
    }
    // `as_chunks` (clippy 1.98+ suggestion) requires Rust 1.88; MSRV is 1.85.
    #[allow(unknown_lints, clippy::chunks_exact_to_as_chunks)]
    let bytes = s
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok())
        .collect::<Option<Vec<_>>>();
    bytes
}

/// Verify a plugin's hash/signature trust chain against a policy.
///
/// Fail-closed rules (both policies):
/// - a *declared* `wasm_hash` must match the actual `.wasm` bytes;
/// - a *present* `signature` must be valid ed25519 over the raw sha256
///   digest and made by a key in [`TRUSTED_PLUGIN_KEYS`];
/// - `signature`/`signed_by` must appear together.
///
/// [`PluginVerification::Required`] additionally rejects manifests with a
/// missing `wasm_hash` or missing signature pair, while
/// [`PluginVerification::AllowUnsigned`] logs a warning and continues.
pub(crate) fn verify_plugin_trust(
    plugin_name: &str,
    wasm_hash: Option<&str>,
    signature: Option<&str>,
    signed_by: Option<&str>,
    wasm_bytes: &[u8],
    policy: &PluginVerification,
) -> Result<(), PluginError> {
    let digest = Sha256::digest(wasm_bytes);
    let actual_hash = hex_encode(&digest);

    // Any declared wasm_hash must match the bytes on disk — a mismatch
    // means the binary was tampered with (or the manifest is stale).
    if let Some(declared) = wasm_hash {
        if !declared.eq_ignore_ascii_case(&actual_hash) {
            return Err(PluginError::InvalidManifest(format!(
                "wasm_hash mismatch for plugin '{}': index declares {declared} but the .wasm hashes to {actual_hash}",
                plugin_name
            )));
        }
    }

    if signature.is_some() != signed_by.is_some() {
        return Err(PluginError::InvalidManifest(format!(
            "plugin '{}' declares signature and signed_by individually; both must be present together",
            plugin_name
        )));
    }

    let verify = || -> Result<(), String> {
        match (signature, signed_by) {
            (Some(sig), Some(signer)) => verify_ed25519_signature(&digest, sig, signer),
            _ => {
                tracing::warn!("unsigned plugin loaded: {plugin_name}");
                Ok(())
            }
        }
    };

    match policy {
        PluginVerification::Required => {
            if wasm_hash.is_none() {
                return Err(PluginError::InvalidManifest(format!(
                    "missing wasm_hash for plugin '{}' (verification policy: required)",
                    plugin_name
                )));
            }
            if signature.is_none() {
                return Err(PluginError::InvalidManifest(format!(
                    "missing signature/signed_by for plugin '{}' (verification policy: required)",
                    plugin_name
                )));
            }
            verify().map_err(|reason| {
                PluginError::InvalidManifest(format!(
                    "signature verification failed for plugin '{}': {reason}",
                    plugin_name
                ))
            })
        }
        PluginVerification::AllowUnsigned => verify().map_err(|reason| {
            PluginError::InvalidManifest(format!(
                "signature verification failed for plugin '{}': {reason}",
                plugin_name
            ))
        }),
    }
}

/// Verify an in-memory plugin artifact's trust chain against the built-in
/// trust store under the strictest ([`PluginVerification::Required`])
/// policy. Used by plugin installation: artifacts are verified BEFORE
/// anything is written to the install root.
///
/// # Errors
///
/// Returns [`PluginError::InvalidManifest`] on hash mismatch, missing
/// trust fields, unknown signer, or invalid signature.
pub fn verify_plugin_artifact(
    plugin_name: &str,
    wasm_bytes: &[u8],
    wasm_hash: &str,
    signature: &str,
    signed_by: &str,
) -> Result<(), PluginError> {
    verify_plugin_trust(
        plugin_name,
        Some(wasm_hash),
        Some(signature),
        Some(signed_by),
        wasm_bytes,
        &PluginVerification::Required,
    )
}

/// Verify a hex ed25519 signature over a digest against the trust store.
fn verify_ed25519_signature(
    digest: &[u8],
    signature_hex: &str,
    signed_by: &str,
) -> Result<(), String> {
    let trusted = TRUSTED_PLUGIN_KEYS
        .iter()
        .find(|key| key.key_id.eq_ignore_ascii_case(signed_by))
        .ok_or_else(|| {
            format!("signed by unknown key id '{signed_by}' (not in the built-in trust store)")
        })?;

    let pubkey_bytes = hex_decode(trusted.public_key_hex).ok_or_else(|| {
        format!(
            "trust store entry '{}' has a malformed public key",
            trusted.key_id
        )
    })?;
    let pubkey_bytes: [u8; 32] = pubkey_bytes.try_into().map_err(|_| {
        format!(
            "trust store entry '{}' public key is not 32 bytes",
            trusted.key_id
        )
    })?;
    let verifying_key = ed25519_dalek::VerifyingKey::from_bytes(&pubkey_bytes)
        .map_err(|e| format!("invalid trusted public key: {e}"))?;

    // Guard against a misconfigured trust store entry whose key id does
    // not actually identify its own public key.
    let expected_id = &hex_encode(&pubkey_bytes)[..16];
    if !expected_id.eq_ignore_ascii_case(trusted.key_id) {
        return Err(format!(
            "trust store entry '{}' does not match its public key (expected id '{expected_id}')",
            trusted.key_id
        ));
    }

    let sig_bytes = hex_decode(signature_hex).ok_or("signature is not valid hex")?;
    let signature = ed25519_dalek::Signature::from_slice(&sig_bytes)
        .map_err(|e| format!("invalid signature encoding: {e}"))?;

    use ed25519_dalek::Verifier;
    verifying_key
        .verify(digest, &signature)
        .map_err(|e| format!("bad signature: {e}"))
}

/// Hash and sign plugin WASM bytes, producing the manifest trust fields.
///
/// `secret_key` is the raw 32-byte ed25519 seed (hex-decoded from the
/// output of `crawlkit plugin keygen`). Returns
/// `(wasm_hash, signature, signed_by)` hex values suitable for the
/// manifest. ed25519-dalek types deliberately do not appear in the
/// public signature; callers pass bytes and receive hex strings.
pub fn sign_plugin_wasm(wasm: &[u8], secret_key: &[u8; 32]) -> (String, String, String) {
    use ed25519_dalek::Signer;

    let signing_key = ed25519_dalek::SigningKey::from_bytes(secret_key);
    let public_hex = hex_encode(&signing_key.verifying_key().to_bytes());
    let digest = Sha256::digest(wasm);
    let signature = signing_key.sign(&digest);
    let key_id = public_hex[..16].to_string();
    (
        hex_encode(&digest),
        hex_encode(&signature.to_bytes()),
        key_id,
    )
}
