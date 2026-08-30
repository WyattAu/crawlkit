use serde::{Deserialize, Serialize};

use super::ManifestError;

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
    /// Plugin kind: `"wasm"` (default, core ABI) or `"wasi-component"`
    /// (WASI Preview 2 component model). When absent, defaults to `"wasm"`.
    pub kind: Option<String>,
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

/// Plugin kind discriminator.
///
/// Determines which runtime adapter handles the plugin. The `kind` field
/// in the manifest maps to this enum; absent or `"wasm"` means the legacy
/// core ABI, `"wasi-component"` means WASI Preview 2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PluginKind {
    /// Legacy core WASM ABI (`crawlkit_plugin_init`, `crawlkit_plugin_analyze`).
    /// This is the default when `kind` is absent from the manifest.
    Wasm,
    /// WASI Preview 2 component model.
    #[serde(rename = "wasi-component")]
    WasiComponent,
}

impl PluginKind {
    /// Parse a `kind` string from the manifest, defaulting to [`Wasm`](Self::Wasm).
    pub fn from_manifest(s: Option<&str>) -> Self {
        match s {
            Some("wasi-component") => Self::WasiComponent,
            _ => Self::Wasm,
        }
    }
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

/// Read and parse a plugin manifest (`crawlkit-plugin.toml`).
pub(crate) fn read_plugin_manifest(
    plugin_dir: &std::path::Path,
) -> Result<PluginManifest, super::PluginError> {
    let manifest_path = plugin_dir.join("crawlkit-plugin.toml");
    let manifest_str = std::fs::read_to_string(&manifest_path)
        .map_err(|e| super::PluginError::ManifestParse(format!("Failed to read manifest: {e}")))?;
    toml::from_str(&manifest_str)
        .map_err(|e| super::PluginError::ManifestParse(format!("Invalid manifest: {e}")))
}
