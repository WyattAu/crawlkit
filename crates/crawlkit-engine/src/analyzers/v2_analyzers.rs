#![allow(clippy::unwrap_used, clippy::manual_range_contains, clippy::redundant_closure, clippy::collapsible_if, clippy::unnecessary_map_or, clippy::default_constructed_unit_structs, clippy::needless_return, clippy::needless_range_loop, clippy::useless_format, clippy::if_same_then_else, clippy::derivable_impls, clippy::manual_pattern_char_comparison, clippy::manual_contains, clippy::collapsible_match)]
use crate::types::{IssueCategory, Severity};
use super::{AnalysisContext, Analyzer, Finding};

// =========================================================================
// Content Score Analyzers
// =========================================================================

pub struct DuplicateContentDetectorV2;
impl Default for DuplicateContentDetectorV2 { fn default() -> Self { Self::new() } }
impl DuplicateContentDetectorV2 { pub fn new() -> Self { Self } }
impl Analyzer for DuplicateContentDetectorV2 {
    fn name(&self) -> &str { "duplicate-content-v2" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(body) = ctx.body {
            let trimmed = body.trim();
            if trimmed.len() < 200 { return findings; }
            let normalized: String = trimmed.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_lowercase();
            let chunk_size = 200;
            let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
            for i in (0..normalized.len()).step_by(chunk_size / 2) {
                let end = (i + chunk_size).min(normalized.len());
                let chunk = &normalized[i..end];
                if chunk.len() >= 50 { *seen.entry(chunk.to_string()).or_insert(0) += 1; }
            }
            let dup_count: usize = seen.values().filter(|&&c| c > 2).count();
            if dup_count > 3 {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Content, code: "DUP-V2001".to_string(), title: "Repeated text chunks detected".to_string(), description: format!("{dup_count} text chunks appear 3+ times, suggesting boilerplate content."), url: url.clone(), recommendation: "Reduce repetitive boilerplate and ensure unique value on each page.".to_string() });
            }
        }
        findings
    }
}

pub struct ContentFreshnessScoreAnalyzer;
impl Default for ContentFreshnessScoreAnalyzer { fn default() -> Self { Self::new() } }
impl ContentFreshnessScoreAnalyzer { pub fn new() -> Self { Self } }
impl Analyzer for ContentFreshnessScoreAnalyzer {
    fn name(&self) -> &str { "content-freshness-score" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let mut date_found = false;
        for sd in &ctx.page.structured_data {
            if let Some(dp) = sd.data.get("datePublished").and_then(|v| v.as_str()) {
                if !dp.is_empty() {
                    date_found = true;
                    if let Ok(parsed) = chrono::NaiveDate::parse_from_str(&dp[..dp.len().min(10)], "%Y-%m-%d") {
                        let today = chrono::NaiveDate::from_ymd_opt(2026, 8, 30).unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(2025, 1, 1).unwrap());
                        let age = (today - parsed).num_days();
                        if age > 365 {
                            findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Content, code: "FRESHSC002".to_string(), title: "Content is over a year old".to_string(), description: format!("Content date is {age} days old. Outdated content may rank lower."), url: url.clone(), recommendation: "Update the content and refresh the date.".to_string() });
                        } else if age > 180 {
                            findings.push(Finding { severity: Severity::Info, category: IssueCategory::Content, code: "FRESHSC003".to_string(), title: "Content is over 6 months old".to_string(), description: format!("Content date is {age} days old. Consider refreshing soon."), url: url.clone(), recommendation: "Review and update the content.".to_string() });
                        }
                    }
                }
            }
        }
        if !date_found {
            findings.push(Finding { severity: Severity::Info, category: IssueCategory::Content, code: "FRESHSC001".to_string(), title: "No date metadata found".to_string(), description: "No datePublished found in structured data.".to_string(), url: url.clone(), recommendation: "Add datePublished to structured data.".to_string() });
        }
        findings
    }
}

pub struct HeadingStructureScoreAnalyzer;
impl Default for HeadingStructureScoreAnalyzer { fn default() -> Self { Self::new() } }
impl HeadingStructureScoreAnalyzer { pub fn new() -> Self { Self } }
impl Analyzer for HeadingStructureScoreAnalyzer {
    fn name(&self) -> &str { "heading-structure-score" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.headings.is_empty() {
            findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Content, code: "HEADSC001".to_string(), title: "No headings found".to_string(), description: "The page has no heading elements.".to_string(), url: url.clone(), recommendation: "Add at least one H1 and hierarchical H2-H6 headings.".to_string() });
            return findings;
        }
        let h1_count = ctx.page.headings.iter().filter(|h| h.level == 1).count();
        if h1_count == 0 {
            findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Content, code: "HEADSC002".to_string(), title: "Missing H1 heading".to_string(), description: "No H1 heading found.".to_string(), url: url.clone(), recommendation: "Add a single H1 heading.".to_string() });
        } else if h1_count > 1 {
            findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Content, code: "HEADSC003".to_string(), title: "Multiple H1 headings".to_string(), description: format!("{h1_count} H1 headings found. Only one is recommended."), url: url.clone(), recommendation: "Use a single H1 heading per page.".to_string() });
        }
        let levels: Vec<(u32, usize)> = (1u32..=6).map(|l| (l, ctx.page.headings.iter().filter(|h| h.level as u32 == l).count())).filter(|(_, c)| *c > 0).collect();
        for w in levels.windows(2) {
            if w[1].0 > w[0].0 + 1 {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Content, code: "HEADSC004".to_string(), title: "Heading level skipped".to_string(), description: format!("Heading jumps from H{} to H{}.", w[0].0, w[1].0), url: url.clone(), recommendation: "Use heading levels sequentially.".to_string() });
            }
        }
        findings
    }
}

pub struct LinkQualityScoreAnalyzer;
impl Default for LinkQualityScoreAnalyzer { fn default() -> Self { Self::new() } }
impl LinkQualityScoreAnalyzer { pub fn new() -> Self { Self } }
impl Analyzer for LinkQualityScoreAnalyzer {
    fn name(&self) -> &str { "link-quality-score" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.links.is_empty() {
            findings.push(Finding { severity: Severity::Info, category: IssueCategory::Content, code: "LINKSC001".to_string(), title: "No links on page".to_string(), description: "The page contains no links.".to_string(), url: url.clone(), recommendation: "Add relevant internal and external links.".to_string() });
            return findings;
        }
        let total = ctx.page.links.len();
        let nofollow = ctx.page.links.iter().filter(|l| l.rel.iter().any(|r| r == "nofollow")).count();
        if nofollow > 0 && nofollow as f64 / total as f64 > 0.5 {
            findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Content, code: "LINKSC002".to_string(), title: "High nofollow link ratio".to_string(), description: format!("{nofollow}/{total} links are nofollowed."), url: url.clone(), recommendation: "Review nofollow usage on internal links.".to_string() });
        }
        let empty_text = ctx.page.links.iter().filter(|l| l.text.trim().is_empty() && l.aria_label.is_none()).count();
        if empty_text > 0 {
            findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Content, code: "LINKSC003".to_string(), title: "Links without anchor text".to_string(), description: format!("{empty_text} link(s) have no text or aria-label."), url: url.clone(), recommendation: "Add descriptive text to all links.".to_string() });
        }
        findings
    }
}

pub struct SchemaCoverageScoreAnalyzer;
impl Default for SchemaCoverageScoreAnalyzer { fn default() -> Self { Self::new() } }
impl SchemaCoverageScoreAnalyzer { pub fn new() -> Self { Self } }
impl Analyzer for SchemaCoverageScoreAnalyzer {
    fn name(&self) -> &str { "schema-coverage-score" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.structured_data.is_empty() {
            findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "SCHEMACOV001".to_string(), title: "No structured data present".to_string(), description: "No JSON-LD structured data found.".to_string(), url: url.clone(), recommendation: "Add Schema.org JSON-LD markup.".to_string() });
            return findings;
        }
        let invalid = ctx.page.structured_data.iter().filter(|sd| sd.context.as_deref() != Some("https://schema.org") && sd.context.as_deref() != Some("schema.org")).count();
        if invalid > 0 {
            findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "SCHEMACOV002".to_string(), title: "Non-standard @context".to_string(), description: format!("{invalid} structured data blocks use non-standard @context."), url: url.clone(), recommendation: "Use https://schema.org as @context.".to_string() });
        }
        findings
    }
}

pub struct SecurityScoreAnalyzer;
impl Default for SecurityScoreAnalyzer { fn default() -> Self { Self::new() } }
impl SecurityScoreAnalyzer { pub fn new() -> Self { Self } }
impl Analyzer for SecurityScoreAnalyzer {
    fn name(&self) -> &str { "security-score" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let mut score: u32 = 100;
        let mut issues: Vec<String> = Vec::new();
        if !ctx.headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("Content-Security-Policy")) { score = score.saturating_sub(15); issues.push("no CSP".to_string()); }
        if !ctx.headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("Strict-Transport-Security")) { score = score.saturating_sub(15); issues.push("no HSTS".to_string()); }
        if !ctx.headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("X-Content-Type-Options")) { score = score.saturating_sub(10); issues.push("no XCTO".to_string()); }
        if !ctx.headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("X-Frame-Options")) && !ctx.headers.iter().any(|(k, v)| k.eq_ignore_ascii_case("Content-Security-Policy") && v.contains("frame-ancestors")) { score = score.saturating_sub(10); issues.push("no XFO".to_string()); }
        if !ctx.headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("Referrer-Policy")) { score = score.saturating_sub(5); issues.push("no RP".to_string()); }
        if !ctx.headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("Permissions-Policy") || k.eq_ignore_ascii_case("Feature-Policy")) { score = score.saturating_sub(5); issues.push("no PP".to_string()); }
        let rec = if score < 50 { "Critical security headers missing.".to_string() } else if score < 80 { "Several security headers missing.".to_string() } else { "Good security posture.".to_string() };
        findings.push(Finding { severity: Severity::Info, category: IssueCategory::Security, code: "SECSC001".to_string(), title: "Security header score".to_string(), description: format!("Score: {score}/100. Issues: {}.", issues.join(", ")), url: url.clone(), recommendation: rec });
        findings
    }
}

pub struct AccessibilityScoreAnalyzer;
impl Default for AccessibilityScoreAnalyzer { fn default() -> Self { Self::new() } }
impl AccessibilityScoreAnalyzer { pub fn new() -> Self { Self } }
impl Analyzer for AccessibilityScoreAnalyzer {
    fn name(&self) -> &str { "accessibility-score" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let mut score: u32 = 100;
        let mut issues: Vec<String> = Vec::new();
        if !ctx.page.has_lang_attribute { score = score.saturating_sub(10); issues.push("no lang".to_string()); }
        if !ctx.page.has_skip_link { score = score.saturating_sub(5); issues.push("no skip link".to_string()); }
        if !ctx.page.has_main_landmark { score = score.saturating_sub(10); issues.push("no main".to_string()); }
        if ctx.page.tables_total > 0 && ctx.page.tables_with_headers == 0 { score = score.saturating_sub(10); issues.push("tables no headers".to_string()); }
        let no_alt = ctx.page.images.iter().filter(|i| i.alt.is_empty() && !i.has_alt).count();
        if no_alt > 0 { score = score.saturating_sub((no_alt as u32 * 2).min(15)); issues.push(format!("{no_alt} images no alt")); }
        if ctx.page.has_positive_tabindex { score = score.saturating_sub(10); issues.push("positive tabindex".to_string()); }
        let rec = if score < 50 { "Significant accessibility issues.".to_string() } else { "Good accessibility.".to_string() };
        findings.push(Finding { severity: Severity::Info, category: IssueCategory::Accessibility, code: "A11YSC001".to_string(), title: "Accessibility compliance score".to_string(), description: format!("Score: {score}/100. Issues: {}.", issues.join(", ")), url: url.clone(), recommendation: rec });
        findings
    }
}

// =========================================================================
// Security V2 Analyzers
// =========================================================================

pub struct CspDirectiveAnalyzerV2;
impl Default for CspDirectiveAnalyzerV2 { fn default() -> Self { Self::new() } }
impl CspDirectiveAnalyzerV2 { pub fn new() -> Self { Self } }
impl Analyzer for CspDirectiveAnalyzerV2 {
    fn name(&self) -> &str { "csp-directive-analyzer-v2" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let csp = ctx.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case("Content-Security-Policy")).map(|(_, v)| v.as_str()).unwrap_or("");
        if csp.is_empty() { return findings; }
        let dirs: Vec<&str> = csp.split(';').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
        let names: Vec<&str> = dirs.iter().filter_map(|d| d.split_whitespace().next()).collect();
        if !names.contains(&"base-uri") { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Security, code: "CSPDIR-V2001".to_string(), title: "CSP missing base-uri".to_string(), description: "Without base-uri, attackers can inject <base> tags.".to_string(), url: url.clone(), recommendation: "Add base-uri 'self'.".to_string() }); }
        if !names.contains(&"form-action") { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Security, code: "CSPDIR-V2002".to_string(), title: "CSP missing form-action".to_string(), description: "Without form-action, forms could submit to attacker URLs.".to_string(), url: url.clone(), recommendation: "Add form-action 'self'.".to_string() }); }
        if !names.contains(&"frame-ancestors") { findings.push(Finding { severity: Severity::Info, category: IssueCategory::Security, code: "CSPDIR-V2003".to_string(), title: "CSP missing frame-ancestors".to_string(), description: "frame-ancestors is the modern replacement for X-Frame-Options.".to_string(), url: url.clone(), recommendation: "Add frame-ancestors 'none' or 'self'.".to_string() }); }
        if !names.contains(&"object-src") { findings.push(Finding { severity: Severity::Info, category: IssueCategory::Security, code: "CSPDIR-V2004".to_string(), title: "CSP missing object-src".to_string(), description: "Without object-src, plugins like Flash could load unchecked.".to_string(), url: url.clone(), recommendation: "Add object-src 'none'.".to_string() }); }
        findings
    }
}

pub struct CorsPolicyAnalyzerV2;
impl Default for CorsPolicyAnalyzerV2 { fn default() -> Self { Self::new() } }
impl CorsPolicyAnalyzerV2 { pub fn new() -> Self { Self } }
impl Analyzer for CorsPolicyAnalyzerV2 {
    fn name(&self) -> &str { "cors-policy-analyzer-v2" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let acao = ctx.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case("Access-Control-Allow-Origin")).map(|(_, v)| v.as_str());
        let acac = ctx.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case("Access-Control-Allow-Credentials")).map(|(_, v)| v.as_str());
        if let Some(origin) = acao {
            if origin == "*" && acac.map(|v| v.eq_ignore_ascii_case("true")).unwrap_or(false) {
                findings.push(Finding { severity: Severity::Critical, category: IssueCategory::Security, code: "CORS-V2001".to_string(), title: "CORS wildcard with credentials".to_string(), description: "Wildcard origin with credentials enabled.".to_string(), url: url.clone(), recommendation: "Use specific origin instead of '*'.".to_string() });
            }
            if origin != "*" && !url.starts_with(origin) {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Security, code: "CORS-V2003".to_string(), title: "CORS origin differs from page".to_string(), description: format!("CORS allows origin '{origin}'."), url: url.clone(), recommendation: "Verify this cross-origin allowance is intentional.".to_string() });
            }
        }
        findings
    }
}

pub struct CookieSecurityFlagAnalyzerV2;
impl Default for CookieSecurityFlagAnalyzerV2 { fn default() -> Self { Self::new() } }
impl CookieSecurityFlagAnalyzerV2 { pub fn new() -> Self { Self } }
impl Analyzer for CookieSecurityFlagAnalyzerV2 {
    fn name(&self) -> &str { "cookie-security-flag-analyzer-v2" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for (k, v) in ctx.headers {
            if !k.eq_ignore_ascii_case("set-cookie") { continue; }
            let lower = v.to_lowercase();
            let name = v.split('=').next().unwrap_or("cookie").trim().to_string();
            if lower.contains("secure") && lower.contains("httponly") && lower.contains("samesite") { continue; }
            if !lower.contains("secure") { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Security, code: "COOKIE-V2002".to_string(), title: format!("Cookie '{name}' missing Secure"), description: "Cookie transmitted over HTTP.".to_string(), url: url.clone(), recommendation: "Add Secure flag.".to_string() }); }
            if !lower.contains("httponly") { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Security, code: "COOKIE-V2003".to_string(), title: format!("Cookie '{name}' missing HttpOnly"), description: "Cookie accessible to JavaScript.".to_string(), url: url.clone(), recommendation: "Add HttpOnly flag.".to_string() }); }
            if !lower.contains("samesite") { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Security, code: "COOKIE-V2004".to_string(), title: format!("Cookie '{name}' missing SameSite"), description: "Without SameSite, cookie is vulnerable to CSRF.".to_string(), url: url.clone(), recommendation: "Add SameSite=Strict or Lax.".to_string() }); }
        }
        findings
    }
}

pub struct MixedContentDetectionAnalyzerV2;
impl Default for MixedContentDetectionAnalyzerV2 { fn default() -> Self { Self::new() } }
impl MixedContentDetectionAnalyzerV2 { pub fn new() -> Self { Self } }
impl Analyzer for MixedContentDetectionAnalyzerV2 {
    fn name(&self) -> &str { "mixed-content-detection-v2" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if !url.starts_with("https://") { return findings; }
        if let Some(body) = ctx.body {
            let lower = body.to_lowercase();
            let http_count = lower.matches("http://").count();
            if http_count > 5 { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Security, code: "MIXCONT-V2001".to_string(), title: format!("{http_count} HTTP references on HTTPS page"), description: "Mixed content degrades HTTPS security.".to_string(), url: url.clone(), recommendation: "Change all URLs to HTTPS.".to_string() }); }
            if !lower.contains("upgrade-insecure-requests") && http_count > 0 {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Security, code: "MIXCONT-V2005".to_string(), title: "No upgrade-insecure-requests CSP".to_string(), description: "CSP doesn't auto-upgrade mixed content.".to_string(), url: url.clone(), recommendation: "Add upgrade-insecure-requests to CSP.".to_string() });
            }
        }
        findings
    }
}

pub struct HstsPreloadReadinessAnalyzerV2;
impl Default for HstsPreloadReadinessAnalyzerV2 { fn default() -> Self { Self::new() } }
impl HstsPreloadReadinessAnalyzerV2 { pub fn new() -> Self { Self } }
impl Analyzer for HstsPreloadReadinessAnalyzerV2 {
    fn name(&self) -> &str { "hsts-preload-readiness-v2" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let hsts = ctx.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case("Strict-Transport-Security")).map(|(_, v)| v.as_str());
        let hsts = match hsts { Some(v) => v, None => return findings };
        let lower = hsts.to_lowercase();
        if !lower.contains("includesubdomains") { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Security, code: "HSTSPR-V2001".to_string(), title: "HSTS missing includeSubDomains".to_string(), description: "Required for preload list submission.".to_string(), url: url.clone(), recommendation: "Add includeSubDomains.".to_string() }); }
        if !lower.contains("preload") { findings.push(Finding { severity: Severity::Info, category: IssueCategory::Security, code: "HSTSPR-V2002".to_string(), title: "HSTS missing preload".to_string(), description: "Without preload, domain won't be in browser preload lists.".to_string(), url: url.clone(), recommendation: "Add preload directive.".to_string() }); }
        if let Some(pos) = lower.find("max-age=") {
            let after = &lower[pos + 8..];
            if let Ok(age) = after.chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse::<u64>() {
                if age < 31536000 { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Security, code: "HSTSPR-V2003".to_string(), title: "HSTS max-age below preload minimum".to_string(), description: format!("max-age is {age}, preload requires 31536000."), url: url.clone(), recommendation: "Set max-age to at least 31536000.".to_string() }); }
            }
        }
        findings
    }
}

pub struct XContentTypeOptionsDeepAnalyzerV2;
impl Default for XContentTypeOptionsDeepAnalyzerV2 { fn default() -> Self { Self::new() } }
impl XContentTypeOptionsDeepAnalyzerV2 { pub fn new() -> Self { Self } }
impl Analyzer for XContentTypeOptionsDeepAnalyzerV2 {
    fn name(&self) -> &str { "x-content-type-options-deep-v2" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let xcto = ctx.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case("X-Content-Type-Options")).map(|(_, v)| v.as_str());
        match xcto {
            None => { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Security, code: "XCTO-V2001".to_string(), title: "Missing X-Content-Type-Options".to_string(), description: "Without nosniff, browsers may MIME-sniff responses.".to_string(), url: url.clone(), recommendation: "Add X-Content-Type-Options: nosniff.".to_string() }); }
            Some(val) if !val.trim().eq_ignore_ascii_case("nosniff") => { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Security, code: "XCTO-V2002".to_string(), title: "Invalid X-Content-Type-Options".to_string(), description: format!("Value is \"{val}\", should be \"nosniff\"."), url: url.clone(), recommendation: "Set to nosniff.".to_string() }); }
            _ => {}
        }
        findings
    }
}

pub struct ReferrerPolicyDeepAnalyzerV2;
impl Default for ReferrerPolicyDeepAnalyzerV2 { fn default() -> Self { Self::new() } }
impl ReferrerPolicyDeepAnalyzerV2 { pub fn new() -> Self { Self } }
impl Analyzer for ReferrerPolicyDeepAnalyzerV2 {
    fn name(&self) -> &str { "referrer-policy-deep-v2" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let rp = ctx.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case("Referrer-Policy")).map(|(_, v)| v.as_str());
        match rp {
            None => { findings.push(Finding { severity: Severity::Info, category: IssueCategory::Security, code: "RPDEEP-V2001".to_string(), title: "Missing Referrer-Policy".to_string(), description: "Browser default may leak referrer info.".to_string(), url: url.clone(), recommendation: "Add Referrer-Policy: strict-origin-when-cross-origin.".to_string() }); }
            Some(val) if val.eq_ignore_ascii_case("unsafe-url") => { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Security, code: "RPDEEP-V2002".to_string(), title: "Referrer-Policy unsafe-url".to_string(), description: "Leaks full URL including path and query.".to_string(), url: url.clone(), recommendation: "Use strict-origin-when-cross-origin.".to_string() }); }
            _ => {}
        }
        findings
    }
}

pub struct XFrameOptionsDeepAnalyzerV2;
impl Default for XFrameOptionsDeepAnalyzerV2 { fn default() -> Self { Self::new() } }
impl XFrameOptionsDeepAnalyzerV2 { pub fn new() -> Self { Self } }
impl Analyzer for XFrameOptionsDeepAnalyzerV2 {
    fn name(&self) -> &str { "x-frame-options-deep-v2" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let xfo = ctx.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case("X-Frame-Options")).map(|(_, v)| v.as_str());
        let csp_frame = ctx.headers.iter().any(|(k, v)| k.eq_ignore_ascii_case("Content-Security-Policy") && v.contains("frame-ancestors"));
        if xfo.is_none() && !csp_frame {
            findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Security, code: "XFODEEP-V2001".to_string(), title: "No clickjacking protection".to_string(), description: "Neither X-Frame-Options nor CSP frame-ancestors set.".to_string(), url: url.clone(), recommendation: "Add X-Frame-Options: DENY or CSP frame-ancestors.".to_string() });
        }
        findings
    }
}

pub struct PermissionsPolicyDeepAnalyzerV2;
impl Default for PermissionsPolicyDeepAnalyzerV2 { fn default() -> Self { Self::new() } }
impl PermissionsPolicyDeepAnalyzerV2 { pub fn new() -> Self { Self } }
impl Analyzer for PermissionsPolicyDeepAnalyzerV2 {
    fn name(&self) -> &str { "permissions-policy-deep-v2" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let pp = ctx.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case("Permissions-Policy") || k.eq_ignore_ascii_case("Feature-Policy")).map(|(_, v)| v.as_str());
        if pp.is_none() {
            findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Security, code: "PERMP-V2001".to_string(), title: "Missing Permissions-Policy".to_string(), description: "Browsers may allow access to sensitive APIs.".to_string(), url: url.clone(), recommendation: "Add Permissions-Policy header.".to_string() });
        }
        findings
    }
}

pub struct CrossOriginIsolationDeepAnalyzerV2;
impl Default for CrossOriginIsolationDeepAnalyzerV2 { fn default() -> Self { Self::new() } }
impl CrossOriginIsolationDeepAnalyzerV2 { pub fn new() -> Self { Self } }
impl Analyzer for CrossOriginIsolationDeepAnalyzerV2 {
    fn name(&self) -> &str { "cross-origin-isolation-deep-v2" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let coep = ctx.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case("Cross-Origin-Embedder-Policy")).map(|(_, v)| v.as_str());
        let coop = ctx.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case("Cross-Origin-Opener-Policy")).map(|(_, v)| v.as_str());
        if coep.is_none() { findings.push(Finding { severity: Severity::Info, category: IssueCategory::Security, code: "COISO-V2001".to_string(), title: "Missing COEP".to_string(), description: "COEP prevents loading cross-origin resources without CORS.".to_string(), url: url.clone(), recommendation: "Add Cross-Origin-Embedder-Policy: require-corp.".to_string() }); }
        if coop.is_none() { findings.push(Finding { severity: Severity::Info, category: IssueCategory::Security, code: "COISO-V2003".to_string(), title: "Missing COOP".to_string(), description: "COOP controls cross-origin document references.".to_string(), url: url.clone(), recommendation: "Add Cross-Origin-Opener-Policy: same-origin.".to_string() }); }
        findings
    }
}

pub struct AriaLandmarksAnalyzerV2;
impl Default for AriaLandmarksAnalyzerV2 { fn default() -> Self { Self::new() } }
impl AriaLandmarksAnalyzerV2 { pub fn new() -> Self { Self } }
impl Analyzer for AriaLandmarksAnalyzerV2 {
    fn name(&self) -> &str { "aria-landmarks-v2" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.landmarks.is_empty() {
            findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Accessibility, code: "ARIALAND-V2006".to_string(), title: "No ARIA landmarks found".to_string(), description: "No ARIA landmarks on page.".to_string(), url: url.clone(), recommendation: "Add landmark roles (banner, navigation, main, contentinfo).".to_string() });
            return findings;
        }
        for &role in &["banner", "navigation", "main", "contentinfo"] {
            if !ctx.page.landmarks.iter().any(|l| l.to_lowercase() == role) {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Accessibility, code: format!("ARIALAND-V200{}", ["banner", "navigation", "main", "contentinfo"].iter().position(|&r| r == role).unwrap_or(0) + 1), title: format!("Missing {role} landmark"), description: format!("No ARIA landmark with role '{role}' found."), url: url.clone(), recommendation: format!("Add a <div role=\"{role}\"> or HTML5 element.") });
            }
        }
        let main_count = ctx.page.landmarks.iter().filter(|l| l.to_lowercase() == "main").count();
        if main_count > 1 { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Accessibility, code: "ARIALAND-V2005".to_string(), title: "Duplicate main landmarks".to_string(), description: format!("{main_count} main landmarks found. Only one is allowed."), url: url.clone(), recommendation: "Use a single main landmark.".to_string() }); }
        findings
    }
}

pub struct HeadingHierarchyDeepAnalyzerV2;
impl Default for HeadingHierarchyDeepAnalyzerV2 { fn default() -> Self { Self::new() } }
impl HeadingHierarchyDeepAnalyzerV2 { pub fn new() -> Self { Self } }
impl Analyzer for HeadingHierarchyDeepAnalyzerV2 {
    fn name(&self) -> &str { "heading-hierarchy-deep-v2" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.headings.is_empty() { return findings; }
        let mut prev_level = 0u8;
        let mut skip_count = 0;
        let mut empty_count = 0;
        for h in &ctx.page.headings {
            if h.text.trim().is_empty() { empty_count += 1; }
            if prev_level > 0 && h.level > prev_level + 1 { skip_count += 1; }
            prev_level = h.level;
        }
        if empty_count > 0 { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Accessibility, code: "HHIER-V2002".to_string(), title: "Empty headings found".to_string(), description: format!("{empty_count} heading(s) have no text."), url: url.clone(), recommendation: "Add meaningful text to all headings.".to_string() }); }
        if skip_count > 0 { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Accessibility, code: "HHIER-V2003".to_string(), title: "Heading levels skipped".to_string(), description: format!("{skip_count} heading level skip(s)."), url: url.clone(), recommendation: "Use heading levels sequentially.".to_string() }); }
        findings
    }
}

pub struct FormLabelsDeepAnalyzerV2;
impl Default for FormLabelsDeepAnalyzerV2 { fn default() -> Self { Self::new() } }
impl FormLabelsDeepAnalyzerV2 { pub fn new() -> Self { Self } }
impl Analyzer for FormLabelsDeepAnalyzerV2 {
    fn name(&self) -> &str { "form-labels-deep-v2" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.forms.is_empty() { return findings; }
        let mut unlabeled = 0;
        for form in &ctx.page.forms {
            for input in &form.inputs {
                let t = input.input_type.as_deref().unwrap_or("text");
                if matches!(t, "hidden" | "submit" | "button" | "image" | "reset") { continue; }
                if !input.has_label && input.aria_label.is_none() && input.aria_labelledby.is_none() && input.placeholder.is_none() { unlabeled += 1; }
            }
        }
        if unlabeled > 0 { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Accessibility, code: "FORMLBL-V2001".to_string(), title: "Form elements without labels".to_string(), description: format!("{unlabeled} input(s) lack labels."), url: url.clone(), recommendation: "Associate inputs with <label> elements.".to_string() }); }
        findings
    }
}

pub struct TableAccessibilityDeepAnalyzerV2;
impl Default for TableAccessibilityDeepAnalyzerV2 { fn default() -> Self { Self::new() } }
impl TableAccessibilityDeepAnalyzerV2 { pub fn new() -> Self { Self } }
impl Analyzer for TableAccessibilityDeepAnalyzerV2 {
    fn name(&self) -> &str { "table-accessibility-deep-v2" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.tables_total == 0 { return findings; }
        if ctx.page.tables_with_headers == 0 { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Accessibility, code: "TABACC-V2001".to_string(), title: "Tables without headers".to_string(), description: format!("{} table(s) lack <th> headers.", ctx.page.tables_total), url: url.clone(), recommendation: "Add <th> elements.".to_string() }); }
        if ctx.page.tables_with_captions == 0 { findings.push(Finding { severity: Severity::Info, category: IssueCategory::Accessibility, code: "TABACC-V2002".to_string(), title: "Tables without captions".to_string(), description: format!("{} table(s) lack <caption>.", ctx.page.tables_total), url: url.clone(), recommendation: "Add <caption> to each table.".to_string() }); }
        findings
    }
}

pub struct LinkTextQualityAnalyzerV2;
impl Default for LinkTextQualityAnalyzerV2 { fn default() -> Self { Self::new() } }
impl LinkTextQualityAnalyzerV2 { pub fn new() -> Self { Self } }
impl Analyzer for LinkTextQualityAnalyzerV2 {
    fn name(&self) -> &str { "link-text-quality-v2" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let generic = ["click here", "here", "read more", "more", "link", "learn more"];
        let mut generic_count = 0;
        let mut empty_count = 0;
        for link in &ctx.page.links {
            let text = link.text.trim().to_lowercase();
            if text.is_empty() && link.aria_label.is_none() { empty_count += 1; }
            if generic.contains(&text.as_str()) { generic_count += 1; }
        }
        if generic_count > 0 { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Accessibility, code: "LINKTQ-V2001".to_string(), title: "Generic link text".to_string(), description: format!("{generic_count} link(s) use generic text."), url: url.clone(), recommendation: "Use descriptive link text.".to_string() }); }
        if empty_count > 0 { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Accessibility, code: "LINKTQ-V2002".to_string(), title: "Empty link text".to_string(), description: format!("{empty_count} link(s) have no text or aria-label."), url: url.clone(), recommendation: "Add descriptive text or aria-label.".to_string() }); }
        findings
    }
}

pub struct ImageAltTextDeepAnalyzerV2;
impl Default for ImageAltTextDeepAnalyzerV2 { fn default() -> Self { Self::new() } }
impl ImageAltTextDeepAnalyzerV2 { pub fn new() -> Self { Self } }
impl Analyzer for ImageAltTextDeepAnalyzerV2 {
    fn name(&self) -> &str { "image-alt-text-deep-v2" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.images.is_empty() { return findings; }
        let mut missing = 0;
        let mut generic = 0;
        for img in &ctx.page.images {
            if !img.has_alt && img.alt.is_empty() { missing += 1; }
            else if ["image", "photo", "picture", "img"].contains(&img.alt.to_lowercase().as_str()) { generic += 1; }
        }
        if missing > 0 { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Accessibility, code: "IMGALT-V2001".to_string(), title: "Images missing alt".to_string(), description: format!("{missing} image(s) have no alt attribute."), url: url.clone(), recommendation: "Add alt attributes.".to_string() }); }
        if generic > 0 { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Accessibility, code: "IMGALT-V2003".to_string(), title: "Generic alt text".to_string(), description: format!("{generic} image(s) use generic alt text."), url: url.clone(), recommendation: "Use descriptive alt text.".to_string() }); }
        findings
    }
}

pub struct FocusManagementDeepAnalyzerV2;
impl Default for FocusManagementDeepAnalyzerV2 { fn default() -> Self { Self::new() } }
impl FocusManagementDeepAnalyzerV2 { pub fn new() -> Self { Self } }
impl Analyzer for FocusManagementDeepAnalyzerV2 {
    fn name(&self) -> &str { "focus-management-deep-v2" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.has_positive_tabindex { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Accessibility, code: "FOCUS-V2001".to_string(), title: "Positive tabindex found".to_string(), description: "Positive tabindex reorders tab sequence confusingly.".to_string(), url: url.clone(), recommendation: "Use tabindex=\"0\" or tabindex=\"-1\".".to_string() }); }
        findings
    }
}

pub struct LanguageAttributesDeepAnalyzerV2;
impl Default for LanguageAttributesDeepAnalyzerV2 { fn default() -> Self { Self::new() } }
impl LanguageAttributesDeepAnalyzerV2 { pub fn new() -> Self { Self } }
impl Analyzer for LanguageAttributesDeepAnalyzerV2 {
    fn name(&self) -> &str { "language-attributes-deep-v2" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if !ctx.page.has_lang_attribute { findings.push(Finding { severity: Severity::Error, category: IssueCategory::Accessibility, code: "LANGATTR-V2001".to_string(), title: "Missing html lang".to_string(), description: "Screen readers cannot determine page language.".to_string(), url: url.clone(), recommendation: "Add lang=\"en\" to <html>.".to_string() }); }
        findings
    }
}

pub struct ColorContrastTextAnalyzerV2;
impl Default for ColorContrastTextAnalyzerV2 { fn default() -> Self { Self::new() } }
impl ColorContrastTextAnalyzerV2 { pub fn new() -> Self { Self } }
impl Analyzer for ColorContrastTextAnalyzerV2 {
    fn name(&self) -> &str { "color-contrast-text-v2" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(body) = ctx.body {
            let lower = body.to_lowercase();
            let hidden = lower.matches("opacity:0").count() + lower.matches("visibility:hidden").count();
            if hidden > 0 { findings.push(Finding { severity: Severity::Info, category: IssueCategory::Accessibility, code: "COLRCT-V2003".to_string(), title: "Hidden text detected".to_string(), description: format!("{hidden} CSS rule(s) hide text."), url: url.clone(), recommendation: "Avoid hiding text with CSS.".to_string() }); }
        }
        findings
    }
}

pub struct ColorContrastLinkAnalyzerV2;
impl Default for ColorContrastLinkAnalyzerV2 { fn default() -> Self { Self::new() } }
impl ColorContrastLinkAnalyzerV2 { pub fn new() -> Self { Self } }
impl Analyzer for ColorContrastLinkAnalyzerV2 {
    fn name(&self) -> &str { "color-contrast-link-v2" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(body) = ctx.body {
            let lower = body.to_lowercase();
            if lower.contains("text-decoration: none") && lower.contains("color:") {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Accessibility, code: "COLRCL-V2001".to_string(), title: "Links without underline".to_string(), description: "Links may be indistinguishable from text.".to_string(), url: url.clone(), recommendation: "Provide non-color visual indicator for links.".to_string() });
            }
        }
        findings
    }
}

pub struct AnchorTextGenericAnalyzerV2;
impl Default for AnchorTextGenericAnalyzerV2 { fn default() -> Self { Self::new() } }
impl AnchorTextGenericAnalyzerV2 { pub fn new() -> Self { Self } }
impl Analyzer for AnchorTextGenericAnalyzerV2 {
    fn name(&self) -> &str { "anchor-text-generic-v2" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let generic = ["click here", "here", "read more", "more info", "learn more", "link"];
        let mut generic_count = 0;
        for link in &ctx.page.links {
            let text = link.text.trim().to_lowercase();
            if generic.contains(&text.as_str()) { generic_count += 1; }
        }
        if generic_count > 0 { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Seo, code: "ANCHGEN-V2001".to_string(), title: "Generic anchor text".to_string(), description: format!("{generic_count} link(s) use generic text."), url: url.clone(), recommendation: "Use descriptive, keyword-rich anchor text.".to_string() }); }
        findings
    }
}

pub struct TableCaptionPresenceAnalyzerV2;
impl Default for TableCaptionPresenceAnalyzerV2 { fn default() -> Self { Self::new() } }
impl TableCaptionPresenceAnalyzerV2 { pub fn new() -> Self { Self } }
impl Analyzer for TableCaptionPresenceAnalyzerV2 {
    fn name(&self) -> &str { "table-caption-presence-v2" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.tables_total > 0 && ctx.page.tables_with_captions == 0 {
            findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Accessibility, code: "TBLCAP-V2001".to_string(), title: "No table captions".to_string(), description: format!("{} table(s) lack <caption>.", ctx.page.tables_total), url: url.clone(), recommendation: "Add <caption> to every data table.".to_string() });
        }
        findings
    }
}

pub struct TableHeaderScopeAnalyzerV2;
impl Default for TableHeaderScopeAnalyzerV2 { fn default() -> Self { Self::new() } }
impl TableHeaderScopeAnalyzerV2 { pub fn new() -> Self { Self } }
impl Analyzer for TableHeaderScopeAnalyzerV2 {
    fn name(&self) -> &str { "table-header-scope-v2" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.tables_total == 0 || ctx.page.tables_with_headers == 0 { return findings; }
        if let Some(body) = ctx.body {
            let lower = body.to_lowercase();
            let th_count = lower.matches("<th").count();
            let th_with_scope = lower.matches("scope=\"").count();
            if th_count > 0 && th_with_scope == 0 {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Accessibility, code: "TBLSCOP-V2001".to_string(), title: "Table headers missing scope".to_string(), description: format!("{th_count} <th> elements without scope."), url: url.clone(), recommendation: "Add scope=\"col\" or scope=\"row\" to <th> elements.".to_string() });
            }
        }
        findings
    }
}

pub struct FormLabelAssociationAnalyzerV2;
impl Default for FormLabelAssociationAnalyzerV2 { fn default() -> Self { Self::new() } }
impl FormLabelAssociationAnalyzerV2 { pub fn new() -> Self { Self } }
impl Analyzer for FormLabelAssociationAnalyzerV2 {
    fn name(&self) -> &str { "form-label-association-v2" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.forms.is_empty() { return findings; }
        let mut dup_ids = 0;
        for form in &ctx.page.forms {
            let ids: std::collections::HashSet<&str> = form.inputs.iter().filter_map(|i| i.id.as_deref()).collect();
            let mut counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
            for id in &ids { *counts.entry(id).or_insert(0) += 1; }
            for &c in counts.values() { if c > 1 { dup_ids += 1; } }
        }
        if dup_ids > 0 { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Accessibility, code: "FORMLAB-V2001".to_string(), title: "Duplicate input IDs".to_string(), description: format!("{dup_ids} duplicate ID(s) found."), url: url.clone(), recommendation: "Ensure all input IDs are unique.".to_string() }); }
        findings
    }
}

pub struct AriaRequiredAttributesAnalyzerV2;
impl Default for AriaRequiredAttributesAnalyzerV2 { fn default() -> Self { Self::new() } }
impl AriaRequiredAttributesAnalyzerV2 { pub fn new() -> Self { Self } }
impl Analyzer for AriaRequiredAttributesAnalyzerV2 {
    fn name(&self) -> &str { "aria-required-attributes-v2" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(body) = ctx.body {
            let lower = body.to_lowercase();
            for role in &["progressbar", "slider", "combobox", "listbox", "tab"] {
                let role_pattern = format!("role=\"{role}\"");
                if let Some(pos) = lower.find(&role_pattern) {
                    let after = &lower[pos + role_pattern.len()..];
                    let segment = &after[..after.len().min(300)];
                    match *role {
                        "progressbar" => { if !segment.contains("aria-valuenow") { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Accessibility, code: "ARIAREQ-V2002".to_string(), title: "progressbar missing aria-valuenow".to_string(), description: "Required for assistive technologies.".to_string(), url: url.clone(), recommendation: "Add aria-valuenow.".to_string() }); } }
                        "slider" => { if !segment.contains("aria-valuenow") || !segment.contains("aria-valuemin") { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Accessibility, code: "ARIAREQ-V2003".to_string(), title: "slider missing required attributes".to_string(), description: "Requires aria-valuenow and aria-valuemin.".to_string(), url: url.clone(), recommendation: "Add aria-valuenow, aria-valuemin, aria-valuemax.".to_string() }); } }
                        "combobox" | "listbox" => { if !segment.contains("aria-expanded") { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Accessibility, code: "ARIAREQ-V2004".to_string(), title: format!("{role} missing aria-expanded"), description: "Should indicate expanded/collapsed state.".to_string(), url: url.clone(), recommendation: "Add aria-expanded.".to_string() }); } }
                        "tab" => { if !segment.contains("aria-selected") { findings.push(Finding { severity: Severity::Info, category: IssueCategory::Accessibility, code: "ARIAREQ-V2005".to_string(), title: "tab missing aria-selected".to_string(), description: "Should indicate active tab state.".to_string(), url: url.clone(), recommendation: "Add aria-selected.".to_string() }); } }
                        _ => {}
                    }
                }
            }
        }
        findings
    }
}

// =========================================================================
// SEO V2/V3/V4 Analyzers
// =========================================================================

pub struct TitleAnalysisDeepAnalyzerV2;
impl Default for TitleAnalysisDeepAnalyzerV2 { fn default() -> Self { Self::new() } }
impl TitleAnalysisDeepAnalyzerV2 { pub fn new() -> Self { Self } }
impl Analyzer for TitleAnalysisDeepAnalyzerV2 {
    fn name(&self) -> &str { "title-analysis-deep-v2" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let title = match &ctx.page.meta.title { Some(t) if !t.trim().is_empty() => t.trim(), _ => return findings };
        if title.len() < 20 { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Seo, code: "TITLEDEEP-V2001".to_string(), title: "Title critically short".to_string(), description: format!("{} chars, below 30-60 target.", title.len()), url: url.clone(), recommendation: "Expand to 30-60 characters.".to_string() }); }
        if title.len() > 70 { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Seo, code: "TITLEDEEP-V2002".to_string(), title: "Title excessively long".to_string(), description: format!("{} chars, truncates at ~60.", title.len()), url: url.clone(), recommendation: "Shorten to under 60 characters.".to_string() }); }
        if title.to_lowercase() == title || title.to_uppercase() == title { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Seo, code: "TITLEDEEP-V2003".to_string(), title: "Title all lowercase or uppercase".to_string(), description: "Affects readability and CTR.".to_string(), url: url.clone(), recommendation: "Use Title Case or sentence case.".to_string() }); }
        findings
    }
}

pub struct MetaDescriptionDeepAnalyzerV2;
impl Default for MetaDescriptionDeepAnalyzerV2 { fn default() -> Self { Self::new() } }
impl MetaDescriptionDeepAnalyzerV2 { pub fn new() -> Self { Self } }
impl Analyzer for MetaDescriptionDeepAnalyzerV2 {
    fn name(&self) -> &str { "meta-description-deep-v2" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let desc = match &ctx.page.meta.description { Some(d) if !d.trim().is_empty() => d.trim(), _ => return findings };
        if desc.len() < 70 { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Seo, code: "METADEEP-V2001".to_string(), title: "Description too short".to_string(), description: format!("{} chars, aim for 120-155.", desc.len()), url: url.clone(), recommendation: "Expand to 120-155 characters.".to_string() }); }
        if desc.len() > 160 { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Seo, code: "METADEEP-V2002".to_string(), title: "Description too long".to_string(), description: format!("{} chars, truncates at ~155.", desc.len()), url: url.clone(), recommendation: "Shorten to under 155 characters.".to_string() }); }
        findings
    }
}

pub struct CanonicalValidationDeepAnalyzerV2;
impl Default for CanonicalValidationDeepAnalyzerV2 { fn default() -> Self { Self::new() } }
impl CanonicalValidationDeepAnalyzerV2 { pub fn new() -> Self { Self } }
impl Analyzer for CanonicalValidationDeepAnalyzerV2 {
    fn name(&self) -> &str { "canonical-validation-deep-v2" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let canonical = match &ctx.page.meta.canonical { Some(c) => c, None => return findings };
        if canonical.as_str().contains('#') { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Seo, code: "CANDEEP-V2005".to_string(), title: "Canonical contains fragment".to_string(), description: "Fragments are ignored by search engines.".to_string(), url: url.clone(), recommendation: "Remove fragment from canonical URL.".to_string() }); }
        if canonical.as_str().contains('?') { findings.push(Finding { severity: Severity::Info, category: IssueCategory::Seo, code: "CANDEEP-V2006".to_string(), title: "Canonical has query params".to_string(), description: "Query params may cause indexing issues.".to_string(), url: url.clone(), recommendation: "Canonical URLs should be parameter-free.".to_string() }); }
        findings
    }
}

pub struct SitemapCoverageDeepAnalyzerV2;
impl Default for SitemapCoverageDeepAnalyzerV2 { fn default() -> Self { Self::new() } }
impl SitemapCoverageDeepAnalyzerV2 { pub fn new() -> Self { Self } }
impl Analyzer for SitemapCoverageDeepAnalyzerV2 {
    fn name(&self) -> &str { "sitemap-coverage-deep-v2" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(robots) = ctx.robots_txt {
            let lower = robots.to_lowercase();
            if !lower.contains("sitemap:") { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Seo, code: "SITEMAPDEEP-V2001".to_string(), title: "No sitemap in robots.txt".to_string(), description: "Search engines may miss pages.".to_string(), url: url.clone(), recommendation: "Add Sitemap: directive to robots.txt.".to_string() }); }
        }
        findings
    }
}

pub struct RobotsTxtAnalysisDeepAnalyzerV2;
impl Default for RobotsTxtAnalysisDeepAnalyzerV2 { fn default() -> Self { Self::new() } }
impl RobotsTxtAnalysisDeepAnalyzerV2 { pub fn new() -> Self { Self } }
impl Analyzer for RobotsTxtAnalysisDeepAnalyzerV2 {
    fn name(&self) -> &str { "robots-txt-analysis-deep-v2" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let robots = match ctx.robots_txt { Some(r) => r, None => return findings };
        let lower = robots.to_lowercase();
        if lower.contains("disallow: /") {
            for line in robots.lines() {
                let t = line.trim();
                if t.to_lowercase().starts_with("disallow:") {
                    let path = t.split(':').nth(1).unwrap_or("").trim();
                    if path == "/" { findings.push(Finding { severity: Severity::Critical, category: IssueCategory::Seo, code: "ROBOTSDEEP-V2001".to_string(), title: "robots.txt blocks all".to_string(), description: "Blanket Disallow: / blocks all crawlers.".to_string(), url: url.clone(), recommendation: "Remove blanket disallow.".to_string() }); break; }
                }
            }
        }
        findings
    }
}

pub struct InternalLinkQualityAnalyzerV2;
impl Default for InternalLinkQualityAnalyzerV2 { fn default() -> Self { Self::new() } }
impl InternalLinkQualityAnalyzerV2 { pub fn new() -> Self { Self } }
impl Analyzer for InternalLinkQualityAnalyzerV2 {
    fn name(&self) -> &str { "internal-link-quality-v2" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let internal: Vec<_> = ctx.page.links.iter().filter(|l| !l.is_external).collect();
        if internal.is_empty() { return findings; }
        let self_links = internal.iter().filter(|l| l.href == *url).count();
        if self_links > 0 { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Seo, code: "INTLINKQ-V2001".to_string(), title: "Self-referencing links".to_string(), description: format!("{self_links} link(s) point to same page."), url: url.clone(), recommendation: "Remove self-referencing links.".to_string() }); }
        let nofollow = internal.iter().filter(|l| l.rel.iter().any(|r| r == "nofollow")).count();
        if nofollow == internal.len() && internal.len() > 1 { findings.push(Finding { severity: Severity::Critical, category: IssueCategory::Seo, code: "INTLINKQ-V2002".to_string(), title: "All internal links nofollowed".to_string(), description: "Blocks all internal PageRank flow.".to_string(), url: url.clone(), recommendation: "Remove nofollow from internal links.".to_string() }); }
        findings
    }
}

pub struct ExternalLinkAuthorityDeepAnalyzerV2;
impl Default for ExternalLinkAuthorityDeepAnalyzerV2 { fn default() -> Self { Self::new() } }
impl ExternalLinkAuthorityDeepAnalyzerV2 { pub fn new() -> Self { Self } }
impl Analyzer for ExternalLinkAuthorityDeepAnalyzerV2 {
    fn name(&self) -> &str { "external-link-authority-deep-v2" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let external: Vec<_> = ctx.page.links.iter().filter(|l| l.is_external).collect();
        if external.is_empty() { return findings; }
        let suspicious_tlds = [".ru", ".cn", ".tk", ".ml", ".xyz", ".top"];
        let mut susp = 0;
        for link in &external {
            let h = link.href.to_lowercase();
            if suspicious_tlds.iter().any(|t| h.contains(t)) { susp += 1; }
        }
        if susp > 0 { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Seo, code: "EXTLINKAUTH-V2001".to_string(), title: "Links to suspicious TLDs".to_string(), description: format!("{susp} external link(s) to suspicious domains."), url: url.clone(), recommendation: "Review these links.".to_string() }); }
        findings
    }
}

pub struct TitleLengthQualityAnalyzerV2;
impl Default for TitleLengthQualityAnalyzerV2 { fn default() -> Self { Self::new() } }
impl TitleLengthQualityAnalyzerV2 { pub fn new() -> Self { Self } }
impl Analyzer for TitleLengthQualityAnalyzerV2 {
    fn name(&self) -> &str { "title-length-quality-v2" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let title = match &ctx.page.meta.title { Some(t) if !t.trim().is_empty() => t.trim(), _ => return findings };
        let len = title.len();
        if len < 20 { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Seo, code: "TITLEQLT-V2001".to_string(), title: "Title too short".to_string(), description: format!("{len} chars, wastes SERP space."), url: url.clone(), recommendation: "Expand to 30-60 characters.".to_string() }); }
        else if len > 60 { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Seo, code: "TITLEQLT-V2002".to_string(), title: "Title may truncate".to_string(), description: format!("{len} chars, Google shows ~60."), url: url.clone(), recommendation: "Keep important keywords within 60 chars.".to_string() }); }
        findings
    }
}

pub struct MetaDescriptionQualityAnalyzerV2;
impl Default for MetaDescriptionQualityAnalyzerV2 { fn default() -> Self { Self::new() } }
impl MetaDescriptionQualityAnalyzerV2 { pub fn new() -> Self { Self } }
impl Analyzer for MetaDescriptionQualityAnalyzerV2 {
    fn name(&self) -> &str { "meta-description-quality-v2" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let desc = match &ctx.page.meta.description { Some(d) if !d.trim().is_empty() => d.trim(), _ => return findings };
        let len = desc.len();
        if len < 50 { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Seo, code: "METAQLT-V2001".to_string(), title: "Description too short".to_string(), description: format!("{len} chars, aim for 120-155."), url: url.clone(), recommendation: "Expand to 120-155 characters.".to_string() }); }
        else if len > 155 { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Seo, code: "METAQLT-V2002".to_string(), title: "Description may truncate".to_string(), description: format!("{len} chars, Google shows ~155."), url: url.clone(), recommendation: "Shorten to under 155 characters.".to_string() }); }
        findings
    }
}

pub struct InternalLinkAnchorAnalyzerV3;
impl Default for InternalLinkAnchorAnalyzerV3 { fn default() -> Self { Self::new() } }
impl InternalLinkAnchorAnalyzerV3 { pub fn new() -> Self { Self } }
impl Analyzer for InternalLinkAnchorAnalyzerV3 {
    fn name(&self) -> &str { "internal-link-anchor-v3" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let internal: Vec<_> = ctx.page.links.iter().filter(|l| !l.is_external).collect();
        if internal.is_empty() { return findings; }
        let generic = ["click here", "here", "read more", "more", "link", "learn more"];
        let generic_count = internal.iter().filter(|l| generic.contains(&l.text.trim().to_lowercase().as_str())).count();
        if generic_count > 0 { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Seo, code: "ANCH-V3001".to_string(), title: "Generic internal anchor text".to_string(), description: format!("{generic_count} link(s) use generic text."), url: url.clone(), recommendation: "Use keyword-rich anchor text.".to_string() }); }
        findings
    }
}

pub struct WikipediaLinkAnalyzerV3;
impl Default for WikipediaLinkAnalyzerV3 { fn default() -> Self { Self::new() } }
impl WikipediaLinkAnalyzerV3 { pub fn new() -> Self { Self } }
impl Analyzer for WikipediaLinkAnalyzerV3 {
    fn name(&self) -> &str { "wikipedia-link-v3" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let wiki_count = ctx.page.links.iter().filter(|l| l.href.contains("wikipedia.org")).count();
        if wiki_count > 0 { findings.push(Finding { severity: Severity::Info, category: IssueCategory::Seo, code: "WIKI-V3001".to_string(), title: "Wikipedia links found".to_string(), description: format!("{wiki_count} Wikipedia link(s). These are nofollowed."), url: url.clone(), recommendation: "Wikipedia links won't pass link equity.".to_string() }); }
        findings
    }
}

pub struct PaginationDepthAnalyzerV2;
impl Default for PaginationDepthAnalyzerV2 { fn default() -> Self { Self::new() } }
impl PaginationDepthAnalyzerV2 { pub fn new() -> Self { Self } }
impl Analyzer for PaginationDepthAnalyzerV2 {
    fn name(&self) -> &str { "pagination-depth-v2" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(body) = ctx.body {
            let lower = body.to_lowercase();
            let has_pagination = lower.contains("page=") || lower.contains("p=") || lower.contains("start=");
            if has_pagination {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Seo, code: "PAGDEP-V2001".to_string(), title: "Paginated URL detected".to_string(), description: "URL appears to be paginated.".to_string(), url: url.clone(), recommendation: "Consider rel=next/prev for paginated content.".to_string() });
            }
        }
        findings
    }
}

pub struct MixedProtocolRedirectValidatorV2;
impl Default for MixedProtocolRedirectValidatorV2 { fn default() -> Self { Self::new() } }
impl MixedProtocolRedirectValidatorV2 { pub fn new() -> Self { Self } }
impl Analyzer for MixedProtocolRedirectValidatorV2 {
    fn name(&self) -> &str { "mixed-protocol-redirect-v2" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.redirect_chain.is_empty() { return findings; }
        let mut https_to_http = false;
        for hop in ctx.redirect_chain {
            let from_https = hop.from.as_str().starts_with("https://");
            let to_https = hop.to.as_str().starts_with("https://");
            if from_https && !to_https { https_to_http = true; }
        }
        if https_to_http { findings.push(Finding { severity: Severity::Critical, category: IssueCategory::Seo, code: "MIXPROT-V2002".to_string(), title: "HTTPS redirecting to HTTP".to_string(), description: "Security downgrade in redirect chain.".to_string(), url: url.clone(), recommendation: "Fix redirect chain to maintain HTTPS.".to_string() }); }
        findings
    }
}

pub struct InternalNofollowOveruseValidatorV2;
impl Default for InternalNofollowOveruseValidatorV2 { fn default() -> Self { Self::new() } }
impl InternalNofollowOveruseValidatorV2 { pub fn new() -> Self { Self } }
impl Analyzer for InternalNofollowOveruseValidatorV2 {
    fn name(&self) -> &str { "internal-nofollow-overuse-v2" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let internal: Vec<_> = ctx.page.links.iter().filter(|l| !l.is_external).collect();
        if internal.is_empty() { return findings; }
        let nofollow = internal.iter().filter(|l| l.rel.iter().any(|r| r == "nofollow")).count();
        let ratio = nofollow as f64 / internal.len() as f64;
        if ratio > 0.8 && internal.len() > 3 { findings.push(Finding { severity: Severity::Critical, category: IssueCategory::Seo, code: "NOFOLLOW-V2001".to_string(), title: "Extreme nofollow overuse".to_string(), description: format!("{nofollow}/{} ({:.0}%) nofollowed.", internal.len(), ratio * 100.0), url: url.clone(), recommendation: "Remove nofollow from most internal links.".to_string() }); }
        else if ratio > 0.5 && internal.len() > 5 { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Seo, code: "NOFOLLOW-V2002".to_string(), title: "High nofollow ratio".to_string(), description: format!("{nofollow}/{} ({:.0}%) nofollowed.", internal.len(), ratio * 100.0), url: url.clone(), recommendation: "Review nofollow usage.".to_string() }); }
        findings
    }
}

pub struct SitemapXmlSizeValidatorV2;
impl Default for SitemapXmlSizeValidatorV2 { fn default() -> Self { Self::new() } }
impl SitemapXmlSizeValidatorV2 { pub fn new() -> Self { Self } }
impl Analyzer for SitemapXmlSizeValidatorV2 {
    fn name(&self) -> &str { "sitemap-xml-size-v2" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(body) = ctx.body {
            if body.contains("<urlset") || body.contains("<sitemapindex") {
                let size_kb = body.len() / 1024;
                if size_kb > 50000 { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Seo, code: "SITEMAPSIZE-V2001".to_string(), title: "Sitemap very large".to_string(), description: format!("{size_kb} KB. Too large to download efficiently."), url: url.clone(), recommendation: "Split into multiple sitemaps.".to_string() }); }
                let url_count = body.matches("<url>").count();
                if url_count > 50000 { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Seo, code: "SITEMAPSIZE-V2003".to_string(), title: "Sitemap exceeds URL limit".to_string(), description: format!("{url_count} URLs, max is 50000."), url: url.clone(), recommendation: "Split into multiple sitemaps.".to_string() }); }
            }
        }
        findings
    }
}

pub struct RobotsTxtSizeValidatorV2;
impl Default for RobotsTxtSizeValidatorV2 { fn default() -> Self { Self::new() } }
impl RobotsTxtSizeValidatorV2 { pub fn new() -> Self { Self } }
impl Analyzer for RobotsTxtSizeValidatorV2 {
    fn name(&self) -> &str { "robots-txt-size-v2" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let robots = match ctx.robots_txt { Some(r) => r, None => return findings };
        let size_kb = robots.len() / 1024;
        if size_kb > 500 { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Seo, code: "ROBOTSSIZE-V2001".to_string(), title: "robots.txt very large".to_string(), description: format!("{size_kb} KB. Slows down crawlers."), url: url.clone(), recommendation: "Simplify rules.".to_string() }); }
        findings
    }
}

pub struct HreflangSelfReferenceValidatorV2;
impl Default for HreflangSelfReferenceValidatorV2 { fn default() -> Self { Self::new() } }
impl HreflangSelfReferenceValidatorV2 { pub fn new() -> Self { Self } }
impl Analyzer for HreflangSelfReferenceValidatorV2 {
    fn name(&self) -> &str { "hreflang-self-reference-v2" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let tags = &ctx.page.meta.hreflang;
        if tags.is_empty() { return findings; }
        let has_self = tags.iter().any(|t| t.url.as_str() == *url);
        if !has_self { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Seo, code: "HREFSELF-V2001".to_string(), title: "Missing hreflang self-reference".to_string(), description: "No self-referencing hreflang tag.".to_string(), url: url.clone(), recommendation: "Add self-referencing hreflang tag.".to_string() }); }
        let mut dup = std::collections::HashMap::new();
        for t in tags { *dup.entry(t.lang.as_str()).or_insert(0) += 1; }
        for (lang, count) in &dup {
            if *count > 1 { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Seo, code: "HREFSELF-V2002".to_string(), title: format!("Duplicate hreflang '{lang}'"), description: format!("'{lang}' declared {count} times."), url: url.clone(), recommendation: "Remove duplicate hreflang entries.".to_string() }); }
        }
        findings
    }
}

pub struct OpenSearchDescriptionValidatorV2;
impl Default for OpenSearchDescriptionValidatorV2 { fn default() -> Self { Self::new() } }
impl OpenSearchDescriptionValidatorV2 { pub fn new() -> Self { Self } }
impl Analyzer for OpenSearchDescriptionValidatorV2 {
    fn name(&self) -> &str { "opensearch-description-v2" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(body) = ctx.body {
            let lower = body.to_lowercase();
            if lower.contains("opensearchdescription") {
                // OpenSearch reference exists on page
            } else {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Seo, code: "OPDESC-V2001".to_string(), title: "No OpenSearch description".to_string(), description: "No OpenSearch link tag found.".to_string(), url: url.clone(), recommendation: "Add OpenSearch description link tag.".to_string() });
            }
        }
        findings
    }
}

pub struct CanonicalDepthAnalyzerV2;
impl Default for CanonicalDepthAnalyzerV2 { fn default() -> Self { Self::new() } }
impl CanonicalDepthAnalyzerV2 { pub fn new() -> Self { Self } }
impl Analyzer for CanonicalDepthAnalyzerV2 {
    fn name(&self) -> &str { "canonical-depth-v2" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(canonical) = &ctx.page.meta.canonical {
            if canonical.as_str().ends_with('/') && canonical.as_str() != "/" && !canonical.as_str().contains("index") {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Seo, code: "CANDEP-V2003".to_string(), title: "Canonical has trailing slash".to_string(), description: "Trailing slashes should match preferred format.".to_string(), url: url.clone(), recommendation: "Ensure canonical URL format is consistent.".to_string() });
            }
        }
        findings
    }
}

pub struct MetaDescriptionLengthAnalyzerV3;
impl Default for MetaDescriptionLengthAnalyzerV3 { fn default() -> Self { Self::new() } }
impl MetaDescriptionLengthAnalyzerV3 { pub fn new() -> Self { Self } }
impl Analyzer for MetaDescriptionLengthAnalyzerV3 {
    fn name(&self) -> &str { "meta-description-length-v3" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        match &ctx.page.meta.description {
            None => { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Seo, code: "META-V3001".to_string(), title: "Missing meta description".to_string(), description: "No meta description found.".to_string(), url: url.clone(), recommendation: "Write a 120-155 character meta description.".to_string() }); }
            Some(d) if d.trim().len() < 50 => { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Seo, code: "META-V3002".to_string(), title: "Description very short".to_string(), description: format!("{} chars.", d.trim().len()), url: url.clone(), recommendation: "Expand to 120-155 characters.".to_string() }); }
            Some(d) if d.len() > 155 => { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Seo, code: "META-V3003".to_string(), title: "Description may truncate".to_string(), description: format!("{} chars.", d.len()), url: url.clone(), recommendation: "Shorten to under 155 characters.".to_string() }); }
            _ => {}
        }
        findings
    }
}

pub struct TitleAnalyzerV4;
impl Default for TitleAnalyzerV4 { fn default() -> Self { Self::new() } }
impl TitleAnalyzerV4 { pub fn new() -> Self { Self } }
impl Analyzer for TitleAnalyzerV4 {
    fn name(&self) -> &str { "title-v4" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        match &ctx.page.meta.title {
            None => { findings.push(Finding { severity: Severity::Error, category: IssueCategory::Seo, code: "TITLE-V4001".to_string(), title: "Missing title tag".to_string(), description: "No <title> tag found.".to_string(), url: url.clone(), recommendation: "Add a unique 30-60 character <title>.".to_string() }); }
            Some(t) => {
                let len = t.len();
                if len < 20 { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Seo, code: "TITLE-V4002".to_string(), title: "Title too short".to_string(), description: format!("{len} chars."), url: url.clone(), recommendation: "Expand to 30-60 characters.".to_string() }); }
                if len > 65 { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Seo, code: "TITLE-V4003".to_string(), title: "Title may truncate".to_string(), description: format!("{len} chars."), url: url.clone(), recommendation: "Shorten to 60 characters.".to_string() }); }
                if t.to_uppercase() == *t { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Seo, code: "TITLE-V4004".to_string(), title: "ALL CAPS title".to_string(), description: "Looks spammy.".to_string(), url: url.clone(), recommendation: "Use Title Case.".to_string() }); }
            }
        }
        findings
    }
}

pub struct CanonicalUrlAnalyzerV3;
impl Default for CanonicalUrlAnalyzerV3 { fn default() -> Self { Self::new() } }
impl CanonicalUrlAnalyzerV3 { pub fn new() -> Self { Self } }
impl Analyzer for CanonicalUrlAnalyzerV3 {
    fn name(&self) -> &str { "canonical-url-v3" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        match &ctx.page.meta.canonical {
            None => { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Seo, code: "CAN-V3001".to_string(), title: "Missing canonical URL".to_string(), description: "No canonical tag found.".to_string(), url: url.clone(), recommendation: "Add a canonical URL tag.".to_string() }); }
            Some(c) => {
                if c.as_str().is_empty() { findings.push(Finding { severity: Severity::Error, category: IssueCategory::Seo, code: "CAN-V3002".to_string(), title: "Empty canonical URL".to_string(), description: "Canonical has empty href.".to_string(), url: url.clone(), recommendation: "Set canonical URL or remove tag.".to_string() }); }
                else if c.as_str().contains('#') { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Seo, code: "CAN-V3005".to_string(), title: "Canonical has fragment".to_string(), description: "Fragments are ignored.".to_string(), url: url.clone(), recommendation: "Remove fragment from canonical.".to_string() }); }
            }
        }
        findings
    }
}

pub struct HreflangValidatorV4;
impl Default for HreflangValidatorV4 { fn default() -> Self { Self::new() } }
impl HreflangValidatorV4 { pub fn new() -> Self { Self } }
impl Analyzer for HreflangValidatorV4 {
    fn name(&self) -> &str { "hreflang-v4" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let tags = &ctx.page.meta.hreflang;
        if tags.is_empty() { return findings; }
        let has_xd = tags.iter().any(|t| t.lang == "x-default");
        if !has_xd { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Seo, code: "HREF-V4001".to_string(), title: "Missing x-default".to_string(), description: "No x-default hreflang tag.".to_string(), url: url.clone(), recommendation: "Add x-default hreflang tag.".to_string() }); }
        let mut seen = std::collections::HashSet::new();
        let mut dup = 0;
        for t in tags { if !seen.insert((&t.lang, t.url.as_str())) { dup += 1; } }
        if dup > 0 { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Seo, code: "HREF-V4003".to_string(), title: "Duplicate hreflang entries".to_string(), description: format!("{dup} duplicate(s)."), url: url.clone(), recommendation: "Remove duplicate hreflang declarations.".to_string() }); }
        findings
    }
}

pub struct SitemapAnalyzerV3;
impl Default for SitemapAnalyzerV3 { fn default() -> Self { Self::new() } }
impl SitemapAnalyzerV3 { pub fn new() -> Self { Self } }
impl Analyzer for SitemapAnalyzerV3 {
    fn name(&self) -> &str { "sitemap-v3" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.robots_txt.is_none() { findings.push(Finding { severity: Severity::Info, category: IssueCategory::Seo, code: "SITEMAP-V3001".to_string(), title: "No robots.txt".to_string(), description: "No robots.txt available.".to_string(), url: url.clone(), recommendation: "Create a robots.txt file.".to_string() }); }
        findings
    }
}

pub struct RobotsTxtAnalyzerV3;
impl Default for RobotsTxtAnalyzerV3 { fn default() -> Self { Self::new() } }
impl RobotsTxtAnalyzerV3 { pub fn new() -> Self { Self } }
impl Analyzer for RobotsTxtAnalyzerV3 {
    fn name(&self) -> &str { "robots-txt-v3" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let robots = match ctx.robots_txt { Some(r) => r, None => { findings.push(Finding { severity: Severity::Info, category: IssueCategory::Seo, code: "ROBOTS-V3001".to_string(), title: "No robots.txt".to_string(), description: "No robots.txt available.".to_string(), url: url.clone(), recommendation: "Create a robots.txt.".to_string() }); return findings; } };
        let lower = robots.to_lowercase();
        if lower.contains("disallow: /") {
            for line in robots.lines() {
                let t = line.trim();
                if t.to_lowercase().starts_with("disallow:") && t.split(':').nth(1).unwrap_or("").trim() == "/" {
                    findings.push(Finding { severity: Severity::Critical, category: IssueCategory::Seo, code: "ROBOTS-V3002".to_string(), title: "robots.txt blocks all".to_string(), description: "Blanket Disallow: /.".to_string(), url: url.clone(), recommendation: "Remove blanket disallow.".to_string() });
                    break;
                }
            }
        }
        findings
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod v2_analyzer_tests {
    use super::*;
    use crate::meta::MetaTags;
    use crate::parser::{StructuredData, Heading};

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

    fn make_ctx<'a>(page: &'a crate::parser::ParsedPage, body: Option<&'a str>) -> AnalysisContext<'a> {
        AnalysisContext { page, body, status_code: Some(200), headers: &[], response_time: None, redirect_chain: &[], robots_txt: None, body_size: None, compressed_size: None, server: None, content_type: None, rendered: None }
    }

    #[test]
    fn test_duplicate_content_v2_empty() { assert!(DuplicateContentDetectorV2::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_duplicate_content_v2_short() { assert!(DuplicateContentDetectorV2::new().analyze(&make_ctx(&make_page("https://example.com"), Some("short body"))).is_empty()); }
    #[test]
    fn test_freshness_score_no_data() { let p = make_page("https://example.com"); assert!(!ContentFreshnessScoreAnalyzer::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_freshness_score_with_date() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".to_string()), r#type: Some("Article".to_string()), data: serde_json::json!({"datePublished": "2020-01-01"}) }]; let ctx = make_ctx(&p, None); let f = ContentFreshnessScoreAnalyzer::new().analyze(&ctx); assert!(f.iter().any(|x| x.code == "FRESHSC002")); }
    #[test]
    fn test_heading_structure_no_headings() { let p = make_page("https://example.com"); assert!(!HeadingStructureScoreAnalyzer::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_heading_structure_with_h1() { let mut p = make_page("https://example.com");         p.headings = vec![Heading { level: 1, text: "Title".to_string(), length: 5 }]; assert!(HeadingStructureScoreAnalyzer::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_link_quality_empty() { let p = make_page("https://example.com"); assert!(!LinkQualityScoreAnalyzer::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_schema_coverage_empty() { let p = make_page("https://example.com"); assert!(!SchemaCoverageScoreAnalyzer::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_security_score() { let p = make_page("https://example.com"); let f = SecurityScoreAnalyzer::new().analyze(&make_ctx(&p, None)); assert_eq!(f.len(), 1); assert_eq!(f[0].code, "SECSC001"); }
    #[test]
    fn test_accessibility_score() { let p = make_page("https://example.com"); let f = AccessibilityScoreAnalyzer::new().analyze(&make_ctx(&p, None)); assert_eq!(f.len(), 1); assert_eq!(f[0].code, "A11YSC001"); }
    #[test]
    fn test_csp_v2_empty() { assert!(CspDirectiveAnalyzerV2::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_cors_v2() { assert!(CorsPolicyAnalyzerV2::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_cookie_v2() { assert!(CookieSecurityFlagAnalyzerV2::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_mixed_content_v2() { assert!(MixedContentDetectionAnalyzerV2::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_hsts_v2() { assert!(HstsPreloadReadinessAnalyzerV2::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_xcto_v2() { let f = XContentTypeOptionsDeepAnalyzerV2::new().analyze(&make_ctx(&make_page("https://example.com"), None)); assert_eq!(f.len(), 1); assert_eq!(f[0].code, "XCTO-V2001"); }
    #[test]
    fn test_rp_v2() { let f = ReferrerPolicyDeepAnalyzerV2::new().analyze(&make_ctx(&make_page("https://example.com"), None)); assert_eq!(f.len(), 1); assert_eq!(f[0].code, "RPDEEP-V2001"); }
    #[test]
    fn test_xfo_v2() { let f = XFrameOptionsDeepAnalyzerV2::new().analyze(&make_ctx(&make_page("https://example.com"), None)); assert_eq!(f.len(), 1); assert_eq!(f[0].code, "XFODEEP-V2001"); }
    #[test]
    fn test_pp_v2() { let f = PermissionsPolicyDeepAnalyzerV2::new().analyze(&make_ctx(&make_page("https://example.com"), None)); assert_eq!(f.len(), 1); assert_eq!(f[0].code, "PERMP-V2001"); }
    #[test]
    fn test_coi_v2() { let f = CrossOriginIsolationDeepAnalyzerV2::new().analyze(&make_ctx(&make_page("https://example.com"), None)); assert_eq!(f.len(), 2); }
    #[test]
    fn test_aria_landmarks_v2_empty() { let p = make_page("https://example.com"); let f = AriaLandmarksAnalyzerV2::new().analyze(&make_ctx(&p, None)); assert_eq!(f.len(), 1); assert_eq!(f[0].code, "ARIALAND-V2006"); }
    #[test]
    fn test_heading_deep_v2() { assert!(HeadingHierarchyDeepAnalyzerV2::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_form_labels_v2() { assert!(FormLabelsDeepAnalyzerV2::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_table_acc_v2() { assert!(TableAccessibilityDeepAnalyzerV2::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_link_text_v2() { assert!(LinkTextQualityAnalyzerV2::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_img_alt_v2() { assert!(ImageAltTextDeepAnalyzerV2::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_focus_v2() { assert!(FocusManagementDeepAnalyzerV2::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_lang_v2() { let p = make_page("https://example.com"); let f = LanguageAttributesDeepAnalyzerV2::new().analyze(&make_ctx(&p, None)); assert_eq!(f.len(), 1); }
    #[test]
    fn test_color_text_v2() { assert!(ColorContrastTextAnalyzerV2::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_color_link_v2() { assert!(ColorContrastLinkAnalyzerV2::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_anchor_v2() { assert!(AnchorTextGenericAnalyzerV2::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_table_caption_v2() { assert!(TableCaptionPresenceAnalyzerV2::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_table_scope_v2() { assert!(TableHeaderScopeAnalyzerV2::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_form_label_v2() { assert!(FormLabelAssociationAnalyzerV2::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_aria_req_v2() { assert!(AriaRequiredAttributesAnalyzerV2::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_title_deep_v2() { assert!(TitleAnalysisDeepAnalyzerV2::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_meta_deep_v2() { assert!(MetaDescriptionDeepAnalyzerV2::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_canonical_deep_v2() { assert!(CanonicalValidationDeepAnalyzerV2::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_sitemap_deep_v2() { assert!(SitemapCoverageDeepAnalyzerV2::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_robots_deep_v2() { assert!(RobotsTxtAnalysisDeepAnalyzerV2::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_int_link_v2() { assert!(InternalLinkQualityAnalyzerV2::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_ext_link_v2() { assert!(ExternalLinkAuthorityDeepAnalyzerV2::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_title_len_v2() { assert!(TitleLengthQualityAnalyzerV2::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_meta_desc_v2() { assert!(MetaDescriptionQualityAnalyzerV2::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_anchor_v3() { assert!(InternalLinkAnchorAnalyzerV3::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_wiki_v3() { assert!(WikipediaLinkAnalyzerV3::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_pagination_v2() { assert!(PaginationDepthAnalyzerV2::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_mixed_proto_v2() { assert!(MixedProtocolRedirectValidatorV2::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_nofollow_v2() { assert!(InternalNofollowOveruseValidatorV2::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_sitemap_size_v2() { assert!(SitemapXmlSizeValidatorV2::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_robots_size_v2() { assert!(RobotsTxtSizeValidatorV2::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_hreflang_self_v2() { assert!(HreflangSelfReferenceValidatorV2::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_opensearch_v2() { assert!(OpenSearchDescriptionValidatorV2::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_canonical_depth_v2() { assert!(CanonicalDepthAnalyzerV2::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_meta_len_v3() { let f = MetaDescriptionLengthAnalyzerV3::new().analyze(&make_ctx(&make_page("https://example.com"), None)); assert_eq!(f.len(), 1); assert_eq!(f[0].code, "META-V3001"); }
    #[test]
    fn test_title_v4() { let f = TitleAnalyzerV4::new().analyze(&make_ctx(&make_page("https://example.com"), None)); assert_eq!(f.len(), 1); assert_eq!(f[0].code, "TITLE-V4001"); }
    #[test]
    fn test_canonical_v3() { let f = CanonicalUrlAnalyzerV3::new().analyze(&make_ctx(&make_page("https://example.com"), None)); assert_eq!(f.len(), 1); assert_eq!(f[0].code, "CAN-V3001"); }
    #[test]
    fn test_hreflang_v4() { assert!(HreflangValidatorV4::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_sitemap_v3() { let f = SitemapAnalyzerV3::new().analyze(&make_ctx(&make_page("https://example.com"), None)); assert_eq!(f.len(), 1); }
    #[test]
    fn test_robots_v3() { let f = RobotsTxtAnalyzerV3::new().analyze(&make_ctx(&make_page("https://example.com"), None)); assert_eq!(f.len(), 1); }
}
