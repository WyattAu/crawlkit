//! `crawlkit plugin` — WASM plugin lifecycle: scaffolding, signing, and
//! trust-chain management.
//!
//! Provides `new` (scaffold a complete plugin project), `keygen` (create an
//! ed25519 signing keypair), `sign` (hash + sign a plugin's `.wasm` and
//! record the trust fields in its manifest), `publish` (build + hash +
//! sign + append to a plugin index), and `verify` (run the same
//! verification the plugin loader performs).

use anyhow::{anyhow, Context, Result};
use clap::Subcommand;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use crawlkit_engine::plugin::{
    sign_plugin_wasm, validate_manifest, verify_plugin_dir, PluginManifest,
};
use crawlkit_engine::{
    install_plugin, list_installed_plugins, parse_plugin_index, PluginIndexError,
};

#[derive(Subcommand)]
pub enum PluginCommands {
    /// Scaffold a new plugin project
    New {
        /// Plugin name (kebab-case)
        name: String,

        /// Output directory (defaults to ./<name>)
        #[arg(short, long)]
        dir: Option<PathBuf>,
    },
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
    /// Install a plugin from an index (verifies hash + signature first)
    Install {
        /// Plugin name as listed in the index
        name: String,

        /// Path or https URL to plugin-index.toml (or CRAWLKIT_PLUGIN_INDEX)
        #[arg(long, short = 'i')]
        index: Option<String>,

        /// Install root (default: ~/.crawlkit/plugins)
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// List installed plugins (default root: ~/.crawlkit/plugins)
    List {
        /// Install root to list
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Remove an installed plugin
    Remove {
        /// Plugin name to remove
        name: String,

        /// Install root (default: ~/.crawlkit/plugins)
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Build, hash, sign, and publish a plugin
    Publish {
        /// Path to the plugin project directory
        path: PathBuf,

        /// Signing key (hex-encoded ed25519 seed)
        #[arg(long, env = "CRAWLKIT_SIGNING_KEY")]
        signing_key: String,

        /// Path to the plugin index to update
        #[arg(long, default_value = "plugins/index/plugin-index.toml")]
        index: PathBuf,
    },
}

/// Entry point for `crawlkit plugin <command>`.
pub fn run(command: PluginCommands) -> Result<()> {
    match command {
        PluginCommands::New { name, dir } => plugin_new(&name, dir.as_deref()),
        PluginCommands::Keygen { out, force } => keygen(&out, force),
        PluginCommands::Sign { plugin, key } => sign(&plugin, &key),
        PluginCommands::Verify { plugin } => verify(&plugin),
        PluginCommands::Install { name, index, root } => install(&name, index.as_deref(), root),
        PluginCommands::List { root } => list(root),
        PluginCommands::Remove { name, root } => remove(&name, root),
        PluginCommands::Publish {
            path,
            signing_key,
            index,
        } => plugin_publish(&path, &signing_key, &index),
    }
}

/// GitHub Actions workflow template shipped with scaffolded plugin projects.
const PLUGIN_CI_TEMPLATE: &str = include_str!("../../../../.github/workflows/plugin-ci.yml.tmpl");

/// `crawlkit plugin new` — scaffold a complete plugin project.
fn plugin_new(name: &str, dir: Option<&Path>) -> Result<()> {
    validate_plugin_name(name)?;
    let out = match dir {
        Some(d) => d.to_path_buf(),
        None => std::env::current_dir()
            .context("cannot determine current directory")?
            .join(name),
    };
    scaffold_plugin(name, &out)?;

    println!("Scaffolded plugin '{name}' in {}", out.display());
    println!();
    println!("Next steps:");
    println!("  cd {}", out.display());
    println!("  cargo test                                     # run the skeleton tests");
    println!("  cargo build --release --target wasm32-unknown-unknown");
    println!("  crawlkit plugin publish .                      # build + sign + publish");
    println!();
    println!("Then edit src/lib.rs and crawlkit-plugin.toml to implement your checks.");
    Ok(())
}

/// Validate a plugin name: kebab-case ASCII (crate name + manifest name rules).
fn validate_plugin_name(name: &str) -> Result<()> {
    let valid = !name.is_empty()
        && name.len() <= 64
        && name.chars().next().is_some_and(|c| c.is_ascii_lowercase())
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !name.ends_with('-');
    if !valid {
        return Err(anyhow!(
            "invalid plugin name '{name}': use kebab-case (lowercase letters, digits, hyphens; \
             must start with a letter)"
        ));
    }
    Ok(())
}

/// Write the full plugin project skeleton into `out`.
fn scaffold_plugin(name: &str, out: &Path) -> Result<()> {
    if out.exists() {
        let non_empty = std::fs::read_dir(out)
            .map(|mut entries| entries.next().is_some())
            .unwrap_or(true);
        if non_empty {
            return Err(anyhow!(
                "output directory {} already exists and is not empty",
                out.display()
            ));
        }
    }

    let src = out.join("src");
    let workflows = out.join(".github").join("workflows");
    std::fs::create_dir_all(&src).with_context(|| format!("failed to create {}", src.display()))?;
    std::fs::create_dir_all(&workflows)
        .with_context(|| format!("failed to create {}", workflows.display()))?;

    let files: [(&Path, String); 6] = [
        (&out.join("Cargo.toml"), scaffold_cargo_toml(name)),
        (&out.join("src").join("lib.rs"), scaffold_lib_rs(name)),
        (&out.join("crawlkit-plugin.toml"), scaffold_manifest(name)),
        (&out.join("README.md"), scaffold_readme(name)),
        (&out.join(".gitignore"), "target/\n".to_string()),
        (
            &workflows.join("plugin-ci.yml"),
            PLUGIN_CI_TEMPLATE.to_string(),
        ),
    ];
    for (path, contents) in &files {
        std::fs::write(path, contents)
            .with_context(|| format!("failed to write {}", path.display()))?;
    }
    Ok(())
}

/// Generate the scaffolded plugin's `Cargo.toml`.
fn scaffold_cargo_toml(name: &str) -> String {
    format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"
rust-version = "1.94"
authors = ["Your Name"]
license = "Apache-2.0"
description = "crawlkit WASM plugin: TODO one-line description"
publish = false

[lib]
crate-type = ["cdylib", "rlib"]

[lints.rust]
unsafe_code = "allow" # export_analyzer! emits the raw WASM ABI surface

[lints.clippy]
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
exit = "deny"

[dependencies]
crawlkit-plugin-sdk = {{ git = "https://github.com/WyattAu/crawlkit" }}
serde_json = "1"
"#
    )
}

/// Generate the scaffolded plugin's `src/lib.rs` — a working `<title>`
/// checker skeleton with tests.
fn scaffold_lib_rs(name: &str) -> String {
    format!(
        r#"//! {name}: a crawlkit WASM plugin.
//!
//! TODO: describe what this plugin checks. Plugins implement the
//! [`Analyzer`] trait and are exported via [`export_analyzer!`]; the host
//! compiles this crate to `wasm32-unknown-unknown` and runs it in a
//! sandboxed wasmtime runtime.

use crawlkit_plugin_sdk::{{AnalysisContext, Analyzer, Finding, IssueCategory, Severity}};

/// TODO: rename this analyzer to something descriptive.
pub struct MyPlugin;

impl MyPlugin {{
    pub fn new() -> Self {{
        Self
    }}
}}

impl Default for MyPlugin {{
    fn default() -> Self {{
        Self::new()
    }}
}}

impl Analyzer for MyPlugin {{
    fn name(&self) -> &str {{
        "{name}"
    }}

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {{
        // TODO: replace this skeleton with your own checks. This example
        // flags pages that are missing a <title> element.
        if ctx.html.to_lowercase().contains("<title") {{
            return Vec::new();
        }}
        vec![Finding {{
            severity: Severity::Warning,
            category: IssueCategory::Seo,
            code: "PLUGIN001".into(),
            title: "Missing <title> tag".into(),
            description: "The page does not contain a <title> element, so \
                          search engines will invent their own headline."
                .into(),
            url: ctx.url.clone(),
            recommendation: "Add a concise <title> (roughly 30-60 characters) \
                             inside the <head> section."
                .into(),
        }}]
    }}
}}

crawlkit_plugin_sdk::export_analyzer!(MyPlugin);

#[cfg(test)]
mod tests {{
    use super::*;

    fn analyze(html: &str) -> Vec<Finding> {{
        MyPlugin.analyze(&AnalysisContext {{
            url: "https://example.com".into(),
            html: html.into(),
            status_code: Some(200),
            headers: Vec::new(),
            response_time_ms: None,
        }})
    }}

    #[test]
    fn missing_title_is_flagged() {{
        let findings = analyze("<html><head></head><body>hi</body></html>");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "PLUGIN001");
        assert_eq!(findings[0].severity, Severity::Warning);
    }}

    #[test]
    fn present_title_is_clean() {{
        let html = "<html><head><title>Hello</title></head></html>";
        assert!(analyze(html).is_empty());
    }}

    #[test]
    fn analyzer_name_matches_manifest() {{
        assert_eq!(MyPlugin.name(), "{name}");
    }}
}}
"#
    )
}

/// Generate the scaffolded plugin's `crawlkit-plugin.toml` manifest.
///
/// The `wasm` entry points at the release wasm32 artifact that
/// `crawlkit plugin publish` (and the manual build command) produce.
fn scaffold_manifest(name: &str) -> String {
    let wasm_artifact = wasm_artifact_path(name);
    format!(
        r#"# crawlkit plugin manifest — read by `crawlkit plugin publish` and the loader.
[plugin]
name = "{name}"
version = "0.1.0"
api_version = "1.0"
author = "Your Name"
description = "TODO: what this plugin checks"
license = "Apache-2.0"
categories = ["seo"]

[plugin.entry]
# Produced by: cargo build --release --target wasm32-unknown-unknown
wasm = "{wasm_artifact}"

[plugin.analyzer]
name = "{name}"
category = "seo"
"#
    )
}

/// Generate the scaffolded plugin's `README.md`.
fn scaffold_readme(name: &str) -> String {
    format!(
        r#"# {name}

A [crawlkit](https://github.com/WyattAu/crawlkit) WASM plugin.
TODO: describe what this plugin checks.

## Layout

- `src/lib.rs` — the analyzer implementation (`export_analyzer!` exports it
  behind the plugin ABI)
- `crawlkit-plugin.toml` — plugin manifest (name, version, entry point);
  `crawlkit plugin publish` reads it and records the built artifact's hash
  and signature
- `.github/workflows/plugin-ci.yml` — CI: tests, wasm32 build, and signed
  publishing on GitHub Releases (set the `CRAWLKIT_SIGNING_KEY` secret)

## Development

```sh
cargo test
cargo build --release --target wasm32-unknown-unknown
```

## Publish

```sh
# One-time: create a signing keypair
crawlkit plugin keygen --out .keys

# Build + hash + sign + append to a plugin index
CRAWLKIT_SIGNING_KEY=$(cat .keys/plugin-signing.key) \
  crawlkit plugin publish . --index dist/plugin-index.toml
```

Users install the published plugin with:

```sh
crawlkit plugin install {name} --index <index-url-or-path>
```
"#
    )
}

/// Default wasm32 release artifact path for a crate named `name`,
/// relative to the plugin project directory.
fn wasm_artifact_path(name: &str) -> String {
    format!(
        "target/wasm32-unknown-unknown/release/{}.wasm",
        name.replace('-', "_")
    )
}

/// `crawlkit plugin publish` — build, hash, sign, and publish a plugin.
///
/// The full chain: `cargo build --release --target wasm32-unknown-unknown`
/// in the plugin directory, SHA-256 + ed25519 over the built `.wasm`,
/// metadata from `crawlkit-plugin.toml`, artifact copied into the index's
/// `artifacts/` directory, and a `[[plugin]]` entry appended to the index
/// TOML.
fn plugin_publish(path: &Path, signing_key: &str, index_path: &Path) -> Result<()> {
    // 5. Parse the plugin manifest for metadata (before building, so a
    // broken manifest fails fast).
    let manifest_path = path.join("crawlkit-plugin.toml");
    let manifest_str = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("failed to read manifest {}", manifest_path.display()))?;
    let manifest: PluginManifest = toml::from_str(&manifest_str)
        .with_context(|| format!("failed to parse manifest {}", manifest_path.display()))?;
    validate_manifest(&manifest.plugin)
        .map_err(|e| anyhow!("invalid manifest {}: {e}", manifest_path.display()))?;
    let categories = manifest_categories(&manifest_str)?;
    let name = manifest.plugin.name.clone();
    let version = manifest.plugin.version.clone();

    // 1. Build for wasm32 (release).
    println!("Building '{name}' for wasm32-unknown-unknown (release)...");
    let status = std::process::Command::new("cargo")
        .current_dir(path)
        .args(["build", "--release", "--target", "wasm32-unknown-unknown"])
        .status()
        .context("failed to run cargo (is it installed and on PATH?)")?;
    if !status.success() {
        return Err(anyhow!("cargo build failed for {}", path.display()));
    }

    // 2. Read the built .wasm.
    let wasm_rel = manifest
        .plugin
        .entry
        .wasm
        .clone()
        .unwrap_or_else(|| wasm_artifact_path(&name));
    let wasm_path = path.join(&wasm_rel);
    let wasm_bytes = std::fs::read(&wasm_path)
        .with_context(|| format!("failed to read built artifact {}", wasm_path.display()))?;

    // 3./4. Compute the SHA-256 hash and sign it with the ed25519 key.
    let seed: [u8; 32] = hex_decode(signing_key.trim())
        .and_then(|bytes| <[u8; 32]>::try_from(bytes).ok())
        .ok_or_else(|| anyhow!("signing key must be 64 hex characters (32 bytes)"))?;
    let (wasm_hash, signature, signed_by) = sign_plugin_wasm(&wasm_bytes, &seed);

    // 7. Copy the artifact into the index's artifacts/ directory.
    let index_dir = index_path.parent().unwrap_or(Path::new("."));
    let artifacts_dir = index_dir.join("artifacts");
    std::fs::create_dir_all(&artifacts_dir)
        .with_context(|| format!("failed to create {}", artifacts_dir.display()))?;
    let artifact_name = format!("{name}-{version}.wasm");
    let artifact_path = artifacts_dir.join(&artifact_name);
    std::fs::copy(&wasm_path, &artifact_path)
        .with_context(|| format!("failed to copy artifact to {}", artifact_path.display()))?;

    // 6. Append the [[plugin]] entry to the index.
    let entry = render_index_entry(&IndexEntry {
        name: &name,
        version: &version,
        api_version: &manifest.plugin.api_version,
        author: &manifest.plugin.author,
        description: &manifest.plugin.description,
        license: &manifest.plugin.license,
        categories: &categories,
        wasm_path: &format!("artifacts/{artifact_name}"),
        wasm_hash: &wasm_hash,
        signature: &signature,
        signed_by: &signed_by,
    });
    append_index_entry(index_path, &entry, &name, &version)?;

    println!("Published '{name}' {version}");
    println!("  artifact:  {}", artifact_path.display());
    println!("  wasm_hash: {wasm_hash}");
    println!("  signed_by: {signed_by}");
    println!("  index:     {}", index_path.display());
    println!(
        "Install it with: crawlkit plugin install {name} --index {}",
        index_path.display()
    );
    Ok(())
}

/// One `[[plugin]]` index entry, ready to be rendered into TOML.
struct IndexEntry<'a> {
    name: &'a str,
    version: &'a str,
    api_version: &'a str,
    author: &'a str,
    description: &'a str,
    license: &'a str,
    categories: &'a [String],
    wasm_path: &'a str,
    wasm_hash: &'a str,
    signature: &'a str,
    signed_by: &'a str,
}

/// Render an index entry in the canonical `[[plugin]]` table format.
fn render_index_entry(entry: &IndexEntry<'_>) -> String {
    let categories = entry
        .categories
        .iter()
        .map(|c| format!("\"{}\"", toml_escape(c)))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "[[plugin]]\n\
         name = \"{}\"\n\
         version = \"{}\"\n\
         api_version = \"{}\"\n\
         author = \"{}\"\n\
         description = \"{}\"\n\
         license = \"{}\"\n\
         categories = [{categories}]\n\
         wasm_path = \"{}\"\n\
         wasm_hash = \"{}\"\n\
         signature = \"{}\"\n\
         signed_by = \"{}\"\n",
        toml_escape(entry.name),
        toml_escape(entry.version),
        toml_escape(entry.api_version),
        toml_escape(entry.author),
        toml_escape(entry.description),
        toml_escape(entry.license),
        toml_escape(entry.wasm_path),
        toml_escape(entry.wasm_hash),
        toml_escape(entry.signature),
        toml_escape(entry.signed_by),
    )
}

/// Escape a string for inclusion in a TOML basic string.
fn toml_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Read the optional `[plugin] categories` array from a manifest.
///
/// The canonical [`PluginManifest`] type does not carry categories (the
/// loader does not need them), but index entries do; scaffolded manifests
/// include the field for exactly this purpose.
fn manifest_categories(manifest_str: &str) -> Result<Vec<String>> {
    #[derive(serde::Deserialize, Default)]
    struct Probe {
        #[serde(default)]
        plugin: PluginTable,
    }
    #[derive(serde::Deserialize, Default)]
    struct PluginTable {
        #[serde(default)]
        categories: Option<Vec<String>>,
    }

    let probe: Probe = toml::from_str(manifest_str).context("failed to parse manifest")?;
    Ok(probe
        .plugin
        .categories
        .unwrap_or_else(|| vec!["custom".to_string()]))
}

/// Append a rendered `[[plugin]]` entry to the index file (creating it and
/// its parent directories as needed).
///
/// Refuses to append when the index already contains an entry with the
/// same name and version — republishing an identical version would
/// silently break content addressing for existing users.
fn append_index_entry(index_path: &Path, entry: &str, name: &str, version: &str) -> Result<()> {
    let existing = match std::fs::read_to_string(index_path) {
        Ok(content) => Some(content),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => {
            return Err(anyhow::Error::new(e)
                .context(format!("failed to read index {}", index_path.display())))
        }
    };

    if let Some(content) = &existing {
        let entries = parse_plugin_index(content)
            .map_err(|e| anyhow!("failed to parse index {}: {e}", index_path.display()))?;
        if entries
            .iter()
            .any(|e| e.name == name && e.version == version)
        {
            return Err(anyhow!(
                "index {} already contains {name} {version}; bump the version in \
                 crawlkit-plugin.toml to publish a new release",
                index_path.display()
            ));
        }
    }

    let mut updated = existing
        .map(|c| c.trim_end().to_string())
        .unwrap_or_default();
    if !updated.is_empty() {
        updated.push_str("\n\n");
    }
    updated.push_str(entry);
    updated.push('\n');

    if let Some(parent) = index_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("failed to create index directory {}", parent.display())
            })?;
        }
    }
    std::fs::write(index_path, &updated)
        .with_context(|| format!("failed to write index {}", index_path.display()))?;
    Ok(())
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
#[allow(unknown_lints, clippy::chunks_exact_to_as_chunks)]
fn hex_decode(s: &str) -> Option<Vec<u8>> {
    if s.len() & 1 != 0 {
        return None;
    }
    s.as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok())
        .collect()
}

/// Default install root: ~/.crawlkit/plugins
fn default_install_root() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("cannot determine home directory"))?;
    Ok(home.join(".crawlkit").join("plugins"))
}

/// `crawlkit plugin install`
fn install(name: &str, index: Option<&str>, root: Option<PathBuf>) -> Result<()> {
    let index_source = index
        .map(str::to_string)
        .or_else(|| std::env::var("CRAWLKIT_PLUGIN_INDEX").ok())
        .ok_or_else(|| {
            anyhow!("no index specified: pass --index <path-or-url> or set CRAWLKIT_PLUGIN_INDEX")
        })?;
    let root = match root {
        Some(r) => r,
        None => default_install_root()?,
    };

    println!("Installing plugin '{name}' from {index_source}...");
    let plugin_dir =
        install_plugin(&index_source, name, &root).map_err(|e| anyhow!("install failed: {e}"))?;

    // Post-install proof: the loader must accept what we just wrote under
    // the strictest policy.
    let metadata = verify_plugin_dir(&plugin_dir)
        .map_err(|e| anyhow!("post-install verification failed (removing): {e}"))?;

    println!(
        "Installed '{name}' {} to {}",
        metadata.version,
        plugin_dir.display()
    );
    println!(
        "  signed_by: {}",
        metadata.signed_by.as_deref().unwrap_or("(unsigned)")
    );
    Ok(())
}

/// `crawlkit plugin list`
fn list(root: Option<PathBuf>) -> Result<()> {
    let root = match root {
        Some(r) => r,
        None => default_install_root()?,
    };
    let installed = list_installed_plugins(&root);
    if installed.is_empty() {
        println!("No plugins installed under {}", root.display());
        return Ok(());
    }
    println!("Installed plugins ({}):", root.display());
    for (name, version) in installed {
        println!("  {name} {version}");
    }
    Ok(())
}

/// `crawlkit plugin remove`
fn remove(name: &str, root: Option<PathBuf>) -> Result<()> {
    let root = match root {
        Some(r) => r,
        None => default_install_root()?,
    };
    let dir = root.join(name);
    if !dir.join("crawlkit-plugin.toml").exists() {
        return Err(anyhow!("no plugin '{name}' under {}", root.display()));
    }
    std::fs::remove_dir_all(&dir).with_context(|| format!("failed to remove {}", dir.display()))?;
    println!("Removed '{name}'");
    Ok(())
}

/// Convert index errors with context (kept for future richer handling).
#[allow(dead_code)]
fn explain(e: PluginIndexError) -> String {
    e.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A unique scratch directory under the OS temp dir (no tempfile dep).
    fn scratch(label: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default();
        std::env::temp_dir().join(format!("crawlkit-{label}-{}-{nanos}", std::process::id()))
    }

    #[test]
    fn plugin_name_validation_accepts_kebab_case() {
        assert!(validate_plugin_name("my-plugin").is_ok());
        assert!(validate_plugin_name("a").is_ok());
        assert!(validate_plugin_name("head1ng-check").is_ok());
    }

    #[test]
    fn plugin_name_validation_rejects_bad_names() {
        for name in [
            "",
            "My-Plugin",
            "1leading-digit",
            "-leading-hyphen",
            "trailing-",
            "has space",
            "has_underscore",
        ] {
            assert!(validate_plugin_name(name).is_err(), "accepted '{name}'");
        }
    }

    #[test]
    fn scaffold_creates_complete_project() {
        let out = scratch("scaffold");
        scaffold_plugin("my-checker", &out).expect("scaffold");

        for file in [
            "Cargo.toml",
            "src/lib.rs",
            "crawlkit-plugin.toml",
            "README.md",
            ".gitignore",
            ".github/workflows/plugin-ci.yml",
        ] {
            assert!(out.join(file).is_file(), "missing {file}");
        }

        let cargo = std::fs::read_to_string(out.join("Cargo.toml")).unwrap();
        assert!(cargo.contains("name = \"my-checker\""));
        assert!(cargo.contains("cdylib"));

        let lib = std::fs::read_to_string(out.join("src/lib.rs")).unwrap();
        assert!(lib.contains("use crawlkit_plugin_sdk::{"));
        assert!(lib.contains("impl Analyzer for MyPlugin"));
        assert!(lib.contains("export_analyzer!(MyPlugin)"));
        assert!(lib.contains("mod tests"));
        assert!(lib.contains("fn missing_title_is_flagged"));

        let manifest_str = std::fs::read_to_string(out.join("crawlkit-plugin.toml")).unwrap();
        let manifest: PluginManifest = toml::from_str(&manifest_str).unwrap();
        validate_manifest(&manifest.plugin).expect("scaffolded manifest must be valid");
        assert_eq!(manifest.plugin.name, "my-checker");
        assert_eq!(manifest.plugin.version, "0.1.0");
        assert_eq!(
            manifest.plugin.entry.wasm.as_deref(),
            Some("target/wasm32-unknown-unknown/release/my_checker.wasm")
        );

        assert!(std::fs::read_to_string(out.join(".gitignore"))
            .unwrap()
            .contains("target/"));
        assert!(
            std::fs::read_to_string(out.join(".github/workflows/plugin-ci.yml"))
                .unwrap()
                .contains("plugin publish")
        );

        std::fs::remove_dir_all(&out).ok();
    }

    #[test]
    fn scaffold_refuses_existing_non_empty_dir() {
        let out = scratch("scaffold-conflict");
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(out.join("occupied.txt"), b"x").unwrap();
        assert!(scaffold_plugin("my-checker", &out).is_err());
        std::fs::remove_dir_all(&out).ok();
    }

    #[test]
    fn scaffold_allows_existing_empty_dir() {
        let out = scratch("scaffold-empty");
        std::fs::create_dir_all(&out).unwrap();
        assert!(scaffold_plugin("my-checker", &out).is_ok());
        std::fs::remove_dir_all(&out).ok();
    }

    #[test]
    fn index_entry_roundtrips_through_parser() {
        let entry = render_index_entry(&IndexEntry {
            name: "my-checker",
            version: "0.2.0",
            api_version: "1.0",
            author: "Someone",
            description: "Checks things, with \"quotes\" and a \\ backslash",
            license: "MIT",
            categories: &["seo".to_string(), "content".to_string()],
            wasm_path: "artifacts/my-checker-0.2.0.wasm",
            wasm_hash: "abc123",
            signature: "def456",
            signed_by: "0123456789abcdef",
        });
        let entries = parse_plugin_index(&entry).expect("rendered entry must parse");
        assert_eq!(entries.len(), 1);
        let parsed = &entries[0];
        assert_eq!(parsed.name, "my-checker");
        assert_eq!(parsed.version, "0.2.0");
        assert_eq!(
            parsed.categories,
            vec!["seo".to_string(), "content".to_string()]
        );
        assert_eq!(
            parsed.description,
            "Checks things, with \"quotes\" and a \\ backslash"
        );
        assert_eq!(parsed.wasm_hash, "abc123");
        assert_eq!(parsed.signed_by, "0123456789abcdef");
    }

    #[test]
    fn append_creates_index_and_rejects_duplicate_versions() {
        let dir = scratch("append");
        let index_path = dir.join("nested").join("plugin-index.toml");
        let entry = render_index_entry(&IndexEntry {
            name: "my-checker",
            version: "1.0.0",
            api_version: "1.0",
            author: "Someone",
            description: "d",
            license: "MIT",
            categories: &["seo".to_string()],
            wasm_path: "artifacts/my-checker-1.0.0.wasm",
            wasm_hash: "h1",
            signature: "s1",
            signed_by: "k1",
        });

        append_index_entry(&index_path, &entry, "my-checker", "1.0.0").expect("first publish");
        let content = std::fs::read_to_string(&index_path).unwrap();
        assert_eq!(parse_plugin_index(&content).unwrap().len(), 1);

        // Same name + version is refused.
        assert!(append_index_entry(&index_path, &entry, "my-checker", "1.0.0").is_err());

        // A new version appends alongside the existing entry.
        let v2 = entry.replace("1.0.0", "1.1.0");
        append_index_entry(&index_path, &v2, "my-checker", "1.1.0").expect("second publish");
        let content = std::fs::read_to_string(&index_path).unwrap();
        let entries = parse_plugin_index(&content).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].version, "1.0.0");
        assert_eq!(entries[1].version, "1.1.0");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn manifest_categories_default_to_custom() {
        let with = manifest_categories("[plugin]\nname = \"x\"\ncategories = [\"seo\"]\n").unwrap();
        assert_eq!(with, vec!["seo".to_string()]);
        let without = manifest_categories("[plugin]\nname = \"x\"\n").unwrap();
        assert_eq!(without, vec!["custom".to_string()]);
    }
}
