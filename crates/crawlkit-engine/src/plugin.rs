use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Plugin trust-chain verification policy.
///
/// Controls how [`WasmConfig`] treats the manifest's `wasm_hash` /
/// `signature` / `signed_by` trust fields during plugin loading.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PluginVerification {
    /// Require a valid `wasm_hash` plus an ed25519 `signature` made by a
    /// key in [`TRUSTED_PLUGIN_KEYS`]. Missing or invalid trust data
    /// rejects the plugin (fail-closed). This is the default.
    #[default]
    Required,
    /// Permit plugins without trust metadata (logged via `tracing::warn!`),
    /// while still rejecting any *present* hash or signature that fails
    /// verification — bad crypto is always fail-closed.
    AllowUnsigned,
}

/// Security configuration for WASM plugin execution.
#[derive(Clone)]
pub struct WasmConfig {
    /// Maximum fuel (instructions) a WASM plugin may consume before being
    /// killed. Prevents infinite loops / CPU exhaustion.
    pub max_fuel: u64,
    /// Maximum bytes the WASM linear memory may grow to.
    pub max_memory_bytes: usize,
    /// Wall-clock timeout for a single `analyze` call in milliseconds,
    /// enforced via wasmtime epoch interruption. A plugin that exceeds it
    /// traps with a deadline error instead of running indefinitely.
    pub max_analysis_timeout_ms: u64,
    /// Trust-chain policy applied to the manifest's hash/signature fields.
    pub plugin_verification: PluginVerification,
    /// When true, plugins whose manifest declares `permissions.network = true`
    /// may call the host `crawlkit_host.fetch` function (SSRF-validated,
    /// redirect-free, 1 MiB cap, 10 s timeout). The default is false:
    /// network access is deny-by-default even if the manifest requests it.
    pub allow_plugin_network: bool,
}

impl Default for WasmConfig {
    fn default() -> Self {
        Self {
            // ~10 billion instructions – generous for legitimate analysis
            // but prevents runaway loops.
            max_fuel: 10_000_000_000,
            // 64 MiB – sufficient for HTML processing without allowing
            // memory-bomb attacks.
            max_memory_bytes: 64 * 1024 * 1024,
            // 30 seconds per analysis call.
            max_analysis_timeout_ms: 30_000,
            // Fail-closed by default: unsigned/untrusted plugins are
            // rejected unless the embedder explicitly opts out.
            plugin_verification: PluginVerification::Required,
            // Network access deny-by-default; must be explicitly enabled
            // by the embedder in addition to the manifest declaring it.
            allow_plugin_network: false,
        }
    }
}

/// Plugin errors.
#[derive(Debug, Error)]
pub enum PluginError {
    #[error("plugin not found: {0}")]
    NotFound(String),

    #[error("plugin load failed: {0}")]
    LoadFailed(String),

    #[error("plugin init failed: {0}")]
    InitFailed(String),

    #[error("plugin analysis failed: {0}")]
    AnalysisFailed(String),

    #[error("incompatible API version: {0} (expected 1.0)")]
    IncompatibleApiVersion(String),

    #[error("manifest parse error: {0}")]
    ManifestParse(String),

    #[error("WASM execution error: {0}")]
    WasmExecution(String),

    #[error("invalid manifest: {0}")]
    InvalidManifest(String),
}

/// Errors specific to plugin manifest validation.
#[derive(Debug, Error, PartialEq)]
pub enum ManifestError {
    #[error("name is required and must be non-empty")]
    NameRequired,

    #[error("name must contain only alphanumeric characters and hyphens")]
    NameInvalid,

    #[error("version is required and must be non-empty")]
    VersionRequired,

    #[error("version must be valid semver (X.Y.Z)")]
    VersionInvalid,

    #[error("description is required and must be non-empty")]
    DescriptionRequired,

    #[error("description exceeds maximum length of 500 characters")]
    DescriptionTooLong,

    #[error("author is required and must be non-empty")]
    AuthorRequired,

    #[error("license is required and must be non-empty")]
    LicenseRequired,

    #[error("license must be a valid SPDX identifier")]
    LicenseInvalid,

    #[error("entry_point (wasm) is required and must be non-empty")]
    EntryPointRequired,

    #[error("entry_point must end with .wasm")]
    EntryPointNotWasm,
}

/// Plugin manifest (crawlkit-plugin.toml).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub plugin: PluginMetadata,
}

/// Plugin metadata from manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub api_version: String,
    pub author: String,
    pub description: String,
    pub license: String,
    pub trust_level: Option<String>,
    pub entry: PluginEntry,
    pub permissions: Option<PluginPermissions>,
    pub analyzer: Option<PluginAnalyzerInfo>,
    /// Hex sha256 digest of the `.wasm` file this manifest describes.
    pub wasm_hash: Option<String>,
    /// Hex ed25519 signature over the raw 32-byte sha256 digest (not over
    /// the hex string). Verified against [`TRUSTED_PLUGIN_KEYS`].
    pub signature: Option<String>,
    /// Key id of the signer — the first 16 hex characters of the signer's
    /// ed25519 public key.
    pub signed_by: Option<String>,
}

/// Plugin entry point configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginEntry {
    pub wasm: Option<String>,
    pub native: Option<String>,
}

/// WASM plugin permissions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginPermissions {
    pub network: Option<bool>,
    pub filesystem: Option<bool>,
    pub env_vars: Option<Vec<String>>,
}

/// Plugin analyzer metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginAnalyzerInfo {
    pub name: String,
    pub category: Option<String>,
    pub description: Option<String>,
    pub severity: Option<String>,
}

/// Validate a version string against semver format (X.Y.Z).
///
/// Accepts major.minor.patch where each component is a non-negative integer
/// with no leading zeros (except "0" itself).
pub fn validate_version(version: &str) -> bool {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() != 3 {
        return false;
    }
    for part in &parts {
        if part.is_empty() || !part.chars().all(|c| c.is_ascii_digit()) {
            return false;
        }
        // Reject leading zeros (except "0" itself)
        if part.len() > 1 && part.starts_with('0') {
            return false;
        }
    }
    true
}

/// Validate a license string against common SPDX identifiers.
///
/// Checks a curated list of widely-used open-source licenses. Returns `true`
/// if the identifier matches one of the known licenses (case-sensitive).
pub fn validate_license(license: &str) -> bool {
    matches!(
        license,
        "MIT"
            | "Apache-2.0"
            | "Apache-2.0 OR MIT"
            | "BSD-2-Clause"
            | "BSD-3-Clause"
            | "GPL-2.0"
            | "GPL-2.0-only"
            | "GPL-2.0-or-later"
            | "GPL-3.0"
            | "GPL-3.0-only"
            | "GPL-3.0-or-later"
            | "LGPL-2.0"
            | "LGPL-2.0-only"
            | "LGPL-2.0-or-later"
            | "LGPL-2.1"
            | "LGPL-2.1-only"
            | "LGPL-2.1-or-later"
            | "LGPL-3.0"
            | "LGPL-3.0-only"
            | "LGPL-3.0-or-later"
            | "MPL-2.0"
            | "ISC"
            | "Unlicense"
            | "0BSD"
            | "CC0-1.0"
            | "CC-BY-4.0"
            | "CC-BY-SA-4.0"
            | "WTFPL"
            | "Zlib"
            | "BSL-1.0"
            | "PostgreSQL"
            | "Python-2.0"
            | "PSF-2.0"
            | "AGPL-3.0"
            | "AGPL-3.0-only"
            | "AGPL-3.0-or-later"
            | "EUPL-1.1"
            | "EUPL-1.2"
            | "CECILL-2.0"
            | "Artistic-2.0"
            | "EPL-1.0"
            | "EPL-2.0"
            | "CDDL-1.0"
            | "CDDL-1.1"
            | "CPL-1.0"
            | "IPL-1.0"
            | "OFL-1.1"
            | "RSA-MD"
            | "curl"
            | "libpng"
            | "boost"
            | "FPL"
    )
}

/// Validate a plugin manifest against all required field rules.
///
/// Checks that all mandatory fields are present, non-empty, and conform
/// to their format constraints. Returns `Ok(())` on success or
/// `Err(ManifestError)` describing the first validation failure.
pub fn validate_manifest(manifest: &PluginMetadata) -> Result<(), ManifestError> {
    // Name validation
    if manifest.name.is_empty() {
        return Err(ManifestError::NameRequired);
    }
    if !manifest
        .name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        return Err(ManifestError::NameInvalid);
    }

    // Version validation
    if manifest.version.is_empty() {
        return Err(ManifestError::VersionRequired);
    }
    if !validate_version(&manifest.version) {
        return Err(ManifestError::VersionInvalid);
    }

    // Description validation
    if manifest.description.is_empty() {
        return Err(ManifestError::DescriptionRequired);
    }
    if manifest.description.len() > 500 {
        return Err(ManifestError::DescriptionTooLong);
    }

    // Author validation
    if manifest.author.is_empty() {
        return Err(ManifestError::AuthorRequired);
    }

    // License validation
    if manifest.license.is_empty() {
        return Err(ManifestError::LicenseRequired);
    }
    if !validate_license(&manifest.license) {
        return Err(ManifestError::LicenseInvalid);
    }

    // Entry point validation
    match manifest.entry.wasm.as_deref() {
        None => return Err(ManifestError::EntryPointRequired),
        Some("") => return Err(ManifestError::EntryPointRequired),
        Some(entry) if !entry.ends_with(".wasm") => return Err(ManifestError::EntryPointNotWasm),
        _ => {}
    }

    Ok(())
}

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
///
/// The single key below is the default first-party development key
/// (also used by the engine's test fixtures); rotate it before
/// marketplace launch.
pub const TRUSTED_PLUGIN_KEYS: &[TrustedPluginKey] = &[TrustedPluginKey {
    key_id: "1f299a0020f6ae90",
    public_key_hex: "1f299a0020f6ae90413db4aed7aea95299632550cc483be1a9f46ce3296a051e",
}];

/// Encode bytes as lowercase hex.
fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// Decode a lowercase-or-uppercase hex string; `None` on malformed input.
fn hex_decode(s: &str) -> Option<Vec<u8>> {
    if s.len() & 1 != 0 {
        return None;
    }
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
fn verify_plugin_trust(
    metadata: &PluginMetadata,
    wasm_bytes: &[u8],
    policy: &PluginVerification,
) -> Result<(), PluginError> {
    let digest = Sha256::digest(wasm_bytes);
    let actual_hash = hex_encode(&digest);

    // Any declared wasm_hash must match the bytes on disk — a mismatch
    // means the binary was tampered with (or the manifest is stale).
    if let Some(declared) = metadata.wasm_hash.as_deref() {
        if !declared.eq_ignore_ascii_case(&actual_hash) {
            return Err(PluginError::InvalidManifest(format!(
                "wasm_hash mismatch for plugin '{}': manifest declares {declared} but the .wasm hashes to {actual_hash}",
                metadata.name
            )));
        }
    }

    let signature = metadata.signature.as_deref();
    let signed_by = metadata.signed_by.as_deref();
    if signature.is_some() != signed_by.is_some() {
        return Err(PluginError::InvalidManifest(format!(
            "plugin '{}' declares signature and signed_by individually; both must be present together",
            metadata.name
        )));
    }

    let verify = || -> Result<(), String> {
        match (signature, signed_by) {
            (Some(sig), Some(signer)) => verify_ed25519_signature(&digest, sig, signer),
            _ => {
                tracing::warn!("unsigned plugin loaded: {}", metadata.name);
                Ok(())
            }
        }
    };

    match policy {
        PluginVerification::Required => {
            if metadata.wasm_hash.is_none() {
                return Err(PluginError::InvalidManifest(format!(
                    "missing wasm_hash for plugin '{}' (verification policy: required)",
                    metadata.name
                )));
            }
            if signature.is_none() {
                return Err(PluginError::InvalidManifest(format!(
                    "missing signature/signed_by for plugin '{}' (verification policy: required)",
                    metadata.name
                )));
            }
            verify().map_err(|reason| {
                PluginError::InvalidManifest(format!(
                    "signature verification failed for plugin '{}': {reason}",
                    metadata.name
                ))
            })
        }
        PluginVerification::AllowUnsigned => verify().map_err(|reason| {
            PluginError::InvalidManifest(format!(
                "signature verification failed for plugin '{}': {reason}",
                metadata.name
            ))
        }),
    }
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

/// Read and parse a plugin manifest (`crawlkit-plugin.toml`).
fn read_plugin_manifest(plugin_dir: &Path) -> Result<PluginManifest, PluginError> {
    let manifest_path = plugin_dir.join("crawlkit-plugin.toml");
    let manifest_str = std::fs::read_to_string(&manifest_path)
        .map_err(|e| PluginError::ManifestParse(format!("Failed to read manifest: {e}")))?;
    toml::from_str(&manifest_str)
        .map_err(|e| PluginError::ManifestParse(format!("Invalid manifest: {e}")))
}

/// Verify a plugin directory's trust chain exactly as the loader does
/// under [`PluginVerification::Required`], without compiling or
/// instantiating the module. Returns the verified metadata on success.
///
/// This is the check exposed to `crawlkit plugin verify`.
pub fn verify_plugin_dir(plugin_dir: &Path) -> Result<PluginMetadata, PluginError> {
    let manifest = read_plugin_manifest(plugin_dir)?;
    validate_manifest(&manifest.plugin).map_err(|e| PluginError::InvalidManifest(e.to_string()))?;

    let wasm_file = manifest
        .plugin
        .entry
        .wasm
        .as_deref()
        .ok_or_else(|| PluginError::LoadFailed("No WASM entry point specified".to_string()))?;
    let wasm_bytes = std::fs::read(plugin_dir.join(wasm_file))
        .map_err(|e| PluginError::LoadFailed(format!("Failed to read WASM file: {e}")))?;

    verify_plugin_trust(&manifest.plugin, &wasm_bytes, &PluginVerification::Required)?;
    Ok(manifest.plugin)
}

/// Loaded WASM plugin instance.
pub struct WasmPlugin {
    pub manifest: PluginMetadata,
    config: WasmConfig,
    engine: wasmtime::Engine,
    store: wasmtime::Store<()>,
    instance: wasmtime::Instance,
    memory: wasmtime::Memory,
}

/// Determine whether `url` is a public HTTP(S) target that the WASM host
/// fetch may follow. Rejects non-HTTP schemes, metadata/internal hostnames,
/// and private/loopback/link-local/multicast IP addresses.
fn is_public_http_url(url: &str) -> bool {
    let Ok(parsed) = url::Url::parse(url) else {
        return false;
    };
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return false;
    }
    let Some(host) = parsed.host_str() else {
        return false;
    };
    let normalized = host.trim_end_matches('.').to_ascii_lowercase();
    if matches!(
        normalized.as_str(),
        "localhost" | "localhost.localdomain" | "metadata.google.internal"
    ) {
        return false;
    }
    let ip_host = host.trim_matches(['[', ']']);
    if let Ok(ip) = ip_host.parse::<std::net::IpAddr>() {
        return !match ip {
            std::net::IpAddr::V4(v4) => {
                v4.is_private()
                    || v4.is_loopback()
                    || v4.is_link_local()
                    || v4.is_broadcast()
                    || v4.is_unspecified()
                    || v4.is_multicast()
                    || (v4.octets()[0] == 100 && (64..=127).contains(&v4.octets()[1]))
                    || (v4.octets()[0] == 192 && v4.octets()[1] == 0 && v4.octets()[2] == 0)
            }
            std::net::IpAddr::V6(v6) => {
                v6.is_loopback()
                    || v6.is_unspecified()
                    || v6.is_unique_local()
                    || v6.is_unicast_link_local()
                    || v6.is_multicast()
            }
        };
    }
    true
}

/// Dedicated blocking runtime for WASM host fetch calls (leaked static,
/// same pattern as [`PgStorage`](crate::pg_storage::BLOCKING_RUNTIME)).
static FETCH_RUNTIME: std::sync::OnceLock<tokio::runtime::Runtime> = std::sync::OnceLock::new();

fn fetch_runtime() -> &'static tokio::runtime::Runtime {
    #[allow(clippy::panic)]
    FETCH_RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .thread_name("crawlkit-wasm-fetch")
            .build()
            .unwrap_or_else(|e| panic!("WASM fetch runtime failed to build: {e}"))
    })
}

// Error types for plugin loading are already defined above.

impl WasmPlugin {
    /// Load a WASM plugin from a directory with default security configuration.
    pub fn load(plugin_dir: &Path) -> Result<Self, PluginError> {
        Self::load_with_config(plugin_dir, &WasmConfig::default())
    }

    /// Load a WASM plugin from a directory with custom security configuration.
    pub fn load_with_config(plugin_dir: &Path, config: &WasmConfig) -> Result<Self, PluginError> {
        let manifest = read_plugin_manifest(plugin_dir)?;

        if !manifest.plugin.api_version.starts_with("1.") {
            return Err(PluginError::IncompatibleApiVersion(
                manifest.plugin.api_version,
            ));
        }

        // Capability enforcement (fail-closed with grantable network).
        // filesystem and env_vars are always rejected; network is grantable
        // only when BOTH the manifest declares it AND the embedder enables it
        // via WasmConfig.allow_plugin_network.
        if let Some(perms) = &manifest.plugin.permissions {
            let network_requested = perms.network.unwrap_or(false);
            let filesystem_requested = perms.filesystem.unwrap_or(false);
            let env_vars_requested = perms.env_vars.as_ref().is_some_and(|v| !v.is_empty());
            if filesystem_requested || env_vars_requested {
                return Err(PluginError::InvalidManifest(
                    concat!(
                        "plugin requests filesystem/env_vars capabilities ",
                        "that the sandbox cannot grant; only network is grantable ",
                        "via allow_plugin_network config",
                    )
                    .to_string(),
                ));
            }
            if network_requested && !config.allow_plugin_network {
                return Err(PluginError::InvalidManifest(
                    concat!(
                        "plugin requests network capability but allow_plugin_network ",
                        "is false; set WasmConfig.allow_plugin_network = true to grant ",
                        "HTTP access (SSRF-validated, no redirects, 1 MiB cap, 10s timeout)"
                    )
                    .to_string(),
                ));
            }
            if network_requested && config.allow_plugin_network {
                tracing::info!(
                    "Granting network capability to plugin: {}",
                    manifest.plugin.name
                );
            }
        }

        // Validate manifest fields before loading WASM
        validate_manifest(&manifest.plugin).map_err(|e| {
            tracing::warn!(
                "Plugin manifest validation failed for {}: {}",
                manifest.plugin.name,
                e
            );
            PluginError::InvalidManifest(e.to_string())
        })?;

        let wasm_file =
            manifest.plugin.entry.wasm.as_ref().ok_or_else(|| {
                PluginError::LoadFailed("No WASM entry point specified".to_string())
            })?;
        let wasm_path = plugin_dir.join(wasm_file);

        // Trust chain (wasm_hash + ed25519 signature) is verified BEFORE
        // the module is handed to wasmtime, so untrusted bytes never even
        // reach the compiler.
        let wasm_bytes = std::fs::read(&wasm_path)
            .map_err(|e| PluginError::LoadFailed(format!("Failed to read WASM file: {e}")))?;
        verify_plugin_trust(&manifest.plugin, &wasm_bytes, &config.plugin_verification)?;

        // Configure wasmtime with fuel limits to prevent infinite loops and
        // epoch interruption to enforce wall-clock timeouts.
        let mut engine_config = wasmtime::Config::new();
        engine_config.consume_fuel(true);
        engine_config.epoch_interruption(true);
        let engine = wasmtime::Engine::new(&engine_config)
            .map_err(|e| PluginError::LoadFailed(format!("Failed to create engine: {}", e)))?;

        let module = wasmtime::Module::from_file(&engine, &wasm_path)
            .map_err(|e| PluginError::LoadFailed(format!("Failed to compile WASM: {}", e)))?;

        let mut store = wasmtime::Store::new(&engine, ());
        store
            .set_fuel(config.max_fuel)
            .map_err(|e| PluginError::LoadFailed(format!("Failed to set fuel: {}", e)))?;

        // With epoch interruption enabled, every call traps unless a
        // deadline is armed. Load-time calls (init) run under a
        // effectively-infinite deadline; only `analyze` arms the tight
        // per-call timeout.
        store.set_epoch_deadline(u64::MAX);

        let mut linker = wasmtime::Linker::new(&engine);

        // When network capability is granted (manifest declares it AND config
        // enables it), link the host fetch function. Otherwise the sandbox
        // remains pure-compute (no imports linked).
        let network_granted = manifest
            .plugin
            .permissions
            .as_ref()
            .is_some_and(|p| p.network.unwrap_or(false))
            && config.allow_plugin_network;

        if network_granted {
            linker
                .func_wrap(
                    "crawlkit_host",
                    "fetch",
                    |mut caller: wasmtime::Caller<'_, ()>, url_ptr: i32, url_len: i32| -> i32 {
                        // Read URL bytes from guest memory
                        let url_bytes = {
                            let memory = match caller.get_export("memory") {
                                Some(wasmtime::Extern::Memory(m)) => m,
                                _ => return 0,
                            };
                            let data = memory.data(&caller);
                            let start = url_ptr as usize;
                            let end = start + url_len as usize;
                            if end > data.len() {
                                return 0;
                            }
                            data[start..end].to_vec()
                        };

                        let url = match String::from_utf8(url_bytes) {
                            Ok(s) => s,
                            Err(_) => return 0,
                        };

                        if !is_public_http_url(&url) {
                            tracing::debug!("WASM fetch blocked by SSRF guard: {url}");
                            return 0;
                        }

                        // Fetch via the dedicated blocking runtime (never
                        // panics from within a Tokio worker because the
                        // runtime is separate and leaked).
                        let rt = fetch_runtime();
                        let result = rt.block_on(async {
                            let client = reqwest::Client::builder()
                                .redirect(reqwest::redirect::Policy::none())
                                .timeout(std::time::Duration::from_secs(10))
                                .build()
                                .map_err(|e| e.to_string())?;
                            let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;
                            let status = resp.status().as_u16();
                            let body = resp.bytes().await.map_err(|e| e.to_string())?;
                            // Cap at 1 MiB
                            let body = if body.len() > 1_048_576 {
                                &body[..1_048_576]
                            } else {
                                &body
                            };
                            let body_str = String::from_utf8_lossy(body).into_owned();
                            Ok::<(u16, String), String>((status, body_str))
                        });

                        let json = match result {
                            Ok((status, body)) => serde_json::json!({
                                "status": status,
                                "body": body,
                            })
                            .to_string(),
                            Err(e) => {
                                tracing::debug!("WASM fetch failed: {e}");
                                return 0;
                            }
                        };

                        let json_bytes = json.as_bytes();
                        let alloc_len = json_bytes.len() + 1; // +1 for NUL

                        // Allocate in guest via crawlkit_plugin_alloc
                        let alloc_fn = match caller
                            .get_export("crawlkit_plugin_alloc")
                            .and_then(|e| e.into_func())
                        {
                            Some(f) => f,
                            None => return 0,
                        };
                        let mut alloc_result = [wasmtime::Val::I32(0)];
                        if alloc_fn
                            .call(
                                &mut caller,
                                &[wasmtime::Val::I32(alloc_len as i32)],
                                &mut alloc_result,
                            )
                            .is_err()
                        {
                            return 0;
                        }
                        let result_ptr = match alloc_result[0] {
                            wasmtime::Val::I32(p) => p,
                            _ => return 0,
                        };
                        if result_ptr == 0 {
                            return 0;
                        }

                        // Write JSON + NUL into guest memory at result_ptr
                        if let Some(wasmtime::Extern::Memory(memory)) = caller.get_export("memory")
                        {
                            let data = memory.data_mut(&mut caller);
                            let start = result_ptr as usize;
                            let end = start + alloc_len;
                            if end <= data.len() {
                                data[start..start + json_bytes.len()].copy_from_slice(json_bytes);
                                data[start + json_bytes.len()] = 0; // NUL terminator
                            }
                        }

                        result_ptr
                    },
                )
                .map_err(|e| {
                    PluginError::LoadFailed(format!("Failed to link crawlkit_host.fetch: {e}"))
                })?;
        }

        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| PluginError::LoadFailed(format!("Failed to instantiate WASM: {}", e)))?;

        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| PluginError::LoadFailed("No memory export found".to_string()))?;

        // Validate initial memory does not exceed configured limit.
        if memory.data_size(&store) > config.max_memory_bytes {
            return Err(PluginError::LoadFailed(format!(
                "Plugin initial memory ({} bytes) exceeds limit ({} bytes)",
                memory.data_size(&store),
                config.max_memory_bytes,
            )));
        }

        let init_func = instance
            .get_typed_func::<i32, i32>(&mut store, "crawlkit_plugin_init")
            .map_err(|e| PluginError::InitFailed(format!("Init function not found: {}", e)))?;

        let result = init_func
            .call(&mut store, 0)
            .map_err(|e| PluginError::InitFailed(format!("Init failed: {}", e)))?;

        if result != 0 {
            return Err(PluginError::InitFailed(format!(
                "Init returned error code: {}",
                result
            )));
        }

        Ok(Self {
            manifest: manifest.plugin,
            config: config.clone(),
            engine,
            store,
            instance,
            memory,
        })
    }

    /// Analyze HTML content using the plugin.
    ///
    /// Enforces the configured wall-clock timeout via epoch interruption:
    /// a watchdog thread increments the engine epoch when the deadline
    /// passes, trapping execution inside the guest.
    pub fn analyze(&mut self, html: &str, url: &str) -> Result<String, PluginError> {
        let analyze_func = self
            .instance
            .get_typed_func::<(i32, i32, i32, i32), i32>(&mut self.store, "crawlkit_plugin_analyze")
            .map_err(|e| {
                PluginError::AnalysisFailed(format!("Analyze function not found: {}", e))
            })?;

        let html_bytes = html.as_bytes();
        let url_bytes = url.as_bytes();

        let alloc_func = self
            .instance
            .get_typed_func::<i32, i32>(&mut self.store, "crawlkit_plugin_alloc")
            .map_err(|e| PluginError::AnalysisFailed(format!("Alloc function not found: {}", e)))?;

        let html_ptr = alloc_func
            .call(&mut self.store, html_bytes.len() as i32)
            .map_err(|e| {
                PluginError::AnalysisFailed(format!("Failed to allocate for HTML: {}", e))
            })?;

        let url_ptr = alloc_func
            .call(&mut self.store, url_bytes.len() as i32)
            .map_err(|e| {
                PluginError::AnalysisFailed(format!("Failed to allocate for URL: {}", e))
            })?;

        // Bounds check: validate allocation results are within memory limits.
        self.validate_wasm_pointer(html_ptr as usize, html_bytes.len())?;
        self.validate_wasm_pointer(url_ptr as usize, url_bytes.len())?;

        self.memory.data_mut(&mut self.store)
            [html_ptr as usize..(html_ptr as usize + html_bytes.len())]
            .copy_from_slice(html_bytes);

        self.memory.data_mut(&mut self.store)
            [url_ptr as usize..(url_ptr as usize + url_bytes.len())]
            .copy_from_slice(url_bytes);

        // Arm the wall-clock deadline for this call, then start the watchdog
        // that bumps the engine epoch if the guest overruns.
        self.store.set_epoch_deadline(1);
        let watchdog =
            EpochWatchdog::spawn(self.engine.clone(), self.config.max_analysis_timeout_ms);

        let analyze_result = analyze_func.call(
            &mut self.store,
            (
                html_ptr,
                html_bytes.len() as i32,
                url_ptr,
                url_bytes.len() as i32,
            ),
        );

        watchdog.cancel();

        let result_ptr = analyze_result.map_err(|e| {
            // Epoch-deadline kills surface as `Trap::Interrupt` (or an
            // "epoch" message on some wasmtime versions); fuel exhaustion
            // reports "all fuel consumed".
            let is_timeout = e
                .downcast_ref::<wasmtime::Trap>()
                .is_some_and(|trap| matches!(trap, wasmtime::Trap::Interrupt))
                || e.to_string().contains("epoch");
            let is_fuel = e.to_string().contains("all fuel consumed");
            if is_timeout {
                PluginError::AnalysisFailed(format!(
                    "Plugin exceeded the {}ms analysis timeout and was terminated",
                    self.config.max_analysis_timeout_ms
                ))
            } else if is_fuel {
                PluginError::AnalysisFailed(format!(
                    "Plugin exhausted its {} instruction fuel budget and was terminated",
                    self.config.max_fuel
                ))
            } else {
                PluginError::AnalysisFailed(format!("Analyze failed: {}", e))
            }
        })?;

        // A null (0) return means the plugin could not produce a result
        // (e.g. allocation failure inside the guest).
        if result_ptr == 0 {
            return Err(PluginError::AnalysisFailed(
                "Plugin analyze returned a null pointer".to_string(),
            ));
        }

        let result = self.read_string(result_ptr as usize)?;

        let free_func = self
            .instance
            .get_typed_func::<i32, ()>(&mut self.store, "crawlkit_plugin_free")
            .map_err(|e| PluginError::AnalysisFailed(format!("Free function not found: {}", e)))?;
        let _ = free_func.call(&mut self.store, html_ptr);
        let _ = free_func.call(&mut self.store, url_ptr);
        let _ = free_func.call(&mut self.store, result_ptr);

        Ok(result)
    }

    /// Validate that a pointer+length region lies within the WASM memory bounds.
    fn validate_wasm_pointer(&self, ptr: usize, len: usize) -> Result<(), PluginError> {
        let mem_size = self.memory.data(&self.store).len();
        if ptr > mem_size || len > mem_size - ptr {
            return Err(PluginError::WasmExecution(format!(
                "WASM pointer out of bounds: ptr={}, len={}, memory_size={}",
                ptr, len, mem_size,
            )));
        }
        // Reject if the write region exceeds the configured memory limit.
        if ptr + len > self.config.max_memory_bytes {
            return Err(PluginError::WasmExecution(format!(
                "WASM memory access exceeds limit: ptr={}, len={}, limit={}",
                ptr, len, self.config.max_memory_bytes,
            )));
        }
        Ok(())
    }

    /// Read a null-terminated string from WASM memory.
    fn read_string(&self, ptr: usize) -> Result<String, PluginError> {
        let data = self.memory.data(&self.store);
        let mem_len = data.len();

        // Bounds check: ptr must be within memory.
        if ptr >= mem_len {
            return Err(PluginError::WasmExecution(format!(
                "String pointer out of bounds: ptr={}, memory_size={}",
                ptr, mem_len,
            )));
        }

        let end = data[ptr..].iter().position(|&b| b == 0).ok_or_else(|| {
            PluginError::WasmExecution(format!(
                "No null terminator found starting at ptr={}, memory_size={}",
                ptr, mem_len,
            ))
        })?;

        let bytes = &data[ptr..ptr + end];
        String::from_utf8(bytes.to_vec())
            .map_err(|e| PluginError::WasmExecution(format!("Invalid UTF-8: {}", e)))
    }

    /// Get plugin metadata.
    pub fn metadata(&self) -> &PluginMetadata {
        &self.manifest
    }
}

/// Wall-clock watchdog for WASM plugin execution.
///
/// Sleeps in small increments until either cancelled (the plugin finished)
/// or the deadline passes, at which point it increments the wasmtime engine
/// epoch — trapping any guest execution armed with an epoch deadline.
struct EpochWatchdog {
    done: Arc<std::sync::atomic::AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl EpochWatchdog {
    fn spawn(engine: wasmtime::Engine, timeout_ms: u64) -> Self {
        let done = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = Arc::clone(&done);
        let handle = std::thread::spawn(move || {
            let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);
            while !flag.load(std::sync::atomic::Ordering::Relaxed) {
                if std::time::Instant::now() >= deadline {
                    engine.increment_epoch();
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        });
        Self {
            done,
            handle: Some(handle),
        }
    }

    /// Signal completion and join the watchdog thread.
    fn cancel(mut self) {
        self.done.store(true, std::sync::atomic::Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for EpochWatchdog {
    fn drop(&mut self) {
        self.done.store(true, std::sync::atomic::Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// Plugin registry managing all loaded plugins.
pub struct PluginRegistry {
    plugins: Arc<RwLock<Vec<WasmPlugin>>>,
    search_paths: Vec<PathBuf>,
}

impl PluginRegistry {
    /// Create empty plugin registry.
    pub fn new() -> Self {
        Self {
            plugins: Arc::new(RwLock::new(Vec::new())),
            search_paths: Vec::new(),
        }
    }

    /// Add a plugin search path.
    pub fn add_search_path(&mut self, path: PathBuf) {
        self.search_paths.push(path);
    }

    /// Scan search paths and load all valid plugins.
    pub fn load_all(&mut self) -> Vec<PluginError> {
        let mut errors = Vec::new();

        for search_path in &self.search_paths {
            if !search_path.exists() {
                continue;
            }

            if let Ok(entries) = std::fs::read_dir(search_path) {
                for entry in entries.flatten() {
                    let plugin_dir = entry.path();
                    if plugin_dir.is_dir() {
                        match WasmPlugin::load(&plugin_dir) {
                            Ok(plugin) => {
                                tracing::info!(
                                    "Loaded plugin: {} v{}",
                                    plugin.metadata().name,
                                    plugin.metadata().version
                                );
                                self.plugins.write().push(plugin);
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "Failed to load plugin from {}: {}",
                                    plugin_dir.display(),
                                    e
                                );
                                errors.push(e);
                            }
                        }
                    }
                }
            }
        }

        errors
    }

    /// Get list of loaded plugin names.
    pub fn list(&self) -> Vec<String> {
        self.plugins
            .read()
            .iter()
            .map(|p| p.metadata().name.clone())
            .collect()
    }

    /// Get plugin count.
    pub fn count(&self) -> usize {
        self.plugins.read().len()
    }

    /// Run analysis through all loaded plugins.
    pub fn analyze_all(&self, html: &str, url: &str) -> Vec<Result<String, PluginError>> {
        let mut results = Vec::new();
        let mut plugins = self.plugins.write();

        for plugin in plugins.iter_mut() {
            results.push(plugin.analyze(html, url));
        }

        results
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
    fn test_plugin_registry_default() {
        let registry = PluginRegistry::new();
        assert_eq!(registry.count(), 0);
        assert!(registry.list().is_empty());
    }

    #[test]
    fn test_plugin_loader_add_search_path() {
        let mut registry = PluginRegistry::new();
        registry.add_search_path(PathBuf::from("/tmp/plugins"));
        assert_eq!(registry.search_paths.len(), 1);
    }

    #[test]
    fn test_plugin_loader_nonexistent_path() {
        let mut registry = PluginRegistry::new();
        registry.add_search_path(PathBuf::from("/nonexistent/path"));
        let errors = registry.load_all();
        assert!(errors.is_empty());
    }

    #[test]
    fn test_validate_version_valid() {
        assert!(validate_version("1.0.0"));
        assert!(validate_version("0.1.0"));
        assert!(validate_version("10.20.30"));
        assert!(validate_version("0.0.1"));
    }

    #[test]
    fn test_validate_version_invalid() {
        assert!(!validate_version("1.0"));
        assert!(!validate_version("1.0.0.0"));
        assert!(!validate_version("1.0.beta"));
        assert!(!validate_version(""));
        assert!(!validate_version("01.0.0"));
        assert!(!validate_version("1.00.0"));
    }

    #[test]
    fn test_validate_license_valid() {
        assert!(validate_license("MIT"));
        assert!(validate_license("Apache-2.0"));
        assert!(validate_license("GPL-3.0-or-later"));
        assert!(validate_license("BSD-3-Clause"));
        assert!(validate_license("ISC"));
        assert!(validate_license("MPL-2.0"));
    }

    #[test]
    fn test_validate_license_invalid() {
        assert!(!validate_license("MIT-style"));
        assert!(!validate_license("Proprietary"));
        assert!(!validate_license(""));
        assert!(!validate_license("Custom-1.0"));
    }

    #[test]
    fn test_validate_manifest_valid() {
        let metadata = PluginMetadata {
            name: "test-plugin".to_string(),
            version: "1.0.0".to_string(),
            api_version: "1.0".to_string(),
            author: "Test Author".to_string(),
            description: "A test plugin".to_string(),
            license: "MIT".to_string(),
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
        assert!(validate_manifest(&metadata).is_ok());
    }

    #[test]
    fn test_validate_manifest_empty_name() {
        let metadata = PluginMetadata {
            name: "".to_string(),
            version: "1.0.0".to_string(),
            api_version: "1.0".to_string(),
            author: "Author".to_string(),
            description: "Desc".to_string(),
            license: "MIT".to_string(),
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
        assert_eq!(
            validate_manifest(&metadata).unwrap_err(),
            ManifestError::NameRequired
        );
    }

    #[test]
    fn test_validate_manifest_invalid_name() {
        let metadata = PluginMetadata {
            name: "test plugin!".to_string(),
            version: "1.0.0".to_string(),
            api_version: "1.0".to_string(),
            author: "Author".to_string(),
            description: "Desc".to_string(),
            license: "MIT".to_string(),
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
        assert_eq!(
            validate_manifest(&metadata).unwrap_err(),
            ManifestError::NameInvalid
        );
    }

    #[test]
    fn test_validate_manifest_invalid_version() {
        let metadata = PluginMetadata {
            name: "my-plugin".to_string(),
            version: "1.0".to_string(),
            api_version: "1.0".to_string(),
            author: "Author".to_string(),
            description: "Desc".to_string(),
            license: "MIT".to_string(),
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
        assert_eq!(
            validate_manifest(&metadata).unwrap_err(),
            ManifestError::VersionInvalid
        );
    }

    #[test]
    fn test_validate_manifest_description_too_long() {
        let metadata = PluginMetadata {
            name: "my-plugin".to_string(),
            version: "1.0.0".to_string(),
            api_version: "1.0".to_string(),
            author: "Author".to_string(),
            description: "a".repeat(501),
            license: "MIT".to_string(),
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
        assert_eq!(
            validate_manifest(&metadata).unwrap_err(),
            ManifestError::DescriptionTooLong
        );
    }

    #[test]
    fn test_validate_manifest_invalid_license() {
        let metadata = PluginMetadata {
            name: "my-plugin".to_string(),
            version: "1.0.0".to_string(),
            api_version: "1.0".to_string(),
            author: "Author".to_string(),
            description: "Desc".to_string(),
            license: "Proprietary".to_string(),
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
        assert_eq!(
            validate_manifest(&metadata).unwrap_err(),
            ManifestError::LicenseInvalid
        );
    }

    #[test]
    fn test_validate_manifest_no_wasm_entry() {
        let metadata = PluginMetadata {
            name: "my-plugin".to_string(),
            version: "1.0.0".to_string(),
            api_version: "1.0".to_string(),
            author: "Author".to_string(),
            description: "Desc".to_string(),
            license: "MIT".to_string(),
            trust_level: None,
            entry: PluginEntry {
                wasm: None,
                native: None,
            },
            permissions: None,
            analyzer: None,
            wasm_hash: None,
            signature: None,
            signed_by: None,
        };
        assert_eq!(
            validate_manifest(&metadata).unwrap_err(),
            ManifestError::EntryPointRequired
        );
    }

    #[test]
    fn test_validate_manifest_non_wasm_entry() {
        let metadata = PluginMetadata {
            name: "my-plugin".to_string(),
            version: "1.0.0".to_string(),
            api_version: "1.0".to_string(),
            author: "Author".to_string(),
            description: "Desc".to_string(),
            license: "MIT".to_string(),
            trust_level: None,
            entry: PluginEntry {
                wasm: Some("plugin.js".to_string()),
                native: None,
            },
            permissions: None,
            analyzer: None,
            wasm_hash: None,
            signature: None,
            signed_by: None,
        };
        assert_eq!(
            validate_manifest(&metadata).unwrap_err(),
            ManifestError::EntryPointNotWasm
        );
    }
}

#[cfg(test)]
mod ssrf_tests {
    use super::is_public_http_url;

    #[test]
    fn rejects_non_http_schemes() {
        assert!(!is_public_http_url("ftp://example.com/file"));
        assert!(!is_public_http_url("file:///etc/passwd"));
    }

    #[test]
    fn rejects_localhost_and_metadata() {
        assert!(!is_public_http_url("http://localhost/"));
        assert!(!is_public_http_url(
            "http://metadata.google.internal/latest"
        ));
    }

    #[test]
    fn rejects_private_ipv4() {
        assert!(!is_public_http_url("http://127.0.0.1/"));
        assert!(!is_public_http_url("http://10.0.0.1/api"));
        assert!(!is_public_http_url(
            "http://169.254.169.254/latest/meta-data"
        ));
        assert!(!is_public_http_url("http://192.168.1.1/admin"));
        assert!(!is_public_http_url("http://172.16.0.1/internal"));
    }

    #[test]
    fn rejects_private_ipv6() {
        assert!(!is_public_http_url("http://[::1]/"));
        assert!(!is_public_http_url("http://[fd00::1]/"));
    }

    #[test]
    fn rejects_empty_and_malformed() {
        assert!(!is_public_http_url(""));
        assert!(!is_public_http_url("not-a-url"));
    }

    #[test]
    fn accepts_valid_public_https() {
        assert!(is_public_http_url("https://example.com"));
        assert!(is_public_http_url("https://api.stripe.com/v1/charges"));
        assert!(is_public_http_url("http://example.com:8080/path?q=1"));
    }
}
