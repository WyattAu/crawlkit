use crate::analyzers::post_crawl_analyzers::CrawlData;
use crate::storage::{PageData, Severity};
use crate::Finding;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Priority level for an insight, derived from the combined impact score.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum InsightPriority {
    Low,
    Medium,
    High,
    Critical,
}

impl InsightPriority {
    pub fn as_str(&self) -> &'static str {
        match self {
            InsightPriority::Critical => "critical",
            InsightPriority::High => "high",
            InsightPriority::Medium => "medium",
            InsightPriority::Low => "low",
        }
    }
}

/// Estimated effort required to fix an issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum InsightEffort {
    Quick,
    Moderate,
    Significant,
}

impl InsightEffort {
    pub fn as_str(&self) -> &'static str {
        match self {
            InsightEffort::Quick => "quick",
            InsightEffort::Moderate => "moderate",
            InsightEffort::Significant => "significant",
        }
    }
}

/// Category of an insight, mapping to the broader domain it affects.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InsightCategory {
    Technical,
    Content,
    SEO,
    Security,
    Performance,
}

impl InsightCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            InsightCategory::Technical => "technical",
            InsightCategory::Content => "content",
            InsightCategory::SEO => "seo",
            InsightCategory::Security => "security",
            InsightCategory::Performance => "performance",
        }
    }
}

/// A prioritized insight derived from crawl findings.
///
/// Aggregates multiple findings of the same code into a single actionable
/// recommendation ranked by impact and effort.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Insight {
    /// Short human-readable title.
    pub title: String,
    /// Detailed description of the issue.
    pub description: String,
    /// Priority level derived from impact score.
    pub priority: InsightPriority,
    /// Numeric impact score (0-100). Higher = more impactful.
    pub impact_score: f64,
    /// Estimated effort to fix.
    pub effort: InsightEffort,
    /// Number of pages affected by this issue.
    pub affected_pages: usize,
    /// Which finding codes contributed to this insight.
    pub finding_codes: Vec<String>,
    /// Actionable recommendation.
    pub recommendation: String,
    /// Broad category of the insight.
    pub category: InsightCategory,
}

/// Map a finding code prefix to an `InsightCategory`.
fn category_for_code(code: &str) -> InsightCategory {
    let upper = code.to_uppercase();
    if upper.starts_with("HTTP") || upper.starts_with("REDIR") || upper.starts_with("QUALITY") {
        InsightCategory::Technical
    } else if upper.starts_with("SEC") || upper.starts_with("CSP") || upper.starts_with("HSTS") {
        InsightCategory::Security
    } else if upper.starts_with("PERF")
        || upper.starts_with("TTFB")
        || upper.starts_with("RESP")
        || upper.starts_with("CACHE")
    {
        InsightCategory::Performance
    } else if upper.starts_with("CONTENT")
        || upper.starts_with("WORD")
        || upper.starts_with("READ")
        || upper.starts_with("FRESH")
        || upper.starts_with("DUP")
    {
        InsightCategory::Content
    } else {
        InsightCategory::SEO
    }
}

/// Map a finding code prefix to an `InsightEffort`.
pub fn estimate_effort(code: &str) -> InsightEffort {
    let upper = code.to_uppercase();
    // Quick: header/config changes (HTTP status, SSL, security headers, mobile)
    if upper.starts_with("HTTP")
        || upper.starts_with("SSL")
        || upper.starts_with("SEC")
        || upper.starts_with("MOB")
        || upper.starts_with("CSP")
        || upper.starts_with("HSTS")
        || upper.starts_with("REDIR")
        || upper.starts_with("QUALITY")
    {
        return InsightEffort::Quick;
    }
    // Moderate: content/meta changes (SEO, meta, canonical, hreflang, sitemap)
    if upper.starts_with("SEO")
        || upper.starts_with("META")
        || upper.starts_with("CANON")
        || upper.starts_with("HREF")
        || upper.starts_with("SITEMAP")
        || upper.starts_with("HEAD")
        || upper.starts_with("ROBOT")
        || upper.starts_with("LINK")
        || upper.starts_with("GRAPH")
        || upper.starts_with("ORPHAN")
        || upper.starts_with("CANNIB")
        || upper.starts_with("KEY-C")
        || upper.starts_with("COVERAGE")
        || upper.starts_with("HEALTH")
        || upper.starts_with("CANON-C")
    {
        return InsightEffort::Moderate;
    }
    // Significant: structural changes (content depth, schema, images, structured data)
    if upper.starts_with("CONTENT")
        || upper.starts_with("SCHEMA")
        || upper.starts_with("IMAGE")
        || upper.starts_with("DUP")
        || upper.starts_with("FRESH")
        || upper.starts_with("WORD")
        || upper.starts_with("READ")
    {
        return InsightEffort::Significant;
    }
    InsightEffort::Moderate
}

/// Map severity to a base impact value.
fn severity_base_impact(severity: Severity) -> f64 {
    match severity {
        Severity::Critical => 100.0,
        Severity::Error => 75.0,
        Severity::Warning => 50.0,
        Severity::Info => 25.0,
    }
}

/// Map priority from an impact score.
fn priority_from_score(score: f64) -> InsightPriority {
    if score >= 75.0 {
        InsightPriority::Critical
    } else if score >= 50.0 {
        InsightPriority::High
    } else if score >= 25.0 {
        InsightPriority::Medium
    } else {
        InsightPriority::Low
    }
}

/// Aggregation bucket for findings sharing the same code.
struct FindingGroup {
    code: String,
    title: String,
    description: String,
    recommendation: String,
    severity: Severity,
    affected_urls: Vec<String>,
}

/// Analyzes all crawl findings and produces prioritized insights,
/// ranked by impact (severity × prevalence) with effort estimates.
///
/// Returns at most `max_insights` insights (default 20).
pub fn generate_insights(findings: &[Finding], pages: &[PageData]) -> Vec<Insight> {
    generate_insights_with_limit(findings, pages, 20)
}

/// Like [`generate_insights`] but with a configurable maximum output count.
pub fn generate_insights_with_limit(findings: &[Finding], pages: &[PageData], max_insights: usize) -> Vec<Insight> {
    if findings.is_empty() || pages.is_empty() {
        return Vec::new();
    }

    let total_pages = pages.len() as f64;

    // 1. Group findings by code
    let mut groups: HashMap<String, FindingGroup> = HashMap::new();
    for finding in findings {
        let entry = groups.entry(finding.code.clone()).or_insert_with(|| FindingGroup {
            code: finding.code.clone(),
            title: finding.title.clone(),
            description: finding.description.clone(),
            recommendation: finding.recommendation.clone(),
            severity: finding.severity,
            affected_urls: Vec::new(),
        });
        if !entry.affected_urls.contains(&finding.url) {
            entry.affected_urls.push(finding.url.clone());
        }
    }

    // 2. For each group, compute impact and build insight
    let mut insights: Vec<Insight> = groups
        .values()
        .map(|group| {
            let prevalence = group.affected_urls.len() as f64 / total_pages;
            let base = severity_base_impact(group.severity);
            let impact_score = (base * prevalence).clamp(0.0, 100.0);
            let effort = estimate_effort(&group.code);
            let category = category_for_code(&group.code);

            Insight {
                title: group.title.clone(),
                description: format!(
                    "{} (affects {}/{} pages, {:.0}% prevalence)",
                    group.description,
                    group.affected_urls.len(),
                    pages.len(),
                    prevalence * 100.0,
                ),
                priority: priority_from_score(impact_score),
                impact_score,
                effort,
                affected_pages: group.affected_urls.len(),
                finding_codes: vec![group.code.clone()],
                recommendation: group.recommendation.clone(),
                category,
            }
        })
        .collect();

    // 3. Sort by impact descending, then by effort ascending (quick fixes first)
    insights.sort_by(|a, b| {
        b.impact_score
            .partial_cmp(&a.impact_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.effort.cmp(&b.effort))
    });

    // 4. Truncate to max
    insights.truncate(max_insights);
    insights
}

/// Generate insights from [`CrawlData`] (post-crawl analysis output).
///
/// Converts post-crawl `Issue`s into `Finding`s and delegates to
/// [`generate_insights`].
pub fn generate_insights_from_crawl_data(data: &CrawlData) -> Vec<Insight> {
    let findings: Vec<Finding> = data
        .issues
        .iter()
        .map(|issue| Finding {
            severity: issue.severity,
            category: issue.category.clone(),
            code: issue.code.clone(),
            title: issue.title.clone(),
            description: issue.description.clone(),
            url: format!("page://{}", issue.page_id),
            recommendation: issue.recommendation.clone(),
        })
        .collect();

    generate_insights(&findings, &data.pages)
}

/// Extract the root code prefix (letters only, before the first digit).
#[cfg(test)]
fn code_root(code: &str) -> String {
    let mut root = String::new();
    for ch in code.chars() {
        if ch.is_ascii_alphabetic() {
            root.push(ch);
        } else {
            break;
        }
    }
    root.to_uppercase()
}

#[cfg(test)]
#[allow(clippy::vec_init_then_push)]
mod tests {
    use super::*;
    use crate::storage::{IssueCategory, Issue, PageData, Severity};
    use chrono::Utc;
    use url::Url;

    fn test_page(url: &str) -> PageData {
        PageData {
            id: format!("p-{}", url),
            url: Url::parse(url).unwrap(),
            final_url: Url::parse(url).unwrap(),
            status_code: 200,
            title: Some("Page".to_string()),
            description: None,
            canonical_url: None,
            word_count: Some(100),
            load_time_ms: Some(200),
            body_size: Some(1024),
            fetched_at: Utc::now(),
            links: vec![],
            tenant_id: None,
            etag: None,
            last_modified: None,
            cwv_lcp: None,
            cwv_cls: None,
            cwv_inp: None,
            has_structured_data: None,
            schema_types: None,
            viewport_ok: None,
            has_csp: None,
            has_hsts: None,
            images_total: None,
            images_missing_alt: None,
            h1_count: None,
            heading_count: None,
            extractions: None,
        }
    }

    fn finding(code: &str, severity: Severity, url: &str) -> Finding {
        Finding {
            severity,
            category: IssueCategory::Seo,
            code: code.to_string(),
            title: format!("Issue {code}"),
            description: format!("Description for {code}"),
            url: url.to_string(),
            recommendation: format!("Fix {code}"),
        }
    }

    // ===== InsightPriority ordering =====

    #[test]
    fn test_priority_ordering() {
        assert!(InsightPriority::Critical > InsightPriority::High);
        assert!(InsightPriority::High > InsightPriority::Medium);
        assert!(InsightPriority::Medium > InsightPriority::Low);
    }

    // ===== estimate_effort =====

    #[test]
    fn test_estimate_effort_quick() {
        assert_eq!(estimate_effort("HTTP001"), InsightEffort::Quick);
        assert_eq!(estimate_effort("SSL001"), InsightEffort::Quick);
        assert_eq!(estimate_effort("SEC001"), InsightEffort::Quick);
        assert_eq!(estimate_effort("MOB001"), InsightEffort::Quick);
        assert_eq!(estimate_effort("CSP001"), InsightEffort::Quick);
        assert_eq!(estimate_effort("HSTS001"), InsightEffort::Quick);
        assert_eq!(estimate_effort("REDIR001"), InsightEffort::Quick);
        assert_eq!(estimate_effort("QUALITY001"), InsightEffort::Quick);
    }

    #[test]
    fn test_estimate_effort_moderate() {
        assert_eq!(estimate_effort("SEO001"), InsightEffort::Moderate);
        assert_eq!(estimate_effort("META001"), InsightEffort::Moderate);
        assert_eq!(estimate_effort("CANON001"), InsightEffort::Moderate);
        assert_eq!(estimate_effort("HREF001"), InsightEffort::Moderate);
        assert_eq!(estimate_effort("SITEMAP001"), InsightEffort::Moderate);
        assert_eq!(estimate_effort("HEAD001"), InsightEffort::Moderate);
        assert_eq!(estimate_effort("GRAPH001"), InsightEffort::Moderate);
        assert_eq!(estimate_effort("ORPHAN001"), InsightEffort::Moderate);
        assert_eq!(estimate_effort("CANNIB001"), InsightEffort::Moderate);
    }

    #[test]
    fn test_estimate_effort_significant() {
        assert_eq!(estimate_effort("CONTENT001"), InsightEffort::Significant);
        assert_eq!(estimate_effort("SCHEMA001"), InsightEffort::Significant);
        assert_eq!(estimate_effort("IMAGE001"), InsightEffort::Significant);
        assert_eq!(estimate_effort("DUP-CROSS001"), InsightEffort::Significant);
        assert_eq!(estimate_effort("FRESH-C001"), InsightEffort::Significant);
    }

    #[test]
    fn test_estimate_effort_unknown_defaults_moderate() {
        assert_eq!(estimate_effort("ZZZZ001"), InsightEffort::Moderate);
    }

    // ===== generate_insights =====

    #[test]
    fn test_empty_findings_returns_empty() {
        let pages = vec![test_page("https://example.com")];
        let insights = generate_insights(&[], &pages);
        assert!(insights.is_empty());
    }

    #[test]
    fn test_empty_pages_returns_empty() {
        let findings = vec![finding(
            "SEO001",
            Severity::Error,
            "https://example.com",
        )];
        let insights = generate_insights(&findings, &[]);
        assert!(insights.is_empty());
    }

    #[test]
    fn test_single_finding() {
        let pages = vec![test_page("https://example.com")];
        let findings = vec![finding(
            "SEO001",
            Severity::Error,
            "https://example.com",
        )];
        let insights = generate_insights(&findings, &pages);
        assert_eq!(insights.len(), 1);
        assert_eq!(insights[0].finding_codes, vec!["SEO001"]);
        assert_eq!(insights[0].affected_pages, 1);
    }

    #[test]
    fn test_severity_to_impact_mapping() {
        let pages: Vec<PageData> = (0..10)
            .map(|i| test_page(&format!("https://example.com/p{i}")))
            .collect();

        // Critical on 100% of pages -> 100 * 1.0 = 100
        let findings: Vec<Finding> = pages
            .iter()
            .map(|p| finding("SEC001", Severity::Critical, p.url.as_str()))
            .collect();
        let insights = generate_insights(&findings, &pages);
        assert_eq!(insights[0].impact_score, 100.0);
        assert_eq!(insights[0].priority, InsightPriority::Critical);
    }

    #[test]
    fn test_prevalence_affects_impact() {
        let pages: Vec<PageData> = (0..100)
            .map(|i| test_page(&format!("https://example.com/p{i}")))
            .collect();

        // Error on 10% of pages -> 75 * 0.1 = 7.5
        let findings: Vec<Finding> = (0..10)
            .map(|i| finding("SEO001", Severity::Error, &format!("https://example.com/p{i}")))
            .collect();
        let insights = generate_insights(&findings, &pages);
        assert!((insights[0].impact_score - 7.5).abs() < 0.01);
    }

    #[test]
    fn test_multiple_codes_ranked_by_impact() {
        let pages: Vec<PageData> = (0..10)
            .map(|i| test_page(&format!("https://example.com/p{i}")))
            .collect();

        let mut findings = Vec::new();
        // SEO001: Warning on 5 pages -> 50 * 0.5 = 25
        for i in 0..5 {
            findings.push(finding("SEO001", Severity::Warning, &format!("https://example.com/p{i}")));
        }
        // SEC001: Critical on 1 page -> 100 * 0.1 = 10
        findings.push(finding("SEC001", Severity::Critical, "https://example.com/p0"));

        let insights = generate_insights(&findings, &pages);
        assert_eq!(insights.len(), 2);
        // SEO001 has higher impact (25 > 10)
        assert_eq!(insights[0].finding_codes[0], "SEO001");
        assert_eq!(insights[1].finding_codes[0], "SEC001");
    }

    #[test]
    fn test_effort_sorting_same_impact() {
        let pages: Vec<PageData> = (0..10)
            .map(|i| test_page(&format!("https://example.com/p{i}")))
            .collect();

        let mut findings = Vec::new();
        // Both have same severity (Warning = 50), same prevalence (1/10 = 0.1) -> impact 5.0
        findings.push(finding("CONTENT001", Severity::Warning, "https://example.com/p0")); // Significant
        findings.push(finding("HTTP001", Severity::Warning, "https://example.com/p1")); // Quick

        let insights = generate_insights(&findings, &pages);
        assert_eq!(insights.len(), 2);
        // Quick effort should come first when scores are equal
        assert_eq!(insights[0].effort, InsightEffort::Quick);
        assert_eq!(insights[1].effort, InsightEffort::Significant);
    }

    #[test]
    fn test_unique_urls_counted_once() {
        let pages: Vec<PageData> = (0..5)
            .map(|i| test_page(&format!("https://example.com/p{i}")))
            .collect();

        // Same URL reported multiple times for same code
        let findings = vec![
            finding("SEO001", Severity::Error, "https://example.com/p0"),
            finding("SEO001", Severity::Error, "https://example.com/p0"),
            finding("SEO001", Severity::Error, "https://example.com/p0"),
        ];
        let insights = generate_insights(&findings, &pages);
        assert_eq!(insights.len(), 1);
        assert_eq!(insights[0].affected_pages, 1);
    }

    #[test]
    fn test_insight_category_from_code() {
        let pages = vec![test_page("https://example.com")];

        let findings = vec![finding("HTTP001", Severity::Error, "https://example.com")];
        let insights = generate_insights(&findings, &pages);
        assert_eq!(insights[0].category, InsightCategory::Technical);
    }

    #[test]
    fn test_max_insights_limit() {
        let pages: Vec<PageData> = (0..50)
            .map(|i| test_page(&format!("https://example.com/p{i}")))
            .collect();

        let findings: Vec<Finding> = (0..30)
            .map(|i| {
                finding(
                    &format!("CODE{i:03}"),
                    Severity::Warning,
                    &format!("https://example.com/p{i}"),
                )
            })
            .collect();

        let insights = generate_insights_with_limit(&findings, &pages, 5);
        assert_eq!(insights.len(), 5);
    }

    // ===== Deduplication =====

    #[test]
    fn test_findings_grouped_by_code() {
        let pages: Vec<PageData> = (0..10)
            .map(|i| test_page(&format!("https://example.com/p{i}")))
            .collect();

        let mut findings = Vec::new();
        // Different codes produce separate insights
        for i in 0..5 {
            findings.push(finding("META001", Severity::Warning, &format!("https://example.com/p{i}")));
        }
        for i in 5..8 {
            findings.push(finding("META002", Severity::Error, &format!("https://example.com/p{i}")));
        }

        let insights = generate_insights(&findings, &pages);
        assert_eq!(insights.len(), 2);
        // META001 (Warning, 5/10 prevalence = 25.0) > META002 (Error, 3/10 = 22.5)
        assert!(insights[0].finding_codes.contains(&"META001".to_string()));
        assert!(insights[1].finding_codes.contains(&"META002".to_string()));
    }

    #[test]
    fn test_dedup_different_categories_not_merged() {
        let pages: Vec<PageData> = (0..10)
            .map(|i| test_page(&format!("https://example.com/p{i}")))
            .collect();

        let mut findings = Vec::new();
        // Same prefix but different categories
        let mut f1 = finding("HTTP001", Severity::Warning, "https://example.com/p0");
        f1.category = IssueCategory::Http;
        findings.push(f1);

        let mut f2 = finding("HTTP002", Severity::Warning, "https://example.com/p1");
        f2.category = IssueCategory::Security;
        findings.push(f2);

        let insights = generate_insights(&findings, &pages);
        // Different categories should not be merged
        assert_eq!(insights.len(), 2);
    }

    // ===== Severity mapping =====

    #[test]
    fn test_severity_base_impact_values() {
        assert_eq!(severity_base_impact(Severity::Critical), 100.0);
        assert_eq!(severity_base_impact(Severity::Error), 75.0);
        assert_eq!(severity_base_impact(Severity::Warning), 50.0);
        assert_eq!(severity_base_impact(Severity::Info), 25.0);
    }

    #[test]
    fn test_priority_from_score_boundaries() {
        assert_eq!(priority_from_score(100.0), InsightPriority::Critical);
        assert_eq!(priority_from_score(75.0), InsightPriority::Critical);
        assert_eq!(priority_from_score(74.9), InsightPriority::High);
        assert_eq!(priority_from_score(50.0), InsightPriority::High);
        assert_eq!(priority_from_score(49.9), InsightPriority::Medium);
        assert_eq!(priority_from_score(25.0), InsightPriority::Medium);
        assert_eq!(priority_from_score(24.9), InsightPriority::Low);
        assert_eq!(priority_from_score(0.0), InsightPriority::Low);
    }

    // ===== Edge cases =====

    #[test]
    fn test_all_same_severity() {
        let pages: Vec<PageData> = (0..5)
            .map(|i| test_page(&format!("https://example.com/p{i}")))
            .collect();

        let findings: Vec<Finding> = (0..5)
            .map(|i| finding(&format!("SEO{i:03}"), Severity::Info, &format!("https://example.com/p{i}")))
            .collect();

        let insights = generate_insights(&findings, &pages);
        assert_eq!(insights.len(), 5);
        // All should have Info base (25) * 0.2 prevalence = 5.0
        for insight in &insights {
            assert!((insight.impact_score - 5.0).abs() < 0.01);
            assert_eq!(insight.priority, InsightPriority::Low);
        }
    }

    #[test]
    fn test_all_critical_severity() {
        let pages: Vec<PageData> = (0..10)
            .map(|i| test_page(&format!("https://example.com/p{i}")))
            .collect();

        let findings: Vec<Finding> = (0..10)
            .map(|i| finding(&format!("SEC{i:03}"), Severity::Critical, &format!("https://example.com/p{i}")))
            .collect();

        let insights = generate_insights(&findings, &pages);
        assert_eq!(insights.len(), 10);
        // Each code affects 1/10 pages: 100 * 0.1 = 10.0
        for insight in &insights {
            assert!((insight.impact_score - 10.0).abs() < 0.01);
            assert_eq!(insight.priority, InsightPriority::Low);
        }
    }

    // ===== code_root =====

    #[test]
    fn test_code_root_extraction() {
        assert_eq!(code_root("SEO001"), "SEO");
        assert_eq!(code_root("META002"), "META");
        assert_eq!(code_root("GRAPH003"), "GRAPH");
        assert_eq!(code_root("DUP-CROSS001"), "DUP");
        assert_eq!(code_root("LINK-EQ001"), "LINK");
    }

    // ===== InsightCategory =====

    #[test]
    fn test_category_as_str() {
        assert_eq!(InsightCategory::Technical.as_str(), "technical");
        assert_eq!(InsightCategory::Content.as_str(), "content");
        assert_eq!(InsightCategory::SEO.as_str(), "seo");
        assert_eq!(InsightCategory::Security.as_str(), "security");
        assert_eq!(InsightCategory::Performance.as_str(), "performance");
    }

    // ===== generate_insights_from_crawl_data =====

    #[test]
    fn test_generate_insights_from_crawl_data() {
        let pages = vec![test_page("https://example.com")];
        let issues = vec![Issue {
            id: "i1".to_string(),
            page_id: "p1".to_string(),
            category: IssueCategory::Seo,
            severity: Severity::Error,
            code: "SEO001".to_string(),
            title: "Missing title".to_string(),
            description: "Page has no title".to_string(),
            element: None,
            recommendation: "Add a title tag".to_string(),
            tenant_id: None,
        }];

        let data = CrawlData {
            pages,
            links: vec![],
            issues,
            seed_url: "https://example.com".to_string(),
        };

        let insights = generate_insights_from_crawl_data(&data);
        assert_eq!(insights.len(), 1);
        assert_eq!(insights[0].title, "Missing title");
    }

    // ===== Serialization roundtrip =====

    #[test]
    fn test_insight_serialization_roundtrip() {
        let insight = Insight {
            title: "Test".to_string(),
            description: "Desc".to_string(),
            priority: InsightPriority::High,
            impact_score: 75.5,
            effort: InsightEffort::Moderate,
            affected_pages: 10,
            finding_codes: vec!["SEO001".to_string()],
            recommendation: "Fix it".to_string(),
            category: InsightCategory::SEO,
        };

        let json = serde_json::to_string(&insight).unwrap();
        let deserialized: Insight = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.title, "Test");
        assert_eq!(deserialized.priority, InsightPriority::High);
        assert_eq!(deserialized.effort, InsightEffort::Moderate);
        assert_eq!(deserialized.category, InsightCategory::SEO);
        assert!((deserialized.impact_score - 75.5).abs() < 0.01);
    }

    // ===== Impact score clamping =====

    #[test]
    fn test_impact_score_clamped_to_100() {
        let pages = vec![test_page("https://example.com")];
        // Critical (100) on 100% of pages = 100, should not exceed
        let findings = vec![finding("SEC001", Severity::Critical, "https://example.com")];
        let insights = generate_insights(&findings, &pages);
        assert!(insights[0].impact_score <= 100.0);
    }

    // ===== Mixed categories =====

    #[test]
    fn test_mixed_category_insights() {
        let pages: Vec<PageData> = (0..20)
            .map(|i| test_page(&format!("https://example.com/p{i}")))
            .collect();

        let mut findings = Vec::new();
        // Security issue on 15 pages
        for i in 0..15 {
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Security,
                code: "SEC001".to_string(),
                title: "Missing CSP".to_string(),
                description: "No CSP header".to_string(),
                url: format!("https://example.com/p{i}"),
                recommendation: "Add CSP".to_string(),
            });
        }
        // SEO issue on 3 pages
        for i in 0..3 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "SEO001".to_string(),
                title: "Missing title".to_string(),
                description: "No title tag".to_string(),
                url: format!("https://example.com/p{i}"),
                recommendation: "Add title".to_string(),
            });
        }

        let insights = generate_insights(&findings, &pages);
        assert_eq!(insights.len(), 2);
        // Security issue has higher impact (75 * 0.75 = 56.25 vs 50 * 0.15 = 7.5)
        assert_eq!(insights[0].category, InsightCategory::Security);
        assert_eq!(insights[1].category, InsightCategory::SEO);
    }
}
