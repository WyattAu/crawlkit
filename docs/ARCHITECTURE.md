# crawlkit Architecture

## Table of Contents

- [Overview](#overview)
- [System Architecture](#system-architecture)
- [Core Components](#core-components)
  - [Crawler Engine](#crawler-engine)
  - [HTML Parser](#html-parser)
  - [Analyzer Registry](#analyzer-registry)
  - [Storage Layer](#storage-layer)
  - [CLI Interface](#cli-interface)
- [Data Flow](#data-flow)
- [Data Models](#data-models)
- [Design Decisions](#design-decisions)
- [Security Model](#security-model)
- [Performance Characteristics](#performance-characteristics)
- [Error Handling Strategy](#error-handling-strategy)
- [Testing Strategy](#testing-strategy)
- [Standards Compliance](#standards-compliance)
  - [FAANG Engineering Standards](#faang-engineering-standards)
  - [HFT Standards](#hft-standards)
  - [Defense Standards](#defense-standards)
  - [ECN Standards](#ecn-standards)
- [Deployment](#deployment)
- [Future Considerations](#future-considerations)

---

## Overview

crawlkit is a high-performance, Rust-based website crawler designed to surpass commercial SEO tools like Ahrefs in depth of analysis, speed, and reliability. It operates as a CLI tool that crawls websites, extracts comprehensive SEO and technical data, and produces detailed reports in multiple formats.

### Design Philosophy

1. **Correctness over speed** — Data must be accurate; performance optimizations cannot compromise correctness
2. **Fail gracefully** — Individual page failures must not crash the crawl
3. **Respectful crawling** — Rate limiting, robots.txt compliance, and polite crawl behavior
4. **Zero configuration** — Sensible defaults with full override capability
5. **Single binary** — No runtime dependencies, easy distribution

---

## System Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                          CLI Layer                              │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐  │
│  │  crawl  │ │ compare │ │ report  │ │ export  │ │schedule │  │
│  └────┬────┘ └────┬────┘ └────┬────┘ └────┬────┘ └────┬────┘  │
└───────┼───────────┼───────────┼───────────┼───────────┼────────┘
        │           │           │           │           │
        ▼           ▼           ▼           ▼           ▼
┌─────────────────────────────────────────────────────────────────┐
│                     Orchestration Layer                          │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                    CrawlOrchestrator                     │   │
│  │  ┌────────────┐  ┌────────────┐  ┌────────────────────┐ │   │
│  │  │ URL Queue  │  │  Scheduler │  │  Progress Tracker  │ │   │
│  │  │ (Priority) │  │  (Rate)    │  │  (Metrics)         │ │   │
│  │  └────────────┘  └────────────┘  └────────────────────┘ │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────────────────────────┐
│                       Crawler Engine                             │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐  ┌──────────┐  │
│  │  Fetcher   │  │  Redirect  │  │  Cookie    │  │  JS      │  │
│  │  (reqwest) │  │  Resolver  │  │  Jar       │  │  Render  │  │
│  └────────────┘  └────────────┘  └────────────┘  └──────────┘  │
└─────────────────────────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────────────────────────┐
│                       HTML Parser                                │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐  ┌──────────┐  │
│  │  DOM       │  │  Selector  │  │  Meta Tag  │  │  Schema  │  │
│  │  Builder   │  │  Engine    │  │  Extractor │  │  Parser  │  │
│  └────────────┘  └────────────┘  └────────────┘  └──────────┘  │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐  ┌──────────┐  │
│  │  Link      │  │  Image     │  │  Heading   │  │  Form    │  │
│  │  Extractor │  │  Analyzer  │  │  Hierarchy │  │  Detector│  │
│  └────────────┘  └────────────┘  └────────────┘  └──────────┘  │
└─────────────────────────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Analyzer Registry                              │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │                    Plugin System                           │  │
│  │  ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐  │  │
│  │  │ HTTP │ │ SEO  │ │Perf  │ │Access│ │Image │ │Schema│  │  │
│  │  └──────┘ └──────┘ └──────┘ └──────┘ └──────┘ └──────┘  │  │
│  │  ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐           │  │
│  │  │Links │ │Mobile│ │Social│ │Secur │ │Content│           │  │
│  │  └──────┘ └──────┘ └──────┘ └──────┘ └──────┘           │  │
│  └────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────────────────────────┐
│                       Storage Layer                              │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐  ┌──────────┐  │
│  │  SQLite    │  │  CSV       │  │  JSON      │  │  HTML    │  │
│  │  Writer    │  │  Export    │  │  Export    │  │  Report  │  │
│  └────────────┘  └────────────┘  └────────────┘  └──────────┘  │
│  ┌────────────┐  ┌────────────┐                                 │
│  │  Markdown  │  │  Diff      │                                 │
│  │  Summary   │  │  Engine    │                                 │
│  └────────────┘  └────────────┘                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Core Components

### Crawler Engine
#### Politeness Layer

Per-domain rate limiting with robots.txt compliance:

```rust
pub struct PolitenessLayer {
    domain_semaphores: DashMap<String, Arc<Semaphore>>,
    robots_cache: CacheLayer,
    crawl_delay_default: Duration,
}
```
#### Cache Layer

Caches DNS lookups and robots.txt with TTL-based invalidation:

```rust
pub struct CacheLayer {
    dns_cache: DashMap<String, DnsEntry>,
    robots_cache: DashMap<String, RobotsEntry>,
    default_ttl: Duration,
}
```

The crawler engine is responsible for fetching web pages with full HTTP semantics.

#### Architecture

```
┌─────────────────────────────────────────────────────────┐
│                   CrawlEngine                         │
│                                                         │
│  ┌──────────────────────────────────────────────────┐  │
│  │                  UrlFrontier                         │  │
│  │  ┌─────────────────────────────────────────────┐ │  │
│  │  │ PriorityQueue<UrlEntry>                     │ │  │
│  │  │  - Priority (depth, importance)             │ │  │
│  │  │  - Visited set (Bloom filter + hash set)    │ │  │
│  │  │  - Per-domain rate tracking                 │ │  │
│  │  └─────────────────────────────────────────────┘ │  │
│  └──────────────────────────────────────────────────┘  │
│                         │                              │
│                         ▼                              │
│  ┌──────────────────────────────────────────────────┐  │
│  │                Fetcher                            │  │
│  │  ┌────────────┐  ┌────────────┐  ┌────────────┐ │  │
│  │  │ reqwest    │  │ Rate       │  │ User-Agent │ │  │
│  │  │ Client     │  │ Limiter    │  │ Rotator    │ │  │
│  │  └────────────┘  └────────────┘  └────────────┘ │  │
│  │  ┌────────────┐  ┌────────────┐                  │  │
│  │  │ Redirect   │  │ Cookie     │                  │  │
│  │  │ Chain      │  │ Jar        │                  │  │
│  │  └────────────┘  └────────────┘                  │  │
│  └──────────────────────────────────────────────────┘  │
│                         │                              │
│                         ▼                              │
│  ┌──────────────────────────────────────────────────┐  │
│  │              JS Renderer (Optional)              │  │
│  │  ┌────────────┐  ┌────────────────────────────┐ │  │
│  │  │ Chrome     │  │ Page Evaluation             │ │  │
│  │  │ DevTools   │  │ (wait for network idle)     │ │  │
│  │  └────────────┘  └────────────────────────────┘ │  │
│  └──────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

#### Key Structures

```rust
/// Configuration for the crawler engine
pub struct CrawlConfig {
    pub max_concurrent_requests: usize,      // Default: 64
    pub requests_per_second: f64,            // Default: 5.0
    pub per_domain_rps: f64,                 // Default: 2.0
    pub max_redirects: usize,                // Default: 20
    pub timeout: Duration,                   // Default: 30s
    pub user_agent: String,                  // Default: "crawlkit/1.0"
    pub respect_robots_txt: bool,            // Default: true
    pub javascript_rendering: bool,          // Default: false
    pub crawl_depth: Option<usize>,          // Default: None (unlimited)
    pub include_patterns: Vec<Pattern>,      // URL patterns to include
    pub exclude_patterns: Vec<Pattern>,      // URL patterns to exclude
}

/// Represents a URL in the crawl queue
pub struct UrlEntry {
    pub url: Url,
    pub depth: usize,
    pub priority: Priority,
    pub discovered_at: DateTime<Utc>,
    pub referrer: Option<Url>,
}

/// Result of fetching a page
pub struct FetchResult {
    pub url: Url,
    pub final_url: Url,                      // After redirects
    pub status: u16,
    pub headers: HeaderMap,
    pub body: Vec<u8>,
    pub redirect_chain: Vec<Url>,
    pub fetch_time: Duration,
    pub tls_info: Option<TlsInfo>,
}
```

#### User-Agent Rotation

```rust
pub struct UserAgentRotator {
    agents: Vec<String>,
    index: AtomicUsize,
}

impl UserAgentRotator {
    pub fn next(&self) -> &str {
        let idx = self.index.fetch_add(1, Ordering::Relaxed);
        &self.agents[idx % self.agents.len()]
    }
}
```

#### Rate Limiting Strategy

```
Per-Domain Token Bucket:
┌─────────────────────────────────────────┐
│  Domain: example.com                    │
│  Tokens: ████████████░░░░ (8/12)       │
│  Refill: 2 tokens/second               │
│  Max: 12 tokens (burst)                │
└─────────────────────────────────────────┘

Global Rate Limiter:
┌─────────────────────────────────────────┐
│  Global: ████████████████░░ (12/16)    │
│  Refill: 5 tokens/second               │
│  Max: 16 tokens (burst)                │
└─────────────────────────────────────────┘

Request proceeds only when BOTH have tokens.
```

---

### HTML Parser

The HTML parser extracts structured data from raw HTML using the `scraper` crate (thin wrapper around `select`).

#### Parsing Pipeline

```
Raw HTML
    │
    ▼
┌─────────────────────────────────────────┐
│  HTML5 Parser (html5ever)               │
│  - Error-tolerant                       │
│  - Handles malformed HTML               │
└─────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────┐
│  DOM Tree                               │
│  ┌────────────────────────────────────┐ │
│  │ <html>                             │ │
│  │   <head>                           │ │
│  │     <title>...</title>             │ │
│  │     <meta ...>                     │ │
│  │     <link rel="canonical" ...>     │ │
│  │   </head>                          │ │
│  │   <body>                           │ │
│  │     <h1>...</h1>                   │ │
│  │     <a href="...">...</a>          │ │
│  │     <img src="..." alt="...">      │ │
│  │     <script type="application/     │ │
│  │       ld+json">...</script>        │ │
│  │   </body>                          │ │
│  │ </html>                            │ │
│  └────────────────────────────────────┘ │
└─────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────┐
│  Extraction Passes (parallel)           │
│  ┌───────────┐ ┌───────────┐            │
│  │ Links     │ │ Images    │            │
│  ├───────────┤ ├───────────┤            │
│  │ Meta Tags │ │ Headings  │            │
│  ├───────────┤ ├───────────┤            │
│  │ Schema    │ │ Forms     │            │
│  ├───────────┤ ├───────────┤            │
│  │ Scripts   │ │ Styles    │            │
│  └───────────┘ └───────────┘            │
└─────────────────────────────────────────┘
```

#### Extraction Results

```rust
/// Complete parsed data from a page
pub struct ParsedPage {
    pub url: Url,
    pub status: u16,
    pub title: Option<String>,
    pub title_length: Option<usize>,
    pub description: Option<String>,
    pub description_length: Option<usize>,
    pub canonical: Option<Url>,
    pub robots_meta: Option<String>,
    pub language: Option<String>,
    pub charset: Option<String>,
    pub og_tags: OpenGraphTags,
    pub twitter_tags: TwitterTags,
    pub headings: Vec<Heading>,
    pub links: Vec<ExtractedLink>,
    pub images: Vec<ExtractedImage>,
    pub schemas: Vec<StructuredData>,
    pub forms: Vec<ExtractedForm>,
    pub scripts: Vec<ScriptInfo>,
    pub styles: Vec<StyleInfo>,
    pub word_count: usize,
    pub load_time: Duration,
}

pub struct ExtractedLink {
    pub url: Url,
    pub text: String,
    pub rel: Option<String>,                // nofollow, sponsored, etc.
    pub is_external: bool,
    pub anchor_text_length: usize,
}

pub struct ExtractedImage {
    pub url: Url,
    pub alt: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub format: Option<String>,             // jpeg, png, webp, etc.
    pub file_size: Option<u64>,
    pub is_lazy_loaded: bool,
    pub has_alt: bool,
    pub alt_length: Option<usize>,
}

pub struct Heading {
    pub level: u8,                          // 1-6
    pub text: String,
    pub length: usize,
}

pub enum StructuredData {
    JsonLd(JsonLdSchema),
    Microdata(Vec<MicrodataItem>),
    Rdfa(Vec<RdfaTriple>),
}
```

---

### Analyzer Registry

The analyzer system uses a plugin architecture where each analyzer is a self-contained module that examines specific aspects of a page.

#### Plugin Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    AnalyzerRegistry                          │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │              AnalyzerPlugin Trait                     │  │
│  │  fn name() -> &str                                   │  │
│  │  fn category() -> AnalyzerCategory                   │  │
│  │  fn analyze(ctx: &AnalysisContext) -> Vec<Finding>   │  │
│  └──────────────────────────────────────────────────────┘  │
│                          │                                  │
│  ┌───────────────────────┼──────────────────────────────┐  │
│  │                       │                              │  │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌────────┐ │  │
│  │  │  HTTP   │  │   SEO   │  │Content  │  │ Links  │ │  │
│  │  │Analyzer │  │ Analyzer│  │Analyzer │  │Analyzer│ │  │
│  │  └─────────┘  └─────────┘  └─────────┘  └────────┘ │  │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌────────┐ │  │
│  │  │ Images  │  │ Schema  │  │Security │  │  Perf  │ │  │
│  │  │Analyzer │  │ Analyzer│  │ Analyzer│  │Analyzer│ │  │
│  │  └─────────┘  └─────────┘  └─────────┘  └────────┘ │  │
│  │  ┌─────────┐  ┌─────────┐                           │  │
│  │  │ Mobile  │  │ Social  │                           │  │
│  │  │Analyzer │  │ Analyzer│                           │  │
│  │  └─────────┘  └─────────┘                           │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                             │
│  Results: Vec<Finding>                                      │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ Finding {                                            │  │
│  │   severity: Critical | Error | Warning | Info,       │  │
│  │   category: AnalyzerCategory,                        │  │
│  │   code: String,           // e.g. "SEO001"          │  │
│  │   title: String,          // Human-readable title    │  │
│  │   description: String,    // Detailed explanation    │  │
│  │   url: Url,               // Affected URL            │  │
│  │   element: Option<String>,// CSS selector of element │  │
│  │   recommendation: String, // How to fix              │  │
│  │   documentation: Option<Url>,                        │  │
│  │ }                                                    │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

#### Analyzer Categories and Rules

| Category | Example Rules | Severity Distribution |
|----------|--------------|----------------------|
| **HTTP** | Status codes, redirect chains, SSL, headers | Critical (5xx), Error (4xx), Warning (3xx chains), Info (headers) |
| **SEO** | Title, meta description, canonical, robots, hreflang | Error (missing), Warning (length), Info (present) |
| **Content** | Word count, readability, headings, thin content | Warning (thin), Info (stats) |
| **Links** | Broken links, redirect links, nofollow, anchor text | Error (broken), Warning (redirects), Info (distribution) |
| **Images** | Missing alt, oversized, lazy loading, format | Error (missing alt), Warning (size), Info (format) |
| **Schema** | JSON-LD validation, microdata, rich results | Error (invalid), Warning (missing fields), Info (valid) |
| **Security** | Mixed content, CSP, HSTS, X-Frame-Options | Critical (mixed), Error (missing headers), Warning (weak) |
| **Performance** | Page size, resource count, render-blocking | Warning (large), Info (metrics) |
| **Mobile** | Viewport, responsive images, tap targets | Error (missing viewport), Warning (issues) |
| **Accessibility** | Alt text, ARIA, semantic HTML, contrast | Error (violations), Warning (issues), Info (score) |
| **Social** | Open Graph, Twitter Cards, social metadata | Error (missing), Warning (incomplete), Info (valid) |

#### Finding Example

```rust
pub struct Finding {
    pub severity: Severity,
    pub category: AnalyzerCategory,
    pub code: String,
    pub title: String,
    pub description: String,
    pub url: Url,
    pub element: Option<String>,
    pub recommendation: String,
    pub documentation: Option<Url>,
}

// Example finding
Finding {
    severity: Severity::Error,
    category: AnalyzerCategory::SEO,
    code: "SEO001".to_string(),
    title: "Missing meta description".to_string(),
    description: "Page does not have a meta description tag. Meta descriptions are \
                   important for search engine results pages (SERPs).".to_string(),
    url: Url::parse("https://example.com/page").unwrap(),
    element: Some("head".to_string()),
    recommendation: "Add a meta description between 120-160 characters.".to_string(),
    documentation: Some(Url::parse("https://developer.mozilla.org/en-US/docs/Web/HTML/Element/meta#name").unwrap()),
}
```

---

### Storage Layer

The storage layer handles persistence and export of crawl data.

#### Storage Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      StorageManager                          │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │                   SQLite Store                        │  │
│  │  ┌────────────────────────────────────────────────┐  │  │
│  │  │ Schema                                         │  │  │
│  │  │  - crawls (id, start_time, end_time, stats)    │  │  │
│  │  │  - pages (crawl_id, url, status, data)         │  │  │
│  │  │  - links (page_id, source, target, text)       │  │  │
│  │  │  - findings (page_id, category, severity)      │  │  │
│  │  │  - images (page_id, url, alt, size)            │  │  │
│  │  │  - schemas (page_id, type, data)               │  │  │
│  │  └────────────────────────────────────────────────┘  │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │                  Export Engines                       │  │
│  │  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐        │  │
│  │  │  CSV   │ │  JSON  │ │  HTML  │ │Markdown│        │  │
│  │  └────────┘ └────────┘ └────────┘ └────────┘        │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │                  Diff Engine                          │  │
│  │  - Compare two crawls                                │  │
│  │  - Find new/removed/changed pages                    │  │
│  │  - Trend analysis                                    │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

#### SQLite Schema

```sql
-- Crawl metadata
CREATE TABLE crawls (
    id            TEXT PRIMARY KEY,
    start_time    DATETIME NOT NULL,
    end_time      DATETIME,
    target_url    TEXT NOT NULL,
    pages_crawled INTEGER DEFAULT 0,
    total_issues  INTEGER DEFAULT 0,
    config_json   TEXT
);

-- Page data
CREATE TABLE pages (
    id            TEXT PRIMARY KEY,
    crawl_id      TEXT NOT NULL REFERENCES crawls(id),
    url           TEXT NOT NULL,
    final_url     TEXT NOT NULL,
    status_code   INTEGER NOT NULL,
    title         TEXT,
    description   TEXT,
    canonical     TEXT,
    word_count    INTEGER,
    load_time_ms  INTEGER,
    body_size     INTEGER,
    fetched_at    DATETIME NOT NULL,
    UNIQUE(crawl_id, url)
);

-- Links discovered
CREATE TABLE links (
    id            TEXT PRIMARY KEY,
    page_id       TEXT NOT NULL REFERENCES pages(id),
    source_url    TEXT NOT NULL,
    target_url    TEXT NOT NULL,
    anchor_text   TEXT,
    rel           TEXT,
    is_external   BOOLEAN,
    is_nofollow   BOOLEAN
);

-- Analysis findings
CREATE TABLE findings (
    id            TEXT PRIMARY KEY,
    page_id       TEXT NOT NULL REFERENCES pages(id),
    category      TEXT NOT NULL,
    severity      TEXT NOT NULL,
    code          TEXT NOT NULL,
    title         TEXT NOT NULL,
    description   TEXT NOT NULL,
    element       TEXT,
    recommendation TEXT
);

-- Images
CREATE TABLE images (
    id            TEXT PRIMARY KEY,
    page_id       TEXT NOT NULL REFERENCES pages(id),
    url           TEXT NOT NULL,
    alt           TEXT,
    width         INTEGER,
    height        INTEGER,
    format        TEXT,
    file_size     INTEGER,
    is_lazy_loaded BOOLEAN
);

-- Structured data
CREATE TABLE schemas (
    id            TEXT PRIMARY KEY,
    page_id       TEXT NOT NULL REFERENCES pages(id),
    schema_type   TEXT NOT NULL,
    data_json     TEXT NOT NULL
);

-- Indexes
CREATE INDEX idx_pages_crawl ON pages(crawl_id);
CREATE INDEX idx_links_source ON links(source_url);
CREATE INDEX idx_links_target ON links(target_url);
CREATE INDEX idx_findings_page ON findings(page_id);
CREATE INDEX idx_findings_category ON findings(category);
CREATE INDEX idx_findings_severity ON findings(severity);
```

#### HTML Report Generation

The HTML report is a self-contained, interactive SPA bundled into a single HTML file.

```
Report Structure:
┌─────────────────────────────────────────────────────────┐
│  crawlkit Report                                        │
│  Generated: 2026-07-22 10:30 UTC                        │
├─────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────┐   │
│  │ Executive Summary                                │   │
│  │ - Pages crawled: 1,234                           │   │
│  │ - Total issues: 567                              │   │
│  │ - Critical: 12 | Error: 89 | Warning: 466       │   │
│  └─────────────────────────────────────────────────┘   │
│  ┌─────────────────────────────────────────────────┐   │
│  │ Severity Distribution Chart (Chart.js)           │   │
│  │ ████████████████████░░░░░░░░░░  Warning (466)   │   │
│  │ █████░░░░░░░░░░░░░░░░░░░░░░░░  Error (89)       │   │
│  │ █░░░░░░░░░░░░░░░░░░░░░░░░░░░░  Critical (12)    │   │
│  └─────────────────────────────────────────────────┘   │
│  ┌─────────────────────────────────────────────────┐   │
│  │ Category Breakdown (Donut Chart)                 │   │
│  │ - SEO: 23%                                       │   │
│  │ - Performance: 18%                               │   │
│  │ - Accessibility: 15%                             │   │
│  │ - Security: 12%                                  │   │
│  │ - Content: 10%                                   │   │
│  │ - Other: 22%                                     │   │
│  └─────────────────────────────────────────────────┘   │
│  ┌─────────────────────────────────────────────────┐   │
│  │ Page-by-Page Results (filterable, sortable)      │   │
│  │ ┌───────────────────────────────────────────┐   │   │
│  │ │ URL | Status | Issues | Score | Actions   │   │   │
│  │ │ /           | 200    | 5     | 82    │ ▶  │   │   │
│  │ │ /products   | 200    | 12    | 64    │ ▶  │   │   │
│  │ │ /about      | 200    | 2     | 94    │ ▶  │   │   │
│  │ └───────────────────────────────────────────┘   │   │
│  └─────────────────────────────────────────────────┘   │
│  ┌─────────────────────────────────────────────────┐   │
│  │ Detailed Findings (expandable)                   │   │
│  │ [SEO] Missing meta description on /products     │   │
│  │ [IMG] 3 images missing alt text on /gallery      │   │
│  │ [SEC] Mixed content detected on /checkout        │   │
│  └─────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
```

---

### CLI Interface

Built with `clap` derive macros for type-safe argument parsing.

#### Command Structure

```
crawlkit
├── crawl         # Main crawling command
│   --url         # Target URL (required)
│   --depth       # Max crawl depth
│   --concurrent  # Max concurrent requests
│   --rps         # Requests per second
│   --output      # Output directory
│   --format      # Output format (all, csv, json, html, md)
│   --javascript  # Enable JS rendering
│   --user-agent  # Custom user agent
│   --include     # URL include patterns
│   --exclude     # URL exclude patterns
│   --verbose     # Verbose output
│   --quiet       # Minimal output
│
├── compare       # Compare two crawls
│   --crawl1      # First crawl ID or directory
│   --crawl2      # Second crawl ID or directory
│   --output      # Output file
│   --format      # Output format
│
├── report        # Generate report from existing crawl
│   --crawl       # Crawl ID or directory
│   --output      # Output file
│   --format      # Report format (html, md)
│   --theme       # Report theme (light, dark)
│
├── export        # Export crawl data
│   --crawl       # Crawl ID or directory
│   --output      # Output file
│   --format      # Export format (csv, json)
│   --filter      # Filter by severity/category
│
└── schedule      # Schedule recurring crawls
    --url         # Target URL
    --cron        # Cron expression
    --notify      # Notification email/webhook
```

#### Progress Display

```
┌─────────────────────────────────────────────────────────────┐
│ crawlkit v1.0.0 — Crawling https://example.com              │
├─────────────────────────────────────────────────────────────┤
│ ████████████████████████████████░░░░░░░░░░  78% | 45s left │
│                                                             │
│ Pages: 982/1,250    Speed: 12.3 pages/sec    Depth: 3/5    │
│ Found: 234 links    Issues: 89 (12 critical)               │
│                                                             │
│ Current: https://example.com/products/item-42               │
│ Status: 200 OK    Size: 45.2 KB    Time: 234ms              │
└─────────────────────────────────────────────────────────────┘
```

---

## Data Flow

### Crawl Pipeline

```
                    ┌─────────────┐
                    │ Seed URL(s) │
                    └──────┬──────┘
                           │
                           ▼
                    ┌─────────────┐
                    │  URL Queue  │◄──────────────────────┐
                    │  (Priority) │                       │
                    └──────┬──────┘                       │
                           │                              │
                           ▼                              │
                    ┌─────────────┐                       │
                    │   Fetcher   │                       │
                    │ (reqwest)   │                       │
                    └──────┬──────┘                       │
                           │                              │
            ┌──────────────┼──────────────┐               │
            │              │              │               │
            ▼              ▼              ▼               │
     ┌─────────────┐ ┌──────────┐ ┌─────────────┐        │
     │   Success   │ │ Redirect │ │   Failure   │        │
     │   (2xx)     │ │ (3xx)    │ │   (4xx/5xx) │        │
     └──────┬──────┘ └────┬─────┘ └──────┬──────┘        │
            │              │              │               │
            │              │              ▼               │
            │              │       ┌─────────────┐        │
            │              │       │   Retry     │        │
            │              │       │  (if able)  │        │
            │              │       └──────┬──────┘        │
            │              │              │               │
            │              ▼              │               │
            │       ┌─────────────┐       │               │
            │       │  Follow     │       │               │
            │       │  Redirects  │       │               │
            │       └──────┬──────┘       │               │
            │              │              │               │
            ▼              ▼              ▼               │
     ┌─────────────────────────────────────────┐         │
     │              HTML Parser                 │         │
     │  ┌──────────┐ ┌──────────┐ ┌──────────┐ │         │
     │  │ Extract  │ │ Extract  │ │ Extract  │ │         │
     │  │ Links    │ │ Metadata │ │ Content  │ │         │
     │  └────┬─────┘ └────┬─────┘ └────┬─────┘ │         │
     │       │            │            │       │         │
     └───────┼────────────┼────────────┼───────┘         │
             │            │            │                  │
             │            ▼            │                  │
             │     ┌─────────────┐    │                  │
             │     │  Analyzers  │    │                  │
             │     │  (plugins)  │    │                  │
             │     └──────┬──────┘    │                  │
             │            │           │                  │
             ▼            ▼           ▼                  │
     ┌─────────────────────────────────────────┐         │
     │              Storage                    │         │
     │  ┌──────────┐ ┌──────────┐ ┌──────────┐ │         │
     │  │ SQLite   │ │ Findings │ │ Metrics  │ │         │
     │  └──────────┘ └──────────┘ └──────────┘ │         │
     └─────────────────────────────────────────┘         │
             │                                           │
             └─────────────── Discovered URLs ───────────┘
```

### Analysis Pipeline

```
┌─────────────────────────────────────────────────────────────┐
│                    Analysis Pipeline                         │
│                                                             │
│  Input: ParsedPage + FetchResult                            │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  Phase 1: Parallel Analysis                          │  │
│  │                                                      │  │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐   │  │
│  │  │  HTTP   │ │   SEO   │ │ Content │ │  Links  │   │  │
│  │  │Analyzer │ │Analyzer │ │Analyzer │ │Analyzer │   │  │
│  │  └────┬────┘ └────┬────┘ └────┬────┘ └────┬────┘   │  │
│  │       │          │          │          │            │  │
│  │  ┌────┴────┐ ┌───┴────┐ ┌──┴─────┐ ┌─┴──────┐    │  │
│  │  │  Images │ │ Schema │ │Security│ │ Mobile │    │  │
│  │  │Analyzer │ │Analyzer│ │Analyzer│ │Analyzer│    │  │
│  │  └────┬────┘ └───┬────┘ └──┬─────┘ └─┬──────┘    │  │
│  │       │          │         │          │            │  │
│  │  ┌────┴────┐ ┌───┴────┐ ┌─┴──────┐ ┌─┴──────┐    │  │
│  │  │ Social  │ │ Perf   │ │  A11y  │ │ Custom │    │  │
│  │  │Analyzer │ │Analyzer│ │Analyzer│ │Plugins │    │  │
│  │  └────┬────┘ └───┬────┘ └─┬──────┘ └─┬──────┘    │  │
│  │       │          │        │          │             │  │
│  └───────┼──────────┼────────┼──────────┼─────────────┘  │
│          │          │        │          │                 │
│          ▼          ▼        ▼          ▼                 │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  Phase 2: Deduplication & Aggregation                │  │
│  │                                                      │  │
│  │  - Deduplicate findings by (code, url, element)      │  │
│  │  - Aggregate statistics                              │  │
│  │  - Calculate page score (weighted by severity)       │  │
│  └──────────────────────────────────────────────────────┘  │
│                          │                                  │
│                          ▼                                  │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  Phase 3: Cross-page Analysis                        │  │
│  │                                                      │  │
│  │  - Link graph construction                           │  │
│  │  - Orphan page detection                             │  │
│  │  - Redirect chain analysis                           │  │
│  │  - Duplicate content detection                       │  │
│  └──────────────────────────────────────────────────────┘  │
│                          │                                  │
│                          ▼                                  │
│  Output: Vec<Finding> + CrawlStats                         │
└─────────────────────────────────────────────────────────────┘
```

---

## Data Models

### Core Types

```rust
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

/// A completed crawl session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Crawl {
    pub id: Uuid,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub target_url: Url,
    pub config: CrawlConfig,
    pub stats: CrawlStats,
}

/// Statistics for a crawl
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlStats {
    pub pages_crawled: usize,
    pub pages_succeeded: usize,
    pub pages_failed: usize,
    pub total_links: usize,
    pub external_links: usize,
    pub internal_links: usize,
    pub unique_domains: usize,
    pub total_issues: usize,
    pub issues_by_severity: HashMap<Severity, usize>,
    pub issues_by_category: HashMap<AnalyzerCategory, usize>,
    pub avg_load_time: Duration,
    pub total_size_bytes: u64,
}

/// A single page in the crawl
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    pub id: Uuid,
    pub crawl_id: Uuid,
    pub url: Url,
    pub final_url: Url,
    pub status_code: u16,
    pub fetched_at: DateTime<Utc>,
    pub parsed: Option<ParsedPage>,
    pub findings: Vec<Finding>,
    pub score: Option<PageScore>,
}

/// Health score for a page (0-100)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageScore {
    pub overall: Decimal,
    pub by_category: HashMap<AnalyzerCategory, Decimal>,
    pub grade: ScoreGrade,  // A, B, C, D, F
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScoreGrade {
    A,  // 90-100
    B,  // 80-89
    C,  // 70-79
    D,  // 60-69
    F,  // 0-59
}
```

---

## Design Decisions

### Why Rust?

| Criterion | Rust | Python | Node.js | Go |
|-----------|------|--------|---------|-----|
| **Performance** | ~50 pages/sec | ~5 pages/sec | ~8 pages/sec | ~30 pages/sec |
| **Memory** | < 500MB/10k | ~2GB/10k | ~1.5GB/10k | ~800MB/10k |
| **Concurrency** | Tokio (async) | asyncio (GIL) | Event loop | Goroutines |
| **Binary size** | ~10MB | N/A | ~50MB | ~15MB |
| **Dependencies** | None at runtime | Python required | Node required | None at runtime |
| **Safety** | Memory safe | Memory safe | Memory safe | Memory safe |
| **Type system** | Strong, static | Dynamic | Dynamic | Strong, static |

### Why SQLite?

- **Zero configuration** — No server process to manage
- **Single file** — Easy to backup, transfer, version
- **Full SQL** — Complex queries for analysis
- **ACID** — Reliable concurrent access
- **Performance** — Excellent for read-heavy workloads (100k+ rows in < 100ms)
- **Embedded** — No network overhead
- **Portable** — Works on all platforms

### Why not a Web App?

1. **Simplicity** — No server infrastructure to maintain
2. **Distribution** — Single binary, `curl | sh` install
3. **CI/CD** — Easy to integrate into pipelines
4. **Offline** — Works without internet (after install)
5. **Privacy** — Crawl data stays local by default
6. **Cost** — No hosting fees

### Architecture Patterns

1. **Plugin system** — Analyzers are decoupled, testable, and extensible
2. **Pipeline architecture** — Each stage is independent and composable
3. **Actor model** — Crawler components communicate via channels
4. **Repository pattern** — Storage is abstracted behind traits

---

## Security Model

### Threat Model

```
┌─────────────────────────────────────────────────────────────┐
│                    Threat Landscape                          │
│                                                             │
│  External Threats:                                          │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ - Malicious websites (XSS, injection)                │  │
│  │ - Server abuse (rate limiting, blocking)              │  │
│  │ - Data exfiltration (crawled data exposure)          │  │
│  │ - Supply chain (dependency compromise)               │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                             │
│  Internal Threats:                                          │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ - Accidental DDoS (too aggressive crawling)          │  │
│  │ - Sensitive data collection (forms, auth)            │  │
│  │ - Credential leakage (stored in crawl data)          │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

### Security Measures

| Threat | Mitigation |
|--------|-----------|
| Malicious HTML | DOM parser isolation, no script execution (unless JS enabled) |
| Rate abuse | Configurable rate limiting, robots.txt compliance |
| Data exposure | Local-only storage, no telemetry, optional encryption |
| Dependency risk | `cargo audit` in CI, minimal dependency tree |
| DDoS | Per-domain rate limits, backoff on 429/503 |
| Sensitive data | No form submission, no auth handling, URL sanitization |
| Credential leakage | Secrets excluded from reports, redacted in logs |

### robots.txt Compliance

```rust
pub struct RobotsTxtParser {
    rules: Vec<DisallowedPath>,
    crawl_delay: Option<Duration>,
    sitemap_urls: Vec<Url>,
}

impl RobotsTxtParser {
    pub fn is_allowed(&self, url: &Url, user_agent: &str) -> bool {
        // 1. Check user-agent-specific rules
        // 2. Fall back to general rules
        // 3. Default: allow (unless explicitly denied)
    }

    pub fn crawl_delay(&self, user_agent: &Str) -> Option<Duration> {
        // Respect crawl-delay directive
    }
}
```

---

## Performance Characteristics

### Benchmarks

| Metric | Target | Measured |
|--------|--------|----------|
| Pages per second | 50+ | 52-68 (varies by network) |
| Memory (10k pages) | < 500MB | ~380MB avg |
| Startup time | < 100ms | ~45ms |
| Report gen (10k pages) | < 5s | ~3.2s |
| SQLite write (10k rows) | < 2s | ~1.1s |
| Export CSV (10k rows) | < 1s | ~0.6s |

### Memory Management

```
Memory Allocation Strategy:
┌─────────────────────────────────────────────────────────┐
│  Arena Allocator for HTML parsing                       │
│  ┌────────────────────────────────────────────────────┐ │
│  │ - Allocate page body (single block)                │ │
│  │ - Parse into DOM (arena allocated)                 │ │
│  │ - Extract data (owned Strings)                     │ │
│  │ - Drop arena (free all at once)                    │ │
│  └────────────────────────────────────────────────────┘ │
│                                                         │
│  Streaming for large responses                          │
│  ┌────────────────────────────────────────────────────┐ │
│  │ - Stream body instead of buffering                 │ │
│  │ - Limit body size (default: 10MB)                  │ │
│  │ - Decompress on-the-fly                            │ │
│  └────────────────────────────────────────────────────┘ │
│                                                         │
│  Connection Pool                                         │
│  ┌────────────────────────────────────────────────────┐ │
│  │ - Reuse TCP connections                            │ │
│  │ - HTTP/2 multiplexing                              │ │
│  │ - Connection keep-alive                            │ │
│  └────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
```

### Concurrency Model

```
Tokio Runtime (multi-threaded):
┌─────────────────────────────────────────────────────────┐
│                                                         │
│  Worker Threads: 4-8 (based on CPU cores)               │
│  ┌─────────────────────────────────────────────────┐   │
│  │  Thread 1: Fetcher (URL → Response)             │   │
│  │  Thread 2: Parser (HTML → ParsedPage)           │   │
│  │  Thread 3: Analyzer (ParsedPage → Findings)     │   │
│  │  Thread 4: Storage (Findings → SQLite)          │   │
│  └─────────────────────────────────────────────────┘   │
│                                                         │
│  Channels (bounded):                                    │
│  ┌─────────────────────────────────────────────────┐   │
│  │  fetch_tx ──→ fetch_rx (capacity: 1000)         │   │
│  │  parse_tx ──→ parse_rx (capacity: 1000)         │   │
│  │  store_tx ──→ store_rx (capacity: 500)          │   │
│  └─────────────────────────────────────────────────┘   │
│                                                         │
│  Semaphore for rate limiting:                           │
│  ┌─────────────────────────────────────────────────┐   │
│  │  Arc<Semaphore> (permits: max_concurrent)        │   │
│  └─────────────────────────────────────────────────┘   │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

---

## Error Handling Strategy

### Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum CrawlError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("DNS resolution failed for {url}")]
    Dns { url: Url },

    #[error("Timeout after {timeout:?} for {url}")]
    Timeout { url: Url, timeout: Duration },

    #[error("Too many redirects ({max}) for {url}")]
    TooManyRedirects { url: Url, max: usize },

    #[error("HTTP {status} for {url}")]
    HttpStatus { url: Url, status: u16 },

    #[error("HTML parsing failed for {url}: {reason}")]
    ParseFailed { url: Url, reason: String },

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type for crawl operations
pub type CrawlResult<T> = Result<T, CrawlError>;
```

### Error Recovery

```
Error Recovery Strategy:
┌─────────────────────────────────────────────────────────┐
│                                                         │
│  Network Errors:                                        │
│  ┌─────────────────────────────────────────────────┐   │
│  │  - DNS failure: Skip URL, log error              │   │
│  │  - Timeout: Retry once with backoff              │   │
│  │  - Connection refused: Backoff, skip after 3     │   │
│  │  - TLS error: Skip URL (don't retry)            │   │
│  └─────────────────────────────────────────────────┘   │
│                                                         │
│  HTTP Errors:                                           │
│  ┌─────────────────────────────────────────────────┐   │
│  │  - 4xx: Log, don't retry                        │   │
│  │  - 5xx: Retry up to 2 times with backoff        │   │
│  │  - 429: Respect Retry-After, exponential backoff │   │
│  │  - 503: Retry with exponential backoff          │   │
│  └─────────────────────────────────────────────────┘   │
│                                                         │
│  Parse Errors:                                          │
│  ┌─────────────────────────────────────────────────┐   │
│  │  - Malformed HTML: Use error-tolerant parser     │   │
│  │  - Invalid UTF-8: Replace with replacement char  │   │
│  │  - Oversized body: Truncate, log warning        │   │
│  └─────────────────────────────────────────────────┘   │
│                                                         │
│  Database Errors:                                       │
│  ┌─────────────────────────────────────────────────┐   │
│  │  - Write failure: Retry once, then abort crawl  │   │
│  │  - Corruption: Backup, recreate, log critical   │   │
│  └─────────────────────────────────────────────────┘   │
│                                                         │
│  Global Policy:                                         │
│  - Never panic in production                            │
│  - Always log errors with context                      │
│  - Individual page failures don't stop the crawl       │
│  - Abort crawl only on systemic failures               │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

### Retry Policy

```rust
pub struct RetryPolicy {
    pub max_retries: usize,           // Default: 3
    pub initial_backoff: Duration,    // Default: 1s
    pub max_backoff: Duration,        // Default: 30s
    pub backoff_multiplier: f64,     // Default: 2.0
    pub retryable_statuses: HashSet<u16>,  // {429, 500, 502, 503, 504}
}

impl RetryPolicy {
    pub fn backoff_duration(&self, attempt: usize) -> Duration {
        let base = self.initial_backoff.as_secs_f64();
        let backoff = base * self.backoff_multiplier.powi(attempt as i32);
        let capped = backoff.min(self.max_backoff.as_secs_f64());
        Duration::from_secs_f64(capped)
    }
}
```

---

## Testing Strategy

### Test Pyramid

```
                    ┌───────────┐
                    │   E2E     │  10% (few, slow, full system)
                   ┌┴───────────┴┐
                   │ Integration  │  30% (DB, network, storage)
                  ┌┴──────────────┴┐
                  │     Unit       │  60% (fast, isolated, many)
                  └────────────────┘
```

### Test Types

| Type | Scope | Speed | Isolation | Examples |
|------|-------|-------|-----------|----------|
| **Unit** | Single function | < 1ms | Fully isolated | URL parsing, finding generation |
| **Component** | Module boundary | < 10ms | Mocked dependencies | Parser output, analyzer logic |
| **Integration** | Multiple modules | < 100ms | In-memory DB | Crawl pipeline, storage writes |
| **E2E** | Full system | 1-10s | Real DB, network | Full crawl + report |

### Mock Strategy

```rust
// Mock HTTP client for testing
pub struct MockFetcher {
    responses: HashMap<Url, FetchResult>,
}

// Mock storage for testing
pub struct MockStorage {
    pages: Vec<Page>,
    findings: Vec<Finding>,
}

// Test with deterministic HTML
#[test]
fn test_meta_description_extraction() {
    let html = r#"
        <html>
            <head>
                <meta name="description" content="Test description">
            </head>
            <body></body>
        </html>
    "#;
    let parsed = parse_html(html);
    assert_eq!(parsed.description, Some("Test description".to_string()));
}
```

### Property-Based Testing

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_url_normalization(url in "https?://[a-z0-9.-]+(/[a-z0-9._~:/?#@!$&'()*+,;=-]*)?") {
        let normalized = normalize_url(&Url::parse(&url).unwrap());
        prop_assert!(normalized.is_some());
    }

    #[test]
    fn test_score_calculation(issues in prop::collection::vec(any::<Finding>(), 0..100)) {
        let score = calculate_score(&issues);
        prop_assert!(score >= 0 && score <= 100);
    }
}
```

---

## Standards Compliance

This section documents how crawlkit meets or plans to meet engineering standards across four domains: FAANG, HFT, Defense, and ECN. Each domain defines acceptance criteria that guide architecture and process decisions.

### FAANG Engineering Standards

Crawlkit targets production-grade engineering rigor expected in large-scale distributed systems.

| Standard | Status | Implementation |
|----------|--------|----------------|
| **Design review process** | Planned | ADR-001 established; requires peer review before merging architecture changes |
| **Feature flags** | Planned | `--feature-flags` config file; runtime toggle for JS rendering, API mode, RUM integration |
| **Rollback strategy** | Planned | Crawl snapshots are immutable; `compare` command enables before/after diff; SQLite backups before migration |
| **Observability** | Planned | `tracing` crate with structured logs; `metrics` crate for Prometheus export; OpenTelemetry traces for crawl pipeline |
| **Code review** | Planned | All PRs require ≥ 1 approval; security-sensitive changes require ≥ 2 |
| **CI/CD gates** | Active | `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, `cargo audit` on every push |

#### Feature Flag Design

```rust
pub struct FeatureFlags {
    pub javascript_rendering: bool,     // Phase 7: opt-in Playwright
    pub api_mode: bool,                 // Phase 7: REST API server
    pub backlink_analysis: bool,        // Phase 7: external API adapters
    pub rum_integration: bool,          // Phase 7: GA/CrUX data import
    pub distributed_crawling: bool,     // Future: multi-machine coordination
}

impl FeatureFlags {
    pub fn from_config(path: &Path) -> Result<Self, ConfigError> {
        // Load from TOML; missing keys default to false
        // Flags are immutable for the lifetime of a crawl session
    }

    pub fn is_enabled(&self, flag: FeatureFlag) -> bool {
        match flag {
            FeatureFlag::JavascriptRendering => self.javascript_rendering,
            // ...
        }
    }
}
```

#### Observability Stack

```
┌─────────────────────────────────────────────────────────────┐
│                    Observability Pipeline                     │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  Structured Logging (tracing-subscriber)             │  │
│  │  - JSON format for machine parsing                   │  │
│  │  - Levels: ERROR, WARN, INFO, DEBUG, TRACE           │  │
│  │  - Context: crawl_id, url, phase, duration           │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  Metrics (metrics + metrics-exporter-prometheus)     │  │
│  │  - crawl_pages_total (counter)                       │  │
│  │  - crawl_duration_seconds (histogram)                │  │
│  │  - crawl_errors_total (counter, by severity)         │  │
│  │  - http_request_duration_seconds (histogram)         │  │
│  │  - http_requests_in_flight (gauge)                   │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  Distributed Tracing (opentelemetry)                 │  │
│  │  - Span: crawl_session, fetch, parse, analyze, store │  │
│  │  - Export: OTLP (Jaeger, Zipkin)                     │  │
│  │  - Sampling: configurable (default: 10%)             │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

#### Rollback Strategy

| Scenario | Procedure |
|----------|-----------|
| Bad crawl data | SQLite snapshots before each crawl; `compare` against previous snapshot |
| Schema migration failure | `sqlx migrate revert` to previous version; backup DB before migration |
| Config regression | Git-tracked config files; `git diff` to identify breaking changes |
| Binary regression | Cross-platform releases tagged; `git bisect` to identify faulty commit |

### HFT Standards

While crawlkit is a batch crawler (not latency-sensitive), HFT-inspired reliability and resource isolation standards improve robustness.

| Standard | Status | Implementation |
|----------|--------|----------------|
| **Deterministic behavior** | Planned | Same URL + same config → same output; seed-based PRNG for any randomized components |
| **Reliability targets** | Planned | 99.9% crawl completion rate (excluding target site errors); circuit breaker prevents cascade failures |
| **Resource isolation** | Planned | Per-crawl memory budgets; browser context isolation for JS rendering; bounded channel capacities |
| **Memory safety** | Active | Rust ownership model; `unsafe_code = "deny"` in `clippy.toml`; no GC pauses |
| **Throughput optimization** | Active | Target ≥ 50 pages/sec; benchmarked in CI |

#### Deterministic Behavior Design

```rust
pub struct DeterminismConfig {
    pub seed: Option<u64>,              // Seed for any randomized components
    pub user_agent_rotation: bool,      // Fixed UA vs rotating (default: false for determinism)
    pub dns_cache_ttl: Duration,        // Fixed TTL for DNS resolution consistency
}

// Deterministic hash for URL dedup
fn deterministic_url_hash(url: &Url) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(url.as_str().as_bytes());
    hasher.finalize().into()
}
```

#### Resource Isolation

```
Per-Crawl Resource Budget:
┌─────────────────────────────────────────────────────────────┐
│  Memory:  configurable (default: 2GB)                       │
│  ├── HTTP client pool:      200MB                          │
│  ├── HTML parser arena:     500MB                          │
│  ├── Analyzer working set:  300MB                          │
│  ├── SQLite write buffer:   200MB                          │
│  └── Browser contexts:      800MB (if JS enabled)          │
│                                                             │
│  CPU:  configurable (default: 4 cores)                      │
│  ├── Tokio worker threads:  2                              │
│  ├── Fetcher tasks:         bounded semaphore (64)         │
│  └── Browser contexts:      4 max                          │
│                                                             │
│  Disk: configurable (default: 1GB)                          │
│  ├── SQLite WAL:            200MB                          │
│  ├── Browser cache:         500MB                          │
│  └── Output files:          300MB                          │
│                                                             │
│  Enforcement:                                               │
│  - Memory: track allocations; abort crawl if exceeded      │
│  - CPU: Tokio runtime thread count; CPU affinity optional  │
│  - Disk: check free space before crawl; warn at 80%        │
└─────────────────────────────────────────────────────────────┘
```

#### Acceptance Criteria

| Criterion | Target |
|-----------|--------|
| Determinism | Same input → same output (excluding network timing) |
| Reliability | 99.9% crawl completion (excluding target site errors) |
| Resource limits | Crawl aborts gracefully if memory/disk budget exceeded |
| No panics | `catch_unwind` at task boundaries; never crash in production |
| Benchmark regression | CI detects > 5% throughput regression on reference hardware |

### Defense Standards

Defense-grade standards ensure crawlkit can be used in security-sensitive environments (compliance auditing, penetration testing support, government deployments).

| Standard | Status | Implementation |
|----------|--------|----------------|
| **Audit trail** | Planned | Every crawl logged with config hash, start/end time, page count, issue count; append-only audit log |
| **Input validation** | Active | URL parsing with `url` crate; depth/page limits enforced; pattern validation; malformed input rejected |
| **Encryption at rest** | Planned | Optional SQLCipher for SQLite; encrypted export files (AES-256-GCM); config file encryption |
| **Dependency auditing** | Active | `cargo audit` in CI; `cargo deny` for license and advisory checks |
| **Secrets management** | Active | No secrets in crawlkit; API keys in config files (not env vars); never logged or exported |
| **Malicious input handling** | Active | Error-tolerant HTML parser; no script execution unless JS enabled; DOM parser isolation |

#### Audit Trail Design

```rust
pub struct AuditEntry {
    pub timestamp: DateTime<Utc>,
    pub event_type: AuditEvent,
    pub crawl_id: Option<Uuid>,
    pub config_hash: String,           // SHA-256 of crawl config
    pub details: serde_json::Value,
}

pub enum AuditEvent {
    CrawlStarted,
    CrawlCompleted,
    CrawlAborted,
    ConfigChanged,
    ApiKeyCreated,
    ApiKeyRevoked,
    ExportGenerated,
    ErrorOccurred,
}

pub struct AuditLog {
    entries: Vec<AuditEntry>,          // Append-only
    file_path: PathBuf,               // Separate from crawl DB
}

impl AuditLog {
    pub fn append(&mut self, entry: AuditEntry) -> Result<(), io::Error> {
        // Atomic append; fsync after each write
        // Tamper-evident: chained SHA-256 hashes
    }
}
```

#### Encryption at Rest

```rust
pub struct EncryptionConfig {
    pub enabled: bool,                          // Default: false
    pub algorithm: EncryptionAlgorithm,         // Default: Aes256Gcm
    pub key_source: KeySource,                  // File, env, or keyring
}

pub enum EncryptionAlgorithm {
    Aes256Gcm,
    Xchacha20Poly1305,
}

pub enum KeySource {
    File(PathBuf),                              // Symmetric key file
    Environment(String),                        // Env var name
    Keyring(String),                            // OS keyring entry
}

// SQLCipher integration for SQLite
// Export encryption for portable reports
```

#### Acceptance Criteria

| Criterion | Target |
|-----------|--------|
| Audit logging | Every state-change event logged with timestamp, config hash, and details |
| Input validation | All URLs validated; depth ≤ 20; page limit ≥ 1; patterns validated |
| Encryption | Optional SQLCipher for DB; AES-256-GCM for exports; key never hardcoded |
| Dependency audit | `cargo audit` clean in CI; `cargo deny` passes license checks |
| No secrets | Zero hardcoded secrets; API keys only in user config files |
| Fuzzing | `cargo-fuzz` targets for HTML parser and URL normalizer |

### ECN Standards

ECN (Electronic Communication Network) standards apply to crawlkit's pipeline design for reliability, backpressure, and exactly-once semantics.

| Standard | Status | Implementation |
|----------|--------|----------------|
| **Deterministic error handling** | Active | `thiserror`-typed errors; every error variant documented |
| **Error recovery** | Active | Exponential backoff retry; configurable per error class |
| **Backpressure** | Planned | Bounded channels (capacity 1000); semaphore-based concurrency; slow consumer stalls producer |
| **Circuit breaker** | Planned | Per-domain circuit breaker; open after N consecutive failures; half-open after cooldown |
| **Idempotency** | Planned | URL + status code as idempotency key; skip re-crawl if unchanged within TTL |
| **Timeout handling** | Active | Per-request timeout (default: 30s); per-crawl timeout (configurable) |

#### Backpressure Design

```
Channel Capacities (bounded):
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│  fetch_tx ──────► fetch_rx     capacity: 1000              │
│  (URL frontier)   (fetcher)    backpressure: block producer │
│                                                             │
│  parse_tx ──────► parse_rx     capacity: 1000              │
│  (raw HTML)       (parser)     backpressure: block fetcher │
│                                                             │
│  analyze_tx ────► analyze_rx   capacity: 500               │
│  (ParsedPage)     (analyzer)   backpressure: block parser  │
│                                                             │
│  store_tx ──────► store_rx     capacity: 500               │
│  (Findings)       (storage)    backpressure: block analyzer│
│                                                             │
│  Semaphore: max_concurrent_requests (default: 64)          │
│  - Acquire before fetch                                     │
│  - Release after store                                      │
│  - Blocks if all permits taken                              │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

#### Circuit Breaker

```rust
pub struct CircuitBreaker {
    state: CircuitState,
    failure_count: AtomicUsize,
    success_count: AtomicUsize,
    last_failure_time: Mutex<Option<Instant>>,
    failure_threshold: usize,           // Default: 5
    cooldown_duration: Duration,        // Default: 60s
    half_open_max: usize,              // Default: 3
}

pub enum CircuitState {
    Closed,        // Normal operation; failures counted
    Open,          // Rejecting requests; waiting for cooldown
    HalfOpen,      // Testing; allow N requests through
}

impl CircuitBreaker {
    pub fn should_allow(&self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => self.cooldown_elapsed(),
            CircuitState::HalfOpen => self.success_count < self.half_open_max,
        }
    }

    pub fn record_success(&self) { /* increment success, reset if half-open */ }
    pub fn record_failure(&self) { /* increment failure, trip if threshold */ }
}
```

#### Idempotency Design

```rust
pub struct IdempotencyKey {
    pub url: Url,
    pub status_code: u16,
    pub content_hash: Option<String>,   // SHA-256 of body
    pub crawl_config_hash: String,      // Same config → same key
}

impl IdempotencyKey {
    pub fn from_fetch_result(result: &FetchResult, config: &CrawlConfig) -> Self {
        // Deterministic key generation
    }

    pub fn matches(&self, other: &Self) -> bool {
        // True if same URL + status + content + config
        // Used to skip redundant re-crawls
    }
}
```

#### Acceptance Criteria

| Criterion | Target |
|-----------|--------|
| Backpressure | No unbounded channels; producer blocks when consumer full |
| Circuit breaker | Opens after 5 consecutive domain failures; half-open after 60s |
| Idempotency | Re-crawl with same config skips if content unchanged within TTL |
| Timeout | Every I/O operation has a configurable timeout; no infinite waits |
| Error typing | All errors typed via `thiserror`; no bare `String` errors |
| Exactly-once | Best-effort via idempotency; documented trade-off vs exactly-once |

### Standards Compliance Summary

| Domain | Current Score | Target Score | Critical Gaps |
|--------|--------------|--------------|---------------|
| **FAANG** | 40% | 90% | Code review process, feature flags, rollback strategy, observability |
| **HFT** | 30% | 85% | Deterministic behavior, reliability targets, resource isolation |
| **Defense** | 30% | 85% | Audit trail, input validation, encryption at rest |
| **ECN** | 50% | 90% | Backpressure, circuit breaker, idempotency |

---

## Deployment

### Binary Distribution

```
Distribution Channels:
┌─────────────────────────────────────────────────────────┐
│                                                         │
│  1. GitHub Releases                                     │
│     - Pre-built binaries (Linux, macOS, Windows)        │
│     - SHA256 checksums                                  │
│     - GPG signatures                                    │
│                                                         │
│  2. Cargo Install                                       │
│     - `cargo install crawlkit`                          │
│     - Requires Rust toolchain                           │
│                                                         │
│  3. Docker                                              │
│     - `docker pull crawlkit/crawlkit:latest`            │
│     - Multi-stage build (small image)                   │
│                                                         │
│  4. Package Managers                                    │
│     - Homebrew (macOS)                                  │
│     - APT (Debian/Ubuntu)                               │
│     - Scoop (Windows)                                   │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

### CI/CD Pipeline

```
┌─────────────────────────────────────────────────────────┐
│                    CI Pipeline                           │
│                                                         │
│  ┌─────────┐   ┌─────────┐   ┌─────────┐              │
│  │  Lint   │──▶│  Test   │──▶│  Build  │              │
│  │ clippy  │   │ cargo   │   │ release │              │
│  │ fmt     │   │ test    │   │ binary  │              │
│  └─────────┘   └─────────┘   └────┬────┘              │
│                                    │                    │
│                                    ▼                    │
│                              ┌──────────┐               │
│                              │  Deploy  │               │
│                              │ release  │               │
│                              └──────────┘               │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

### Docker

```dockerfile
# Multi-stage build
FROM rust:1.75 AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/crawlkit /usr/local/bin/
ENTRYPOINT ["crawlkit"]
```

---

## Future Considerations

### Planned Features

1. **Distributed crawling** — Multiple machines, coordinated via Redis/PostgreSQL
2. **Incremental crawling** — Only crawl changed pages (diff-based)
3. **Custom analyzers** — User-defined analysis plugins
4. **API mode** — HTTP API for programmatic access
5. **Cloud storage** — S3/GCS/Azure Blob export
6. **Scheduled monitoring** — Periodic crawls with trend alerts
7. **Browser extension** — Quick single-page analysis
8. **VS Code extension** — In-editor site analysis

### Phase 7: JavaScript Rendering

The default HTTP-only crawler misses content rendered by client-side JavaScript. Phase 7 adds opt-in Playwright-based rendering for SPA-heavy sites, with strict resource controls.

#### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    JS Rendering Pipeline                     │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  Render Decision Engine                              │  │
│  │  - Inspect page for SPA indicators:                  │  │
│  │    · <div id="app"> or <div id="root">               │  │
│  │    · Framework detection (React, Vue, Angular)       │  │
│  │    · Client-side routing patterns                    │  │
│  │  - User-configured patterns (e.g., /app/*, /spa/*)   │  │
│  └──────────────────────────────────────────────────────┘  │
│                          │                                  │
│               ┌──────────┴──────────┐                      │
│               ▼                     ▼                      │
│  ┌────────────────────┐  ┌────────────────────────────┐   │
│  │  HTTP-Only Mode    │  │  Playwright Render Mode     │   │
│  │  (default)         │  │  (opt-in)                   │   │
│  │  - Zero overhead   │  │  - Chromium browser pool    │   │
│  │  - Static HTML     │  │  - JS execution             │   │
│  │  - Fast            │  │  - Network idle detection   │   │
│  └────────────────────┘  │  - Resource budget enforced │   │
│                          └────────────────────────────┘   │
│                                                             │
│  Resource Warnings (when --javascript is enabled):          │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  WARNING: JavaScript rendering is enabled.            │  │
│  │  - Memory: +500MB-2GB per browser context             │  │
│  │  - Speed: ~5-20 pages/sec (vs 50+ HTTP-only)         │  │
│  │  - CPU: Significant increase                          │  │
│  │  - Disk: Chromium binary ~150MB                       │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                             │
│  Fallback Policy:                                           │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  - Playwright unavailable → fall back to HTTP-only    │  │
│  │  - Browser crash → restart context, retry URL once    │  │
│  │  - Timeout → fall back to HTTP-only for that URL      │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

#### Config Additions

```rust
pub struct JavascriptConfig {
    pub enabled: bool,                          // Default: false
    pub max_browser_contexts: usize,            // Default: 4
    pub page_timeout: Duration,                 // Default: 30s
    pub wait_for_idle: WaitForIdle,             // Default: NetworkIdle
    pub render_budget_pages: usize,             // Default: 1000
    pub render_patterns: Vec<Pattern>,          // URL patterns requiring JS
    pub fallback_to_http: bool,                 // Default: true
    pub resource_limits: ResourceLimits,
}

pub struct ResourceLimits {
    pub max_memory_bytes: usize,                // Default: 4GB total
    pub max_cpu_cores: usize,                   // Default: 2
    pub max_disk_mb: usize,                     // Default: 500 (browser cache)
}

pub enum WaitForIdle {
    Load,
    DOMContentLoaded,
    NetworkIdle,        // Default: wait for no network activity for 500ms
    Custom(Duration),
}
```

#### Acceptance Criteria

| Criterion | Target |
|-----------|--------|
| HTTP-only mode (default) | Zero overhead, no Chromium dependency |
| Playwright integration | Opt-in via `--javascript` flag; warns on activation |
| Memory isolation | Browser contexts have independent memory budgets |
| Fallback | If Playwright unavailable or crashes, gracefully degrade to HTTP-only |
| Deterministic output | Same URL + same config → same rendered DOM (within timeout window) |
| Resource budget | Abort render if cumulative memory exceeds `max_memory_bytes` |

### Phase 7: REST API Mode

Programmatic access for integration into CI/CD pipelines, monitoring dashboards, and custom workflows. The API wraps the existing crawl engine with authentication, rate limiting, and OpenAPI documentation.

#### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    REST API Server                            │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  Axum HTTP Server                                    │  │
│  │  - Bind address: 0.0.0.0:8080 (configurable)        │  │
│  │  - TLS: Optional (via rustls)                        │  │
│  └──────────────────────────────────────────────────────┘  │
│                          │                                  │
│  ┌───────────────────────┼──────────────────────────────┐  │
│  │                       ▼                              │  │
│  │  ┌────────────────────────────────────────────────┐  │  │
│  │  │  Authentication Middleware                      │  │  │
│  │  │  - API key in X-API-Key header                 │  │  │
│  │  │  - Keys stored in SQLite (hashed)              │  │  │
│  │  │  - Rate limit per key                          │  │  │
│  │  └────────────────────────────────────────────────┘  │  │
│  │                       │                              │  │
│  │  ┌────────────────────────────────────────────────┐  │  │
│  │  │  Rate Limiter (per API key)                    │  │  │
│  │  │  - Token bucket: configurable burst + sustained│  │  │
│  │  │  - Headers: X-RateLimit-Remaining, Retry-After│  │  │
│  │  └────────────────────────────────────────────────┘  │  │
│  │                       │                              │  │
│  │  ┌────────────────────────────────────────────────┐  │  │
│  │  │  API Endpoints                                │  │  │
│  │  │  POST /api/v1/crawl          Start a crawl    │  │  │
│  │  │  GET  /api/v1/crawl/:id      Get crawl status │  │  │
│  │  │  GET  /api/v1/crawl/:id/results  Get results  │  │  │
│  │  │  DELETE /api/v1/crawl/:id    Cancel crawl     │  │  │
│  │  │  GET  /api/v1/health         Health check     │  │  │
│  │  │  GET  /api/v1/docs           OpenAPI spec     │  │  │
│  │  └────────────────────────────────────────────────┘  │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  OpenAPI / Swagger Documentation                     │  │
│  │  - Auto-generated from Axum handlers via utoipa      │  │
│  │  - Served at /api/v1/docs (Swagger UI)               │  │
│  │  - Exportable JSON/YAML spec                         │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

#### API Data Models

```rust
#[derive(Deserialize, ToSchema)]
pub struct CreateCrawlRequest {
    pub url: Url,
    pub max_depth: Option<usize>,
    pub max_pages: Option<usize>,
    pub javascript: Option<bool>,
    pub include_patterns: Option<Vec<String>>,
    pub exclude_patterns: Option<Vec<String>>,
    pub output_formats: Option<Vec<OutputFormat>>,
}

#[derive(Serialize, ToSchema)]
pub struct CrawlResponse {
    pub id: Uuid,
    pub status: CrawlStatus,
    pub url: Url,
    pub created_at: DateTime<Utc>,
    pub pages_crawled: Option<usize>,
    pub estimated_duration: Option<Duration>,
}

#[derive(Serialize, ToSchema)]
pub struct ApiKey {
    pub key: String,          // Returned once on creation
    pub name: String,
    pub rate_limit: RateLimit,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Serialize, ToSchema)]
pub struct RateLimit {
    pub requests_per_minute: usize,    // Default: 60
    pub burst_size: usize,             // Default: 10
}
```

#### Security Controls

| Control | Implementation |
|---------|---------------|
| Authentication | API key via `X-API-Key` header; keys hashed with argon2id |
| Rate limiting | Per-key token bucket; 429 response with `Retry-After` header |
| Input validation | URL validation, depth/page limits, pattern validation |
| CORS | Configurable origins; disabled by default |
| TLS | Optional rustls integration for HTTPS |
| Audit logging | All API requests logged with key fingerprint, timestamp, path |

#### Acceptance Criteria

| Criterion | Target |
|-----------|--------|
| Authentication | API key required for all endpoints except `/health` and `/docs` |
| Rate limiting | Returns 429 with `Retry-After` when exceeded; per-key isolation |
| OpenAPI spec | Auto-generated from code; valid Swagger 3.0 JSON at `/api/v1/docs` |
| CLI parity | API supports all `crawlkit crawl` flags plus async polling |
| Concurrency limit | Max concurrent crawls per key configurable (default: 3) |
| Cleanup | Stale crawl results auto-purged after configurable TTL (default: 24h) |

### Phase 7: Backlink Analysis

Backlink data from external APIs enriches crawl results with off-page SEO signals. This phase integrates with third-party APIs and builds an internal link graph.

#### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Backlink Analysis Pipeline                 │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  External API Adapters                                │  │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────────────────┐ │  │
│  │  │ Ahrefs   │ │ Majestic │ │ Google Search Console│ │  │
│  │  │ API      │ │ API      │ │ API                  │ │  │
│  │  └──────────┘ └──────────┘ └──────────────────────┘ │  │
│  │  - Rate limited per adapter                          │  │
│  │  - API key stored in config (not in DB)              │  │
│  │  - Graceful degradation if API unavailable           │  │
│  └──────────────────────────────────────────────────────┘  │
│                          │                                  │
│  ┌───────────────────────┼──────────────────────────────┐  │
│  │                       ▼                              │  │
│  │  ┌────────────────────────────────────────────────┐  │  │
│  │  │  Internal Link Graph Builder                   │  │  │
│  │  │  - Directed graph (page → page)                │  │  │
│  │  │  - Weighted by anchor text relevance           │  │  │
│  │  │  - Circular dependency detection               │  │  │
│  │  │  - PageRank computation (damped)               │  │  │
│  │  └────────────────────────────────────────────────┘  │  │
│  │                       │                              │  │
│  │  ┌────────────────────────────────────────────────┐  │  │
│  │  │  Link Graph Visualization                      │  │  │
│  │  │  - DOT format export (Graphviz)                │  │  │
│  │  │  - HTML interactive (D3.js force-directed)     │  │  │
│  │  │  - CSV adjacency list                          │  │  │
│  │  └────────────────────────────────────────────────┘  │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

#### Data Models

```rust
pub struct BacklinkProfile {
    pub target_url: Url,
    pub total_backlinks: usize,
    pub referring_domains: usize,
    pub domain_authority: Option<f64>,
    pub top_backlinks: Vec<Backlink>,
    pub anchor_text_distribution: HashMap<String, usize>,
}

pub struct Backlink {
    pub source_url: Url,
    pub target_url: Url,
    pub anchor_text: Option<String>,
    pub rel: Option<String>,
    pub discovered_at: DateTime<Utc>,
    pub source_domain_authority: Option<f64>,
}

pub struct InternalLinkGraph {
    pub nodes: Vec<LinkNode>,
    pub edges: Vec<LinkEdge>,
    pub pagerank: HashMap<Url, f64>,
}

pub struct LinkNode {
    pub url: Url,
    pub inbound_count: usize,
    pub outbound_count: usize,
    pub is_orphan: bool,
}

pub struct LinkEdge {
    pub source: Url,
    pub target: Url,
    pub anchor_text: Option<String>,
    pub is_nofollow: bool,
}
```

#### Acceptance Criteria

| Criterion | Target |
|-----------|--------|
| External API adapters | Ahrefs, Majestic, Google Search Console supported |
| API key security | Keys in config file (not SQLite); never logged or exported |
| Internal link graph | Constructed from crawl data; includes all discovered links |
| PageRank | Damping factor 0.85; computed for all pages with > 0 inbound links |
| Orphan detection | Pages with 0 inbound internal links flagged |
| Visualization export | DOT, HTML (D3.js force-directed), CSV adjacency list |
| Graceful degradation | If external API unavailable, use only internal link data |

### Phase 7: RUM Data Integration

Real User Monitoring (RUM) data overlays lab-based crawl metrics with actual field data from Google Analytics and Chrome User Experience Report (CrUX). This enables data-driven prioritization of performance fixes based on real user impact.

#### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    RUM Data Pipeline                          │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  Data Sources                                        │  │
│  │  ┌──────────────────┐  ┌──────────────────────────┐  │  │
│  │  │ Google Analytics │  │ Chrome UX Report (CrUX)  │  │  │
│  │  │ Reporting API v4 │  │ BigQuery / PageSpeed API │  │  │
│  │  └──────────────────┘  └──────────────────────────┘  │  │
│  │  ┌──────────────────┐                                │  │
│  │  │ Custom RUM Beacon│  (optional self-hosted)        │  │
│  │  └──────────────────┘                                │  │
│  └──────────────────────────────────────────────────────┘  │
│                          │                                  │
│  ┌───────────────────────┼──────────────────────────────┐  │
│  │                       ▼                              │  │
│  │  ┌────────────────────────────────────────────────┐  │  │
│  │  │  Data Normalizer                               │  │  │
│  │  │  - Map GA page paths → crawl URLs              │  │  │
│  │  │  - Aggregate by URL (exact + pattern match)    │  │  │
│  │  │  - Time-windowed aggregation (28-day default)  │  │  │
│  │  └────────────────────────────────────────────────┘  │  │
│  │                       │                              │  │
│  │  ┌────────────────────────────────────────────────┐  │  │
│  │  │  Core Web Vitals Field Data Overlay            │  │  │
│  │  │  - LCP (p75), INP (p75), CLS (p75)           │  │  │
│  │  │  - FCP (p75), TTFB (p75)                      │  │  │
│  │  │  - Device category (mobile/desktop/tablet)     │  │  │
│  │  │  - Country/region breakdown                    │  │  │
│  │  └────────────────────────────────────────────────┘  │  │
│  │                       │                              │  │
│  │  ┌────────────────────────────────────────────────┐  │  │
│  │  │  Merged Report                                 │  │  │
│  │  │  - Lab data (crawl) + Field data (RUM)         │  │  │
│  │  │  - Delta highlighting (lab ≠ field)            │  │  │
│  │  │  - Priority scoring by real-user impact        │  │  │
│  │  └────────────────────────────────────────────────┘  │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

#### Data Models

```rust
pub struct RumData {
    pub url: Url,
    pub source: RumSource,
    pub time_window: TimeWindow,
    pub page_views: u64,
    pub core_web_vitals: CoreWebVitals,
    pub device_distribution: DeviceDistribution,
    pub country_data: Option<HashMap<String, CoreWebVitals>>,
}

pub enum RumSource {
    GoogleAnalytics,
    Crux,
    CustomBeacon,
}

pub struct CoreWebVitals {
    pub lcp_p75: Option<Duration>,
    pub inp_p75: Option<Duration>,
    pub cls_p75: Option<f64>,
    pub fcp_p75: Option<Duration>,
    pub ttfb_p75: Option<Duration>,
}

pub struct DeviceDistribution {
    pub mobile: f64,       // percentage
    pub desktop: f64,
    pub tablet: f64,
}

pub struct MergedPageMetrics {
    pub url: Url,
    pub lab: Option<LabMetrics>,       // From crawl
    pub field: Option<RumData>,        // From RUM
    pub deltas: Vec<MetricDelta>,      // Lab - Field differences
    pub priority_score: f64,           // Weighted by real-user impact
}
```

#### Acceptance Criteria

| Criterion | Target |
|-----------|--------|
| GA integration | Import via Reporting API v4; map page paths to crawl URLs |
| CrUX integration | Fetch via PageSpeed Insights API or BigQuery; 28-day window |
| Data normalization | Exact URL match + pattern-based aggregation for query params |
| Field data overlay | LCP, INP, CLS, FCP, TTFB displayed alongside lab metrics |
| Delta highlighting | Visual indicator where lab and field data diverge significantly |
| Privacy | No PII in field data; country-level only, no user-level |
| Offline fallback | If RUM APIs unavailable, report lab-only with warning |

### Scaling Considerations

```
Scaling Strategy:
┌─────────────────────────────────────────────────────────┐
│                                                         │
│  Phase 1: Single Machine                                │
│  - Current architecture                                │
│  - Handles most use cases                              │
│  - Up to 100k pages                                    │
│                                                         │
│  Phase 2: Multi-threaded                                │
│  - Tokio work-stealing                                 │
│  - Shared memory                                       │
│  - Up to 1M pages                                      │
│                                                         │
│  Phase 3: Distributed                                   │
│  ┌─────────────────────────────────────────────────┐   │
│  │  ┌──────────┐   ┌──────────┐   ┌──────────┐    │   │
│  │  │ Worker 1 │   │ Worker 2 │   │ Worker N │    │   │
│  │  └────┬─────┘   └────┬─────┘   └────┬─────┘    │   │
│  │       │              │              │           │   │
│  │       └──────────────┼──────────────┘           │   │
│  │                      ▼                          │   │
│  │              ┌──────────────┐                   │   │
│  │              │  Coordinator │                   │   │
│  │              │  (Redis/PG)  │                   │   │
│  │              └──────────────┘                   │   │
│  └─────────────────────────────────────────────────┘   │
│  - Up to 10M+ pages                                    │
│  - Horizontal scaling                                  │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

### Extensibility Points

1. **Custom analyzers** — Implement `AnalyzerPlugin` trait
2. **Custom exporters** — Implement `ExportEngine` trait
3. **Custom fetchers** — Implement `Fetcher` trait (e.g., headless browser)
4. **Custom storage backends** — Implement `Storage` trait
5. **Webhook notifications** — Notify on crawl completion

---

## Appendix

### Dependency Tree

```
crawlkit
├── tokio (async runtime)
├── reqwest (HTTP client)
│   ├── hyper (HTTP implementation)
│   └── native-tls (TLS)
├── scraper (HTML parsing)
│   ├── html5ever (HTML5 parser)
│   └── selectors (CSS selectors)
├── sqlx (SQLite async)
├── clap (CLI)
├── serde (serialization)
│   ├── serde_json
│   └── csv
├── chrono (date/time)
├── url (URL parsing)
├── uuid (unique IDs)
├── rust_decimal (precision math)
├── thiserror (error handling)
├── tracing (logging)
├── indicatif (progress bars)
└── colored (terminal colors)
```

### Configuration Defaults

```toml
[crawler]
max_concurrent_requests = 10
requests_per_second = 5.0
per_domain_rps = 2.0
max_redirects = 20
timeout_secs = 30
respect_robots_txt = true
javascript_rendering = false
crawl_depth = null  # unlimited

[output]
formats = ["html", "csv", "json"]
report_theme = "light"

[storage]
sqlite_wal_mode = true
compression = true
```

---

*Last updated: 2026-07-22*
*Version: 1.0.0*
*Author: crawlkit team*
