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

The crawler engine is responsible for fetching web pages with full HTTP semantics.

#### Architecture

```
┌─────────────────────────────────────────────────────────┐
│                   CrawlerEngine                         │
│                                                         │
│  ┌──────────────────────────────────────────────────┐  │
│  │                  URLQueue                         │  │
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
pub struct CrawlerConfig {
    pub max_concurrent_requests: usize,      // Default: 10
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
│  Generated: 2025-01-15 10:30 UTC                        │
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
    pub config: CrawlerConfig,
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

*Last updated: 2025-01-15*
*Version: 1.0.0*
*Author: crawlkit team*
