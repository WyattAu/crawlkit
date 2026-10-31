//! Lighthouse / Core Web Vitals ingestion — GAP G5.
//!
//! # Why this exists
//!
//! Lighthouse and PageSpeed Insights are where operators already look for
//! Core Web Vitals. crawlkit's own lab measurements (`web_vitals`, CrUX)
//! answer "what does this page score", but a Lighthouse JSON report a team
//! already produced sat outside every crawlkit report. This analyzer ingests
//! that report and turns its category scores and CWV audits into findings
//! rendered (and stored) like any other.
//!
//! # What is reported
//!
//! - One finding per Lighthouse category present (`performance`,
//!   `accessibility`, `best-practices`, `seo`), scored 0–100 with
//!   Lighthouse's own color bands: ≥90 good, 50–89 needs improvement,
//!   <50 poor.
//! - One finding per Core Web Vitals audit present, classified with the
//!   web.dev thresholds: LCP good ≤2500ms (poor >4000ms), CLS good ≤0.1
//!   (poor >0.25), TBT good ≤200ms (poor >600ms), TTFB good ≤800ms
//!   (poor >1800ms).
//!
//! # What is deliberately NOT done
//!
//! - **No network requests and no shelling out.** The analyzer reads a
//!   Lighthouse JSON report handed to it (from a file via the
//!   `crawlkit lighthouse` CLI, or from a stored body). Running Lighthouse
//!   is the caller's job; ingesting its output is ours.
//! - **No synthesis.** An audit the report does not contain produces no
//!   finding. Absent data is not invented, and a metric measured elsewhere
//!   is not compared or merged.
//!
//! A body that does not look like a Lighthouse report produces nothing —
//! the analyzer is registered in the default registry and runs on every
//! crawled page, so ordinary HTML must stay silent. A body that *claims*
//! to be a report but does not parse produces one Error finding rather
//! than a panic or a silent skip.

use crate::analyzers::Analyzer;
use crate::types::{IssueCategory, Severity};
use crate::{AnalysisContext, Finding};

/// LCP good-or-better boundary (web.dev): ≤2500ms.
pub const LCP_GOOD_MS: f64 = 2500.0;
/// LCP poor boundary (web.dev): >4000ms.
pub const LCP_POOR_MS: f64 = 4000.0;
/// CLS good-or-better boundary (web.dev): ≤0.1.
pub const CLS_GOOD: f64 = 0.1;
/// CLS poor boundary (web.dev): >0.25.
pub const CLS_POOR: f64 = 0.25;
/// TBT good-or-better boundary (web.dev): ≤200ms.
pub const TBT_GOOD_MS: f64 = 200.0;
/// TBT poor boundary (web.dev): >600ms.
pub const TBT_POOR_MS: f64 = 600.0;
/// TTFB good-or-better boundary (web.dev): ≤800ms.
pub const TTFB_GOOD_MS: f64 = 800.0;
/// TTFB poor boundary (web.dev): >1800ms.
pub const TTFB_POOR_MS: f64 = 1800.0;

/// Lighthouse category-score bands (0–100): ≥90 good, <50 poor.
const CATEGORY_GOOD: f64 = 90.0;
/// Lighthouse category-score band below which a category is poor.
const CATEGORY_POOR: f64 = 50.0;

/// The metrics this analyzer reports, in fixed output order.
const CWV_METRICS: &[(&str, &str)] = &[
    ("largest-contentful-paint", "cwv:lcp"),
    ("cumulative-layout-shift", "cwv:cls"),
    ("total-blocking-time", "cwv:tbt"),
    ("server-response-time", "cwv:ttfb"),
];

/// Category score order, matching the report's own presentation order.
const CATEGORY_ORDER: &[&str] = &["performance", "accessibility", "best-practices", "seo"];

/// Why a Lighthouse report could not be ingested.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LighthouseParseError {
    /// The body is not valid JSON.
    Json(String),
    /// Valid JSON, but not a Lighthouse report (no `audits` object).
    MissingAudits,
}

impl std::fmt::Display for LighthouseParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LighthouseParseError::Json(e) => write!(f, "report is not valid JSON: {e}"),
            LighthouseParseError::MissingAudits => {
                write!(
                    f,
                    "report has no `audits` object; is it a Lighthouse report?"
                )
            }
        }
    }
}

impl std::error::Error for LighthouseParseError {}

/// A Lighthouse report reduced to what this analyzer reports.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LighthouseReport {
    /// Category scores normalized to 0–100, in [`CATEGORY_ORDER`] order.
    pub categories: Vec<(String, f64)>,
    /// Largest Contentful Paint in milliseconds (`largest-contentful-paint`).
    pub lcp_ms: Option<f64>,
    /// Cumulative Layout Shift, dimensionless (`cumulative-layout-shift`).
    pub cls: Option<f64>,
    /// Total Blocking Time in milliseconds (`total-blocking-time`, falling
    /// back to the legacy `interactive` audit when TBT is absent).
    pub tbt_ms: Option<f64>,
    /// Server response time in milliseconds (`server-response-time`).
    pub ttfb_ms: Option<f64>,
}

impl LighthouseReport {
    /// Value and finding code of one tracked CWV audit, or `None` when the
    /// report does not contain it.
    fn metric(&self, audit_id: &str) -> Option<(f64, &'static str)> {
        match audit_id {
            "largest-contentful-paint" => self.lcp_ms.map(|v| (v, "cwv:lcp")),
            "cumulative-layout-shift" => self.cls.map(|v| (v, "cwv:cls")),
            "total-blocking-time" => self.tbt_ms.map(|v| (v, "cwv:tbt")),
            "server-response-time" => self.ttfb_ms.map(|v| (v, "cwv:ttfb")),
            _ => None,
        }
    }

    /// Units and web.dev boundaries for one tracked audit.
    fn metric_bounds(audit_id: &str) -> (&'static str, f64, f64) {
        match audit_id {
            "largest-contentful-paint" => ("ms", LCP_GOOD_MS, LCP_POOR_MS),
            "cumulative-layout-shift" => ("", CLS_GOOD, CLS_POOR),
            "total-blocking-time" => ("ms", TBT_GOOD_MS, TBT_POOR_MS),
            // Server response time is assessed with the TTFB thresholds:
            // web.dev's TTFB guidance *is* the server-response-time audit.
            "server-response-time" => ("ms", TTFB_GOOD_MS, TTFB_POOR_MS),
            _ => ("", f64::INFINITY, f64::INFINITY),
        }
    }
}

/// Whether a raw body claims to be a Lighthouse report.
///
/// Deliberately narrow: the analyzer runs on every crawled page, and a loose
/// match (any JSON with an `audits` key) would fire the parse-error finding
/// on unrelated JSON documents. `lighthouseVersion` is present in every
/// Lighthouse report since v2 and is not a field ordinary JSON carries.
fn looks_like_lighthouse_report(body: &str) -> bool {
    body.contains("\"lighthouseVersion\"")
        || (body.contains("\"audits\"") && body.contains("\"largest-contentful-paint\""))
}

/// Read `audits.<id>.numericValue` from a Lighthouse report object.
fn audit_numeric_value(
    audits: &serde_json::Map<String, serde_json::Value>,
    id: &str,
) -> Option<f64> {
    let audit = audits.get(id)?.as_object()?;
    audit.get("numericValue").and_then(|v| v.as_f64())
}

/// Parse a Lighthouse JSON report.
///
/// Category scores arrive from Lighthouse on a 0–1 scale; scores ≤1 are
/// therefore scaled ×100, and a score already >1 is taken as 0–100 as-is,
/// so hand-written fixtures in either convention classify the same way.
/// `null` category scores (Lighthouse emits them when a category could not
/// be scored) and absent audits are skipped, not defaulted.
pub fn parse_lighthouse_report(json: &str) -> Result<LighthouseReport, LighthouseParseError> {
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|e| LighthouseParseError::Json(e.to_string()))?;
    let obj = match value.as_object() {
        Some(obj) => obj,
        None => return Err(LighthouseParseError::MissingAudits),
    };
    let audits = match obj.get("audits").and_then(|a| a.as_object()) {
        Some(audits) => audits,
        None => return Err(LighthouseParseError::MissingAudits),
    };

    let mut report = LighthouseReport {
        lcp_ms: audit_numeric_value(audits, "largest-contentful-paint"),
        cls: audit_numeric_value(audits, "cumulative-layout-shift"),
        // TBT replaced Time to Interactive as the field metric; older
        // reports only carry `interactive`, which answers the same "how
        // long is the main thread blocked" question for this purpose.
        tbt_ms: audit_numeric_value(audits, "total-blocking-time")
            .or_else(|| audit_numeric_value(audits, "interactive")),
        ttfb_ms: audit_numeric_value(audits, "server-response-time"),
        categories: Vec::new(),
    };

    if let Some(categories) = obj.get("categories").and_then(|c| c.as_object()) {
        for name in CATEGORY_ORDER {
            let Some(category) = categories.get(*name) else {
                continue;
            };
            let Some(score) = category.get("score").and_then(|s| s.as_f64()) else {
                continue;
            };
            let normalized = if score <= 1.0 { score * 100.0 } else { score };
            report
                .categories
                .push(((*name).to_string(), (normalized * 100.0).round() / 100.0));
        }
    }

    Ok(report)
}

/// Shared assessment ladder: `≤ good` is Info, `> poor` is Error, and the
/// band between is the needs-improvement Warning.
fn classify(value: f64, good_max: f64, poor_min: f64) -> Severity {
    if value <= good_max {
        Severity::Info
    } else if value > poor_min {
        Severity::Error
    } else {
        Severity::Warning
    }
}

/// Classify an LCP value in milliseconds against the web.dev thresholds.
#[must_use]
pub fn classify_lcp(ms: f64) -> Severity {
    classify(ms, LCP_GOOD_MS, LCP_POOR_MS)
}

/// Classify a CLS value against the web.dev thresholds.
#[must_use]
pub fn classify_cls(cls: f64) -> Severity {
    classify(cls, CLS_GOOD, CLS_POOR)
}

/// Classify a TBT value in milliseconds against the web.dev thresholds.
#[must_use]
pub fn classify_tbt(ms: f64) -> Severity {
    classify(ms, TBT_GOOD_MS, TBT_POOR_MS)
}

/// Classify a TTFB (server response time) value in milliseconds against the
/// web.dev thresholds.
#[must_use]
pub fn classify_ttfb(ms: f64) -> Severity {
    classify(ms, TTFB_GOOD_MS, TTFB_POOR_MS)
}

/// Classify a Lighthouse category score (0–100) against Lighthouse's bands.
///
/// Unlike the metric ladders, a *high* score is good: ≥90 good (Info),
/// <50 poor (Error), the band between needs improvement (Warning).
#[must_use]
pub fn classify_category_score(score: f64) -> Severity {
    if score >= CATEGORY_GOOD {
        Severity::Info
    } else if score < CATEGORY_POOR {
        Severity::Error
    } else {
        Severity::Warning
    }
}

/// Human wording for a metric value, omitting the unit for unitless CLS.
fn format_metric_value(value: f64, unit: &str) -> String {
    if unit.is_empty() {
        format!("{value:.3}")
    } else {
        format!("{value:.0}{unit}")
    }
}

/// Lighthouse JSON report ingestion: category scores and CWV audits.
pub struct LighthouseAnalyzer;

impl Default for LighthouseAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl LighthouseAnalyzer {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Ingest a report body and produce its findings, split out from
    /// [`Analyzer::analyze`] so the CLI can run the analyzer on a report
    /// file without assembling a fetch-shaped context around it.
    pub fn analyze_report(&self, report_json: &str, url: &str) -> Vec<Finding> {
        if !looks_like_lighthouse_report(report_json) {
            return Vec::new();
        }
        let report = match parse_lighthouse_report(report_json) {
            Ok(report) => report,
            Err(error) => {
                return vec![Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Custom("lighthouse".to_string()),
                    code: "lighthouse:invalid-report".to_string(),
                    title: "Lighthouse report could not be parsed".to_string(),
                    description: format!(
                        "The supplied Lighthouse report was not ingested: {error}. \
                         No CWV assessment was made; run `lighthouse --output=json` \
                         and pass the file it wrote."
                    ),
                    url: url.to_string(),
                    recommendation: "Re-export the report as Lighthouse JSON and \
                                     re-run the ingestion."
                        .to_string(),
                }];
            }
        };

        let mut findings = Vec::new();
        for (name, score) in &report.categories {
            let severity = classify_category_score(*score);
            let band = match severity {
                Severity::Info => "good",
                Severity::Warning => "needs improvement",
                _ => "poor",
            };
            findings.push(Finding {
                severity,
                category: IssueCategory::Custom("lighthouse".to_string()),
                code: format!("lighthouse:category-{name}"),
                title: format!("Lighthouse {name} score: {score:.0}/100 ({band})"),
                description: format!(
                    "The ingested Lighthouse report scores the `{name}` category \
                     {score:.0}/100, which lands in the \"{band}\" band of \
                     Lighthouse's own color scale (good ≥90, needs improvement \
                     50–89, poor <50)."
                ),
                url: url.to_string(),
                recommendation: match severity {
                    Severity::Info => "No action required; the category is in the \
                                       good band."
                        .to_string(),
                    _ => "Open the full Lighthouse report and work through the \
                          failed audits in this category, highest-weighted first."
                        .to_string(),
                },
            });
        }

        for (audit_id, code) in CWV_METRICS {
            let Some((value, _)) = report.metric(audit_id) else {
                continue;
            };
            let (unit, good, poor) = LighthouseReport::metric_bounds(audit_id);
            let severity = classify(value, good, poor);
            let (metric_name, band, advice) = match *code {
                "cwv:lcp" => (
                    "Largest Contentful Paint (LCP)",
                    "good ≤2500ms, poor >4000ms",
                    "preload the LCP image, serve it from the same origin, and \
                     defer anything competing with it",
                ),
                "cwv:cls" => (
                    "Cumulative Layout Shift (CLS)",
                    "good ≤0.1, poor >0.25",
                    "reserve space for late-loading media and embeds with width \
                     and height or aspect-ratio",
                ),
                "cwv:tbt" => (
                    "Total Blocking Time (TBT)",
                    "good ≤200ms, poor >600ms",
                    "split long tasks, reduce third-party script, and defer \
                     non-critical JavaScript",
                ),
                _ => (
                    "Time to First Byte (TTFB)",
                    "good ≤800ms, poor >1800ms",
                    "add caching/CDN edge delivery and check slow backend routes",
                ),
            };
            findings.push(Finding {
                severity,
                category: IssueCategory::Performance,
                code: (*code).to_string(),
                title: format!(
                    "{}: {} ({})",
                    metric_name,
                    format_metric_value(value, unit),
                    match severity {
                        Severity::Info => "good",
                        Severity::Warning => "needs improvement",
                        _ => "poor",
                    }
                ),
                description: format!(
                    "The ingested Lighthouse report measures {metric_name} at {}. \
                     web.dev thresholds: {band}.",
                    format_metric_value(value, unit),
                ),
                url: url.to_string(),
                recommendation: match severity {
                    Severity::Info => {
                        "No action required; the metric is within the good band.".to_string()
                    }
                    _ => format!("To improve {metric_name}: {advice}."),
                },
            });
        }
        findings
    }
}

impl Analyzer for LighthouseAnalyzer {
    fn name(&self) -> &str {
        "lighthouse"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let Some(body) = ctx.body else {
            return Vec::new();
        };
        self.analyze_report(body, &ctx.page.url.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meta::MetaTags;
    use crate::parser::ParsedPage;

    /// A minimal parsed page carrying the URL findings are attributed to.
    fn fixture_page(url: &str) -> ParsedPage {
        ParsedPage {
            url: url.to_string(),
            meta: MetaTags::default(),
            headings: vec![],
            links: vec![],
            images: vec![],
            forms: vec![],
            scripts: vec![],
            styles: vec![],
            structured_data: vec![],
            word_count: 0,
            sentence_count: 0,
            landmarks: vec![],
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

    /// Build a report JSON with the given audit numeric values and
    /// category scores (on Lighthouse's native 0–1 scale).
    fn report_json(audits: &[(&str, f64)], categories: &[(&str, f64)]) -> String {
        let audit_pairs: Vec<String> = audits
            .iter()
            .map(|(id, v)| format!("\"{id}\": {{\"numericValue\": {v}}}"))
            .collect();
        let cat_pairs: Vec<String> = categories
            .iter()
            .map(|(name, score)| format!("\"{name}\": {{\"score\": {score}}}"))
            .collect();
        format!(
            "{{\"lighthouseVersion\": \"11.0.0\", \"audits\": {{{}}}, \
             \"categories\": {{{}}}}}",
            audit_pairs.join(", "),
            cat_pairs.join(", ")
        )
    }

    fn analyze_json(json: &str) -> Vec<Finding> {
        let page = fixture_page("https://example.test/");
        let ctx = AnalysisContext {
            body: Some(json),
            ..AnalysisContext::new(&page)
        };
        LighthouseAnalyzer::new().analyze(&ctx)
    }

    // ---- CWV classification (web.dev thresholds) ------------------------

    #[test]
    fn lcp_classification_ladder() {
        assert_eq!(classify_lcp(2500.0), Severity::Info, "at the good boundary");
        assert_eq!(classify_lcp(800.0), Severity::Info);
        assert_eq!(classify_lcp(2500.1), Severity::Warning, "just past good");
        assert_eq!(
            classify_lcp(4000.0),
            Severity::Warning,
            "poor starts above 4000"
        );
        assert_eq!(classify_lcp(4000.1), Severity::Error);
        assert_eq!(classify_lcp(9000.0), Severity::Error);
    }

    #[test]
    fn cls_classification_ladder() {
        assert_eq!(classify_cls(0.1), Severity::Info, "at the good boundary");
        assert_eq!(classify_cls(0.0), Severity::Info);
        assert_eq!(classify_cls(0.15), Severity::Warning);
        assert_eq!(
            classify_cls(0.25),
            Severity::Warning,
            "poor starts above 0.25"
        );
        assert_eq!(classify_cls(0.3), Severity::Error);
    }

    #[test]
    fn tbt_and_ttfb_classification_ladders() {
        assert_eq!(classify_tbt(200.0), Severity::Info);
        assert_eq!(classify_tbt(450.0), Severity::Warning);
        assert_eq!(classify_tbt(600.1), Severity::Error);
        assert_eq!(classify_ttfb(800.0), Severity::Info);
        assert_eq!(classify_ttfb(1200.0), Severity::Warning);
        assert_eq!(classify_ttfb(1800.1), Severity::Error);
    }

    #[test]
    fn category_score_classification_uses_lighthouse_bands() {
        assert_eq!(classify_category_score(100.0), Severity::Info);
        assert_eq!(classify_category_score(90.0), Severity::Info);
        assert_eq!(classify_category_score(89.9), Severity::Warning);
        assert_eq!(classify_category_score(50.0), Severity::Warning);
        assert_eq!(classify_category_score(49.9), Severity::Error);
        assert_eq!(classify_category_score(0.0), Severity::Error);
    }

    // ---- Report parsing (fixture JSON, no network) ----------------------

    #[test]
    fn parses_all_good_report() {
        let json = report_json(
            &[
                ("largest-contentful-paint", 1200.0),
                ("cumulative-layout-shift", 0.02),
                ("total-blocking-time", 100.0),
                ("server-response-time", 300.0),
            ],
            &[
                ("performance", 0.95),
                ("accessibility", 1.0),
                ("best-practices", 1.0),
                ("seo", 0.92),
            ],
        );
        let report = parse_lighthouse_report(&json).unwrap();
        assert_eq!(report.lcp_ms, Some(1200.0));
        assert_eq!(report.cls, Some(0.02));
        assert_eq!(report.tbt_ms, Some(100.0));
        assert_eq!(report.ttfb_ms, Some(300.0));
        assert_eq!(
            report.categories,
            vec![
                ("performance".to_string(), 95.0),
                ("accessibility".to_string(), 100.0),
                ("best-practices".to_string(), 100.0),
                ("seo".to_string(), 92.0),
            ]
        );
    }

    #[test]
    fn parses_needs_improvement_report() {
        let json = report_json(
            &[
                ("largest-contentful-paint", 3100.0),
                ("cumulative-layout-shift", 0.18),
                ("total-blocking-time", 420.0),
                ("server-response-time", 1200.0),
            ],
            &[("performance", 0.62)],
        );
        let report = parse_lighthouse_report(&json).unwrap();
        assert_eq!(report.lcp_ms, Some(3100.0));
        assert_eq!(report.cls, Some(0.18));
        assert_eq!(report.tbt_ms, Some(420.0));
        assert_eq!(report.ttfb_ms, Some(1200.0));
        assert_eq!(report.categories.len(), 1);
        assert_eq!(report.categories[0].0, "performance");
    }

    #[test]
    fn parses_poor_report_with_legacy_interactive_fallback() {
        // Pre-TBT Lighthouse reports carry `interactive` instead of
        // `total-blocking-time`; the fallback keeps them ingestible.
        let json = report_json(
            &[
                ("largest-contentful-paint", 6200.0),
                ("cumulative-layout-shift", 0.31),
                ("interactive", 7400.0),
                ("server-response-time", 2400.0),
            ],
            &[("performance", 0.31), ("seo", 0.44)],
        );
        let report = parse_lighthouse_report(&json).unwrap();
        assert_eq!(
            report.tbt_ms,
            Some(7400.0),
            "`interactive` feeds the TBT slot"
        );
        assert_eq!(report.lcp_ms, Some(6200.0));

        let findings = analyze_json(&json);
        let by_code: std::collections::HashMap<String, Severity> = findings
            .iter()
            .map(|f| (f.code.clone(), f.severity))
            .collect();
        assert_eq!(by_code.get("cwv:lcp"), Some(&Severity::Error));
        assert_eq!(by_code.get("cwv:cls"), Some(&Severity::Error));
        assert_eq!(by_code.get("cwv:tbt"), Some(&Severity::Error));
        assert_eq!(by_code.get("cwv:ttfb"), Some(&Severity::Error));
        assert_eq!(
            by_code.get("lighthouse:category-performance"),
            Some(&Severity::Error)
        );
        assert_eq!(
            by_code.get("lighthouse:category-seo"),
            Some(&Severity::Error)
        );
    }

    #[test]
    fn mixed_report_classifies_each_metric_independently() {
        let json = report_json(
            &[
                ("largest-contentful-paint", 1800.0),
                ("cumulative-layout-shift", 0.2),
                ("total-blocking-time", 90.0),
                ("server-response-time", 2400.0),
            ],
            &[("accessibility", 0.55)],
        );
        let findings = analyze_json(&json);
        let by_code: std::collections::HashMap<String, Severity> = findings
            .iter()
            .map(|f| (f.code.clone(), f.severity))
            .collect();
        assert_eq!(by_code.get("cwv:lcp"), Some(&Severity::Info));
        assert_eq!(by_code.get("cwv:cls"), Some(&Severity::Warning));
        assert_eq!(by_code.get("cwv:tbt"), Some(&Severity::Info));
        assert_eq!(by_code.get("cwv:ttfb"), Some(&Severity::Error));
        assert_eq!(
            by_code.get("lighthouse:category-accessibility"),
            Some(&Severity::Warning)
        );
    }

    #[test]
    fn missing_audits_produce_no_metric_findings() {
        let json = report_json(&[], &[("performance", 0.7)]);
        let findings = analyze_json(&json);
        assert!(
            findings.iter().all(|f| !f.code.starts_with("cwv:")),
            "absent audits must not synthesize metrics: {:?}",
            findings.iter().map(|f| &f.code).collect::<Vec<_>>()
        );
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "lighthouse:category-performance");
    }

    #[test]
    fn malformed_report_json_yields_error_finding_not_panic() {
        // Claims to be a report, but the JSON is truncated.
        let json = "{\"lighthouseVersion\": \"11.0.0\", \"audits\": {";
        let findings = analyze_json(json);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "lighthouse:invalid-report");
        assert_eq!(findings[0].severity, Severity::Error);
        assert!(findings[0].description.contains("not valid JSON"));
    }

    #[test]
    fn json_without_audits_yields_error_finding() {
        let findings = analyze_json("{\"lighthouseVersion\": \"11.0.0\"}");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "lighthouse:invalid-report");
        assert!(findings[0].description.contains("`audits`"));
    }

    #[test]
    fn plain_html_body_produces_nothing() {
        // The analyzer is registered in the default registry and runs on
        // every crawled page; ordinary bodies must stay silent.
        assert!(analyze_json("<html><body><p>hello</p></body></html>").is_empty());
        assert!(analyze_json("{\"title\": \"unrelated json\"}").is_empty());
        let page = fixture_page("https://example.test/");
        let ctx = AnalysisContext {
            body: None,
            ..AnalysisContext::new(&page)
        };
        assert!(LighthouseAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn analyzer_name_and_output_order_are_stable() {
        let json = report_json(
            &[
                ("largest-contentful-paint", 1000.0),
                ("cumulative-layout-shift", 0.01),
                ("total-blocking-time", 50.0),
                ("server-response-time", 200.0),
            ],
            &[("performance", 0.99), ("seo", 0.98)],
        );
        let findings = analyze_json(&json);
        let codes: Vec<&str> = findings.iter().map(|f| f.code.as_str()).collect();
        assert_eq!(
            codes,
            vec![
                "lighthouse:category-performance",
                "lighthouse:category-seo",
                "cwv:lcp",
                "cwv:cls",
                "cwv:tbt",
                "cwv:ttfb",
            ]
        );
        assert_eq!(LighthouseAnalyzer::new().name(), "lighthouse");
    }

    #[test]
    fn score_scale_is_normalized_from_zero_to_one() {
        // Lighthouse emits 0–1; a fixture already on a 0–100 scale must not
        // be multiplied again.
        let zero_one = parse_lighthouse_report(&report_json(&[], &[("seo", 0.91)])).unwrap();
        assert_eq!(zero_one.categories[0].1, 91.0);
        let already_hundred = parse_lighthouse_report(&report_json(&[], &[("seo", 91.0)])).unwrap();
        assert_eq!(already_hundred.categories[0].1, 91.0);
    }

    #[test]
    fn null_category_score_is_skipped() {
        let json = "{\"lighthouseVersion\": \"11.0.0\", \
                    \"audits\": {}, \"categories\": {\"performance\": {\"score\": null}, \
                    \"seo\": {\"score\": 0.9}}}";
        let report = parse_lighthouse_report(json).unwrap();
        assert_eq!(
            report.categories,
            vec![("seo".to_string(), 90.0)],
            "a null score means Lighthouse could not score it; reporting a 0 \
             would invent a failure"
        );
    }
}
