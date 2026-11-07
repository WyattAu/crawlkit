//! Mixed-content and HSTS preload-readiness analyzers.
//!
//! Extracted from `security_analyzers.rs` as a Phase 2 SRP step. Public
//! analyzer names and behavior are preserved through `analyzers::mod`
//! re-exports.

use crate::types::{IssueCategory, Severity};

use super::{AnalysisContext, Analyzer, Finding};

/// Ceiling on per-occurrence mixed-content findings for one page.
///
/// A page with hundreds of insecure references is one defect with one fix;
/// capping keeps the report readable while still naming every occurrence up
/// to the limit. `MixedContentFormValidator` shares the limit.
pub(crate) const MIXED_CONTENT_CAP: usize = 10;

/// The `http://` src of every `<iframe>` on the page, document order, at most
/// [`MIXED_CONTENT_CAP`] entries.
///
/// A deliberately small scanner consistent with the substring checks this
/// module already runs. Indices stay aligned between the lowercased scan copy
/// and the original because `to_ascii_lowercase` preserves byte length, so
/// evidence can be sliced from the original with mixed case intact. The first
/// `src` attribute per tag wins; a second occurrence on one iframe is a
/// markup error, not a second subresource.
fn http_iframe_srcs(html: &str) -> Vec<String> {
    let lower = html.to_ascii_lowercase();
    let mut out = Vec::new();
    let mut cursor = 0usize;
    while let Some(rel) = lower[cursor..].find("<iframe") {
        let tag_start = cursor + rel;
        let after = &lower[tag_start..];
        let tag_len = after.find('>').map_or(after.len(), |i| i + 1);
        let tag_lower = &lower[tag_start..tag_start + tag_len];
        let tag_orig = &html[tag_start..tag_start + tag_len];
        for quote in ['"', '\''] {
            let needle = format!("src={quote}http://");
            if let Some(pos) = tag_lower.find(&needle) {
                let rest = &tag_orig[pos + needle.len()..];
                if let Some(end) = rest.find(quote) {
                    out.push(rest[..end].to_string());
                }
                break;
            }
        }
        if out.len() >= MIXED_CONTENT_CAP {
            return out;
        }
        if tag_len == after.len() {
            // Unterminated final tag: nothing further can match.
            break;
        }
        cursor = tag_start + tag_len;
    }
    out
}

// =========================================================================
// MixedContentDetectionAnalyzer
// =========================================================================

pub struct MixedContentDetectionAnalyzer;

impl Default for MixedContentDetectionAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl MixedContentDetectionAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for MixedContentDetectionAnalyzer {
    fn name(&self) -> &str {
        "mixed-content-detection"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if !url.starts_with("https://") {
            return findings;
        }
        if let Some(body) = ctx.body {
            let lower = body.to_lowercase();
            let http_script = lower.contains("src=\"http://") && lower.contains("<script");
            let http_img = lower.contains("src=\"http://")
                && (lower.contains("<img") || lower.contains("background-image"));
            let http_link = lower.contains("href=\"http://") && lower.contains("<link");

            if http_script {
                findings.push(Finding {
                    severity: Severity::Critical,
                    category: IssueCategory::Security,
                    code: "MIXCONT001".to_string(),
                    title: "Mixed content: active script".to_string(),
                    description:
                        "HTTPS page loads scripts over HTTP, which can be intercepted and modified."
                            .to_string(),
                    url: url.clone(),
                    recommendation: "Change all script src attributes to use HTTPS.".to_string(),
                });
            }
            if http_img {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "MIXCONT002".to_string(),
                    title: "Mixed content: passive image".to_string(),
                    description: "HTTPS page loads images over HTTP.".to_string(),
                    url: url.clone(),
                    recommendation: "Change image sources to use HTTPS.".to_string(),
                });
            }
            if http_link {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "MIXCONT003".to_string(),
                    title: "Mixed content: stylesheet/resource".to_string(),
                    description: "HTTPS page loads stylesheets or other resources over HTTP."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Change resource URLs to use HTTPS.".to_string(),
                });
            }

            // iframes were the one subresource class this family had no check
            // for: scripts, images and `<link>`s are covered above, an
            // embedded frame is loaded and rendered exactly the same way, and
            // browsers treat HTTP frames on an HTTPS page as mixed content.
            // One finding per occurrence, capped, so each insecure embed is
            // named and fixable individually.
            for src in http_iframe_srcs(body) {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "MIXCONT004".to_string(),
                    title: "Mixed content: HTTP iframe".to_string(),
                    description: format!(
                        "HTTPS page embeds the frame `{src}` over HTTP. The frame \
                         content can be intercepted or replaced, and browsers may \
                         block it as mixed content."
                    ),
                    url: url.clone(),
                    recommendation: "Load the iframe from an HTTPS URL.".to_string(),
                });
            }
        }
        findings
    }
}

// =========================================================================
// HstsPreloadReadinessAnalyzer
// =========================================================================

pub struct HstsPreloadReadinessAnalyzer;

impl Default for HstsPreloadReadinessAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl HstsPreloadReadinessAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for HstsPreloadReadinessAnalyzer {
    fn name(&self) -> &str {
        "hsts-preload-readiness"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let hsts = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Strict-Transport-Security"))
            .map(|(_, v)| v.as_str());
        let hsts = match hsts {
            Some(v) => v,
            None => return findings,
        };

        let lower = hsts.to_lowercase();
        if !lower.contains("includesubdomains") {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Security,
                code: "HSTSPR001".to_string(),
                title: "HSTS missing includeSubDomains for preload".to_string(),
                description:
                    "HSTS header lacks includeSubDomains, required for preload list submission."
                        .to_string(),
                url: url.clone(),
                recommendation: "Add includeSubDomains to the HSTS header.".to_string(),
            });
        }
        if !lower.contains("preload") {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Security,
                code: "HSTSPR002".to_string(),
                title: "HSTS missing preload directive".to_string(),
                description:
                    "HSTS header lacks the preload directive for browser preload list inclusion."
                        .to_string(),
                url: url.clone(),
                recommendation: "Add preload to the HSTS header.".to_string(),
            });
        }
        if let Some(ma_pos) = lower.find("max-age=") {
            let after = &lower[ma_pos + 8..];
            let num_str: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(age) = num_str.parse::<u64>() {
                if age < 31536000 {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Security,
                        code: "HSTSPR003".to_string(),
                        title: "HSTS max-age too low for preload".to_string(),
                        description: format!(
                            "max-age is {age}, preload requires at least 31536000 (1 year)."
                        ),
                        url: url.clone(),
                        recommendation: "Set max-age to at least 31536000.".to_string(),
                    });
                }
            }
        }
        findings
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meta::MetaTags;
    use crate::parser::ParsedPage;

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
            has_lang_attribute: true,
            html_lang: Some("en".to_string()),
            has_aria_hidden: false,
            tables_with_headers: 0,
            tables_total: 0,
            tables_with_captions: 0,
            og_image_width: None,
            og_image_height: None,
        }
    }

    fn ctx_with_body<'a>(page: &'a ParsedPage, body: &'a str) -> AnalysisContext<'a> {
        AnalysisContext {
            body: Some(body),
            ..AnalysisContext::new(page)
        }
    }

    fn analyze_https(body: &str) -> Vec<Finding> {
        let page = fixture_page("https://example.com/");
        MixedContentDetectionAnalyzer::new().analyze(&ctx_with_body(&page, body))
    }

    #[test]
    fn http_iframe_is_reported_per_occurrence() {
        let findings = analyze_https(
            "<iframe src=\"http://ads.example.com/frame\"></iframe>\
             <iframe src=\"http://player.example.com/v\"></iframe>",
        );
        let iframes: Vec<&Finding> = findings.iter().filter(|f| f.code == "MIXCONT004").collect();
        assert_eq!(iframes.len(), 2, "one finding per insecure embed");
        assert_eq!(iframes[0].severity, Severity::Warning);
        assert!(
            iframes[0].description.contains("ads.example.com/frame")
                && iframes[1].description.contains("player.example.com/v"),
            "each finding names its own frame: {} | {}",
            iframes[0].description,
            iframes[1].description
        );
    }

    #[test]
    fn https_iframes_are_silent() {
        let findings = analyze_https("<iframe src=\"https://example.com/embed\"></iframe>");
        assert!(
            findings.iter().all(|f| f.code != "MIXCONT004"),
            "{findings:?}"
        );
    }

    #[test]
    fn findings_are_capped_per_page() {
        let mut html = String::new();
        for i in 0..25 {
            html.push_str(&format!(
                "<iframe src=\"http://f.example.com/{i}\"></iframe>"
            ));
        }
        let findings = analyze_https(&html);
        let count = findings.iter().filter(|f| f.code == "MIXCONT004").count();
        assert_eq!(count, MIXED_CONTENT_CAP, "cap at {MIXED_CONTENT_CAP}");
    }

    #[test]
    fn non_https_page_is_silent() {
        let page = fixture_page("http://example.com/");
        let findings = MixedContentDetectionAnalyzer::new().analyze(&ctx_with_body(
            &page,
            "<iframe src=\"http://ads.example.com/f\"></iframe>",
        ));
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn existing_category_findings_are_unchanged_by_iframe_addition() {
        // MIXCONT001-003 keep their granularity: one finding per category,
        // so adding the iframe check must not duplicate them.
        let findings = analyze_https(
            "<script src=\"http://evil.example/x.js\"></script>\
             <img src=\"http://img.example/p.png\">\
             <link rel=\"stylesheet\" href=\"http://css.example/s.css\">\
             <iframe src=\"http://ads.example/f\"></iframe>",
        );
        let codes: Vec<&str> = findings.iter().map(|f| f.code.as_str()).collect();
        assert!(codes.contains(&"MIXCONT001"), "{codes:?}");
        assert!(codes.contains(&"MIXCONT002"), "{codes:?}");
        assert!(codes.contains(&"MIXCONT003"), "{codes:?}");
        assert_eq!(
            codes.iter().filter(|c| **c == "MIXCONT001").count(),
            1,
            "script category stays one finding"
        );
        assert_eq!(
            codes.iter().filter(|c| **c == "MIXCONT004").count(),
            1,
            "exactly one iframe finding"
        );
    }

    #[test]
    fn single_quotes_and_unterminated_tags_are_handled() {
        let findings = analyze_https(
            "<iframe src='http://single.example/f'></iframe><iframe src=\"http://cut",
        );
        let iframes: Vec<&Finding> = findings.iter().filter(|f| f.code == "MIXCONT004").collect();
        assert_eq!(iframes.len(), 1, "unterminated tail tag cannot match");
        assert!(iframes[0].description.contains("single.example/f"));
    }
}
