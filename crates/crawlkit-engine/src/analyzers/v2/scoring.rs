#![allow(
    clippy::unwrap_used,
    clippy::manual_range_contains,
    clippy::redundant_closure,
    clippy::collapsible_if,
    clippy::unnecessary_map_or,
    clippy::default_constructed_unit_structs,
    clippy::needless_return,
    clippy::needless_range_loop,
    clippy::useless_format,
    clippy::if_same_then_else,
    clippy::derivable_impls,
    clippy::manual_pattern_char_comparison,
    clippy::manual_contains,
    clippy::collapsible_match,
    clippy::redundant_clone,
    clippy::useless_conversion
)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct ContentFreshnessScoreAnalyzer;
impl Default for ContentFreshnessScoreAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
impl ContentFreshnessScoreAnalyzer {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for ContentFreshnessScoreAnalyzer {
    fn name(&self) -> &str {
        "content-freshness-score"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let mut date_found = false;
        for sd in &ctx.page.structured_data {
            if let Some(dp) = sd.data.get("datePublished").and_then(|v| v.as_str()) {
                if !dp.is_empty() {
                    date_found = true;
                    if let Ok(parsed) =
                        chrono::NaiveDate::parse_from_str(&dp[..dp.len().min(10)], "%Y-%m-%d")
                    {
                        let today = chrono::NaiveDate::from_ymd_opt(2026, 8, 30).unwrap_or(parsed);
                        let age = (today - parsed).num_days();
                        if age > 365 {
                            findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Content, code: "FRESHSC002".to_string(), title: "Content is over a year old".to_string(), description: format!("Content date is {age} days old. Outdated content may rank lower."), url: url.clone(), recommendation: "Update the content and refresh the date.".to_string() });
                        } else if age > 180 {
                            findings.push(Finding {
                                severity: Severity::Info,
                                category: IssueCategory::Content,
                                code: "FRESHSC003".to_string(),
                                title: "Content is over 6 months old".to_string(),
                                description: format!(
                                    "Content date is {age} days old. Consider refreshing soon."
                                ),
                                url: url.clone(),
                                recommendation: "Review and update the content.".to_string(),
                            });
                        }
                    }
                }
            }
        }
        if !date_found {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Content,
                code: "FRESHSC001".to_string(),
                title: "No date metadata found".to_string(),
                description: "No datePublished found in structured data.".to_string(),
                url: url.clone(),
                recommendation: "Add datePublished to structured data.".to_string(),
            });
        }
        findings
    }
}

pub struct HeadingStructureScoreAnalyzer;
impl Default for HeadingStructureScoreAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
impl HeadingStructureScoreAnalyzer {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for HeadingStructureScoreAnalyzer {
    fn name(&self) -> &str {
        "heading-structure-score"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.headings.is_empty() {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Content,
                code: "HEADSC001".to_string(),
                title: "No headings found".to_string(),
                description: "The page has no heading elements.".to_string(),
                url: url.clone(),
                recommendation: "Add at least one H1 and hierarchical H2-H6 headings.".to_string(),
            });
            return findings;
        }
        let h1_count = ctx.page.headings.iter().filter(|h| h.level == 1).count();
        if h1_count == 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Content,
                code: "HEADSC002".to_string(),
                title: "Missing H1 heading".to_string(),
                description: "No H1 heading found.".to_string(),
                url: url.clone(),
                recommendation: "Add a single H1 heading.".to_string(),
            });
        } else if h1_count > 1 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Content,
                code: "HEADSC003".to_string(),
                title: "Multiple H1 headings".to_string(),
                description: format!("{h1_count} H1 headings found. Only one is recommended."),
                url: url.clone(),
                recommendation: "Use a single H1 heading per page.".to_string(),
            });
        }
        let levels: Vec<(u32, usize)> = (1u32..=6)
            .map(|l| {
                (
                    l,
                    ctx.page
                        .headings
                        .iter()
                        .filter(|h| h.level as u32 == l)
                        .count(),
                )
            })
            .filter(|(_, c)| *c > 0)
            .collect();
        for w in levels.windows(2) {
            if w[1].0 > w[0].0 + 1 {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Content,
                    code: "HEADSC004".to_string(),
                    title: "Heading level skipped".to_string(),
                    description: format!("Heading jumps from H{} to H{}.", w[0].0, w[1].0),
                    url: url.clone(),
                    recommendation: "Use heading levels sequentially.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct LinkQualityScoreAnalyzer;
impl Default for LinkQualityScoreAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
impl LinkQualityScoreAnalyzer {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for LinkQualityScoreAnalyzer {
    fn name(&self) -> &str {
        "link-quality-score"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.links.is_empty() {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Content,
                code: "LINKSC001".to_string(),
                title: "No links on page".to_string(),
                description: "The page contains no links.".to_string(),
                url: url.clone(),
                recommendation: "Add relevant internal and external links.".to_string(),
            });
            return findings;
        }
        let total = ctx.page.links.len();
        let nofollow = ctx
            .page
            .links
            .iter()
            .filter(|l| l.rel.iter().any(|r| r == "nofollow"))
            .count();
        if nofollow > 0 && nofollow as f64 / total as f64 > 0.5 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Content,
                code: "LINKSC002".to_string(),
                title: "High nofollow link ratio".to_string(),
                description: format!("{nofollow}/{total} links are nofollowed."),
                url: url.clone(),
                recommendation: "Review nofollow usage on internal links.".to_string(),
            });
        }
        let empty_text = ctx
            .page
            .links
            .iter()
            .filter(|l| l.text.trim().is_empty() && l.aria_label.is_none())
            .count();
        if empty_text > 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Content,
                code: "LINKSC003".to_string(),
                title: "Links without anchor text".to_string(),
                description: format!("{empty_text} link(s) have no text or aria-label."),
                url: url.clone(),
                recommendation: "Add descriptive text to all links.".to_string(),
            });
        }
        findings
    }
}

pub struct SchemaCoverageScoreAnalyzer;
impl Default for SchemaCoverageScoreAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
impl SchemaCoverageScoreAnalyzer {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for SchemaCoverageScoreAnalyzer {
    fn name(&self) -> &str {
        "schema-coverage-score"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.structured_data.is_empty() {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Schema,
                code: "SCHEMACOV001".to_string(),
                title: "No structured data present".to_string(),
                description: "No JSON-LD structured data found.".to_string(),
                url: url.clone(),
                recommendation: "Add Schema.org JSON-LD markup.".to_string(),
            });
            return findings;
        }
        let invalid = ctx
            .page
            .structured_data
            .iter()
            .filter(|sd| {
                sd.context.as_deref() != Some("https://schema.org")
                    && sd.context.as_deref() != Some("schema.org")
            })
            .count();
        if invalid > 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Schema,
                code: "SCHEMACOV002".to_string(),
                title: "Non-standard @context".to_string(),
                description: format!("{invalid} structured data blocks use non-standard @context."),
                url: url.clone(),
                recommendation: "Use https://schema.org as @context.".to_string(),
            });
        }
        findings
    }
}

pub struct SecurityScoreAnalyzer;
impl Default for SecurityScoreAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
impl SecurityScoreAnalyzer {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for SecurityScoreAnalyzer {
    fn name(&self) -> &str {
        "security-score"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let mut score: u32 = 100;
        let mut issues: Vec<String> = Vec::new();
        if !ctx
            .headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("Content-Security-Policy"))
        {
            score = score.saturating_sub(15);
            issues.push("no CSP".to_string());
        }
        if !ctx
            .headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("Strict-Transport-Security"))
        {
            score = score.saturating_sub(15);
            issues.push("no HSTS".to_string());
        }
        if !ctx
            .headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("X-Content-Type-Options"))
        {
            score = score.saturating_sub(10);
            issues.push("no XCTO".to_string());
        }
        if !ctx
            .headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("X-Frame-Options"))
            && !ctx.headers.iter().any(|(k, v)| {
                k.eq_ignore_ascii_case("Content-Security-Policy") && v.contains("frame-ancestors")
            })
        {
            score = score.saturating_sub(10);
            issues.push("no XFO".to_string());
        }
        if !ctx
            .headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("Referrer-Policy"))
        {
            score = score.saturating_sub(5);
            issues.push("no RP".to_string());
        }
        if !ctx.headers.iter().any(|(k, _)| {
            k.eq_ignore_ascii_case("Permissions-Policy") || k.eq_ignore_ascii_case("Feature-Policy")
        }) {
            score = score.saturating_sub(5);
            issues.push("no PP".to_string());
        }
        let rec = if score < 50 {
            "Critical security headers missing.".to_string()
        } else if score < 80 {
            "Several security headers missing.".to_string()
        } else {
            "Good security posture.".to_string()
        };
        findings.push(Finding {
            severity: Severity::Info,
            category: IssueCategory::Security,
            code: "SECSC001".to_string(),
            title: "Security header score".to_string(),
            description: format!("Score: {score}/100. Issues: {}.", issues.join(", ")),
            url: url.clone(),
            recommendation: rec,
        });
        findings
    }
}

pub struct AccessibilityScoreAnalyzer;
impl Default for AccessibilityScoreAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
impl AccessibilityScoreAnalyzer {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for AccessibilityScoreAnalyzer {
    fn name(&self) -> &str {
        "accessibility-score"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let mut score: u32 = 100;
        let mut issues: Vec<String> = Vec::new();
        if !ctx.page.has_lang_attribute {
            score = score.saturating_sub(10);
            issues.push("no lang".to_string());
        }
        if !ctx.page.has_skip_link {
            score = score.saturating_sub(5);
            issues.push("no skip link".to_string());
        }
        if !ctx.page.has_main_landmark {
            score = score.saturating_sub(10);
            issues.push("no main".to_string());
        }
        if ctx.page.tables_total > 0 && ctx.page.tables_with_headers == 0 {
            score = score.saturating_sub(10);
            issues.push("tables no headers".to_string());
        }
        let no_alt = ctx
            .page
            .images
            .iter()
            .filter(|i| i.alt.is_empty() && !i.has_alt)
            .count();
        if no_alt > 0 {
            score = score.saturating_sub((no_alt as u32 * 2).min(15));
            issues.push(format!("{no_alt} images no alt"));
        }
        if ctx.page.has_positive_tabindex {
            score = score.saturating_sub(10);
            issues.push("positive tabindex".to_string());
        }
        let rec = if score < 50 {
            "Significant accessibility issues.".to_string()
        } else {
            "Good accessibility.".to_string()
        };
        findings.push(Finding {
            severity: Severity::Info,
            category: IssueCategory::Accessibility,
            code: "A11YSC001".to_string(),
            title: "Accessibility compliance score".to_string(),
            description: format!("Score: {score}/100. Issues: {}.", issues.join(", ")),
            url: url.clone(),
            recommendation: rec,
        });
        findings
    }
}

// =========================================================================
// Security V2 Analyzers
// =========================================================================

pub struct ExternalLinksAuthorityScoreValidator;
impl Default for ExternalLinksAuthorityScoreValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl ExternalLinksAuthorityScoreValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for ExternalLinksAuthorityScoreValidator {
    fn name(&self) -> &str {
        "external-links-authority-score-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let external: Vec<&str> = ctx
            .page
            .links
            .iter()
            .filter(|l| l.is_external)
            .map(|l| l.href.as_str())
            .collect();
        if external.len() > 5 {
            let high_authority = [
                "wikipedia.org",
                "github.com",
                "stackoverflow.com",
                "mozilla.org",
                "w3.org",
                "schema.org",
            ];
            let authority_count = external
                .iter()
                .filter(|href| high_authority.iter().any(|dom| href.contains(dom)))
                .count();
            if authority_count == 0 {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Links,
                    code: "EXTAUTH-V6105".to_string(),
                    title: "No authoritative external links".to_string(),
                    description: format!(
                        "{} external links, none to high-authority domains.",
                        external.len()
                    ),
                    url: url.clone(),
                    recommendation: "Link to reputable, authoritative sources.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct ExternalLinksAuthorityScoreDeepValidator;
impl Default for ExternalLinksAuthorityScoreDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl ExternalLinksAuthorityScoreDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for ExternalLinksAuthorityScoreDeepValidator {
    fn name(&self) -> &str {
        "external-links-authority-score-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let external: Vec<&str> = ctx
            .page
            .links
            .iter()
            .filter(|l| l.is_external)
            .map(|l| l.href.as_str())
            .collect();
        if external.len() > 3 {
            let high_authority = [
                "wikipedia.org",
                "github.com",
                "stackoverflow.com",
                "mozilla.org",
                "w3.org",
                "schema.org",
                "google.com",
                "developer.mozilla.org",
            ];
            let low_authority = [
                ".ru", ".cn", ".tk", ".ml", ".xyz", ".top", ".buzz", ".click",
            ];
            let authority_count = external
                .iter()
                .filter(|href| high_authority.iter().any(|dom| href.contains(dom)))
                .count();
            let suspicious_count = external
                .iter()
                .filter(|href| low_authority.iter().any(|tld| href.contains(tld)))
                .count();
            if authority_count == 0 && external.len() > 5 {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Seo,
                    code: "EXTAUTHDP001".to_string(),
                    title: "No high-authority external links".to_string(),
                    description: format!(
                        "{} external links, none to authoritative domains.",
                        external.len()
                    ),
                    url: url.clone(),
                    recommendation: "Link to reputable, authoritative sources.".to_string(),
                });
            }
            if suspicious_count > 0 {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "EXTAUTHDP002".to_string(),
                    title: "Links to low-reputation TLDs".to_string(),
                    description: format!(
                        "{}/{} external links point to suspicious TLDs.",
                        suspicious_count,
                        external.len()
                    ),
                    url: url.clone(),
                    recommendation: "Review external links to low-reputation TLDs.".to_string(),
                });
            }
            let ratio = authority_count as f64 / external.len() as f64;
            if external.len() > 10 && ratio < 0.1 {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Seo,
                    code: "EXTAUTHDP003".to_string(),
                    title: "Low authority link ratio".to_string(),
                    description: format!(
                        "{}/{} ({:.0}%) external links are to authoritative sources.",
                        authority_count,
                        external.len(),
                        ratio * 100.0
                    ),
                    url: url.clone(),
                    recommendation: "Increase the proportion of links to authoritative domains."
                        .to_string(),
                });
            }
        }
        findings
    }
}

// =========================================================================
// V8 Content Validators (30)
// =========================================================================

pub struct ExternalLinksAuthorityScoreDeepDeepValidator;
impl Default for ExternalLinksAuthorityScoreDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl ExternalLinksAuthorityScoreDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for ExternalLinksAuthorityScoreDeepDeepValidator {
    fn name(&self) -> &str {
        "external-links-authority-score-deep-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let external: Vec<&str> = ctx
            .page
            .links
            .iter()
            .filter(|l| l.is_external)
            .map(|l| l.href.as_str())
            .collect();
        if external.len() > 3 {
            let high_authority = [
                "wikipedia.org",
                "github.com",
                "stackoverflow.com",
                "mozilla.org",
                "w3.org",
                "schema.org",
                "google.com",
            ];
            let authority_count = external
                .iter()
                .filter(|href| high_authority.iter().any(|dom| href.contains(dom)))
                .count();
            let ratio = authority_count as f64 / external.len() as f64;
            if ratio < 0.1 {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Seo, code: "EXTAUTHDP-V2001".to_string(), title: "Low authority link ratio (deep-deep)".to_string(), description: format!("{}/{} ({:.0}%) external links are to authoritative sources in deep analysis.", authority_count, external.len(), ratio * 100.0), url: url.clone(), recommendation: "Increase links to authoritative domains.".to_string() });
            }
        }
        findings
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::meta::MetaTags;
    use crate::parser::{Heading, StructuredData};

    fn make_page(url: &str) -> crate::parser::ParsedPage {
        crate::parser::ParsedPage {
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
            sentence_count: 0,
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

    fn make_ctx<'a>(
        page: &'a crate::parser::ParsedPage,
        body: Option<&'a str>,
    ) -> AnalysisContext<'a> {
        AnalysisContext {
            page,
            body,
            status_code: Some(200),
            headers: &[],
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        }
    }

    #[test]
    fn test_freshness_score_no_data() {
        let p = make_page("https://example.com");
        assert!(!ContentFreshnessScoreAnalyzer::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_freshness_score_with_date() {
        let mut p = make_page("https://example.com");
        p.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({"datePublished": "2020-01-01"}),
        }];
        let ctx = make_ctx(&p, None);
        let f = ContentFreshnessScoreAnalyzer::new().analyze(&ctx);
        assert!(f.iter().any(|x| x.code == "FRESHSC002"));
    }
    #[test]
    fn test_heading_structure_no_headings() {
        let p = make_page("https://example.com");
        assert!(!HeadingStructureScoreAnalyzer::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_heading_structure_with_h1() {
        let mut p = make_page("https://example.com");
        p.headings = vec![Heading {
            level: 1,
            text: "Title".to_string(),
            length: 5,
        }];
        assert!(HeadingStructureScoreAnalyzer::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_link_quality_empty() {
        let p = make_page("https://example.com");
        assert!(!LinkQualityScoreAnalyzer::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_schema_coverage_empty() {
        let p = make_page("https://example.com");
        assert!(!SchemaCoverageScoreAnalyzer::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_security_score() {
        let p = make_page("https://example.com");
        let f = SecurityScoreAnalyzer::new().analyze(&make_ctx(&p, None));
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].code, "SECSC001");
    }
    #[test]
    fn test_accessibility_score() {
        let p = make_page("https://example.com");
        let f = AccessibilityScoreAnalyzer::new().analyze(&make_ctx(&p, None));
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].code, "A11YSC001");
    }
    #[test]
    fn test_ext_links_auth() {
        let p = make_page("https://example.com");
        assert!(ExternalLinksAuthorityScoreValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_ext_auth_no_links() {
        let p = make_page("https://example.com");
        assert!(ExternalLinksAuthorityScoreDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_ext_auth_few_links() {
        let mut p = make_page("https://example.com");
        p.links = vec![crate::parser::ExtractedLink {
            href: "https://other.com/page".into(),
            text: "Link".into(),
            rel: vec![],
            is_external: true,
            aria_label: None,
            img_alt: None,
        }];
        assert!(ExternalLinksAuthorityScoreDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_ext_auth_with_authority() {
        let mut p = make_page("https://example.com");
        p.links = vec![crate::parser::ExtractedLink {
            href: "https://en.wikipedia.org/wiki/Example".into(),
            text: "Wiki".into(),
            rel: vec![],
            is_external: true,
            aria_label: None,
            img_alt: None,
        }];
        assert!(ExternalLinksAuthorityScoreDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_ext_auth_no_authority() {
        let mut p = make_page("https://example.com");
        p.links = (0..10)
            .map(|i| crate::parser::ExtractedLink {
                href: format!("https://random{i}.com/page"),
                text: format!("Link {i}"),
                rel: vec![],
                is_external: true,
                aria_label: None,
                img_alt: None,
            })
            .collect();
        let f = ExternalLinksAuthorityScoreDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(f.iter().any(|x| x.code == "EXTAUTHDP001"));
    }
    #[test]
    fn test_ext_auth_suspicious_tld() {
        let mut p = make_page("https://example.com");
        p.links = vec![
            crate::parser::ExtractedLink {
                href: "https://malicious.ru/page".into(),
                text: "Link".into(),
                rel: vec![],
                is_external: true,
                aria_label: None,
                img_alt: None,
            },
            crate::parser::ExtractedLink {
                href: "https://another.ru/page".into(),
                text: "Link2".into(),
                rel: vec![],
                is_external: true,
                aria_label: None,
                img_alt: None,
            },
            crate::parser::ExtractedLink {
                href: "https://third.xyz/page".into(),
                text: "Link3".into(),
                rel: vec![],
                is_external: true,
                aria_label: None,
                img_alt: None,
            },
            crate::parser::ExtractedLink {
                href: "https://fourth.tk/page".into(),
                text: "Link4".into(),
                rel: vec![],
                is_external: true,
                aria_label: None,
                img_alt: None,
            },
        ];
        let f = ExternalLinksAuthorityScoreDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_ext_auth_low_ratio() {
        let mut p = make_page("https://example.com");
        let mut links: Vec<crate::parser::ExtractedLink> = (0..15)
            .map(|i| crate::parser::ExtractedLink {
                href: format!("https://random{i}.com/page"),
                text: format!("Link {i}"),
                rel: vec![],
                is_external: true,
                aria_label: None,
                img_alt: None,
            })
            .collect();
        links.push(crate::parser::ExtractedLink {
            href: "https://en.wikipedia.org/wiki/Example".into(),
            text: "Wiki".into(),
            rel: vec![],
            is_external: true,
            aria_label: None,
            img_alt: None,
        });
        p.links = links;
        let f = ExternalLinksAuthorityScoreDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(f.iter().any(|x| x.code == "EXTAUTHDP003"));
    }
    #[test]
    fn test_ext_auth_internal_only() {
        let mut p = make_page("https://example.com");
        p.links = (0..10)
            .map(|i| crate::parser::ExtractedLink {
                href: format!("https://example.com/page{i}"),
                text: format!("Link {i}"),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            })
            .collect();
        assert!(ExternalLinksAuthorityScoreDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_ext_auth_empty_links() {
        let mut p = make_page("https://example.com");
        p.links = vec![];
        assert!(ExternalLinksAuthorityScoreDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_ext_auth_mixed_suspicious() {
        let mut p = make_page("https://example.com");
        p.links = (0..5)
            .map(|i| crate::parser::ExtractedLink {
                href: format!("https://link{i}.xyz/page"),
                text: format!("Link {i}"),
                rel: vec![],
                is_external: true,
                aria_label: None,
                img_alt: None,
            })
            .collect();
        let f = ExternalLinksAuthorityScoreDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(f.iter().any(|x| x.code == "EXTAUTHDP002"));
    }
    #[test]
    fn test_ext_auth_high_authority_domains() {
        let mut p = make_page("https://example.com");
        let domains = [
            "wikipedia.org",
            "github.com",
            "stackoverflow.com",
            "mozilla.org",
            "w3.org",
            "schema.org",
        ];
        p.links = domains
            .iter()
            .map(|d| crate::parser::ExtractedLink {
                href: format!("https://{d}/page"),
                text: d.to_string(),
                rel: vec![],
                is_external: true,
                aria_label: None,
                img_alt: None,
            })
            .collect();
        assert!(ExternalLinksAuthorityScoreDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }

    // ===== V8 Content Validators (30) tests =====
}
