# ADR-001: Crawlkit Crawler Architecture

**Status:** Proposed  
**Date:** 2026-07-22  
**Author:** Wyatt Au  
**Deciders:** Wyatt Au  
**Reviewers:** TBD  
**Supersedes:** N/A  
**Amends:** N/A

---

## Context

### Problem Statement

Existing site crawling and SEO analysis tools—most notably Ahrefs—provide valuable but fundamentally incomplete capabilities. They suffer from several critical limitations:

1. **Incomplete Redirect Handling**: Most crawlers follow only 1–2 redirect hops, silently dropping deeper chains. This masks misconfigurations, creates blind spots in link equity analysis, and fails to detect redirect loops or excessive chains that degrade performance and crawl budget.

2. **Selective URL Checking**: Crawlers often prioritize discovered internal links and sitemap entries, but neglect canonical references, hreflang alternates, and externally-linked URLs. This produces an incomplete picture of a site's link graph and hreflang implementation.

3. **Shallow Technical Analysis**: While tools like Ahrefs report on crawlability and basic on-page factors, they lack deep inspection of structured data validity, security header posture, Core Web Vitals measurement, and accessibility compliance—dimensions that directly impact search rankings and user experience.

4. **No Comparative Analysis**: There is no first-class support for comparing two crawl snapshots to detect regressions, new issues, or improvements over time—essential for iterative SEO work and post-deployment validation.

5. **Export and Integration Limitations**: Export options are typically limited to CSV or proprietary dashboards. There is no native support for SQLite (enabling ad-hoc SQL queries), HTML reports, or structured JSON suitable for downstream pipeline integration.

6. **Black-Box Architecture**: Existing tools are SaaS platforms with opaque algorithms. Users cannot customize crawling behavior, scoring logic, or output schemas to fit their specific workflows.

### Goals

Build a standalone Rust binary (`crawlkit`) that:

- Crawls a site exhaustively, following **all** redirects (not just 1–2 hops) and tracking full redirect chains.
- Validates **every** URL encountered: sitemap entries, internal links, external links, canonical references, and hreflang alternates.
- Detects trailing slash mismatches and canonicalization issues.
- Validates structured data (JSON-LD schemas) against schema.org specifications.
- Checks security headers (CSP, HSTS, X-Frame-Options, etc.) against best-practice baselines.
- Measures Core Web Vitals (LCP, FID/INP, CLS) via integration with headless Chromium performance APIs.
- Validates accessibility against WCAG 2.1 AA guidelines.
- Checks mobile-friendliness (viewport configuration, touch targets, font sizing).
- Analyzes content quality: readability scores (Flesch-Kincaid, Coleman-Liau), keyword density, heading hierarchy, image alt-text coverage.
- Exports data in multiple formats: CSV, JSON, SQLite, and self-contained HTML reports.
- Supports crawl comparison (diff between two crawl snapshots).
- Ships as a single, cross-platform binary with zero runtime dependencies beyond the OS.

### Constraints

- **Language**: Rust (for performance, safety, and single-binary distribution).
- **Concurrency**: Must handle thousands of concurrent connections efficiently without blocking.
- **Politeness**: Must respect `robots.txt`, rate limits, and crawl-delay directives.
- **Memory**: Must handle large sites (100k+ pages) without unbounded memory growth.
- **Network**: Must handle TLS, HTTP/2, HTTP/3, and certificate verification.
- **Portability**: Must compile and run on Linux, macOS, and Windows.

---

## Decision

We will build `crawlkit` as a Rust binary with the following architecture:

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                          crawlkit CLI                              │
├─────────────────────────────────────────────────────────────────────┤
│  Configuration Layer (TOML/YAML/CLI args + environment variables)  │
├──────────┬──────────┬──────────┬──────────┬──────────┬─────────────┤
│  Crawl   │ Redirect │  URL     │ Analyzer │ Reporter │  Scheduler  │
│  Engine  │ Tracker  │ Validator│ Pipeline │  Engine  │  & Rate     │
│          │          │          │          │          │  Limiter    │
├──────────┴──────────┴──────────┴──────────┴──────────┴─────────────┤
│                        Core Runtime (tokio)                        │
├──────────┬──────────┬──────────┬──────────┬──────────┬─────────────┤
│   HTTP   │  TLS/    │  DNS     │  Cache   │  SQLite  │   Logging   │
│   Client │  HTTP2+3 │ Resolver │  Layer   │   Store  │  & Telemetry│
└──────────┴──────────┴──────────┴──────────┴──────────┴─────────────┘
```

### Core Components

#### 1. Crawl Engine

The crawl engine is the central coordinator. It manages:

- **URL Frontier**: A priority queue (min-heap by priority score) with deduplication via a concurrent hash set (`dashmap`). URLs are scored by estimated importance (depth, inlink count, sitemap priority).
- **Worker Pool**: A configurable number of async tasks (default: 64) that pull URLs from the frontier, fetch them, and push discovered URLs back.
- **Politeness Layer**: Per-domain rate limiting with configurable `robots.txt` compliance. Respects `Crawl-delay` directives. Implements a `robots.txt` cache with TTL-based invalidation.
- **Depth Control**: Configurable maximum crawl depth (default: 10). Depth is computed as shortest path from a seed URL.
- **Scope Control**: Domain allowlists/blocklists, path pattern matching (glob and regex), and crawl scope isolation (e.g., crawl only `/blog/` subtree).

```rust
pub struct CrawlEngine {
    frontier: UrlFrontier,
    workers: JoinSet<CrawlTask>,
    politeness: PolitenessLayer,
    config: CrawlConfig,
    state: Arc<CrawlState>,
}

pub struct UrlFrontier {
    queue: BinaryHeap<PrioritizedUrl>,
    seen: DashSet<UrlKey>,
    domain_budgets: DashMap<String, DomainBudget>,
}
```

#### 2. Redirect Tracker

Captures the **complete** redirect chain for every URL, not just the final destination:

```rust
pub struct RedirectChain {
    pub original_url: Url,
    pub hops: Vec<RedirectHop>,
    pub final_url: Url,
    pub total_hops: usize,
    pub is_self_redirect: bool,      // A → A (loop)
    pub is_chain_too_long: bool,      // > 5 hops (configurable)
    pub has_mixed_protocols: bool,    // http → https → http
    pub has_domain_change: bool,      // cross-domain redirect
}

pub struct RedirectHop {
    pub from: Url,
    pub to: Url,
    pub status: u16,                  // 301, 302, 307, 308, etc.
    pub location_header: Option<String>,
    pub relative_redirect: bool,      // Location without full URL
    pub latency: Duration,
    pub timestamp: DateTime<Utc>,
}
```

Key behaviors:
- Follows **all** redirects up to a configurable maximum (default: 20 hops).
- Detects redirect loops and terminates with a diagnostic error.
- Preserves the full chain for analysis (useful for diagnosing redirect chains, detecting soft-404s masquerading as redirects, and identifying unnecessary hops).
- Reports on redirect patterns: chains that could be shortened, redirect loops, and mixed-protocol redirects.

#### 3. URL Validator

Validates every URL encountered during the crawl:

```rust
pub struct UrlValidator {
    canonical_checker: CanonicalChecker,
    trailing_slash_checker: TrailingSlashChecker,
    hreflang_validator: HreflangValidator,
    sitemap_validator: SitemapValidator,
}
```

**Canonical Validation**:
- Compares `<link rel="canonical">` targets against the URL being fetched.
- Detects canonical targets that return non-200 status codes.
- Detects canonical chains (A canonicalizes to B, which canonicalizes to C).
- Detects self-referencing canonicals on pages that don't need them.
- Flags canonicals pointing to different domains without justification.

**Trailing Slash Mismatch Detection**:
- For every URL, fetches both `/path` and `/path/` (configurable).
- Detects inconsistent behavior: one returns 200, the other 301.
- Identifies trailing slash mismatches between canonical tags, internal links, and sitemap entries.
- Reports on pages where trailing slash behavior differs from the site's stated preference.

**Hreflang Validation**:
- Parses all `<link rel="alternate" hreflang="...">` tags.
- Validates that hreflang values are valid BCP 47 language tags.
- Checks for reciprocal hreflang declarations (if A links to B, B must link back to A).
- Detects missing `x-default` hreflang for internationalized sites.
- Validates that hreflang targets actually return 200 status codes.
- Detects hreflang targets that canonicalize elsewhere (broken hreflang implementation).

**Sitemap Validation**:
- Parses XML sitemaps and sitemap index files.
- Cross-references sitemap URLs against discovered crawl URLs.
- Detects URLs in sitemaps that are noindex, nofollow, or robots.txt-blocked.
- Validates `<lastmod>`, `<changefreq>`, and `<priority>` values against observed behavior.
- Detects sitemap bloat (URLs in sitemap that don't exist or return errors).

#### 4. Analyzer Pipeline

A composable pipeline of analysis modules, each operating on the fetched HTML/response:

```rust
pub struct AnalyzerPipeline {
    analyzers: Vec<Box<dyn Analyzer>>,
}

pub trait Analyzer: Send + Sync {
    fn name(&self) -> &str;
    fn analyze(&self, ctx: &AnalysisContext) -> AnalysisResult;
    fn severity(&self) -> Severity;  // Info, Warning, Error, Critical
}
```

**Structured Data Analyzer (JSON-LD)**:
- Extracts all `<script type="application/ld+json">` blocks.
- Parses and validates against schema.org vocabulary.
- Detects missing required properties for each schema type (e.g., `Article` needs `headline`, `author`, `datePublished`).
- Validates data types (e.g., `datePublished` must be ISO 8601).
- Detects multiple conflicting schemas of the same type.
- Reports on schema coverage: which rich results a page is eligible for and which are missing.

**Security Header Analyzer**:
- Checks for presence and correctness of:
  - `Content-Security-Policy` (CSP): Validates directive syntax, reports on overly permissive policies.
  - `Strict-Transport-Security` (HSTS): Validates `max-age`, `includeSubDomains`, `preload`.
  - `X-Frame-Options`: Validates value is `DENY` or `SAMEORIGIN`.
  - `X-Content-Type-Options`: Must be `nosniff`.
  - `Referrer-Policy`: Validates against recommended values.
  - `Permissions-Policy`: Checks for camera, microphone, geolocation defaults.
  - `X-XSS-Protection`: Flags use of deprecated header.
  - `Cross-Origin-Embedder-Policy`, `Cross-Origin-Opener-Policy`, `Cross-Origin-Resource-Policy`.
- Produces a security posture score (0–100) with severity-weighted findings.

**Core Web Vitals Analyzer**:
- Integrates with headless Chromium via `chromiumoxide` or similar crate.
- Measures:
  - **LCP** (Largest Contentful Paint): Time to render the largest visible element.
  - **FID/INP** (First Input Delay / Interaction to Next Paint): Responsiveness metrics.
  - **CLS** (Cumulative Layout Visual Stability): Layout shift score.
  - **TTFB** (Time to First Byte): Server response time.
  - **FCP** (First Contentful Paint): Time to first visible content.
- Uses Chromium Performance Observer APIs via CDP (Chrome DevTools Protocol).
- Supports both simulated throttling and real-device simulation modes.
- Reports percentile-based measurements across crawl sessions.

**Accessibility Analyzer (WCAG 2.1)**:
- Checks against WCAG 2.1 AA criteria:
  - Image `alt` text coverage and quality (not just present, but descriptive).
  - Heading hierarchy (no skipped levels, single `<h1>`).
  - Form label association (`<label>` elements, `aria-label`, `aria-labelledby`).
  - Color contrast ratios (via computed styles).
  - Keyboard navigation indicators (`tabindex`, focus management).
  - ARIA attribute usage and validity.
  - Landmark roles and landmark completeness.
  - Link text quality (no "click here", "read more" without context).
  - Table accessibility (headers, scope, captions).
- Uses a lightweight HTML parser (`lol_html` or `scraper`) for DOM analysis.
- Does not require a full browser rendering for most checks.

**Mobile-Friendliness Analyzer**:
- Validates `<meta name="viewport">` configuration.
- Checks for:
  - Viewport width set appropriately.
  - No `user-scalable=no` or `maximum-scale=1.0` (prevents zoom).
  - Touch target sizes (minimum 48x48 CSS pixels).
  - Font sizes (minimum 16px for body text).
  - Horizontal scrolling (content width exceeds viewport).
  - Font scaling and line-height adequacy.
- Produces a mobile-friendliness score with specific remediation guidance.

**Content Quality Analyzer**:
- **Readability**: Flesch-Kincaid Grade Level, Flesch Reading Ease, Coleman-Liau Index, Automated Readability Index.
- **Keyword Analysis**: TF-IDF scoring, keyword density (with stopword filtering), prominent keyword detection.
- **Heading Hierarchy**: H1–H6 structure analysis, missing headings, excessive heading depth.
- **Image Analysis**: Alt text coverage, image-to-text ratio, lazy loading detection, modern format usage (WebP, AVIF).
- **Content Metrics**: Word count, paragraph count, average sentence length, estimated reading time.
- **SEO Signals**: Title tag analysis (length, keyword placement), meta description (length, uniqueness), Open Graph tags, Twitter Card tags.

#### 5. Reporter Engine

Generates structured output in multiple formats:

```rust
pub trait ReportFormat: Send + Sync {
    fn write(&self, results: &CrawlResults, writer: &mut dyn Write) -> Result<()>;
    fn extension(&self) -> &str;
    fn mime_type(&self) -> &str;
}
```

**CSV Export**:
- One row per URL with all analysis results as columns.
- Configurable column selection and ordering.
- Handles nested data via JSON-encoded columns.

**JSON Export**:
- Full structured output with nested objects for redirect chains, structured data, etc.
- Streaming JSON writer for large crawls (avoids buffering entire result set in memory).
- Configurable pretty-printing.

**SQLite Export**:
- Normalized relational schema:
  - `urls` table: URL, status, content type, title, etc.
  - `redirect_chains` table: From URL, to URL, hop number, status code.
  - `issues` table: URL, analyzer, severity, message, details (JSON).
  - `structured_data` table: URL, schema type, valid, errors (JSON).
  - `headers` table: URL, header name, header value.
  - `analytics` table: URL, metric name, metric value.
- Indexes on commonly queried columns (URL, severity, analyzer name).
- Enables ad-hoc SQL queries for custom analysis.
- SQLite file is self-contained and portable.

**HTML Report**:
- Self-contained single-file HTML with embedded CSS/JS.
- Interactive dashboard with:
  - Summary statistics (total URLs, issues by severity, crawl duration).
  - URL detail view with full redirect chain visualization.
  - Issue browser with filtering by severity, analyzer, and URL pattern.
  - Chart.js visualizations for issue distribution and crawl timeline.
  - Search and sort capabilities.
- Responsive design for mobile viewing.

#### 6. Crawl Comparator

Compares two crawl snapshots to detect changes:

```rust
pub struct CrawlDiff {
    pub new_urls: Vec<Url>,
    pub removed_urls: Vec<Url>,
    pub status_changes: Vec<StatusChange>,
    pub content_changes: Vec<ContentChange>,
    pub new_issues: Vec<Issue>,
    pub resolved_issues: Vec<Issue>,
    pub redirect_changes: Vec<RedirectChange>,
    pub performance_changes: Vec<PerformanceChange>,
}
```

- Operates on SQLite exports from two different crawls.
- Detects: new pages, removed pages, status code changes, content changes (via hash comparison), new/resolved issues, redirect chain changes, and performance regressions.
- Outputs a structured diff report (JSON, HTML, or terminal-friendly).

### Technology Stack

| Component | Crate | Rationale |
|-----------|-------|-----------|
| Async Runtime | `tokio` | Industry-standard async runtime; battle-tested at scale |
| HTTP Client | `reqwest` (with `rustls-tls`) | Ergonomic, async-native, supports HTTP/2; rustls avoids OpenSSL dependency |
| HTML Parsing | `lol_html` | Streaming HTML rewriter; low memory, high throughput; used by Cloudflare |
| CSS Parsing | `cssparser` + `selectors` | Servo-derived; spec-compliant parsing and matching |
| JSON-LD Parsing | `serde_json` + `json-ld` | Schema validation against JSON-LD context |
| SQLite | `rusqlite` | Mature, well-maintained, compile-time checked queries via `sqlx` |
| CLI Framework | `clap` (derive) | Type-safe argument parsing with auto-generated docs |
| Configuration | `figment` | Multi-source config (file + env + CLI args) with merging |
| Logging | `tracing` + `tracing-subscriber` | Structured logging with context propagation |
| DNS Resolver | `hickory-resolver` | Pure-Rust DNS; avoids system resolver dependency |
| Rate Limiting | `governor` | Token-bucket rate limiter; works across async tasks |
| Chromium Integration | `chromiumoxide` | CDP protocol client for Web Vitals measurement |
| Parallel Hashing | `xxhash-rust` | Fast non-cryptographic hashing for content dedup |
| Date/Time | `chrono` | Standard date/time handling |
| Error Handling | `thiserror` + `anyhow` | Typed errors for libraries; context-rich errors for application |
| Serialization | `serde` + `serde_json` + `quick-xml` | JSON and XML parsing for sitemaps, structured data, config |
| Regex | `regex` (with `aho-corasick`) | Pattern matching for URL validation, content analysis |
| Unicode | `unicode-segmentation` | Word and grapheme boundary detection for readability analysis |

### Concurrency Model

```
┌─────────────────────────────────────────────┐
│              Tokio Runtime                  │
│                                             │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐    │
│  │ Worker 1│  │ Worker 2│  │ Worker N│    │
│  │  (fetch)│  │  (fetch)│  │  (fetch)│    │
│  └────┬────┘  └────┬────┘  └────┬────┘    │
│       │             │             │         │
│       ▼             ▼             ▼         │
│  ┌─────────────────────────────────────┐   │
│  │        Analyzer Pipeline            │   │
│  │  (sequential per-URL analysis)      │   │
│  └─────────────────┬───────────────────┘   │
│                    │                       │
│                    ▼                       │
│  ┌─────────────────────────────────────┐   │
│  │        Result Writer (batched)      │   │
│  │  (buffered SQLite inserts,          │   │
│  │   streaming JSON writes)            │   │
│  └─────────────────────────────────────┘   │
│                                             │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐    │
│  │  DNS    │  │  Robots │  │  Cache  │    │
│  │ Cache   │  │  Cache  │  │  Layer  │    │
│  └─────────┘  └─────────┘  └─────────┘    │
└─────────────────────────────────────────────┘
```

- **Worker tasks** are spawned as tokio tasks; default concurrency is 64, configurable up to 512.
- **Politeness** is enforced via per-domain semaphores (`tokio::sync::Semaphore`) with configurable permits.
- **DNS resolution** uses a shared cache with TTL-based expiry to avoid redundant lookups.
- **robots.txt** is fetched once per domain and cached with configurable TTL (default: 1 hour).
- **Result writing** is batched: SQLite inserts are accumulated in a write buffer and flushed every N rows or every M seconds (configurable). JSON output streams incrementally.
- **Memory management**: The URL frontier uses a probabilistic data structure (HyperLogLog or Bloom filter) for URL deduplication when the crawl exceeds configurable memory thresholds. This trades slight inaccuracy for bounded memory usage on very large crawls.

### Configuration Schema

```toml
# crawlkit.toml

[crawl]
seed_urls = ["https://example.com"]
max_depth = 10
max_redirect_hops = 20
max_concurrent_requests = 64
request_timeout_secs = 30
crawl_delay_default_ms = 1000
user_agent = "crawlkit/0.1.0 (+https://github.com/WyattAu/crawlkit)"
respect_robots_txt = true
follow_nofollow = false

[crawl.scope]
allowed_domains = ["example.com"]
blocked_patterns = ["/wp-admin/*", "/api/*"]
include_external_links = false

[analyzers]
enabled = [
    "structured-data",
    "security-headers",
    "accessibility",
    "mobile-friendliness",
    "content-quality",
    "canonical",
    "trailing-slash",
    "hreflang",
    "sitemap",
]

[analyzers.security-headers]
hsts_min_max_age = 31536000
require_csp = true

[analyzers.accessibility]
wcag_level = "AA"

[analyzers.content-quality]
min_flesch_reading_ease = 30.0
max_keyword_density = 0.03

[web-vitals]
enabled = true
chromium_path = "/usr/bin/chromium"
throttling = "simulated"  # "simulated" | "none"

[output]
formats = ["json", "sqlite", "html"]
output_dir = "./crawl-results"

[output.sqlite]
schema_version = 1

[output.html]
self_contained = true
include_charts = true

[comparison]
enabled = false
baseline_file = "./crawl-results/baseline.sqlite"

[logging]
level = "info"
format = "pretty"  # "pretty" | "compact" | "json"
```

### Output Schema (SQLite)

```sql
-- Core tables
CREATE TABLE crawl_sessions (
    id INTEGER PRIMARY KEY,
    started_at TEXT NOT NULL,
    completed_at TEXT,
    config_json TEXT NOT NULL,
    total_urls INTEGER DEFAULT 0,
    total_issues INTEGER DEFAULT 0
);

CREATE TABLE urls (
    id INTEGER PRIMARY KEY,
    session_id INTEGER REFERENCES crawl_sessions(id),
    url TEXT NOT NULL,
    status_code INTEGER,
    content_type TEXT,
    title TEXT,
    word_count INTEGER,
    load_time_ms INTEGER,
    depth INTEGER,
    is_indexed BOOLEAN,
    canonical_url TEXT,
    crawled_at TEXT NOT NULL
);

CREATE TABLE redirect_chains (
    id INTEGER PRIMARY KEY,
    session_id INTEGER REFERENCES crawl_sessions(id),
    original_url TEXT NOT NULL,
    final_url TEXT NOT NULL,
    hop_count INTEGER NOT NULL,
    is_loop BOOLEAN DEFAULT FALSE,
    created_at TEXT NOT NULL
);

CREATE TABLE redirect_hops (
    id INTEGER PRIMARY KEY,
    chain_id INTEGER REFERENCES redirect_chains(id),
    hop_number INTEGER NOT NULL,
    from_url TEXT NOT NULL,
    to_url TEXT NOT NULL,
    status_code INTEGER NOT NULL,
    latency_ms INTEGER
);

CREATE TABLE issues (
    id INTEGER PRIMARY KEY,
    session_id INTEGER REFERENCES crawl_sessions(id),
    url_id INTEGER REFERENCES urls(id),
    analyzer TEXT NOT NULL,
    severity TEXT NOT NULL CHECK (severity IN ('info', 'warning', 'error', 'critical')),
    code TEXT NOT NULL,
    message TEXT NOT NULL,
    details TEXT,  -- JSON
    created_at TEXT NOT NULL
);

CREATE TABLE headers (
    id INTEGER PRIMARY KEY,
    url_id INTEGER REFERENCES urls(id),
    name TEXT NOT NULL,
    value TEXT NOT NULL,
    is_security_header BOOLEAN DEFAULT FALSE
);

CREATE TABLE structured_data (
    id INTEGER PRIMARY KEY,
    url_id INTEGER REFERENCES urls(id),
    schema_type TEXT NOT NULL,
    is_valid BOOLEAN NOT NULL,
    errors TEXT,  -- JSON array
    raw_json TEXT NOT NULL
);

CREATE TABLE web_vitals (
    id INTEGER PRIMARY KEY,
    url_id INTEGER REFERENCES urls(id),
    metric_name TEXT NOT NULL,
    metric_value REAL NOT NULL,
    metric_unit TEXT NOT NULL,
    rating TEXT  -- "good", "needs-improvement", "poor"
);

CREATE TABLE analytics (
    id INTEGER PRIMARY KEY,
    url_id INTEGER REFERENCES urls(id),
    metric_name TEXT NOT NULL,
    metric_value REAL NOT NULL
);

-- Indexes
CREATE INDEX idx_urls_url ON urls(url);
CREATE INDEX idx_urls_status ON urls(status_code);
CREATE INDEX idx_issues_url ON issues(url_id);
CREATE INDEX idx_issues_severity ON issues(severity);
CREATE INDEX idx_issues_analyzer ON issues(analyzer);
CREATE INDEX idx_headers_url ON headers(url_id);
CREATE INDEX idx_structured_data_url ON structured_data(url_id);
CREATE INDEX idx_web_vitals_url ON web_vitals(url_id);
```

### CLI Interface

```
crawlkit 0.1.0
Wyatt Au <wyatt@example.com>
A comprehensive site crawler for SEO analysis

USAGE:
    crawlkit <COMMAND>

OPTIONS:
    -h, --help       Print help
    -V, --version    Print version

COMMANDS:
    crawl        Run a full crawl and analysis
    compare      Compare two crawl snapshots
    report       Generate a report from existing crawl data
    validate     Validate a crawlkit configuration file
    help         Print help

SUBCOMMAND OPTIONS:
    schedule:      Schedule recurring crawls
    export:        Export crawl data to various formats
    crawl:
        -u, --url <URL>              Seed URL (can be repeated)
        -c, --config <FILE>          Configuration file path
        -o, --output <DIR>           Output directory
        -d, --max-depth <N>          Maximum crawl depth
        -j, --concurrency <N>        Number of concurrent workers
        --format <FORMAT>            Output format (csv, json, sqlite, html)
        --no-web-vitals              Skip Core Web Vitals measurement
        --no-redirects               Don't follow redirects
        --timeout <SECONDS>          Request timeout

    compare:
        --baseline <FILE>            Baseline crawl SQLite file
        --current <FILE>             Current crawl SQLite file
        -o, --output <FILE>          Output diff file
        --format <FORMAT>            Output format (json, html)

    report:
        --input <FILE>               Crawl SQLite file
        --format <FORMAT>            Report format (html, csv, json)
        --filter <EXPRESSION>        Filter issues by expression
        --severity <LEVEL>           Minimum severity level
```

---

## Alternatives Considered

### Alternative 1: Python (Scrapy/BeautifulSoup)

**Pros**:
- Rapid prototyping; extensive ecosystem for web scraping.
- Large community; abundant libraries for HTML parsing, HTTP, etc.

**Cons**:
- **Performance**: Python is 10–100x slower than Rust for CPU-bound work. Crawling 100k+ pages would take hours instead of minutes.
- **Memory**: GIL and reference counting lead to higher memory usage per connection.
- **Distribution**: Requires Python runtime + dependencies installed on target machines. No single-binary distribution.
- **Concurrency**: asyncio is functional but less mature than tokio for high-concurrency network I/O.
- **Type Safety**: Dynamic typing increases risk of runtime errors in complex analysis pipelines.

**Verdict**: Rejected. Performance and distribution requirements cannot be met.

### Alternative 2: Go (Colly/Standard Library)

**Pros**:
- Good performance; single-binary distribution.
- Strong standard library for HTTP and HTML.
- Goroutines are lightweight and well-suited for concurrent crawling.

**Cons**:
- **Memory Model**: GC pauses can cause latency spikes during high-throughput crawling.
- **Ecosystem**: Fewer mature libraries for SEO analysis (structured data, accessibility) compared to Rust's `lol_html`, `cssparser`, etc.
- **Flexibility**: Less control over memory layout and allocation patterns than Rust.
- **Error Handling**: Go's error handling is verbose and lacks the composability of Rust's `?` operator and `anyhow`/`thiserror`.

**Verdict**: Considered viable but inferior to Rust for this use case. The memory model and ecosystem gaps tipped the decision.

### Alternative 3: Node.js (Puppeteer/Cheerio)

**Pros**:
- Excellent headless browser integration (Puppeteer).
- Rich HTML/CSS/JS ecosystem.
- Fast prototyping.

**Cons**:
- **Performance**: V8 is fast but Node.js runtime overhead and GC are significant for high-throughput crawling.
- **Memory**: Each page load in Puppeteer consumes significant memory. Crawling 100k pages would require distributed infrastructure.
- **Distribution**: Requires Node.js runtime; no single-binary distribution.
- **Concurrency**: Event loop is single-threaded; worker threads add complexity.

**Verdict**: Rejected. Performance, memory, and distribution requirements cannot be met.

### Alternative 4: Rust with Headless Browser (Playwright/Chromium)

**Pros**:
- Full browser rendering for JavaScript-heavy sites.
- Accurate Core Web Vitals measurement.
- Best compatibility with complex SPAs.

**Cons**:
- **Memory**: Chromium is memory-heavy; each tab consumes 50–200MB. Crawling thousands of pages is impractical.
- **Speed**: Browser-based crawling is 10–100x slower than HTTP-only crawling.
- **Dependency**: Requires Chromium installed; not a standalone binary.
- **Cost**: Browser-based crawling is significantly more resource-intensive.

**Verdict**: Partially adopted. Chromium integration is optional and limited to Web Vitals measurement (a few pages per crawl), not full crawl rendering. The primary crawl path uses HTTP-only fetching.

### Alternative 5: Distributed Architecture (Rust + Message Queue)

**Pros**:
- Horizontal scalability for very large sites.
- Fault tolerance via distributed workers.
- Independent scaling of crawl, analysis, and reporting.

**Cons**:
- **Complexity**: Requires message queue (Redis, Kafka), coordination service, distributed state management.
- **Deployment**: Multiple services to deploy, monitor, and maintain.
- **Overhead**: Network overhead between components; not suitable for single-site crawls.
- **Cost**: Significant infrastructure cost for what is fundamentally a single-site tool.

**Verdict**: Rejected for v1. Architecture allows future horizontal scaling if needed, but v1 targets single-machine deployment.

---

## Consequences

### Positive

1. **Performance**: Rust's zero-cost abstractions, lack of GC, and efficient memory management enable crawling 100k+ pages in minutes on commodity hardware.

2. **Single Binary Distribution**: `cargo build --release` produces a single executable with no runtime dependencies. Distribution is trivial: download and run.

3. **Comprehensive Analysis**: The modular analyzer pipeline enables deep, multi-dimensional analysis that exceeds any single existing tool.

4. **Extensibility**: New analyzers can be added by implementing the `Analyzer` trait. The pipeline composition is configuration-driven.

5. **Reproducibility**: Configuration files enable reproducible crawls. The same configuration produces comparable results across runs.

6. **Actionable Output**: Multiple export formats (especially SQLite and HTML) enable both programmatic and human consumption of results.

7. **Politeness**: Built-in robots.txt compliance, rate limiting, and crawl-delay respect ensure ethical crawling behavior.

8. **Safety**: Rust's memory safety guarantees eliminate entire classes of bugs (use-after-free, data races) that are common in concurrent C/C++ crawlers.

### Negative

1. **Development Speed**: Rust's steep learning curve and strict compiler slow initial development compared to Python or Go.

2. **Ecosystem Gaps**: Some specialized libraries (e.g., JSON-LD validation, advanced readability scoring) may require custom implementation rather than using existing crates.

3. **Compilation Time**: Full release builds may take 5–10 minutes, slowing the edit-compile-test cycle.

4. **JavaScript-Heavy Sites**: The HTTP-only crawl path cannot render JavaScript. Sites that rely on client-side rendering will have incomplete analysis. The optional Chromium integration mitigates this but adds significant overhead.

5. **Distributed Crawling**: v1 is single-machine only. Very large sites (millions of pages) may require distributed crawling, which is not yet supported.

6. **Maintenance Burden**: Keeping dependencies updated (especially Chromium via `chromiumoxide`) requires ongoing maintenance effort.

### Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Chromium API instability | Medium | High | Abstract CDP behind trait; pin Chromium version; fallback to HTTP-only mode |
| robots.txt edge cases | Medium | Low | Use mature `robotstxt` crate; comprehensive test suite |
| Memory exhaustion on large crawls | Low | High | Bloom filter fallback for URL dedup; configurable memory limits; streaming output |
| DNS resolution failures | Low | Medium | Configurable DNS resolver; fallback to system resolver |
| Rate limiting from target sites | High | Medium | Configurable delays; adaptive rate limiting based on response codes |

---

## Related

### Internal References
- None (initial ADR)

### External References
- [Google Search Central: Crawling](https://developers.google.com/search/docs/crawling-indexing/overview-google-crawlers)
- [Ahrefs Crawler Documentation](https://ahrefs.com/robot)
- [Schema.org Validation](https://validator.schema.org/)
- [WCAG 2.1 Guidelines](https://www.w3.org/TR/WCAG21/)
- [Core Web Vitals](https://web.dev/vitals/)
- [robots.txt Specification](https://www.robotstxt.org/robotstxt.html)
- [RFC 7231: HTTP/1.1 Semantics](https://tools.ietf.org/html/rfc7231) (redirect status codes)
- [BCP 47: Language Tags](https://www.rfc-editor.org/info/bcp47) (hreflang validation)

### Related Crates (Development)
- `lol_html` — Cloudflare's streaming HTML rewriter
- `chromiumoxide` — Rust CDP client for Chromium
- `reqwest` — Ergonomic HTTP client
- `tokio` — Async runtime
- `rusqlite` — SQLite bindings
- `clap` — CLI argument parsing

---

## Revision History

| Date | Author | Change |
|------|--------|--------|
| 2026-07-22 | Wyatt Au | Initial draft |

---

*This document is the source of truth for the crawlkit crawler architecture. All implementation decisions should reference this ADR.*
