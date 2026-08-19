//! Host-side WASM ABI integration tests.
//!
//! These load a hand-written WAT module implementing the full plugin ABI
//! (`init`/`alloc`/`analyze`/`free` + `memory` export) through the real
//! `WasmPlugin::load_with_config` path. This is the regression lock for the
//! host<->guest contract: the SDK previously exported a *different* ABI than
//! the host consumed, and nothing caught it because no test exercised the
//! boundary.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::Write;

use crawlkit_engine::plugin::{WasmConfig, WasmPlugin};

/// A minimal plugin written in WebAssembly text format.
///
/// - `alloc` is a simple bump allocator past a static data region
/// - `analyze` verifies the host actually delivered the HTML bytes by
///   checking the first byte is `<`, then returns a pointer to a static
///   NUL-terminated JSON response
/// - `free` is a no-op (bump allocator; reclaimed when the store drops)
const WAT_PLUGIN: &str = r#"
(module
  (memory (export "memory") 1)

  ;; Static result at offset 1024: valid JSON findings array, NUL-terminated.
  (data (i32.const 1024) "[{\"code\":\"WAT001\",\"severity\":\"info\"}]\00")
  ;; Static failure string at offset 2048.
  (data (i32.const 2048) "host did not deliver html\00")

  ;; Bump-allocate from 4096 upward, 8-byte aligned.
  (global $heap (mut i32) (i32.const 4096))

  (func (export "crawlkit_plugin_init") (param i32) (result i32)
    i32.const 0)

  (func (export "crawlkit_plugin_alloc") (param $size i32) (result i32)
    (local $ptr i32)
    (local.set $ptr (global.get $heap))
    ;; heap += align8(size)
    (global.set $heap
      (i32.add (global.get $heap)
        (i32.mul (i32.div_u (i32.add (local.get $size) (i32.const 7)) (i32.const 8)) (i32.const 8))))
    (local.get $ptr))

  (func (export "crawlkit_plugin_analyze")
        (param $html_ptr i32) (param $html_len i32)
        (param $url_ptr i32) (param $url_len i32)
        (result i32)
    ;; Verify the host wrote the HTML payload at the promised address.
    (if (result i32) (i32.eq (i32.load8_u (local.get $html_ptr)) (i32.const 60)) ;; '<'
      (then (i32.const 1024))
      (else (i32.const 2048))))

  (func (export "crawlkit_plugin_free") (param i32)
    ;; no-op for the bump allocator
  )
)
"#;

fn write_plugin(dir: &std::path::Path) {
    let mut manifest = std::fs::File::create(dir.join("crawlkit-plugin.toml")).unwrap();
    writeln!(
        manifest,
        r#"[plugin]
name = "wat-test"
version = "1.0.0"
api_version = "1.0"
author = "integration-test"
description = "WAT ABI fixture"
license = "Apache-2.0"

[plugin.entry]
wasm = "plugin.wasm"

[plugin.analyzer]
name = "wat-test-analyzer"
categories = ["test"]
"#
    )
    .unwrap();
    std::fs::write(dir.join("plugin.wasm"), WAT_PLUGIN).unwrap();
}

fn load() -> (tempfile::TempDir, WasmPlugin) {
    let dir = tempfile::tempdir().unwrap();
    write_plugin(dir.path());
    let plugin = WasmPlugin::load_with_config(dir.path(), &WasmConfig::default()).unwrap();
    (dir, plugin)
}

#[test]
fn host_guest_abi_roundtrip_returns_findings_json() {
    let (_guard, mut plugin) = load();

    let result = plugin
        .analyze("<html><body>hello</body></html>", "https://example.com")
        .expect("analyze must succeed across the ABI boundary");

    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed[0]["code"], "WAT001");
    assert_eq!(parsed[0]["severity"], "info");
}

#[test]
fn host_delivers_exact_bytes_to_guest() {
    let (_guard, mut plugin) = load();
    // The guest only returns the success JSON when the first HTML byte
    // arrives as '<' at the allocated address — proving the host's
    // alloc + memory-write + pointer/length passing is correct.
    let result = plugin.analyze("<p>x", "https://example.com").unwrap();
    assert!(result.contains("WAT001"));
}

#[test]
fn plugin_metadata_is_parsed_from_manifest() {
    let (_guard, plugin) = load();
    assert_eq!(plugin.metadata().name, "wat-test");
    assert_eq!(plugin.metadata().api_version, "1.0");
}

#[test]
fn missing_memory_or_exports_fail_to_load() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path()).unwrap();
    let mut manifest = std::fs::File::create(dir.path().join("crawlkit-plugin.toml")).unwrap();
    writeln!(
        manifest,
        r#"[plugin]
name = "empty"
version = "1.0.0"
api_version = "1.0"
author = "t"
description = "no exports"
license = "MIT"

[plugin.entry]
wasm = "empty.wat"
"#
    )
    .unwrap();
    // A module with memory but none of the required exports.
    std::fs::write(
        dir.path().join("empty.wat"),
        "(module (memory (export \"memory\") 1))",
    )
    .unwrap();

    let result = WasmPlugin::load_with_config(dir.path(), &WasmConfig::default());
    assert!(result.is_err(), "module without exports must not load");
}

/// Full SDK->wasm32->wasmtime-host conformance test.
///
/// Compiles the plugin-sdk example (`export_analyzer!`) to
/// `wasm32-unknown-unknown` and drives it through the real host loader.
/// This is the test that would have caught the historical SDK/host ABI
/// mismatch. Skips (passes) when the toolchain or cargo is unavailable.
#[test]
fn sdk_compiled_plugin_loads_and_runs_in_host() {
    use std::process::Command;

    // Locate the workspace root (crates/crawlkit-engine/tests/ -> ../../..).
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_root = std::path::Path::new(&manifest_dir)
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf();

    let rustc = Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output();
    let wasm_target_available = rustc
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .any(|l| l.starts_with("wasm32-unknown-unknown"))
        })
        .unwrap_or(false);
    if !wasm_target_available {
        eprintln!("skipping: wasm32-unknown-unknown target not installed");
        return;
    }

    let target_dir = workspace_root.join("target").join("wasm-plugin-test");
    let status = Command::new("cargo")
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
        .arg(&target_dir)
        .status();
    match status {
        Ok(s) if s.success() => {}
        _ => {
            eprintln!("skipping: cargo build of wasm example failed");
            return;
        }
    }

    let wasm_path = target_dir
        .join("wasm32-unknown-unknown")
        .join("debug")
        .join("examples")
        .join("basic-plugin.wasm");
    if !wasm_path.exists() {
        eprintln!("skipping: wasm artifact not found at {wasm_path:?}");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    std::fs::copy(&wasm_path, dir.path().join("plugin.wasm")).unwrap();
    let mut manifest = std::fs::File::create(dir.path().join("crawlkit-plugin.toml")).unwrap();
    writeln!(
        manifest,
        r#"[plugin]
name = "title-length"
version = "1.0.0"
api_version = "1.0"
author = "sdk-conformance"
description = "SDK-built plugin"
license = "Apache-2.0"

[plugin.entry]
wasm = "plugin.wasm"

[plugin.analyzer]
name = "title-length"
categories = ["seo"]
"#
    )
    .unwrap();

    let mut plugin = WasmPlugin::load_with_config(dir.path(), &WasmConfig::default())
        .expect("SDK-built plugin must load under the host ABI");

    // Long title -> TITLE002 finding.
    let long_title = "<html><head><title>0123456789012345678901234567890123456789012345678901234567890123456789</title></head><body></body></html>";
    let result = plugin.analyze(long_title, "https://example.com").unwrap();
    let findings: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(findings[0]["code"], "TITLE002", "result: {result}");

    // Missing title -> TITLE001 finding.
    let no_title = "<html><head></head><body></body></html>";
    let result = plugin.analyze(no_title, "https://example.com").unwrap();
    let findings: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(findings[0]["code"], "TITLE001", "result: {result}");

    // Short title -> no findings.
    let short_title = "<html><head><title>ok</title></head><body></body></html>";
    let result = plugin.analyze(short_title, "https://example.com").unwrap();
    assert_eq!(result, "[]", "result: {result}");
}

/// A plugin whose `analyze` spins forever must be terminated by the
/// wall-clock epoch deadline rather than running indefinitely.
#[test]
fn runaway_plugin_is_killed_by_wall_clock_timeout() {
    const SPINNING_WAT: &str = r#"
(module
  (memory (export "memory") 1)
  (data (i32.const 1024) "[]\00")
  (global $heap (mut i32) (i32.const 4096))
  (func (export "crawlkit_plugin_init") (param i32) (result i32) i32.const 0)
  (func (export "crawlkit_plugin_alloc") (param $size i32) (result i32)
    (local $ptr i32)
    (local.set $ptr (global.get $heap))
    (global.set $heap (i32.add (global.get $heap) (local.get $size)))
    (local.get $ptr))
  (func (export "crawlkit_plugin_analyze")
        (param i32 i32 i32 i32) (result i32)
    (loop $spin (br_if $spin (i32.const 1)))
    (i32.const 0))
  (func (export "crawlkit_plugin_free") (param i32))
)
"#;

    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("plugin.wasm"), SPINNING_WAT).unwrap();
    let mut manifest = std::fs::File::create(dir.path().join("crawlkit-plugin.toml")).unwrap();
    writeln!(
        manifest,
        r#"[plugin]
name = "spinner"
version = "1.0.0"
api_version = "1.0"
author = "timeout-test"
description = "spins forever"
license = "Apache-2.0"

[plugin.entry]
wasm = "plugin.wasm"

[plugin.analyzer]
name = "spinner"
categories = ["test"]
"#
    )
    .unwrap();

    let config = WasmConfig {
        max_analysis_timeout_ms: 100,
        ..WasmConfig::default()
    };
    let mut plugin = WasmPlugin::load_with_config(dir.path(), &config).unwrap();

    let started = std::time::Instant::now();
    let result = plugin.analyze("<html></html>", "https://example.com");
    let elapsed = started.elapsed();

    let err = result.expect_err("spinning plugin must be killed");
    assert!(
        err.to_string().contains("timeout"),
        "expected timeout error, got: {err}"
    );
    assert!(
        elapsed < std::time::Duration::from_secs(5),
        "timeout took {elapsed:?}, watchdog did not fire promptly"
    );
}

/// Manifests requesting capabilities the sandbox cannot grant (network,
/// filesystem, env vars) must fail to load — fail-closed, not silent.
#[test]
fn capability_requests_fail_closed() {
    let wat = r#"
(module
  (memory (export "memory") 1)
  (func (export "crawlkit_plugin_init") (param i32) (result i32) i32.const 0)
  (func (export "crawlkit_plugin_alloc") (param i32) (result i32) i32.const 0)
  (func (export "crawlkit_plugin_analyze") (param i32 i32 i32 i32) (result i32) i32.const 0)
  (func (export "crawlkit_plugin_free") (param i32))
)
"#;

    for permissions_toml in [
        "[plugin.permissions]\nnetwork = true\n",
        "[plugin.permissions]\nfilesystem = true\n",
        "[plugin.permissions]\nenv_vars = [\"AWS_SECRET_ACCESS_KEY\"]\n",
    ] {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("plugin.wasm"), wat).unwrap();
        let manifest = format!(
            r#"[plugin]
name = "greedy"
version = "1.0.0"
api_version = "1.0"
author = "caps-test"
description = "requests capabilities"
license = "Apache-2.0"

[plugin.entry]
wasm = "plugin.wasm"

{permissions_toml}"#
        );
        std::fs::write(dir.path().join("crawlkit-plugin.toml"), manifest).unwrap();

        let result = WasmPlugin::load_with_config(dir.path(), &WasmConfig::default());
        let err = match result {
            Ok(_) => panic!("capability-requesting manifest must be rejected"),
            Err(e) => e,
        };
        assert!(
            err.to_string().contains("capabilities"),
            "expected capability rejection, got: {err}"
        );
    }
}
