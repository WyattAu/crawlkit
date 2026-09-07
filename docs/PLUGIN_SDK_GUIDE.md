# crawlkit Plugin SDK Guide

A complete guide to building, testing, signing, and publishing crawlkit
WASM plugins with `crawlkit-plugin-sdk`.

> **Audience:** Rust developers who want to write custom SEO analyzers that
> run inside crawlkit's sandboxed WASM runtime. No prior WASM experience is
> required — the SDK generates all the FFI plumbing for you.

---

## Quick Start (5 minutes)

### 1. Add the SDK to your `Cargo.toml`

```bash
cargo new my-plugin --lib
cd my-plugin
cargo add crawlkit-plugin-sdk
```

`crates/my-plugin/Cargo.toml`:

```toml
[package]
name = "my-plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
crawlkit-plugin-sdk = "5"
```

The `cdylib` crate type is what produces the `.wasm` artifact; keep `rlib`
so you can unit-test the analyzer natively.

### 2. Implement the `Analyzer` trait

```rust
use crawlkit_plugin_sdk::{AnalysisContext, Analyzer, Finding, Severity};

pub struct TitleCaseAnalyzer;

impl TitleCaseAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TitleCaseAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for TitleCaseAnalyzer {
    fn name(&self) -> &str {
        "title-case"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let Some(start) = ctx.html.find("<title>") else {
            return vec![];
        };
        let Some(end) = ctx.html[start + 7..].find("</title>") else {
            return vec![];
        };
        let title = &ctx.html[start + 7..start + 7 + end];
        if title.chars().next().is_some_and(|c| c.is_lowercase()) {
            vec![Finding {
                severity: Severity::Warning,
                category: "seo".into(),
                code: "TITLE010".into(),
                title: "Title starts with a lowercase letter".into(),
                description: format!("Title is {title:?}"),
                url: ctx.url.clone(),
                recommendation: "Capitalise the first word of the title".into(),
            }]
        } else {
            vec![]
        }
    }
}
```

### 3. Export with the `export_analyzer!` macro

```rust
crawlkit_plugin_sdk::export_analyzer!(TitleCaseAnalyzer);
```

That single line generates every ABI symbol the crawlkit host expects
(see [Anatomy of a Plugin](#anatomy-of-a-plugin) for what is generated).

### 4. Build for `wasm32-unknown-unknown`

```bash
rustup target add wasm32-unknown-unknown
cargo build --target wasm32-unknown-unknown --release
# artifact: target/wasm32-unknown-unknown/release/my_plugin.wasm
```

### 5. Sign and install

```bash
# One-time: create an ed25519 signing keypair
crawlkit plugin keygen --out keys/

# Lay out the plugin directory
mkdir -p my-plugin-pkg
cp target/wasm32-unknown-unknown/release/my_plugin.wasm my-plugin-pkg/plugin.wasm
# ...create crawlkit-plugin.toml (see "Signing and Trust" below)...

# Hash + sign the .wasm, recording trust fields in the manifest
crawlkit plugin sign --plugin my-plugin-pkg/ --key keys/plugin-signing.key

# Verify with the same code path the loader uses
crawlkit plugin verify --plugin my-plugin-pkg/

# Install from an index (verifies the trust chain first)
crawlkit plugin install my-plugin --index plugin-index.toml
```

Done. The analyzer now runs inside crawlkit's sandbox on every crawl that
enables it.

---

## Anatomy of a Plugin

A plugin is a WASM module that exports a small set of C-ABI functions.
The SDK's `export_analyzer!` macro generates them; here is what each one
does and why it exists.

### The full source, annotated

```rust
use crawlkit_plugin_sdk::{AnalysisContext, Analyzer, Finding, Severity};

/// 1. Your analyzer type. It must be `Send`-free friendly, cheap to
///    construct, and provide `new()` — the host calls it exactly once
///    during `crawlkit_plugin_init`.
pub struct ImageAltAnalyzer;

impl ImageAltAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ImageAltAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// 2. The analysis logic. `analyze` is called once per page.
impl Analyzer for ImageAltAnalyzer {
    /// Stable identifier used in reports, audit logs, and the marketplace.
    fn name(&self) -> &str {
        "image-alt"
    }

    /// Receive the page context, return zero or more findings.
    /// Must be total: never panic (a panic traps the WASM module and the
    /// host skips the page).
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let img_count = ctx.html.matches("<img").count();
        let alt_count = ctx.html.matches("alt=\"").count();

        if img_count > alt_count {
            vec![Finding {
                severity: Severity::Warning,
                category: "accessibility".into(),
                code: "A11Y001".into(),
                title: "Images missing alt text".into(),
                description: format!(
                    "Found {img_count} images but only {alt_count} have alt text"
                ),
                url: ctx.url.clone(),
                recommendation: "Add descriptive alt attributes to all images".into(),
            }]
        } else {
            vec![]
        }
    }
}

/// 3. The ABI export. Generates:
///    - crawlkit_plugin_init(reserved) -> i32
///    - crawlkit_plugin_alloc(size) -> ptr
///    - crawlkit_plugin_analyze(html_ptr, html_len, url_ptr, url_len) -> ptr
///    - crawlkit_plugin_free(ptr)
///    - crawlkit_plugin_free_string(ptr)   (legacy alias)
///    - crawlkit_plugin_api_version() -> ptr ("1.0")
crawlkit_plugin_sdk::export_analyzer!(ImageAltAnalyzer);
```

### The host ABI, function by function

| Export | Signature | Purpose |
|---|---|---|
| `crawlkit_plugin_init` | `(reserved: usize) -> i32` | Constructs your analyzer. `0` = success. |
| `crawlkit_plugin_alloc` | `(size: usize) -> ptr` | Hands the host a guest buffer to write page input into. |
| `crawlkit_plugin_analyze` | `(html_ptr, html_len, url_ptr, url_len) -> ptr` | Runs your analyzer; returns a pointer to a NUL-terminated JSON array of `Finding`s (or `0` on failure). |
| `crawlkit_plugin_free` | `(ptr)` | Releases any pointer from `alloc`/`analyze`. |
| `crawlkit_plugin_api_version` | `() -> ptr` | Returns the string `"1.0"` for host diagnostics. |

### Memory layout notes

- Allocations carry a 4-byte little-endian size header immediately before
  the returned pointer, so `free` can reconstruct the original layout.
- The host owns guest memory it allocates via `crawlkit_plugin_alloc`;
  after `crawlkit_plugin_analyze` returns, the host reads the result up to
  the NUL terminator and then calls `crawlkit_plugin_free`.
- `usize` parameters are 32-bit on `wasm32-unknown-unknown`
  (ABI-identical to the host's `i32`) and 64-bit on native targets, which
  is why the ABI can be unit-tested without WASM (see
  [Testing Your Plugin](#testing-your-plugin)).

### Minimal manifest (`crawlkit-plugin.toml`)

The host will not load your `.wasm` without a sibling manifest:

```toml
[plugin]
name = "image-alt"
version = "1.0.0"
api_version = "1.0"
author = "you <you@example.com>"
description = "Flags images missing alt text"
license = "Apache-2.0"

[plugin.entry]
wasm = "plugin.wasm"

# Filled in by `crawlkit plugin sign` — see "Signing and Trust".
# wasm_hash = "…"
# signature = "…"
# signed_by = "…"
```

---

## The AnalysisContext

`AnalysisContext` is the data your plugin receives for every page:

```rust
pub struct AnalysisContext {
    /// The page URL, e.g. "https://example.com/blog/post-1".
    pub url: String,
    /// The full HTML body as a string.
    pub html: String,
    /// HTTP status code, when the host captured it (may be `None`).
    pub status_code: Option<u16>,
    /// Response headers as (name, value) pairs (may be empty).
    pub headers: Vec<(String, String)>,
    /// Server response time in milliseconds, when known.
    pub response_time_ms: Option<u64>,
}
```

Practical guidance:

- **`html` is the raw document.** The SDK intentionally has no DOM API
  (string scanning keeps plugins small and fast). For *parsed* page data —
  title, meta description, canonical, headings, word counts, link/image
  counts, `lang` — use the [Host Context API](#host-context-api); the host
  precomputes it so you don't re-parse.
- **`status_code`/`headers` are best-effort.** A plugin may be exercised
  through a plain `analyze` path where they are absent (e.g. the
  `crawlkit_plugin_analyze` ABI only carries HTML + URL). Always handle
  `None` gracefully. When you need response metadata reliably, prefer
  `crawlkit_plugin_sdk::host::context()`.
- **Treat it as read-only and transient.** Nothing persists between calls
  unless you manage it yourself (see [Advanced Patterns](#advanced-patterns)).

---

## Returning Findings

`analyze` returns `Vec<Finding>`. Return an empty `Vec` when the page is
clean — never return placeholder findings.

```rust
pub struct Finding {
    pub severity: Severity,
    pub category: IssueCategory,
    /// Machine-readable code, unique within your plugin, e.g. "IMG001".
    pub code: String,
    /// Short human-readable title.
    pub title: String,
    /// Detailed description (include the measured value where possible).
    pub description: String,
    /// URL of the page the finding applies to — pass `ctx.url.clone()`.
    pub url: String,
    /// Actionable fix guidance.
    pub recommendation: String,
}
```

### Severity levels

| `Severity` | Meaning | Typical use |
|---|---|---|
| `Critical` | Immediate attention required | Security holes, pages unreachable to crawlers |
| `Error` | Should be fixed | Missing `<title>`, broken canonical |
| `Warning` | Improvement suggested | Oversized title, missing alt text |
| `Info` | Informational note | Missing canonical, no meta description |

Serialized as lowercase strings (`"critical"`, `"error"`, `"warning"`,
`"info"`) in reports and the database.

### Categories

`IssueCategory` groups findings for filtering and reporting. Fixed
variants: `Http`, `Seo`, `Content`, `Links`, `Images`, `Schema`,
`Security`, `Performance`, `Mobile`, `Accessibility`, `Social`. Any other
value becomes `Custom(name)`.

Because `From<&str>` is implemented, you can write categories as strings:

```rust
category: "seo".into(),            // IssueCategory::Seo
category: "accessibility".into(),  // IssueCategory::Accessibility
category: "custom:my-check".into() // IssueCategory::Custom("my-check")
```

Prefer a fixed variant when one matches; reserve `Custom` for analysis
domains the core does not model.

### Code conventions

- Codes are stable identifiers machines key on: `<DOMAIN><NNN>` — e.g.
  `TITLE001`, `SEO002`, `A11Y01`, `VP002`, `SOFT404`.
- Never renumber or repurpose a code in a minor release; downstream
  dashboards filter on them.

---

## Host Context API

Some information is expensive or unreliable to derive from raw HTML. The
host therefore links a `crawlkit_host.get_context` import for every
plugin. The SDK wraps it in `crawlkit_plugin_sdk::host`:

```rust
use crawlkit_plugin_sdk::host;

match host::context() {
    Some(Ok(host_ctx)) => { /* structured metadata available */ }
    Some(Err(_)) => { /* version skew between host and SDK: degrade */ }
    None => { /* host provided no context for this call */ }
}
```

### `HostContext`

```rust
pub struct HostContext {
    pub url: String,
    pub status_code: Option<u16>,
    pub response_time_ms: Option<u64>,
    pub headers: Vec<(String, String)>,
    /// Precomputed parsed-page summary (may be `None`).
    pub parsed: Option<ParsedSummary>,
}

pub struct ParsedSummary {
    pub title: Option<String>,
    pub description: Option<String>,
    pub canonical: Option<String>,
    pub word_count: usize,
    pub sentence_count: usize,
    pub headings: Vec<HeadingSummary>, // { level: u8, text: String }
    pub link_count: usize,
    pub image_count: usize,
    pub lang: Option<String>,
}
```

### Worked example: soft-404 detection

The engine ships this pattern as `examples/soft-404.rs` — flag pages that
returned 4xx/5xx but were still analyzed (soft-404s), using *only* host
metadata, no HTML parsing:

```rust
use crawlkit_plugin_sdk::host::{self, HostContext};
use crawlkit_plugin_sdk::{AnalysisContext, Analyzer, Finding, Severity};

struct Soft404Detector;

impl Soft404Detector {
    pub fn new() -> Self { Self }
}
impl Default for Soft404Detector {
    fn default() -> Self { Self::new() }
}

fn findings_for(ctx: &HostContext) -> Vec<Finding> {
    let Some(code @ 400..=599) = ctx.status_code else {
        return vec![];
    };
    vec![Finding {
        severity: Severity::Warning,
        category: "seo".into(),
        code: "SOFT404".into(),
        title: "Error page analyzed".into(),
        description: format!("This URL returned HTTP {code}; error responses \
                              reachable from internal links are soft-404s."),
        url: ctx.url.clone(),
        recommendation: "Fix internal links pointing at error responses.".into(),
    }]
}

impl Analyzer for Soft404Detector {
    fn name(&self) -> &str { "soft-404" }

    fn analyze(&self, _ctx: &AnalysisContext) -> Vec<Finding> {
        match host::context() {
            Some(Ok(host_ctx)) => findings_for(&host_ctx),
            _ => vec![], // degrade silently when no context is available
        }
    }
}

crawlkit_plugin_sdk::export_analyzer!(Soft404Detector);
```

Notes:

- The import is only linked into your module if you reference the `host`
  module — plugins that never call it are unchanged.
- In *native* unit tests `host::context()` returns `None` (no WASM host is
  present), which is why the example keeps the finding logic in a pure
  `findings_for(&HostContext)` function you can test with a hand-built
  `HostContext`.

---

## Signing and Trust

crawlkit verifies a plugin's integrity through a hash + signature trust
chain (ADR-006) *before* the WASM module is ever instantiated.

### The chain

1. `wasm_hash` — the hex sha256 digest of your `.wasm` artifact.
2. `signature` — an **ed25519 signature over the raw 32-byte digest**
   (not the hex string), produced by your signing key.
3. `signed_by` — the *key id*: the first 16 hex characters of the
   signer's public key.

The loader recomputes the digest, looks `signed_by` up in its built-in
`TRUSTED_PLUGIN_KEYS` trust store, and verifies the signature. Unknown
signers, hash mismatches, and malformed manifests fail closed.

### Generating keys

```bash
crawlkit plugin keygen --out keys/
# keys/plugin-signing.key  (hex ed25519 seed — keep secret)
# keys/plugin-signing.pub  (hex public key)
```

Treat `plugin-signing.key` like a production secret: the file on disk
should live in a secure key store in CI; first-party crawlkit signing
secrets never appear in the repository.

### Signing

```bash
crawlkit plugin sign --plugin my-plugin-pkg/ --key keys/plugin-signing.key
```

This hashes the `.wasm` referenced by `[plugin.entry] wasm`, signs the
digest, and writes `wasm_hash`, `signature`, and `signed_by` into
`crawlkit-plugin.toml`. **Re-sign after every rebuild** — any byte change
to the `.wasm` invalidates the recorded hash.

### Verification and policies

```bash
crawlkit plugin verify --plugin my-plugin-pkg/
```

runs the same checks the loader performs. Hosts may configure plugin
trust policies:

- **Required** — plugins must carry a valid signature from a trusted key
  (production posture).
- **Permissive** — unsigned plugins load with reduced trust (development).

Plugins signed with your own key only load under *Required* if your
public key is added to the engine's `TRUSTED_PLUGIN_KEYS` trust store —
this is what the marketplace verification flow governs for published
plugins.

---

## Testing Your Plugin

### 1. Native unit tests

Because `Analyzer` is a plain Rust trait, test your logic without WASM:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crawlkit_plugin_sdk::{AnalysisContext, Severity};

    fn ctx(html: &str) -> AnalysisContext {
        AnalysisContext {
            url: "https://example.com".into(),
            html: html.into(),
            status_code: Some(200),
            headers: vec![],
            response_time_ms: None,
        }
    }

    #[test]
    fn flags_images_without_alt() {
        let findings = ImageAltAnalyzer.analyze(&ctx(
            r#"<html><body><img src="a.png"></body></html>"#,
        ));
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "A11Y001");
        assert_eq!(findings[0].severity, Severity::Warning);
    }

    #[test]
    fn clean_page_yields_no_findings() {
        let findings = ImageAltAnalyzer.analyze(&ctx(
            r#"<html><body><img src="a.png" alt="A"></body></html>"#,
        ));
        assert!(findings.is_empty());
    }
}
```

Run with `cargo test`.

### 2. WASM ABI tests (no browser needed)

The generated exports are callable natively (the `usize` parameters are
ABI-identical to `i32` under wasm32). Simulate exactly what the host does:
allocate, copy input, call `crawlkit_plugin_analyze`, read the
NUL-terminated JSON, free every pointer:

```rust
#[test]
fn abi_roundtrip() {
    let _ = crawlkit_plugin_init(0);
    let html = r#"<html><body><img src="a.png"></body></html>"#;
    let url = "https://example.com";

    let html_ptr = crawlkit_plugin_alloc(html.len());
    let url_ptr = crawlkit_plugin_alloc(url.len());
    unsafe {
        std::ptr::copy_nonoverlapping(html.as_ptr(), html_ptr as *mut u8, html.len());
        std::ptr::copy_nonoverlapping(url.as_ptr(), url_ptr as *mut u8, url.len());
    }

    let result_ptr =
        unsafe { crawlkit_plugin_analyze(html_ptr, html.len(), url_ptr, url.len()) };
    assert_ne!(result_ptr, 0);

    // Read NUL-terminated JSON, then free all three allocations.
    let mut len = 0usize;
    unsafe { while *(result_ptr as *const u8).add(len) != 0 { len += 1 } }
    let json = unsafe {
        String::from_utf8(
            std::slice::from_raw_parts(result_ptr as *const u8, len).to_vec(),
        ).unwrap()
    };
    crawlkit_plugin_free(html_ptr);
    crawlkit_plugin_free(url_ptr);
    crawlkit_plugin_free(result_ptr);

    assert!(json.contains(r#""code":"A11Y001""#));
}
```

The SDK's own test suite (`crates/crawlkit-plugin-sdk/src/export.rs`)
follows this pattern; copy those tests as a starting point.

### 3. Engine conformance tests

`crawlkit-engine` runs `wasm_abi_tests`: an end-to-end conformance suite
that compiles the SDK examples (e.g. `basic-plugin.rs`) to real
`wasm32-unknown-unknown` modules, loads them in wasmtime, and proves the
SDK-generated ABI and the host loader agree. Building your plugin as an
example of the SDK crate lets you ride the same harness.

### 4. Full integration

Run the engine with your installed plugin enabled and check findings end
to end. For API-level behaviour (ratings, downloads, verification badges),
see the marketplace endpoints below.

---

## Publishing to the Marketplace

### 1. Prepare a release

- Bump `version` in the manifest (semver `X.Y.Z`, validated by the
  tooling).
- Rebuild `--release`, re-sign, and `crawlkit plugin verify` the
  directory.

### 2. Add the plugin to the index

The first-party index is a versioned TOML file
(`plugins/index/plugin-index.toml`). Add one entry:

```toml
[[plugin]]
name = "image-alt"
version = "1.0.0"
api_version = "1.0"
author = "you <you@example.com>"
description = "Flags images missing alt text"
license = "Apache-2.0"
categories = ["accessibility"]
wasm_path = "artifacts/image-alt-1.0.0.wasm"
wasm_hash = "<sha256 hex of the .wasm>"
signature = "<ed25519 signature hex over the hash>"
signed_by = "<key id>"
```

Submit a pull request adding the entry and the signed artifact under
`artifacts/`. Installation fetches the artifact, re-verifies the hash and
signature against the trust store, and only then materializes the plugin
directory.

### 3. Register in the marketplace API

The marketplace service mirrors index metadata and adds ratings,
downloads, and verification badges:

```bash
# Publish (requires marketplace:write)
curl -X POST https://api.crawlkit.dev/api/v1/marketplace/plugins \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
        "name": "image-alt",
        "version": "1.0.0",
        "author": "you <you@example.com>",
        "description": "Flags images missing alt text",
        "license": "Apache-2.0",
        "categories": ["accessibility"],
        "tags": ["images", "a11y"]
      }'
```

Users then interact with your plugin through:

| Endpoint | Purpose |
|---|---|
| `GET /api/v1/marketplace/plugins` | List all plugins |
| `GET /api/v1/marketplace/plugins/{name}` | Plugin details |
| `GET /api/v1/marketplace/plugins/search?q=&category=` | Full-text search over names, descriptions, authors, tags; optional category filter |
| `POST /api/v1/marketplace/plugins/{name}/download` | Record a download (increments the counter) |
| `POST /api/v1/marketplace/plugins/{name}/rate` | Submit a 0.0–5.0 rating |
| `POST /api/v1/marketplace/plugins/{name}/test` | Execute the plugin against a test input |
| `POST /api/v1/marketplace/plugins/{name}/verify` | Admin-only: award the verified badge |

The **verified** badge is applied by registry maintainers after they
confirm the artifact in the index matches a trusted signature — it is the
marketplace-facing counterpart of the engine's trust chain.

---

## Advanced Patterns

### Stateful plugins

WASM modules are single-threaded, so interior mutability with `static mut`
(or `Cell`/`RefCell` wrappers) is safe and is exactly how the SDK's own
generated code stores the analyzer instance. To accumulate state across
pages *within one module instance*, keep counters in the analyzer struct
and wrap in a `RefCell`:

```rust
use std::cell::RefCell;

pub struct LatencyHistogram {
    buckets: RefCell<Vec<(u64, usize)>>, // (threshold_ms, count)
}

impl Analyzer for LatencyHistogram {
    fn name(&self) -> &str { "latency-histogram" }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        if let Some(ms) = ctx.response_time_ms {
            let mut buckets = self.buckets.borrow_mut();
            for (threshold, count) in buckets.iter_mut() {
                if ms >= *threshold {
                    *count += 1;
                }
            }
        }
        vec![] // emit summary via the host's teardown/reporting hook
    }
}
```

Rules of thumb:

- Never spawn threads or use `std::sync` primitives — the module is
  single-threaded and WASM has no OS threads.
- `init` runs once per module instantiation; don't assume pages arrive in
  crawl order.
- Keep state bounded — WASM linear memory is finite and a trap (OOM) skips
  the rest of the crawl's plugin work for that page.

### Multiple findings per page

Return as many findings as the page warrants — the ABI is an array, not a
single value:

```rust
use crawlkit_plugin_sdk::{AnalysisContext, Finding, IssueCategory, Severity};

fn finding(
    ctx: &AnalysisContext,
    severity: Severity,
    code: &str,
    title: &str,
    recommendation: &str,
) -> Finding {
    Finding {
        severity,
        category: IssueCategory::Seo,
        code: code.into(),
        title: title.into(),
        description: title.into(),
        url: ctx.url.clone(),
        recommendation: recommendation.into(),
    }
}

fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
    let mut findings = Vec::new();

    if !ctx.html.contains("<title") {
        findings.push(finding(ctx, Severity::Error, "SEO001",
            "Missing title tag", "Add a <title> in <head>"));
    }
    if !ctx.html.contains("meta name=\"description\"") {
        findings.push(finding(ctx, Severity::Warning, "SEO002",
            "Missing meta description", "Add a meta description tag"));
    }
    if !ctx.html.contains("<h1") {
        findings.push(finding(ctx, Severity::Warning, "SEO003",
            "Missing H1", "Add exactly one H1 per page"));
    }
    findings
}
```

See `examples/seo_checker.rs` for the complete five-check version.

### Performance optimization

Plugins run in-process with the crawl; hot loops are felt directly.

- **Scan once, test many.** `str::find`/`matches` are memchr-fast, but
  repeated lowercase transformations of a 1 MB page add up. The
  `viewport-checker` example bounds case-insensitive matching to the first
  4096 chars (where `<head>` lives) instead of lowercasing the whole
  document.
- **Prefer the host context over re-parsing.** `host::context()` hands you
  title, word counts, heading structure, and link/image counts for free —
  don't rebuild a parser.
- **Avoid regex crates.** They inflate `.wasm` size and compile time;
  structured string scans cover virtually all SEO checks.
- **Size `Vec`s up front** when the count is known
  (`Vec::with_capacity`) and return early — an empty `Vec` is the cheapest
  path and keeps cold pages fast.
- **Watch artifact size.** `cargo build --release` + default opt settings
  keep a typical plugin under ~100 KB; every byte is fetched and verified
  on install.

### Versioning and compatibility

- `api_version = "1.0"` in the manifest must match what the host expects;
  the engine refuses to load plugins advertising an incompatible ABI.
- If your analyzer reads `HostContext`, code defensively: `parsed` and
  individual fields may be absent, and `host::context()` can return
  `Some(Err(_))` on engine/SDK version skew. Degrade to HTML-only checks
  rather than returning nothing.
