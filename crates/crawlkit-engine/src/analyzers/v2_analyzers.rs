#![allow(clippy::unwrap_used, clippy::manual_range_contains, clippy::redundant_closure, clippy::collapsible_if, clippy::unnecessary_map_or, clippy::default_constructed_unit_structs, clippy::needless_return, clippy::needless_range_loop, clippy::useless_format, clippy::if_same_then_else, clippy::derivable_impls, clippy::manual_pattern_char_comparison, clippy::manual_contains, clippy::collapsible_match, clippy::redundant_clone, clippy::useless_conversion)]
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

// =========================================================================
// Content V5 Analyzers (1-30)
// =========================================================================

pub struct ArticleWordCountAnalyzer;
impl Default for ArticleWordCountAnalyzer { fn default() -> Self { Self::new() } }
impl ArticleWordCountAnalyzer { pub fn new() -> Self { Self } }
impl Analyzer for ArticleWordCountAnalyzer {
    fn name(&self) -> &str { "article-word-count-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let is_article = ctx.page.structured_data.iter().any(|sd| sd.r#type.as_deref() == Some("Article") || sd.r#type.as_deref() == Some("BlogPosting"));
        if !is_article { return findings; }
        if ctx.page.word_count < 300 {
            findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Content, code: "ARTWC-V5001".to_string(), title: "Article too short".to_string(), description: format!("{} words, recommended 300+.", ctx.page.word_count), url: url.clone(), recommendation: "Expand article to 300+ words.".to_string() });
        }
        findings
    }
}

pub struct ArticleAuthorUrlValidator;
impl Default for ArticleAuthorUrlValidator { fn default() -> Self { Self::new() } }
impl ArticleAuthorUrlValidator { pub fn new() -> Self { Self } }
impl Analyzer for ArticleAuthorUrlValidator {
    fn name(&self) -> &str { "article-author-url-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Article" && t != "BlogPosting" && t != "NewsArticle" { continue; }
            if let Some(author) = sd.data.get("author") {
                if author.is_string() { continue; }
                if let Some(obj) = author.as_object() {
                    if !obj.contains_key("url") && !obj.contains_key("@id") {
                        findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Content, code: "ARTAUTH-V5001".to_string(), title: "Author missing URL".to_string(), description: "Author object has no url or @id.".to_string(), url: url.clone(), recommendation: "Add url to author object.".to_string() });
                    }
                }
            }
        }
        findings
    }
}

pub struct ArticleDateModifiedValidator;
impl Default for ArticleDateModifiedValidator { fn default() -> Self { Self::new() } }
impl ArticleDateModifiedValidator { pub fn new() -> Self { Self } }
impl Analyzer for ArticleDateModifiedValidator {
    fn name(&self) -> &str { "article-date-modified-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Article" && t != "BlogPosting" && t != "NewsArticle" { continue; }
            if sd.data.get("dateModified").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Content, code: "ARTMOD-V5001".to_string(), title: "Article missing dateModified".to_string(), description: "No dateModified in structured data.".to_string(), url: url.clone(), recommendation: "Add dateModified to indicate freshness.".to_string() });
            }
        }
        findings
    }
}

pub struct OrganizationUrlValidatorV5;
impl Default for OrganizationUrlValidatorV5 { fn default() -> Self { Self::new() } }
impl OrganizationUrlValidatorV5 { pub fn new() -> Self { Self } }
impl Analyzer for OrganizationUrlValidatorV5 {
    fn name(&self) -> &str { "organization-url-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Organization") && sd.r#type.as_deref() != Some("LocalBusiness") { continue; }
            if let Some(org) = sd.data.as_object() {
                if !org.contains_key("url") && !org.contains_key("@id") {
                    findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "ORGURL-V5001".to_string(), title: "Organization missing URL".to_string(), description: "Organization has no url or @id.".to_string(), url: url.clone(), recommendation: "Add url to Organization schema.".to_string() });
                }
            }
        }
        findings
    }
}

pub struct OrganizationLogoUrlValidator;
impl Default for OrganizationLogoUrlValidator { fn default() -> Self { Self::new() } }
impl OrganizationLogoUrlValidator { pub fn new() -> Self { Self } }
impl Analyzer for OrganizationLogoUrlValidator {
    fn name(&self) -> &str { "organization-logo-url-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Organization") { continue; }
            if let Some(logo) = sd.data.get("logo") {
                if logo.is_string() { continue; }
                if let Some(obj) = logo.as_object() {
                    if obj.get("url").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
                        findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "ORGLOGO-V5001".to_string(), title: "Organization logo missing URL".to_string(), description: "Logo ImageObject has no url.".to_string(), url: url.clone(), recommendation: "Add url to logo ImageObject.".to_string() });
                    }
                }
            }
        }
        findings
    }
}

pub struct OrganizationContactValidator;
impl Default for OrganizationContactValidator { fn default() -> Self { Self::new() } }
impl OrganizationContactValidator { pub fn new() -> Self { Self } }
impl Analyzer for OrganizationContactValidator {
    fn name(&self) -> &str { "organization-contact-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Organization") { continue; }
            if sd.data.get("contactPoint").is_none() {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Schema, code: "ORGCONT-V5001".to_string(), title: "Organization missing contactPoint".to_string(), description: "No contactPoint in Organization.".to_string(), url: url.clone(), recommendation: "Add contactPoint with contactType.".to_string() });
            }
        }
        findings
    }
}

pub struct PersonUrlValidator;
impl Default for PersonUrlValidator { fn default() -> Self { Self::new() } }
impl PersonUrlValidator { pub fn new() -> Self { Self } }
impl Analyzer for PersonUrlValidator {
    fn name(&self) -> &str { "person-url-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Person") { continue; }
            if sd.data.get("url").is_none() {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Schema, code: "PERSURL-V5001".to_string(), title: "Person missing url".to_string(), description: "Person schema has no url.".to_string(), url: url.clone(), recommendation: "Add url to Person schema.".to_string() });
            }
        }
        findings
    }
}

pub struct JobPostingEmploymentTypeValidator;
impl Default for JobPostingEmploymentTypeValidator { fn default() -> Self { Self::new() } }
impl JobPostingEmploymentTypeValidator { pub fn new() -> Self { Self } }
impl Analyzer for JobPostingEmploymentTypeValidator {
    fn name(&self) -> &str { "job-posting-employment-type-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let valid_types = ["FULL_TIME", "PART_TIME", "CONTRACT", "TEMPORARY", "INTERN", "VOLUNTEER", "PER_DIEM", "OTHER"];
        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("JobPosting") { continue; }
            if let Some(et) = sd.data.get("employmentType").and_then(|v| v.as_str()) {
                if !valid_types.contains(&et) {
                    findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "JOBEMP-V5001".to_string(), title: "Invalid employmentType".to_string(), description: format!("'{et}' is not a valid Schema.org employmentType."), url: url.clone(), recommendation: "Use one of: FULL_TIME, PART_TIME, CONTRACT, TEMPORARY, INTERN, VOLUNTEER, PER_DIEM, OTHER.".to_string() });
                }
            }
        }
        findings
    }
}

pub struct JobPostingSalaryCurrencyValidator;
impl Default for JobPostingSalaryCurrencyValidator { fn default() -> Self { Self::new() } }
impl JobPostingSalaryCurrencyValidator { pub fn new() -> Self { Self } }
impl Analyzer for JobPostingSalaryCurrencyValidator {
    fn name(&self) -> &str { "job-posting-salary-currency-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("JobPosting") { continue; }
            if let Some(salary) = sd.data.get("baseSalary") {
                if let Some(obj) = salary.as_object() {
                    if let Some(currency) = obj.get("currency") {
                        if let Some(c) = currency.as_str() {
                            if c.len() != 3 || !c.chars().all(|ch| ch.is_ascii_uppercase()) {
                                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "JOBCURR-V5001".to_string(), title: "Invalid salary currency".to_string(), description: format!("'{c}' is not a valid ISO 4217 code."), url: url.clone(), recommendation: "Use a 3-letter ISO 4217 currency code.".to_string() });
                            }
                        }
                    }
                }
            }
        }
        findings
    }
}

pub struct JobPostingValidThroughValidator;
impl Default for JobPostingValidThroughValidator { fn default() -> Self { Self::new() } }
impl JobPostingValidThroughValidator { pub fn new() -> Self { Self } }
impl Analyzer for JobPostingValidThroughValidator {
    fn name(&self) -> &str { "job-posting-valid-through-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("JobPosting") { continue; }
            if sd.data.get("validThrough").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "JOBVT-V5001".to_string(), title: "JobPosting missing validThrough".to_string(), description: "No validThrough date.".to_string(), url: url.clone(), recommendation: "Add validThrough to indicate expiration.".to_string() });
            }
        }
        findings
    }
}

pub struct CourseDescriptionValidator;
impl Default for CourseDescriptionValidator { fn default() -> Self { Self::new() } }
impl CourseDescriptionValidator { pub fn new() -> Self { Self } }
impl Analyzer for CourseDescriptionValidator {
    fn name(&self) -> &str { "course-description-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Course") { continue; }
            if sd.data.get("description").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "COURSEDESC-V5001".to_string(), title: "Course missing description".to_string(), description: "No description in Course schema.".to_string(), url: url.clone(), recommendation: "Add a description to the Course.".to_string() });
            }
        }
        findings
    }
}

pub struct CourseProviderNameValidator;
impl Default for CourseProviderNameValidator { fn default() -> Self { Self::new() } }
impl CourseProviderNameValidator { pub fn new() -> Self { Self } }
impl Analyzer for CourseProviderNameValidator {
    fn name(&self) -> &str { "course-provider-name-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Course") { continue; }
            if let Some(provider) = sd.data.get("provider") {
                if let Some(obj) = provider.as_object() {
                    if obj.get("name").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
                        findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "COURSEPV-V5001".to_string(), title: "Course provider missing name".to_string(), description: "Provider object has no name.".to_string(), url: url.clone(), recommendation: "Add name to provider.".to_string() });
                    }
                }
            }
        }
        findings
    }
}

pub struct RecipePrepTimeValidator;
impl Default for RecipePrepTimeValidator { fn default() -> Self { Self::new() } }
impl RecipePrepTimeValidator { pub fn new() -> Self { Self } }
impl Analyzer for RecipePrepTimeValidator {
    fn name(&self) -> &str { "recipe-prep-time-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Recipe") { continue; }
            if sd.data.get("prepTime").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Schema, code: "RECIPEPT-V5001".to_string(), title: "Recipe missing prepTime".to_string(), description: "No prepTime in Recipe schema.".to_string(), url: url.clone(), recommendation: "Add prepTime in ISO 8601 format.".to_string() });
            }
        }
        findings
    }
}

pub struct RecipeIngredientsValidator;
impl Default for RecipeIngredientsValidator { fn default() -> Self { Self::new() } }
impl RecipeIngredientsValidator { pub fn new() -> Self { Self } }
impl Analyzer for RecipeIngredientsValidator {
    fn name(&self) -> &str { "recipe-ingredients-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Recipe") { continue; }
            if let Some(ingredients) = sd.data.get("recipeIngredient") {
                if let Some(arr) = ingredients.as_array() {
                    if arr.is_empty() {
                        findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "RECIPEING-V5001".to_string(), title: "Recipe has empty ingredients".to_string(), description: "recipeIngredient array is empty.".to_string(), url: url.clone(), recommendation: "List all recipe ingredients.".to_string() });
                    }
                }
            } else {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "RECIPEING-V5002".to_string(), title: "Recipe missing ingredients".to_string(), description: "No recipeIngredient field.".to_string(), url: url.clone(), recommendation: "Add recipeIngredient array.".to_string() });
            }
        }
        findings
    }
}

pub struct ProductPriceValidator;
impl Default for ProductPriceValidator { fn default() -> Self { Self::new() } }
impl ProductPriceValidator { pub fn new() -> Self { Self } }
impl Analyzer for ProductPriceValidator {
    fn name(&self) -> &str { "product-price-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Product") { continue; }
            if let Some(offers) = sd.data.get("offers") {
                let check_offer = |offer: &serde_json::Value| {
                    if let Some(obj) = offer.as_object() {
                        if obj.get("price").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty())
                            && obj.get("price").and_then(|v| v.as_f64()).is_none() {
                            return true;
                        }
                        if obj.get("priceCurrency").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
                            return true;
                        }
                    }
                    false
                };
                if offers.is_object() && check_offer(offers) {
                    findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "PRODPRICE-V5001".to_string(), title: "Product offer missing price/currency".to_string(), description: "Offer lacks price or priceCurrency.".to_string(), url: url.clone(), recommendation: "Add price and priceCurrency to offer.".to_string() });
                }
            }
        }
        findings
    }
}

pub struct ProductImageValidatorV5;
impl Default for ProductImageValidatorV5 { fn default() -> Self { Self::new() } }
impl ProductImageValidatorV5 { pub fn new() -> Self { Self } }
impl Analyzer for ProductImageValidatorV5 {
    fn name(&self) -> &str { "product-image-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Product") { continue; }
            if sd.data.get("image").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
                if let Some(img) = sd.data.get("image") {
                    if img.is_array() && img.as_array().map_or(true, |a| a.is_empty()) {
                        findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "PRODIMG-V5001".to_string(), title: "Product has empty image array".to_string(), description: "image array is empty.".to_string(), url: url.clone(), recommendation: "Add product images.".to_string() });
                    }
                } else {
                    findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "PRODIMG-V5002".to_string(), title: "Product missing image".to_string(), description: "No image in Product schema.".to_string(), url: url.clone(), recommendation: "Add an image to the Product.".to_string() });
                }
            }
        }
        findings
    }
}

pub struct BreadcrumbItemCountValidator;
impl Default for BreadcrumbItemCountValidator { fn default() -> Self { Self::new() } }
impl BreadcrumbItemCountValidator { pub fn new() -> Self { Self } }
impl Analyzer for BreadcrumbItemCountValidator {
    fn name(&self) -> &str { "breadcrumb-item-count-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("BreadcrumbList") { continue; }
            if let Some(items) = sd.data.get("itemListElement") {
                if let Some(arr) = items.as_array() {
                    if arr.len() < 2 {
                        findings.push(Finding { severity: Severity::Info, category: IssueCategory::Schema, code: "BREADCRITM-V5001".to_string(), title: "Breadcrumb has too few items".to_string(), description: format!("{} item(s), recommend 2+.", arr.len()), url: url.clone(), recommendation: "Add more breadcrumb levels.".to_string() });
                    }
                }
            }
        }
        findings
    }
}

pub struct BreadcrumbUrlValidator;
impl Default for BreadcrumbUrlValidator { fn default() -> Self { Self::new() } }
impl BreadcrumbUrlValidator { pub fn new() -> Self { Self } }
impl Analyzer for BreadcrumbUrlValidator {
    fn name(&self) -> &str { "breadcrumb-url-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("BreadcrumbList") { continue; }
            if let Some(items) = sd.data.get("itemListElement") {
                if let Some(arr) = items.as_array() {
                    for item in arr {
                        if let Some(obj) = item.as_object() {
                            if let Some(item_url) = obj.get("item").and_then(|v| v.get("@id").or(Some(v))).and_then(|v| v.as_str()) {
                                if !item_url.starts_with("http://") && !item_url.starts_with("https://") {
                                    findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "BREADURL-V5001".to_string(), title: "Breadcrumb has relative URL".to_string(), description: format!("URL '{item_url}' is not absolute."), url: url.clone(), recommendation: "Use absolute URLs in breadcrumbs.".to_string() });
                                }
                            }
                        }
                    }
                }
            }
        }
        findings
    }
}

pub struct EventOrganizerValidatorV5;
impl Default for EventOrganizerValidatorV5 { fn default() -> Self { Self::new() } }
impl EventOrganizerValidatorV5 { pub fn new() -> Self { Self } }
impl Analyzer for EventOrganizerValidatorV5 {
    fn name(&self) -> &str { "event-organizer-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Event") { continue; }
            if let Some(organizer) = sd.data.get("organizer") {
                if let Some(obj) = organizer.as_object() {
                    if obj.get("name").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
                        findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "EVTORG-V5001".to_string(), title: "Event organizer missing name".to_string(), description: "Organizer has no name.".to_string(), url: url.clone(), recommendation: "Add name to organizer.".to_string() });
                    }
                }
            } else {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "EVTORG-V5002".to_string(), title: "Event missing organizer".to_string(), description: "No organizer in Event schema.".to_string(), url: url.clone(), recommendation: "Add an organizer to the Event.".to_string() });
            }
        }
        findings
    }
}

pub struct EventPerformerValidator;
impl Default for EventPerformerValidator { fn default() -> Self { Self::new() } }
impl EventPerformerValidator { pub fn new() -> Self { Self } }
impl Analyzer for EventPerformerValidator {
    fn name(&self) -> &str { "event-performer-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Event") { continue; }
            if sd.data.get("performer").is_none() {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Schema, code: "EVTPERF-V5001".to_string(), title: "Event missing performer".to_string(), description: "No performer in Event schema.".to_string(), url: url.clone(), recommendation: "Add performer information.".to_string() });
            }
        }
        findings
    }
}

pub struct VideoThumbnailValidator;
impl Default for VideoThumbnailValidator { fn default() -> Self { Self::new() } }
impl VideoThumbnailValidator { pub fn new() -> Self { Self } }
impl Analyzer for VideoThumbnailValidator {
    fn name(&self) -> &str { "video-thumbnail-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "VideoObject" { continue; }
            if sd.data.get("thumbnailUrl").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "VIDTHUMB-V5001".to_string(), title: "Video missing thumbnailUrl".to_string(), description: "No thumbnailUrl in VideoObject.".to_string(), url: url.clone(), recommendation: "Add thumbnailUrl for video preview.".to_string() });
            }
        }
        findings
    }
}

pub struct VideoDurationFormatValidator;
impl Default for VideoDurationFormatValidator { fn default() -> Self { Self::new() } }
impl VideoDurationFormatValidator { pub fn new() -> Self { Self } }
impl Analyzer for VideoDurationFormatValidator {
    fn name(&self) -> &str { "video-duration-format-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "VideoObject" { continue; }
            if let Some(dur) = sd.data.get("duration").and_then(|v| v.as_str()) {
                if !dur.starts_with("PT") && !dur.starts_with("P") {
                    findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "VIDDURFMT-V5001".to_string(), title: "Video duration not ISO 8601".to_string(), description: format!("'{dur}' is not ISO 8601 duration format."), url: url.clone(), recommendation: "Use ISO 8601 duration (e.g., PT1H30M).".to_string() });
                }
            }
        }
        findings
    }
}

pub struct SoftwareOffersValidatorV5;
impl Default for SoftwareOffersValidatorV5 { fn default() -> Self { Self::new() } }
impl SoftwareOffersValidatorV5 { pub fn new() -> Self { Self } }
impl Analyzer for SoftwareOffersValidatorV5 {
    fn name(&self) -> &str { "software-offers-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "SoftwareApplication" { continue; }
            if sd.data.get("offers").is_none() {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "SOFTOFF-V5001".to_string(), title: "Software missing offers".to_string(), description: "No offers in SoftwareApplication.".to_string(), url: url.clone(), recommendation: "Add offers with price information.".to_string() });
            }
        }
        findings
    }
}

pub struct SoftwareScreenshotValidator;
impl Default for SoftwareScreenshotValidator { fn default() -> Self { Self::new() } }
impl SoftwareScreenshotValidator { pub fn new() -> Self { Self } }
impl Analyzer for SoftwareScreenshotValidator {
    fn name(&self) -> &str { "software-screenshot-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "SoftwareApplication" { continue; }
            if sd.data.get("screenshot").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Schema, code: "SOFTSS-V5001".to_string(), title: "Software missing screenshot".to_string(), description: "No screenshot in SoftwareApplication.".to_string(), url: url.clone(), recommendation: "Add a screenshot URL.".to_string() });
            }
        }
        findings
    }
}

pub struct FAQAnswerLengthValidator;
impl Default for FAQAnswerLengthValidator { fn default() -> Self { Self::new() } }
impl FAQAnswerLengthValidator { pub fn new() -> Self { Self } }
impl Analyzer for FAQAnswerLengthValidator {
    fn name(&self) -> &str { "faq-answer-length-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "FAQPage" { continue; }
            if let Some(entities) = sd.data.get("mainEntity").and_then(|v| v.as_array()) {
                let mut short_answers = 0;
                for entity in entities {
                    if let Some(answer) = entity.get("acceptedAnswer") {
                        if let Some(text) = answer.get("text").and_then(|v| v.as_str()) {
                            if text.len() < 50 { short_answers += 1; }
                        }
                    }
                }
                if short_answers > 0 {
                    findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Content, code: "FAQANSLEN-V5001".to_string(), title: "FAQ answers too short".to_string(), description: format!("{short_answers} answer(s) under 50 chars."), url: url.clone(), recommendation: "Provide substantive FAQ answers (50+ chars).".to_string() });
                }
            }
        }
        findings
    }
}

pub struct HowToNameValidator;
impl Default for HowToNameValidator { fn default() -> Self { Self::new() } }
impl HowToNameValidator { pub fn new() -> Self { Self } }
impl Analyzer for HowToNameValidator {
    fn name(&self) -> &str { "howto-name-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "HowTo" { continue; }
            if sd.data.get("name").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "HOWTONAME-V5001".to_string(), title: "HowTo missing name".to_string(), description: "No name in HowTo schema.".to_string(), url: url.clone(), recommendation: "Add a descriptive name to HowTo.".to_string() });
            }
        }
        findings
    }
}

pub struct HowToStepDescriptionValidator;
impl Default for HowToStepDescriptionValidator { fn default() -> Self { Self::new() } }
impl HowToStepDescriptionValidator { pub fn new() -> Self { Self } }
impl Analyzer for HowToStepDescriptionValidator {
    fn name(&self) -> &str { "howto-step-desc-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "HowTo" { continue; }
            if let Some(steps) = sd.data.get("step") {
                let step_list = if steps.is_array() { steps.as_array().unwrap() } else { std::slice::from_ref(steps) };
                let mut missing_desc = 0;
                for step in step_list {
                    if step.get("text").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty())
                        && step.get("name").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
                        missing_desc += 1;
                    }
                }
                if missing_desc > 0 {
                    findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "HOWTOSTEP-V5001".to_string(), title: "HowTo steps missing description".to_string(), description: format!("{missing_desc} step(s) have no text or name."), url: url.clone(), recommendation: "Add descriptive text to each step.".to_string() });
                }
            }
        }
        findings
    }
}

pub struct DatasetLicenseValidator;
impl Default for DatasetLicenseValidator { fn default() -> Self { Self::new() } }
impl DatasetLicenseValidator { pub fn new() -> Self { Self } }
impl Analyzer for DatasetLicenseValidator {
    fn name(&self) -> &str { "dataset-license-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Dataset" { continue; }
            if sd.data.get("license").is_none() {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Schema, code: "DSLICENSE-V5001".to_string(), title: "Dataset missing license".to_string(), description: "No license in Dataset schema.".to_string(), url: url.clone(), recommendation: "Add license information.".to_string() });
            }
        }
        findings
    }
}

pub struct DatasetDistributionValidator;
impl Default for DatasetDistributionValidator { fn default() -> Self { Self::new() } }
impl DatasetDistributionValidator { pub fn new() -> Self { Self } }
impl Analyzer for DatasetDistributionValidator {
    fn name(&self) -> &str { "dataset-distribution-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Dataset" { continue; }
            if let Some(dist) = sd.data.get("distribution") {
                if let Some(arr) = dist.as_array() {
                    for d in arr {
                        if let Some(obj) = d.as_object() {
                            if obj.get("contentUrl").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
                                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "DSDIST-V5001".to_string(), title: "Dataset distribution missing contentUrl".to_string(), description: "Distribution has no contentUrl.".to_string(), url: url.clone(), recommendation: "Add contentUrl to distribution.".to_string() });
                            }
                        }
                    }
                }
            }
        }
        findings
    }
}

// =========================================================================
// Security V5 Analyzers (31-50)
// =========================================================================

pub struct CspScriptSrcValidator;
impl Default for CspScriptSrcValidator { fn default() -> Self { Self::new() } }
impl CspScriptSrcValidator { pub fn new() -> Self { Self } }
impl Analyzer for CspScriptSrcValidator {
    fn name(&self) -> &str { "csp-script-src-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let csp = ctx.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case("Content-Security-Policy")).map(|(_, v)| v.as_str()).unwrap_or("");
        if csp.is_empty() { return findings; }
        if let Some(directive) = csp.split(';').find(|d| d.trim().starts_with("script-src")) {
            let value = directive.trim().trim_start_matches("script-src").trim();
            if value.contains("'unsafe-inline'") {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Security, code: "CSPSSRC-V5001".to_string(), title: "CSP script-src allows unsafe-inline".to_string(), description: "unsafe-inline weakens CSP.".to_string(), url: url.clone(), recommendation: "Remove unsafe-inline and use nonces or hashes.".to_string() });
            }
            if value.contains("'unsafe-eval'") {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Security, code: "CSPSSRC-V5002".to_string(), title: "CSP script-src allows unsafe-eval".to_string(), description: "unsafe-eval allows eval().".to_string(), url: url.clone(), recommendation: "Remove unsafe-eval.".to_string() });
            }
            if value.contains("*") && !value.contains("'none'") {
                findings.push(Finding { severity: Severity::Critical, category: IssueCategory::Security, code: "CSPSSRC-V5003".to_string(), title: "CSP script-src wildcard".to_string(), description: "Wildcard allows any script source.".to_string(), url: url.clone(), recommendation: "Restrict script-src to specific origins.".to_string() });
            }
        } else {
            findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Security, code: "CSPSSRC-V5004".to_string(), title: "CSP missing script-src".to_string(), description: "No script-src directive.".to_string(), url: url.clone(), recommendation: "Add script-src directive.".to_string() });
        }
        findings
    }
}

pub struct CspStyleSrcValidator;
impl Default for CspStyleSrcValidator { fn default() -> Self { Self::new() } }
impl CspStyleSrcValidator { pub fn new() -> Self { Self } }
impl Analyzer for CspStyleSrcValidator {
    fn name(&self) -> &str { "csp-style-src-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let csp = ctx.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case("Content-Security-Policy")).map(|(_, v)| v.as_str()).unwrap_or("");
        if csp.is_empty() { return findings; }
        if let Some(directive) = csp.split(';').find(|d| d.trim().starts_with("style-src")) {
            let value = directive.trim().trim_start_matches("style-src").trim();
            if value.contains("'unsafe-inline'") {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Security, code: "CSPSTYLE-V5001".to_string(), title: "CSP style-src allows unsafe-inline".to_string(), description: "unsafe-inline for styles is common but weakens CSP.".to_string(), url: url.clone(), recommendation: "Consider using nonces or hashes for styles.".to_string() });
            }
        } else {
            findings.push(Finding { severity: Severity::Info, category: IssueCategory::Security, code: "CSPSTYLE-V5002".to_string(), title: "CSP missing style-src".to_string(), description: "No style-src directive.".to_string(), url: url.clone(), recommendation: "Add style-src directive.".to_string() });
        }
        findings
    }
}

pub struct CspFrameAncestorsValidator;
impl Default for CspFrameAncestorsValidator { fn default() -> Self { Self::new() } }
impl CspFrameAncestorsValidator { pub fn new() -> Self { Self } }
impl Analyzer for CspFrameAncestorsValidator {
    fn name(&self) -> &str { "csp-frame-ancestors-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let csp = ctx.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case("Content-Security-Policy")).map(|(_, v)| v.as_str()).unwrap_or("");
        if csp.is_empty() { return findings; }
        if !csp.split(';').any(|d| d.trim().starts_with("frame-ancestors")) {
            findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Security, code: "CSPFRAME-V5001".to_string(), title: "CSP missing frame-ancestors".to_string(), description: "frame-ancestors prevents clickjacking.".to_string(), url: url.clone(), recommendation: "Add frame-ancestors 'none' or 'self'.".to_string() });
        }
        findings
    }
}

pub struct HstsMaxAgeValidator;
impl Default for HstsMaxAgeValidator { fn default() -> Self { Self::new() } }
impl HstsMaxAgeValidator { pub fn new() -> Self { Self } }
impl Analyzer for HstsMaxAgeValidator {
    fn name(&self) -> &str { "hsts-max-age-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let hsts = ctx.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case("Strict-Transport-Security")).map(|(_, v)| v.as_str());
        match hsts {
            None => { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Security, code: "HSTSMAX-V5001".to_string(), title: "Missing HSTS header".to_string(), description: "No Strict-Transport-Security.".to_string(), url: url.clone(), recommendation: "Add HSTS with max-age >= 31536000.".to_string() }); }
            Some(val) => {
                let lower = val.to_lowercase();
                if let Some(pos) = lower.find("max-age=") {
                    let after = &lower[pos + 8..];
                    if let Ok(age) = after.chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse::<u64>() {
                        if age < 31536000 {
                            findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Security, code: "HSTSMAX-V5002".to_string(), title: "HSTS max-age too low".to_string(), description: format!("max-age is {age}, recommend 31536000+."), url: url.clone(), recommendation: "Set max-age to at least 31536000.".to_string() });
                        }
                    }
                } else {
                    findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Security, code: "HSTSMAX-V5003".to_string(), title: "HSTS missing max-age".to_string(), description: "HSTS header has no max-age.".to_string(), url: url.clone(), recommendation: "Add max-age directive.".to_string() });
                }
            }
        }
        findings
    }
}

pub struct HstsIncludeSubDomainsValidator;
impl Default for HstsIncludeSubDomainsValidator { fn default() -> Self { Self::new() } }
impl HstsIncludeSubDomainsValidator { pub fn new() -> Self { Self } }
impl Analyzer for HstsIncludeSubDomainsValidator {
    fn name(&self) -> &str { "hsts-include-subdomains-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(val) = ctx.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case("Strict-Transport-Security")).map(|(_, v)| v.as_str()) {
            if !val.to_lowercase().contains("includesubdomains") {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Security, code: "HSTSSUB-V5001".to_string(), title: "HSTS missing includeSubDomains".to_string(), description: "Subdomains not covered by HSTS.".to_string(), url: url.clone(), recommendation: "Add includeSubDomains directive.".to_string() });
            }
        }
        findings
    }
}

pub struct HstsPreloadValidatorV5;
impl Default for HstsPreloadValidatorV5 { fn default() -> Self { Self::new() } }
impl HstsPreloadValidatorV5 { pub fn new() -> Self { Self } }
impl Analyzer for HstsPreloadValidatorV5 {
    fn name(&self) -> &str { "hsts-preload-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(val) = ctx.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case("Strict-Transport-Security")).map(|(_, v)| v.as_str()) {
            if !val.to_lowercase().contains("preload") {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Security, code: "HSTSPRE-V5001".to_string(), title: "HSTS missing preload".to_string(), description: "Domain not in browser preload list.".to_string(), url: url.clone(), recommendation: "Add preload directive.".to_string() });
            }
        }
        findings
    }
}

pub struct XContentTypeOptionsValidatorV5;
impl Default for XContentTypeOptionsValidatorV5 { fn default() -> Self { Self::new() } }
impl XContentTypeOptionsValidatorV5 { pub fn new() -> Self { Self } }
impl Analyzer for XContentTypeOptionsValidatorV5 {
    fn name(&self) -> &str { "x-content-type-options-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        match ctx.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case("X-Content-Type-Options")).map(|(_, v)| v.as_str()) {
            None => { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Security, code: "XCTO-V5001".to_string(), title: "Missing X-Content-Type-Options".to_string(), description: "Browsers may MIME-sniff responses.".to_string(), url: url.clone(), recommendation: "Add X-Content-Type-Options: nosniff.".to_string() }); }
            Some(val) if !val.trim().eq_ignore_ascii_case("nosniff") => { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Security, code: "XCTO-V5002".to_string(), title: "Invalid X-Content-Type-Options".to_string(), description: format!("Value '{val}', expected 'nosniff'."), url: url.clone(), recommendation: "Set to nosniff.".to_string() }); }
            _ => {}
        }
        findings
    }
}

pub struct ReferrerPolicyValidatorV5;
impl Default for ReferrerPolicyValidatorV5 { fn default() -> Self { Self::new() } }
impl ReferrerPolicyValidatorV5 { pub fn new() -> Self { Self } }
impl Analyzer for ReferrerPolicyValidatorV5 {
    fn name(&self) -> &str { "referrer-policy-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        match ctx.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case("Referrer-Policy")).map(|(_, v)| v.as_str()) {
            None => { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Security, code: "RP-V5001".to_string(), title: "Missing Referrer-Policy".to_string(), description: "No referrer policy set.".to_string(), url: url.clone(), recommendation: "Add Referrer-Policy: strict-origin-when-cross-origin.".to_string() }); }
            Some(val) if val.eq_ignore_ascii_case("unsafe-url") => { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Security, code: "RP-V5002".to_string(), title: "Referrer-Policy unsafe-url".to_string(), description: "Leaks full URL path and query.".to_string(), url: url.clone(), recommendation: "Use strict-origin-when-cross-origin.".to_string() }); }
            Some(val) if val.eq_ignore_ascii_case("no-referrer") => { findings.push(Finding { severity: Severity::Info, category: IssueCategory::Security, code: "RP-V5003".to_string(), title: "Referrer-Policy no-referrer".to_string(), description: "No referrer sent at all.".to_string(), url: url.clone(), recommendation: "Consider strict-origin-when-cross-origin.".to_string() }); }
            _ => {}
        }
        findings
    }
}

pub struct XFrameOptionsValidatorV5;
impl Default for XFrameOptionsValidatorV5 { fn default() -> Self { Self::new() } }
impl XFrameOptionsValidatorV5 { pub fn new() -> Self { Self } }
impl Analyzer for XFrameOptionsValidatorV5 {
    fn name(&self) -> &str { "x-frame-options-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let xfo = ctx.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case("X-Frame-Options")).map(|(_, v)| v.as_str());
        let csp_frame = ctx.headers.iter().any(|(k, v)| k.eq_ignore_ascii_case("Content-Security-Policy") && v.contains("frame-ancestors"));
        if xfo.is_none() && !csp_frame {
            findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Security, code: "XFO-V5001".to_string(), title: "No clickjacking protection".to_string(), description: "Neither X-Frame-Options nor CSP frame-ancestors.".to_string(), url: url.clone(), recommendation: "Add X-Frame-Options: DENY or CSP frame-ancestors.".to_string() });
        }
        if let Some(val) = xfo {
            if !val.eq_ignore_ascii_case("DENY") && !val.eq_ignore_ascii_case("SAMEORIGIN") {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Security, code: "XFO-V5002".to_string(), title: "Invalid X-Frame-Options value".to_string(), description: format!("'{val}' is not DENY or SAMEORIGIN."), url: url.clone(), recommendation: "Use DENY or SAMEORIGIN.".to_string() });
            }
        }
        findings
    }
}

pub struct PermissionsPolicyCameraValidator;
impl Default for PermissionsPolicyCameraValidator { fn default() -> Self { Self::new() } }
impl PermissionsPolicyCameraValidator { pub fn new() -> Self { Self } }
impl Analyzer for PermissionsPolicyCameraValidator {
    fn name(&self) -> &str { "permissions-policy-camera-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let pp = ctx.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case("Permissions-Policy") || k.eq_ignore_ascii_case("Feature-Policy")).map(|(_, v)| v.as_str());
        if let Some(val) = pp {
            if val.contains("camera") && val.contains("camera=()") {
                // Explicitly denied - good
            } else if val.contains("camera") && !val.contains("camera=()") {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Security, code: "PPCAM-V5001".to_string(), title: "Camera access not explicitly denied".to_string(), description: "Permissions-Policy doesn't deny camera.".to_string(), url: url.clone(), recommendation: "Add camera=() to Permissions-Policy.".to_string() });
            }
        }
        findings
    }
}

pub struct PermissionsPolicyMicrophoneValidator;
impl Default for PermissionsPolicyMicrophoneValidator { fn default() -> Self { Self::new() } }
impl PermissionsPolicyMicrophoneValidator { pub fn new() -> Self { Self } }
impl Analyzer for PermissionsPolicyMicrophoneValidator {
    fn name(&self) -> &str { "permissions-policy-microphone-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let pp = ctx.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case("Permissions-Policy") || k.eq_ignore_ascii_case("Feature-Policy")).map(|(_, v)| v.as_str());
        if let Some(val) = pp {
            if val.contains("microphone") && !val.contains("microphone=()") {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Security, code: "PPMICRO-V5001".to_string(), title: "Microphone access not explicitly denied".to_string(), description: "Permissions-Policy doesn't deny microphone.".to_string(), url: url.clone(), recommendation: "Add microphone=() to Permissions-Policy.".to_string() });
            }
        }
        findings
    }
}

pub struct PermissionsPolicyGeolocationValidator;
impl Default for PermissionsPolicyGeolocationValidator { fn default() -> Self { Self::new() } }
impl PermissionsPolicyGeolocationValidator { pub fn new() -> Self { Self } }
impl Analyzer for PermissionsPolicyGeolocationValidator {
    fn name(&self) -> &str { "permissions-policy-geolocation-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let pp = ctx.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case("Permissions-Policy") || k.eq_ignore_ascii_case("Feature-Policy")).map(|(_, v)| v.as_str());
        if let Some(val) = pp {
            if val.contains("geolocation") && !val.contains("geolocation=()") {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Security, code: "PPGEO-V5001".to_string(), title: "Geolocation not explicitly denied".to_string(), description: "Permissions-Policy doesn't deny geolocation.".to_string(), url: url.clone(), recommendation: "Add geolocation=() to Permissions-Policy.".to_string() });
            }
        }
        findings
    }
}

pub struct CoepValidator;
impl Default for CoepValidator { fn default() -> Self { Self::new() } }
impl CoepValidator { pub fn new() -> Self { Self } }
impl Analyzer for CoepValidator {
    fn name(&self) -> &str { "coep-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if !ctx.headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("Cross-Origin-Embedder-Policy")) {
            findings.push(Finding { severity: Severity::Info, category: IssueCategory::Security, code: "COEP-V5001".to_string(), title: "Missing Cross-Origin-Embedder-Policy".to_string(), description: "COEP prevents loading cross-origin resources.".to_string(), url: url.clone(), recommendation: "Add Cross-Origin-Embedder-Policy: require-corp.".to_string() });
        }
        findings
    }
}

pub struct CoopValidator;
impl Default for CoopValidator { fn default() -> Self { Self::new() } }
impl CoopValidator { pub fn new() -> Self { Self } }
impl Analyzer for CoopValidator {
    fn name(&self) -> &str { "coop-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if !ctx.headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("Cross-Origin-Opener-Policy")) {
            findings.push(Finding { severity: Severity::Info, category: IssueCategory::Security, code: "COOP-V5001".to_string(), title: "Missing Cross-Origin-Opener-Policy".to_string(), description: "COOP controls cross-origin references.".to_string(), url: url.clone(), recommendation: "Add Cross-Origin-Opener-Policy: same-origin.".to_string() });
        }
        findings
    }
}

pub struct CookieSecureFlagValidatorV5;
impl Default for CookieSecureFlagValidatorV5 { fn default() -> Self { Self::new() } }
impl CookieSecureFlagValidatorV5 { pub fn new() -> Self { Self } }
impl Analyzer for CookieSecureFlagValidatorV5 {
    fn name(&self) -> &str { "cookie-secure-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for (k, v) in ctx.headers {
            if !k.eq_ignore_ascii_case("set-cookie") { continue; }
            let lower = v.to_lowercase();
            if !lower.contains("secure") {
                let name = v.split('=').next().unwrap_or("cookie").trim().to_string();
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Security, code: "COOKIESEC-V5001".to_string(), title: format!("Cookie '{name}' missing Secure"), description: "Cookie transmitted over HTTP.".to_string(), url: url.clone(), recommendation: "Add Secure flag.".to_string() });
            }
        }
        findings
    }
}

pub struct CookieHttpOnlyFlagValidatorV5;
impl Default for CookieHttpOnlyFlagValidatorV5 { fn default() -> Self { Self::new() } }
impl CookieHttpOnlyFlagValidatorV5 { pub fn new() -> Self { Self } }
impl Analyzer for CookieHttpOnlyFlagValidatorV5 {
    fn name(&self) -> &str { "cookie-httponly-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for (k, v) in ctx.headers {
            if !k.eq_ignore_ascii_case("set-cookie") { continue; }
            let lower = v.to_lowercase();
            if !lower.contains("httponly") {
                let name = v.split('=').next().unwrap_or("cookie").trim().to_string();
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Security, code: "COOKIEHTTP-V5001".to_string(), title: format!("Cookie '{name}' missing HttpOnly"), description: "Cookie accessible to JavaScript.".to_string(), url: url.clone(), recommendation: "Add HttpOnly flag.".to_string() });
            }
        }
        findings
    }
}

pub struct CookieSameSiteValidator;
impl Default for CookieSameSiteValidator { fn default() -> Self { Self::new() } }
impl CookieSameSiteValidator { pub fn new() -> Self { Self } }
impl Analyzer for CookieSameSiteValidator {
    fn name(&self) -> &str { "cookie-samesite-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for (k, v) in ctx.headers {
            if !k.eq_ignore_ascii_case("set-cookie") { continue; }
            let lower = v.to_lowercase();
            if !lower.contains("samesite") {
                let name = v.split('=').next().unwrap_or("cookie").trim().to_string();
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Security, code: "COOKIESAME-V5001".to_string(), title: format!("Cookie '{name}' missing SameSite"), description: "Without SameSite, cookie is vulnerable to CSRF.".to_string(), url: url.clone(), recommendation: "Add SameSite=Strict or Lax.".to_string() });
            }
        }
        findings
    }
}

pub struct MixedContentScriptValidatorV5;
impl Default for MixedContentScriptValidatorV5 { fn default() -> Self { Self::new() } }
impl MixedContentScriptValidatorV5 { pub fn new() -> Self { Self } }
impl Analyzer for MixedContentScriptValidatorV5 {
    fn name(&self) -> &str { "mixed-content-script-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if !url.starts_with("https://") { return findings; }
        if let Some(body) = ctx.body {
            let lower = body.to_lowercase();
            let http_scripts = lower.matches("src=\"http://").count() + lower.matches("src='http://").count();
            if http_scripts > 0 {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Security, code: "MIXSCRIPT-V5001".to_string(), title: format!("{http_scripts} HTTP script(s) on HTTPS page"), description: "Mixed content scripts are blocked by browsers.".to_string(), url: url.clone(), recommendation: "Change script URLs to HTTPS.".to_string() });
            }
        }
        findings
    }
}

pub struct MixedContentStylesheetValidator;
impl Default for MixedContentStylesheetValidator { fn default() -> Self { Self::new() } }
impl MixedContentStylesheetValidator { pub fn new() -> Self { Self } }
impl Analyzer for MixedContentStylesheetValidator {
    fn name(&self) -> &str { "mixed-content-stylesheet-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if !url.starts_with("https://") { return findings; }
        if let Some(body) = ctx.body {
            let lower = body.to_lowercase();
            let http_css = lower.matches("href=\"http://").count() + lower.matches("href='http://").count();
            if http_css > 0 {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Security, code: "MIXCSS-V5001".to_string(), title: format!("{http_css} HTTP stylesheet(s) on HTTPS page"), description: "Mixed content stylesheets degrade security.".to_string(), url: url.clone(), recommendation: "Change stylesheet URLs to HTTPS.".to_string() });
            }
        }
        findings
    }
}

pub struct SriValidator;
impl Default for SriValidator { fn default() -> Self { Self::new() } }
impl SriValidator { pub fn new() -> Self { Self } }
impl Analyzer for SriValidator {
    fn name(&self) -> &str { "sri-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(body) = ctx.body {
            let lower = body.to_lowercase();
            let _script_count = lower.matches("<script").count();
            let sri_count = lower.matches("integrity=").count();
            let external_scripts = ctx.page.scripts.iter().filter(|s| s.src.is_some()).count();
            if external_scripts > 0 && sri_count == 0 {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Security, code: "SRI-V5001".to_string(), title: "No SRI on external scripts".to_string(), description: format!("{external_scripts} external script(s) without integrity."), url: url.clone(), recommendation: "Add integrity attribute to external scripts.".to_string() });
            }
        }
        findings
    }
}

// =========================================================================
// SEO V5 Analyzers (51-65)
// =========================================================================

pub struct TitleKeywordPresenceValidator;
impl Default for TitleKeywordPresenceValidator { fn default() -> Self { Self::new() } }
impl TitleKeywordPresenceValidator { pub fn new() -> Self { Self } }
impl Analyzer for TitleKeywordPresenceValidator {
    fn name(&self) -> &str { "title-keyword-presence-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let title = match &ctx.page.meta.title { Some(t) if !t.trim().is_empty() => t.trim().to_lowercase(), _ => return findings };
        if let Some(desc) = &ctx.page.meta.description {
            let desc_words: Vec<&str> = desc.split_whitespace().filter(|w| w.len() > 3).take(5).collect();
            let missing: Vec<&str> = desc_words.iter().filter(|w| !title.contains(*w)).copied().collect();
            if missing.len() > 2 {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Seo, code: "TITLEKWP-V5001".to_string(), title: "Title missing description keywords".to_string(), description: format!("Title doesn't contain {} description keywords.", missing.len()), url: url.clone(), recommendation: "Include important keywords from description in title.".to_string() });
            }
        }
        findings
    }
}

pub struct TitleBrandValidator;
impl Default for TitleBrandValidator { fn default() -> Self { Self::new() } }
impl TitleBrandValidator { pub fn new() -> Self { Self } }
impl Analyzer for TitleBrandValidator {
    fn name(&self) -> &str { "title-brand-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let title = match &ctx.page.meta.title { Some(t) if !t.trim().is_empty() => t.trim().to_lowercase(), _ => return findings };
        let org_name = ctx.page.structured_data.iter().find_map(|sd| {
            if sd.r#type.as_deref() == Some("Organization") || sd.r#type.as_deref() == Some("LocalBusiness") {
                sd.data.get("name").and_then(|v| v.as_str()).map(|s| s.to_lowercase())
            } else { None }
        });
        if let Some(brand) = org_name {
            if !brand.is_empty() && !title.contains(&brand) {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Seo, code: "TITLEBRAND-V5001".to_string(), title: "Title missing brand name".to_string(), description: format!("Title doesn't contain brand '{brand}'."), url: url.clone(), recommendation: "Include brand name in title for recognition.".to_string() });
            }
        }
        findings
    }
}

pub struct TitleLengthValidatorV5;
impl Default for TitleLengthValidatorV5 { fn default() -> Self { Self::new() } }
impl TitleLengthValidatorV5 { pub fn new() -> Self { Self } }
impl Analyzer for TitleLengthValidatorV5 {
    fn name(&self) -> &str { "title-length-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        match &ctx.page.meta.title {
            None => { findings.push(Finding { severity: Severity::Error, category: IssueCategory::Seo, code: "TITLELEN-V5001".to_string(), title: "Missing title tag".to_string(), description: "No <title> found.".to_string(), url: url.clone(), recommendation: "Add a 30-60 character title.".to_string() }); }
            Some(t) => {
                let len = t.len();
                if len < 30 { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Seo, code: "TITLELEN-V5002".to_string(), title: "Title too short".to_string(), description: format!("{len} chars, aim for 30-60."), url: url.clone(), recommendation: "Expand to 30-60 characters.".to_string() }); }
                else if len > 60 { findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Seo, code: "TITLELEN-V5003".to_string(), title: "Title may truncate".to_string(), description: format!("{len} chars, Google shows ~60."), url: url.clone(), recommendation: "Shorten to 60 characters.".to_string() }); }
            }
        }
        findings
    }
}

pub struct MetaDescriptionKeywordValidator;
impl Default for MetaDescriptionKeywordValidator { fn default() -> Self { Self::new() } }
impl MetaDescriptionKeywordValidator { pub fn new() -> Self { Self } }
impl Analyzer for MetaDescriptionKeywordValidator {
    fn name(&self) -> &str { "meta-description-keyword-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let desc = match &ctx.page.meta.description { Some(d) if !d.trim().is_empty() => d.trim(), _ => return findings };
        let title = match &ctx.page.meta.title { Some(t) => t.as_str(), None => "" };
        if !title.is_empty() {
            let title_words: Vec<&str> = title.split_whitespace().filter(|w| w.len() > 3).collect();
            let desc_lower = desc.to_lowercase();
            let missing: Vec<&str> = title_words.iter().filter(|w| !desc_lower.contains(&w.to_lowercase())).copied().collect();
            if missing.len() > 1 {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Seo, code: "METAKEY-V5001".to_string(), title: "Description missing title keywords".to_string(), description: format!("Description missing {} title keywords.", missing.len()), url: url.clone(), recommendation: "Include title keywords in description.".to_string() });
            }
        }
        findings
    }
}

pub struct MetaDescriptionUniqueValidator;
impl Default for MetaDescriptionUniqueValidator { fn default() -> Self { Self::new() } }
impl MetaDescriptionUniqueValidator { pub fn new() -> Self { Self } }
impl Analyzer for MetaDescriptionUniqueValidator {
    fn name(&self) -> &str { "meta-description-unique-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let desc = match &ctx.page.meta.description { Some(d) if !d.trim().is_empty() => d.trim().to_lowercase(), _ => return findings };
        let title = match &ctx.page.meta.title { Some(t) if !t.trim().is_empty() => t.trim().to_lowercase(), _ => return findings };
        if desc == title {
            findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Seo, code: "METAUNIQ-V5001".to_string(), title: "Description identical to title".to_string(), description: "Description should complement title, not duplicate it.".to_string(), url: url.clone(), recommendation: "Write a unique description that expands on the title.".to_string() });
        }
        findings
    }
}

pub struct CanonicalSelfReferenceValidatorV5;
impl Default for CanonicalSelfReferenceValidatorV5 { fn default() -> Self { Self::new() } }
impl CanonicalSelfReferenceValidatorV5 { pub fn new() -> Self { Self } }
impl Analyzer for CanonicalSelfReferenceValidatorV5 {
    fn name(&self) -> &str { "canonical-self-reference-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(canonical) = &ctx.page.meta.canonical {
            let canonical_str = canonical.as_str();
            if canonical_str != url {
                if let Ok(page_url) = url::Url::parse(url) {
                    if canonical.path() == page_url.path() && canonical.query() == page_url.query() {
                        // Same path - might be a trailing slash issue, not a real problem
                    } else {
                        findings.push(Finding { severity: Severity::Info, category: IssueCategory::Seo, code: "CANSELF-V5001".to_string(), title: "Canonical points to different URL".to_string(), description: format!("Canonical '{}' differs from page URL.", canonical_str), url: url.clone(), recommendation: "Verify canonical points to correct URL.".to_string() });
                    }
                }
            }
        }
        findings
    }
}

pub struct CanonicalChainValidatorV5;
impl Default for CanonicalChainValidatorV5 { fn default() -> Self { Self::new() } }
impl CanonicalChainValidatorV5 { pub fn new() -> Self { Self } }
impl Analyzer for CanonicalChainValidatorV5 {
    fn name(&self) -> &str { "canonical-chain-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(canonical) = &ctx.page.meta.canonical {
            let canonical_str = canonical.as_str();
            if let Some(body) = ctx.body {
                let lower = body.to_lowercase();
                let canonical_in_body = format!("href=\"{}\"", canonical_str.to_lowercase());
                if lower.contains(&canonical_in_body) {
                    // Self-referencing - that's expected
                } else if canonical_str != url {
                    // Canonical points elsewhere, which is fine but we can flag chains
                    if lower.contains("rel=\"canonical\"") {
                        findings.push(Finding { severity: Severity::Info, category: IssueCategory::Seo, code: "CANCHAIN-V5001".to_string(), title: "Canonical points off-page".to_string(), description: format!("Canonical '{}' doesn't match current URL.", canonical_str), url: url.clone(), recommendation: "Ensure no canonical chains exist.".to_string() });
                    }
                }
            }
        }
        findings
    }
}

pub struct CanonicalDepthValidatorV5;
impl Default for CanonicalDepthValidatorV5 { fn default() -> Self { Self::new() } }
impl CanonicalDepthValidatorV5 { pub fn new() -> Self { Self } }
impl Analyzer for CanonicalDepthValidatorV5 {
    fn name(&self) -> &str { "canonical-depth-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(canonical) = &ctx.page.meta.canonical {
            let depth = canonical.path().trim_matches('/').split('/').filter(|s| !s.is_empty()).count();
            if depth > 5 {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Seo, code: "CANDEPTH-V5001".to_string(), title: "Canonical URL too deep".to_string(), description: format!("{} path segments in canonical URL.", depth), url: url.clone(), recommendation: "Flatten URL structure if possible.".to_string() });
            }
        }
        findings
    }
}

pub struct HreflangReciprocalValidatorV5;
impl Default for HreflangReciprocalValidatorV5 { fn default() -> Self { Self::new() } }
impl HreflangReciprocalValidatorV5 { pub fn new() -> Self { Self } }
impl Analyzer for HreflangReciprocalValidatorV5 {
    fn name(&self) -> &str { "hreflang-reciprocal-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let tags = &ctx.page.meta.hreflang;
        if tags.is_empty() { return findings; }
        let has_self = tags.iter().any(|t| t.url.as_str() == *url);
        if !has_self {
            findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Seo, code: "HREFRECIP-V5001".to_string(), title: "Missing self-referencing hreflang".to_string(), description: "No hreflang tag points to this page.".to_string(), url: url.clone(), recommendation: "Add self-referencing hreflang tag.".to_string() });
        }
        findings
    }
}

pub struct HreflangXDefaultValidator;
impl Default for HreflangXDefaultValidator { fn default() -> Self { Self::new() } }
impl HreflangXDefaultValidator { pub fn new() -> Self { Self } }
impl Analyzer for HreflangXDefaultValidator {
    fn name(&self) -> &str { "hreflang-x-default-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let tags = &ctx.page.meta.hreflang;
        if tags.is_empty() { return findings; }
        let has_xd = tags.iter().any(|t| t.lang == "x-default");
        if !has_xd {
            findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Seo, code: "HREFXD-V5001".to_string(), title: "Missing x-default hreflang".to_string(), description: "No x-default hreflang tag.".to_string(), url: url.clone(), recommendation: "Add x-default hreflang for default language.".to_string() });
        }
        findings
    }
}

pub struct HreflangLocaleFormatValidator;
impl Default for HreflangLocaleFormatValidator { fn default() -> Self { Self::new() } }
impl HreflangLocaleFormatValidator { pub fn new() -> Self { Self } }
impl Analyzer for HreflangLocaleFormatValidator {
    fn name(&self) -> &str { "hreflang-locale-format-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let tags = &ctx.page.meta.hreflang;
        if tags.is_empty() { return findings; }
        for tag in tags {
            if tag.lang == "x-default" { continue; }
            let lang = &tag.lang;
            if !lang.contains('-') && !lang.contains('_') && lang.len() > 3 {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Seo, code: "HREFLOCALE-V5001".to_string(), title: "Hreflang missing region".to_string(), description: format!("'{lang}' should include region (e.g., en-US)."), url: url.clone(), recommendation: "Use format like 'en-US' instead of 'en'.".to_string() });
            }
        }
        findings
    }
}

pub struct SitemapCoverageValidatorV5;
impl Default for SitemapCoverageValidatorV5 { fn default() -> Self { Self::new() } }
impl SitemapCoverageValidatorV5 { pub fn new() -> Self { Self } }
impl Analyzer for SitemapCoverageValidatorV5 {
    fn name(&self) -> &str { "sitemap-coverage-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(robots) = ctx.robots_txt {
            let lower = robots.to_lowercase();
            if !lower.contains("sitemap:") {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Seo, code: "SITEMAPCOV-V5001".to_string(), title: "No sitemap in robots.txt".to_string(), description: "robots.txt has no Sitemap directive.".to_string(), url: url.clone(), recommendation: "Add Sitemap: directive to robots.txt.".to_string() });
            }
        }
        findings
    }
}

pub struct SitemapLastmodValidator;
impl Default for SitemapLastmodValidator { fn default() -> Self { Self::new() } }
impl SitemapLastmodValidator { pub fn new() -> Self { Self } }
impl Analyzer for SitemapLastmodValidator {
    fn name(&self) -> &str { "sitemap-lastmod-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(body) = ctx.body {
            if body.contains("<urlset") {
                let lastmod_count = body.matches("<lastmod>").count();
                let url_count = body.matches("<url>").count();
                if url_count > 0 && lastmod_count == 0 {
                    findings.push(Finding { severity: Severity::Info, category: IssueCategory::Seo, code: "SITEMAPMOD-V5001".to_string(), title: "Sitemap missing lastmod".to_string(), description: "No lastmod dates in sitemap.".to_string(), url: url.clone(), recommendation: "Add lastmod to help crawlers prioritize.".to_string() });
                }
            }
        }
        findings
    }
}

pub struct SitemapPriorityValidator;
impl Default for SitemapPriorityValidator { fn default() -> Self { Self::new() } }
impl SitemapPriorityValidator { pub fn new() -> Self { Self } }
impl Analyzer for SitemapPriorityValidator {
    fn name(&self) -> &str { "sitemap-priority-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(body) = ctx.body {
            if body.contains("<urlset") {
                let priority_count = body.matches("<priority>").count();
                let url_count = body.matches("<url>").count();
                if url_count > 0 && priority_count == 0 {
                    findings.push(Finding { severity: Severity::Info, category: IssueCategory::Seo, code: "SITEMAPPRI-V5001".to_string(), title: "Sitemap missing priority".to_string(), description: "No priority values in sitemap.".to_string(), url: url.clone(), recommendation: "Add priority to indicate page importance.".to_string() });
                }
            }
        }
        findings
    }
}

pub struct RobotsTxtDisallowValidator;
impl Default for RobotsTxtDisallowValidator { fn default() -> Self { Self::new() } }
impl RobotsTxtDisallowValidator { pub fn new() -> Self { Self } }
impl Analyzer for RobotsTxtDisallowValidator {
    fn name(&self) -> &str { "robots-txt-disallow-v5" }
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
                    if path == "/" {
                        findings.push(Finding { severity: Severity::Critical, category: IssueCategory::Seo, code: "ROBOTSDIS-V5001".to_string(), title: "robots.txt blocks all crawlers".to_string(), description: "Disallow: / blocks everything.".to_string(), url: url.clone(), recommendation: "Remove blanket disallow.".to_string() });
                        break;
                    }
                }
            }
        }
        let disallow_count = lower.lines().filter(|l| l.trim().starts_with("disallow:")).count();
        if disallow_count > 50 {
            findings.push(Finding { severity: Severity::Info, category: IssueCategory::Seo, code: "ROBOTSDIS-V5002".to_string(), title: "robots.txt has many Disallow rules".to_string(), description: format!("{disallow_count} Disallow rules."), url: url.clone(), recommendation: "Simplify robots.txt rules.".to_string() });
        }
        findings
    }
}

// =========================================================================
// Accessibility V5 Analyzers (66-75)
// =========================================================================

pub struct LandmarkMainValidatorV5;
impl Default for LandmarkMainValidatorV5 { fn default() -> Self { Self::new() } }
impl LandmarkMainValidatorV5 { pub fn new() -> Self { Self } }
impl Analyzer for LandmarkMainValidatorV5 {
    fn name(&self) -> &str { "landmark-main-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if !ctx.page.landmarks.iter().any(|l| l.to_lowercase() == "main") && !ctx.page.has_main_landmark {
            findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Accessibility, code: "LANDMAIN-V5001".to_string(), title: "Missing main landmark".to_string(), description: "No <main> or role='main' found.".to_string(), url: url.clone(), recommendation: "Add a main landmark for primary content.".to_string() });
        }
        let main_count = ctx.page.landmarks.iter().filter(|l| l.to_lowercase() == "main").count();
        if main_count > 1 {
            findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Accessibility, code: "LANDMAIN-V5002".to_string(), title: "Multiple main landmarks".to_string(), description: format!("{main_count} main landmarks. Only one allowed."), url: url.clone(), recommendation: "Use a single main landmark.".to_string() });
        }
        findings
    }
}

pub struct LandmarkNavValidatorV5;
impl Default for LandmarkNavValidatorV5 { fn default() -> Self { Self::new() } }
impl LandmarkNavValidatorV5 { pub fn new() -> Self { Self } }
impl Analyzer for LandmarkNavValidatorV5 {
    fn name(&self) -> &str { "landmark-nav-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if !ctx.page.landmarks.iter().any(|l| l.to_lowercase() == "navigation") && !ctx.page.has_nav_landmark {
            findings.push(Finding { severity: Severity::Info, category: IssueCategory::Accessibility, code: "LANDNAV-V5001".to_string(), title: "Missing navigation landmark".to_string(), description: "No <nav> or role='navigation' found.".to_string(), url: url.clone(), recommendation: "Add a navigation landmark.".to_string() });
        }
        findings
    }
}

pub struct LandmarkBannerValidatorV5;
impl Default for LandmarkBannerValidatorV5 { fn default() -> Self { Self::new() } }
impl LandmarkBannerValidatorV5 { pub fn new() -> Self { Self } }
impl Analyzer for LandmarkBannerValidatorV5 {
    fn name(&self) -> &str { "landmark-banner-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if !ctx.page.landmarks.iter().any(|l| l.to_lowercase() == "banner") {
            findings.push(Finding { severity: Severity::Info, category: IssueCategory::Accessibility, code: "LANDBANNER-V5001".to_string(), title: "Missing banner landmark".to_string(), description: "No <header> or role='banner' found.".to_string(), url: url.clone(), recommendation: "Add a banner landmark for site header.".to_string() });
        }
        findings
    }
}

pub struct HeadingSkipLevelsValidator;
impl Default for HeadingSkipLevelsValidator { fn default() -> Self { Self::new() } }
impl HeadingSkipLevelsValidator { pub fn new() -> Self { Self } }
impl Analyzer for HeadingSkipLevelsValidator {
    fn name(&self) -> &str { "heading-skip-levels-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.headings.is_empty() { return findings; }
        let mut prev_level = 0u8;
        let mut skip_count = 0;
        for h in &ctx.page.headings {
            if prev_level > 0 && h.level > prev_level + 1 {
                skip_count += 1;
            }
            prev_level = h.level;
        }
        if skip_count > 0 {
            findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Accessibility, code: "HEADSKIP-V5001".to_string(), title: "Heading levels skipped".to_string(), description: format!("{skip_count} heading level skip(s)."), url: url.clone(), recommendation: "Use heading levels sequentially (H1 > H2 > H3).".to_string() });
        }
        findings
    }
}

pub struct HeadingMultipleH1Validator;
impl Default for HeadingMultipleH1Validator { fn default() -> Self { Self::new() } }
impl HeadingMultipleH1Validator { pub fn new() -> Self { Self } }
impl Analyzer for HeadingMultipleH1Validator {
    fn name(&self) -> &str { "heading-multiple-h1-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let h1_count = ctx.page.headings.iter().filter(|h| h.level == 1).count();
        if h1_count == 0 {
            findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Accessibility, code: "HEADH1-V5001".to_string(), title: "Missing H1 heading".to_string(), description: "No H1 heading found.".to_string(), url: url.clone(), recommendation: "Add a single H1 heading.".to_string() });
        } else if h1_count > 1 {
            findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Accessibility, code: "HEADH1-V5002".to_string(), title: "Multiple H1 headings".to_string(), description: format!("{h1_count} H1 headings found."), url: url.clone(), recommendation: "Use only one H1 per page.".to_string() });
        }
        findings
    }
}

pub struct FormLabelAssociationValidatorV5;
impl Default for FormLabelAssociationValidatorV5 { fn default() -> Self { Self::new() } }
impl FormLabelAssociationValidatorV5 { pub fn new() -> Self { Self } }
impl Analyzer for FormLabelAssociationValidatorV5 {
    fn name(&self) -> &str { "form-label-association-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.forms.is_empty() { return findings; }
        let mut unlabeled = 0;
        for form in &ctx.page.forms {
            for input in &form.inputs {
                let t = input.input_type.as_deref().unwrap_or("text");
                if matches!(t, "hidden" | "submit" | "button" | "image" | "reset") { continue; }
                if !input.has_label && input.aria_label.is_none() && input.aria_labelledby.is_none() {
                    unlabeled += 1;
                }
            }
        }
        if unlabeled > 0 {
            findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Accessibility, code: "FORMLBLASSOC-V5001".to_string(), title: "Form inputs without labels".to_string(), description: format!("{unlabeled} input(s) lack associated labels."), url: url.clone(), recommendation: "Associate inputs with <label> elements.".to_string() });
        }
        findings
    }
}

pub struct FormRequiredFieldsValidator;
impl Default for FormRequiredFieldsValidator { fn default() -> Self { Self::new() } }
impl FormRequiredFieldsValidator { pub fn new() -> Self { Self } }
impl Analyzer for FormRequiredFieldsValidator {
    fn name(&self) -> &str { "form-required-fields-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.forms.is_empty() { return findings; }
        let mut missing_required = 0;
        for form in &ctx.page.forms {
            for input in &form.inputs {
                if input.required && input.aria_label.is_none() && !input.has_label {
                    missing_required += 1;
                }
            }
        }
        if missing_required > 0 {
            findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Accessibility, code: "FORMREQ-V5001".to_string(), title: "Required fields missing labels".to_string(), description: format!("{missing_required} required input(s) lack labels."), url: url.clone(), recommendation: "Add labels to required form fields.".to_string() });
        }
        findings
    }
}

pub struct TableHeadersValidatorV5;
impl Default for TableHeadersValidatorV5 { fn default() -> Self { Self::new() } }
impl TableHeadersValidatorV5 { pub fn new() -> Self { Self } }
impl Analyzer for TableHeadersValidatorV5 {
    fn name(&self) -> &str { "table-headers-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.tables_total == 0 { return findings; }
        if ctx.page.tables_with_headers == 0 {
            findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Accessibility, code: "TBLHDR-V5001".to_string(), title: "All tables missing headers".to_string(), description: format!("{} table(s) lack <th> headers.", ctx.page.tables_total), url: url.clone(), recommendation: "Add <th> elements to data tables.".to_string() });
        } else if ctx.page.tables_with_headers < ctx.page.tables_total {
            findings.push(Finding { severity: Severity::Info, category: IssueCategory::Accessibility, code: "TBLHDR-V5002".to_string(), title: "Some tables missing headers".to_string(), description: format!("{}/{} tables have headers.", ctx.page.tables_with_headers, ctx.page.tables_total), url: url.clone(), recommendation: "Add headers to all data tables.".to_string() });
        }
        findings
    }
}

pub struct TableCaptionValidatorV5;
impl Default for TableCaptionValidatorV5 { fn default() -> Self { Self::new() } }
impl TableCaptionValidatorV5 { pub fn new() -> Self { Self } }
impl Analyzer for TableCaptionValidatorV5 {
    fn name(&self) -> &str { "table-caption-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.tables_total == 0 { return findings; }
        if ctx.page.tables_with_captions == 0 {
            findings.push(Finding { severity: Severity::Info, category: IssueCategory::Accessibility, code: "TBLCAPT-V5001".to_string(), title: "Tables missing captions".to_string(), description: format!("{} table(s) lack <caption>.", ctx.page.tables_total), url: url.clone(), recommendation: "Add <caption> to data tables.".to_string() });
        }
        findings
    }
}

pub struct TableScopeValidator;
impl Default for TableScopeValidator { fn default() -> Self { Self::new() } }
impl TableScopeValidator { pub fn new() -> Self { Self } }
impl Analyzer for TableScopeValidator {
    fn name(&self) -> &str { "table-scope-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.tables_total == 0 { return findings; }
        if let Some(body) = ctx.body {
            let lower = body.to_lowercase();
            let th_count = lower.matches("<th").count();
            let scope_count = lower.matches("scope=\"").count();
            if th_count > 0 && scope_count == 0 {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Accessibility, code: "TBLSCOPE-V5001".to_string(), title: "Table headers missing scope".to_string(), description: format!("{th_count} <th> without scope attribute."), url: url.clone(), recommendation: "Add scope='col' or scope='row'.".to_string() });
            }
        }
        findings
    }
}

// =========================================================================
// Performance V5 Analyzers (76-80)
// =========================================================================

pub struct PreconnectHintValidator;
impl Default for PreconnectHintValidator { fn default() -> Self { Self::new() } }
impl PreconnectHintValidator { pub fn new() -> Self { Self } }
impl Analyzer for PreconnectHintValidator {
    fn name(&self) -> &str { "preconnect-hint-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(body) = ctx.body {
            let lower = body.to_lowercase();
            let has_preconnect = lower.contains("rel=\"preconnect\"");
            let external = ctx.page.links.iter().filter(|l| l.is_external).count();
            if external > 3 && !has_preconnect {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Performance, code: "PRECON-V5001".to_string(), title: "No preconnect hints".to_string(), description: format!("{external} external origins, no preconnect."), url: url.clone(), recommendation: "Add <link rel='preconnect'> for critical origins.".to_string() });
            }
        }
        findings
    }
}

pub struct DnsPrefetchHintValidator;
impl Default for DnsPrefetchHintValidator { fn default() -> Self { Self::new() } }
impl DnsPrefetchHintValidator { pub fn new() -> Self { Self } }
impl Analyzer for DnsPrefetchHintValidator {
    fn name(&self) -> &str { "dns-prefetch-hint-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(body) = ctx.body {
            let lower = body.to_lowercase();
            let has_prefetch = lower.contains("rel=\"dns-prefetch\"");
            let external = ctx.page.links.iter().filter(|l| l.is_external).count();
            if external > 5 && !has_prefetch {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Performance, code: "DNSPREFETCH-V5001".to_string(), title: "No dns-prefetch hints".to_string(), description: format!("{external} external origins, no dns-prefetch."), url: url.clone(), recommendation: "Add <link rel='dns-prefetch'> for external origins.".to_string() });
            }
        }
        findings
    }
}

pub struct ScriptAsyncDeferValidator;
impl Default for ScriptAsyncDeferValidator { fn default() -> Self { Self::new() } }
impl ScriptAsyncDeferValidator { pub fn new() -> Self { Self } }
impl Analyzer for ScriptAsyncDeferValidator {
    fn name(&self) -> &str { "script-async-defer-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let blocking = ctx.page.scripts.iter().filter(|s| s.src.is_some() && !s.r#async && !s.defer).count();
        if blocking > 0 {
            findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Performance, code: "SCRIPTBLK-V5001".to_string(), title: "Blocking scripts detected".to_string(), description: format!("{blocking} external script(s) without async/defer."), url: url.clone(), recommendation: "Add async or defer to external scripts.".to_string() });
        }
        findings
    }
}

pub struct ImageLazyLoadingValidatorV5;
impl Default for ImageLazyLoadingValidatorV5 { fn default() -> Self { Self::new() } }
impl ImageLazyLoadingValidatorV5 { pub fn new() -> Self { Self } }
impl Analyzer for ImageLazyLoadingValidatorV5 {
    fn name(&self) -> &str { "image-lazy-loading-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let total_images = ctx.page.images.len();
        let lazy_images = ctx.page.images.iter().filter(|i| i.is_lazy_loaded).count();
        if total_images > 3 && lazy_images == 0 {
            findings.push(Finding { severity: Severity::Info, category: IssueCategory::Performance, code: "IMGLAZY-V5001".to_string(), title: "No lazy loading on images".to_string(), description: format!("{total_images} images without lazy loading."), url: url.clone(), recommendation: "Add loading='lazy' to below-fold images.".to_string() });
        }
        findings
    }
}

pub struct ImageModernFormatValidator;
impl Default for ImageModernFormatValidator { fn default() -> Self { Self::new() } }
impl ImageModernFormatValidator { pub fn new() -> Self { Self } }
impl Analyzer for ImageModernFormatValidator {
    fn name(&self) -> &str { "image-modern-format-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.images.is_empty() { return findings; }
        let total = ctx.page.images.len();
        let modern = ctx.page.images.iter().filter(|i| {
            let src = i.src.to_lowercase();
            src.ends_with(".webp") || src.ends_with(".avif") || src.contains(".webp?") || src.contains(".avif?")
        }).count();
        if total > 3 && modern == 0 {
            findings.push(Finding { severity: Severity::Info, category: IssueCategory::Performance, code: "IMGFMT-V5001".to_string(), title: "No modern image formats".to_string(), description: format!("{total} images, none use WebP/AVIF."), url: url.clone(), recommendation: "Use WebP or AVIF for better compression.".to_string() });
        }
        findings
    }
}

pub struct ImageDimensionsValidatorV5;
impl Default for ImageDimensionsValidatorV5 { fn default() -> Self { Self::new() } }
impl ImageDimensionsValidatorV5 { pub fn new() -> Self { Self } }
impl Analyzer for ImageDimensionsValidatorV5 {
    fn name(&self) -> &str { "image-dimensions-v5" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let missing = ctx.page.images.iter().filter(|i| i.width.is_none() || i.height.is_none()).count();
        if missing > 0 {
            findings.push(Finding { severity: Severity::Info, category: IssueCategory::Performance, code: "IMGDIM-V5001".to_string(), title: "Images missing dimensions".to_string(), description: format!("{missing} image(s) lack width/height."), url: url.clone(), recommendation: "Add width and height attributes to prevent layout shift.".to_string() });
        }
        findings
    }
}

// =========================================================================
// V6 Content Validators (1-40)
// =========================================================================

pub struct CreativeWorkMissingNameValidator;
impl Default for CreativeWorkMissingNameValidator { fn default() -> Self { Self::new() } }
impl CreativeWorkMissingNameValidator { pub fn new() -> Self { Self } }
impl Analyzer for CreativeWorkMissingNameValidator {
    fn name(&self) -> &str { "creative-work-missing-name-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "CreativeWork" { continue; }
            if sd.data.get("name").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "CWNAME-V6001".to_string(), title: "CreativeWork missing name".to_string(), description: "No name in CreativeWork schema.".to_string(), url: url.clone(), recommendation: "Add a name to CreativeWork.".to_string() });
            }
        }
        findings
    }
}

pub struct CreativeWorkMissingDescriptionValidator;
impl Default for CreativeWorkMissingDescriptionValidator { fn default() -> Self { Self::new() } }
impl CreativeWorkMissingDescriptionValidator { pub fn new() -> Self { Self } }
impl Analyzer for CreativeWorkMissingDescriptionValidator {
    fn name(&self) -> &str { "creative-work-missing-description-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "CreativeWork" { continue; }
            if sd.data.get("description").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Schema, code: "CWDESC-V6002".to_string(), title: "CreativeWork missing description".to_string(), description: "No description in CreativeWork schema.".to_string(), url: url.clone(), recommendation: "Add a description to CreativeWork.".to_string() });
            }
        }
        findings
    }
}

pub struct CreativeWorkMissingDateCreatedValidator;
impl Default for CreativeWorkMissingDateCreatedValidator { fn default() -> Self { Self::new() } }
impl CreativeWorkMissingDateCreatedValidator { pub fn new() -> Self { Self } }
impl Analyzer for CreativeWorkMissingDateCreatedValidator {
    fn name(&self) -> &str { "creative-work-missing-date-created-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "CreativeWork" { continue; }
            if sd.data.get("dateCreated").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Schema, code: "CWDATE-V6003".to_string(), title: "CreativeWork missing dateCreated".to_string(), description: "No dateCreated in CreativeWork schema.".to_string(), url: url.clone(), recommendation: "Add a dateCreated to CreativeWork.".to_string() });
            }
        }
        findings
    }
}

pub struct PlaylistMissingNumberOfItemsValidator;
impl Default for PlaylistMissingNumberOfItemsValidator { fn default() -> Self { Self::new() } }
impl PlaylistMissingNumberOfItemsValidator { pub fn new() -> Self { Self } }
impl Analyzer for PlaylistMissingNumberOfItemsValidator {
    fn name(&self) -> &str { "playlist-missing-number-of-items-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Playlist" { continue; }
            if sd.data.get("numberOfItems").is_none() {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Schema, code: "PLNUM-V6004".to_string(), title: "Playlist missing numberOfItems".to_string(), description: "No numberOfItems in Playlist schema.".to_string(), url: url.clone(), recommendation: "Add numberOfItems to Playlist.".to_string() });
            }
        }
        findings
    }
}

pub struct FoodEstablishmentMissingMenuValidator;
impl Default for FoodEstablishmentMissingMenuValidator { fn default() -> Self { Self::new() } }
impl FoodEstablishmentMissingMenuValidator { pub fn new() -> Self { Self } }
impl Analyzer for FoodEstablishmentMissingMenuValidator {
    fn name(&self) -> &str { "food-establishment-missing-menu-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "FoodEstablishment" { continue; }
            if sd.data.get("menu").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "FEMENU-V6005".to_string(), title: "FoodEstablishment missing menu".to_string(), description: "No menu in FoodEstablishment schema.".to_string(), url: url.clone(), recommendation: "Add a menu URL to FoodEstablishment.".to_string() });
            }
        }
        findings
    }
}

pub struct FoodEstablishmentMissingServesCuisineValidator;
impl Default for FoodEstablishmentMissingServesCuisineValidator { fn default() -> Self { Self::new() } }
impl FoodEstablishmentMissingServesCuisineValidator { pub fn new() -> Self { Self } }
impl Analyzer for FoodEstablishmentMissingServesCuisineValidator {
    fn name(&self) -> &str { "food-establishment-missing-serves-cuisine-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "FoodEstablishment" { continue; }
            if sd.data.get("servesCuisine").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Schema, code: "FECUIS-V6006".to_string(), title: "FoodEstablishment missing servesCuisine".to_string(), description: "No servesCuisine in FoodEstablishment schema.".to_string(), url: url.clone(), recommendation: "Add servesCuisine to FoodEstablishment.".to_string() });
            }
        }
        findings
    }
}

pub struct LodgingBusinessMissingStarRatingValidator;
impl Default for LodgingBusinessMissingStarRatingValidator { fn default() -> Self { Self::new() } }
impl LodgingBusinessMissingStarRatingValidator { pub fn new() -> Self { Self } }
impl Analyzer for LodgingBusinessMissingStarRatingValidator {
    fn name(&self) -> &str { "lodging-business-missing-star-rating-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "LodgingBusiness" { continue; }
            if sd.data.get("starRating").is_none() {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Schema, code: "LBSTAR-V6007".to_string(), title: "LodgingBusiness missing starRating".to_string(), description: "No starRating in LodgingBusiness schema.".to_string(), url: url.clone(), recommendation: "Add starRating to LodgingBusiness.".to_string() });
            }
        }
        findings
    }
}

pub struct LodgingBusinessMissingAmenityFeatureValidator;
impl Default for LodgingBusinessMissingAmenityFeatureValidator { fn default() -> Self { Self::new() } }
impl LodgingBusinessMissingAmenityFeatureValidator { pub fn new() -> Self { Self } }
impl Analyzer for LodgingBusinessMissingAmenityFeatureValidator {
    fn name(&self) -> &str { "lodging-business-missing-amenity-feature-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "LodgingBusiness" { continue; }
            if sd.data.get("amenityFeature").is_none() {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Schema, code: "LBAMEN-V6008".to_string(), title: "LodgingBusiness missing amenityFeature".to_string(), description: "No amenityFeature in LodgingBusiness schema.".to_string(), url: url.clone(), recommendation: "Add amenityFeature to LodgingBusiness.".to_string() });
            }
        }
        findings
    }
}

pub struct SportsActivityLocationMissingSportValidator;
impl Default for SportsActivityLocationMissingSportValidator { fn default() -> Self { Self::new() } }
impl SportsActivityLocationMissingSportValidator { pub fn new() -> Self { Self } }
impl Analyzer for SportsActivityLocationMissingSportValidator {
    fn name(&self) -> &str { "sports-activity-location-missing-sport-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "SportsActivityLocation" { continue; }
            if sd.data.get("sport").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "SASPORT-V6009".to_string(), title: "SportsActivityLocation missing sport".to_string(), description: "No sport in SportsActivityLocation schema.".to_string(), url: url.clone(), recommendation: "Add a sport property to SportsActivityLocation.".to_string() });
            }
        }
        findings
    }
}

pub struct CivicStructureMissingNameValidator;
impl Default for CivicStructureMissingNameValidator { fn default() -> Self { Self::new() } }
impl CivicStructureMissingNameValidator { pub fn new() -> Self { Self } }
impl Analyzer for CivicStructureMissingNameValidator {
    fn name(&self) -> &str { "civic-structure-missing-name-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "CivicStructure" { continue; }
            if sd.data.get("name").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "CVNAME-V6010".to_string(), title: "CivicStructure missing name".to_string(), description: "No name in CivicStructure schema.".to_string(), url: url.clone(), recommendation: "Add a name to CivicStructure.".to_string() });
            }
        }
        findings
    }
}

pub struct LandformMissingNameValidator;
impl Default for LandformMissingNameValidator { fn default() -> Self { Self::new() } }
impl LandformMissingNameValidator { pub fn new() -> Self { Self } }
impl Analyzer for LandformMissingNameValidator {
    fn name(&self) -> &str { "landform-missing-name-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Landform" { continue; }
            if sd.data.get("name").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "LFNAME-V6011".to_string(), title: "Landform missing name".to_string(), description: "No name in Landform schema.".to_string(), url: url.clone(), recommendation: "Add a name to Landform.".to_string() });
            }
        }
        findings
    }
}

pub struct LandmarkMissingNameValidator;
impl Default for LandmarkMissingNameValidator { fn default() -> Self { Self::new() } }
impl LandmarkMissingNameValidator { pub fn new() -> Self { Self } }
impl Analyzer for LandmarkMissingNameValidator {
    fn name(&self) -> &str { "landmark-missing-name-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "LandmarksOrHistoricalBuildings" { continue; }
            if sd.data.get("name").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "LMNAME-V6012".to_string(), title: "Landmark missing name".to_string(), description: "No name in LandmarksOrHistoricalBuildings schema.".to_string(), url: url.clone(), recommendation: "Add a name to Landmark.".to_string() });
            }
        }
        findings
    }
}

pub struct TouristAttractionMissingNameValidator;
impl Default for TouristAttractionMissingNameValidator { fn default() -> Self { Self::new() } }
impl TouristAttractionMissingNameValidator { pub fn new() -> Self { Self } }
impl Analyzer for TouristAttractionMissingNameValidator {
    fn name(&self) -> &str { "tourist-attraction-missing-name-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "TouristAttraction" { continue; }
            if sd.data.get("name").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "TANAME-V6013".to_string(), title: "TouristAttraction missing name".to_string(), description: "No name in TouristAttraction schema.".to_string(), url: url.clone(), recommendation: "Add a name to TouristAttraction.".to_string() });
            }
        }
        findings
    }
}

pub struct TouristDestinationMissingNameValidator;
impl Default for TouristDestinationMissingNameValidator { fn default() -> Self { Self::new() } }
impl TouristDestinationMissingNameValidator { pub fn new() -> Self { Self } }
impl Analyzer for TouristDestinationMissingNameValidator {
    fn name(&self) -> &str { "tourist-destination-missing-name-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "TouristDestination" { continue; }
            if sd.data.get("name").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "TDNAME-V6014".to_string(), title: "TouristDestination missing name".to_string(), description: "No name in TouristDestination schema.".to_string(), url: url.clone(), recommendation: "Add a name to TouristDestination.".to_string() });
            }
        }
        findings
    }
}

pub struct SportsEventMissingSportValidator;
impl Default for SportsEventMissingSportValidator { fn default() -> Self { Self::new() } }
impl SportsEventMissingSportValidator { pub fn new() -> Self { Self } }
impl Analyzer for SportsEventMissingSportValidator {
    fn name(&self) -> &str { "sports-event-missing-sport-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "SportsEvent" { continue; }
            if sd.data.get("sport").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "SESPORT-V6015".to_string(), title: "SportsEvent missing sport".to_string(), description: "No sport in SportsEvent schema.".to_string(), url: url.clone(), recommendation: "Add sport to SportsEvent.".to_string() });
            }
        }
        findings
    }
}

pub struct SportsEventMissingNameValidator;
impl Default for SportsEventMissingNameValidator { fn default() -> Self { Self::new() } }
impl SportsEventMissingNameValidator { pub fn new() -> Self { Self } }
impl Analyzer for SportsEventMissingNameValidator {
    fn name(&self) -> &str { "sports-event-missing-name-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "SportsEvent" { continue; }
            if sd.data.get("name").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "SENAME-V6016".to_string(), title: "SportsEvent missing name".to_string(), description: "No name in SportsEvent schema.".to_string(), url: url.clone(), recommendation: "Add a name to SportsEvent.".to_string() });
            }
        }
        findings
    }
}

pub struct EducationalOrganizationMissingNameValidator;
impl Default for EducationalOrganizationMissingNameValidator { fn default() -> Self { Self::new() } }
impl EducationalOrganizationMissingNameValidator { pub fn new() -> Self { Self } }
impl Analyzer for EducationalOrganizationMissingNameValidator {
    fn name(&self) -> &str { "educational-organization-missing-name-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "EducationalOrganization" { continue; }
            if sd.data.get("name").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "EDUNAME-V6017".to_string(), title: "EducationalOrganization missing name".to_string(), description: "No name in EducationalOrganization schema.".to_string(), url: url.clone(), recommendation: "Add a name to EducationalOrganization.".to_string() });
            }
        }
        findings
    }
}

pub struct NGOMissingNameValidator;
impl Default for NGOMissingNameValidator { fn default() -> Self { Self::new() } }
impl NGOMissingNameValidator { pub fn new() -> Self { Self } }
impl Analyzer for NGOMissingNameValidator {
    fn name(&self) -> &str { "ngo-missing-name-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "NGO" { continue; }
            if sd.data.get("name").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "NGONAME-V6018".to_string(), title: "NGO missing name".to_string(), description: "No name in NGO schema.".to_string(), url: url.clone(), recommendation: "Add a name to NGO.".to_string() });
            }
        }
        findings
    }
}

pub struct PerformingArtsSeriesMissingNameValidator;
impl Default for PerformingArtsSeriesMissingNameValidator { fn default() -> Self { Self::new() } }
impl PerformingArtsSeriesMissingNameValidator { pub fn new() -> Self { Self } }
impl Analyzer for PerformingArtsSeriesMissingNameValidator {
    fn name(&self) -> &str { "performing-arts-series-missing-name-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "PerformingArtsSeries" { continue; }
            if sd.data.get("name").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "PASNAME-V6019".to_string(), title: "PerformingArtsSeries missing name".to_string(), description: "No name in PerformingArtsSeries schema.".to_string(), url: url.clone(), recommendation: "Add a name to PerformingArtsSeries.".to_string() });
            }
        }
        findings
    }
}

pub struct BroadcastEventMissingNameValidator;
impl Default for BroadcastEventMissingNameValidator { fn default() -> Self { Self::new() } }
impl BroadcastEventMissingNameValidator { pub fn new() -> Self { Self } }
impl Analyzer for BroadcastEventMissingNameValidator {
    fn name(&self) -> &str { "broadcast-event-missing-name-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "BroadcastEvent" { continue; }
            if sd.data.get("name").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "BENAME-V6020".to_string(), title: "BroadcastEvent missing name".to_string(), description: "No name in BroadcastEvent schema.".to_string(), url: url.clone(), recommendation: "Add a name to BroadcastEvent.".to_string() });
            }
        }
        findings
    }
}

pub struct ProductMissingBrandValidator;
impl Default for ProductMissingBrandValidator { fn default() -> Self { Self::new() } }
impl ProductMissingBrandValidator { pub fn new() -> Self { Self } }
impl Analyzer for ProductMissingBrandValidator {
    fn name(&self) -> &str { "product-missing-brand-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Product" { continue; }
            if sd.data.get("brand").is_none() {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "PRODBRAND-V6021".to_string(), title: "Product missing brand".to_string(), description: "No brand in Product schema.".to_string(), url: url.clone(), recommendation: "Add brand to Product.".to_string() });
            }
        }
        findings
    }
}

pub struct ProductMissingCategoryValidator;
impl Default for ProductMissingCategoryValidator { fn default() -> Self { Self::new() } }
impl ProductMissingCategoryValidator { pub fn new() -> Self { Self } }
impl Analyzer for ProductMissingCategoryValidator {
    fn name(&self) -> &str { "product-missing-category-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Product" { continue; }
            if sd.data.get("category").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Schema, code: "PRODCAT-V6022".to_string(), title: "Product missing category".to_string(), description: "No category in Product schema.".to_string(), url: url.clone(), recommendation: "Add a category to Product.".to_string() });
            }
        }
        findings
    }
}

pub struct ProductMissingReviewValidator;
impl Default for ProductMissingReviewValidator { fn default() -> Self { Self::new() } }
impl ProductMissingReviewValidator { pub fn new() -> Self { Self } }
impl Analyzer for ProductMissingReviewValidator {
    fn name(&self) -> &str { "product-missing-review-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Product" { continue; }
            if sd.data.get("review").is_none() && sd.data.get("aggregateRating").is_none() {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Schema, code: "PRODREV-V6023".to_string(), title: "Product missing review/aggregateRating".to_string(), description: "No review or aggregateRating in Product schema.".to_string(), url: url.clone(), recommendation: "Add review or aggregateRating to Product.".to_string() });
            }
        }
        findings
    }
}

pub struct BookMissingAuthorValidator;
impl Default for BookMissingAuthorValidator { fn default() -> Self { Self::new() } }
impl BookMissingAuthorValidator { pub fn new() -> Self { Self } }
impl Analyzer for BookMissingAuthorValidator {
    fn name(&self) -> &str { "book-missing-author-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Book" { continue; }
            if sd.data.get("author").is_none() {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "BOOKAUTH-V6024".to_string(), title: "Book missing author".to_string(), description: "No author in Book schema.".to_string(), url: url.clone(), recommendation: "Add author to Book.".to_string() });
            }
        }
        findings
    }
}

pub struct BookMissingIsbnValidator;
impl Default for BookMissingIsbnValidator { fn default() -> Self { Self::new() } }
impl BookMissingIsbnValidator { pub fn new() -> Self { Self } }
impl Analyzer for BookMissingIsbnValidator {
    fn name(&self) -> &str { "book-missing-isbn-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Book" { continue; }
            if sd.data.get("isbn").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Schema, code: "BOOKISBN-V6025".to_string(), title: "Book missing isbn".to_string(), description: "No isbn in Book schema.".to_string(), url: url.clone(), recommendation: "Add isbn to Book.".to_string() });
            }
        }
        findings
    }
}

pub struct BookMissingDatePublishedValidator;
impl Default for BookMissingDatePublishedValidator { fn default() -> Self { Self::new() } }
impl BookMissingDatePublishedValidator { pub fn new() -> Self { Self } }
impl Analyzer for BookMissingDatePublishedValidator {
    fn name(&self) -> &str { "book-missing-date-published-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Book" { continue; }
            if sd.data.get("datePublished").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Schema, code: "BOOKDATE-V6026".to_string(), title: "Book missing datePublished".to_string(), description: "No datePublished in Book schema.".to_string(), url: url.clone(), recommendation: "Add datePublished to Book.".to_string() });
            }
        }
        findings
    }
}

pub struct MovieMissingDirectorValidator;
impl Default for MovieMissingDirectorValidator { fn default() -> Self { Self::new() } }
impl MovieMissingDirectorValidator { pub fn new() -> Self { Self } }
impl Analyzer for MovieMissingDirectorValidator {
    fn name(&self) -> &str { "movie-missing-director-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Movie" { continue; }
            if sd.data.get("director").is_none() {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "MOVDIR-V6027".to_string(), title: "Movie missing director".to_string(), description: "No director in Movie schema.".to_string(), url: url.clone(), recommendation: "Add director to Movie.".to_string() });
            }
        }
        findings
    }
}

pub struct MovieMissingDurationValidator;
impl Default for MovieMissingDurationValidator { fn default() -> Self { Self::new() } }
impl MovieMissingDurationValidator { pub fn new() -> Self { Self } }
impl Analyzer for MovieMissingDurationValidator {
    fn name(&self) -> &str { "movie-missing-duration-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Movie" { continue; }
            if sd.data.get("duration").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Schema, code: "MOVDUR-V6028".to_string(), title: "Movie missing duration".to_string(), description: "No duration in Movie schema.".to_string(), url: url.clone(), recommendation: "Add ISO 8601 duration to Movie.".to_string() });
            }
        }
        findings
    }
}

pub struct MovieMissingDateCreatedValidator;
impl Default for MovieMissingDateCreatedValidator { fn default() -> Self { Self::new() } }
impl MovieMissingDateCreatedValidator { pub fn new() -> Self { Self } }
impl Analyzer for MovieMissingDateCreatedValidator {
    fn name(&self) -> &str { "movie-missing-date-created-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Movie" { continue; }
            if sd.data.get("dateCreated").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Schema, code: "MOVDATE-V6029".to_string(), title: "Movie missing dateCreated".to_string(), description: "No dateCreated in Movie schema.".to_string(), url: url.clone(), recommendation: "Add dateCreated to Movie.".to_string() });
            }
        }
        findings
    }
}

pub struct TVSeriesMissingNumberOfSeasonsValidator;
impl Default for TVSeriesMissingNumberOfSeasonsValidator { fn default() -> Self { Self::new() } }
impl TVSeriesMissingNumberOfSeasonsValidator { pub fn new() -> Self { Self } }
impl Analyzer for TVSeriesMissingNumberOfSeasonsValidator {
    fn name(&self) -> &str { "tv-series-missing-number-of-seasons-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "TVSeries" { continue; }
            if sd.data.get("numberOfSeasons").is_none() {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Schema, code: "TVSEASON-V6030".to_string(), title: "TVSeries missing numberOfSeasons".to_string(), description: "No numberOfSeasons in TVSeries schema.".to_string(), url: url.clone(), recommendation: "Add numberOfSeasons to TVSeries.".to_string() });
            }
        }
        findings
    }
}

pub struct TVSeriesMissingEpisodeValidator;
impl Default for TVSeriesMissingEpisodeValidator { fn default() -> Self { Self::new() } }
impl TVSeriesMissingEpisodeValidator { pub fn new() -> Self { Self } }
impl Analyzer for TVSeriesMissingEpisodeValidator {
    fn name(&self) -> &str { "tv-series-missing-episode-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "TVSeries" { continue; }
            if sd.data.get("episode").is_none() {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Schema, code: "TVEP-V6031".to_string(), title: "TVSeries missing episode".to_string(), description: "No episode in TVSeries schema.".to_string(), url: url.clone(), recommendation: "Add episode information to TVSeries.".to_string() });
            }
        }
        findings
    }
}

pub struct MusicRecordingMissingByArtistValidator;
impl Default for MusicRecordingMissingByArtistValidator { fn default() -> Self { Self::new() } }
impl MusicRecordingMissingByArtistValidator { pub fn new() -> Self { Self } }
impl Analyzer for MusicRecordingMissingByArtistValidator {
    fn name(&self) -> &str { "music-recording-missing-by-artist-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "MusicRecording" { continue; }
            if sd.data.get("byArtist").is_none() {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "MUSART-V6032".to_string(), title: "MusicRecording missing byArtist".to_string(), description: "No byArtist in MusicRecording schema.".to_string(), url: url.clone(), recommendation: "Add byArtist to MusicRecording.".to_string() });
            }
        }
        findings
    }
}

pub struct MusicRecordingMissingAlbumValidator;
impl Default for MusicRecordingMissingAlbumValidator { fn default() -> Self { Self::new() } }
impl MusicRecordingMissingAlbumValidator { pub fn new() -> Self { Self } }
impl Analyzer for MusicRecordingMissingAlbumValidator {
    fn name(&self) -> &str { "music-recording-missing-album-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "MusicRecording" { continue; }
            if sd.data.get("inAlbum").is_none() {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Schema, code: "MUSALB-V6033".to_string(), title: "MusicRecording missing inAlbum".to_string(), description: "No inAlbum in MusicRecording schema.".to_string(), url: url.clone(), recommendation: "Add inAlbum to MusicRecording.".to_string() });
            }
        }
        findings
    }
}

pub struct ServiceMissingAreaServedValidator;
impl Default for ServiceMissingAreaServedValidator { fn default() -> Self { Self::new() } }
impl ServiceMissingAreaServedValidator { pub fn new() -> Self { Self } }
impl Analyzer for ServiceMissingAreaServedValidator {
    fn name(&self) -> &str { "service-missing-area-served-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Service" { continue; }
            if sd.data.get("areaServed").is_none() {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Schema, code: "SVCAREA-V6034".to_string(), title: "Service missing areaServed".to_string(), description: "No areaServed in Service schema.".to_string(), url: url.clone(), recommendation: "Add areaServed to Service.".to_string() });
            }
        }
        findings
    }
}

pub struct ServiceMissingProviderValidator;
impl Default for ServiceMissingProviderValidator { fn default() -> Self { Self::new() } }
impl ServiceMissingProviderValidator { pub fn new() -> Self { Self } }
impl Analyzer for ServiceMissingProviderValidator {
    fn name(&self) -> &str { "service-missing-provider-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Service" { continue; }
            if sd.data.get("provider").is_none() {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "SVCPRV-V6035".to_string(), title: "Service missing provider".to_string(), description: "No provider in Service schema.".to_string(), url: url.clone(), recommendation: "Add provider to Service.".to_string() });
            }
        }
        findings
    }
}

pub struct HealthPlanMissingProviderValidator;
impl Default for HealthPlanMissingProviderValidator { fn default() -> Self { Self::new() } }
impl HealthPlanMissingProviderValidator { pub fn new() -> Self { Self } }
impl Analyzer for HealthPlanMissingProviderValidator {
    fn name(&self) -> &str { "health-plan-missing-provider-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "HealthPlan" { continue; }
            if sd.data.get("provider").is_none() {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "HPPRV-V6036".to_string(), title: "HealthPlan missing provider".to_string(), description: "No provider in HealthPlan schema.".to_string(), url: url.clone(), recommendation: "Add provider to HealthPlan.".to_string() });
            }
        }
        findings
    }
}

pub struct HealthPlanMissingCoverageAreaValidator;
impl Default for HealthPlanMissingCoverageAreaValidator { fn default() -> Self { Self::new() } }
impl HealthPlanMissingCoverageAreaValidator { pub fn new() -> Self { Self } }
impl Analyzer for HealthPlanMissingCoverageAreaValidator {
    fn name(&self) -> &str { "health-plan-missing-coverage-area-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "HealthPlan" { continue; }
            if sd.data.get("coverageArea").is_none() {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Schema, code: "HPCOV-V6037".to_string(), title: "HealthPlan missing coverageArea".to_string(), description: "No coverageArea in HealthPlan schema.".to_string(), url: url.clone(), recommendation: "Add coverageArea to HealthPlan.".to_string() });
            }
        }
        findings
    }
}

pub struct InvoiceMissingAccountValidator;
impl Default for InvoiceMissingAccountValidator { fn default() -> Self { Self::new() } }
impl InvoiceMissingAccountValidator { pub fn new() -> Self { Self } }
impl Analyzer for InvoiceMissingAccountValidator {
    fn name(&self) -> &str { "invoice-missing-account-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Invoice" { continue; }
            if sd.data.get("account").is_none() {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "INVACCT-V6038".to_string(), title: "Invoice missing account".to_string(), description: "No account in Invoice schema.".to_string(), url: url.clone(), recommendation: "Add account to Invoice.".to_string() });
            }
        }
        findings
    }
}

pub struct InvoiceMissingPaymentDueDateValidator;
impl Default for InvoiceMissingPaymentDueDateValidator { fn default() -> Self { Self::new() } }
impl InvoiceMissingPaymentDueDateValidator { pub fn new() -> Self { Self } }
impl Analyzer for InvoiceMissingPaymentDueDateValidator {
    fn name(&self) -> &str { "invoice-missing-payment-due-date-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Invoice" { continue; }
            if sd.data.get("paymentDueDate").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "INVPAY-V6039".to_string(), title: "Invoice missing paymentDueDate".to_string(), description: "No paymentDueDate in Invoice schema.".to_string(), url: url.clone(), recommendation: "Add paymentDueDate to Invoice.".to_string() });
            }
        }
        findings
    }
}

pub struct PermitMissingPermitNumberValidator;
impl Default for PermitMissingPermitNumberValidator { fn default() -> Self { Self::new() } }
impl PermitMissingPermitNumberValidator { pub fn new() -> Self { Self } }
impl Analyzer for PermitMissingPermitNumberValidator {
    fn name(&self) -> &str { "permit-missing-permit-number-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Permit" { continue; }
            if sd.data.get("permitNumber").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Schema, code: "PERMNUM-V6040".to_string(), title: "Permit missing permitNumber".to_string(), description: "No permitNumber in Permit schema.".to_string(), url: url.clone(), recommendation: "Add permitNumber to Permit.".to_string() });
            }
        }
        findings
    }
}

pub struct PermitMissingIssuedByValidator;
impl Default for PermitMissingIssuedByValidator { fn default() -> Self { Self::new() } }
impl PermitMissingIssuedByValidator { pub fn new() -> Self { Self } }
impl Analyzer for PermitMissingIssuedByValidator {
    fn name(&self) -> &str { "permit-missing-issued-by-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Permit" { continue; }
            if sd.data.get("issuedBy").is_none() {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Schema, code: "PERMISS-V6041".to_string(), title: "Permit missing issuedBy".to_string(), description: "No issuedBy in Permit schema.".to_string(), url: url.clone(), recommendation: "Add issuedBy to Permit.".to_string() });
            }
        }
        findings
    }
}

pub struct PlanMissingDescriptionValidator;
impl Default for PlanMissingDescriptionValidator { fn default() -> Self { Self::new() } }
impl PlanMissingDescriptionValidator { pub fn new() -> Self { Self } }
impl Analyzer for PlanMissingDescriptionValidator {
    fn name(&self) -> &str { "plan-missing-description-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Plan" { continue; }
            if sd.data.get("description").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Schema, code: "PLANDESC-V6042".to_string(), title: "Plan missing description".to_string(), description: "No description in Plan schema.".to_string(), url: url.clone(), recommendation: "Add a description to Plan.".to_string() });
            }
        }
        findings
    }
}

pub struct PlanMissingAboutValidator;
impl Default for PlanMissingAboutValidator { fn default() -> Self { Self::new() } }
impl PlanMissingAboutValidator { pub fn new() -> Self { Self } }
impl Analyzer for PlanMissingAboutValidator {
    fn name(&self) -> &str { "plan-missing-about-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Plan" { continue; }
            if sd.data.get("about").is_none() {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Schema, code: "PLANABOUT-V6043".to_string(), title: "Plan missing about".to_string(), description: "No about in Plan schema.".to_string(), url: url.clone(), recommendation: "Add about to Plan.".to_string() });
            }
        }
        findings
    }
}

pub struct ResearchProjectMissingAboutValidator;
impl Default for ResearchProjectMissingAboutValidator { fn default() -> Self { Self::new() } }
impl ResearchProjectMissingAboutValidator { pub fn new() -> Self { Self } }
impl Analyzer for ResearchProjectMissingAboutValidator {
    fn name(&self) -> &str { "research-project-missing-about-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "ResearchProject" { continue; }
            if sd.data.get("about").is_none() {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Schema, code: "RPABOUT-V6044".to_string(), title: "ResearchProject missing about".to_string(), description: "No about in ResearchProject schema.".to_string(), url: url.clone(), recommendation: "Add about to ResearchProject.".to_string() });
            }
        }
        findings
    }
}

pub struct ResearchProjectMissingFunderValidator;
impl Default for ResearchProjectMissingFunderValidator { fn default() -> Self { Self::new() } }
impl ResearchProjectMissingFunderValidator { pub fn new() -> Self { Self } }
impl Analyzer for ResearchProjectMissingFunderValidator {
    fn name(&self) -> &str { "research-project-missing-funder-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "ResearchProject" { continue; }
            if sd.data.get("funder").is_none() {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Schema, code: "RPFUND-V6045".to_string(), title: "ResearchProject missing funder".to_string(), description: "No funder in ResearchProject schema.".to_string(), url: url.clone(), recommendation: "Add funder to ResearchProject.".to_string() });
            }
        }
        findings
    }
}

pub struct ScheduleMissingTimezoneValidator;
impl Default for ScheduleMissingTimezoneValidator { fn default() -> Self { Self::new() } }
impl ScheduleMissingTimezoneValidator { pub fn new() -> Self { Self } }
impl Analyzer for ScheduleMissingTimezoneValidator {
    fn name(&self) -> &str { "schedule-missing-timezone-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Schedule" { continue; }
            if sd.data.get("scheduleTimezone").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Schema, code: "SCHEDTZ-V6046".to_string(), title: "Schedule missing scheduleTimezone".to_string(), description: "No scheduleTimezone in Schedule schema.".to_string(), url: url.clone(), recommendation: "Add scheduleTimezone to Schedule.".to_string() });
            }
        }
        findings
    }
}

pub struct TripMissingItineraryValidator;
impl Default for TripMissingItineraryValidator { fn default() -> Self { Self::new() } }
impl TripMissingItineraryValidator { pub fn new() -> Self { Self } }
impl Analyzer for TripMissingItineraryValidator {
    fn name(&self) -> &str { "trip-missing-itinerary-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Trip" { continue; }
            if sd.data.get("itinerary").is_none() {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Schema, code: "TRIPIT-V6047".to_string(), title: "Trip missing itinerary".to_string(), description: "No itinerary in Trip schema.".to_string(), url: url.clone(), recommendation: "Add itinerary to Trip.".to_string() });
            }
        }
        findings
    }
}

pub struct WorkersUnionMissingNameValidator;
impl Default for WorkersUnionMissingNameValidator { fn default() -> Self { Self::new() } }
impl WorkersUnionMissingNameValidator { pub fn new() -> Self { Self } }
impl Analyzer for WorkersUnionMissingNameValidator {
    fn name(&self) -> &str { "workers-union-missing-name-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "WorkersUnion" { continue; }
            if sd.data.get("name").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "WUNAME-V6048".to_string(), title: "WorkersUnion missing name".to_string(), description: "No name in WorkersUnion schema.".to_string(), url: url.clone(), recommendation: "Add a name to WorkersUnion.".to_string() });
            }
        }
        findings
    }
}

pub struct WebAPIMissingDocumentationValidator;
impl Default for WebAPIMissingDocumentationValidator { fn default() -> Self { Self::new() } }
impl WebAPIMissingDocumentationValidator { pub fn new() -> Self { Self } }
impl Analyzer for WebAPIMissingDocumentationValidator {
    fn name(&self) -> &str { "webapi-missing-documentation-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "WebAPI" { continue; }
            if sd.data.get("documentation").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Schema, code: "WAPIDOC-V6049".to_string(), title: "WebAPI missing documentation".to_string(), description: "No documentation in WebAPI schema.".to_string(), url: url.clone(), recommendation: "Add documentation URL to WebAPI.".to_string() });
            }
        }
        findings
    }
}

pub struct WearableMissingDeviceTypeValidator;
impl Default for WearableMissingDeviceTypeValidator { fn default() -> Self { Self::new() } }
impl WearableMissingDeviceTypeValidator { pub fn new() -> Self { Self } }
impl Analyzer for WearableMissingDeviceTypeValidator {
    fn name(&self) -> &str { "wearable-missing-device-type-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Wearable" { continue; }
            if sd.data.get("deviceType").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Schema, code: "WEARDEV-V6050".to_string(), title: "Wearable missing deviceType".to_string(), description: "No deviceType in Wearable schema.".to_string(), url: url.clone(), recommendation: "Add deviceType to Wearable.".to_string() });
            }
        }
        findings
    }
}

pub struct WebPageElementMissingNameValidator;
impl Default for WebPageElementMissingNameValidator { fn default() -> Self { Self::new() } }
impl WebPageElementMissingNameValidator { pub fn new() -> Self { Self } }
impl Analyzer for WebPageElementMissingNameValidator {
    fn name(&self) -> &str { "webpage-element-missing-name-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "WebPageElement" { continue; }
            if sd.data.get("name").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "WPELNAME-V6051".to_string(), title: "WebPageElement missing name".to_string(), description: "No name in WebPageElement schema.".to_string(), url: url.clone(), recommendation: "Add a name to WebPageElement.".to_string() });
            }
        }
        findings
    }
}

pub struct WorkerMissingJobTitleValidator;
impl Default for WorkerMissingJobTitleValidator { fn default() -> Self { Self::new() } }
impl WorkerMissingJobTitleValidator { pub fn new() -> Self { Self } }
impl Analyzer for WorkerMissingJobTitleValidator {
    fn name(&self) -> &str { "worker-missing-job-title-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Worker" { continue; }
            if sd.data.get("jobTitle").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Schema, code: "WORKJOB-V6052".to_string(), title: "Worker missing jobTitle".to_string(), description: "No jobTitle in Worker schema.".to_string(), url: url.clone(), recommendation: "Add jobTitle to Worker.".to_string() });
            }
        }
        findings
    }
}

pub struct CreativeWorkMissingLicenseValidator;
impl Default for CreativeWorkMissingLicenseValidator { fn default() -> Self { Self::new() } }
impl CreativeWorkMissingLicenseValidator { pub fn new() -> Self { Self } }
impl Analyzer for CreativeWorkMissingLicenseValidator {
    fn name(&self) -> &str { "creative-work-missing-license-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "CreativeWork" { continue; }
            if sd.data.get("license").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Schema, code: "CWLIC-V6053".to_string(), title: "CreativeWork missing license".to_string(), description: "No license in CreativeWork schema.".to_string(), url: url.clone(), recommendation: "Add license to CreativeWork.".to_string() });
            }
        }
        findings
    }
}

pub struct ProductMissingSkuValidator;
impl Default for ProductMissingSkuValidator { fn default() -> Self { Self::new() } }
impl ProductMissingSkuValidator { pub fn new() -> Self { Self } }
impl Analyzer for ProductMissingSkuValidator {
    fn name(&self) -> &str { "product-missing-sku-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Product" { continue; }
            if sd.data.get("sku").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Schema, code: "PRODSKU-V6054".to_string(), title: "Product missing sku".to_string(), description: "No sku in Product schema.".to_string(), url: url.clone(), recommendation: "Add sku to Product.".to_string() });
            }
        }
        findings
    }
}

pub struct BookMissingPublisherValidator;
impl Default for BookMissingPublisherValidator { fn default() -> Self { Self::new() } }
impl BookMissingPublisherValidator { pub fn new() -> Self { Self } }
impl Analyzer for BookMissingPublisherValidator {
    fn name(&self) -> &str { "book-missing-publisher-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Book" { continue; }
            if sd.data.get("publisher").is_none() {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Schema, code: "BOOKPUB-V6055".to_string(), title: "Book missing publisher".to_string(), description: "No publisher in Book schema.".to_string(), url: url.clone(), recommendation: "Add publisher to Book.".to_string() });
            }
        }
        findings
    }
}

pub struct MovieMissingActorValidator;
impl Default for MovieMissingActorValidator { fn default() -> Self { Self::new() } }
impl MovieMissingActorValidator { pub fn new() -> Self { Self } }
impl Analyzer for MovieMissingActorValidator {
    fn name(&self) -> &str { "movie-missing-actor-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Movie" { continue; }
            if sd.data.get("actor").is_none() {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Schema, code: "MOVACT-V6056".to_string(), title: "Movie missing actor".to_string(), description: "No actor in Movie schema.".to_string(), url: url.clone(), recommendation: "Add actor information to Movie.".to_string() });
            }
        }
        findings
    }
}

pub struct ServiceMissingServiceTypeValidator;
impl Default for ServiceMissingServiceTypeValidator { fn default() -> Self { Self::new() } }
impl ServiceMissingServiceTypeValidator { pub fn new() -> Self { Self } }
impl Analyzer for ServiceMissingServiceTypeValidator {
    fn name(&self) -> &str { "service-missing-service-type-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Service" { continue; }
            if sd.data.get("serviceType").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Schema, code: "SVCTYPE-V6057".to_string(), title: "Service missing serviceType".to_string(), description: "No serviceType in Service schema.".to_string(), url: url.clone(), recommendation: "Add serviceType to Service.".to_string() });
            }
        }
        findings
    }
}

pub struct EventMissingStartDateValidator;
impl Default for EventMissingStartDateValidator { fn default() -> Self { Self::new() } }
impl EventMissingStartDateValidator { pub fn new() -> Self { Self } }
impl Analyzer for EventMissingStartDateValidator {
    fn name(&self) -> &str { "event-missing-start-date-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Event" { continue; }
            if sd.data.get("startDate").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty()) {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "EVTSTART-V6058".to_string(), title: "Event missing startDate".to_string(), description: "No startDate in Event schema.".to_string(), url: url.clone(), recommendation: "Add startDate to Event.".to_string() });
            }
        }
        findings
    }
}

pub struct EventMissingLocationValidator;
impl Default for EventMissingLocationValidator { fn default() -> Self { Self::new() } }
impl EventMissingLocationValidator { pub fn new() -> Self { Self } }
impl Analyzer for EventMissingLocationValidator {
    fn name(&self) -> &str { "event-missing-location-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "Event" { continue; }
            if sd.data.get("location").is_none() {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "EVTLOC-V6059".to_string(), title: "Event missing location".to_string(), description: "No location in Event schema.".to_string(), url: url.clone(), recommendation: "Add location to Event.".to_string() });
            }
        }
        findings
    }
}

pub struct LocalBusinessMissingGeoValidator;
impl Default for LocalBusinessMissingGeoValidator { fn default() -> Self { Self::new() } }
impl LocalBusinessMissingGeoValidator { pub fn new() -> Self { Self } }
impl Analyzer for LocalBusinessMissingGeoValidator {
    fn name(&self) -> &str { "local-business-missing-geo-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let t = sd.r#type.as_deref().unwrap_or("");
            if t != "LocalBusiness" { continue; }
            if sd.data.get("geo").is_none() {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Schema, code: "LBGEO-V6060".to_string(), title: "LocalBusiness missing geo".to_string(), description: "No geo in LocalBusiness schema.".to_string(), url: url.clone(), recommendation: "Add geo coordinates to LocalBusiness.".to_string() });
            }
        }
        findings
    }
}

// =========================================================================
// V6 Security Validators (41-65)
// =========================================================================

pub struct CspConnectSrcValidator;
impl Default for CspConnectSrcValidator { fn default() -> Self { Self::new() } }
impl CspConnectSrcValidator { pub fn new() -> Self { Self } }
impl Analyzer for CspConnectSrcValidator {
    fn name(&self) -> &str { "csp-connect-src-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let csp = ctx.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case("Content-Security-Policy")).map(|(_, v)| v.as_str()).unwrap_or("");
        if csp.is_empty() { return findings; }
        if !csp.split(';').any(|d| d.trim().starts_with("connect-src") || d.trim().starts_with("default-src")) {
            findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Security, code: "CSPEXEC-V6061".to_string(), title: "CSP missing connect-src".to_string(), description: "No connect-src or default-src directive limits fetch/XHR targets.".to_string(), url: url.clone(), recommendation: "Add connect-src to restrict API endpoints.".to_string() });
        }
        findings
    }
}

pub struct CspFontSrcValidator;
impl Default for CspFontSrcValidator { fn default() -> Self { Self::new() } }
impl CspFontSrcValidator { pub fn new() -> Self { Self } }
impl Analyzer for CspFontSrcValidator {
    fn name(&self) -> &str { "csp-font-src-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let csp = ctx.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case("Content-Security-Policy")).map(|(_, v)| v.as_str()).unwrap_or("");
        if csp.is_empty() { return findings; }
        if !csp.split(';').any(|d| d.trim().starts_with("font-src") || d.trim().starts_with("default-src")) {
            findings.push(Finding { severity: Severity::Info, category: IssueCategory::Security, code: "CSPFONT-V6062".to_string(), title: "CSP missing font-src".to_string(), description: "No font-src directive limits font loading.".to_string(), url: url.clone(), recommendation: "Add font-src directive.".to_string() });
        }
        findings
    }
}

pub struct HstsMaxAgeThresholdValidator;
impl Default for HstsMaxAgeThresholdValidator { fn default() -> Self { Self::new() } }
impl HstsMaxAgeThresholdValidator { pub fn new() -> Self { Self } }
impl Analyzer for HstsMaxAgeThresholdValidator {
    fn name(&self) -> &str { "hsts-max-age-threshold-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(val) = ctx.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case("Strict-Transport-Security")).map(|(_, v)| v.as_str()) {
            let lower = val.to_lowercase();
            if let Some(pos) = lower.find("max-age=") {
                let after = &lower[pos + 8..];
                if let Ok(age) = after.chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse::<u64>() {
                    if age < 15768000 {
                        findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Security, code: "HSTSTHRESH-V6063".to_string(), title: "HSTS max-age below recommended threshold".to_string(), description: format!("max-age is {age}, recommended >= 15768000 (6 months)."), url: url.clone(), recommendation: "Set max-age to at least 15768000.".to_string() });
                    }
                }
            }
        }
        findings
    }
}

pub struct HstsPreloadListCheckValidator;
impl Default for HstsPreloadListCheckValidator { fn default() -> Self { Self::new() } }
impl HstsPreloadListCheckValidator { pub fn new() -> Self { Self } }
impl Analyzer for HstsPreloadListCheckValidator {
    fn name(&self) -> &str { "hsts-preload-list-check-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(val) = ctx.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case("Strict-Transport-Security")).map(|(_, v)| v.as_str()) {
            let lower = val.to_lowercase();
            if lower.contains("includesubdomains") && lower.contains("preload") {
                if let Some(pos) = lower.find("max-age=") {
                    let after = &lower[pos + 8..];
                    if let Ok(age) = after.chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse::<u64>() {
                        if age >= 31536000 {
                            return findings;
                        }
                    }
                }
            }
            findings.push(Finding { severity: Severity::Info, category: IssueCategory::Security, code: "HSTSPRELIST-V6064".to_string(), title: "HSTS preload readiness incomplete".to_string(), description: "Missing includeSubDomains, preload, or sufficient max-age for preload list.".to_string(), url: url.clone(), recommendation: "Add includeSubDomains; preload; max-age=31536000.".to_string() });
        }
        findings
    }
}

pub struct CookieSecureFlagDeepValidator;
impl Default for CookieSecureFlagDeepValidator { fn default() -> Self { Self::new() } }
impl CookieSecureFlagDeepValidator { pub fn new() -> Self { Self } }
impl Analyzer for CookieSecureFlagDeepValidator {
    fn name(&self) -> &str { "cookie-secure-flag-deep-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for (k, v) in ctx.headers {
            if !k.eq_ignore_ascii_case("set-cookie") { continue; }
            let lower = v.to_lowercase();
            let name = v.split('=').next().unwrap_or("cookie").trim().to_string();
            if !lower.contains("secure") {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Security, code: "COOKIESEC-V6065".to_string(), title: format!("Cookie '{name}' missing Secure flag"), description: "Cookie transmitted over HTTP.".to_string(), url: url.clone(), recommendation: "Add Secure flag to cookie.".to_string() });
            }
        }
        findings
    }
}

pub struct CookieHttpOnlyFlagDeepValidator;
impl Default for CookieHttpOnlyFlagDeepValidator { fn default() -> Self { Self::new() } }
impl CookieHttpOnlyFlagDeepValidator { pub fn new() -> Self { Self } }
impl Analyzer for CookieHttpOnlyFlagDeepValidator {
    fn name(&self) -> &str { "cookie-httponly-flag-deep-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for (k, v) in ctx.headers {
            if !k.eq_ignore_ascii_case("set-cookie") { continue; }
            let lower = v.to_lowercase();
            let name = v.split('=').next().unwrap_or("cookie").trim().to_string();
            if !lower.contains("httponly") {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Security, code: "COOKIEHTTP-V6066".to_string(), title: format!("Cookie '{name}' missing HttpOnly flag"), description: "Cookie accessible to JavaScript.".to_string(), url: url.clone(), recommendation: "Add HttpOnly flag to cookie.".to_string() });
            }
        }
        findings
    }
}

pub struct CookieSameSiteDeepValidator;
impl Default for CookieSameSiteDeepValidator { fn default() -> Self { Self::new() } }
impl CookieSameSiteDeepValidator { pub fn new() -> Self { Self } }
impl Analyzer for CookieSameSiteDeepValidator {
    fn name(&self) -> &str { "cookie-samesite-deep-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for (k, v) in ctx.headers {
            if !k.eq_ignore_ascii_case("set-cookie") { continue; }
            let lower = v.to_lowercase();
            let name = v.split('=').next().unwrap_or("cookie").trim().to_string();
            if !lower.contains("samesite") {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Security, code: "COOKIESAME-V6067".to_string(), title: format!("Cookie '{name}' missing SameSite attribute"), description: "Without SameSite, cookie is vulnerable to CSRF.".to_string(), url: url.clone(), recommendation: "Add SameSite=Strict or Lax.".to_string() });
            }
        }
        findings
    }
}

pub struct MixedContentIframeValidator;
impl Default for MixedContentIframeValidator { fn default() -> Self { Self::new() } }
impl MixedContentIframeValidator { pub fn new() -> Self { Self } }
impl Analyzer for MixedContentIframeValidator {
    fn name(&self) -> &str { "mixed-content-iframe-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if !url.starts_with("https://") { return findings; }
        if let Some(body) = ctx.body {
            let lower = body.to_lowercase();
            let http_iframes = lower.matches("<iframe").filter(|_| body.to_lowercase().contains("http://")).count();
            if http_iframes > 0 {
                findings.push(Finding { severity: Severity::Critical, category: IssueCategory::Security, code: "MIXIFRAME-V6068".to_string(), title: "Mixed content iframe detected".to_string(), description: format!("Found {http_iframes} iframe(s) loading HTTP content on HTTPS page."), url: url.clone(), recommendation: "Change iframe sources to HTTPS.".to_string() });
            }
        }
        findings
    }
}

pub struct CorsWildcardValidator;
impl Default for CorsWildcardValidator { fn default() -> Self { Self::new() } }
impl CorsWildcardValidator { pub fn new() -> Self { Self } }
impl Analyzer for CorsWildcardValidator {
    fn name(&self) -> &str { "cors-wildcard-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(val) = ctx.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case("Access-Control-Allow-Origin")).map(|(_, v)| v.as_str()) {
            if val == "*" {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Security, code: "CORSWILD-V6069".to_string(), title: "CORS allows all origins".to_string(), description: "Access-Control-Allow-Origin is set to '*'.".to_string(), url: url.clone(), recommendation: "Restrict CORS to specific trusted origins.".to_string() });
            }
        }
        findings
    }
}

pub struct CorsMissingHeaderValidator;
impl Default for CorsMissingHeaderValidator { fn default() -> Self { Self::new() } }
impl CorsMissingHeaderValidator { pub fn new() -> Self { Self } }
impl Analyzer for CorsMissingHeaderValidator {
    fn name(&self) -> &str { "cors-missing-header-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let has_cors = ctx.headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("Access-Control-Allow-Origin"));
        if !has_cors {
            findings.push(Finding { severity: Severity::Info, category: IssueCategory::Security, code: "CORSMISS-V6070".to_string(), title: "No CORS headers".to_string(), description: "No Access-Control-Allow-Origin header present.".to_string(), url: url.clone(), recommendation: "Add CORS headers if cross-origin requests are needed.".to_string() });
        }
        findings
    }
}

pub struct ReferrerPolicyStrictValidator;
impl Default for ReferrerPolicyStrictValidator { fn default() -> Self { Self::new() } }
impl ReferrerPolicyStrictValidator { pub fn new() -> Self { Self } }
impl Analyzer for ReferrerPolicyStrictValidator {
    fn name(&self) -> &str { "referrer-policy-strict-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(val) = ctx.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case("Referrer-Policy")).map(|(_, v)| v.as_str()) {
            let lower = val.to_lowercase();
            if lower.contains("unsafe-url") || lower.contains("no-referrer-when-downgrade") {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Security, code: "RPSTRICT-V6071".to_string(), title: "Referrer-Policy too permissive".to_string(), description: format!("'{val}' may leak sensitive URL information."), url: url.clone(), recommendation: "Use strict-origin-when-cross-origin or stricter.".to_string() });
            }
        }
        findings
    }
}

pub struct XFrameOptionsMissingValidator;
impl Default for XFrameOptionsMissingValidator { fn default() -> Self { Self::new() } }
impl XFrameOptionsMissingValidator { pub fn new() -> Self { Self } }
impl Analyzer for XFrameOptionsMissingValidator {
    fn name(&self) -> &str { "x-frame-options-missing-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let has_xfo = ctx.headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("X-Frame-Options"));
        let has_csp_frame = ctx.headers.iter().any(|(k, v)| k.eq_ignore_ascii_case("Content-Security-Policy") && v.contains("frame-ancestors"));
        if !has_xfo && !has_csp_frame {
            findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Security, code: "XFOMISS-V6072".to_string(), title: "No clickjacking protection".to_string(), description: "Neither X-Frame-Options nor CSP frame-ancestors set.".to_string(), url: url.clone(), recommendation: "Add X-Frame-Options: DENY or CSP frame-ancestors.".to_string() });
        }
        findings
    }
}

pub struct PermissionsPolicyPaymentValidator;
impl Default for PermissionsPolicyPaymentValidator { fn default() -> Self { Self::new() } }
impl PermissionsPolicyPaymentValidator { pub fn new() -> Self { Self } }
impl Analyzer for PermissionsPolicyPaymentValidator {
    fn name(&self) -> &str { "permissions-policy-payment-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let pp = ctx.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case("Permissions-Policy") || k.eq_ignore_ascii_case("Feature-Policy")).map(|(_, v)| v.as_str());
        if let Some(val) = pp {
            if val.contains("payment") && !val.contains("payment=()") {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Security, code: "PPPAY-V6073".to_string(), title: "Payment API access not explicitly denied".to_string(), description: "Permissions-Policy doesn't deny payment API.".to_string(), url: url.clone(), recommendation: "Add payment=() to Permissions-Policy.".to_string() });
            }
        }
        findings
    }
}

pub struct PermissionsPolicyFullscreenValidator;
impl Default for PermissionsPolicyFullscreenValidator { fn default() -> Self { Self::new() } }
impl PermissionsPolicyFullscreenValidator { pub fn new() -> Self { Self } }
impl Analyzer for PermissionsPolicyFullscreenValidator {
    fn name(&self) -> &str { "permissions-policy-fullscreen-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let pp = ctx.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case("Permissions-Policy") || k.eq_ignore_ascii_case("Feature-Policy")).map(|(_, v)| v.as_str());
        if let Some(val) = pp {
            if val.contains("fullscreen") && !val.contains("fullscreen=(self)") {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Security, code: "PPFULL-V6074".to_string(), title: "Fullscreen access not restricted to self".to_string(), description: "Permissions-Policy allows fullscreen from non-self origins.".to_string(), url: url.clone(), recommendation: "Add fullscreen=(self) to Permissions-Policy.".to_string() });
            }
        }
        findings
    }
}

pub struct PermissionsPolicyXrVrValidator;
impl Default for PermissionsPolicyXrVrValidator { fn default() -> Self { Self::new() } }
impl PermissionsPolicyXrVrValidator { pub fn new() -> Self { Self } }
impl Analyzer for PermissionsPolicyXrVrValidator {
    fn name(&self) -> &str { "permissions-policy-xr-vr-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let pp = ctx.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case("Permissions-Policy") || k.eq_ignore_ascii_case("Feature-Policy")).map(|(_, v)| v.as_str());
        if let Some(val) = pp {
            if val.contains("xr-spatial-tracking") && !val.contains("xr-spatial-tracking=()") {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Security, code: "PPXR-V6075".to_string(), title: "XR spatial tracking access not denied".to_string(), description: "Permissions-Policy doesn't deny XR spatial tracking.".to_string(), url: url.clone(), recommendation: "Add xr-spatial-tracking=() to Permissions-Policy.".to_string() });
            }
        }
        findings
    }
}

pub struct CoepRequireCorpValidator;
impl Default for CoepRequireCorpValidator { fn default() -> Self { Self::new() } }
impl CoepRequireCorpValidator { pub fn new() -> Self { Self } }
impl Analyzer for CoepRequireCorpValidator {
    fn name(&self) -> &str { "coep-require-corp-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(val) = ctx.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case("Cross-Origin-Embedder-Policy")).map(|(_, v)| v.as_str()) {
            if val.eq_ignore_ascii_case("unsafe-none") {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Security, code: "COEPU-V6076".to_string(), title: "COEP set to unsafe-none".to_string(), description: "COEP unsafe-none provides no isolation.".to_string(), url: url.clone(), recommendation: "Set COEP to require-corp.".to_string() });
            }
        }
        findings
    }
}

pub struct CoopSameOriginValidator;
impl Default for CoopSameOriginValidator { fn default() -> Self { Self::new() } }
impl CoopSameOriginValidator { pub fn new() -> Self { Self } }
impl Analyzer for CoopSameOriginValidator {
    fn name(&self) -> &str { "coop-same-origin-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(val) = ctx.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case("Cross-Origin-Opener-Policy")).map(|(_, v)| v.as_str()) {
            if val.eq_ignore_ascii_case("unsafe-none") {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Security, code: "COOPU-V6077".to_string(), title: "COOP set to unsafe-none".to_string(), description: "COOP unsafe-none provides no isolation.".to_string(), url: url.clone(), recommendation: "Set COOP to same-origin.".to_string() });
            }
        }
        findings
    }
}

pub struct CspObjectSrcNoneValidator;
impl Default for CspObjectSrcNoneValidator { fn default() -> Self { Self::new() } }
impl CspObjectSrcNoneValidator { pub fn new() -> Self { Self } }
impl Analyzer for CspObjectSrcNoneValidator {
    fn name(&self) -> &str { "csp-object-src-none-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let csp = ctx.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case("Content-Security-Policy")).map(|(_, v)| v.as_str()).unwrap_or("");
        if csp.is_empty() { return findings; }
        if let Some(directive) = csp.split(';').find(|d| d.trim().starts_with("object-src")) {
            let value = directive.trim().trim_start_matches("object-src").trim();
            if value != "'none'" && !value.is_empty() {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Security, code: "CSPOBJ-V6078".to_string(), title: "CSP object-src not set to 'none'".to_string(), description: format!("object-src is '{value}', recommend 'none'."), url: url.clone(), recommendation: "Set object-src to 'none'.".to_string() });
            }
        }
        findings
    }
}

pub struct CspBaseUriSelfValidator;
impl Default for CspBaseUriSelfValidator { fn default() -> Self { Self::new() } }
impl CspBaseUriSelfValidator { pub fn new() -> Self { Self } }
impl Analyzer for CspBaseUriSelfValidator {
    fn name(&self) -> &str { "csp-base-uri-self-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let csp = ctx.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case("Content-Security-Policy")).map(|(_, v)| v.as_str()).unwrap_or("");
        if csp.is_empty() { return findings; }
        if !csp.split(';').any(|d| d.trim().starts_with("base-uri")) {
            findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Security, code: "CSPBASE-V6079".to_string(), title: "CSP missing base-uri".to_string(), description: "Without base-uri, attackers can inject <base> tags.".to_string(), url: url.clone(), recommendation: "Add base-uri 'self'.".to_string() });
        }
        findings
    }
}

pub struct CspFormActionSelfValidator;
impl Default for CspFormActionSelfValidator { fn default() -> Self { Self::new() } }
impl CspFormActionSelfValidator { pub fn new() -> Self { Self } }
impl Analyzer for CspFormActionSelfValidator {
    fn name(&self) -> &str { "csp-form-action-self-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let csp = ctx.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case("Content-Security-Policy")).map(|(_, v)| v.as_str()).unwrap_or("");
        if csp.is_empty() { return findings; }
        if !csp.split(';').any(|d| d.trim().starts_with("form-action")) {
            findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Security, code: "CSPFORM-V6080".to_string(), title: "CSP missing form-action".to_string(), description: "Without form-action, forms could submit to attacker URLs.".to_string(), url: url.clone(), recommendation: "Add form-action 'self'.".to_string() });
        }
        findings
    }
}

// =========================================================================
// V6 SEO Validators (66-85)
// =========================================================================

pub struct TitleMissingValidator;
impl Default for TitleMissingValidator { fn default() -> Self { Self::new() } }
impl TitleMissingValidator { pub fn new() -> Self { Self } }
impl Analyzer for TitleMissingValidator {
    fn name(&self) -> &str { "title-missing-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.meta.title.as_deref().map_or(true, |t| t.is_empty()) {
            findings.push(Finding { severity: Severity::Critical, category: IssueCategory::Seo, code: "TITLEMISS-V6081".to_string(), title: "Missing title tag".to_string(), description: "No title tag found.".to_string(), url: url.clone(), recommendation: "Add a unique, descriptive title.".to_string() });
        }
        findings
    }
}

pub struct TitleKeywordDensityValidator;
impl Default for TitleKeywordDensityValidator { fn default() -> Self { Self::new() } }
impl TitleKeywordDensityValidator { pub fn new() -> Self { Self } }
impl Analyzer for TitleKeywordDensityValidator {
    fn name(&self) -> &str { "title-keyword-density-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(title) = &ctx.page.meta.title {
            if let Some(desc) = &ctx.page.meta.description {
                let title_lower = title.to_lowercase();
                let words: Vec<&str> = desc.split_whitespace().collect();
                if !words.is_empty() {
                    let first = words[0].to_lowercase();
                    if !title_lower.contains(&first) && first.len() > 3 {
                        findings.push(Finding { severity: Severity::Info, category: IssueCategory::Seo, code: "TITLEKDEN-V6082".to_string(), title: "Title keyword density low".to_string(), description: format!("First description word '{first}' not in title."), url: url.clone(), recommendation: "Include primary keyword in title.".to_string() });
                    }
                }
            }
        }
        findings
    }
}

pub struct TitleBrandPlacementValidator;
impl Default for TitleBrandPlacementValidator { fn default() -> Self { Self::new() } }
impl TitleBrandPlacementValidator { pub fn new() -> Self { Self } }
impl Analyzer for TitleBrandPlacementValidator {
    fn name(&self) -> &str { "title-brand-placement-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(title) = &ctx.page.meta.title {
            if let Some(sd) = ctx.page.structured_data.iter().find(|s| s.r#type.as_deref() == Some("Organization")) {
                if let Some(brand) = sd.data.get("name").and_then(|v| v.as_str()) {
                    let title_lower = title.to_lowercase();
                    let brand_lower = brand.to_lowercase();
                    if title_lower.contains(&brand_lower) {
                        if let Some(pos) = title_lower.find(&brand_lower) {
                            if pos > 0 && pos + brand.len() < title.len() {
                                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Seo, code: "TITLEBRAND-V6083".to_string(), title: "Brand not at start of title".to_string(), description: format!("Brand '{brand}' found at position {pos} in title."), url: url.clone(), recommendation: "Place brand at the start of title for recognition.".to_string() });
                            }
                        }
                    }
                }
            }
        }
        findings
    }
}

pub struct TitlePixelWidthValidator;
impl Default for TitlePixelWidthValidator { fn default() -> Self { Self::new() } }
impl TitlePixelWidthValidator { pub fn new() -> Self { Self } }
impl Analyzer for TitlePixelWidthValidator {
    fn name(&self) -> &str { "title-pixel-width-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(title) = &ctx.page.meta.title {
            let char_width: usize = title.chars().map(|c| if c.is_ascii() { 8 } else { 12 }).sum();
            if char_width > 580 {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Seo, code: "TITLEPX-V6084".to_string(), title: "Title may be truncated in SERPs".to_string(), description: format!("Estimated pixel width is {char_width}, max ~580."), url: url.clone(), recommendation: "Shorten title to fit within SERP pixel limit.".to_string() });
            }
        }
        findings
    }
}

pub struct MetaDescriptionMissingValidator;
impl Default for MetaDescriptionMissingValidator { fn default() -> Self { Self::new() } }
impl MetaDescriptionMissingValidator { pub fn new() -> Self { Self } }
impl Analyzer for MetaDescriptionMissingValidator {
    fn name(&self) -> &str { "meta-description-missing-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.meta.description.as_deref().map_or(true, |d| d.is_empty()) {
            findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Seo, code: "METADESCMISS-V6085".to_string(), title: "Missing meta description".to_string(), description: "No meta description tag found.".to_string(), url: url.clone(), recommendation: "Add a unique, compelling meta description.".to_string() });
        }
        findings
    }
}

pub struct MetaDescriptionTooShortValidator;
impl Default for MetaDescriptionTooShortValidator { fn default() -> Self { Self::new() } }
impl MetaDescriptionTooShortValidator { pub fn new() -> Self { Self } }
impl Analyzer for MetaDescriptionTooShortValidator {
    fn name(&self) -> &str { "meta-description-too-short-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(desc) = &ctx.page.meta.description {
            if desc.len() < 70 {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Seo, code: "METADESSSHORT-V6086".to_string(), title: "Meta description too short".to_string(), description: format!("Description is {} chars, recommend 120-160.", desc.len()), url: url.clone(), recommendation: "Expand meta description to 120-160 chars.".to_string() });
            }
        }
        findings
    }
}

pub struct MetaDescriptionUniquenessValidator;
impl Default for MetaDescriptionUniquenessValidator { fn default() -> Self { Self::new() } }
impl MetaDescriptionUniquenessValidator { pub fn new() -> Self { Self } }
impl Analyzer for MetaDescriptionUniquenessValidator {
    fn name(&self) -> &str { "meta-description-uniqueness-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let (Some(title), Some(desc)) = (&ctx.page.meta.title, &ctx.page.meta.description) {
            if title == desc {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Seo, code: "METADESCUNIQ-V6087".to_string(), title: "Meta description matches title".to_string(), description: "Title and description are identical.".to_string(), url: url.clone(), recommendation: "Write a unique meta description.".to_string() });
            }
        }
        findings
    }
}

pub struct CanonicalMissingValidator;
impl Default for CanonicalMissingValidator { fn default() -> Self { Self::new() } }
impl CanonicalMissingValidator { pub fn new() -> Self { Self } }
impl Analyzer for CanonicalMissingValidator {
    fn name(&self) -> &str { "canonical-missing-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.meta.canonical.is_none() {
            findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Seo, code: "CANMISS-V6088".to_string(), title: "Missing canonical URL".to_string(), description: "No canonical link tag found.".to_string(), url: url.clone(), recommendation: "Add a self-referencing canonical URL.".to_string() });
        }
        findings
    }
}

pub struct CanonicalSelfReferenceDeepValidator;
impl Default for CanonicalSelfReferenceDeepValidator { fn default() -> Self { Self::new() } }
impl CanonicalSelfReferenceDeepValidator { pub fn new() -> Self { Self } }
impl Analyzer for CanonicalSelfReferenceDeepValidator {
    fn name(&self) -> &str { "canonical-self-reference-deep-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(canonical) = &ctx.page.meta.canonical {
            if canonical.as_str() != url {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Seo, code: "CANSELFRF-V6089".to_string(), title: "Canonical does not self-reference".to_string(), description: format!("Canonical points to '{}', page is '{}'.", canonical, url), url: url.clone(), recommendation: "Set canonical to the current page URL.".to_string() });
            }
        }
        findings
    }
}

pub struct CanonicalChainDeepValidator;
impl Default for CanonicalChainDeepValidator { fn default() -> Self { Self::new() } }
impl CanonicalChainDeepValidator { pub fn new() -> Self { Self } }
impl Analyzer for CanonicalChainDeepValidator {
    fn name(&self) -> &str { "canonical-chain-deep-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(body) = ctx.body {
            let canonical_count = body.matches("rel=\"canonical\"").count() + body.matches("rel='canonical'").count();
            if canonical_count > 1 {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Seo, code: "CANCHAIN-V6090".to_string(), title: "Multiple canonical tags".to_string(), description: format!("{canonical_count} canonical tags found."), url: url.clone(), recommendation: "Use only one canonical tag per page.".to_string() });
            }
        }
        findings
    }
}

pub struct CanonicalDepthDeepValidator;
impl Default for CanonicalDepthDeepValidator { fn default() -> Self { Self::new() } }
impl CanonicalDepthDeepValidator { pub fn new() -> Self { Self } }
impl Analyzer for CanonicalDepthDeepValidator {
    fn name(&self) -> &str { "canonical-depth-deep-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(canonical) = &ctx.page.meta.canonical {
            let path = canonical.path();
            let depth = path.trim_matches('/').matches('/').count();
            if depth > 4 {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Seo, code: "CANDEEP-V6091".to_string(), title: "Canonical URL path deeply nested".to_string(), description: format!("Canonical path has {} levels.", depth + 1), url: url.clone(), recommendation: "Consider flattening URL structure.".to_string() });
            }
        }
        findings
    }
}

pub struct HreflangMissingValidator;
impl Default for HreflangMissingValidator { fn default() -> Self { Self::new() } }
impl HreflangMissingValidator { pub fn new() -> Self { Self } }
impl Analyzer for HreflangMissingValidator {
    fn name(&self) -> &str { "hreflang-missing-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.meta.hreflang.is_empty() {
            findings.push(Finding { severity: Severity::Info, category: IssueCategory::Seo, code: "HREFMISS-V6092".to_string(), title: "No hreflang tags".to_string(), description: "No hreflang link tags found.".to_string(), url: url.clone(), recommendation: "Add hreflang tags for international targeting.".to_string() });
        }
        findings
    }
}

pub struct HreflangReciprocalDeepValidator;
impl Default for HreflangReciprocalDeepValidator { fn default() -> Self { Self::new() } }
impl HreflangReciprocalDeepValidator { pub fn new() -> Self { Self } }
impl Analyzer for HreflangReciprocalDeepValidator {
    fn name(&self) -> &str { "hreflang-reciprocal-deep-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.meta.hreflang.len() > 1 {
            let langs: Vec<&str> = ctx.page.meta.hreflang.iter().map(|h| h.lang.as_str()).collect();
            let unique: std::collections::HashSet<&str> = langs.iter().copied().collect();
            if unique.len() < langs.len() {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Seo, code: "HREFRECIP-V6093".to_string(), title: "Duplicate hreflang languages".to_string(), description: format!("{} hreflang entries but {} unique languages.", langs.len(), unique.len()), url: url.clone(), recommendation: "Remove duplicate hreflang entries.".to_string() });
            }
        }
        findings
    }
}

pub struct HreflangXDefaultMissingValidator;
impl Default for HreflangXDefaultMissingValidator { fn default() -> Self { Self::new() } }
impl HreflangXDefaultMissingValidator { pub fn new() -> Self { Self } }
impl Analyzer for HreflangXDefaultMissingValidator {
    fn name(&self) -> &str { "hreflang-x-default-missing-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if !ctx.page.meta.hreflang.is_empty() {
            let has_xdefault = ctx.page.meta.hreflang.iter().any(|h| h.lang == "x-default");
            if !has_xdefault {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Seo, code: "HREFXD-V6094".to_string(), title: "Missing hreflang x-default".to_string(), description: "No x-default hreflang tag found.".to_string(), url: url.clone(), recommendation: "Add hreflang x-default for fallback language.".to_string() });
            }
        }
        findings
    }
}

pub struct HreflangLocaleFormatDeepValidator;
impl Default for HreflangLocaleFormatDeepValidator { fn default() -> Self { Self::new() } }
impl HreflangLocaleFormatDeepValidator { pub fn new() -> Self { Self } }
impl Analyzer for HreflangLocaleFormatDeepValidator {
    fn name(&self) -> &str { "hreflang-locale-format-deep-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for h in &ctx.page.meta.hreflang {
            if h.lang == "x-default" { continue; }
            if !h.lang.contains('-') && h.lang.len() > 2 {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Seo, code: "HREFFMT-V6095".to_string(), title: "Hreflang locale format".to_string(), description: format!("'{}' may not follow ISO 639-1/BCP 47 format.", h.lang), url: url.clone(), recommendation: "Use ISO 639-1 format (e.g., 'en', 'en-US').".to_string() });
            }
        }
        findings
    }
}

pub struct SitemapMissingValidator;
impl Default for SitemapMissingValidator { fn default() -> Self { Self::new() } }
impl SitemapMissingValidator { pub fn new() -> Self { Self } }
impl Analyzer for SitemapMissingValidator {
    fn name(&self) -> &str { "sitemap-missing-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(robots) = ctx.robots_txt {
            if !robots.to_lowercase().contains("sitemap:") {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Seo, code: "SITEMAPMISS-V6096".to_string(), title: "No sitemap in robots.txt".to_string(), description: "robots.txt doesn't reference a sitemap.".to_string(), url: url.clone(), recommendation: "Add Sitemap directive to robots.txt.".to_string() });
            }
        }
        findings
    }
}

pub struct SitemapLastmodFormatValidator;
impl Default for SitemapLastmodFormatValidator { fn default() -> Self { Self::new() } }
impl SitemapLastmodFormatValidator { pub fn new() -> Self { Self } }
impl Analyzer for SitemapLastmodFormatValidator {
    fn name(&self) -> &str { "sitemap-lastmod-format-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(body) = ctx.body {
            if body.contains("<urlset") || body.contains("<sitemapindex") {
                let lower = body.to_lowercase();
                let lastmod_count = lower.matches("<lastmod>").count();
                let valid_iso = body.matches("<lastmod>20").count() + body.matches("<lastmod>19").count();
                if lastmod_count > 0 && valid_iso < lastmod_count / 2 {
                    findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Seo, code: "SITEMAPLMFMT-V6097".to_string(), title: "Sitemap lastmod format issues".to_string(), description: format!("{lastmod_count} lastmod entries, many not ISO 8601."), url: url.clone(), recommendation: "Use W3C Datetime format for lastmod.".to_string() });
                }
            }
        }
        findings
    }
}

pub struct SitemapPriorityRangeValidator;
impl Default for SitemapPriorityRangeValidator { fn default() -> Self { Self::new() } }
impl SitemapPriorityRangeValidator { pub fn new() -> Self { Self } }
impl Analyzer for SitemapPriorityRangeValidator {
    fn name(&self) -> &str { "sitemap-priority-range-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(body) = ctx.body {
            if body.contains("<urlset") {
                for line in body.lines() {
                    if line.contains("<priority>") {
                        if let Some(start) = line.find("<priority>") {
                            let after = &line[start + 9..];
                            if let Some(end) = after.find("</priority>") {
                                let val = &after[..end];
                                if let Ok(p) = val.parse::<f64>() {
                                    if p < 0.0 || p > 1.0 {
                                        findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Seo, code: "SITEMAPPRI-V6098".to_string(), title: "Sitemap priority out of range".to_string(), description: format!("Priority {val} is outside 0.0-1.0."), url: url.clone(), recommendation: "Set priority between 0.0 and 1.0.".to_string() });
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        findings
    }
}

pub struct RobotsTxtEmptyValidator;
impl Default for RobotsTxtEmptyValidator { fn default() -> Self { Self::new() } }
impl RobotsTxtEmptyValidator { pub fn new() -> Self { Self } }
impl Analyzer for RobotsTxtEmptyValidator {
    fn name(&self) -> &str { "robots-txt-empty-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(robots) = ctx.robots_txt {
            if robots.trim().is_empty() {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Seo, code: "ROBOTSEMPTY-V6099".to_string(), title: "Empty robots.txt".to_string(), description: "robots.txt file is empty.".to_string(), url: url.clone(), recommendation: "Add directives to robots.txt.".to_string() });
            }
        }
        findings
    }
}

pub struct RobotsTxtDisallowDepthValidator;
impl Default for RobotsTxtDisallowDepthValidator { fn default() -> Self { Self::new() } }
impl RobotsTxtDisallowDepthValidator { pub fn new() -> Self { Self } }
impl Analyzer for RobotsTxtDisallowDepthValidator {
    fn name(&self) -> &str { "robots-txt-disallow-depth-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(robots) = ctx.robots_txt {
            let deep_blocks = robots.lines()
                .filter(|l| l.starts_with("Disallow:") || l.starts_with("disallow:"))
                .filter(|l| l.matches('/').count() > 5)
                .count();
            if deep_blocks > 0 {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Seo, code: "ROBOTSDEPTH-V6100".to_string(), title: "Deep disallow paths in robots.txt".to_string(), description: format!("{deep_blocks} disallow rule(s) with deep path depth."), url: url.clone(), recommendation: "Simplify disallow rules to directory level.".to_string() });
            }
        }
        findings
    }
}

pub struct RobotsTxtWildcardDisallowValidator;
impl Default for RobotsTxtWildcardDisallowValidator { fn default() -> Self { Self::new() } }
impl RobotsTxtWildcardDisallowValidator { pub fn new() -> Self { Self } }
impl Analyzer for RobotsTxtWildcardDisallowValidator {
    fn name(&self) -> &str { "robots-txt-wildcard-disallow-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(robots) = ctx.robots_txt {
            let block_all = robots.lines()
                .any(|l| (l.starts_with("Disallow:") || l.starts_with("disallow:")) && l.trim() == "Disallow: /");
            if block_all {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Seo, code: "ROBOTSWILD-V6101".to_string(), title: "robots.txt blocks all crawlers".to_string(), description: "Disallow: / blocks all crawling.".to_string(), url: url.clone(), recommendation: "Review disallow rules to avoid blocking everything.".to_string() });
            }
        }
        findings
    }
}

pub struct RobotsTxtMissingUserAgentValidator;
impl Default for RobotsTxtMissingUserAgentValidator { fn default() -> Self { Self::new() } }
impl RobotsTxtMissingUserAgentValidator { pub fn new() -> Self { Self } }
impl Analyzer for RobotsTxtMissingUserAgentValidator {
    fn name(&self) -> &str { "robots-txt-missing-user-agent-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(robots) = ctx.robots_txt {
            if !robots.to_lowercase().contains("user-agent:") {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Seo, code: "ROBOTSUA-V6102".to_string(), title: "robots.txt missing User-agent".to_string(), description: "No User-agent directive found.".to_string(), url: url.clone(), recommendation: "Add User-agent directive to robots.txt.".to_string() });
            }
        }
        findings
    }
}

pub struct InternalLinksDiversityValidator;
impl Default for InternalLinksDiversityValidator { fn default() -> Self { Self::new() } }
impl InternalLinksDiversityValidator { pub fn new() -> Self { Self } }
impl Analyzer for InternalLinksDiversityValidator {
    fn name(&self) -> &str { "internal-links-diversity-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let internal: Vec<&str> = ctx.page.links.iter()
            .filter(|l| !l.is_external)
            .map(|l| l.text.trim())
            .filter(|t| !t.is_empty())
            .collect();
        if internal.len() > 5 {
            let unique_texts: std::collections::HashSet<&str> = internal.iter().copied().collect();
            if unique_texts.len() < internal.len() / 2 {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Links, code: "INTDIV-V6103".to_string(), title: "Low internal link text diversity".to_string(), description: format!("{} internal links but only {} unique texts.", internal.len(), unique_texts.len()), url: url.clone(), recommendation: "Diversify anchor text for internal links.".to_string() });
            }
        }
        findings
    }
}

pub struct InternalLinksDepthDistributionValidator;
impl Default for InternalLinksDepthDistributionValidator { fn default() -> Self { Self::new() } }
impl InternalLinksDepthDistributionValidator { pub fn new() -> Self { Self } }
impl Analyzer for InternalLinksDepthDistributionValidator {
    fn name(&self) -> &str { "internal-links-depth-distribution-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let internal: Vec<&str> = ctx.page.links.iter().filter(|l| !l.is_external).map(|l| l.href.as_str()).collect();
        if internal.len() > 10 {
            let base = url::Url::parse(url).ok();
            if let Some(base) = base {
                let deep = internal.iter().filter(|href| {
                    url::Url::parse(href).ok().map_or(false, |u| {
                        u.path().matches('/').count() > 4 && u.host_str() == base.host_str()
                    })
                }).count();
                if deep > internal.len() / 3 {
                    findings.push(Finding { severity: Severity::Info, category: IssueCategory::Links, code: "INTDEPTH-V6104".to_string(), title: "Many deep internal links".to_string(), description: format!("{deep}/{} internal links point to deeply nested URLs.", internal.len()), url: url.clone(), recommendation: "Flatten URL structure for important pages.".to_string() });
                }
            }
        }
        findings
    }
}

pub struct ExternalLinksAuthorityScoreValidator;
impl Default for ExternalLinksAuthorityScoreValidator { fn default() -> Self { Self::new() } }
impl ExternalLinksAuthorityScoreValidator { pub fn new() -> Self { Self } }
impl Analyzer for ExternalLinksAuthorityScoreValidator {
    fn name(&self) -> &str { "external-links-authority-score-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let external: Vec<&str> = ctx.page.links.iter().filter(|l| l.is_external).map(|l| l.href.as_str()).collect();
        if external.len() > 5 {
            let high_authority = ["wikipedia.org", "github.com", "stackoverflow.com", "mozilla.org", "w3.org", "schema.org"];
            let authority_count = external.iter().filter(|href| {
                high_authority.iter().any(|dom| href.contains(dom))
            }).count();
            if authority_count == 0 {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Links, code: "EXTAUTH-V6105".to_string(), title: "No authoritative external links".to_string(), description: format!("{} external links, none to high-authority domains.", external.len()), url: url.clone(), recommendation: "Link to reputable, authoritative sources.".to_string() });
            }
        }
        findings
    }
}

pub struct ExternalLinksNofollowAnalysisValidator;
impl Default for ExternalLinksNofollowAnalysisValidator { fn default() -> Self { Self::new() } }
impl ExternalLinksNofollowAnalysisValidator { pub fn new() -> Self { Self } }
impl Analyzer for ExternalLinksNofollowAnalysisValidator {
    fn name(&self) -> &str { "external-links-nofollow-analysis-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let ext_total = ctx.page.links.iter().filter(|l| l.is_external).count();
        if ext_total > 3 {
            let nofollow_ext = ctx.page.links.iter().filter(|l| l.is_external && l.rel.iter().any(|r| r == "nofollow")).count();
            if nofollow_ext == 0 {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Links, code: "EXTNOFOLLOW-V6106".to_string(), title: "No nofollow on external links".to_string(), description: format!("{} external links, none nofollowed.", ext_total), url: url.clone(), recommendation: "Consider nofollow for untrusted external links.".to_string() });
            }
        }
        findings
    }
}

pub struct HeadingH1CountValidator;
impl Default for HeadingH1CountValidator { fn default() -> Self { Self::new() } }
impl HeadingH1CountValidator { pub fn new() -> Self { Self } }
impl Analyzer for HeadingH1CountValidator {
    fn name(&self) -> &str { "heading-h1-count-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let h1_count = ctx.page.headings.iter().filter(|h| h.level == 1).count();
        if h1_count == 0 {
            findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Accessibility, code: "H1COUNT-V6107".to_string(), title: "Missing H1 heading".to_string(), description: "No H1 heading found.".to_string(), url: url.clone(), recommendation: "Add a single H1 heading.".to_string() });
        } else if h1_count > 1 {
            findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Accessibility, code: "H1MULTI-V6108".to_string(), title: "Multiple H1 headings".to_string(), description: format!("{h1_count} H1 headings found."), url: url.clone(), recommendation: "Use a single H1 heading per page.".to_string() });
        }
        findings
    }
}

pub struct HeadingDepthAnalysisValidator;
impl Default for HeadingDepthAnalysisValidator { fn default() -> Self { Self::new() } }
impl HeadingDepthAnalysisValidator { pub fn new() -> Self { Self } }
impl Analyzer for HeadingDepthAnalysisValidator {
    fn name(&self) -> &str { "heading-depth-analysis-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(max_level) = ctx.page.headings.iter().map(|h| h.level).max() {
            if max_level >= 6 {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Accessibility, code: "HEADDEPTH-V6109".to_string(), title: "Headings reach H6 level".to_string(), description: "H6 headings may indicate overly deep content structure.".to_string(), url: url.clone(), recommendation: "Consider flattening heading hierarchy.".to_string() });
            }
        }
        findings
    }
}

pub struct FormRequiredFieldsDeepValidator;
impl Default for FormRequiredFieldsDeepValidator { fn default() -> Self { Self::new() } }
impl FormRequiredFieldsDeepValidator { pub fn new() -> Self { Self } }
impl Analyzer for FormRequiredFieldsDeepValidator {
    fn name(&self) -> &str { "form-required-fields-deep-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for form in &ctx.page.forms {
            let required_inputs: Vec<_> = form.inputs.iter().filter(|i| i.required).collect();
            let labeled: Vec<_> = required_inputs.iter().filter(|i| i.has_label || i.aria_label.is_some() || i.aria_labelledby.is_some()).collect();
            if !required_inputs.is_empty() && labeled.len() < required_inputs.len() {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Accessibility, code: "FORMREQ-V6110".to_string(), title: "Required fields missing labels".to_string(), description: format!("{}/{} required inputs lack labels.", required_inputs.len() - labeled.len(), required_inputs.len()), url: url.clone(), recommendation: "Add labels to all required form fields.".to_string() });
            }
        }
        findings
    }
}

pub struct TableHeadersScopeValidator;
impl Default for TableHeadersScopeValidator { fn default() -> Self { Self::new() } }
impl TableHeadersScopeValidator { pub fn new() -> Self { Self } }
impl Analyzer for TableHeadersScopeValidator {
    fn name(&self) -> &str { "table-headers-scope-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.tables_total > 0 && ctx.page.tables_with_headers == 0 {
            findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Accessibility, code: "TABSCOPE-V6111".to_string(), title: "Tables missing header scope".to_string(), description: format!("{} table(s) lack header cells with scope.", ctx.page.tables_total), url: url.clone(), recommendation: "Add scope attribute to <th> elements.".to_string() });
        }
        findings
    }
}

pub struct TableCaptionMissingDeepValidator;
impl Default for TableCaptionMissingDeepValidator { fn default() -> Self { Self::new() } }
impl TableCaptionMissingDeepValidator { pub fn new() -> Self { Self } }
impl Analyzer for TableCaptionMissingDeepValidator {
    fn name(&self) -> &str { "table-caption-missing-deep-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.tables_total > 0 && ctx.page.tables_with_captions == 0 {
            findings.push(Finding { severity: Severity::Info, category: IssueCategory::Accessibility, code: "TABCAP-V6112".to_string(), title: "Tables missing captions".to_string(), description: format!("{} table(s) lack <caption>.", ctx.page.tables_total), url: url.clone(), recommendation: "Add <caption> to each table.".to_string() });
        }
        findings
    }
}

pub struct LinkTextGenericDeepValidator;
impl Default for LinkTextGenericDeepValidator { fn default() -> Self { Self::new() } }
impl LinkTextGenericDeepValidator { pub fn new() -> Self { Self } }
impl Analyzer for LinkTextGenericDeepValidator {
    fn name(&self) -> &str { "link-text-generic-deep-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let generic = ["click here", "here", "read more", "more", "link", "learn more", "go", "this page"];
        let generic_count = ctx.page.links.iter()
            .filter(|l| {
                let text = l.text.trim().to_lowercase();
                generic.contains(&text.as_str())
            })
            .count();
        if generic_count > 0 {
            findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Accessibility, code: "LINKGEN-V6113".to_string(), title: "Generic link text".to_string(), description: format!("{generic_count} link(s) use generic text."), url: url.clone(), recommendation: "Use descriptive link text.".to_string() });
        }
        findings
    }
}

pub struct LinkTextEmptyDeepValidator;
impl Default for LinkTextEmptyDeepValidator { fn default() -> Self { Self::new() } }
impl LinkTextEmptyDeepValidator { pub fn new() -> Self { Self } }
impl Analyzer for LinkTextEmptyDeepValidator {
    fn name(&self) -> &str { "link-text-empty-deep-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let empty_count = ctx.page.links.iter()
            .filter(|l| l.text.trim().is_empty() && l.aria_label.is_none())
            .count();
        if empty_count > 0 {
            findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Accessibility, code: "LINKEEMPTY-V6114".to_string(), title: "Empty link text".to_string(), description: format!("{empty_count} link(s) have no text or aria-label."), url: url.clone(), recommendation: "Add descriptive text or aria-label to links.".to_string() });
        }
        findings
    }
}

pub struct LinkTextDuplicateValidator;
impl Default for LinkTextDuplicateValidator { fn default() -> Self { Self::new() } }
impl LinkTextDuplicateValidator { pub fn new() -> Self { Self } }
impl Analyzer for LinkTextDuplicateValidator {
    fn name(&self) -> &str { "link-text-duplicate-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let texts: Vec<&str> = ctx.page.links.iter().filter(|l| !l.text.trim().is_empty()).map(|l| l.text.trim()).collect();
        if texts.len() > 5 {
            let mut counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
            for t in &texts { *counts.entry(t).or_insert(0) += 1; }
            let dupes: Vec<(&&str, &usize)> = counts.iter().filter(|(_, &c)| c > 3).collect();
            if !dupes.is_empty() {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Accessibility, code: "LINKDUP-V6115".to_string(), title: "Duplicate link text".to_string(), description: format!("{} link text(s) appear 3+ times.", dupes.len()), url: url.clone(), recommendation: "Use unique, descriptive text for each link.".to_string() });
            }
        }
        findings
    }
}

pub struct ImageAltMissingDeepValidator;
impl Default for ImageAltMissingDeepValidator { fn default() -> Self { Self::new() } }
impl ImageAltMissingDeepValidator { pub fn new() -> Self { Self } }
impl Analyzer for ImageAltMissingDeepValidator {
    fn name(&self) -> &str { "image-alt-missing-deep-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let missing = ctx.page.images.iter().filter(|i| !i.has_alt && i.alt.is_empty()).count();
        if missing > 0 {
            findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Accessibility, code: "IMGALTMISS-V6116".to_string(), title: "Images missing alt text".to_string(), description: format!("{missing} image(s) lack alt text."), url: url.clone(), recommendation: "Add descriptive alt text to images.".to_string() });
        }
        findings
    }
}

pub struct ImageAltEmptyDeepValidator;
impl Default for ImageAltEmptyDeepValidator { fn default() -> Self { Self::new() } }
impl ImageAltEmptyDeepValidator { pub fn new() -> Self { Self } }
impl Analyzer for ImageAltEmptyDeepValidator {
    fn name(&self) -> &str { "image-alt-empty-deep-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let empty = ctx.page.images.iter().filter(|i| i.has_alt && i.alt.is_empty()).count();
        if empty > 0 {
            findings.push(Finding { severity: Severity::Info, category: IssueCategory::Accessibility, code: "IMGALTEMPTY-V6117".to_string(), title: "Empty alt attributes".to_string(), description: format!("{empty} image(s) have empty alt attributes."), url: url.clone(), recommendation: "Add meaningful alt text or mark as decorative.".to_string() });
        }
        findings
    }
}

pub struct ImageAltDecorativePatternValidator;
impl Default for ImageAltDecorativePatternValidator { fn default() -> Self { Self::new() } }
impl ImageAltDecorativePatternValidator { pub fn new() -> Self { Self } }
impl Analyzer for ImageAltDecorativePatternValidator {
    fn name(&self) -> &str { "image-alt-decorative-pattern-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let decorative_patterns = ["image", "photo", "picture", "img", "decorative"];
        let suspicious = ctx.page.images.iter().filter(|i| {
            i.has_alt && !i.alt.is_empty() && {
                let lower = i.alt.to_lowercase();
                decorative_patterns.iter().any(|p| lower == *p)
            }
        }).count();
        if suspicious > 2 {
            findings.push(Finding { severity: Severity::Info, category: IssueCategory::Accessibility, code: "IMGDECPAT-V6118".to_string(), title: "Suspiciously generic alt text".to_string(), description: format!("{suspicious} image(s) have generic alt text."), url: url.clone(), recommendation: "Provide descriptive alt text instead of generic words.".to_string() });
        }
        findings
    }
}

pub struct FocusTabindexPositiveValidator;
impl Default for FocusTabindexPositiveValidator { fn default() -> Self { Self::new() } }
impl FocusTabindexPositiveValidator { pub fn new() -> Self { Self } }
impl Analyzer for FocusTabindexPositiveValidator {
    fn name(&self) -> &str { "focus-tabindex-positive-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.has_positive_tabindex {
            findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Accessibility, code: "FOCUSTABPOS-V6119".to_string(), title: "Positive tabindex detected".to_string(), description: "Positive tabindex disrupts natural tab order.".to_string(), url: url.clone(), recommendation: "Remove positive tabindex and use DOM order.".to_string() });
        }
        findings
    }
}

pub struct FocusTrapMissingValidator;
impl Default for FocusTrapMissingValidator { fn default() -> Self { Self::new() } }
impl FocusTrapMissingValidator { pub fn new() -> Self { Self } }
impl Analyzer for FocusTrapMissingValidator {
    fn name(&self) -> &str { "focus-trap-missing-v6" }
    fn analyze(&self, _ctx: &AnalysisContext) -> Vec<Finding> {
        Vec::new()
    }
}

pub struct HeadingSkipLevelsDeepValidator;
impl Default for HeadingSkipLevelsDeepValidator { fn default() -> Self { Self::new() } }
impl HeadingSkipLevelsDeepValidator { pub fn new() -> Self { Self } }
impl Analyzer for HeadingSkipLevelsDeepValidator {
    fn name(&self) -> &str { "heading-skip-levels-deep-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.headings.len() < 2 { return findings; }
        let mut skip_count = 0;
        let mut prev_level = 0u8;
        for h in &ctx.page.headings {
            if prev_level > 0 && h.level > prev_level + 1 {
                skip_count += 1;
            }
            prev_level = h.level;
        }
        if skip_count > 0 {
            findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Accessibility, code: "HEADSKIP-V6121".to_string(), title: "Heading levels skipped".to_string(), description: format!("{skip_count} heading level skip(s) detected."), url: url.clone(), recommendation: "Use heading levels sequentially.".to_string() });
        }
        findings
    }
}

pub struct HeadingEmptyValidator;
impl Default for HeadingEmptyValidator { fn default() -> Self { Self::new() } }
impl HeadingEmptyValidator { pub fn new() -> Self { Self } }
impl Analyzer for HeadingEmptyValidator {
    fn name(&self) -> &str { "heading-empty-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let empty = ctx.page.headings.iter().filter(|h| h.text.trim().is_empty()).count();
        if empty > 0 {
            findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Accessibility, code: "HEADEMPTY-V6122".to_string(), title: "Empty headings".to_string(), description: format!("{empty} heading(s) have no text."), url: url.clone(), recommendation: "Add meaningful text to all headings.".to_string() });
        }
        findings
    }
}

pub struct FormFieldsetLegendValidator;
impl Default for FormFieldsetLegendValidator { fn default() -> Self { Self::new() } }
impl FormFieldsetLegendValidator { pub fn new() -> Self { Self } }
impl Analyzer for FormFieldsetLegendValidator {
    fn name(&self) -> &str { "form-fieldset-legend-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let multi_input_forms = ctx.page.forms.iter().filter(|f| f.inputs.len() > 3).count();
        if multi_input_forms > 0 {
            findings.push(Finding { severity: Severity::Info, category: IssueCategory::Accessibility, code: "FORMFSLG-V6123".to_string(), title: "Complex forms without fieldset".to_string(), description: format!("{multi_input_forms} form(s) with 3+ inputs may benefit from fieldset/legend."), url: url.clone(), recommendation: "Group related inputs with <fieldset> and <legend>.".to_string() });
        }
        findings
    }
}

pub struct LandmarkContentinfoValidator;
impl Default for LandmarkContentinfoValidator { fn default() -> Self { Self::new() } }
impl LandmarkContentinfoValidator { pub fn new() -> Self { Self } }
impl Analyzer for LandmarkContentinfoValidator {
    fn name(&self) -> &str { "landmark-contentinfo-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if !ctx.page.landmarks.iter().any(|l| l.to_lowercase() == "contentinfo") {
            findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Accessibility, code: "LANDCINFO-V6124".to_string(), title: "Missing contentinfo landmark".to_string(), description: "No contentinfo (footer) landmark found.".to_string(), url: url.clone(), recommendation: "Add a <footer> or role='contentinfo' landmark.".to_string() });
        }
        findings
    }
}

pub struct LandmarkComplementaryValidator;
impl Default for LandmarkComplementaryValidator { fn default() -> Self { Self::new() } }
impl LandmarkComplementaryValidator { pub fn new() -> Self { Self } }
impl Analyzer for LandmarkComplementaryValidator {
    fn name(&self) -> &str { "landmark-complementary-v6" }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if !ctx.page.landmarks.iter().any(|l| l.to_lowercase() == "complementary") {
            findings.push(Finding { severity: Severity::Info, category: IssueCategory::Accessibility, code: "LANDCOMP-V6125".to_string(), title: "No complementary landmark".to_string(), description: "No complementary (aside) landmark found.".to_string(), url: url.clone(), recommendation: "Consider adding <aside> or role='complementary'.".to_string() });
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

    // ===== Content V5 Tests =====
    #[test]
    fn test_article_word_count_no_sd() { let p = make_page("https://example.com"); assert!(ArticleWordCountAnalyzer::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_article_word_count_short() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Article".into()), data: serde_json::json!({}) }]; p.word_count = 100; let f = ArticleWordCountAnalyzer::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); assert_eq!(f[0].code, "ARTWC-V5001"); }
    #[test]
    fn test_article_word_count_ok() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Article".into()), data: serde_json::json!({}) }]; p.word_count = 500; assert!(ArticleWordCountAnalyzer::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_article_author_url_no_url() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Article".into()), data: serde_json::json!({"author": {"@type": "Person", "name": "John"}}) }]; let f = ArticleAuthorUrlValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_article_author_url_string() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Article".into()), data: serde_json::json!({"author": "John"}) }]; assert!(ArticleAuthorUrlValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_article_date_modified_missing() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("BlogPosting".into()), data: serde_json::json!({}) }]; let f = ArticleDateModifiedValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_article_date_modified_present() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Article".into()), data: serde_json::json!({"dateModified": "2025-01-01"}) }]; assert!(ArticleDateModifiedValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_org_url_missing() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Organization".into()), data: serde_json::json!({"name": "Acme"}) }]; let f = OrganizationUrlValidatorV5::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_org_url_present() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Organization".into()), data: serde_json::json!({"url": "https://example.com"}) }]; assert!(OrganizationUrlValidatorV5::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_org_logo_url_missing() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Organization".into()), data: serde_json::json!({"logo": {"@type": "ImageObject"}}) }]; let f = OrganizationLogoUrlValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_org_logo_string() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Organization".into()), data: serde_json::json!({"logo": "https://example.com/logo.png"}) }]; assert!(OrganizationLogoUrlValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_org_contact_missing() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Organization".into()), data: serde_json::json!({}) }]; let f = OrganizationContactValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_person_url_missing() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Person".into()), data: serde_json::json!({"name": "Jane"}) }]; let f = PersonUrlValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_person_url_ok() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Person".into()), data: serde_json::json!({"url": "https://example.com/jane"}) }]; assert!(PersonUrlValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_job_employment_type_invalid() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("JobPosting".into()), data: serde_json::json!({"employmentType": "FULL"}) }]; let f = JobPostingEmploymentTypeValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_job_employment_type_valid() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("JobPosting".into()), data: serde_json::json!({"employmentType": "FULL_TIME"}) }]; assert!(JobPostingEmploymentTypeValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_job_salary_currency_invalid() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("JobPosting".into()), data: serde_json::json!({"baseSalary": {"currency": "US"}}) }]; let f = JobPostingSalaryCurrencyValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_job_valid_through_missing() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("JobPosting".into()), data: serde_json::json!({}) }]; let f = JobPostingValidThroughValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_course_desc_missing() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Course".into()), data: serde_json::json!({}) }]; let f = CourseDescriptionValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_course_provider_name_missing() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Course".into()), data: serde_json::json!({"provider": {"@type": "Organization"}}) }]; let f = CourseProviderNameValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_recipe_prep_time_missing() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Recipe".into()), data: serde_json::json!({}) }]; let f = RecipePrepTimeValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_recipe_ingredients_missing() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Recipe".into()), data: serde_json::json!({}) }]; let f = RecipeIngredientsValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_recipe_ingredients_empty() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Recipe".into()), data: serde_json::json!({"recipeIngredient": []}) }]; let f = RecipeIngredientsValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_product_price_missing() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Product".into()), data: serde_json::json!({"offers": {"@type": "Offer"}}) }]; let f = ProductPriceValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_product_image_missing() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Product".into()), data: serde_json::json!({}) }]; let f = ProductImageValidatorV5::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_breadcrumb_item_count() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("BreadcrumbList".into()), data: serde_json::json!({"itemListElement": [{"@type": "ListItem"}]}) }]; let f = BreadcrumbItemCountValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_breadcrumb_url_relative() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("BreadcrumbList".into()), data: serde_json::json!({"itemListElement": [{"@type": "ListItem", "item": {"@id": "/about"}}]}) }]; let f = BreadcrumbUrlValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_event_organizer_missing() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Event".into()), data: serde_json::json!({}) }]; let f = EventOrganizerValidatorV5::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_event_organizer_no_name() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Event".into()), data: serde_json::json!({"organizer": {"@type": "Organization"}}) }]; let f = EventOrganizerValidatorV5::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_event_performer_missing() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Event".into()), data: serde_json::json!({}) }]; let f = EventPerformerValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_video_thumbnail_missing() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("VideoObject".into()), data: serde_json::json!({}) }]; let f = VideoThumbnailValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_video_duration_format() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("VideoObject".into()), data: serde_json::json!({"duration": "5min"}) }]; let f = VideoDurationFormatValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_video_duration_iso() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("VideoObject".into()), data: serde_json::json!({"duration": "PT5M"}) }]; assert!(VideoDurationFormatValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_software_offers_missing() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("SoftwareApplication".into()), data: serde_json::json!({}) }]; let f = SoftwareOffersValidatorV5::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_software_screenshot_missing() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("SoftwareApplication".into()), data: serde_json::json!({}) }]; let f = SoftwareScreenshotValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_faq_answer_short() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("FAQPage".into()), data: serde_json::json!({"mainEntity": [{"@type": "Question", "acceptedAnswer": {"@type": "Answer", "text": "Short"}}]}) }]; let f = FAQAnswerLengthValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_faq_answer_ok() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("FAQPage".into()), data: serde_json::json!({"mainEntity": [{"@type": "Question", "acceptedAnswer": {"@type": "Answer", "text": "This is a much longer answer that should be well over the fifty character minimum threshold."}}]}) }]; assert!(FAQAnswerLengthValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_howto_name_missing() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("HowTo".into()), data: serde_json::json!({}) }]; let f = HowToNameValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_howto_step_desc_missing() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("HowTo".into()), data: serde_json::json!({"step": [{"@type": "HowToStep"}]}) }]; let f = HowToStepDescriptionValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_dataset_license_missing() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Dataset".into()), data: serde_json::json!({}) }]; let f = DatasetLicenseValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_dataset_dist_no_url() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Dataset".into()), data: serde_json::json!({"distribution": [{"@type": "DataDownload"}]}) }]; let f = DatasetDistributionValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }

    // ===== Security V5 Tests =====
    #[test]
    fn test_csp_script_src_empty() { assert!(CspScriptSrcValidator::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_csp_script_src_unsafe_inline() { let p = make_page("https://example.com"); let headers = vec![("Content-Security-Policy".into(), "script-src 'unsafe-inline'".into())]; let ctx = AnalysisContext { page: &p, body: None, status_code: Some(200), headers: &headers, response_time: None, redirect_chain: &[], robots_txt: None, body_size: None, compressed_size: None, server: None, content_type: None, rendered: None }; let f = CspScriptSrcValidator::new().analyze(&ctx); assert!(f.iter().any(|x| x.code == "CSPSSRC-V5001")); }
    #[test]
    fn test_csp_script_src_wildcard() { let p = make_page("https://example.com"); let headers = vec![("Content-Security-Policy".into(), "script-src *".into())]; let ctx = AnalysisContext { page: &p, body: None, status_code: Some(200), headers: &headers, response_time: None, redirect_chain: &[], robots_txt: None, body_size: None, compressed_size: None, server: None, content_type: None, rendered: None }; let f = CspScriptSrcValidator::new().analyze(&ctx); assert!(f.iter().any(|x| x.code == "CSPSSRC-V5003")); }
    #[test]
    fn test_csp_script_src_missing() { let p = make_page("https://example.com"); let headers = vec![("Content-Security-Policy".into(), "default-src 'self'".into())]; let ctx = AnalysisContext { page: &p, body: None, status_code: Some(200), headers: &headers, response_time: None, redirect_chain: &[], robots_txt: None, body_size: None, compressed_size: None, server: None, content_type: None, rendered: None }; let f = CspScriptSrcValidator::new().analyze(&ctx); assert!(f.iter().any(|x| x.code == "CSPSSRC-V5004")); }
    #[test]
    fn test_csp_style_src_empty() { assert!(CspStyleSrcValidator::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_csp_frame_ancestors_missing() { let p = make_page("https://example.com"); let headers = vec![("Content-Security-Policy".into(), "script-src 'self'".into())]; let ctx = AnalysisContext { page: &p, body: None, status_code: Some(200), headers: &headers, response_time: None, redirect_chain: &[], robots_txt: None, body_size: None, compressed_size: None, server: None, content_type: None, rendered: None }; let f = CspFrameAncestorsValidator::new().analyze(&ctx); assert!(!f.is_empty()); }
    #[test]
    fn test_hsts_max_age_missing() { let p = make_page("https://example.com"); let f = HstsMaxAgeValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_hsts_max_age_low() { let p = make_page("https://example.com"); let headers = vec![("Strict-Transport-Security".into(), "max-age=100".into())]; let ctx = AnalysisContext { page: &p, body: None, status_code: Some(200), headers: &headers, response_time: None, redirect_chain: &[], robots_txt: None, body_size: None, compressed_size: None, server: None, content_type: None, rendered: None }; let f = HstsMaxAgeValidator::new().analyze(&ctx); assert!(f.iter().any(|x| x.code == "HSTSMAX-V5002")); }
    #[test]
    fn test_hsts_max_age_ok() { let p = make_page("https://example.com"); let headers = vec![("Strict-Transport-Security".into(), "max-age=31536000".into())]; let ctx = AnalysisContext { page: &p, body: None, status_code: Some(200), headers: &headers, response_time: None, redirect_chain: &[], robots_txt: None, body_size: None, compressed_size: None, server: None, content_type: None, rendered: None }; assert!(HstsMaxAgeValidator::new().analyze(&ctx).is_empty()); }
    #[test]
    fn test_hsts_include_subdomains() { let p = make_page("https://example.com"); let headers = vec![("Strict-Transport-Security".into(), "max-age=31536000".into())]; let ctx = AnalysisContext { page: &p, body: None, status_code: Some(200), headers: &headers, response_time: None, redirect_chain: &[], robots_txt: None, body_size: None, compressed_size: None, server: None, content_type: None, rendered: None }; let f = HstsIncludeSubDomainsValidator::new().analyze(&ctx); assert!(!f.is_empty()); }
    #[test]
    fn test_hsts_preload_missing() { let p = make_page("https://example.com"); let headers = vec![("Strict-Transport-Security".into(), "max-age=31536000; includeSubDomains".into())]; let ctx = AnalysisContext { page: &p, body: None, status_code: Some(200), headers: &headers, response_time: None, redirect_chain: &[], robots_txt: None, body_size: None, compressed_size: None, server: None, content_type: None, rendered: None }; let f = HstsPreloadValidatorV5::new().analyze(&ctx); assert!(!f.is_empty()); }
    #[test]
    fn test_xcto_v5_missing() { let f = XContentTypeOptionsValidatorV5::new().analyze(&make_ctx(&make_page("https://example.com"), None)); assert!(!f.is_empty()); }
    #[test]
    fn test_xcto_v5_ok() { let p = make_page("https://example.com"); let headers = vec![("X-Content-Type-Options".into(), "nosniff".into())]; let ctx = AnalysisContext { page: &p, body: None, status_code: Some(200), headers: &headers, response_time: None, redirect_chain: &[], robots_txt: None, body_size: None, compressed_size: None, server: None, content_type: None, rendered: None }; assert!(XContentTypeOptionsValidatorV5::new().analyze(&ctx).is_empty()); }
    #[test]
    fn test_rp_v5_missing() { let f = ReferrerPolicyValidatorV5::new().analyze(&make_ctx(&make_page("https://example.com"), None)); assert!(!f.is_empty()); }
    #[test]
    fn test_rp_v5_unsafe_url() { let p = make_page("https://example.com"); let headers = vec![("Referrer-Policy".into(), "unsafe-url".into())]; let ctx = AnalysisContext { page: &p, body: None, status_code: Some(200), headers: &headers, response_time: None, redirect_chain: &[], robots_txt: None, body_size: None, compressed_size: None, server: None, content_type: None, rendered: None }; let f = ReferrerPolicyValidatorV5::new().analyze(&ctx); assert!(f.iter().any(|x| x.code == "RP-V5002")); }
    #[test]
    fn test_xfo_v5_missing() { let f = XFrameOptionsValidatorV5::new().analyze(&make_ctx(&make_page("https://example.com"), None)); assert!(!f.is_empty()); }
    #[test]
    fn test_xfo_v5_invalid() { let p = make_page("https://example.com"); let headers = vec![("X-Frame-Options".into(), "ALLOW".into())]; let ctx = AnalysisContext { page: &p, body: None, status_code: Some(200), headers: &headers, response_time: None, redirect_chain: &[], robots_txt: None, body_size: None, compressed_size: None, server: None, content_type: None, rendered: None }; let f = XFrameOptionsValidatorV5::new().analyze(&ctx); assert!(f.iter().any(|x| x.code == "XFO-V5002")); }
    #[test]
    fn test_pp_camera() { let p = make_page("https://example.com"); let headers = vec![("Permissions-Policy".into(), "camera=(self)".into())]; let ctx = AnalysisContext { page: &p, body: None, status_code: Some(200), headers: &headers, response_time: None, redirect_chain: &[], robots_txt: None, body_size: None, compressed_size: None, server: None, content_type: None, rendered: None }; let f = PermissionsPolicyCameraValidator::new().analyze(&ctx); assert!(!f.is_empty()); }
    #[test]
    fn test_pp_microphone() { let p = make_page("https://example.com"); let headers = vec![("Permissions-Policy".into(), "microphone=(self)".into())]; let ctx = AnalysisContext { page: &p, body: None, status_code: Some(200), headers: &headers, response_time: None, redirect_chain: &[], robots_txt: None, body_size: None, compressed_size: None, server: None, content_type: None, rendered: None }; let f = PermissionsPolicyMicrophoneValidator::new().analyze(&ctx); assert!(!f.is_empty()); }
    #[test]
    fn test_pp_geolocation() { let p = make_page("https://example.com"); let headers = vec![("Permissions-Policy".into(), "geolocation=(self)".into())]; let ctx = AnalysisContext { page: &p, body: None, status_code: Some(200), headers: &headers, response_time: None, redirect_chain: &[], robots_txt: None, body_size: None, compressed_size: None, server: None, content_type: None, rendered: None }; let f = PermissionsPolicyGeolocationValidator::new().analyze(&ctx); assert!(!f.is_empty()); }
    #[test]
    fn test_coep_missing() { let f = CoepValidator::new().analyze(&make_ctx(&make_page("https://example.com"), None)); assert!(!f.is_empty()); }
    #[test]
    fn test_coop_missing() { let f = CoopValidator::new().analyze(&make_ctx(&make_page("https://example.com"), None)); assert!(!f.is_empty()); }
    #[test]
    fn test_cookie_secure_v5() { let p = make_page("https://example.com"); let headers = vec![("Set-Cookie".into(), "session=abc123; HttpOnly".into())]; let ctx = AnalysisContext { page: &p, body: None, status_code: Some(200), headers: &headers, response_time: None, redirect_chain: &[], robots_txt: None, body_size: None, compressed_size: None, server: None, content_type: None, rendered: None }; let f = CookieSecureFlagValidatorV5::new().analyze(&ctx); assert!(!f.is_empty()); }
    #[test]
    fn test_cookie_httponly_v5() { let p = make_page("https://example.com"); let headers = vec![("Set-Cookie".into(), "session=abc123; Secure".into())]; let ctx = AnalysisContext { page: &p, body: None, status_code: Some(200), headers: &headers, response_time: None, redirect_chain: &[], robots_txt: None, body_size: None, compressed_size: None, server: None, content_type: None, rendered: None }; let f = CookieHttpOnlyFlagValidatorV5::new().analyze(&ctx); assert!(!f.is_empty()); }
    #[test]
    fn test_cookie_samesite() { let p = make_page("https://example.com"); let headers = vec![("Set-Cookie".into(), "session=abc123; Secure; HttpOnly".into())]; let ctx = AnalysisContext { page: &p, body: None, status_code: Some(200), headers: &headers, response_time: None, redirect_chain: &[], robots_txt: None, body_size: None, compressed_size: None, server: None, content_type: None, rendered: None }; let f = CookieSameSiteValidator::new().analyze(&ctx); assert!(!f.is_empty()); }
    #[test]
    fn test_mixed_script_v5() { assert!(MixedContentScriptValidatorV5::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_mixed_script_v5_with_content() { let f = MixedContentScriptValidatorV5::new().analyze(&make_ctx(&make_page("https://example.com"), Some("<script src=\"http://evil.com/x.js\"></script>"))); assert!(!f.is_empty()); }
    #[test]
    fn test_mixed_css_v5() { let f = MixedContentStylesheetValidator::new().analyze(&make_ctx(&make_page("https://example.com"), Some("<link href=\"http://evil.com/style.css\" rel=\"stylesheet\">"))); assert!(!f.is_empty()); }
    #[test]
    fn test_sri_v5() { let mut p = make_page("https://example.com"); p.scripts = vec![crate::parser::ScriptInfo { src: Some("https://cdn.com/lib.js".into()), r#async: false, defer: false, script_type: None, has_integrity: false }]; let f = SriValidator::new().analyze(&make_ctx(&p, Some("<script src=\"https://cdn.com/lib.js\"></script>"))); assert!(!f.is_empty()); }

    // ===== SEO V5 Tests =====
    #[test]
    fn test_title_keyword_v5() { let mut p = make_page("https://example.com"); p.meta.title = Some("Amazing Widgets Online".into()); p.meta.description = Some("Buying amazing widgets online today".into()); assert!(TitleKeywordPresenceValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_title_brand_v5() { let mut p = make_page("https://example.com"); p.meta.title = Some("Best Products".into()); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Organization".into()), data: serde_json::json!({"name": "Acme Corp"}) }]; let f = TitleBrandValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_title_length_v5_missing() { let f = TitleLengthValidatorV5::new().analyze(&make_ctx(&make_page("https://example.com"), None)); assert_eq!(f[0].code, "TITLELEN-V5001"); }
    #[test]
    fn test_title_length_v5_short() { let mut p = make_page("https://example.com"); p.meta.title = Some("Hi".into()); let f = TitleLengthValidatorV5::new().analyze(&make_ctx(&p, None)); assert_eq!(f[0].code, "TITLELEN-V5002"); }
    #[test]
    fn test_title_length_v5_ok() { let mut p = make_page("https://example.com"); p.meta.title = Some("This is a perfectly normal length title".into()); assert!(TitleLengthValidatorV5::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_meta_keyword_v5() { let mut p = make_page("https://example.com"); p.meta.title = Some("Amazing Widgets".into()); p.meta.description = Some("Buy the best gadgets and gizmos today".into()); let f = MetaDescriptionKeywordValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_meta_unique_v5() { let mut p = make_page("https://example.com"); p.meta.title = Some("Hello World".into()); p.meta.description = Some("Hello World".into()); let f = MetaDescriptionUniqueValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_meta_unique_v5_different() { let mut p = make_page("https://example.com"); p.meta.title = Some("Hello World".into()); p.meta.description = Some("A completely different description about things".into()); assert!(MetaDescriptionUniqueValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_canonical_self_v5_diff() { let mut p = make_page("https://example.com/page"); p.meta.canonical = Some(url::Url::parse("https://other.com/different-page").unwrap()); let f = CanonicalSelfReferenceValidatorV5::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_canonical_self_v5_same() { let mut p = make_page("https://example.com/page"); p.meta.canonical = Some(url::Url::parse("https://example.com/page").unwrap()); assert!(CanonicalSelfReferenceValidatorV5::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_canonical_chain_v5() { let mut p = make_page("https://example.com"); p.meta.canonical = Some(url::Url::parse("https://other.com").unwrap()); let f = CanonicalChainValidatorV5::new().analyze(&make_ctx(&p, Some("<html><head><link rel=\"canonical\" href=\"https://other.com\"></head></html>"))); assert!(!f.is_empty()); }
    #[test]
    fn test_canonical_depth_v5() { let mut p = make_page("https://example.com"); p.meta.canonical = Some(url::Url::parse("https://example.com/a/b/c/d/e/f").unwrap()); let f = CanonicalDepthValidatorV5::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_hreflang_reciprocal_v5() { let mut p = make_page("https://example.com/page"); p.meta.hreflang = vec![crate::meta::HreflangTag { lang: "fr".into(), url: url::Url::parse("https://example.com/page/fr").unwrap() }]; let f = HreflangReciprocalValidatorV5::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_hreflang_xd_v5() { let mut p = make_page("https://example.com/page"); p.meta.hreflang = vec![crate::meta::HreflangTag { lang: "en".into(), url: url::Url::parse("https://example.com/page/en").unwrap() }]; let f = HreflangXDefaultValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_hreflang_locale_v5() { let mut p = make_page("https://example.com/page"); p.meta.hreflang = vec![crate::meta::HreflangTag { lang: "english".into(), url: url::Url::parse("https://example.com/page/en").unwrap() }]; let f = HreflangLocaleFormatValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_sitemap_coverage_v5() { let robots = "User-agent: *\nDisallow: /admin"; let p = make_page("https://example.com"); let ctx = AnalysisContext { page: &p, body: None, status_code: Some(200), headers: &[], response_time: None, redirect_chain: &[], robots_txt: Some(robots), body_size: None, compressed_size: None, server: None, content_type: None, rendered: None }; let f = SitemapCoverageValidatorV5::new().analyze(&ctx); assert!(!f.is_empty()); }
    #[test]
    fn test_sitemap_lastmod_v5() { let body = "<urlset><url><loc>https://example.com</loc></url></urlset>"; let f = SitemapLastmodValidator::new().analyze(&make_ctx(&make_page("https://example.com"), Some(body))); assert!(!f.is_empty()); }
    #[test]
    fn test_sitemap_priority_v5() { let body = "<urlset><url><loc>https://example.com</loc></url></urlset>"; let f = SitemapPriorityValidator::new().analyze(&make_ctx(&make_page("https://example.com"), Some(body))); assert!(!f.is_empty()); }
    #[test]
    fn test_robots_disallow_v5() { let robots = "User-agent: *\nDisallow: /"; let p = make_page("https://example.com"); let ctx = AnalysisContext { page: &p, body: None, status_code: Some(200), headers: &[], response_time: None, redirect_chain: &[], robots_txt: Some(robots), body_size: None, compressed_size: None, server: None, content_type: None, rendered: None }; let f = RobotsTxtDisallowValidator::new().analyze(&ctx); assert!(!f.is_empty()); }

    // ===== Accessibility V5 Tests =====
    #[test]
    fn test_landmark_main_v5_missing() { let f = LandmarkMainValidatorV5::new().analyze(&make_ctx(&make_page("https://example.com"), None)); assert!(!f.is_empty()); }
    #[test]
    fn test_landmark_main_v5_ok() { let mut p = make_page("https://example.com"); p.landmarks = vec!["main".into()]; assert!(LandmarkMainValidatorV5::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_landmark_main_v5_multi() { let mut p = make_page("https://example.com"); p.landmarks = vec!["main".into(), "main".into()]; let f = LandmarkMainValidatorV5::new().analyze(&make_ctx(&p, None)); assert!(f.iter().any(|x| x.code == "LANDMAIN-V5002")); }
    #[test]
    fn test_landmark_nav_v5_missing() { let f = LandmarkNavValidatorV5::new().analyze(&make_ctx(&make_page("https://example.com"), None)); assert!(!f.is_empty()); }
    #[test]
    fn test_landmark_nav_v5_ok() { let mut p = make_page("https://example.com"); p.landmarks = vec!["navigation".into()]; assert!(LandmarkNavValidatorV5::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_landmark_banner_v5_missing() { let f = LandmarkBannerValidatorV5::new().analyze(&make_ctx(&make_page("https://example.com"), None)); assert!(!f.is_empty()); }
    #[test]
    fn test_heading_skip_v5() { let mut p = make_page("https://example.com"); p.headings = vec![Heading { level: 1, text: "H1".into(), length: 2 }, Heading { level: 3, text: "H3".into(), length: 2 }]; let f = HeadingSkipLevelsValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_heading_skip_v5_ok() { let mut p = make_page("https://example.com"); p.headings = vec![Heading { level: 1, text: "H1".into(), length: 2 }, Heading { level: 2, text: "H2".into(), length: 2 }]; assert!(HeadingSkipLevelsValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_heading_h1_v5_missing() { let mut p = make_page("https://example.com"); p.headings = vec![Heading { level: 2, text: "H2".into(), length: 2 }]; let f = HeadingMultipleH1Validator::new().analyze(&make_ctx(&p, None)); assert_eq!(f[0].code, "HEADH1-V5001"); }
    #[test]
    fn test_heading_h1_v5_multi() { let mut p = make_page("https://example.com"); p.headings = vec![Heading { level: 1, text: "H1a".into(), length: 3 }, Heading { level: 1, text: "H1b".into(), length: 3 }]; let f = HeadingMultipleH1Validator::new().analyze(&make_ctx(&p, None)); assert_eq!(f[0].code, "HEADH1-V5002"); }
    #[test]
    fn test_form_label_v5() { assert!(FormLabelAssociationValidatorV5::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_form_required_v5() { assert!(FormRequiredFieldsValidator::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_table_headers_v5_none() { let mut p = make_page("https://example.com"); p.tables_total = 3; p.tables_with_headers = 0; let f = TableHeadersValidatorV5::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_table_headers_v5_partial() { let mut p = make_page("https://example.com"); p.tables_total = 3; p.tables_with_headers = 1; let f = TableHeadersValidatorV5::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_table_headers_v5_ok() { let mut p = make_page("https://example.com"); p.tables_total = 3; p.tables_with_headers = 3; assert!(TableHeadersValidatorV5::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_table_caption_v5() { let mut p = make_page("https://example.com"); p.tables_total = 1; p.tables_with_captions = 0; let f = TableCaptionValidatorV5::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_table_scope_v5() { let body = "<table><tr><th>Name</th><th>Age</th></tr><tr><td>John</td><td>30</td></tr></table>"; let mut p = make_page("https://example.com"); p.tables_total = 1; p.tables_with_headers = 1; let f = TableScopeValidator::new().analyze(&make_ctx(&p, Some(body))); assert!(!f.is_empty()); }

    // ===== Performance V5 Tests =====
    #[test]
    fn test_preconnect_v5() { assert!(PreconnectHintValidator::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_preconnect_v5_needed() { let mut p = make_page("https://example.com"); p.links = vec![crate::parser::ExtractedLink { href: "https://a.com".into(), text: "".into(), rel: vec![], is_external: true, aria_label: None, img_alt: None }, crate::parser::ExtractedLink { href: "https://b.com".into(), text: "".into(), rel: vec![], is_external: true, aria_label: None, img_alt: None }, crate::parser::ExtractedLink { href: "https://c.com".into(), text: "".into(), rel: vec![], is_external: true, aria_label: None, img_alt: None }, crate::parser::ExtractedLink { href: "https://d.com".into(), text: "".into(), rel: vec![], is_external: true, aria_label: None, img_alt: None }]; let f = PreconnectHintValidator::new().analyze(&make_ctx(&p, Some("<html><head></head><body></body></html>"))); assert!(!f.is_empty()); }
    #[test]
    fn test_dns_prefetch_v5() { assert!(DnsPrefetchHintValidator::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_script_async_v5() { assert!(ScriptAsyncDeferValidator::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_script_async_v5_blocking() { let mut p = make_page("https://example.com"); p.scripts = vec![crate::parser::ScriptInfo { src: Some("https://cdn.com/lib.js".into()), r#async: false, defer: false, script_type: None, has_integrity: false }]; let f = ScriptAsyncDeferValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_script_async_v5_ok() { let mut p = make_page("https://example.com"); p.scripts = vec![crate::parser::ScriptInfo { src: Some("https://cdn.com/lib.js".into()), r#async: true, defer: false, script_type: None, has_integrity: false }]; assert!(ScriptAsyncDeferValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_image_lazy_v5() { assert!(ImageLazyLoadingValidatorV5::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_image_format_v5() { assert!(ImageModernFormatValidator::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_image_format_v5_all_jpg() { let mut p = make_page("https://example.com"); p.images = vec![crate::parser::ExtractedImage { src: "https://example.com/1.jpg".into(), alt: "".into(), has_alt: false, width: None, height: None, is_lazy_loaded: false, aria_hidden: false }, crate::parser::ExtractedImage { src: "https://example.com/2.jpg".into(), alt: "".into(), has_alt: false, width: None, height: None, is_lazy_loaded: false, aria_hidden: false }, crate::parser::ExtractedImage { src: "https://example.com/3.jpg".into(), alt: "".into(), has_alt: false, width: None, height: None, is_lazy_loaded: false, aria_hidden: false }, crate::parser::ExtractedImage { src: "https://example.com/4.jpg".into(), alt: "".into(), has_alt: false, width: None, height: None, is_lazy_loaded: false, aria_hidden: false }]; let f = ImageModernFormatValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_image_dims_v5() { let mut p = make_page("https://example.com"); p.images = vec![crate::parser::ExtractedImage { src: "https://example.com/1.jpg".into(), alt: "".into(), has_alt: false, width: None, height: None, is_lazy_loaded: false, aria_hidden: false }]; let f = ImageDimensionsValidatorV5::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_image_dims_v5_ok() { let mut p = make_page("https://example.com"); p.images = vec![crate::parser::ExtractedImage { src: "https://example.com/1.jpg".into(), alt: "".into(), has_alt: false, width: Some(100), height: Some(100), is_lazy_loaded: false, aria_hidden: false }]; assert!(ImageDimensionsValidatorV5::new().analyze(&make_ctx(&p, None)).is_empty()); }

    // ===== V6 Content Validators Tests =====
    #[test]
    fn test_creative_work_name_missing() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("CreativeWork".into()), data: serde_json::json!({}) }]; let f = CreativeWorkMissingNameValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); assert_eq!(f[0].code, "CWNAME-V6001"); }
    #[test]
    fn test_creative_work_name_ok() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("CreativeWork".into()), data: serde_json::json!({"name": "My Work"}) }]; assert!(CreativeWorkMissingNameValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_creative_work_desc_missing() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("CreativeWork".into()), data: serde_json::json!({}) }]; assert!(!CreativeWorkMissingDescriptionValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_creative_work_date_missing() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("CreativeWork".into()), data: serde_json::json!({}) }]; assert!(!CreativeWorkMissingDateCreatedValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_playlist_num_items() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Playlist".into()), data: serde_json::json!({}) }]; assert!(!PlaylistMissingNumberOfItemsValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_food_est_menu() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("FoodEstablishment".into()), data: serde_json::json!({}) }]; assert!(!FoodEstablishmentMissingMenuValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_food_est_cuisine() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("FoodEstablishment".into()), data: serde_json::json!({}) }]; assert!(!FoodEstablishmentMissingServesCuisineValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_lodging_star_rating() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("LodgingBusiness".into()), data: serde_json::json!({}) }]; assert!(!LodgingBusinessMissingStarRatingValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_lodging_amenity() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("LodgingBusiness".into()), data: serde_json::json!({}) }]; assert!(!LodgingBusinessMissingAmenityFeatureValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_sports_loc_sport() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("SportsActivityLocation".into()), data: serde_json::json!({}) }]; assert!(!SportsActivityLocationMissingSportValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_civic_name() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("CivicStructure".into()), data: serde_json::json!({}) }]; assert!(!CivicStructureMissingNameValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_landform_name() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Landform".into()), data: serde_json::json!({}) }]; assert!(!LandformMissingNameValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_tourist_attr_name() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("TouristAttraction".into()), data: serde_json::json!({}) }]; assert!(!TouristAttractionMissingNameValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_tourist_dest_name() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("TouristDestination".into()), data: serde_json::json!({}) }]; assert!(!TouristDestinationMissingNameValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_sports_event_sport() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("SportsEvent".into()), data: serde_json::json!({}) }]; assert!(!SportsEventMissingSportValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_sports_event_name() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("SportsEvent".into()), data: serde_json::json!({}) }]; assert!(!SportsEventMissingNameValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_edu_org_name() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("EducationalOrganization".into()), data: serde_json::json!({}) }]; assert!(!EducationalOrganizationMissingNameValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_ngo_name() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("NGO".into()), data: serde_json::json!({}) }]; assert!(!NGOMissingNameValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_performing_arts_name() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("PerformingArtsSeries".into()), data: serde_json::json!({}) }]; assert!(!PerformingArtsSeriesMissingNameValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_broadcast_event_name() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("BroadcastEvent".into()), data: serde_json::json!({}) }]; assert!(!BroadcastEventMissingNameValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_product_brand() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Product".into()), data: serde_json::json!({}) }]; assert!(!ProductMissingBrandValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_product_category() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Product".into()), data: serde_json::json!({}) }]; assert!(!ProductMissingCategoryValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_product_review() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Product".into()), data: serde_json::json!({}) }]; assert!(!ProductMissingReviewValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_book_author() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Book".into()), data: serde_json::json!({}) }]; assert!(!BookMissingAuthorValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_book_isbn() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Book".into()), data: serde_json::json!({}) }]; assert!(!BookMissingIsbnValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_book_date_published() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Book".into()), data: serde_json::json!({}) }]; assert!(!BookMissingDatePublishedValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_movie_director() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Movie".into()), data: serde_json::json!({}) }]; assert!(!MovieMissingDirectorValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_movie_duration() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Movie".into()), data: serde_json::json!({}) }]; assert!(!MovieMissingDurationValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_movie_date_created() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Movie".into()), data: serde_json::json!({}) }]; assert!(!MovieMissingDateCreatedValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_tv_series_seasons() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("TVSeries".into()), data: serde_json::json!({}) }]; assert!(!TVSeriesMissingNumberOfSeasonsValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_tv_series_episode() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("TVSeries".into()), data: serde_json::json!({}) }]; assert!(!TVSeriesMissingEpisodeValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_music_recording_artist() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("MusicRecording".into()), data: serde_json::json!({}) }]; assert!(!MusicRecordingMissingByArtistValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_music_recording_album() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("MusicRecording".into()), data: serde_json::json!({}) }]; assert!(!MusicRecordingMissingAlbumValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_service_area_served() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Service".into()), data: serde_json::json!({}) }]; assert!(!ServiceMissingAreaServedValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_service_provider() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Service".into()), data: serde_json::json!({}) }]; assert!(!ServiceMissingProviderValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_health_plan_provider() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("HealthPlan".into()), data: serde_json::json!({}) }]; assert!(!HealthPlanMissingProviderValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_health_plan_coverage() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("HealthPlan".into()), data: serde_json::json!({}) }]; assert!(!HealthPlanMissingCoverageAreaValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_invoice_account() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Invoice".into()), data: serde_json::json!({}) }]; assert!(!InvoiceMissingAccountValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_invoice_payment_date() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Invoice".into()), data: serde_json::json!({}) }]; assert!(!InvoiceMissingPaymentDueDateValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_permit_number() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Permit".into()), data: serde_json::json!({}) }]; assert!(!PermitMissingPermitNumberValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_permit_issued_by() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Permit".into()), data: serde_json::json!({}) }]; assert!(!PermitMissingIssuedByValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_plan_description() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Plan".into()), data: serde_json::json!({}) }]; assert!(!PlanMissingDescriptionValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_plan_about() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Plan".into()), data: serde_json::json!({}) }]; assert!(!PlanMissingAboutValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_research_about() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("ResearchProject".into()), data: serde_json::json!({}) }]; assert!(!ResearchProjectMissingAboutValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_research_funder() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("ResearchProject".into()), data: serde_json::json!({}) }]; assert!(!ResearchProjectMissingFunderValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_schedule_tz() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Schedule".into()), data: serde_json::json!({}) }]; assert!(!ScheduleMissingTimezoneValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_trip_itinerary() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Trip".into()), data: serde_json::json!({}) }]; assert!(!TripMissingItineraryValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_workers_union_name() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("WorkersUnion".into()), data: serde_json::json!({}) }]; assert!(!WorkersUnionMissingNameValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_webapi_doc() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("WebAPI".into()), data: serde_json::json!({}) }]; assert!(!WebAPIMissingDocumentationValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_wearable_device() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Wearable".into()), data: serde_json::json!({}) }]; assert!(!WearableMissingDeviceTypeValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_webpage_element_name() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("WebPageElement".into()), data: serde_json::json!({}) }]; assert!(!WebPageElementMissingNameValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_worker_job_title() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Worker".into()), data: serde_json::json!({}) }]; assert!(!WorkerMissingJobTitleValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_creative_work_license() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("CreativeWork".into()), data: serde_json::json!({}) }]; assert!(!CreativeWorkMissingLicenseValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_product_sku() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Product".into()), data: serde_json::json!({}) }]; assert!(!ProductMissingSkuValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_book_publisher() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Book".into()), data: serde_json::json!({}) }]; assert!(!BookMissingPublisherValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_movie_actor() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Movie".into()), data: serde_json::json!({}) }]; assert!(!MovieMissingActorValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_service_type() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Service".into()), data: serde_json::json!({}) }]; assert!(!ServiceMissingServiceTypeValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_event_start_date() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Event".into()), data: serde_json::json!({}) }]; assert!(!EventMissingStartDateValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_event_location() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Event".into()), data: serde_json::json!({}) }]; assert!(!EventMissingLocationValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_local_business_geo() { let mut p = make_page("https://example.com"); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("LocalBusiness".into()), data: serde_json::json!({}) }]; assert!(!LocalBusinessMissingGeoValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }

    // ===== V6 Security Validators Tests =====
    #[test]
    fn test_csp_connect_src() { assert!(CspConnectSrcValidator::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_csp_connect_src_present() { let p = make_page("https://example.com"); let headers = vec![("Content-Security-Policy".into(), "connect-src 'self'".into())]; let ctx = AnalysisContext { page: &p, body: None, status_code: Some(200), headers: &headers, response_time: None, redirect_chain: &[], robots_txt: None, body_size: None, compressed_size: None, server: None, content_type: None, rendered: None }; assert!(CspConnectSrcValidator::new().analyze(&ctx).is_empty()); }
    #[test]
    fn test_csp_font_src() { assert!(CspFontSrcValidator::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_hsts_threshold_low() { let p = make_page("https://example.com"); let headers = vec![("Strict-Transport-Security".into(), "max-age=100".into())]; let ctx = AnalysisContext { page: &p, body: None, status_code: Some(200), headers: &headers, response_time: None, redirect_chain: &[], robots_txt: None, body_size: None, compressed_size: None, server: None, content_type: None, rendered: None }; let f = HstsMaxAgeThresholdValidator::new().analyze(&ctx); assert!(!f.is_empty()); }
    #[test]
    fn test_hsts_threshold_ok() { let p = make_page("https://example.com"); let headers = vec![("Strict-Transport-Security".into(), "max-age=31536000".into())]; let ctx = AnalysisContext { page: &p, body: None, status_code: Some(200), headers: &headers, response_time: None, redirect_chain: &[], robots_txt: None, body_size: None, compressed_size: None, server: None, content_type: None, rendered: None }; assert!(HstsMaxAgeThresholdValidator::new().analyze(&ctx).is_empty()); }
    #[test]
    fn test_hsts_preload_check() { let p = make_page("https://example.com"); let headers = vec![("Strict-Transport-Security".into(), "max-age=31536000; includeSubDomains; preload".into())]; let ctx = AnalysisContext { page: &p, body: None, status_code: Some(200), headers: &headers, response_time: None, redirect_chain: &[], robots_txt: None, body_size: None, compressed_size: None, server: None, content_type: None, rendered: None }; assert!(HstsPreloadListCheckValidator::new().analyze(&ctx).is_empty()); }
    #[test]
    fn test_cookie_secure_deep() { let p = make_page("https://example.com"); let headers = vec![("Set-Cookie".into(), "session=abc".into())]; let ctx = AnalysisContext { page: &p, body: None, status_code: Some(200), headers: &headers, response_time: None, redirect_chain: &[], robots_txt: None, body_size: None, compressed_size: None, server: None, content_type: None, rendered: None }; let f = CookieSecureFlagDeepValidator::new().analyze(&ctx); assert!(!f.is_empty()); }
    #[test]
    fn test_cookie_httponly_deep() { let p = make_page("https://example.com"); let headers = vec![("Set-Cookie".into(), "session=abc; Secure".into())]; let ctx = AnalysisContext { page: &p, body: None, status_code: Some(200), headers: &headers, response_time: None, redirect_chain: &[], robots_txt: None, body_size: None, compressed_size: None, server: None, content_type: None, rendered: None }; let f = CookieHttpOnlyFlagDeepValidator::new().analyze(&ctx); assert!(!f.is_empty()); }
    #[test]
    fn test_cookie_samesite_deep() { let p = make_page("https://example.com"); let headers = vec![("Set-Cookie".into(), "session=abc; Secure; HttpOnly".into())]; let ctx = AnalysisContext { page: &p, body: None, status_code: Some(200), headers: &headers, response_time: None, redirect_chain: &[], robots_txt: None, body_size: None, compressed_size: None, server: None, content_type: None, rendered: None }; let f = CookieSameSiteDeepValidator::new().analyze(&ctx); assert!(!f.is_empty()); }
    #[test]
    fn test_mixed_content_iframe() { assert!(MixedContentIframeValidator::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_cors_wildcard() { let p = make_page("https://example.com"); let headers = vec![("Access-Control-Allow-Origin".into(), "*".into())]; let ctx = AnalysisContext { page: &p, body: None, status_code: Some(200), headers: &headers, response_time: None, redirect_chain: &[], robots_txt: None, body_size: None, compressed_size: None, server: None, content_type: None, rendered: None }; let f = CorsWildcardValidator::new().analyze(&ctx); assert!(!f.is_empty()); }
    #[test]
    fn test_cors_missing() { let f = CorsMissingHeaderValidator::new().analyze(&make_ctx(&make_page("https://example.com"), None)); assert!(!f.is_empty()); }
    #[test]
    fn test_rp_strict() { let p = make_page("https://example.com"); let headers = vec![("Referrer-Policy".into(), "unsafe-url".into())]; let ctx = AnalysisContext { page: &p, body: None, status_code: Some(200), headers: &headers, response_time: None, redirect_chain: &[], robots_txt: None, body_size: None, compressed_size: None, server: None, content_type: None, rendered: None }; let f = ReferrerPolicyStrictValidator::new().analyze(&ctx); assert!(!f.is_empty()); }
    #[test]
    fn test_xfo_missing() { let f = XFrameOptionsMissingValidator::new().analyze(&make_ctx(&make_page("https://example.com"), None)); assert!(!f.is_empty()); }
    #[test]
    fn test_pp_payment() { assert!(PermissionsPolicyPaymentValidator::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_pp_fullscreen() { assert!(PermissionsPolicyFullscreenValidator::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_pp_xr_vr() { assert!(PermissionsPolicyXrVrValidator::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_coep_require_corp() { assert!(CoepRequireCorpValidator::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_coop_same_origin() { assert!(CoopSameOriginValidator::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_csp_obj_src() { assert!(CspObjectSrcNoneValidator::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_csp_base_uri() { assert!(CspBaseUriSelfValidator::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }
    #[test]
    fn test_csp_form_action() { assert!(CspFormActionSelfValidator::new().analyze(&make_ctx(&make_page("https://example.com"), None)).is_empty()); }

    // ===== V6 SEO Validators Tests =====
    #[test]
    fn test_title_missing() { let f = TitleMissingValidator::new().analyze(&make_ctx(&make_page("https://example.com"), None)); assert!(!f.is_empty()); assert_eq!(f[0].code, "TITLEMISS-V6081"); }
    #[test]
    fn test_title_present() { let mut p = make_page("https://example.com"); p.meta.title = Some("Good Title".into()); assert!(TitleMissingValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_title_keyword_density() { let mut p = make_page("https://example.com"); p.meta.title = Some("Amazing Widgets Online".into()); p.meta.description = Some("Amazing widgets available online today".into()); assert!(TitleKeywordDensityValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_title_brand() { let mut p = make_page("https://example.com"); p.meta.title = Some("Best Products by Acme Corp".into()); p.structured_data = vec![StructuredData { context: Some("https://schema.org".into()), r#type: Some("Organization".into()), data: serde_json::json!({"name": "Acme Corp"}) }]; assert!(TitleBrandPlacementValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_title_pixel_width() { let mut p = make_page("https://example.com"); p.meta.title = Some("Short Title".into()); assert!(TitlePixelWidthValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_title_pixel_width_long() { let mut p = make_page("https://example.com"); p.meta.title = Some("A".repeat(100).into()); let f = TitlePixelWidthValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_meta_desc_missing() { let f = MetaDescriptionMissingValidator::new().analyze(&make_ctx(&make_page("https://example.com"), None)); assert!(!f.is_empty()); }
    #[test]
    fn test_meta_desc_short() { let mut p = make_page("https://example.com"); p.meta.description = Some("Short".into()); let f = MetaDescriptionTooShortValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_meta_desc_unique() { let mut p = make_page("https://example.com"); p.meta.title = Some("Hello World".into()); p.meta.description = Some("Hello World".into()); let f = MetaDescriptionUniquenessValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_canonical_missing() { let f = CanonicalMissingValidator::new().analyze(&make_ctx(&make_page("https://example.com"), None)); assert!(!f.is_empty()); }
    #[test]
    fn test_canonical_self_ref() { let mut p = make_page("https://example.com/page"); p.meta.canonical = Some(url::Url::parse("https://other.com/different").unwrap()); let f = CanonicalSelfReferenceDeepValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_canonical_self_ok() { let mut p = make_page("https://example.com/page"); p.meta.canonical = Some(url::Url::parse("https://example.com/page").unwrap()); assert!(CanonicalSelfReferenceDeepValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_canonical_chain() { let f = CanonicalChainDeepValidator::new().analyze(&make_ctx(&make_page("https://example.com"), Some("<html><head><link rel=\"canonical\" href=\"https://example.com\"><link rel=\"canonical\" href=\"https://example.com/other\"></head></html>"))); assert!(!f.is_empty()); }
    #[test]
    fn test_canonical_depth() { let mut p = make_page("https://example.com"); p.meta.canonical = Some(url::Url::parse("https://example.com/a/b/c/d/e/f").unwrap()); let f = CanonicalDepthDeepValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_hreflang_missing() { let f = HreflangMissingValidator::new().analyze(&make_ctx(&make_page("https://example.com"), None)); assert!(!f.is_empty()); assert_eq!(f[0].code, "HREFMISS-V6092"); }
    #[test]
    fn test_hreflang_not_missing() { let mut p = make_page("https://example.com"); p.meta.hreflang = vec![crate::meta::HreflangTag { lang: "en".into(), url: url::Url::parse("https://example.com/en").unwrap() }]; assert!(HreflangMissingValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_hreflang_reciprocal_dup() { let mut p = make_page("https://example.com"); p.meta.hreflang = vec![crate::meta::HreflangTag { lang: "fr".into(), url: url::Url::parse("https://example.com/fr").unwrap() }, crate::meta::HreflangTag { lang: "fr".into(), url: url::Url::parse("https://example.com/fr2").unwrap() }]; let f = HreflangReciprocalDeepValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_hreflang_xdefault() { let mut p = make_page("https://example.com"); p.meta.hreflang = vec![crate::meta::HreflangTag { lang: "en".into(), url: url::Url::parse("https://example.com/en").unwrap() }]; let f = HreflangXDefaultMissingValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_sitemap_missing() { let p = make_page("https://example.com"); let ctx = AnalysisContext { page: &p, body: None, status_code: Some(200), headers: &[], response_time: None, redirect_chain: &[], robots_txt: Some("User-agent: *\nDisallow: /admin"), body_size: None, compressed_size: None, server: None, content_type: None, rendered: None }; let f = SitemapMissingValidator::new().analyze(&ctx); assert!(!f.is_empty()); }
    #[test]
    fn test_robots_txt_empty() { let p = make_page("https://example.com"); let ctx = AnalysisContext { page: &p, body: None, status_code: Some(200), headers: &[], response_time: None, redirect_chain: &[], robots_txt: Some(""), body_size: None, compressed_size: None, server: None, content_type: None, rendered: None }; let f = RobotsTxtEmptyValidator::new().analyze(&ctx); assert!(!f.is_empty()); }
    #[test]
    fn test_robots_txt_deep_disallow() { let p = make_page("https://example.com"); let ctx = AnalysisContext { page: &p, body: None, status_code: Some(200), headers: &[], response_time: None, redirect_chain: &[], robots_txt: Some("User-agent: *\nDisallow: /a/b/c/d/e/f"), body_size: None, compressed_size: None, server: None, content_type: None, rendered: None }; let f = RobotsTxtDisallowDepthValidator::new().analyze(&ctx); assert!(!f.is_empty()); }
    #[test]
    fn test_robots_txt_block_all() { let p = make_page("https://example.com"); let ctx = AnalysisContext { page: &p, body: None, status_code: Some(200), headers: &[], response_time: None, redirect_chain: &[], robots_txt: Some("User-agent: *\nDisallow: /"), body_size: None, compressed_size: None, server: None, content_type: None, rendered: None }; let f = RobotsTxtWildcardDisallowValidator::new().analyze(&ctx); assert!(!f.is_empty()); }
    #[test]
    fn test_robots_txt_no_ua() { let p = make_page("https://example.com"); let ctx = AnalysisContext { page: &p, body: None, status_code: Some(200), headers: &[], response_time: None, redirect_chain: &[], robots_txt: Some("Disallow: /admin"), body_size: None, compressed_size: None, server: None, content_type: None, rendered: None }; let f = RobotsTxtMissingUserAgentValidator::new().analyze(&ctx); assert!(!f.is_empty()); }
    #[test]
    fn test_int_links_diversity() { let p = make_page("https://example.com"); assert!(InternalLinksDiversityValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_ext_links_auth() { let p = make_page("https://example.com"); assert!(ExternalLinksAuthorityScoreValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_ext_links_nofollow() { let p = make_page("https://example.com"); assert!(ExternalLinksNofollowAnalysisValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }

    // ===== V6 Accessibility Validators Tests =====
    #[test]
    fn test_h1_count() { let mut p = make_page("https://example.com"); p.headings = vec![Heading { level: 1, text: "Title".into(), length: 5 }]; assert!(HeadingH1CountValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_h1_count_missing() { let mut p = make_page("https://example.com"); p.headings = vec![Heading { level: 2, text: "Subtitle".into(), length: 8 }]; let f = HeadingH1CountValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_heading_depth() { let p = make_page("https://example.com"); assert!(HeadingDepthAnalysisValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_form_required() { let p = make_page("https://example.com"); assert!(FormRequiredFieldsDeepValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_table_scope() { let mut p = make_page("https://example.com"); p.tables_total = 2; p.tables_with_headers = 0; let f = TableHeadersScopeValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_table_caption_deep() { let mut p = make_page("https://example.com"); p.tables_total = 2; p.tables_with_captions = 0; let f = TableCaptionMissingDeepValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_link_text_generic_deep() { let mut p = make_page("https://example.com"); p.links = vec![crate::parser::ExtractedLink { href: "https://example.com/1".into(), text: "click here".into(), rel: vec![], is_external: false, aria_label: None, img_alt: None }]; let f = LinkTextGenericDeepValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_link_text_empty_deep() { let mut p = make_page("https://example.com"); p.links = vec![crate::parser::ExtractedLink { href: "https://example.com/1".into(), text: "".into(), rel: vec![], is_external: false, aria_label: None, img_alt: None }]; let f = LinkTextEmptyDeepValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_link_text_dup() { let mut p = make_page("https://example.com"); for i in 0..10 { p.links.push(crate::parser::ExtractedLink { href: format!("https://example.com/{i}"), text: "same".into(), rel: vec![], is_external: false, aria_label: None, img_alt: None }); } let f = LinkTextDuplicateValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_img_alt_missing_deep() { let mut p = make_page("https://example.com"); p.images = vec![crate::parser::ExtractedImage { src: "https://example.com/1.jpg".into(), alt: "".into(), has_alt: false, width: None, height: None, is_lazy_loaded: false, aria_hidden: false }]; let f = ImageAltMissingDeepValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_img_alt_empty_deep() { let mut p = make_page("https://example.com"); p.images = vec![crate::parser::ExtractedImage { src: "https://example.com/1.jpg".into(), alt: "".into(), has_alt: true, width: None, height: None, is_lazy_loaded: false, aria_hidden: false }]; let f = ImageAltEmptyDeepValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_img_alt_decorative() { let mut p = make_page("https://example.com"); for _ in 0..5 { p.images.push(crate::parser::ExtractedImage { src: "https://example.com/1.jpg".into(), alt: "image".into(), has_alt: true, width: None, height: None, is_lazy_loaded: false, aria_hidden: false }); } let f = ImageAltDecorativePatternValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_focus_tabindex() { let mut p = make_page("https://example.com"); p.has_positive_tabindex = true; let f = FocusTabindexPositiveValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_focus_trap() { let p = make_page("https://example.com"); assert!(FocusTrapMissingValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_heading_skip_deep() { let mut p = make_page("https://example.com"); p.headings = vec![Heading { level: 1, text: "H1".into(), length: 2 }, Heading { level: 3, text: "H3".into(), length: 2 }]; let f = HeadingSkipLevelsDeepValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_heading_empty() { let mut p = make_page("https://example.com"); p.headings = vec![Heading { level: 1, text: "".into(), length: 0 }]; let f = HeadingEmptyValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
    #[test]
    fn test_form_fieldset() { let p = make_page("https://example.com"); assert!(FormFieldsetLegendValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_landmark_contentinfo() { let f = LandmarkContentinfoValidator::new().analyze(&make_ctx(&make_page("https://example.com"), None)); assert!(!f.is_empty()); }
    #[test]
    fn test_landmark_contentinfo_ok() { let mut p = make_page("https://example.com"); p.landmarks = vec!["contentinfo".into()]; assert!(LandmarkContentinfoValidator::new().analyze(&make_ctx(&p, None)).is_empty()); }
    #[test]
    fn test_landmark_complementary() { let p = make_page("https://example.com"); let f = LandmarkComplementaryValidator::new().analyze(&make_ctx(&p, None)); assert!(!f.is_empty()); }
}
