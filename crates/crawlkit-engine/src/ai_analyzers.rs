use crate::ai_bots::AiBotRegistry;
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::storage::{IssueCategory, Severity};
use crate::CrawlConfig;

// ---------------------------------------------------------------------------
// AI Crawler Accessibility Analyzer
// ---------------------------------------------------------------------------

/// Detects whether AI crawlers can access the site via robots.txt.
pub struct AiCrawlerAccessibilityAnalyzer {
    registry: AiBotRegistry,
}

impl AiCrawlerAccessibilityAnalyzer {
    /// Create a new AI crawler accessibility analyzer.
    #[must_use]
    pub fn new() -> Self {
        Self {
            registry: AiBotRegistry::default_registry(),
        }
    }
}

impl Default for AiCrawlerAccessibilityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for AiCrawlerAccessibilityAnalyzer {
    fn name(&self) -> &str {
        "ai-crawler-accessibility"
    }

    fn analyze(&self, ctx: &AnalysisContext, _config: &CrawlConfig) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        // Check for robots.txt content in page metadata or headers
        // For now, we check if the page has any AI-blocking indicators
        let robots_txt = extract_robots_txt(ctx);

        if let Some(txt) = robots_txt {
            for bot in self.registry.bots() {
                if crate::ai_bots::robots_txt_disallows_bot(txt, bot.name) {
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
                        url: url.to_string(),
                        recommendation: format!(
                            "If you want {} to access your site, remove the Disallow rule \
                            for {} in robots.txt.",
                            bot.owner, bot.name
                        ),
                    });
                }
            }
        } else {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Seo,
                code: "AI-ACC009".to_string(),
                title: "No robots.txt found".to_string(),
                description: "Site has no robots.txt file. All crawlers, including AI bots, \
                    have unrestricted access."
                    .to_string(),
                url: url.to_string(),
                recommendation: "Consider adding robots.txt to control AI crawler access."
                    .to_string(),
            });
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// AI Content Structure Analyzer
// ---------------------------------------------------------------------------

/// Detects whether content is structured for AI extraction and comprehension.
pub struct AiContentStructureAnalyzer;

impl AiContentStructureAnalyzer {
    /// Create a new AI content structure analyzer.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for AiContentStructureAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for AiContentStructureAnalyzer {
    fn name(&self) -> &str {
        "ai-content-structure"
    }

    fn analyze(&self, ctx: &AnalysisContext, _config: &CrawlConfig) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        // Content structure analysis uses word_count and heading structure as proxies.
        // Full analysis would require raw text access, which is not available in ParsedPage.

        // AI-CS002: Content without subheadings (check heading count vs word count)
        let heading_count = ctx.page.headings.len();
        if ctx.page.word_count > 500 && heading_count < 3 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Content,
                code: "AI-CS002".to_string(),
                title: "Content lacks subheadings".to_string(),
                description: format!(
                    "Page has {} words but only {} heading(s). \
                    AI struggles to extract key points from dense text.",
                    ctx.page.word_count, heading_count
                ),
                url: url.to_string(),
                recommendation: "Break long paragraphs into sections with H2/H3 subheadings."
                    .to_string(),
            });
        }

        // AI-CS008: Missing date metadata
        // Only flag on content pages — utility pages don't need dates
        let is_content_page = !url.contains("/account")
            && !url.contains("/compare")
            && !url.contains("/wishlist")
            && !url.contains("/cart")
            && !url.contains("/checkout")
            && !url.contains("/login")
            && !url.contains("/register")
            && !url.contains("/forgot")
            && !url.ends_with("/");

        let has_date_info = ctx
            .page
            .structured_data
            .iter()
            .any(|sd| sd.data.get("datePublished").is_some());

        if !has_date_info && ctx.page.word_count > 300 && is_content_page {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Content,
                code: "AI-CS008".to_string(),
                title: "Missing date metadata".to_string(),
                description: "Content lacks date information. AI search engines \
                    prioritize fresh, dated content."
                    .to_string(),
                url: url.to_string(),
                recommendation: "Add publication date in <time> tag or JSON-LD schema.".to_string(),
            });
        }

        // AI-CS009: Missing author attribution
        // Only flag on content pages — utility pages don't need authors
        let has_author_info = ctx
            .page
            .structured_data
            .iter()
            .any(|sd| sd.data.get("author").is_some());

        if !has_author_info && ctx.page.word_count > 500 && is_content_page {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Content,
                code: "AI-CS009".to_string(),
                title: "Missing author attribution".to_string(),
                description: "Long-form content lacks author attribution. AI engines \
                    cite authoritative, attributed sources."
                    .to_string(),
                url: url.to_string(),
                recommendation: "Add author name in JSON-LD schema or visible byline.".to_string(),
            });
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// AI Citation Eligibility Analyzer
// ---------------------------------------------------------------------------

/// Detects signals that make content citable by AI search engines.
pub struct AiCitationEligibilityAnalyzer;

impl AiCitationEligibilityAnalyzer {
    /// Create a new AI citation eligibility analyzer.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for AiCitationEligibilityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for AiCitationEligibilityAnalyzer {
    fn name(&self) -> &str {
        "ai-citation-eligibility"
    }

    fn analyze(&self, ctx: &AnalysisContext, _config: &CrawlConfig) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        // AI-CIT001: Missing canonical
        if ctx.page.meta.canonical.is_none() {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Seo,
                code: "AI-CIT001".to_string(),
                title: "Missing canonical URL".to_string(),
                description: "No <link rel=\"canonical\"> found. AI search engines cannot \
                    determine the canonical version of this page."
                    .to_string(),
                url: url.to_string(),
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
                url: url.to_string(),
                recommendation: "Add JSON-LD structured data with appropriate Schema.org type."
                    .to_string(),
            });
        }

        // AI-CIT007: Missing OpenGraph tags
        // Check if meta tags have OG data
        let has_og = ctx.page.meta.title.is_some() || ctx.page.meta.description.is_some();
        if !has_og {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Social,
                code: "AI-CIT007".to_string(),
                title: "Missing OpenGraph tags".to_string(),
                description: "No OpenGraph tags found. OpenGraph tags signal content \
                    legitimacy to AI engines."
                    .to_string(),
                url: url.to_string(),
                recommendation: "Add og:title, og:description, and og:image meta tags.".to_string(),
            });
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// AI Answer Box Analyzer
// ---------------------------------------------------------------------------

/// Detects whether content is optimized for AI answer boxes and featured snippets.
pub struct AiAnswerBoxAnalyzer;

impl AiAnswerBoxAnalyzer {
    /// Create a new AI answer box analyzer.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for AiAnswerBoxAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for AiAnswerBoxAnalyzer {
    fn name(&self) -> &str {
        "ai-answer-box"
    }

    fn analyze(&self, ctx: &AnalysisContext, _config: &CrawlConfig) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        // AI-AB001: No FAQ schema
        let has_faq = ctx.page.structured_data.iter().any(|sd| {
            sd.data
                .get("@type")
                .and_then(|v| v.as_str())
                .map(|t| t == "FAQPage")
                .unwrap_or(false)
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
                url: url.to_string(),
                recommendation: "Add JSON-LD FAQPage schema with question/answer pairs."
                    .to_string(),
            });
        }

        // AI-AB003: No Q&A format
        let has_qa = ctx.page.headings.iter().any(|h| {
            h.text.starts_with("What ")
                || h.text.starts_with("How ")
                || h.text.starts_with("Why ")
                || h.text.starts_with("When ")
                || h.text.starts_with("Where ")
                || h.text.starts_with("Who ")
                || h.text.contains('?')
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
                url: url.to_string(),
                recommendation: "Consider restructuring key sections as Q&A pairs \
                    or adding FAQ section."
                    .to_string(),
            });
        }

        // AI-AB007: Missing speakable schema
        let has_speakable = ctx
            .page
            .structured_data
            .iter()
            .any(|sd| sd.data.get("speakable").is_some());

        if !has_speakable {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Seo,
                code: "AI-AB007".to_string(),
                title: "Missing speakable schema".to_string(),
                description: "No speakable property in structured data. speakable marks \
                    content for voice AI assistants (Siri, Alexa, Google Assistant)."
                    .to_string(),
                url: url.to_string(),
                recommendation: "Add speakable property to JSON-LD schema for \
                    voice-friendly content."
                    .to_string(),
            });
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

fn extract_robots_txt<'a>(ctx: &AnalysisContext<'a>) -> Option<&'a str> {
    // Read pre-fetched robots.txt from AnalysisContext.
    // The crawl loop fetches robots.txt once per domain and stores it here.
    ctx.robots_txt
}

// AiBot::ordinal() is defined in ai_bots.rs

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meta::MetaTags;
    use crate::parser::{Heading, ParsedPage};
    use std::time::Duration;

    fn make_page(url: &str) -> ParsedPage {
        ParsedPage {
            url: url.to_string(),
            meta: MetaTags::default(),
            headings: Vec::new(),
            links: Vec::new(),
            images: Vec::new(),
            forms: Vec::new(),
            scripts: Vec::new(),
            styles: Vec::new(),
            structured_data: Vec::new(),
            word_count: 0,
            landmarks: Vec::new(),
            has_skip_link: false,
            has_main_landmark: false,
            has_nav_landmark: false,
            has_positive_tabindex: false,
            tabindex_negative_count: 0,
            aria_role_count: 0,
            aria_label_count: 0,
            has_lang_attribute: false,
            html_lang: None,
            has_aria_hidden: false,
            tables_with_headers: 0,
            tables_total: 0,
            tables_with_captions: 0,
            og_image_width: None,
            og_image_height: None,
        }
    }

    fn default_config() -> CrawlConfig {
        CrawlConfig::default()
    }

    #[test]
    fn test_ai_crawler_accessibility_analyzer() {
        let analyzer = AiCrawlerAccessibilityAnalyzer::new();
        let page = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &page,
            body: None,
            status_code: Some(200),
            headers: &[],
            response_time: Some(Duration::from_millis(100)),
            redirect_chain: &[],
            robots_txt: None,
        };

        let findings = analyzer.analyze(&ctx, &default_config());
        // Should find "no robots.txt" since we don't have one
        assert!(findings.iter().any(|f| f.code == "AI-ACC009"));
    }

    #[test]
    fn test_ai_citation_eligibility_missing_canonical() {
        let analyzer = AiCitationEligibilityAnalyzer::new();
        let page = make_page("https://example.com");

        let ctx = AnalysisContext {
            page: &page,
            body: None,
            status_code: Some(200),
            headers: &[],
            response_time: Some(Duration::from_millis(100)),
            redirect_chain: &[],
            robots_txt: None,
        };

        let findings = analyzer.analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "AI-CIT001"));
    }

    #[test]
    fn test_ai_answer_box_missing_faq() {
        let analyzer = AiAnswerBoxAnalyzer::new();
        let mut page = make_page("https://example.com");
        page.headings = vec![Heading {
            level: 2,
            text: "What is crawlkit?".to_string(),
            length: 18,
        }];
        page.word_count = 1500;

        let ctx = AnalysisContext {
            page: &page,
            body: None,
            status_code: Some(200),
            headers: &[],
            response_time: Some(Duration::from_millis(100)),
            redirect_chain: &[],
            robots_txt: None,
        };

        let findings = analyzer.analyze(&ctx, &default_config());
        assert!(findings.iter().any(|f| f.code == "AI-AB001"));
    }

    #[test]
    fn test_ai_bot_registry() {
        let registry = AiBotRegistry::default_registry();
        assert!(registry.find("GPTBot").is_some());
        assert!(registry.find("Google-Extended").is_some());
        assert!(registry.find("ClaudeBot").is_some());
        assert!(registry.find("nonexistent").is_none());
    }
}
