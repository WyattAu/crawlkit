//! Plugin index installation tests.
//!
//! The full distribution chain: build a real SDK plugin to wasm32, sign it
//! with the trusted dev key, publish it via a local index, install it
//! through `install_plugin`, and load + run it via `WasmPlugin` under the
//! default Required policy. Tampered artifacts, untrusted signers, and
//! unknown names are rejected without touching the install root.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crawlkit_engine::plugin::sign_plugin_wasm;
use crawlkit_engine::plugin::WasmPlugin;
use crawlkit_engine::plugin_index::{install_plugin, list_installed_plugins, parse_plugin_index};
use crawlkit_engine::PluginIndexError;

/// Seed (hex) of the first-party dev key embedded in `TRUSTED_PLUGIN_KEYS`
/// — test fixture only (mirrors wasm_abi_tests).
const TRUSTED_SEED_HEX: &str = "92bb3bc94dc375ea2c3111e1636511a8c0b22995437ee0338f4d21cdb9bfdd4d";

/// An unrelated key NOT in the trust store.
const ATTACKER_SEED_HEX: &str = "0000000000000000000000000000000000000000000000000000000000000001";

fn seed(hex: &str) -> [u8; 32] {
    hex.as_bytes()
        .chunks(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect::<Vec<u8>>()
        .try_into()
        .unwrap()
}

/// Build the SDK basic-plugin example to wasm32 (same pattern as the
/// wasm_abi_tests conformance suite; skipped if the target is missing).
fn build_example_plugin(target_dir: &std::path::Path) -> Option<std::path::PathBuf> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_root = std::path::Path::new(&manifest_dir)
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf();

    let installed = std::process::Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
        .ok()?;
    if !String::from_utf8_lossy(&installed.stdout)
        .lines()
        .any(|l| l.starts_with("wasm32-unknown-unknown"))
    {
        return None;
    }

    let status = std::process::Command::new("cargo")
        .current_dir(&workspace_root)
        .args([
            "build",
            "-p",
            "crawlkit-plugin-sdk",
            "--example",
            "basic-plugin",
            "--target",
            "wasm32-unknown-unknown",
            "--target-dir",
        ])
        .arg(target_dir)
        .status()
        .ok()?;
    if !status.success() {
        return None;
    }
    Some(
        target_dir
            .join("wasm32-unknown-unknown")
            .join("debug")
            .join("examples")
            .join("basic-plugin.wasm"),
    )
}

/// A local index fixture directory: index.toml + signed artifact.
struct LocalIndex {
    dir: std::path::PathBuf,
}

/// Panic-free fixture creation: skips the test when the wasm32 target
/// (or cargo sub-build) is unavailable in this environment.
macro_rules! local_index {
    ($base:expr, $seed:expr) => {
        match LocalIndex::try_create($base, $seed) {
            Some(f) => f,
            None => {
                eprintln!("skipping: wasm32-unknown-unknown build unavailable");
                return;
            }
        }
    };
}

impl LocalIndex {
    fn try_create(
        base: &std::path::Path,
        seed_hex: &str,
    ) -> Option<(Self, String, String, String)> {
        let target_dir = base.join("target");
        let wasm_path = build_example_plugin(&target_dir)?;
        let wasm_bytes = std::fs::read(&wasm_path).unwrap();
        let (wasm_hash, signature, signed_by) = sign_plugin_wasm(&wasm_bytes, &seed(seed_hex));

        let dir = base.join("index");
        std::fs::create_dir_all(dir.join("artifacts")).unwrap();
        std::fs::write(dir.join("artifacts/title-length.wasm"), &wasm_bytes).unwrap();

        let index = format!(
            "[[plugin]]\n\
             name = \"title-length\"\n\
             version = \"1.0.0\"\n\
             api_version = \"1.0\"\n\
             author = \"crawlkit\"\n\
             description = \"Title length analyzer fixture\"\n\
             license = \"Apache-2.0\"\n\
             categories = [\"seo\"]\n\
             wasm_path = \"artifacts/title-length.wasm\"\n\
             wasm_hash = \"{wasm_hash}\"\n\
             signature = \"{signature}\"\n\
             signed_by = \"{signed_by}\"\n"
        );
        std::fs::write(dir.join("plugin-index.toml"), &index).unwrap();
        Some((Self { dir }, wasm_hash, signature, signed_by))
    }

    fn path(&self) -> String {
        self.dir
            .join("plugin-index.toml")
            .to_str()
            .unwrap()
            .to_string()
    }

    fn entry(&self) -> crawlkit_engine::PluginIndexEntry {
        let entries = parse_plugin_index(&std::fs::read_to_string(self.path()).unwrap()).unwrap();
        entries.into_iter().next().unwrap()
    }
}

#[test]
fn index_parses_and_roundtrips_fields() {
    let tmp = tempfile::tempdir().unwrap();
    let (index, wasm_hash, _sig, signed_by) = local_index!(tmp.path(), TRUSTED_SEED_HEX);
    let entry = index.entry();
    assert_eq!(entry.name, "title-length");
    assert_eq!(entry.version, "1.0.0");
    assert_eq!(entry.categories, vec!["seo".to_string()]);
    assert_eq!(entry.wasm_hash, wasm_hash);
    assert_eq!(entry.signed_by, signed_by);
    assert_eq!(signed_by.len(), 16);
}

#[test]
fn install_and_load_full_chain() {
    let tmp = tempfile::tempdir().unwrap();
    let (index, _, _, _) = local_index!(tmp.path(), TRUSTED_SEED_HEX);
    let install_root = tmp.path().join("installed");

    let plugin_dir = install_plugin(&index.path(), "title-length", &install_root).expect("install");

    // The installed layout loads under the DEFAULT Required policy.
    let mut plugin = WasmPlugin::load(&plugin_dir).expect("load under Required policy");
    let long_title = "<html><head><title>0123456789012345678901234567890123456789012345678901234567890123456789</title></head></html>";
    let result = plugin.analyze(long_title, "https://example.com").unwrap();
    assert!(result.contains("TITLE002"), "result: {result}");

    let listed = list_installed_plugins(&install_root);
    assert_eq!(
        listed,
        vec![("title-length".to_string(), "1.0.0".to_string())]
    );
}

#[test]
fn tampered_artifact_is_rejected_and_installs_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let (index, _, _, _) = local_index!(tmp.path(), TRUSTED_SEED_HEX);
    // Flip a byte in the published artifact after signing.
    let artifact = index.dir.join("artifacts/title-length.wasm");
    let mut bytes = std::fs::read(&artifact).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 0xff;
    std::fs::write(&artifact, &bytes).unwrap();

    let install_root = tmp.path().join("installed");
    let err = install_plugin(&index.path(), "title-length", &install_root)
        .expect_err("tampered artifact must be rejected");
    assert!(matches!(err, PluginIndexError::Trust(_)), "got: {err:?}");
    // Install root untouched.
    assert!(!install_root.join("title-length").exists());
}

#[test]
fn untrusted_signer_is_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let (index, _, _, attacker_key_id) = local_index!(tmp.path(), ATTACKER_SEED_HEX);
    let install_root = tmp.path().join("installed");
    let err = install_plugin(&index.path(), "title-length", &install_root)
        .expect_err("untrusted signer must be rejected");
    assert!(matches!(err, PluginIndexError::Trust(_)), "got: {err:?}");
    assert!(!install_root.join("title-length").exists());
    // The attacker's key id must not be the trusted dev key id.
    assert_ne!(attacker_key_id, "1f299a0020f6ae90");
}

#[test]
fn unknown_plugin_name_is_reported() {
    let tmp = tempfile::tempdir().unwrap();
    let (index, _, _, _) = local_index!(tmp.path(), TRUSTED_SEED_HEX);
    let install_root = tmp.path().join("installed");
    let err =
        install_plugin(&index.path(), "does-not-exist", &install_root).expect_err("unknown name");
    assert!(matches!(err, PluginIndexError::NotFound(_)));
}

#[test]
fn malformed_index_is_a_parse_error() {
    let err = parse_plugin_index("not valid toml [[[").unwrap_err();
    assert!(matches!(err, PluginIndexError::Parse(_)));
}

/// Local smoke-testing helper: writes a fully-signed index fixture to
/// `$SMOKE_INDEX_DIR` for exercising the CLI (`crawlkit plugin install`).
/// Not run in CI: `cargo test --test plugin_index_tests dump_smoke_index -- --ignored`
#[test]
#[ignore = "manual: requires SMOKE_INDEX_DIR"]
fn dump_smoke_index() {
    let dir = std::env::var("SMOKE_INDEX_DIR").expect("SMOKE_INDEX_DIR not set");
    let tmp = tempfile::tempdir().unwrap();
    let (index, wasm_hash, signature, signed_by) =
        LocalIndex::try_create(tmp.path(), TRUSTED_SEED_HEX)
            .expect("manual helper: wasm32 build required");
    let dest = std::path::Path::new(&dir);
    std::fs::create_dir_all(dest.join("artifacts")).unwrap();
    std::fs::copy(
        index.dir.join("artifacts/title-length.wasm"),
        dest.join("artifacts/title-length.wasm"),
    )
    .unwrap();
    std::fs::write(
        dest.join("plugin-index.toml"),
        format!(
            "[[plugin]]\nname = \"title-length\"\nversion = \"1.0.0\"\napi_version = \"1.0\"\nauthor = \"crawlkit\"\ndescription = \"Smoke fixture\"\nlicense = \"Apache-2.0\"\ncategories = [\"seo\"]\nwasm_path = \"artifacts/title-length.wasm\"\nwasm_hash = \"{wasm_hash}\"\nsignature = \"{signature}\"\nsigned_by = \"{signed_by}\"\n"
        ),
    )
    .unwrap();
    println!("smoke index written to {dir}");
}
