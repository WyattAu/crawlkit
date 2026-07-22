# Writing Custom Analyzers

crawlkit's analyzer system is extensible. You can add domain-specific checks by implementing the `Analyzer` trait.

## The `Analyzer` trait

```rust
pub trait Analyzer: Send + Sync {
    /// Returns the human-readable name of this analyzer.
    fn name(&self) -> &str;

    /// Analyze a page and return any findings/issues.
    fn analyze(&self, ctx: &AnalysisContext, config: &CrawlConfig) -> Vec<Finding>;
}
```

### Key types

| Type | Purpose |
|------|---------|
| `AnalysisContext` | Bundles parsed HTML, HTTP status, headers, response time, and redirect chain |
| `Finding` | A single issue detected — severity, category, code, title, description, recommendation |
| `Severity` | `Critical`, `Error`, `Warning`, `Info` |
| `IssueCategory` | `Http`, `Seo`, `Content`, `Links`, `Images`, `Schema`, `Security`, `Performance`, `Mobile`, `Accessibility`, `Social`, `Custom(String)` |
| `CrawlConfig` | The crawl configuration — available for context-aware analysis |

## Step-by-step: Building a custom analyzer

### 1. Create the struct

```rust
struct ResponseSizeAnalyzer {
    max_bytes: usize,
}

impl ResponseSizeAnalyzer {
    fn new(max_bytes: usize) -> Self {
        Self { max_bytes }
    }
}
```

### 2. Implement the trait

```rust
use crawlkit_core::analyzers::{Analyzer, AnalysisContext, Finding};
use crawlkit_core::storage::{IssueCategory, Severity};
use crawlkit_core::CrawlConfig;

impl Analyzer for ResponseSizeAnalyzer {
    fn name(&self) -> &str {
        "response-size"
    }

    fn analyze(&self, ctx: &AnalysisContext, _config: &CrawlConfig) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        // ctx.page is a ParsedPage — check the word_count as a proxy,
        // or access headers for Content-Length.
        // For a real size check you'd pass body_size through the context.

        // Example: flag pages with zero content (possible soft 404)
        if ctx.page.word_count == 0 && ctx.status_code == Some(200) {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Content,
                code: "CUSTOM010".to_string(),
                title: "Empty page body".to_string(),
                description: "Page returned 200 but has no visible text content.".to_string(),
                url: url.clone(),
                recommendation: "Verify the page renders correctly. Fix server-side rendering \
                                 issues or remove the page."
                    .to_string(),
            });
        }

        findings
    }
}
```

### 3. Use it

```rust
let analyzer = ResponseSizeAnalyzer::new(500_000);
let findings = analyzer.analyze(&ctx, &config);
```

## Complete examples

### Thin content detector

Flags pages below a word count threshold:

```rust
struct ThinContentAnalyzer {
    min_words: usize,
}

impl ThinContentAnalyzer {
    fn new(min_words: usize) -> Self {
        Self { min_words }
    }
}

impl Analyzer for ThinContentAnalyzer {
    fn name(&self) -> &str {
        "thin-content"
    }

    fn analyze(&self, ctx: &AnalysisContext, _config: &CrawlConfig) -> Vec<Finding> {
        if ctx.page.word_count < self.min_words {
            vec![Finding {
                severity: Severity::Warning,
                category: IssueCategory::Content,
                code: "CUSTOM001".to_string(),
                title: "Thin content".to_string(),
                description: format!(
                    "Page has {} words, below the minimum of {}.",
                    ctx.page.word_count, self.min_words
                ),
                url: ctx.page.url.clone(),
                recommendation: "Add more unique, valuable content.".to_string(),
            }]
        } else {
            vec![]
        }
    }
}
```

### Missing structured data validator

Checks that specific Schema.org types are present:

```rust
struct RequiredSchemaAnalyzer {
    required_types: Vec<String>,
}

impl RequiredSchemaAnalyzer {
    fn new(required_types: Vec<&str>) -> Self {
        Self {
            required_types: required_types.into_iter().map(String::from).collect(),
        }
    }
}

impl Analyzer for RequiredSchemaAnalyzer {
    fn name(&self) -> &str {
        "required-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext, _config: &CrawlConfig) -> Vec<Finding> {
        let mut findings = Vec::new();
        let found_types: Vec<String> = ctx
            .page
            .structured_data
            .iter()
            .filter_map(|sd| sd.r#type.clone())
            .collect();

        for required in &self.required_types {
            if !found_types.iter().any(|t| t == required) {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "CUSTOM020".to_string(),
                    title: format!("Missing {required} structured data"),
                    description: format!(
                        "No JSON-LD with @type \"{required}\" was found on this page."
                    ),
                    url: ctx.page.url.clone(),
                    recommendation: format!(
                        "Add a <script type=\"application/ld+json\"> block with @type \"{required}\"."
                    ),
                });
            }
        }

        findings
    }
}
```

### Custom `IssueCategory`

For project-specific categories, use `IssueCategory::Custom`:

```rust
use crawlkit_core::storage::IssueCategory;

// This round-trips through "custom:ecommerce" in the database
let category = IssueCategory::Custom("ecommerce".to_string());
assert_eq!(category.as_str(), "custom:ecommerce");
```

## AnalysisContext fields

The `AnalysisContext` provides everything an analyzer needs:

```rust
pub struct AnalysisContext<'a> {
    /// The parsed page content (meta tags, headings, links, images, forms, etc.)
    pub page: &'a ParsedPage,

    /// HTTP status code (if fetched)
    pub status_code: Option<u16>,

    /// Response headers as (name, value) pairs
    pub headers: &'a [(String, String)],

    /// Time from request start to response complete
    pub response_time: Option<Duration>,

    /// Redirect hops from original to final URL
    pub redirect_chain: &'a [RedirectHop],
}
```

### ParsedPage highlights

```rust
pub struct ParsedPage {
    pub url: String,
    pub meta: MetaTags,           // title, description, canonical, OG, Twitter, hreflang
    pub headings: Vec<Heading>,   // H1-H6 with level, text, length
    pub links: Vec<ExtractedLink>,// href, text, rel, is_external
    pub images: Vec<ExtractedImage>, // src, alt, width, height, has_alt, is_lazy_loaded
    pub forms: Vec<ExtractedForm>,   // action, method, inputs with ARIA info
    pub scripts: Vec<ScriptInfo>,    // src, async, defer, type
    pub styles: Vec<StyleInfo>,      // href, media, is_inline
    pub structured_data: Vec<StructuredData>, // JSON-LD blocks
    pub word_count: usize,
    // Accessibility fields
    pub has_skip_link: bool,
    pub has_main_landmark: bool,
    pub has_lang_attribute: bool,
    // ... and more
}
```

## Severity guidelines

| Severity | When to use |
|----------|-------------|
| `Critical` | Site is down, data loss risk, security vulnerability |
| `Error` | Broken functionality, missing required elements |
| `Warning` | Suboptimal configuration, potential SEO impact |
| `Info` | Observations, best-practice suggestions |

## Code naming conventions

Use a consistent prefix for your custom codes:

```rust
// Format: PREFIX + 3-digit number
code: "CUSTOM001".to_string()   // generic custom
code: "ECOM001".to_string()     // e-commerce specific
code: "A11Y001".to_string()     // accessibility custom
```

## Tips

- **Keep analyzers focused.** One analyzer = one concern.
- **Always return `Vec<Finding>`, not a single finding.** An analyzer may detect multiple issues per page.
- **Use `Send + Sync`.** Analyzers run concurrently across pages.
- **Don't make network calls** inside `analyze()`. The context is provided pre-fetched.
- **Use existing categories** when they fit. Reserve `Custom` for truly domain-specific checks.
