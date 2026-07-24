# Coding Standards & Quality Gates

**crawlkit Engineering Standards**
**Version:** 1.0.0
**Status:** Proposed
**Date:** 2026-07-23

---

## 1. Rust Coding Standards

### 1.1 Naming Conventions (Google Rust Style + Extensions)

| Element | Convention | Example | Bad |
|---------|------------|---------|-----|
| Types | PascalCase | `HttpStatusCode` | `http_status_code` |
| Enums | PascalCase, variants PascalCase | `StatusCode::Ok` | `status_code::ok` |
| Functions | snake_case | `fetch_page()` | `fetchPage()` |
| Methods | snake_case, verb-first | `parse_html()` | `html_parse()` |
| Variables | snake_case | `page_count` | `pageCount` |
| Constants | SCREAMING_SNAKE_CASE | `MAX_RETRY_COUNT` | `maxRetryCount` |
| Module files | snake_case | `http_client.rs` | `HttpClient.rs` |
| Test modules | `#[cfg(test)]` in same file | — | Separate test files |
| Feature flags | kebab-case | `js-rendering` | `jsRendering` |

### 1.2 Function Design Rules

**Single Responsibility:** Each function does exactly one thing. If you need "and" to describe what it does, split it.

```rust
// GOOD: Single responsibility
fn extract_meta_tags(document: &HtmlDocument) -> MetaTags { ... }
fn validate_meta_tags(tags: &MetaTags) -> Vec<Finding> { ... }

// BAD: Two responsibilities
fn extract_and_validate_meta_tags(document: &HtmlDocument) -> (MetaTags, Vec<Finding>) { ... }
```

**Maximum Length:** 30 lines (excluding blank lines and comments). Functions exceeding this must be refactored.

**Parameter Limit:** 5 maximum. Use structs for more.

```rust
// GOOD: Struct for many params
struct FetchConfig {
    timeout: Duration,
    max_retries: u32,
    user_agent: String,
    follow_redirects: bool,
}

fn fetch_page(url: &Url, config: &FetchConfig) -> Result<FetchResult, CrawlError> { ... }

// BAD: Too many params
fn fetch_page(url: &Url, timeout: Duration, retries: u32, agent: &str, follow: bool) -> ... { ... }
```

**Return Values:** Prefer `Result<T, E>` over `Option<T>` for errors. Never return `Box<dyn Error>`.

**Ownership:** Prefer `&str` over `String` in function parameters. Return `String` only when ownership transfer is needed.

### 1.3 Error Handling Standards

**Error Types:** Use `thiserror` for library errors, `anyhow` for application errors.

```rust
// Library error (thiserror)
#[derive(Debug, thiserror::Error)]
pub enum CrawlError {
    #[error("Invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),
    
    #[error("Request failed after {attempts} attempts")]
    MaxRetriesExceeded { attempts: usize },
    
    #[error("Storage error: {0}")]
    Storage(String),
}

// Application error (anyhow) — CLI/API only
fn main() -> anyhow::Result<()> { ... }
```

**Error Propagation:** Use `?` operator. Never `.unwrap()` in production code.

```rust
// GOOD
let response = client.fetch(url).await?;
let page = parser.parse(&response.body)?;

// BAD
let response = client.fetch(url).await.unwrap();
let page = parser.parse(&response.body).unwrap();
```

**`unwrap()` Ban:** Allowed ONLY in:
- Test code (`#[test]` functions)
- `main()` for initialization errors
- `Mutex::lock()` when poison is unrecoverable (document reason)

**`expect()` Ban:** Allowed ONLY with descriptive message explaining why invariant holds.

```rust
// GOOD
let config = Mutex::new(raw_config);
// Invariant: config loaded from valid TOML in main()
let config = config.lock().expect("config initialized in main, before lock");

// BAD
let config = config.lock().expect("lock failed");
```

### 1.4 Memory Safety Rules

**Zero-Allocation Hot Paths:** Crawler fetch-parse-analyze loop must not allocate in steady state.

```rust
// Pre-allocate buffers outside hot loop
let mut html_buffer = Vec::with_capacity(64 * 1024);
let mut findings = Vec::with_capacity(64);

// Reuse across iterations
for url in queue.iter() {
    html_buffer.clear();
    findings.clear();
    // ... process with reuse
}
```

**String Handling:** Prefer `&str` slicing over `String` allocation. Use `Cow<'_, str>` for conditional allocation.

```rust
// GOOD: No allocation
fn extract_attribute<'a>(html: &'a str, attr: &str) -> Option<&'a str> { ... }

// BAD: Unnecessary allocation
fn extract_attribute(html: &str, attr: &str) -> Option<String> { ... }
```

**Collections:** Use `Vec::with_capacity()` when size is predictable. Prefer `&[T]` slices over `&Vec<T>`.

**Lifetime Annotations:** Explicit over elided. Use `'_` only when compiler allows and intent is clear.

### 1.5 Concurrency Rules

**Send + Sync:** All types shared across threads must be `Send + Sync`. Verify with `#[cfg(test)]` compile-time checks.

```rust
// Compile-time verification
fn assert_send_sync<T: Send + Sync>() {}
assert_send_sync::<AnalysisContext>();
assert_send_sync::<Finding>();
```

**Lock Hierarchy:** Always acquire locks in consistent order. Document lock ordering in module comments.

```rust
// Lock ordering: storage → queue → cache
// Acquiring out of order = deadlock risk
```

**Arc/Mutex Pattern:** Prefer `Arc<T>` for shared ownership, `RwLock<T>` over `Mutex<T>` when reads >> writes.

**Async Rules:**
- Use `tokio::spawn()` for independent tasks
- Use `tokio::join!()` for concurrent operations
- Never `.await` inside `Mutex::lock()`
- Use `tokio::sync::Semaphore` for bounded concurrency

### 1.6 Documentation Standards

**Module-Level:** Every `mod.rs` or `lib.rs` gets doc comment explaining purpose.

```rust
//! HTTP client with retry logic and redirect tracking.
//!
//! # Architecture
//! Uses reqwest with rustls-tls backend. Implements token-bucket
//! rate limiting per domain. Supports HTTP/2 multiplexing.
//!
//! # Performance
//! - Connection pooling via reqwest::Client (100 keep-alive)
//! - DNS caching via custom DnsCache
//! - Streaming response bodies for large pages
```

**Function-Level:** All public functions get doc comments with:
- One-line summary
- `# Arguments` section (if non-obvious)
- `# Returns` section (if non-obvious)
- `# Errors` section (if fallible)
- `# Examples` section (for public API)

```rust
/// Fetch a page with retry logic and redirect tracking.
///
/// # Arguments
/// * `url` - Target URL to fetch
/// * `config` - Fetch configuration (timeout, retries, user-agent)
///
/// # Returns
/// * `Ok(FetchResult)` - Successfully fetched page with status, headers, body
/// * `Err(CrawlError::RequestFailed)` - All retry attempts exhausted
/// * `Err(CrawlError::TooManyRedirects)` - Redirect chain exceeds limit
///
/// # Examples
/// ```rust
/// let config = FetchConfig::default();
/// let result = client.fetch(&url, &config).await?;
/// println!("Status: {}", result.status_code);
/// ```
pub async fn fetch(&self, url: &Url, config: &FetchConfig) -> Result<FetchResult, CrawlError> { ... }
```

**Inline Comments:** Explain *why*, not *what*. No comments restating code.

```rust
// GOOD: Explains why
// Retry on 503 (Service Unavailable) — common during deployments
if status == 503 { return self.retry(request).await; }

// BAD: Restates code
// Check if status is 503
if status == 503 { return self.retry(request).await; }
```

---

## 2. FAANG Standards

### 2.1 Code Review Requirements

| Gate | Requirement |
|------|-------------|
| Approvals | ≥ 1 for standard changes |
| Security changes | ≥ 2 approvals (one from security team) |
| Core pipeline | ≥ 2 approvals (one from architect) |
| New analyzer | ≥ 1 approval + test coverage ≥ 90% |
| API changes | ≥ 1 approval + API docs updated |

### 2.2 Complexity Limits

| Metric | Threshold | Action |
|--------|-----------|--------|
| Cyclomatic complexity | ≤ 10 per function | Refactor if exceeded |
| Cognitive complexity | ≤ 15 per function | Refactor if exceeded |
| Nesting depth | ≤ 4 levels | Extract helper functions |
| Function length | ≤ 30 lines | Extract helpers |
| Module length | ≤ 500 lines | Split into submodules |
| File length | ≤ 1000 lines | Split into modules |
| Struct field count | ≤ 12 fields | Use nested structs |

### 2.3 Test Requirements

| Category | Minimum Coverage | Test Type |
|----------|-----------------|-----------|
| Core pipeline (fetch/parse/analyze) | 95% branch | Unit + integration |
| Analyzers | 90% branch | Unit with test vectors |
| Error paths | 85% branch | Unit with error injection |
| Storage layer | 90% branch | Unit + integration |
| API endpoints | 85% branch | Integration + E2E |
| CLI commands | 80% branch | Integration |
| Overall | 90% branch | — |

**Test Naming:** `test_<function>_<scenario>_<expected>`

```rust
#[test]
fn test_extract_meta_tags_missing_title_returns_empty() { ... }

#[test]
fn test_extract_meta_tags_valid_html_extracts_all() { ... }

#[tokio::test]
async fn test_fetch_page_timeout_returns_error() { ... }
```

**Test Structure:** Arrange-Act-Assert (AAA) pattern.

```rust
#[test]
fn test_soft_404_detection() {
    // Arrange
    let html = "<html><body>404 Not Found</body></html>";
    
    // Act
    let is_soft_404 = HttpStatusAnalyzer::is_soft_404(html);
    
    // Assert
    assert!(is_soft_404, "Should detect soft 404 from '404 Not Found'");
}
```

**Test Data:** Use `test_vectors/` directory for complex inputs. Document source and verification method.

### 2.4 Refactoring Triggers

Refactor when you see:
- Duplicate code (≥ 3 similar blocks)
- Comment explaining what code does
- `if/else` that could be a match
- Function with > 2 levels of nesting
- Type with > 5 methods doing different things
- Module with > 10 public functions

---

## 3. HFT/ECN Standards

### 3.1 Zero-Allocation Rules

**Hot Path:** The fetch-parse-analyze loop must not allocate after warmup.

| Operation | Allocation Budget | Strategy |
|-----------|------------------|----------|
| HTML parsing | 0 | Reuse `HtmlParser` buffer |
| Finding creation | 0 | Reuse `Vec<Finding>` |
| String extraction | 0 | Use `&str` slices |
| URL parsing | 0 | Cache parsed URLs |
| JSON serialization | 0 | Reuse `serde_json::Value` buffer |

**Measurement:** Profile with `dhat` or `jemalloc` profiling. Peak RSS < 500MB for 10k pages.

```rust
// Pre-allocate all buffers
struct CrawlBuffers {
    html: Vec<u8>,
    findings: Vec<Finding>,
    urls: Vec<Url>,
    text: String,
}

impl CrawlBuffers {
    fn with_capacity(cap: usize) -> Self {
        Self {
            html: Vec::with_capacity(256 * 1024),  // 256KB
            findings: Vec::with_capacity(128),
            urls: Vec::with_capacity(cap),
            text: String::with_capacity(128 * 1024), // 128KB
        }
    }
}
```

### 3.2 Determinism Rules

**Same Input → Same Output:**
- No `thread_rng()` without explicit seed
- No `SystemTime` for logic (only for timestamps in output)
- No hash iteration order (use `BTreeMap` over `HashMap` for deterministic output)
- Seed-based PRNG for any randomization

```rust
// GOOD: Deterministic
fn process_batch(urls: &[Url], seed: u64) -> Vec<Finding> {
    let mut rng = SmallRng::seed_from_u64(seed);
    // ... deterministic processing
}

// BAD: Non-deterministic
fn process_batch(urls: &[Url]) -> Vec<Finding> {
    let mut rng = rand::thread_rng(); // No!
    // ... non-deterministic
}
```

### 3.3 Latency Budgets

| Operation | P50 Target | P99 Target | Measurement |
|-----------|------------|------------|-------------|
| Single page fetch | < 200ms | < 500ms | reqwest timing |
| HTML parse | < 10ms | < 50ms | Criterion benchmark |
| Analysis (23+ analyzers) | < 50ms | < 100ms | Criterion benchmark |
| Storage write | < 5ms | < 20ms | SQLite WAL commit |
| Total per-page | < 300ms | < 700ms | End-to-end |

**Measurement:** Use `criterion` for micro-benchmarks, `tracing` for production latency.

### 3.4 Backpressure Rules

**Bounded Channels:** All pipeline channels must have bounded capacity.

```rust
// GOOD: Bounded channel with backpressure
let (tx, rx) = tokio::sync::mpsc::channel::<UrlEntry>(1000);
// Producer blocks when channel full → natural backpressure

// BAD: Unbounded channel — memory explosion
let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<UrlEntry>();
```

**Circuit Breaker:** Per-domain circuit breaker prevents cascade failures.

```rust
struct CircuitBreaker {
    failure_count: AtomicU32,
    last_failure: AtomicU64,
    state: AtomicU8, // Closed=0, Open=1, HalfOpen=2
}
```

### 3.5 Exactly-Once Semantics

**Idempotency Key:** `(url, status_code, content_hash)` as composite key.

**Deduplication:** Skip re-crawl if content hash unchanged within TTL.

```rust
// In storage.rs
fn should_skip_crawl(&self, url: &Url, content_hash: u64) -> bool {
    // Check if same URL + hash exists within TTL window
    self.db.query_row(
        "SELECT COUNT(*) FROM pages WHERE url = ?1 AND content_hash = ?2 
         AND crawled_at > datetime('now', '-24 hours')",
        params![url.as_str(), content_hash],
        |row| row.get::<_, i64>(0),
    ).unwrap_or(0) > 0
}
```

---

## 4. Defence Standards

### 4.1 Audit Trail Requirements

**Every state-change event logged:**
- Timestamp (ISO 8601, UTC)
- Event type (CRAWL_START, PAGE_FETCHED, FINDING_STORED, etc.)
- Actor (API key, CLI user, scheduled job)
- Payload hash (SHA-256 of affected data)
- Previous state hash (tamper-evident chaining)

```rust
#[derive(Debug, Clone, Serialize)]
pub struct AuditEvent {
    pub timestamp: DateTime<Utc>,
    pub event_type: EventType,
    pub actor: String,
    pub payload_hash: String,      // SHA-256
    pub previous_hash: String,     // Tamper-evident chain
    pub details: serde_json::Value,
}

pub enum EventType {
    CrawlStarted,
    PageFetched,
    FindingStored,
    ExportGenerated,
    ConfigChanged,
    ApiKeyCreated,
    ApiKeyRevoked,
}
```

**Storage:** Append-only SQLite table with SHA-256 chaining.

```sql
CREATE TABLE audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL,
    event_type TEXT NOT NULL,
    actor TEXT NOT NULL,
    payload_hash TEXT NOT NULL,
    previous_hash TEXT NOT NULL,
    details TEXT NOT NULL
);

CREATE INDEX idx_audit_timestamp ON audit_log(timestamp);
CREATE INDEX idx_audit_actor ON audit_log(actor);
```

### 4.2 Input Validation Rules

**All external input validated at boundary:**

| Input | Validation | Error |
|-------|------------|-------|
| URLs | Valid scheme (http/https), valid host, max 2048 chars | `InvalidUrl` |
| Depth | 0 ≤ depth ≤ 20 | `OutOfScope` |
| Page limit | 1 ≤ limit ≤ 10,000,000 | `ConfigError` |
| Patterns | Valid regex | `InvalidPattern` |
| API keys | Format: `ck_[a-z0-9]{32}` | `Unauthorized` |
| Config files | Valid TOML, required fields present | `ConfigError` |

**SQL Injection Prevention:** Parameterized queries only. No string interpolation.

```rust
// GOOD: Parameterized
db.execute(
    "INSERT INTO pages (url, status) VALUES (?1, ?2)",
    params![url, status],
)?;

// BAD: String interpolation (SQL injection risk)
db.execute(&format!("INSERT INTO pages (url, status) VALUES ('{}', {})", url, status), [])?;
```

### 4.3 Encryption at Rest

**Optional feature:** `sqlcipher` for SQLite, AES-256-GCM for exports.

```toml
[features]
default = []
encryption = ["sqlcipher"]

[dependencies]
sqlcipher = { version = "0.5", optional = true }
```

**Key Management:** File, environment variable, or system keyring. Never hardcoded.

### 4.4 Formal Verification

**Algorithm Verification:** Critical algorithms get Lean4/Coq proofs.

| Algorithm | Verification | Status |
|-----------|-------------|--------|
| Rate limiter (token bucket) | Lean4 proof | Pending |
| PageRank (damping 0.85) | Lean4 proof | Pending |
| URL deduplication (hash) | Test vectors | Done |
| Circuit breaker state machine | TLA+ model check | Pending |

**Proof File Location:** `.specs/02_architecture/proofs/`

---

## 5. Design Patterns

### 5.1 Analyzer Pattern (Strategy + Registry)

```rust
// Strategy pattern: each analyzer implements Analyzer trait
pub trait Analyzer: Send + Sync {
    fn name(&self) -> &str;
    fn analyze(&self, ctx: &AnalysisContext, config: &CrawlConfig) -> Vec<Finding>;
}

// Registry pattern: central registration
pub struct AnalyzerRegistry {
    analyzers: Vec<Box<dyn Analyzer>>,
}

impl AnalyzerRegistry {
    pub fn with_analyzers(analyzers: Vec<Box<dyn Analyzer>>) -> Self {
        Self { analyzers }
    }
    
    pub fn register(&mut self, analyzer: Box<dyn Analyzer>) {
        self.analyzers.push(analyzer);
    }
    
    pub fn analyze(&self, ctx: &AnalysisContext, config: &CrawlConfig) -> Vec<Finding> {
        self.analyzers.iter()
            .flat_map(|a| a.analyze(ctx, config))
            .collect()
    }
}
```

### 5.2 Builder Pattern (Configuration)

```rust
pub struct CrawlConfigBuilder {
    max_pages: usize,
    max_depth: u32,
    concurrency: usize,
    rate_limit: Duration,
    user_agent: String,
}

impl CrawlConfigBuilder {
    pub fn new() -> Self { ... }
    pub fn max_pages(mut self, val: usize) -> Self { ... }
    pub fn max_depth(mut self, val: u32) -> Self { ... }
    pub fn build(self) -> CrawlConfig { ... }
}

// Usage
let config = CrawlConfigBuilder::new()
    .max_pages(1000)
    .max_depth(5)
    .concurrency(10)
    .build();
```

### 5.3 Newtype Pattern (Type Safety)

```rust
// Instead of raw strings everywhere
pub struct PageUrl(Url);
pub struct ContentHash(u64);
pub struct ApiKey(String);

impl PageUrl {
    pub fn new(url: Url) -> Result<Self, CrawlError> {
        if !url.scheme().starts_with("http") {
            return Err(CrawlError::InvalidUrl(url.to_string()));
        }
        Ok(Self(url))
    }
    
    pub fn as_str(&self) -> &str { self.0.as_str() }
}
```

### 5.4 State Machine Pattern (Circuit Breaker)

```rust
pub enum CircuitState {
    Closed { failure_count: u32 },
    Open { opened_at: Instant },
    HalfOpen { test_count: u32 },
}

impl CircuitState {
    pub fn record_success(&self) -> Self {
        match self {
            Self::Closed { .. } => Self::Closed { failure_count: 0 },
            Self::HalfOpen { .. } => Self::Closed { failure_count: 0 },
            Self::Open { .. } => self.clone(),
        }
    }
    
    pub fn record_failure(&self) -> Self {
        match self {
            Self::Closed { failure_count } => {
                if failure_count + 1 >= FAILURE_THRESHOLD {
                    Self::Open { opened_at: Instant::now() }
                } else {
                    Self::Closed { failure_count: failure_count + 1 }
                }
            }
            Self::Open { opened_at } => {
                if opened_at.elapsed() > RECOVERY_TIMEOUT {
                    Self::HalfOpen { test_count: 0 }
                } else {
                    self.clone()
                }
            }
            Self::HalfOpen { .. } => Self::Open { opened_at: Instant::now() },
        }
    }
}
```

### 5.5 Visitor Pattern (Finding Export)

```rust
pub trait FindingVisitor {
    fn visit_http_finding(&mut self, finding: &HttpFinding) -> Result<(), ExportError>;
    fn visit_seo_finding(&mut self, finding: &SeoFinding) -> Result<(), ExportError>;
    fn visit_content_finding(&mut self, finding: &ContentFinding) -> Result<(), ExportError>;
    // ... other finding types
}

pub struct CsvExporter {
    writer: csv::Writer<File>,
}

impl FindingVisitor for CsvExporter {
    fn visit_http_finding(&mut self, finding: &HttpFinding) -> Result<(), ExportError> {
        self.writer.serialize(finding)?;
        Ok(())
    }
    // ...
}
```

---

## 6. Quality Gates

### 6.1 Pre-Commit Gates

| Gate | Tool | Threshold |
|------|------|-----------|
| Formatting | `cargo fmt --check` | 100% pass |
| Linting | `cargo clippy -D warnings` | 0 warnings |
| Unit tests | `cargo test` | 100% pass |
| License check | `cargo deny check` | 0 advisories |
| Secret scan | `trufflehog` | 0 secrets |

### 6.2 CI/CD Gates

| Gate | Tool | Threshold | Blocking |
|------|------|-----------|----------|
| Formatting | `cargo fmt --check` | 100% pass | Yes |
| Linting | `cargo clippy -D warnings` | 0 warnings | Yes |
| Unit tests | `cargo test --workspace` | 100% pass | Yes |
| Integration tests | `cargo test --workspace --features integration` | 100% pass | Yes |
| Coverage (critical) | `cargo tarpaulin` | ≥ 95% branch | Yes |
| Coverage (overall) | `cargo tarpaulin` | ≥ 90% branch | Yes |
| Benchmarks | `cargo bench` | No > 5% regression | Yes |
| Security scan | `cargo audit` | 0 critical | Yes |
| SBOM generation | `cargo sbom` | SPDX format | No |
| Cross-compile | `cross build --target` | 4 targets pass | No |
| Documentation | `cargo doc --no-deps` | 0 warnings | No |

### 6.3 Release Gates

| Gate | Requirement | Blocking |
|------|-------------|----------|
| All CI gates pass | Green CI | Yes |
| Changelog updated | Human review | Yes |
| Version bump | Semantic versioning | Yes |
| Binary size | < 10MB | Yes |
| Startup time | < 100ms | Yes |
| Memory (10k pages) | < 500MB peak RSS | Yes |
| Cross-platform | Linux/macOS/Windows | Yes |
| Documentation | All public APIs | Yes |
| ADRs | All decisions documented | Yes |

### 6.4 Post-Release Gates

| Gate | Frequency | Tool |
|------|-----------|------|
| Dependency audit | Daily | `cargo audit` |
| Performance regression | Per-commit | `cargo bench` + CI |
| Memory leak detection | Weekly | `valgrind --leak-check` |
| Security scan | Weekly | `trufflehog` + `cargo audit` |
| Code coverage trend | Weekly | `cargo tarpaulin` |

---

## 7. Toolchain Requirements

| Tool | Version | Purpose | Gate |
|------|---------|---------|------|
| `rustfmt` | Latest stable | Code formatting | Pre-commit, CI |
| `clippy` | Latest stable | Linting | Pre-commit, CI |
| `cargo-tarpaulin` | 0.27+ | Code coverage | CI |
| `cargo-deny` | 0.14+ | License/advisory check | CI |
| `cargo-audit` | 0.18+ | Security audit | CI |
| `criterion` | 0.5+ | Benchmarking | CI |
| `trufflehog` | 3.6+ | Secret scanning | CI |
| `cross` | 0.25+ | Cross-compilation | Release |
| `sqlx-cli` | 0.7+ | DB migrations | Development |

---

## 8. Configuration

### 8.1 rustfmt.toml

```toml
edition = "2021"
max_width = 100
tab_spaces = 4
use_field_init_shorthand = true
use_try_shorthand = true
```

### 8.2 clippy.toml

```toml
cognitive-complexity-threshold = 15
too-many-arguments-threshold = 5
type-complexity-threshold = 250
```

### 8.3 deny.toml

```toml
[advisories]
vulnerability = "deny"
unmaintained = "warn"
yanked = "warn"

[licenses]
allow = [
    "MIT",
    "Apache-2.0",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "Unicode-DFS-2016",
]
```

---

*Generated: 2026-07-23 | Version: 1.0.0 | Status: Proposed*
