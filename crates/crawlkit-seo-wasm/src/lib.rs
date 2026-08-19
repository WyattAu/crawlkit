use crawlkit_engine::analyzers::{AnalysisContext, AnalyzerRegistry, Finding};
use crawlkit_engine::parser::{HtmlParser, ParsedPage};
use crawlkit_engine::types::Severity;
use crawlkit_engine::{CrawlConfig, RedirectHop};
use serde::{Deserialize, Serialize};
use url::Url;
use wasm_bindgen::prelude::*;

// ---------------------------------------------------------------------------
// JS-facing types (matches the existing SeoResult JSON contract)
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize)]
struct JsIssue {
    severity: String,
    code: String,
    title: String,
    description: String,
}

#[derive(Serialize, Deserialize)]
struct SeoResult {
    url: String,
    title: String,
    title_length: usize,
    description: String,
    description_length: usize,
    canonical: Option<String>,
    has_lang: bool,
    lang: String,
    heading_counts: Vec<usize>,
    h1_count: usize,
    h2_count: usize,
    image_count: usize,
    images_missing_alt: usize,
    internal_links: usize,
    external_links: usize,
    word_count: usize,
    readability_score: f64,
    has_og_title: bool,
    has_og_description: bool,
    has_og_image: bool,
    has_twitter_card: bool,
    has_structured_data: bool,
    has_robots_meta: bool,
    robots_content: String,
    issues: Vec<JsIssue>,
    score: u32,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn count_words(text: &str) -> usize {
    text.split_whitespace().count()
}

fn flesch_reading_ease(text: &str) -> f64 {
    let sentences = text.split(['.', '!', '?']).count();
    let words = count_words(text);
    let syllables: usize = text
        .split_whitespace()
        .map(|w| w.chars().filter(|c| "aeiouy".contains(*c)).count().max(1))
        .sum();
    if sentences == 0 || words == 0 {
        return 0.0;
    }
    let s = words as f64 / sentences as f64;
    let w = syllables as f64 / words as f64;
    206.835 - 1.015 * s - 84.6 * w
}

fn severity_to_string(sev: &Severity) -> String {
    match sev {
        Severity::Critical | Severity::Error => "error".to_string(),
        Severity::Warning => "warning".to_string(),
        Severity::Info => "info".to_string(),
    }
}

fn findings_to_issues(findings: &[Finding]) -> Vec<JsIssue> {
    findings
        .iter()
        .map(|f| JsIssue {
            severity: severity_to_string(&f.severity),
            code: f.code.clone(),
            title: f.title.clone(),
            description: f.description.clone(),
        })
        .collect()
}

fn compute_score(findings: &[Finding]) -> u32 {
    if findings.is_empty() {
        return 100;
    }
    let deductions: f64 = findings
        .iter()
        .map(|f| match f.severity {
            Severity::Critical => 15.0,
            Severity::Error => 10.0,
            Severity::Warning => 3.0,
            Severity::Info => 0.0,
        })
        .sum();
    (100.0 - deductions).clamp(0.0, 100.0) as u32
}

// ---------------------------------------------------------------------------
// Main entry point
// ---------------------------------------------------------------------------

/// Analyze HTML for SEO issues using crawlkit-engine analyzers.
///
/// Returns a JSON-encoded `SeoResult` string (same contract as the legacy
/// `seo-wasm` crate).
#[wasm_bindgen]
pub fn analyze_seo(html: &str, url: &str) -> String {
    // 1. Parse URL
    let parsed_url = Url::parse(url)
        .unwrap_or_else(|_| Url::parse("https://example.com").expect("hardcoded URL must parse"));

    // 2. Parse HTML with crawlkit-engine parser
    let parsed: ParsedPage = HtmlParser::parse(html, &parsed_url);

    // 3. Build analyzer registry and config
    let config = CrawlConfig::default();
    let registry = AnalyzerRegistry::new(&config);

    // 4. Build analysis context (empty headers / no redirects for WASM)
    let empty_redirects: Vec<RedirectHop> = vec![];
    let empty_headers: Vec<(String, String)> = vec![];
    let ctx = AnalysisContext {
        page: &parsed,
        body: Some(html),
        status_code: Some(200),
        headers: &empty_headers,
        response_time: None,
        redirect_chain: &empty_redirects,
        robots_txt: None,
    };

    // 5. Run analyzers
    let findings = registry.analyze(&ctx);

    // 6. Extract metadata from parsed page
    let title = parsed.meta.title.clone().unwrap_or_default();
    let description = parsed.meta.description.clone().unwrap_or_default();
    let canonical = parsed.meta.canonical.as_ref().map(|u| u.to_string());
    let has_lang = parsed.has_lang_attribute;
    let lang = parsed.html_lang.clone().unwrap_or_default();

    let heading_counts: Vec<usize> = {
        let mut counts = vec![0usize; 6];
        for h in &parsed.headings {
            if h.level >= 1 && h.level <= 6 {
                counts[(h.level - 1) as usize] += 1;
            }
        }
        counts
    };
    let h1_count = heading_counts.first().copied().unwrap_or(0);
    let h2_count = heading_counts.get(1).copied().unwrap_or(0);

    let image_count = parsed.images.len();
    let images_missing_alt = parsed.images.iter().filter(|img| !img.has_alt).count();

    let internal_links = parsed.links.iter().filter(|l| !l.is_external).count();
    let external_links = parsed.links.iter().filter(|l| l.is_external).count();

    let word_count = parsed.word_count;
    let readability_score = if word_count > 0 {
        // Reconstruct body text approximation from word count
        // (ParsedPage stores word_count but not full body text)
        // Use a heuristic: 0.0 means we can't compute; the inline version
        // computed from raw HTML text. We'll do the same.
        let body_text: String = scraper::Html::parse_document(html)
            .root_element()
            .text()
            .collect::<Vec<_>>()
            .join(" ");
        flesch_reading_ease(&body_text)
    } else {
        0.0
    };

    let has_og_title = parsed.meta.og.title.is_some();
    let has_og_description = parsed.meta.og.description.is_some();
    let has_og_image = parsed.meta.og.image.is_some();
    let has_twitter_card = parsed.meta.twitter.card.is_some();

    let has_structured_data = !parsed.structured_data.is_empty();

    let robots_content = parsed.meta.robots.clone().unwrap_or_default();
    let has_robots_meta = !robots_content.is_empty();

    // 7. Map findings and compute score
    let issues = findings_to_issues(&findings);
    let score = compute_score(&findings);

    // 8. Serialize to JSON
    let result = SeoResult {
        url: url.to_string(),
        title_length: title.len(),
        description_length: description.len(),
        title,
        description,
        canonical,
        has_lang,
        lang,
        heading_counts,
        h1_count,
        h2_count,
        image_count,
        images_missing_alt,
        internal_links,
        external_links,
        word_count,
        readability_score,
        has_og_title,
        has_og_description,
        has_og_image,
        has_twitter_card,
        has_structured_data,
        has_robots_meta,
        robots_content,
        issues,
        score,
    };

    serde_json::to_string(&result).unwrap_or_else(|_| "{}".to_string())
}
