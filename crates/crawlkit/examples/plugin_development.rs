//! Example: The crawlkit WASM plugin lifecycle — build, sign, verify.
//!
//! This walks through the same steps as the `crawlkit plugin` CLI commands
//! at the library level:
//!
//! 1. **Manifest** — author a `crawlkit-plugin.toml` (`PluginMetadata`) and
//!    validate it with `validate_manifest` (name, semver, SPDX license,
//!    entry point rules).
//! 2. **Sign** — hash the plugin `.wasm` bytes and sign the digest with an
//!    ed25519 key via `sign_plugin_wasm`, producing the manifest trust
//!    fields (`wasm_hash`, `signature`, `signed_by`).
//! 3. **Verify** — run `verify_plugin_artifact`, which checks that the
//!    bytes match the declared hash and that the signature comes from a
//!    key in the built-in trust store (`TRUSTED_PLUGIN_KEYS`).
//!
//! A self-signed demo plugin intentionally fails trust-store verification:
//! production plugins must be signed by a release key. Tampered bytes are
//! always rejected by the hash check.
//!
//! Run with: cargo run --example plugin_development

use crawlkit_engine::plugin::{
    sign_plugin_wasm, validate_manifest, verify_plugin_artifact, PluginEntry, PluginMetadata,
    TRUSTED_PLUGIN_KEYS,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // --- Step 1: author and validate the manifest --------------------------
    println!("[1/4] Building plugin manifest...");
    let mut manifest = PluginMetadata {
        name: "acme-meta-check".to_string(),
        version: "1.0.0".to_string(),
        // The plugin ABI version crawlkit expects.
        api_version: "1.0".to_string(),
        author: "Acme SEO Team".to_string(),
        description: "Checks for Acme-specific meta conventions".to_string(),
        // Must be a known SPDX identifier (see validate_license).
        license: "Apache-2.0".to_string(),
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

    validate_manifest(&manifest)?;
    println!("  manifest valid: {} v{}", manifest.name, manifest.version);

    // --- Step 2: the plugin artifact ---------------------------------------
    // A real plugin is a WASM binary produced by targeting
    // `wasm32-wasip1` with the crawlkit-plugin-sdk. For this walkthrough a
    // placeholder byte blob stands in for the compiled artifact.
    let wasm_bytes: &[u8] = b"\0asm\x01\x00\x00\x00 demo crawlkit plugin module";

    // --- Step 3: sign the artifact -----------------------------------------
    println!("\n[2/4] Signing plugin artifact...");
    // In production this 32-byte seed comes from `crawlkit plugin keygen`
    // (or the release signing environment) — never hard-code a real key.
    let signing_key: [u8; 32] =
        hex_decode_seed("9a6d1e0c2f5b48a7c3d1e6f2a8b4c0d5e7f1a3b5c9d2e4f6a8b0c2d4e6f8a1b3")?;
    let (wasm_hash, signature, signed_by) = sign_plugin_wasm(wasm_bytes, &signing_key);

    manifest.wasm_hash = Some(wasm_hash.clone());
    manifest.signature = Some(signature.clone());
    manifest.signed_by = Some(signed_by.clone());

    println!("  wasm_hash: {wasm_hash}");
    println!("  signature: {}...", &signature[..32.min(signature.len())]);
    println!("  signed_by: {signed_by} (first 16 hex chars of the public key)");

    // --- Step 4a: tamper detection (hash check) ----------------------------
    println!("\n[3/4] Verifying a *tampered* artifact...");
    let mut tampered = wasm_bytes.to_vec();
    tampered.extend_from_slice(b" injected");
    match verify_plugin_artifact(
        &manifest.name,
        &tampered,
        manifest.wasm_hash.as_deref().unwrap_or_default(),
        manifest.signature.as_deref().unwrap_or_default(),
        manifest.signed_by.as_deref().unwrap_or_default(),
    ) {
        Ok(()) => println!("  unexpectedly verified (this should not happen)"),
        Err(e) => println!("  rejected as expected: {e}"),
    }

    // --- Step 4b: trust-store check ----------------------------------------
    println!("\n[4/4] Verifying the authentic artifact...");
    println!("  built-in trust store key ids:");
    for key in TRUSTED_PLUGIN_KEYS {
        println!("    - {}", key.key_id);
    }
    match verify_plugin_artifact(
        &manifest.name,
        wasm_bytes,
        manifest.wasm_hash.as_deref().unwrap_or_default(),
        manifest.signature.as_deref().unwrap_or_default(),
        manifest.signed_by.as_deref().unwrap_or_default(),
    ) {
        Ok(()) => println!("  verified: signed by a trusted release key"),
        Err(e) => {
            // Expected for this demo: the hash matches (the bytes were not
            // tampered with), but the *signer* is not in the trust store.
            println!("  rejected as expected: {e}");
            println!();
            println!("  Self-signed plugins are rejected under the default");
            println!("  `PluginVerification::Required` policy. Publish through a");
            println!("  release process whose key is in TRUSTED_PLUGIN_KEYS, or");
            println!("  explicitly load with `PluginVerification::AllowUnsigned`");
            println!("  in trusted development environments.");
        }
    }

    println!("\nPlugin development walkthrough complete.");
    Ok(())
}

/// Decode a 64-character hex string into a 32-byte key seed.
fn hex_decode_seed(hex: &str) -> Result<[u8; 32], String> {
    let bytes = (0..hex.len() / 2)
        .map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16))
        .collect::<Result<Vec<u8>, _>>()
        .map_err(|e| format!("invalid hex seed: {e}"))?;
    bytes
        .try_into()
        .map_err(|_| "seed must be 32 bytes".to_string())
}
