//! Git/file-based plugin index: distribution for signed WASM plugins.
//!
//! The index is a single TOML file (versioned in a git repository) mapping
//! plugin names to content-addressed, ed25519-signed artifacts. Installation
//! fetches an artifact, verifies its hash and signature against the
//! engine's built-in trust store (ADR-006), and materializes the exact
//! directory layout [`WasmPlugin`](crate::plugin::WasmPlugin) loads —
//! no server infrastructure required.
//!
//! Index format (`plugin-index.toml`):
//!
//! ```toml
//! [[plugin]]
//! name = "title-length"
//! version = "1.0.0"
//! api_version = "1.0"
//! author = "crawlkit"
//! description = "Flags missing and oversized <title> elements"
//! license = "Apache-2.0"
//! categories = ["seo"]
//! # Artifact source: a path relative to the index file, or an https URL.
//! wasm_path = "artifacts/title-length-1.0.0.wasm"
//! # Trust fields (produced by `crawlkit plugin sign`).
//! wasm_hash = "<sha256 hex of the .wasm>"
//! signature = "<ed25519 signature hex over the hash>"
//! signed_by = "<key id: first 16 hex chars of the signer's public key>"
//! ```
//!
//! [`install_plugin`] is the entry point; unknown signers, hash mismatches,
//! and malformed entries are rejected before anything touches the install
//! root.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use thiserror::Error;

use crate::plugin::{verify_plugin_artifact, PluginError};

/// Errors from index parsing and plugin installation.
#[derive(Debug, Error)]
pub enum PluginIndexError {
    #[error("index parse error: {0}")]
    Parse(String),

    #[error("plugin '{0}' not found in index")]
    NotFound(String),

    #[error("artifact fetch failed: {0}")]
    Fetch(String),

    #[error("artifact I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("trust verification failed: {0}")]
    Trust(#[from] PluginError),
}

/// One `[[plugin]]` entry in the index.
#[derive(Debug, Clone, Deserialize)]
pub struct PluginIndexEntry {
    pub name: String,
    pub version: String,
    pub api_version: String,
    pub author: String,
    pub description: String,
    pub license: String,
    pub categories: Vec<String>,
    /// Path relative to the index file, or an https URL.
    pub wasm_path: String,
    pub wasm_hash: String,
    pub signature: String,
    pub signed_by: String,
}

/// Parse an index document. Entry order is preserved.
///
/// # Errors
///
/// Returns [`PluginIndexError::Parse`] on malformed TOML.
pub fn parse_plugin_index(toml_str: &str) -> Result<Vec<PluginIndexEntry>, PluginIndexError> {
    #[derive(Deserialize)]
    struct Index {
        plugin: Vec<PluginIndexEntry>,
    }
    let index: Index =
        toml::from_str(toml_str).map_err(|e| PluginIndexError::Parse(e.to_string()))?;
    Ok(index.plugin)
}

/// Fetch bytes over https on the plugin fetch runtime.
fn http_get(url: &str) -> Result<Vec<u8>, PluginIndexError> {
    let rt = crate::plugin::fetch_runtime();
    rt.block_on(async {
        let resp = reqwest::get(url)
            .await
            .map_err(|e| PluginIndexError::Fetch(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(PluginIndexError::Fetch(format!(
                "HTTP {} for {url}",
                resp.status()
            )));
        }
        resp.bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| PluginIndexError::Fetch(e.to_string()))
    })
}

/// Read the index and resolve an artifact's bytes for `entry`.
fn fetch_artifact(
    index_source: &str,
    entry: &PluginIndexEntry,
) -> Result<Vec<u8>, PluginIndexError> {
    if entry.wasm_path.starts_with("https://") || entry.wasm_path.starts_with("http://") {
        return http_get(&entry.wasm_path);
    }
    // Path relative to the index file's directory.
    let index_dir = Path::new(index_source)
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let artifact = index_dir.join(&entry.wasm_path);
    std::fs::read(&artifact)
        .map_err(|e| PluginIndexError::Fetch(format!("{}: {e}", artifact.display())))
}

/// Install a plugin by name from an index into `install_root/<name>/`.
///
/// `index_source` is a filesystem path to a `plugin-index.toml` (or an
/// https URL to one). The artifact's hash and ed25519 signature are
/// verified against the built-in trust store BEFORE anything is written;
/// on success the directory is immediately loadable by
/// [`WasmPlugin::load`](crate::plugin::WasmPlugin::load) under the
/// default Required policy.
///
/// # Errors
///
/// See [`PluginIndexError`]. On any error the install root is left
/// untouched (the plugin directory is written only after verification).
pub fn install_plugin(
    index_source: &str,
    name: &str,
    install_root: &Path,
) -> Result<PathBuf, PluginIndexError> {
    let index_str = if index_source.starts_with("https://") {
        let bytes = http_get(index_source)?;
        String::from_utf8(bytes)
            .map_err(|e| PluginIndexError::Parse(format!("index is not UTF-8: {e}")))?
    } else {
        std::fs::read_to_string(index_source)
            .map_err(|e| PluginIndexError::Fetch(format!("index {index_source}: {e}")))?
    };

    let entries = parse_plugin_index(&index_str)?;
    let entry = entries
        .iter()
        .find(|e| e.name == name)
        .ok_or_else(|| PluginIndexError::NotFound(name.to_string()))?;

    let wasm_bytes = fetch_artifact(index_source, entry)?;

    // Trust gate BEFORE writing: unknown signer, bad hash, bad signature,
    // or tampered bytes never reach the install root.
    verify_plugin_artifact(
        &entry.name,
        &wasm_bytes,
        &entry.wasm_hash,
        &entry.signature,
        &entry.signed_by,
    )?;

    let plugin_dir = install_root.join(sanitize_dir_name(&entry.name));
    std::fs::create_dir_all(&plugin_dir)?;
    std::fs::write(plugin_dir.join("plugin.wasm"), &wasm_bytes)?;

    let categories = entry
        .categories
        .iter()
        .map(|c| format!("\"{c}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let manifest = format!(
        "[plugin]\n\
         name = \"{}\"\n\
         version = \"{}\"\n\
         api_version = \"{}\"\n\
         author = \"{}\"\n\
         description = \"{}\"\n\
         license = \"{}\"\n\
         wasm_hash = \"{}\"\n\
         signature = \"{}\"\n\
         signed_by = \"{}\"\n\
         \n\
         [plugin.entry]\n\
         wasm = \"plugin.wasm\"\n\
         \n\
         [plugin.analyzer]\n\
         name = \"{}\"\n\
         categories = [{}]\n",
        escape_toml(&entry.name),
        escape_toml(&entry.version),
        escape_toml(&entry.api_version),
        escape_toml(&entry.author),
        escape_toml(&entry.description),
        escape_toml(&entry.license),
        entry.wasm_hash,
        entry.signature,
        entry.signed_by,
        escape_toml(&entry.name),
        categories,
    );
    std::fs::write(plugin_dir.join("crawlkit-plugin.toml"), manifest)?;

    Ok(plugin_dir)
}

/// List installed plugins under `install_root` (one subdirectory per
/// plugin). Skips directories without a parseable manifest.
pub fn list_installed_plugins(install_root: &Path) -> Vec<(String, String)> {
    let Ok(entries) = std::fs::read_dir(install_root) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let dir = entry.path();
        if let Ok(manifest_str) = std::fs::read_to_string(dir.join("crawlkit-plugin.toml")) {
            #[derive(Deserialize)]
            struct Minimal {
                plugin: MinimalPlugin,
            }
            #[derive(Deserialize)]
            struct MinimalPlugin {
                name: String,
                version: String,
            }
            if let Ok(m) = toml::from_str::<Minimal>(&manifest_str) {
                out.push((m.plugin.name, m.plugin.version));
            }
        }
    }
    out.sort();
    out
}

/// Directory names are derived from plugin names; the index validator
/// already restricts names to `[a-z0-9-]`, but defense in depth.
fn sanitize_dir_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Escape a string for a basic TOML value.
fn escape_toml(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
