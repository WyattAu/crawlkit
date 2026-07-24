# AI Result Optimization Analyzers — Implementation Spec

**ADR Reference:** ADR-001
**Status:** Proposed
**Estimated Effort:** 2-3 weeks (all static, ships with v1.0)

---

## Overview

Four analyzers detect AI-specific SEO signals for Google SGE/AI Overviews, Bing Copilot, Perplexity, ChatGPT Browse, and other AI search engines.

| Analyzer | Focus | Priority |
|----------|-------|----------|
| `AiCrawlerAccessibilityAnalyzer` | robots.txt AI bot directives | P0 |
| `AiContentStructureAnalyzer` | AI-friendly content patterns | P0 |
| `AiCitationEligibilityAnalyzer` | Source authority signals | P1 |
| `AiAnswerBoxAnalyzer` | FAQ/HowTo/Q&A schema readiness | P1 |

---

## Analyzer 1: AiCrawlerAccessibilityAnalyzer

### Purpose
Detect whether AI crawlers can access and index the site.

### Detection Signals

| Signal ID | Pattern | Severity | Category | Description |
|-----------|---------|----------|----------|-------------|
| AI-ACC001 | `robots.txt` blocks `GPTBot` | Critical | SEO | ChatGPT cannot crawl site |
| AI-ACC002 | `robots.txt` blocks `Google-Extended` | Critical | SEO | Google AI Overviews cannot use content |
| AI-ACC003 | `robots.txt` blocks `PerplexityBot` | Critical | SEO | Perplexity cannot cite site |
| AI-ACC004 | `robots.txt` blocks `Anthropic-AI` | Critical | SEO | Anthropic Claude cannot access |
| AI-ACC005 | `robots.txt` blocks `Bytespider` | Warning | SEO | ByteDance AI cannot access |
| AI-ACC006 | `robots.txt` blocks `Amazonbot` | Warning | SEO | Amazon/Alexa AI cannot access |
| AI-ACC007 | `robots.txt` blocks all crawlers for AI-related paths | Info | SEO | AI-specific paths blocked |
| AI-ACC008 | `X-Robots-Tag: noai` header present | Warning | SEO | Explicit AI indexing directive |
| AI-ACC009 | `robots.txt` missing entirely | Info | SEO | No crawl directives (open to all) |
| AI-ACC010 | AI bot blocked but `nosnippet` absent | Info | SEO | Inconsistent AI policy |

### Known AI Bots Registry

```rust
pub const AI_BOTS: &[AiBot] = &[
    AiBot {
        name: "GPTBot",
        owner: "OpenAI",
        purpose: "ChatGPT training and browsing",
        severity: Severity::Critical,
    },
    AiBot {
        name: "Google-Extended",
        owner: "Google",
        purpose: "Gemini/AI Overviews training",
        severity: Severity::Critical,
    },
    AiBot {
        name: "PerplexityBot",
        owner: "Perplexity AI",
        purpose: "Real-time search answers",
        severity: Severity::Critical,
    },
    AiBot {
        name: "Anthropic-AI",
        owner: "Anthropic",
        purpose: "Claude training",
        severity: Severity::Critical,
    },
    AiBot {
        name: "Bytespider",
        owner: "ByteDance",
        purpose: "TikTok/Douyin AI",
        severity: Severity::Warning,
    },
    AiBot {
        name: "Amazonbot",
        owner: "Amazon",
        purpose: "Alexa/ shopping AI",
        severity: Severity::Warning,
    },
    AiBot {
        name: "Meta-ExternalAgent",
        owner: "Meta",
        purpose: "LLaMA training",
        severity: Severity::Warning,
    },
    AiBot {
        name: "Applebot-Extended",
        owner: "Apple",
        purpose: "Apple Intelligence",
        severity: Severity::Warning,
    },
    AiBot {
        name: "ClaudeBot",
        owner: "Anthropic",
        purpose: "Claude web search",
        severity: Severity::Critical,
    },
    AiBot {
        name: "cohere-ai",
        owner: "Cohere",
        purpose: "Command R training",
        severity: Severity::Info,
    },
];
```

### Implementation

```rust
pub struct AiCrawlerAccessibilityAnalyzer {
    bots: &'static [AiBot],
}

impl AiCrawlerAccessibilityAnalyzer {
    pub fn new() -> Self {
        Self { bots: AI_BOTS }
    }
}

impl Analyzer for AiCrawlerAccessibilityAnalyzer {
    fn name(&self) -> &str {
        "ai-crawler-accessibility"
    }

    fn analyze(&self, ctx: &AnalysisContext, _config: &CrawlConfig) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        // Parse robots.txt content (pre-fetched and stored in AnalysisContext)
        let robots_txt = match &ctx.robots_txt {
            Some(txt) => txt,
            None => {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Seo,
                    code: "AI-ACC009".to_string(),
                    title: "No robots.txt found".to_string(),
                    description: "Site has no robots.txt file. All crawlers, including AI bots, \
                        have unrestricted access."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Consider adding robots.txt to control AI crawler access."
                        .to_string(),
                });
                return findings;
            }
        };

        for bot in self.bots {
            let is_blocked = robots_txt_disallows_bot(robots_txt, bot.name);
            if is_blocked {
                findings.push(Finding {
                    severity: bot.severity.clone(),
                    category: IssueCategory::Seo,
                    code: format!("AI-ACC{:03}", bot.ordinal()),
                    title: format!("AI bot '{}' blocked by robots.txt", bot.name),
                    description: format!(
                        "robots.txt blocks {}. {} will not be able to crawl or index \
                        this site for AI purposes.",
                        bot.name, bot.owner
                    ),
                    url: url.clone(),
                    recommendation: format!(
                        "If you want {} to access your site, remove the Disallow rule \
                        for {} in robots.txt.",
                        bot.owner, bot.name
                    ),
                });
            }
        }

        findings
    }
}
```

---

## Analyzer 2: AiContentStructureAnalyzer

### Purpose
Detect whether content is structured for AI extraction and comprehension.

### Detection Signals

| Signal ID | Pattern | Severity | Category | Description |
|-----------|---------|----------|----------|-------------|
| AI-CS001 | No clear answer-format content (lists, tables, definitions) | Warning | Content | AI prefers structured, extractable content |
| AI-CS002 | Paragraphs > 300 words without subheadings | Warning | Content | Hard for AI to extract key points |
| AI-CS003 | Missing introductory summary/TL;DR | Info | Content | AI benefits from upfront answers |
| AI-CS004 | No numbered/bulleted lists | Info | Content | Lists are easily parsed by AI |
| AI-CS005 | No tables for comparative data | Info | Content | Tables provide structured comparisons |
| AI-CS006 | Content uses jargon without definition | Warning | Content | AI may misinterpret specialized terms |
| AI-CS007 | No questions posed in content | Info | Content | Q&A format aligns with AI query patterns |
| AI-CS008 | Missing date/time metadata | Warning | Content | AI prioritizes fresh, dated content |
| AI-CS009 | No author attribution | Warning | Content | AI cites authoritative, attributed sources |
| AI-CS010 | Content-to-HTML ratio < 20% | Warning | Content | Too much markup, little extractable text |

### Implementation

```rust
pub struct AiContentStructureAnalyzer;

impl Analyzer for AiContentStructureAnalyzer {
    fn name(&self) -> &str {
        "ai-content-structure"
    }

    fn analyze(&self, ctx: &AnalysisContext, _config: &CrawlConfig) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let text = &ctx.page.visible_text;

        // AI-CS001: No structured content
        let has_lists = text.contains("• ") || text.contains("1.") || text.contains("- ");
        let has_tables = text.contains("|") && text.lines().filter(|l| l.contains('|')).count() > 2;
        let has_definitions = text.contains(":") && text.lines()
            .filter(|l| l.ends_with(':') && l.len() < 50)
            .count() > 2;

        if !has_lists && !has_tables && !has_definitions {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Content,
                code: "AI-CS001".to_string(),
                title: "No structured content detected".to_string(),
                description: "AI search engines prefer content with lists, tables, or \
                    definition-style formatting for easy extraction."
                    .to_string(),
                url: url.clone(),
                recommendation: "Add bulleted/numbered lists, tables, or definition-style \
                    sections to make content AI-extractable."
                    .to_string(),
            });
        }

        // AI-CS002: Long paragraphs without subheadings
        let paragraphs: Vec<&str> = text.split("\n\n").collect();
        let long_paras = paragraphs.iter()
            .filter(|p| p.split_whitespace().count() > 300)
            .count();
        if long_paras > 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Content,
                code: "AI-CS002".to_string(),
                title: "Long paragraphs without subheadings".to_string(),
                description: format!(
                    "{} paragraph(s) exceed 300 words without subheadings. \
                    AI struggles to extract key points from dense text.",
                    long_paras
                ),
                url: url.clone(),
                recommendation: "Break long paragraphs into sections with H2/H3 subheadings."
                    .to_string(),
            });
        }

        // AI-CS008: Missing date metadata
        let has_date = ctx.page.meta_tags.is_some() // Simplified check
            || ctx.page.visible_text.contains("2024")
            || ctx.page.visible_text.contains("2025")
            || ctx.page.visible_text.contains("2026");
        if !has_date {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Content,
                code: "AI-CS008".to_string(),
                title: "Missing date metadata".to_string(),
                description: "Content lacks visible date information. AI search engines \
                    prioritize fresh, dated content."
                    .to_string(),
                url: url.clone(),
                recommendation: "Add publication date in <time> tag or visible byline."
                    .to_string(),
            });
        }

        // AI-CS009: Missing author attribution
        let has_author = text.to_lowercase().contains("by ")
            || text.to_lowercase().contains("author:")
            || text.to_lowercase().contains("written by");
        if !has_author && text.split_whitespace().count() > 500 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Content,
                code: "AI-CS009".to_string(),
                title: "Missing author attribution".to_string(),
                description: "Long-form content lacks author attribution. AI engines \
                    cite authoritative, attributed sources."
                    .to_string(),
                url: url.clone(),
                recommendation: "Add author name and credentials in visible byline."
                    .to_string(),
            });
        }

        findings
    }
}
```

---

## Analyzer 3: AiCitationEligibilityAnalyzer

### Purpose
Detect signals that make content citable by AI search engines.

### Detection Signals

| Signal ID | Pattern | Severity | Category | Description |
|-----------|---------|----------|----------|-------------|
| AI-CIT001 | No canonical URL | Error | SEO | AI cannot deduplicate canonical source |
| AI-CIT002 | No author name | Warning | Authority | AI prefers attributed content |
| AI-CIT003 | No publication date | Warning | Freshness | AI prioritizes dated content |
| AI-CIT004 | No external references/links | Info | Authority | Well-sourced content ranks higher |
| AI-CIT005 | No structured data | Warning | SEO | Schema.org helps AI understand content |
| AI-CIT006 | Domain not in major knowledge bases | Info | Authority | Notable domains cited more often |
| AI-CIT007 | No OpenGraph tags | Warning | Social | OG tags signal content legitimacy |
| AI-CIT008 | Missing `article:author` OG tag | Info | Authority | Author signal for AI citation |

### Implementation

```rust
pub struct AiCitationEligibilityAnalyzer;

impl Analyzer for AiCitationEligibilityAnalyzer {
    fn name(&self) -> &str {
        "ai-citation-eligibility"
    }

    fn analyze(&self, ctx: &AnalysisContext, _config: &CrawlConfig) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        // AI-CIT001: Missing canonical
        if ctx.page.canonical.is_none() {
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Seo,
                code: "AI-CIT001".to_string(),
                title: "Missing canonical URL".to_string(),
                description: "No <link rel=\"canonical\"> found. AI search engines cannot \
                    determine the canonical version of this page."
                    .to_string(),
                url: url.clone(),
                recommendation: "Add <link rel=\"canonical\" href=\"...\"> pointing to \
                    the preferred URL."
                    .to_string(),
            });
        }

        // AI-CIT005: No structured data
        if ctx.page.structured_data.is_empty() {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "AI-CIT005".to_string(),
                title: "No structured data found".to_string(),
                description: "Page has no JSON-LD or Microdata. Structured data helps \
                    AI understand content type and relationships."
                    .to_string(),
                url: url.clone(),
                recommendation: "Add JSON-LD structured data with appropriate Schema.org type."
                    .to_string(),
            });
        }

        // AI-CIT007: Missing OpenGraph
        if ctx.page.open_graph.is_none() || ctx.page.open_graph.as_ref()
            .map(|og| og.title.is_none() && og.description.is_none() && og.image.is_none())
            .unwrap_or(true)
        {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Social,
                code: "AI-CIT007".to_string(),
                title: "Missing or incomplete OpenGraph tags".to_string(),
                description: "OpenGraph tags signal content legitimacy to AI engines. \
                    Incomplete OG tags reduce citation confidence."
                    .to_string(),
                url: url.clone(),
                recommendation: "Add og:title, og:description, and og:image meta tags."
                    .to_string(),
            });
        }

        findings
    }
}
```

---

## Analyzer 4: AiAnswerBoxAnalyzer

### Purpose
Detect whether content is optimized for AI answer boxes and featured snippets.

### Detection Signals

| Signal ID | Pattern | Severity | Category | Description |
|-----------|---------|----------|----------|-------------|
| AI-AB001 | No FAQ schema (`FAQPage`) | Info | Schema | FAQ pages get AI answer box treatment |
| AI-AB002 | No HowTo schema (`HowTo`) | Info | Schema | HowTo content gets step-by-step AI answers |
| AI-AB003 | No Q&A format in content | Info | Content | Questions/answers align with AI query patterns |
| AI-AB004 | Answer > 50 words before question appears | Warning | Content | AI prefers upfront answers |
| AI-AB005 | No definition-style content | Info | Content | Definitions are easily extracted for AI answers |
| AI-AB006 | No "What is" / "How to" / "Why" headings | Info | Content | Question-style headings match AI queries |
| AI-AB007 | Missing `speakable` schema property | Info | Schema | `speakable` marks content for voice AI |
| AI-AB008 | No `<address>` or contact schema | Info | Schema | Contact info needed for local AI answers |

### Implementation

```rust
pub struct AiAnswerBoxAnalyzer;

impl Analyzer for AiAnswerBoxAnalyzer {
    fn name(&self) -> &str {
        "ai-answer-box"
    }

    fn analyze(&self, ctx: &AnalysisContext, _config: &CrawlConfig) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        // AI-AB001: No FAQ schema
        let has_faq = ctx.page.structured_data.iter().any(|sd| {
            sd.get("@type").map(|t| t == "FAQPage").unwrap_or(false)
        });
        if !has_faq && ctx.page.headings.iter().any(|h| h.text.contains('?')) {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Seo,
                code: "AI-AB001".to_string(),
                title: "No FAQ schema detected".to_string(),
                description: "Page contains question-style headings but lacks FAQPage \
                    structured data. FAQ schema enables AI answer boxes."
                    .to_string(),
                url: url.clone(),
                recommendation: "Add JSON-LD FAQPage schema with question/answer pairs."
                    .to_string(),
            });
        }

        // AI-AB003: No Q&A format
        let has_qa = ctx.page.headings.iter().any(|h| {
            h.text.starts_with("What ") || h.text.starts_with("How ")
                || h.text.starts_with("Why ") || h.text.starts_with("When ")
                || h.text.starts_with("Where ") || h.text.starts_with("Who ")
                || h.text.contains("?")
        });
        if !has_qa && ctx.page.word_count > 1000 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Content,
                code: "AI-AB003".to_string(),
                title: "No question/answer format detected".to_string(),
                description: "Long-form content without Q&A structure. AI engines \
                    extract answers from question-formatted content."
                    .to_string(),
                url: url.clone(),
                recommendation: "Consider restructuring key sections as Q&A pairs \
                    or adding FAQ section."
                    .to_string(),
            });
        }

        // AI-AB004: Answer not upfront
        let first_h1 = ctx.page.headings.iter().find(|h| h.level == 1);
        if let Some(h1) = first_h1 {
            let h1_position = ctx.page.visible_text.find(&h1.text).unwrap_or(0);
            let first_paragraph_start = ctx.page.visible_text.find("\n\n").unwrap_or(0);
            let words_before = ctx.page.visible_text[..first_paragraph_start.min(h1_position)]
                .split_whitespace()
                .count();
            if words_before > 50 {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Content,
                    code: "AI-AB004".to_string(),
                    title: "Answer not upfront".to_string(),
                    description: format!(
                        "First {} words before main content. AI engines prefer \
                        answers appearing early in the content.",
                        words_before
                    ),
                    url: url.clone(),
                    recommendation: "Move key answer/summary to first 1-2 sentences."
                        .to_string(),
                });
            }
        }

        // AI-AB007: Missing speakable schema
        let has_speakable = ctx.page.structured_data.iter().any(|sd| {
            sd.contains_key("speakable")
        });
        if !has_speakable {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Seo,
                code: "AI-AB007".to_string(),
                title: "Missing speakable schema".to_string(),
                description: "No speakable property in structured data. speakable marks \
                    content for voice AI assistants (Siri, Alexa, Google Assistant)."
                    .to_string(),
                url: url.clone(),
                recommendation: "Add speakable property to JSON-LD schema for \
                    voice-friendly content."
                    .to_string(),
            });
        }

        findings
    }
}
```

---

## Integration Points

### Storage Schema Extensions

```sql
-- New table for AI-specific findings
CREATE TABLE IF NOT EXISTS ai_findings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    page_id INTEGER NOT NULL REFERENCES pages(id),
    signal_id TEXT NOT NULL,
    severity TEXT NOT NULL,
    description TEXT NOT NULL,
    recommendation TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(page_id, signal_id)
);

-- Index for fast queries
CREATE INDEX IF NOT EXISTS idx_ai_findings_page ON ai_findings(page_id);
CREATE INDEX IF NOT EXISTS idx_ai_findings_signal ON ai_findings(signal_id);
```

### Export Extensions

Add AI-specific columns to CSV/JSON export:

```toml
[export.ai_signals]
include = true
columns = [
    "ai_crawler_accessibility_score",  # 0-100
    "ai_content_structure_score",      # 0-100
    "ai_citation_eligibility_score",   # 0-100
    "ai_answer_box_readiness_score",   # 0-100
    "ai_overall_score",                # 0-100
    "blocked_ai_bots",                 # List of blocked bots
]
```

### Report Extensions

HTML report gets new section:

```html
<div class="ai-optimization-section">
  <h2>AI Search Optimization</h2>
  <div class="score-card">
    <div class="metric">
      <span class="label">AI Accessibility</span>
      <span class="value" data-score="ai_crawler_accessibility_score">85/100</span>
    </div>
    <!-- ... other metrics ... -->
  </div>
  <div class="findings">
    <!-- AI-specific findings listed here -->
  </div>
</div>
```

---

## Test Vectors

| ID | Input | Expected Findings |
|----|-------|-------------------|
| AI-TV-001 | HTML with `robots.txt` blocking GPTBot | AI-ACC001 |
| AI-TV-002 | HTML with FAQPage schema | No AI-AB001 |
| AI-TV-003 | HTML with >500 words, no lists/tables | AI-CS001 |
| AI-TV-004 | HTML with canonical, author, date, OG tags | No AI-CIT001, AI-CIT003, AI-CIT007 |
| AI-TV-005 | HTML with question headings but no FAQ schema | AI-AB001 |
| AI-TV-006 | HTML with `speakable` schema | No AI-AB007 |
| AI-TV-007 | HTML blocking GPTBot + Google-Extended + ClaudeBot | AI-ACC001, AI-ACC002, AI-ACC004 |
| AI-TV-008 | Clean HTML with all AI signals optimized | No findings |

---

## File Locations

| File | Path | Purpose |
|------|------|---------|
| `ai_analyzers.rs` | `crates/crawlkit-engine/src/ai_analyzers.rs` | All 4 AI analyzers |
| `ai_bots.rs` | `crates/crawlkit-engine/src/ai_bots.rs` | AI bot registry |
| `ai_test_vectors.toml` | `crates/crawlkit-engine/test_vectors/ai_test_vectors.toml` | Test data |
| `ai_integration_test.rs` | `crates/crawlkit-engine/tests/ai_integration_test.rs` | Integration tests |

---

## Dependencies

| Dependency | Version | Purpose | Status |
|------------|---------|---------|--------|
| `scraper` | 0.18+ | HTML parsing | Already in Cargo.toml |
| `url` | 2.4+ | URL parsing | Already in Cargo.toml |
| `serde` | 1.0+ | Serialization | Already in Cargo.toml |
| `toml` | 0.8+ | Bot registry config | Already in Cargo.toml |

**No new external dependencies required.**

---

## Registration

Add to `AnalyzerRegistry::default()` in `analyzers.rs`:

```rust
Self::with_analyzers(vec![
    // ... existing 23 analyzers ...
    Box::new(AiCrawlerAccessibilityAnalyzer::new()),
    Box::new(AiContentStructureAnalyzer::new()),
    Box::new(AiCitationEligibilityAnalyzer::new()),
    Box::new(AiAnswerBoxAnalyzer::new()),
])
```

---

*Generated: 2026-07-23 | Version: 1.0.0*
