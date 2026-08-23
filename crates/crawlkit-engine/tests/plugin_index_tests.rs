//! Plugin index installation tests.
//!
//! The full distribution chain: build a real SDK plugin to wasm32, sign it
//! with the trusted dev key, publish it via a local index, install it
//! through `install_plugin`, and load + run it via `WasmPlugin` under the
//! default Required policy. Tampered artifacts, untrusted signers, and
//! unknown names are rejected without touching the install root.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crawlkit_engine::plugin::sign_plugin_wasm;
use crawlkit_engine::plugin::{PluginVerification, WasmConfig, WasmPlugin};
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

/// Build an SDK example to wasm32 (same pattern as the wasm_abi_tests
/// conformance suite; skipped if the target is missing).
fn build_example(target_dir: &std::path::Path, example: &str) -> Option<std::path::PathBuf> {
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
            example,
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
            .join(format!("{example}.wasm")),
    )
}

/// Build the SDK basic-plugin example to wasm32.
fn build_example_plugin(target_dir: &std::path::Path) -> Option<std::path::PathBuf> {
    build_example(target_dir, "basic-plugin")
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

#[test]
fn artifact_source_resolution_matrix() {
    use crawlkit_engine::plugin_index::resolve_artifact_source;

    // Absolute artifact URL: used verbatim regardless of index type.
    assert_eq!(
        resolve_artifact_source(
            "https://example.com/index/plugin-index.toml",
            "https://cdn.example.org/p.wasm"
        ),
        "https://cdn.example.org/p.wasm"
    );
    assert_eq!(
        resolve_artifact_source("/local/index.toml", "https://cdn.example.org/p.wasm"),
        "https://cdn.example.org/p.wasm"
    );

    // Remote index + relative artifact: joins against the URL base.
    assert_eq!(
        resolve_artifact_source(
            "https://example.com/index/plugin-index.toml",
            "artifacts/p.wasm"
        ),
        "https://example.com/index/artifacts/p.wasm"
    );
    assert_eq!(
        resolve_artifact_source("https://example.com/plugin-index.toml", "artifacts/p.wasm"),
        "https://example.com/artifacts/p.wasm"
    );

    // Local index + relative artifact: returned verbatim (the caller joins
    // it against the filesystem parent of the index path).
    assert_eq!(
        resolve_artifact_source("/local/dir/plugin-index.toml", "artifacts/p.wasm"),
        "artifacts/p.wasm"
    );

    // http:// indexes are treated like https for base joining.
    assert_eq!(
        resolve_artifact_source("http://localhost:8000/plugin-index.toml", "a/b.wasm"),
        "http://localhost:8000/a/b.wasm"
    );
}

/// Builds and signs the first-party plugin index into
/// `$FIRST_PARTY_INDEX_DIR` (repository path: plugins/index). Artifacts
/// are release-built wasm32 and signed with the trusted dev key.
///
/// Run via `scripts/build-plugin-index.sh`. Not part of CI: commits to the
/// repository are deliberate release events.
#[test]
#[ignore = "manual: requires FIRST_PARTY_INDEX_DIR"]
fn dump_first_party_index() {
    use crawlkit_engine::plugin::sign_plugin_wasm;

    let dir = std::env::var("FIRST_PARTY_INDEX_DIR").expect("FIRST_PARTY_INDEX_DIR not set");
    let tmp = tempfile::tempdir().unwrap();

    // (example name, published name, version, description, categories)
    let specs: &[(&str, &str, &str, &str, &str)] = &[
        (
            "basic-plugin",
            "title-length",
            "1.0.0",
            "Flags missing and oversized <title> elements",
            "seo",
        ),
        (
            "viewport-checker",
            "viewport-checker",
            "1.0.0",
            "Flags missing viewport meta tags and fixed-width viewports",
            "mobile",
        ),
        (
            "soft-404",
            "soft-404",
            "1.0.0",
            "Flags error pages that were still analyzed (host context API)",
            "seo",
        ),
    ];

    let dest = std::path::Path::new(&dir);
    std::fs::create_dir_all(dest.join("artifacts")).unwrap();

    let mut index = String::new();
    for (example, name, version, description, category) in specs {
        let target_dir = tmp.path().join(*example);
        let built = build_example(&target_dir, example).expect("wasm32 build");
        // Rebuild in release profile for the published artifact.
        let workspace_root =
            std::path::Path::new(std::env::var("CARGO_MANIFEST_DIR").unwrap().as_str())
                .ancestors()
                .nth(2)
                .unwrap()
                .to_path_buf();
        let status = std::process::Command::new("cargo")
            .current_dir(&workspace_root)
            .args([
                "build",
                "-p",
                "crawlkit-plugin-sdk",
                "--release",
                "--example",
                example,
                "--target",
                "wasm32-unknown-unknown",
                "--target-dir",
            ])
            .arg(&target_dir)
            .status()
            .unwrap();
        assert!(status.success(), "release build failed for {example}");
        let release_wasm = target_dir
            .join("wasm32-unknown-unknown")
            .join("release")
            .join("examples")
            .join(format!("{example}.wasm"));
        let wasm_bytes = std::fs::read(&release_wasm).unwrap();
        let (wasm_hash, signature, signed_by) =
            sign_plugin_wasm(&wasm_bytes, &seed(TRUSTED_SEED_HEX));

        let artifact_name = format!("{name}-{version}.wasm");
        std::fs::write(dest.join("artifacts").join(&artifact_name), &wasm_bytes).unwrap();
        index.push_str(&format!(
            "[[plugin]]\n\
             name = \"{name}\"\n\
             version = \"{version}\"\n\
             api_version = \"1.0\"\n\
             author = \"crawlkit\"\n\
             description = \"{description}\"\n\
             license = \"Apache-2.0\"\n\
             categories = [\"{category}\"]\n\
             wasm_path = \"artifacts/{artifact_name}\"\n\
             wasm_hash = \"{wasm_hash}\"\n\
             signature = \"{signature}\"\n\
             signed_by = \"{signed_by}\"\n\n"
        ));
        println!("signed {name} {version}: {wasm_hash}");
        let _ = built;
    }
    std::fs::write(
        dest.join("plugin-index.toml"),
        index.trim_end().to_string() + "\n",
    )
    .unwrap();
    println!("first-party index written to {dir}");
}

/// Functional check of the second first-party plugin: viewport-checker
/// must report VP001 (missing viewport) / VP002 (fixed width) / clean as
/// appropriate, through the real host ABI.
#[test]
fn viewport_checker_plugin_functional() {
    let target_dir = std::env::temp_dir().join("crawlkit-vp-test");
    let Some(wasm_path) = build_example(&target_dir, "viewport-checker") else {
        eprintln!("skipping: wasm32-unknown-unknown build unavailable");
        return;
    };
    let wasm_bytes = std::fs::read(&wasm_path).unwrap();
    let (wasm_hash, signature, signed_by) = sign_plugin_wasm(&wasm_bytes, &seed(TRUSTED_SEED_HEX));

    let dir = tempfile::tempdir().unwrap();
    let plugin_dir = dir.path().join("viewport-checker");
    std::fs::create_dir_all(&plugin_dir).unwrap();
    std::fs::write(plugin_dir.join("plugin.wasm"), &wasm_bytes).unwrap();
    std::fs::write(
        plugin_dir.join("crawlkit-plugin.toml"),
        format!(
            "[plugin]\nname = \"viewport-checker\"\nversion = \"1.0.0\"\napi_version = \"1.0\"\nauthor = \"test\"\ndescription = \"functional fixture\"\nlicense = \"Apache-2.0\"\nwasm_hash = \"{wasm_hash}\"\nsignature = \"{signature}\"\nsigned_by = \"{signed_by}\"\n\n[plugin.entry]\nwasm = \"plugin.wasm\"\n\n[plugin.analyzer]\nname = \"viewport-checker\"\ncategories = [\"mobile\"]\n"
        ),
    )
    .unwrap();

    let mut plugin = WasmPlugin::load(&plugin_dir).expect("load under Required policy");

    let no_viewport = "<html><head><title>x</title></head><body></body></html>";
    let r = plugin.analyze(no_viewport, "https://example.com").unwrap();
    assert!(r.contains("VP001"), "missing viewport must fire VP001: {r}");

    let fixed_width = "<html><head><meta name=\"viewport\" content=\"width=980\"></head></html>";
    let r = plugin.analyze(fixed_width, "https://example.com").unwrap();
    assert!(r.contains("VP002"), "fixed width must fire VP002: {r}");

    let responsive = "<html><head><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"></head></html>";
    let r = plugin.analyze(responsive, "https://example.com").unwrap();
    assert_eq!(r, "[]", "responsive viewport must be clean: {r}");
}

/// B4: a guest importing `crawlkit_host.get_context` receives the JSON
/// context set via `analyze_with_context`; without one it gets null (0).
#[test]
fn get_context_host_function_end_to_end() {
    // Guest: allocates nothing itself; calls get_context and returns its
    // pointer directly (NUL-terminated host-written string).
    const CTX_WAT: &str = r#"
(module
  (import "crawlkit_host" "get_context" (func $get_context (result i32)))
  (memory (export "memory") 1)
  (data (i32.const 1024) "null-result\00")
  (global $heap (mut i32) (i32.const 2048))
  (func (export "crawlkit_plugin_init") (param i32) (result i32) i32.const 0)
  (func (export "crawlkit_plugin_alloc") (param $size i32) (result i32)
    (local $ptr i32)
    (local.set $ptr (global.get $heap))
    (global.set $heap (i32.add (global.get $heap) (local.get $size)))
    (local.get $ptr))
  (func (export "crawlkit_plugin_free") (param i32))
  (func (export "crawlkit_plugin_analyze")
        (param i32 i32 i32 i32) (result i32)
    ;; With context: return get_context()'s pointer; null falls back to
    ;; the static marker so the host read_string distinguishes them.
    (local $r i32)
    (local.set $r (call $get_context))
    (if (result i32) (i32.eqz (local.get $r))
      (then (i32.const 1024))
      (else (local.get $r))))
)
"#;

    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("plugin.wasm"), CTX_WAT).unwrap();
    std::fs::write(
        dir.path().join("crawlkit-plugin.toml"),
        "[plugin]\nname = \"ctx-probe\"\nversion = \"1.0.0\"\napi_version = \"1.0\"\nauthor = \"t\"\ndescription = \"context probe\"\nlicense = \"Apache-2.0\"\n\n[plugin.entry]\nwasm = \"plugin.wasm\"\n\n[plugin.analyzer]\nname = \"ctx-probe\"\ncategories = [\"test\"]\n",
    )
    .unwrap();

    let config = WasmConfig {
        plugin_verification: PluginVerification::AllowUnsigned,
        ..WasmConfig::default()
    };
    let mut plugin = WasmPlugin::load_with_config(dir.path(), &config).unwrap();

    // No context set: get_context returns 0 -> guest's static marker.
    let r = plugin
        .analyze("<html></html>", "https://example.com")
        .unwrap();
    assert_eq!(r, "null-result");

    // With context: the guest returns the host-written JSON verbatim.
    let ctx_json = r#"{"url":"https://example.com/a","status_code":200,"headers":[["x","y"]]}"#;
    let r = plugin
        .analyze_with_context("<html></html>", "https://example.com/a", Some(ctx_json))
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&r).expect("guest echoed host JSON");
    assert_eq!(parsed["url"], "https://example.com/a");
    assert_eq!(parsed["status_code"], 200);

    // Context does not leak into a subsequent plain analyze call.
    let r = plugin
        .analyze("<html></html>", "https://example.com")
        .unwrap();
    assert_eq!(r, "null-result");
}

/// Full B4 conformance: the soft-404 SDK example consumes
/// `crawlkit_host.get_context` through the real wasm32 ABI — findings
/// fire on 404 context, stay clean on 200, and degrade to no-op without
/// context.
#[test]
fn soft404_plugin_uses_host_context_end_to_end() {
    let target_dir = std::env::temp_dir().join("crawlkit-soft404-test");
    let Some(wasm_path) = build_example(&target_dir, "soft-404") else {
        eprintln!("skipping: wasm32-unknown-unknown build unavailable");
        return;
    };
    let wasm_bytes = std::fs::read(&wasm_path).unwrap();
    let (wasm_hash, signature, signed_by) = sign_plugin_wasm(&wasm_bytes, &seed(TRUSTED_SEED_HEX));

    let dir = tempfile::tempdir().unwrap();
    let plugin_dir = dir.path().join("soft-404");
    std::fs::create_dir_all(&plugin_dir).unwrap();
    std::fs::write(plugin_dir.join("plugin.wasm"), &wasm_bytes).unwrap();
    std::fs::write(
        plugin_dir.join("crawlkit-plugin.toml"),
        format!(
            "[plugin]\nname = \"soft-404\"\nversion = \"1.0.0\"\napi_version = \"1.0\"\nauthor = \"test\"\ndescription = \"context conformance\"\nlicense = \"Apache-2.0\"\nwasm_hash = \"{wasm_hash}\"\nsignature = \"{signature}\"\nsigned_by = \"{signed_by}\"\n\n[plugin.entry]\nwasm = \"plugin.wasm\"\n\n[plugin.analyzer]\nname = \"soft-404\"\ncategories = [\"seo\"]\n"
        ),
    )
    .unwrap();

    let mut plugin = WasmPlugin::load(&plugin_dir).expect("load under Required policy");

    // 404 context -> SOFT404 finding.
    let ctx404 = r#"{"url":"https://example.com/missing","status_code":404,"headers":[]}"#;
    let r = plugin
        .analyze_with_context(
            "<html>gone</html>",
            "https://example.com/missing",
            Some(ctx404),
        )
        .unwrap();
    assert!(r.contains("SOFT404"), "404 context must fire: {r}");

    // 200 context -> clean.
    let ctx200 = r#"{"url":"https://example.com/","status_code":200,"headers":[]}"#;
    let r = plugin
        .analyze_with_context("<html>home</html>", "https://example.com/", Some(ctx200))
        .unwrap();
    assert_eq!(r, "[]", "200 context must be clean: {r}");

    // No context -> graceful no-op (v1 behavior preserved).
    let r = plugin
        .analyze("<html>whatever</html>", "https://example.com")
        .unwrap();
    assert_eq!(r, "[]", "no context must degrade cleanly: {r}");
}
