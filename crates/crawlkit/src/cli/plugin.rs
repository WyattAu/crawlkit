//! `crawlkit plugin` — WASM plugin signing and trust-chain management.
//!
//! Provides `keygen` (create an ed25519 signing keypair), `sign` (hash +
//! sign a plugin's `.wasm` and record the trust fields in its manifest),
//! and `verify` (run the same verification the plugin loader performs).

use anyhow::{anyhow, Context, Result};
use clap::Subcommand;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use crawlkit_engine::plugin::{sign_plugin_wasm, verify_plugin_dir, PluginManifest};

#[derive(Subcommand)]
pub enum PluginCommands {
    /// Generate an ed25519 signing keypair for plugin manifests
    Keygen {
        /// Directory to write plugin-signing.key / plugin-signing.pub into
        #[arg(long)]
        out: PathBuf,

        /// Overwrite existing key files
        #[arg(long)]
        force: bool,
    },
    /// Hash + sign a plugin's .wasm and record the trust fields in its manifest
    Sign {
        /// Plugin directory containing crawlkit-plugin.toml and the .wasm
        #[arg(long)]
        plugin: PathBuf,

        /// Secret key file (hex seed) produced by `plugin keygen`
        #[arg(long)]
        key: PathBuf,
    },
    /// Verify a plugin's hash/signature trust chain (same check as the loader)
    Verify {
        /// Plugin directory containing crawlkit-plugin.toml and the .wasm
        #[arg(long)]
        plugin: PathBuf,
    },
}

/// Entry point for `crawlkit plugin <command>`.
pub fn run(command: PluginCommands) -> Result<()> {
    match command {
        PluginCommands::Keygen { out, force } => keygen(&out, force),
        PluginCommands::Sign { plugin, key } => sign(&plugin, &key),
        PluginCommands::Verify { plugin } => verify(&plugin),
    }
}

/// Generate a fresh ed25519 keypair and write it as hex files.
fn keygen(out: &Path, force: bool) -> Result<()> {
    use ed25519_dalek::SigningKey;
    use rand::RngCore;

    let secret_path = out.join("plugin-signing.key");
    let public_path = out.join("plugin-signing.pub");
    if !force {
        for path in [&secret_path, &public_path] {
            if path.exists() {
                return Err(anyhow!(
                    "key file already exists at {} (use --force to overwrite)",
                    path.display()
                ));
            }
        }
    }

    let mut seed = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut seed);
    let signing_key = SigningKey::from_bytes(&seed);
    let public_hex = hex_encode(&signing_key.verifying_key().to_bytes());
    let secret_hex = hex_encode(&seed);

    std::fs::create_dir_all(out)
        .with_context(|| format!("failed to create key directory {}", out.display()))?;
    std::fs::write(&secret_path, format!("{secret_hex}\n"))
        .with_context(|| format!("failed to write {}", secret_path.display()))?;
    std::fs::write(&public_path, format!("{public_hex}\n"))
        .with_context(|| format!("failed to write {}", public_path.display()))?;

    println!("Generated plugin signing key in {}", out.display());
    println!("  secret key: {}", secret_path.display());
    println!("  public key: {}", public_path.display());
    println!("  key id:     {}", &public_hex[..16]);
    println!(
        "Note: plugins signed with this key only load under a Required policy if its\n\
         public key is added to the engine's TRUSTED_PLUGIN_KEYS trust store."
    );
    Ok(())
}

/// Hash + sign the plugin's .wasm and write the trust fields to its manifest.
fn sign(plugin_dir: &Path, key_path: &Path) -> Result<()> {
    let key_hex = std::fs::read_to_string(key_path)
        .with_context(|| format!("failed to read signing key {}", key_path.display()))?;
    let seed: [u8; 32] = hex_decode(key_hex.trim())
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or_else(|| anyhow!("signing key must be 64 hex characters (32 bytes)"))?;

    let manifest_path = plugin_dir.join("crawlkit-plugin.toml");
    let manifest_str = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("failed to read manifest {}", manifest_path.display()))?;
    let mut manifest: PluginManifest = toml::from_str(&manifest_str)
        .with_context(|| format!("failed to parse manifest {}", manifest_path.display()))?;

    let wasm_file = manifest
        .plugin
        .entry
        .wasm
        .clone()
        .ok_or_else(|| anyhow!("manifest declares no [plugin.entry] wasm path"))?;
    let wasm_path = plugin_dir.join(&wasm_file);
    let wasm_bytes = std::fs::read(&wasm_path)
        .with_context(|| format!("failed to read {}", wasm_path.display()))?;

    let (wasm_hash, signature, signed_by) = sign_plugin_wasm(&wasm_bytes, &seed);
    manifest.plugin.wasm_hash = Some(wasm_hash.clone());
    manifest.plugin.signature = Some(signature);
    manifest.plugin.signed_by = Some(signed_by.clone());

    let updated = toml::to_string(&manifest).context("failed to serialize updated manifest")?;
    std::fs::write(&manifest_path, &updated)
        .with_context(|| format!("failed to write manifest {}", manifest_path.display()))?;

    println!(
        "Signed plugin '{}' ({}): wasm_hash {wasm_hash}, signed_by {signed_by}",
        manifest.plugin.name,
        wasm_path.display()
    );
    Ok(())
}

/// Run the loader's trust-chain verification against a plugin directory.
fn verify(plugin_dir: &Path) -> Result<()> {
    let metadata =
        verify_plugin_dir(plugin_dir).map_err(|e| anyhow!("plugin verification failed: {e}"))?;

    println!(
        "Plugin '{}' v{} verification: OK",
        metadata.name, metadata.version
    );
    println!(
        "  wasm_hash: {}",
        metadata.wasm_hash.as_deref().unwrap_or("<none>")
    );
    println!(
        "  signed_by: {} (trusted)",
        metadata.signed_by.as_deref().unwrap_or("<none>")
    );

    if let Some(wasm_file) = &metadata.entry.wasm {
        let wasm_bytes = std::fs::read(plugin_dir.join(wasm_file))
            .with_context(|| format!("failed to re-read {}", wasm_file))?;
        use sha2::Digest;
        println!(
            "  computed sha256: {}",
            hex_encode(&sha2::Sha256::digest(&wasm_bytes))
        );
    }
    Ok(())
}

/// Encode bytes as lowercase hex.
fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// Decode a hex string; `None` on malformed input.
fn hex_decode(s: &str) -> Option<Vec<u8>> {
    if s.len() & 1 != 0 {
        return None;
    }
    s.as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok())
        .collect()
}
