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

pub struct CspDirectiveAnalyzerV2;
impl Default for CspDirectiveAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl CspDirectiveAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CspDirectiveAnalyzerV2 {
    fn name(&self) -> &str {
        "csp-directive-analyzer-v2"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let csp = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Content-Security-Policy"))
            .map(|(_, v)| v.as_str())
            .unwrap_or("");
        if csp.is_empty() {
            return findings;
        }
        let dirs: Vec<&str> = csp
            .split(';')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        let names: Vec<&str> = dirs
            .iter()
            .filter_map(|d| d.split_whitespace().next())
            .collect();
        if !names.contains(&"base-uri") {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Security,
                code: "CSPDIR-V2001".to_string(),
                title: "CSP missing base-uri".to_string(),
                description: "Without base-uri, attackers can inject <base> tags.".to_string(),
                url: url.clone(),
                recommendation: "Add base-uri 'self'.".to_string(),
            });
        }
        if !names.contains(&"form-action") {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Security,
                code: "CSPDIR-V2002".to_string(),
                title: "CSP missing form-action".to_string(),
                description: "Without form-action, forms could submit to attacker URLs."
                    .to_string(),
                url: url.clone(),
                recommendation: "Add form-action 'self'.".to_string(),
            });
        }
        if !names.contains(&"frame-ancestors") {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Security,
                code: "CSPDIR-V2003".to_string(),
                title: "CSP missing frame-ancestors".to_string(),
                description: "frame-ancestors is the modern replacement for X-Frame-Options."
                    .to_string(),
                url: url.clone(),
                recommendation: "Add frame-ancestors 'none' or 'self'.".to_string(),
            });
        }
        if !names.contains(&"object-src") {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Security,
                code: "CSPDIR-V2004".to_string(),
                title: "CSP missing object-src".to_string(),
                description: "Without object-src, plugins like Flash could load unchecked."
                    .to_string(),
                url: url.clone(),
                recommendation: "Add object-src 'none'.".to_string(),
            });
        }
        findings
    }
}

pub struct CorsPolicyAnalyzerV2;
impl Default for CorsPolicyAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl CorsPolicyAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CorsPolicyAnalyzerV2 {
    fn name(&self) -> &str {
        "cors-policy-analyzer-v2"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let acao = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Access-Control-Allow-Origin"))
            .map(|(_, v)| v.as_str());
        let acac = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Access-Control-Allow-Credentials"))
            .map(|(_, v)| v.as_str());
        if let Some(origin) = acao {
            if origin == "*"
                && acac
                    .map(|v| v.eq_ignore_ascii_case("true"))
                    .unwrap_or(false)
            {
                findings.push(Finding {
                    severity: Severity::Critical,
                    category: IssueCategory::Security,
                    code: "CORS-V2001".to_string(),
                    title: "CORS wildcard with credentials".to_string(),
                    description: "Wildcard origin with credentials enabled.".to_string(),
                    url: url.clone(),
                    recommendation: "Use specific origin instead of '*'.".to_string(),
                });
            }
            if origin != "*" && !url.starts_with(origin) {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Security,
                    code: "CORS-V2003".to_string(),
                    title: "CORS origin differs from page".to_string(),
                    description: format!("CORS allows origin '{origin}'."),
                    url: url.clone(),
                    recommendation: "Verify this cross-origin allowance is intentional."
                        .to_string(),
                });
            }
        }
        findings
    }
}

pub struct CookieSecurityFlagAnalyzerV2;
impl Default for CookieSecurityFlagAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl CookieSecurityFlagAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CookieSecurityFlagAnalyzerV2 {
    fn name(&self) -> &str {
        "cookie-security-flag-analyzer-v2"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for (k, v) in ctx.headers {
            if !k.eq_ignore_ascii_case("set-cookie") {
                continue;
            }
            let lower = v.to_lowercase();
            let name = v.split('=').next().unwrap_or("cookie").trim().to_string();
            if lower.contains("secure") && lower.contains("httponly") && lower.contains("samesite")
            {
                continue;
            }
            if !lower.contains("secure") {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "COOKIE-V2002".to_string(),
                    title: format!("Cookie '{name}' missing Secure"),
                    description: "Cookie transmitted over HTTP.".to_string(),
                    url: url.clone(),
                    recommendation: "Add Secure flag.".to_string(),
                });
            }
            if !lower.contains("httponly") {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "COOKIE-V2003".to_string(),
                    title: format!("Cookie '{name}' missing HttpOnly"),
                    description: "Cookie accessible to JavaScript.".to_string(),
                    url: url.clone(),
                    recommendation: "Add HttpOnly flag.".to_string(),
                });
            }
            if !lower.contains("samesite") {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "COOKIE-V2004".to_string(),
                    title: format!("Cookie '{name}' missing SameSite"),
                    description: "Without SameSite, cookie is vulnerable to CSRF.".to_string(),
                    url: url.clone(),
                    recommendation: "Add SameSite=Strict or Lax.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct MixedContentDetectionAnalyzerV2;
impl Default for MixedContentDetectionAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl MixedContentDetectionAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for MixedContentDetectionAnalyzerV2 {
    fn name(&self) -> &str {
        "mixed-content-detection-v2"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if !url.starts_with("https://") {
            return findings;
        }
        if let Some(body) = ctx.body {
            let lower = body.to_lowercase();
            let http_count = lower.matches("http://").count();
            if http_count > 5 {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "MIXCONT-V2001".to_string(),
                    title: format!("{http_count} HTTP references on HTTPS page"),
                    description: "Mixed content degrades HTTPS security.".to_string(),
                    url: url.clone(),
                    recommendation: "Change all URLs to HTTPS.".to_string(),
                });
            }
            if !lower.contains("upgrade-insecure-requests") && http_count > 0 {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Security,
                    code: "MIXCONT-V2005".to_string(),
                    title: "No upgrade-insecure-requests CSP".to_string(),
                    description: "CSP doesn't auto-upgrade mixed content.".to_string(),
                    url: url.clone(),
                    recommendation: "Add upgrade-insecure-requests to CSP.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct HstsPreloadReadinessAnalyzerV2;
impl Default for HstsPreloadReadinessAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl HstsPreloadReadinessAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for HstsPreloadReadinessAnalyzerV2 {
    fn name(&self) -> &str {
        "hsts-preload-readiness-v2"
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
                code: "HSTSPR-V2001".to_string(),
                title: "HSTS missing includeSubDomains".to_string(),
                description: "Required for preload list submission.".to_string(),
                url: url.clone(),
                recommendation: "Add includeSubDomains.".to_string(),
            });
        }
        if !lower.contains("preload") {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Security,
                code: "HSTSPR-V2002".to_string(),
                title: "HSTS missing preload".to_string(),
                description: "Without preload, domain won't be in browser preload lists."
                    .to_string(),
                url: url.clone(),
                recommendation: "Add preload directive.".to_string(),
            });
        }
        if let Some(pos) = lower.find("max-age=") {
            let after = &lower[pos + 8..];
            if let Ok(age) = after
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>()
                .parse::<u64>()
            {
                if age < 31536000 {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Security,
                        code: "HSTSPR-V2003".to_string(),
                        title: "HSTS max-age below preload minimum".to_string(),
                        description: format!("max-age is {age}, preload requires 31536000."),
                        url: url.clone(),
                        recommendation: "Set max-age to at least 31536000.".to_string(),
                    });
                }
            }
        }
        findings
    }
}

pub struct XContentTypeOptionsDeepAnalyzerV2;
impl Default for XContentTypeOptionsDeepAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl XContentTypeOptionsDeepAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for XContentTypeOptionsDeepAnalyzerV2 {
    fn name(&self) -> &str {
        "x-content-type-options-deep-v2"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let xcto = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("X-Content-Type-Options"))
            .map(|(_, v)| v.as_str());
        match xcto {
            None => {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "XCTO-V2001-DEEP".to_string(),
                    title: "Missing X-Content-Type-Options".to_string(),
                    description: "Without nosniff, browsers may MIME-sniff responses.".to_string(),
                    url: url.clone(),
                    recommendation: "Add X-Content-Type-Options: nosniff.".to_string(),
                });
            }
            Some(val) if !val.trim().eq_ignore_ascii_case("nosniff") => {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "XCTO-V2002".to_string(),
                    title: "Invalid X-Content-Type-Options".to_string(),
                    description: format!("Value is \"{val}\", should be \"nosniff\"."),
                    url: url.clone(),
                    recommendation: "Set to nosniff.".to_string(),
                });
            }
            _ => {}
        }
        findings
    }
}

pub struct ReferrerPolicyDeepAnalyzerV2;
impl Default for ReferrerPolicyDeepAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl ReferrerPolicyDeepAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for ReferrerPolicyDeepAnalyzerV2 {
    fn name(&self) -> &str {
        "referrer-policy-deep-v2"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let rp = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Referrer-Policy"))
            .map(|(_, v)| v.as_str());
        match rp {
            None => {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Security,
                    code: "RPDEEP-V2001".to_string(),
                    title: "Missing Referrer-Policy".to_string(),
                    description: "Browser default may leak referrer info.".to_string(),
                    url: url.clone(),
                    recommendation: "Add Referrer-Policy: strict-origin-when-cross-origin."
                        .to_string(),
                });
            }
            Some(val) if val.eq_ignore_ascii_case("unsafe-url") => {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "RPDEEP-V2002".to_string(),
                    title: "Referrer-Policy unsafe-url".to_string(),
                    description: "Leaks full URL including path and query.".to_string(),
                    url: url.clone(),
                    recommendation: "Use strict-origin-when-cross-origin.".to_string(),
                });
            }
            _ => {}
        }
        findings
    }
}

pub struct XFrameOptionsDeepAnalyzerV2;
impl Default for XFrameOptionsDeepAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl XFrameOptionsDeepAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for XFrameOptionsDeepAnalyzerV2 {
    fn name(&self) -> &str {
        "x-frame-options-deep-v2"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let xfo = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("X-Frame-Options"))
            .map(|(_, v)| v.as_str());
        let csp_frame = ctx.headers.iter().any(|(k, v)| {
            k.eq_ignore_ascii_case("Content-Security-Policy") && v.contains("frame-ancestors")
        });
        if xfo.is_none() && !csp_frame {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Security,
                code: "XFODEEP-V2001".to_string(),
                title: "No clickjacking protection".to_string(),
                description: "Neither X-Frame-Options nor CSP frame-ancestors set.".to_string(),
                url: url.clone(),
                recommendation: "Add X-Frame-Options: DENY or CSP frame-ancestors.".to_string(),
            });
        }
        findings
    }
}

pub struct PermissionsPolicyDeepAnalyzerV2;
impl Default for PermissionsPolicyDeepAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl PermissionsPolicyDeepAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for PermissionsPolicyDeepAnalyzerV2 {
    fn name(&self) -> &str {
        "permissions-policy-deep-v2"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let pp = ctx
            .headers
            .iter()
            .find(|(k, _)| {
                k.eq_ignore_ascii_case("Permissions-Policy")
                    || k.eq_ignore_ascii_case("Feature-Policy")
            })
            .map(|(_, v)| v.as_str());
        if pp.is_none() {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Security,
                code: "PERMP-V2001".to_string(),
                title: "Missing Permissions-Policy".to_string(),
                description: "Browsers may allow access to sensitive APIs.".to_string(),
                url: url.clone(),
                recommendation: "Add Permissions-Policy header.".to_string(),
            });
        }
        findings
    }
}

pub struct CrossOriginIsolationDeepAnalyzerV2;
impl Default for CrossOriginIsolationDeepAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl CrossOriginIsolationDeepAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CrossOriginIsolationDeepAnalyzerV2 {
    fn name(&self) -> &str {
        "cross-origin-isolation-deep-v2"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let coep = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Cross-Origin-Embedder-Policy"))
            .map(|(_, v)| v.as_str());
        let coop = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Cross-Origin-Opener-Policy"))
            .map(|(_, v)| v.as_str());
        if coep.is_none() {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Security,
                code: "COISO-V2001".to_string(),
                title: "Missing COEP".to_string(),
                description: "COEP prevents loading cross-origin resources without CORS."
                    .to_string(),
                url: url.clone(),
                recommendation: "Add Cross-Origin-Embedder-Policy: require-corp.".to_string(),
            });
        }
        if coop.is_none() {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Security,
                code: "COISO-V2003".to_string(),
                title: "Missing COOP".to_string(),
                description: "COOP controls cross-origin document references.".to_string(),
                url: url.clone(),
                recommendation: "Add Cross-Origin-Opener-Policy: same-origin.".to_string(),
            });
        }
        findings
    }
}

pub struct CspScriptSrcValidator;
impl Default for CspScriptSrcValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CspScriptSrcValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CspScriptSrcValidator {
    fn name(&self) -> &str {
        "csp-script-src-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let csp = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Content-Security-Policy"))
            .map(|(_, v)| v.as_str())
            .unwrap_or("");
        if csp.is_empty() {
            return findings;
        }
        if let Some(directive) = csp.split(';').find(|d| d.trim().starts_with("script-src")) {
            let value = directive.trim().trim_start_matches("script-src").trim();
            if value.contains("'unsafe-inline'") {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "CSPSSRC-V5001".to_string(),
                    title: "CSP script-src allows unsafe-inline".to_string(),
                    description: "unsafe-inline weakens CSP.".to_string(),
                    url: url.clone(),
                    recommendation: "Remove unsafe-inline and use nonces or hashes.".to_string(),
                });
            }
            if value.contains("'unsafe-eval'") {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "CSPSSRC-V5002".to_string(),
                    title: "CSP script-src allows unsafe-eval".to_string(),
                    description: "unsafe-eval allows eval().".to_string(),
                    url: url.clone(),
                    recommendation: "Remove unsafe-eval.".to_string(),
                });
            }
            if value.contains("*") && !value.contains("'none'") {
                findings.push(Finding {
                    severity: Severity::Critical,
                    category: IssueCategory::Security,
                    code: "CSPSSRC-V5003".to_string(),
                    title: "CSP script-src wildcard".to_string(),
                    description: "Wildcard allows any script source.".to_string(),
                    url: url.clone(),
                    recommendation: "Restrict script-src to specific origins.".to_string(),
                });
            }
        } else {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Security,
                code: "CSPSSRC-V5004".to_string(),
                title: "CSP missing script-src".to_string(),
                description: "No script-src directive.".to_string(),
                url: url.clone(),
                recommendation: "Add script-src directive.".to_string(),
            });
        }
        findings
    }
}

pub struct CspStyleSrcValidator;
impl Default for CspStyleSrcValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CspStyleSrcValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CspStyleSrcValidator {
    fn name(&self) -> &str {
        "csp-style-src-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let csp = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Content-Security-Policy"))
            .map(|(_, v)| v.as_str())
            .unwrap_or("");
        if csp.is_empty() {
            return findings;
        }
        if let Some(directive) = csp.split(';').find(|d| d.trim().starts_with("style-src")) {
            let value = directive.trim().trim_start_matches("style-src").trim();
            if value.contains("'unsafe-inline'") {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Security,
                    code: "CSPSTYLE-V5001".to_string(),
                    title: "CSP style-src allows unsafe-inline".to_string(),
                    description: "unsafe-inline for styles is common but weakens CSP.".to_string(),
                    url: url.clone(),
                    recommendation: "Consider using nonces or hashes for styles.".to_string(),
                });
            }
        } else {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Security,
                code: "CSPSTYLE-V5002".to_string(),
                title: "CSP missing style-src".to_string(),
                description: "No style-src directive.".to_string(),
                url: url.clone(),
                recommendation: "Add style-src directive.".to_string(),
            });
        }
        findings
    }
}

pub struct CspFrameAncestorsValidator;
impl Default for CspFrameAncestorsValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CspFrameAncestorsValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CspFrameAncestorsValidator {
    fn name(&self) -> &str {
        "csp-frame-ancestors-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let csp = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Content-Security-Policy"))
            .map(|(_, v)| v.as_str())
            .unwrap_or("");
        if csp.is_empty() {
            return findings;
        }
        if !csp
            .split(';')
            .any(|d| d.trim().starts_with("frame-ancestors"))
        {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Security,
                code: "CSPFRAME-V5001".to_string(),
                title: "CSP missing frame-ancestors".to_string(),
                description: "frame-ancestors prevents clickjacking.".to_string(),
                url: url.clone(),
                recommendation: "Add frame-ancestors 'none' or 'self'.".to_string(),
            });
        }
        findings
    }
}

pub struct HstsMaxAgeValidator;
impl Default for HstsMaxAgeValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl HstsMaxAgeValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for HstsMaxAgeValidator {
    fn name(&self) -> &str {
        "hsts-max-age-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let hsts = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Strict-Transport-Security"))
            .map(|(_, v)| v.as_str());
        match hsts {
            None => {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "HSTSMAX-V5001".to_string(),
                    title: "Missing HSTS header".to_string(),
                    description: "No Strict-Transport-Security.".to_string(),
                    url: url.clone(),
                    recommendation: "Add HSTS with max-age >= 31536000.".to_string(),
                });
            }
            Some(val) => {
                let lower = val.to_lowercase();
                if let Some(pos) = lower.find("max-age=") {
                    let after = &lower[pos + 8..];
                    if let Ok(age) = after
                        .chars()
                        .take_while(|c| c.is_ascii_digit())
                        .collect::<String>()
                        .parse::<u64>()
                    {
                        if age < 31536000 {
                            findings.push(Finding {
                                severity: Severity::Warning,
                                category: IssueCategory::Security,
                                code: "HSTSMAX-V5002".to_string(),
                                title: "HSTS max-age too low".to_string(),
                                description: format!("max-age is {age}, recommend 31536000+."),
                                url: url.clone(),
                                recommendation: "Set max-age to at least 31536000.".to_string(),
                            });
                        }
                    }
                } else {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Security,
                        code: "HSTSMAX-V5003".to_string(),
                        title: "HSTS missing max-age".to_string(),
                        description: "HSTS header has no max-age.".to_string(),
                        url: url.clone(),
                        recommendation: "Add max-age directive.".to_string(),
                    });
                }
            }
        }
        findings
    }
}

pub struct HstsIncludeSubDomainsValidator;
impl Default for HstsIncludeSubDomainsValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl HstsIncludeSubDomainsValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for HstsIncludeSubDomainsValidator {
    fn name(&self) -> &str {
        "hsts-include-subdomains-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(val) = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Strict-Transport-Security"))
            .map(|(_, v)| v.as_str())
        {
            if !val.to_lowercase().contains("includesubdomains") {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Security,
                    code: "HSTSSUB-V5001".to_string(),
                    title: "HSTS missing includeSubDomains".to_string(),
                    description: "Subdomains not covered by HSTS.".to_string(),
                    url: url.clone(),
                    recommendation: "Add includeSubDomains directive.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct HstsPreloadValidatorV5;
impl Default for HstsPreloadValidatorV5 {
    fn default() -> Self {
        Self::new()
    }
}
impl HstsPreloadValidatorV5 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for HstsPreloadValidatorV5 {
    fn name(&self) -> &str {
        "hsts-preload-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(val) = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Strict-Transport-Security"))
            .map(|(_, v)| v.as_str())
        {
            if !val.to_lowercase().contains("preload") {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Security,
                    code: "HSTSPRE-V5001".to_string(),
                    title: "HSTS missing preload".to_string(),
                    description: "Domain not in browser preload list.".to_string(),
                    url: url.clone(),
                    recommendation: "Add preload directive.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct XContentTypeOptionsValidatorV5;
impl Default for XContentTypeOptionsValidatorV5 {
    fn default() -> Self {
        Self::new()
    }
}
impl XContentTypeOptionsValidatorV5 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for XContentTypeOptionsValidatorV5 {
    fn name(&self) -> &str {
        "x-content-type-options-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        match ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("X-Content-Type-Options"))
            .map(|(_, v)| v.as_str())
        {
            None => {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "XCTO-V5001".to_string(),
                    title: "Missing X-Content-Type-Options".to_string(),
                    description: "Browsers may MIME-sniff responses.".to_string(),
                    url: url.clone(),
                    recommendation: "Add X-Content-Type-Options: nosniff.".to_string(),
                });
            }
            Some(val) if !val.trim().eq_ignore_ascii_case("nosniff") => {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "XCTO-V5002".to_string(),
                    title: "Invalid X-Content-Type-Options".to_string(),
                    description: format!("Value '{val}', expected 'nosniff'."),
                    url: url.clone(),
                    recommendation: "Set to nosniff.".to_string(),
                });
            }
            _ => {}
        }
        findings
    }
}

pub struct ReferrerPolicyValidatorV5;
impl Default for ReferrerPolicyValidatorV5 {
    fn default() -> Self {
        Self::new()
    }
}
impl ReferrerPolicyValidatorV5 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for ReferrerPolicyValidatorV5 {
    fn name(&self) -> &str {
        "referrer-policy-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        match ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Referrer-Policy"))
            .map(|(_, v)| v.as_str())
        {
            None => {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "RP-V5001".to_string(),
                    title: "Missing Referrer-Policy".to_string(),
                    description: "No referrer policy set.".to_string(),
                    url: url.clone(),
                    recommendation: "Add Referrer-Policy: strict-origin-when-cross-origin."
                        .to_string(),
                });
            }
            Some(val) if val.eq_ignore_ascii_case("unsafe-url") => {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "RP-V5002".to_string(),
                    title: "Referrer-Policy unsafe-url".to_string(),
                    description: "Leaks full URL path and query.".to_string(),
                    url: url.clone(),
                    recommendation: "Use strict-origin-when-cross-origin.".to_string(),
                });
            }
            Some(val) if val.eq_ignore_ascii_case("no-referrer") => {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Security,
                    code: "RP-V5003".to_string(),
                    title: "Referrer-Policy no-referrer".to_string(),
                    description: "No referrer sent at all.".to_string(),
                    url: url.clone(),
                    recommendation: "Consider strict-origin-when-cross-origin.".to_string(),
                });
            }
            _ => {}
        }
        findings
    }
}

pub struct XFrameOptionsValidatorV5;
impl Default for XFrameOptionsValidatorV5 {
    fn default() -> Self {
        Self::new()
    }
}
impl XFrameOptionsValidatorV5 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for XFrameOptionsValidatorV5 {
    fn name(&self) -> &str {
        "x-frame-options-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let xfo = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("X-Frame-Options"))
            .map(|(_, v)| v.as_str());
        let csp_frame = ctx.headers.iter().any(|(k, v)| {
            k.eq_ignore_ascii_case("Content-Security-Policy") && v.contains("frame-ancestors")
        });
        if xfo.is_none() && !csp_frame {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Security,
                code: "XFO-V5001".to_string(),
                title: "No clickjacking protection".to_string(),
                description: "Neither X-Frame-Options nor CSP frame-ancestors.".to_string(),
                url: url.clone(),
                recommendation: "Add X-Frame-Options: DENY or CSP frame-ancestors.".to_string(),
            });
        }
        if let Some(val) = xfo {
            if !val.eq_ignore_ascii_case("DENY") && !val.eq_ignore_ascii_case("SAMEORIGIN") {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "XFO-V5002".to_string(),
                    title: "Invalid X-Frame-Options value".to_string(),
                    description: format!("'{val}' is not DENY or SAMEORIGIN."),
                    url: url.clone(),
                    recommendation: "Use DENY or SAMEORIGIN.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct PermissionsPolicyCameraValidator;
impl Default for PermissionsPolicyCameraValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl PermissionsPolicyCameraValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for PermissionsPolicyCameraValidator {
    fn name(&self) -> &str {
        "permissions-policy-camera-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let pp = ctx
            .headers
            .iter()
            .find(|(k, _)| {
                k.eq_ignore_ascii_case("Permissions-Policy")
                    || k.eq_ignore_ascii_case("Feature-Policy")
            })
            .map(|(_, v)| v.as_str());
        if let Some(val) = pp {
            if val.contains("camera") && val.contains("camera=()") {
                // Explicitly denied - good
            } else if val.contains("camera") && !val.contains("camera=()") {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Security,
                    code: "PPCAM-V5001".to_string(),
                    title: "Camera access not explicitly denied".to_string(),
                    description: "Permissions-Policy doesn't deny camera.".to_string(),
                    url: url.clone(),
                    recommendation: "Add camera=() to Permissions-Policy.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct PermissionsPolicyMicrophoneValidator;
impl Default for PermissionsPolicyMicrophoneValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl PermissionsPolicyMicrophoneValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for PermissionsPolicyMicrophoneValidator {
    fn name(&self) -> &str {
        "permissions-policy-microphone-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let pp = ctx
            .headers
            .iter()
            .find(|(k, _)| {
                k.eq_ignore_ascii_case("Permissions-Policy")
                    || k.eq_ignore_ascii_case("Feature-Policy")
            })
            .map(|(_, v)| v.as_str());
        if let Some(val) = pp {
            if val.contains("microphone") && !val.contains("microphone=()") {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Security,
                    code: "PPMICRO-V5001".to_string(),
                    title: "Microphone access not explicitly denied".to_string(),
                    description: "Permissions-Policy doesn't deny microphone.".to_string(),
                    url: url.clone(),
                    recommendation: "Add microphone=() to Permissions-Policy.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct PermissionsPolicyGeolocationValidator;
impl Default for PermissionsPolicyGeolocationValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl PermissionsPolicyGeolocationValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for PermissionsPolicyGeolocationValidator {
    fn name(&self) -> &str {
        "permissions-policy-geolocation-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let pp = ctx
            .headers
            .iter()
            .find(|(k, _)| {
                k.eq_ignore_ascii_case("Permissions-Policy")
                    || k.eq_ignore_ascii_case("Feature-Policy")
            })
            .map(|(_, v)| v.as_str());
        if let Some(val) = pp {
            if val.contains("geolocation") && !val.contains("geolocation=()") {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Security,
                    code: "PPGEO-V5001".to_string(),
                    title: "Geolocation not explicitly denied".to_string(),
                    description: "Permissions-Policy doesn't deny geolocation.".to_string(),
                    url: url.clone(),
                    recommendation: "Add geolocation=() to Permissions-Policy.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct CoepValidator;
impl Default for CoepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CoepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CoepValidator {
    fn name(&self) -> &str {
        "coep-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if !ctx
            .headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("Cross-Origin-Embedder-Policy"))
        {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Security,
                code: "COEP-V5001".to_string(),
                title: "Missing Cross-Origin-Embedder-Policy".to_string(),
                description: "COEP prevents loading cross-origin resources.".to_string(),
                url: url.clone(),
                recommendation: "Add Cross-Origin-Embedder-Policy: require-corp.".to_string(),
            });
        }
        findings
    }
}

pub struct CoopValidator;
impl Default for CoopValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CoopValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CoopValidator {
    fn name(&self) -> &str {
        "coop-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if !ctx
            .headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("Cross-Origin-Opener-Policy"))
        {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Security,
                code: "COOP-V5001".to_string(),
                title: "Missing Cross-Origin-Opener-Policy".to_string(),
                description: "COOP controls cross-origin references.".to_string(),
                url: url.clone(),
                recommendation: "Add Cross-Origin-Opener-Policy: same-origin.".to_string(),
            });
        }
        findings
    }
}

pub struct CookieSecureFlagValidatorV5;
impl Default for CookieSecureFlagValidatorV5 {
    fn default() -> Self {
        Self::new()
    }
}
impl CookieSecureFlagValidatorV5 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CookieSecureFlagValidatorV5 {
    fn name(&self) -> &str {
        "cookie-secure-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for (k, v) in ctx.headers {
            if !k.eq_ignore_ascii_case("set-cookie") {
                continue;
            }
            let lower = v.to_lowercase();
            if !lower.contains("secure") {
                let name = v.split('=').next().unwrap_or("cookie").trim().to_string();
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "COOKIESEC-V5001".to_string(),
                    title: format!("Cookie '{name}' missing Secure"),
                    description: "Cookie transmitted over HTTP.".to_string(),
                    url: url.clone(),
                    recommendation: "Add Secure flag.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct CookieHttpOnlyFlagValidatorV5;
impl Default for CookieHttpOnlyFlagValidatorV5 {
    fn default() -> Self {
        Self::new()
    }
}
impl CookieHttpOnlyFlagValidatorV5 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CookieHttpOnlyFlagValidatorV5 {
    fn name(&self) -> &str {
        "cookie-httponly-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for (k, v) in ctx.headers {
            if !k.eq_ignore_ascii_case("set-cookie") {
                continue;
            }
            let lower = v.to_lowercase();
            if !lower.contains("httponly") {
                let name = v.split('=').next().unwrap_or("cookie").trim().to_string();
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "COOKIEHTTP-V5001".to_string(),
                    title: format!("Cookie '{name}' missing HttpOnly"),
                    description: "Cookie accessible to JavaScript.".to_string(),
                    url: url.clone(),
                    recommendation: "Add HttpOnly flag.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct CookieSameSiteValidator;
impl Default for CookieSameSiteValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CookieSameSiteValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CookieSameSiteValidator {
    fn name(&self) -> &str {
        "cookie-samesite-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for (k, v) in ctx.headers {
            if !k.eq_ignore_ascii_case("set-cookie") {
                continue;
            }
            let lower = v.to_lowercase();
            if !lower.contains("samesite") {
                let name = v.split('=').next().unwrap_or("cookie").trim().to_string();
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "COOKIESAME-V5001".to_string(),
                    title: format!("Cookie '{name}' missing SameSite"),
                    description: "Without SameSite, cookie is vulnerable to CSRF.".to_string(),
                    url: url.clone(),
                    recommendation: "Add SameSite=Strict or Lax.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct MixedContentScriptValidatorV5;
impl Default for MixedContentScriptValidatorV5 {
    fn default() -> Self {
        Self::new()
    }
}
impl MixedContentScriptValidatorV5 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for MixedContentScriptValidatorV5 {
    fn name(&self) -> &str {
        "mixed-content-script-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if !url.starts_with("https://") {
            return findings;
        }
        if let Some(body) = ctx.body {
            let lower = body.to_lowercase();
            let http_scripts =
                lower.matches("src=\"http://").count() + lower.matches("src='http://").count();
            if http_scripts > 0 {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "MIXSCRIPT-V5001".to_string(),
                    title: format!("{http_scripts} HTTP script(s) on HTTPS page"),
                    description: "Mixed content scripts are blocked by browsers.".to_string(),
                    url: url.clone(),
                    recommendation: "Change script URLs to HTTPS.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct MixedContentStylesheetValidator;
impl Default for MixedContentStylesheetValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl MixedContentStylesheetValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for MixedContentStylesheetValidator {
    fn name(&self) -> &str {
        "mixed-content-stylesheet-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if !url.starts_with("https://") {
            return findings;
        }
        if let Some(body) = ctx.body {
            let lower = body.to_lowercase();
            let http_css =
                lower.matches("href=\"http://").count() + lower.matches("href='http://").count();
            if http_css > 0 {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "MIXCSS-V5001".to_string(),
                    title: format!("{http_css} HTTP stylesheet(s) on HTTPS page"),
                    description: "Mixed content stylesheets degrade security.".to_string(),
                    url: url.clone(),
                    recommendation: "Change stylesheet URLs to HTTPS.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct SriValidator;
impl Default for SriValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl SriValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for SriValidator {
    fn name(&self) -> &str {
        "sri-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(body) = ctx.body {
            let lower = body.to_lowercase();
            let _script_count = lower.matches("<script").count();
            let sri_count = lower.matches("integrity=").count();
            let external_scripts = ctx.page.scripts.iter().filter(|s| s.src.is_some()).count();
            if external_scripts > 0 && sri_count == 0 {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "SRI-V5001".to_string(),
                    title: "No SRI on external scripts".to_string(),
                    description: format!(
                        "{external_scripts} external script(s) without integrity."
                    ),
                    url: url.clone(),
                    recommendation: "Add integrity attribute to external scripts.".to_string(),
                });
            }
        }
        findings
    }
}

// =========================================================================
// SEO V5 Analyzers (51-65)
// =========================================================================

pub struct CspConnectSrcValidator;
impl Default for CspConnectSrcValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CspConnectSrcValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CspConnectSrcValidator {
    fn name(&self) -> &str {
        "csp-connect-src-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let csp = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Content-Security-Policy"))
            .map(|(_, v)| v.as_str())
            .unwrap_or("");
        if csp.is_empty() {
            return findings;
        }
        if !csp
            .split(';')
            .any(|d| d.trim().starts_with("connect-src") || d.trim().starts_with("default-src"))
        {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Security,
                code: "CSPEXEC-V6061".to_string(),
                title: "CSP missing connect-src".to_string(),
                description: "No connect-src or default-src directive limits fetch/XHR targets."
                    .to_string(),
                url: url.clone(),
                recommendation: "Add connect-src to restrict API endpoints.".to_string(),
            });
        }
        findings
    }
}

pub struct CspFontSrcValidator;
impl Default for CspFontSrcValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CspFontSrcValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CspFontSrcValidator {
    fn name(&self) -> &str {
        "csp-font-src-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let csp = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Content-Security-Policy"))
            .map(|(_, v)| v.as_str())
            .unwrap_or("");
        if csp.is_empty() {
            return findings;
        }
        if !csp
            .split(';')
            .any(|d| d.trim().starts_with("font-src") || d.trim().starts_with("default-src"))
        {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Security,
                code: "CSPFONT-V6062".to_string(),
                title: "CSP missing font-src".to_string(),
                description: "No font-src directive limits font loading.".to_string(),
                url: url.clone(),
                recommendation: "Add font-src directive.".to_string(),
            });
        }
        findings
    }
}

pub struct HstsMaxAgeThresholdValidator;
impl Default for HstsMaxAgeThresholdValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl HstsMaxAgeThresholdValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for HstsMaxAgeThresholdValidator {
    fn name(&self) -> &str {
        "hsts-max-age-threshold-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(val) = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Strict-Transport-Security"))
            .map(|(_, v)| v.as_str())
        {
            let lower = val.to_lowercase();
            if let Some(pos) = lower.find("max-age=") {
                let after = &lower[pos + 8..];
                if let Ok(age) = after
                    .chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect::<String>()
                    .parse::<u64>()
                {
                    if age < 15768000 {
                        findings.push(Finding {
                            severity: Severity::Warning,
                            category: IssueCategory::Security,
                            code: "HSTSTHRESH-V6063".to_string(),
                            title: "HSTS max-age below recommended threshold".to_string(),
                            description: format!(
                                "max-age is {age}, recommended >= 15768000 (6 months)."
                            ),
                            url: url.clone(),
                            recommendation: "Set max-age to at least 15768000.".to_string(),
                        });
                    }
                }
            }
        }
        findings
    }
}

pub struct HstsPreloadListCheckValidator;
impl Default for HstsPreloadListCheckValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl HstsPreloadListCheckValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for HstsPreloadListCheckValidator {
    fn name(&self) -> &str {
        "hsts-preload-list-check-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(val) = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Strict-Transport-Security"))
            .map(|(_, v)| v.as_str())
        {
            let lower = val.to_lowercase();
            if lower.contains("includesubdomains") && lower.contains("preload") {
                if let Some(pos) = lower.find("max-age=") {
                    let after = &lower[pos + 8..];
                    if let Ok(age) = after
                        .chars()
                        .take_while(|c| c.is_ascii_digit())
                        .collect::<String>()
                        .parse::<u64>()
                    {
                        if age >= 31536000 {
                            return findings;
                        }
                    }
                }
            }
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Security,
                code: "HSTSPRELIST-V6064".to_string(),
                title: "HSTS preload readiness incomplete".to_string(),
                description:
                    "Missing includeSubDomains, preload, or sufficient max-age for preload list."
                        .to_string(),
                url: url.clone(),
                recommendation: "Add includeSubDomains; preload; max-age=31536000.".to_string(),
            });
        }
        findings
    }
}

pub struct CookieSecureFlagDeepValidator;
impl Default for CookieSecureFlagDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CookieSecureFlagDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CookieSecureFlagDeepValidator {
    fn name(&self) -> &str {
        "cookie-secure-flag-deep-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for (k, v) in ctx.headers {
            if !k.eq_ignore_ascii_case("set-cookie") {
                continue;
            }
            let lower = v.to_lowercase();
            let name = v.split('=').next().unwrap_or("cookie").trim().to_string();
            if !lower.contains("secure") {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "COOKIESEC-V6065".to_string(),
                    title: format!("Cookie '{name}' missing Secure flag"),
                    description: "Cookie transmitted over HTTP.".to_string(),
                    url: url.clone(),
                    recommendation: "Add Secure flag to cookie.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct CookieHttpOnlyFlagDeepValidator;
impl Default for CookieHttpOnlyFlagDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CookieHttpOnlyFlagDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CookieHttpOnlyFlagDeepValidator {
    fn name(&self) -> &str {
        "cookie-httponly-flag-deep-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for (k, v) in ctx.headers {
            if !k.eq_ignore_ascii_case("set-cookie") {
                continue;
            }
            let lower = v.to_lowercase();
            let name = v.split('=').next().unwrap_or("cookie").trim().to_string();
            if !lower.contains("httponly") {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "COOKIEHTTP-V6066".to_string(),
                    title: format!("Cookie '{name}' missing HttpOnly flag"),
                    description: "Cookie accessible to JavaScript.".to_string(),
                    url: url.clone(),
                    recommendation: "Add HttpOnly flag to cookie.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct CookieSameSiteDeepValidator;
impl Default for CookieSameSiteDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CookieSameSiteDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CookieSameSiteDeepValidator {
    fn name(&self) -> &str {
        "cookie-samesite-deep-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for (k, v) in ctx.headers {
            if !k.eq_ignore_ascii_case("set-cookie") {
                continue;
            }
            let lower = v.to_lowercase();
            let name = v.split('=').next().unwrap_or("cookie").trim().to_string();
            if !lower.contains("samesite") {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "COOKIESAME-V6067".to_string(),
                    title: format!("Cookie '{name}' missing SameSite attribute"),
                    description: "Without SameSite, cookie is vulnerable to CSRF.".to_string(),
                    url: url.clone(),
                    recommendation: "Add SameSite=Strict or Lax.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct MixedContentIframeValidator;
impl Default for MixedContentIframeValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl MixedContentIframeValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for MixedContentIframeValidator {
    fn name(&self) -> &str {
        "mixed-content-iframe-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if !url.starts_with("https://") {
            return findings;
        }
        if let Some(body) = ctx.body {
            let lower = body.to_lowercase();
            let http_iframes = lower
                .matches("<iframe")
                .filter(|_| body.to_lowercase().contains("http://"))
                .count();
            if http_iframes > 0 {
                findings.push(Finding {
                    severity: Severity::Critical,
                    category: IssueCategory::Security,
                    code: "MIXIFRAME-V6068".to_string(),
                    title: "Mixed content iframe detected".to_string(),
                    description: format!(
                        "Found {http_iframes} iframe(s) loading HTTP content on HTTPS page."
                    ),
                    url: url.clone(),
                    recommendation: "Change iframe sources to HTTPS.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct CorsWildcardValidator;
impl Default for CorsWildcardValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CorsWildcardValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CorsWildcardValidator {
    fn name(&self) -> &str {
        "cors-wildcard-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(val) = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Access-Control-Allow-Origin"))
            .map(|(_, v)| v.as_str())
        {
            if val == "*" {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Security,
                    code: "CORSWILD-V6069".to_string(),
                    title: "CORS allows all origins".to_string(),
                    description: "Access-Control-Allow-Origin is set to '*'.".to_string(),
                    url: url.clone(),
                    recommendation: "Restrict CORS to specific trusted origins.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct CorsMissingHeaderValidator;
impl Default for CorsMissingHeaderValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CorsMissingHeaderValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CorsMissingHeaderValidator {
    fn name(&self) -> &str {
        "cors-missing-header-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let has_cors = ctx
            .headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("Access-Control-Allow-Origin"));
        if !has_cors {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Security,
                code: "CORSMISS-V6070".to_string(),
                title: "No CORS headers".to_string(),
                description: "No Access-Control-Allow-Origin header present.".to_string(),
                url: url.clone(),
                recommendation: "Add CORS headers if cross-origin requests are needed.".to_string(),
            });
        }
        findings
    }
}

pub struct ReferrerPolicyStrictValidator;
impl Default for ReferrerPolicyStrictValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl ReferrerPolicyStrictValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for ReferrerPolicyStrictValidator {
    fn name(&self) -> &str {
        "referrer-policy-strict-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(val) = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Referrer-Policy"))
            .map(|(_, v)| v.as_str())
        {
            let lower = val.to_lowercase();
            if lower.contains("unsafe-url") || lower.contains("no-referrer-when-downgrade") {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "RPSTRICT-V6071".to_string(),
                    title: "Referrer-Policy too permissive".to_string(),
                    description: format!("'{val}' may leak sensitive URL information."),
                    url: url.clone(),
                    recommendation: "Use strict-origin-when-cross-origin or stricter.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct XFrameOptionsMissingValidator;
impl Default for XFrameOptionsMissingValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl XFrameOptionsMissingValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for XFrameOptionsMissingValidator {
    fn name(&self) -> &str {
        "x-frame-options-missing-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let has_xfo = ctx
            .headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("X-Frame-Options"));
        let has_csp_frame = ctx.headers.iter().any(|(k, v)| {
            k.eq_ignore_ascii_case("Content-Security-Policy") && v.contains("frame-ancestors")
        });
        if !has_xfo && !has_csp_frame {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Security,
                code: "XFOMISS-V6072".to_string(),
                title: "No clickjacking protection".to_string(),
                description: "Neither X-Frame-Options nor CSP frame-ancestors set.".to_string(),
                url: url.clone(),
                recommendation: "Add X-Frame-Options: DENY or CSP frame-ancestors.".to_string(),
            });
        }
        findings
    }
}

pub struct PermissionsPolicyPaymentValidator;
impl Default for PermissionsPolicyPaymentValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl PermissionsPolicyPaymentValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for PermissionsPolicyPaymentValidator {
    fn name(&self) -> &str {
        "permissions-policy-payment-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let pp = ctx
            .headers
            .iter()
            .find(|(k, _)| {
                k.eq_ignore_ascii_case("Permissions-Policy")
                    || k.eq_ignore_ascii_case("Feature-Policy")
            })
            .map(|(_, v)| v.as_str());
        if let Some(val) = pp {
            if val.contains("payment") && !val.contains("payment=()") {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Security,
                    code: "PPPAY-V6073".to_string(),
                    title: "Payment API access not explicitly denied".to_string(),
                    description: "Permissions-Policy doesn't deny payment API.".to_string(),
                    url: url.clone(),
                    recommendation: "Add payment=() to Permissions-Policy.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct PermissionsPolicyFullscreenValidator;
impl Default for PermissionsPolicyFullscreenValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl PermissionsPolicyFullscreenValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for PermissionsPolicyFullscreenValidator {
    fn name(&self) -> &str {
        "permissions-policy-fullscreen-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let pp = ctx
            .headers
            .iter()
            .find(|(k, _)| {
                k.eq_ignore_ascii_case("Permissions-Policy")
                    || k.eq_ignore_ascii_case("Feature-Policy")
            })
            .map(|(_, v)| v.as_str());
        if let Some(val) = pp {
            if val.contains("fullscreen") && !val.contains("fullscreen=(self)") {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Security,
                    code: "PPFULL-V6074".to_string(),
                    title: "Fullscreen access not restricted to self".to_string(),
                    description: "Permissions-Policy allows fullscreen from non-self origins."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add fullscreen=(self) to Permissions-Policy.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct PermissionsPolicyXrVrValidator;
impl Default for PermissionsPolicyXrVrValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl PermissionsPolicyXrVrValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for PermissionsPolicyXrVrValidator {
    fn name(&self) -> &str {
        "permissions-policy-xr-vr-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let pp = ctx
            .headers
            .iter()
            .find(|(k, _)| {
                k.eq_ignore_ascii_case("Permissions-Policy")
                    || k.eq_ignore_ascii_case("Feature-Policy")
            })
            .map(|(_, v)| v.as_str());
        if let Some(val) = pp {
            if val.contains("xr-spatial-tracking") && !val.contains("xr-spatial-tracking=()") {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Security,
                    code: "PPXR-V6075".to_string(),
                    title: "XR spatial tracking access not denied".to_string(),
                    description: "Permissions-Policy doesn't deny XR spatial tracking.".to_string(),
                    url: url.clone(),
                    recommendation: "Add xr-spatial-tracking=() to Permissions-Policy.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct CoepRequireCorpValidator;
impl Default for CoepRequireCorpValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CoepRequireCorpValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CoepRequireCorpValidator {
    fn name(&self) -> &str {
        "coep-require-corp-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(val) = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Cross-Origin-Embedder-Policy"))
            .map(|(_, v)| v.as_str())
        {
            if val.eq_ignore_ascii_case("unsafe-none") {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "COEPU-V6076".to_string(),
                    title: "COEP set to unsafe-none".to_string(),
                    description: "COEP unsafe-none provides no isolation.".to_string(),
                    url: url.clone(),
                    recommendation: "Set COEP to require-corp.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct CoopSameOriginValidator;
impl Default for CoopSameOriginValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CoopSameOriginValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CoopSameOriginValidator {
    fn name(&self) -> &str {
        "coop-same-origin-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(val) = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Cross-Origin-Opener-Policy"))
            .map(|(_, v)| v.as_str())
        {
            if val.eq_ignore_ascii_case("unsafe-none") {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Security,
                    code: "COOPU-V6077".to_string(),
                    title: "COOP set to unsafe-none".to_string(),
                    description: "COOP unsafe-none provides no isolation.".to_string(),
                    url: url.clone(),
                    recommendation: "Set COOP to same-origin.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct CspObjectSrcNoneValidator;
impl Default for CspObjectSrcNoneValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CspObjectSrcNoneValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CspObjectSrcNoneValidator {
    fn name(&self) -> &str {
        "csp-object-src-none-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let csp = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Content-Security-Policy"))
            .map(|(_, v)| v.as_str())
            .unwrap_or("");
        if csp.is_empty() {
            return findings;
        }
        if let Some(directive) = csp.split(';').find(|d| d.trim().starts_with("object-src")) {
            let value = directive.trim().trim_start_matches("object-src").trim();
            if value != "'none'" && !value.is_empty() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Security,
                    code: "CSPOBJ-V6078".to_string(),
                    title: "CSP object-src not set to 'none'".to_string(),
                    description: format!("object-src is '{value}', recommend 'none'."),
                    url: url.clone(),
                    recommendation: "Set object-src to 'none'.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct CspBaseUriSelfValidator;
impl Default for CspBaseUriSelfValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CspBaseUriSelfValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CspBaseUriSelfValidator {
    fn name(&self) -> &str {
        "csp-base-uri-self-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let csp = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Content-Security-Policy"))
            .map(|(_, v)| v.as_str())
            .unwrap_or("");
        if csp.is_empty() {
            return findings;
        }
        if !csp.split(';').any(|d| d.trim().starts_with("base-uri")) {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Security,
                code: "CSPBASE-V6079".to_string(),
                title: "CSP missing base-uri".to_string(),
                description: "Without base-uri, attackers can inject <base> tags.".to_string(),
                url: url.clone(),
                recommendation: "Add base-uri 'self'.".to_string(),
            });
        }
        findings
    }
}

pub struct CspFormActionSelfValidator;
impl Default for CspFormActionSelfValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CspFormActionSelfValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CspFormActionSelfValidator {
    fn name(&self) -> &str {
        "csp-form-action-self-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let csp = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Content-Security-Policy"))
            .map(|(_, v)| v.as_str())
            .unwrap_or("");
        if csp.is_empty() {
            return findings;
        }
        if !csp.split(';').any(|d| d.trim().starts_with("form-action")) {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Security,
                code: "CSPFORM-V6080".to_string(),
                title: "CSP missing form-action".to_string(),
                description: "Without form-action, forms could submit to attacker URLs."
                    .to_string(),
                url: url.clone(),
                recommendation: "Add form-action 'self'.".to_string(),
            });
        }
        findings
    }
}

// =========================================================================
// V6 SEO Validators (66-85)
// =========================================================================

pub struct CspScriptSrcSelfValidator;
impl Default for CspScriptSrcSelfValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CspScriptSrcSelfValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CspScriptSrcSelfValidator {
    fn name(&self) -> &str {
        "csp-script-src-self-v7"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let csp = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Content-Security-Policy"))
            .map(|(_, v)| v.as_str())
            .unwrap_or("");
        if csp.is_empty() {
            return findings;
        }
        for directive in csp.split(';') {
            let d = directive.trim();
            if d.starts_with("script-src") {
                if !d.contains("'self'") {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Security,
                        code: "CSPSS001".to_string(),
                        title: "CSP script-src missing 'self'".to_string(),
                        description: "script-src directive does not include 'self'.".to_string(),
                        url: url.clone(),
                        recommendation: "Add 'self' to script-src directive.".to_string(),
                    });
                }
                break;
            }
        }
        findings
    }
}

pub struct CspStyleSrcSelfValidator;
impl Default for CspStyleSrcSelfValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CspStyleSrcSelfValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CspStyleSrcSelfValidator {
    fn name(&self) -> &str {
        "csp-style-src-self-v7"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let csp = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Content-Security-Policy"))
            .map(|(_, v)| v.as_str())
            .unwrap_or("");
        if csp.is_empty() {
            return findings;
        }
        for directive in csp.split(';') {
            let d = directive.trim();
            if d.starts_with("style-src") {
                if !d.contains("'self'") {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Security,
                        code: "CSPSTY001".to_string(),
                        title: "CSP style-src missing 'self'".to_string(),
                        description: "style-src directive does not include 'self'.".to_string(),
                        url: url.clone(),
                        recommendation: "Add 'self' to style-src directive.".to_string(),
                    });
                }
                break;
            }
        }
        findings
    }
}

pub struct CspConnectSrcAnalysisValidator;
impl Default for CspConnectSrcAnalysisValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CspConnectSrcAnalysisValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CspConnectSrcAnalysisValidator {
    fn name(&self) -> &str {
        "csp-connect-src-analysis-v7"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let csp = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Content-Security-Policy"))
            .map(|(_, v)| v.as_str())
            .unwrap_or("");
        if csp.is_empty() {
            return findings;
        }
        for directive in csp.split(';') {
            let d = directive.trim();
            if d.starts_with("connect-src") {
                if d.contains("*") {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Security,
                        code: "CSPCON001".to_string(),
                        title: "CSP connect-src allows wildcard".to_string(),
                        description: "connect-src directive includes wildcard source.".to_string(),
                        url: url.clone(),
                        recommendation: "Replace '*' with specific origins in connect-src."
                            .to_string(),
                    });
                }
                break;
            }
        }
        findings
    }
}

pub struct CspFontSrcAnalysisValidator;
impl Default for CspFontSrcAnalysisValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CspFontSrcAnalysisValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CspFontSrcAnalysisValidator {
    fn name(&self) -> &str {
        "csp-font-src-analysis-v7"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let csp = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Content-Security-Policy"))
            .map(|(_, v)| v.as_str())
            .unwrap_or("");
        if csp.is_empty() {
            return findings;
        }
        for directive in csp.split(';') {
            let d = directive.trim();
            if d.starts_with("font-src") {
                if d.contains("*") {
                    findings.push(Finding {
                        severity: Severity::Info,
                        category: IssueCategory::Security,
                        code: "CSPFNT001".to_string(),
                        title: "CSP font-src allows wildcard".to_string(),
                        description: "font-src directive includes wildcard source.".to_string(),
                        url: url.clone(),
                        recommendation: "Replace '*' with specific origins in font-src."
                            .to_string(),
                    });
                }
                break;
            }
        }
        findings
    }
}

pub struct CspObjectSrcNoneDeepValidator;
impl Default for CspObjectSrcNoneDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CspObjectSrcNoneDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CspObjectSrcNoneDeepValidator {
    fn name(&self) -> &str {
        "csp-object-src-none-deep-v7"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let csp = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Content-Security-Policy"))
            .map(|(_, v)| v.as_str())
            .unwrap_or("");
        if csp.is_empty() {
            return findings;
        }
        let mut found = false;
        for directive in csp.split(';') {
            let d = directive.trim();
            if d.starts_with("object-src") {
                found = true;
                if !d.contains("'none'") {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Security,
                        code: "CSPOBJ001".to_string(),
                        title: "CSP object-src not set to 'none'".to_string(),
                        description: "object-src should be 'none' to block plugins.".to_string(),
                        url: url.clone(),
                        recommendation: "Set object-src to 'none'.".to_string(),
                    });
                }
                break;
            }
        }
        if !found {
            let has_default = csp.split(';').any(|d| d.trim().starts_with("default-src"));
            if !has_default {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Security,
                    code: "CSPOBJ001".to_string(),
                    title: "CSP missing object-src directive".to_string(),
                    description: "No object-src or default-src directive found.".to_string(),
                    url: url.clone(),
                    recommendation: "Add object-src 'none' to CSP.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct CspBaseUriSelfDeepValidator;
impl Default for CspBaseUriSelfDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CspBaseUriSelfDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CspBaseUriSelfDeepValidator {
    fn name(&self) -> &str {
        "csp-base-uri-self-deep-v7"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let csp = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Content-Security-Policy"))
            .map(|(_, v)| v.as_str())
            .unwrap_or("");
        if csp.is_empty() {
            return findings;
        }
        let mut found = false;
        for directive in csp.split(';') {
            let d = directive.trim();
            if d.starts_with("base-uri") {
                found = true;
                if !d.contains("'self'") {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Security,
                        code: "CSPBASE001".to_string(),
                        title: "CSP base-uri missing 'self'".to_string(),
                        description: "base-uri directive does not include 'self'.".to_string(),
                        url: url.clone(),
                        recommendation: "Add 'self' to base-uri directive.".to_string(),
                    });
                }
                break;
            }
        }
        if !found {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Security,
                code: "CSPBASE001".to_string(),
                title: "CSP missing base-uri directive".to_string(),
                description: "No base-uri directive found. An attacker could inject a base tag."
                    .to_string(),
                url: url.clone(),
                recommendation: "Add base-uri 'self' to CSP.".to_string(),
            });
        }
        findings
    }
}

pub struct CspFormActionSelfDeepValidator;
impl Default for CspFormActionSelfDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CspFormActionSelfDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CspFormActionSelfDeepValidator {
    fn name(&self) -> &str {
        "csp-form-action-self-deep-v7"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let csp = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Content-Security-Policy"))
            .map(|(_, v)| v.as_str())
            .unwrap_or("");
        if csp.is_empty() {
            return findings;
        }
        let mut found = false;
        for directive in csp.split(';') {
            let d = directive.trim();
            if d.starts_with("form-action") {
                found = true;
                if !d.contains("'self'") {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Security,
                        code: "CSPFORM001".to_string(),
                        title: "CSP form-action missing 'self'".to_string(),
                        description: "form-action directive does not include 'self'.".to_string(),
                        url: url.clone(),
                        recommendation: "Add 'self' to form-action directive.".to_string(),
                    });
                }
                break;
            }
        }
        if !found {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Security,
                code: "CSPFORM001".to_string(),
                title: "CSP missing form-action directive".to_string(),
                description: "No form-action directive found.".to_string(),
                url: url.clone(),
                recommendation: "Add form-action 'self' to CSP.".to_string(),
            });
        }
        findings
    }
}

pub struct HstsPreloadReadyDeepValidator;
impl Default for HstsPreloadReadyDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl HstsPreloadReadyDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for HstsPreloadReadyDeepValidator {
    fn name(&self) -> &str {
        "hsts-preload-ready-deep-v7"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(val) = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Strict-Transport-Security"))
            .map(|(_, v)| v.as_str())
        {
            let lower = val.to_lowercase();
            let has_isd = lower.contains("includesubdomains");
            let has_preload = lower.contains("preload");
            let max_age = lower
                .find("max-age=")
                .map(|pos| {
                    let after = &lower[pos + 8..];
                    after
                        .chars()
                        .take_while(|c| c.is_ascii_digit())
                        .collect::<String>()
                        .parse::<u64>()
                        .unwrap_or(0)
                })
                .unwrap_or(0);
            if !has_isd || !has_preload || max_age < 31536000 {
                let mut missing = Vec::new();
                if !has_isd {
                    missing.push("includeSubDomains");
                }
                if !has_preload {
                    missing.push("preload");
                }
                if max_age < 31536000 {
                    missing.push("max-age>=31536000");
                }
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Security,
                    code: "HSTSPR001".to_string(),
                    title: "HSTS preload readiness incomplete".to_string(),
                    description: format!("Missing: {}.", missing.join(", ")),
                    url: url.clone(),
                    recommendation: "Add includeSubDomains; preload; max-age=31536000.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct HstsMaxAgeDeepValidator;
impl Default for HstsMaxAgeDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl HstsMaxAgeDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for HstsMaxAgeDeepValidator {
    fn name(&self) -> &str {
        "hsts-max-age-deep-v7"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(val) = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Strict-Transport-Security"))
            .map(|(_, v)| v.as_str())
        {
            let lower = val.to_lowercase();
            if let Some(pos) = lower.find("max-age=") {
                let after = &lower[pos + 8..];
                if let Ok(age) = after
                    .chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect::<String>()
                    .parse::<u64>()
                {
                    if age < 31536000 {
                        findings.push(Finding {
                            severity: Severity::Warning,
                            category: IssueCategory::Security,
                            code: "HSTSMAX001".to_string(),
                            title: "HSTS max-age below 1 year".to_string(),
                            description: format!(
                                "max-age is {age}, recommended >= 31536000 (1 year)."
                            ),
                            url: url.clone(),
                            recommendation: "Set max-age to at least 31536000.".to_string(),
                        });
                    }
                }
            }
        }
        findings
    }
}

pub struct CookieSecureDeepDeepValidator;
impl Default for CookieSecureDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CookieSecureDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CookieSecureDeepDeepValidator {
    fn name(&self) -> &str {
        "cookie-secure-deep-deep-v7"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let mut cookie_count = 0;
        let mut insecure_count = 0;
        for (k, v) in ctx.headers {
            if !k.eq_ignore_ascii_case("set-cookie") {
                continue;
            }
            cookie_count += 1;
            let lower = v.to_lowercase();
            let name = v.split('=').next().unwrap_or("cookie").trim().to_string();
            let is_session = lower.contains("session");
            if !lower.contains("secure") && !is_session {
                insecure_count += 1;
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "COOKIESEC001".to_string(),
                    title: format!("Cookie '{name}' missing Secure flag"),
                    description: "Non-session cookie transmitted without Secure flag.".to_string(),
                    url: url.clone(),
                    recommendation: "Add Secure flag to cookie.".to_string(),
                });
            }
        }
        if cookie_count > 0 && insecure_count == 0 {
            // All cookies are fine - no finding needed
        }
        findings
    }
}

pub struct CookieHttpOnlyDeepDeepValidator;
impl Default for CookieHttpOnlyDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CookieHttpOnlyDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CookieHttpOnlyDeepDeepValidator {
    fn name(&self) -> &str {
        "cookie-httponly-deep-deep-v7"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for (k, v) in ctx.headers {
            if !k.eq_ignore_ascii_case("set-cookie") {
                continue;
            }
            let lower = v.to_lowercase();
            let name = v.split('=').next().unwrap_or("cookie").trim().to_string();
            if !lower.contains("httponly") {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "COOKIEHTTP001".to_string(),
                    title: format!("Cookie '{name}' missing HttpOnly flag"),
                    description: "Cookie accessible to JavaScript without HttpOnly.".to_string(),
                    url: url.clone(),
                    recommendation: "Add HttpOnly flag to cookie.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct CookieSameSiteDeepDeepValidator;
impl Default for CookieSameSiteDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CookieSameSiteDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CookieSameSiteDeepDeepValidator {
    fn name(&self) -> &str {
        "cookie-samesite-deep-deep-v7"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for (k, v) in ctx.headers {
            if !k.eq_ignore_ascii_case("set-cookie") {
                continue;
            }
            let lower = v.to_lowercase();
            let name = v.split('=').next().unwrap_or("cookie").trim().to_string();
            if !lower.contains("samesite") {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "COOKIESAME001".to_string(),
                    title: format!("Cookie '{name}' missing SameSite attribute"),
                    description: "Without SameSite, cookie is vulnerable to CSRF attacks."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add SameSite=Strict or SameSite=Lax.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct MixedContentIframeDeepValidator;
impl Default for MixedContentIframeDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl MixedContentIframeDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for MixedContentIframeDeepValidator {
    fn name(&self) -> &str {
        "mixed-content-iframe-deep-v7"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if !url.starts_with("https://") {
            return findings;
        }
        if let Some(body) = ctx.body {
            let lower = body.to_lowercase();
            let http_iframe_count = lower.matches("<iframe").count();
            let http_in_iframes = body.matches("http://").count();
            if http_iframe_count > 0 && http_in_iframes > 0 {
                findings.push(Finding {
                    severity: Severity::Critical,
                    category: IssueCategory::Security,
                    code: "MIXIFRAME001".to_string(),
                    title: "Mixed content in iframes".to_string(),
                    description: format!(
                        "Found {http_iframe_count} iframe(s) with HTTP resources on HTTPS page."
                    ),
                    url: url.clone(),
                    recommendation: "Change all iframe sources to HTTPS.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct CorsWildcardDeepValidator;
impl Default for CorsWildcardDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CorsWildcardDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CorsWildcardDeepValidator {
    fn name(&self) -> &str {
        "cors-wildcard-deep-v7"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(val) = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Access-Control-Allow-Origin"))
            .map(|(_, v)| v.as_str())
        {
            if val == "*" {
                let has_credentials = ctx.headers.iter().any(|(k, v)| {
                    k.eq_ignore_ascii_case("Access-Control-Allow-Credentials")
                        && v.eq_ignore_ascii_case("true")
                });
                if has_credentials {
                    findings.push(Finding { severity: Severity::Critical, category: IssueCategory::Security, code: "CORSWILD001".to_string(), title: "CORS wildcard with credentials".to_string(), description: "Access-Control-Allow-Origin is '*' with Access-Control-Allow-Credentials: true. This is a security vulnerability.".to_string(), url: url.clone(), recommendation: "Use a specific origin instead of '*' when credentials are allowed.".to_string() });
                } else {
                    findings.push(Finding {
                        severity: Severity::Info,
                        category: IssueCategory::Security,
                        code: "CORSWILD001".to_string(),
                        title: "CORS allows all origins".to_string(),
                        description: "Access-Control-Allow-Origin is set to '*'.".to_string(),
                        url: url.clone(),
                        recommendation: "Consider restricting CORS to specific origins."
                            .to_string(),
                    });
                }
            }
        }
        findings
    }
}

pub struct ReferrerPolicyStrictDeepValidator;
impl Default for ReferrerPolicyStrictDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl ReferrerPolicyStrictDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for ReferrerPolicyStrictDeepValidator {
    fn name(&self) -> &str {
        "referrer-policy-strict-deep-v7"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(val) = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Referrer-Policy"))
            .map(|(_, v)| v.as_str())
        {
            let lower = val.to_lowercase();
            let is_strict = lower == "no-referrer"
                || lower == "same-origin"
                || lower == "strict-origin"
                || lower == "strict-origin-when-cross-origin"
                || lower == "no-referrer-when-downgrade";
            if !is_strict {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Security,
                    code: "RPSTRICT001".to_string(),
                    title: "Referrer-Policy not using strict policy".to_string(),
                    description: format!(
                        "Current policy is '{val}'. Consider a stricter policy for better privacy."
                    ),
                    url: url.clone(),
                    recommendation:
                        "Use no-referrer, same-origin, or strict-origin-when-cross-origin."
                            .to_string(),
                });
            }
        }
        findings
    }
}

// =========================================================================
// V7 SEO Validators (36-50)
// =========================================================================

pub struct PermissionsPolicyPaymentDeepValidator;
impl Default for PermissionsPolicyPaymentDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl PermissionsPolicyPaymentDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for PermissionsPolicyPaymentDeepValidator {
    fn name(&self) -> &str {
        "permissions-policy-payment-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let pp = ctx
            .headers
            .iter()
            .find(|(k, _)| {
                k.eq_ignore_ascii_case("Permissions-Policy")
                    || k.eq_ignore_ascii_case("Feature-Policy")
            })
            .map(|(_, v)| v.as_str());
        if let Some(val) = pp {
            let lower = val.to_lowercase();
            if !lower.contains("payment") {
                return findings;
            }
            if lower.trim() == "payment" {
                return findings;
            }
            if lower.contains("payment=()")
                || lower.contains("payment ()")
                || lower.contains("payment *")
                || lower.contains("payment=*")
                || (lower.contains("payment (self)") && !lower.contains("payment=(self)"))
            {
                return findings;
            }
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Security,
                code: "PPAYDP001".to_string(),
                title: "Payment API access not explicitly denied".to_string(),
                description: "Permissions-Policy mentions payment but doesn't explicitly deny it."
                    .to_string(),
                url: url.clone(),
                recommendation: "Add payment=() to Permissions-Policy.".to_string(),
            });
        }
        findings
    }
}

pub struct PermissionsPolicyFullscreenDeepValidator;
impl Default for PermissionsPolicyFullscreenDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl PermissionsPolicyFullscreenDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for PermissionsPolicyFullscreenDeepValidator {
    fn name(&self) -> &str {
        "permissions-policy-fullscreen-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let pp = ctx
            .headers
            .iter()
            .find(|(k, _)| {
                k.eq_ignore_ascii_case("Permissions-Policy")
                    || k.eq_ignore_ascii_case("Feature-Policy")
            })
            .map(|(_, v)| v.as_str());
        if let Some(val) = pp {
            let lower = val.to_lowercase();
            if !lower.contains("fullscreen") {
                return findings;
            }
            if lower.trim() == "fullscreen" {
                return findings;
            }
            if lower.contains("fullscreen=()")
                || lower.contains("fullscreen ()")
                || lower.contains("fullscreen *")
                || lower.contains("fullscreen=*")
            {
                return findings;
            }
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Security,
                code: "PPFULLDP001".to_string(),
                title: "Fullscreen API access not explicitly restricted".to_string(),
                description:
                    "Permissions-Policy mentions fullscreen but doesn't explicitly deny it."
                        .to_string(),
                url: url.clone(),
                recommendation: "Consider adding fullscreen=() to Permissions-Policy.".to_string(),
            });
        }
        findings
    }
}

pub struct PermissionsPolicyXrVrDeepValidator;
impl Default for PermissionsPolicyXrVrDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl PermissionsPolicyXrVrDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for PermissionsPolicyXrVrDeepValidator {
    fn name(&self) -> &str {
        "permissions-policy-xr-vr-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let pp = ctx
            .headers
            .iter()
            .find(|(k, _)| {
                k.eq_ignore_ascii_case("Permissions-Policy")
                    || k.eq_ignore_ascii_case("Feature-Policy")
            })
            .map(|(_, v)| v.as_str());
        if let Some(val) = pp {
            let lower = val.to_lowercase();
            let directives: Vec<&str> = lower.split(',').map(|s| s.trim()).collect();
            for dir in &directives {
                let name = dir
                    .split('=')
                    .next()
                    .unwrap_or(dir)
                    .split('(')
                    .next()
                    .unwrap_or(dir)
                    .trim();
                if name == "xr-spatial-tracking"
                    || name == "vr"
                    || name == "xr-spatial-tracking *"
                    || name == "vr *"
                {
                    let has_value = dir.contains("=()")
                        || dir.contains(" ()")
                        || dir.contains("=*")
                        || dir.contains("= *");
                    let has_feature_policy_self =
                        dir.contains(" (self)") && !dir.contains("=(self)");
                    if has_value || has_feature_policy_self || name.ends_with(" *") {
                        continue;
                    }
                    let bare_keyword = dir.trim() == name;
                    if bare_keyword {
                        continue;
                    }
                    findings.push(Finding {
                        severity: Severity::Info,
                        category: IssueCategory::Security,
                        code: "PPXRDP001".to_string(),
                        title: format!("{name} API access not explicitly denied"),
                        description: format!(
                            "Permissions-Policy mentions {name} but doesn't explicitly deny it."
                        ),
                        url: url.clone(),
                        recommendation: format!("Add {name}=() to Permissions-Policy."),
                    });
                    break;
                }
            }
        }
        findings
    }
}

// =========================================================================
// V8 SEO Validators (Internal/External links deep validators)
// =========================================================================

pub struct CspScriptSrcSelfDeepValidator;
impl Default for CspScriptSrcSelfDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CspScriptSrcSelfDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CspScriptSrcSelfDeepValidator {
    fn name(&self) -> &str {
        "csp-script-src-self-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let csp = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Content-Security-Policy"))
            .map(|(_, v)| v.as_str())
            .unwrap_or("");
        if csp.is_empty() {
            return findings;
        }
        let mut found = false;
        for directive in csp.split(';') {
            let d = directive.trim();
            if d.starts_with("script-src") {
                found = true;
                if !d.contains("'self'") {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Security,
                        code: "CSPSS-V2001".to_string(),
                        title: "CSP script-src missing 'self' (deep)".to_string(),
                        description:
                            "script-src directive does not include 'self' in deep analysis."
                                .to_string(),
                        url: url.clone(),
                        recommendation: "Add 'self' to script-src directive.".to_string(),
                    });
                }
                if d.contains("'unsafe-inline'") {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Security,
                        code: "CSPSS-V2002".to_string(),
                        title: "CSP script-src allows unsafe-inline".to_string(),
                        description: "script-src directive includes 'unsafe-inline'.".to_string(),
                        url: url.clone(),
                        recommendation: "Remove 'unsafe-inline' and use nonces or hashes."
                            .to_string(),
                    });
                }
                if d.contains("'unsafe-eval'") {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Security,
                        code: "CSPSS-V2003".to_string(),
                        title: "CSP script-src allows unsafe-eval".to_string(),
                        description: "script-src directive includes 'unsafe-eval'.".to_string(),
                        url: url.clone(),
                        recommendation: "Remove 'unsafe-eval' from script-src directive."
                            .to_string(),
                    });
                }
                break;
            }
        }
        if !found {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Security,
                code: "CSPSS-V2001".to_string(),
                title: "CSP missing script-src directive".to_string(),
                description: "No script-src directive found in CSP.".to_string(),
                url: url.clone(),
                recommendation: "Add script-src directive to Content-Security-Policy.".to_string(),
            });
        }
        findings
    }
}

pub struct CspStyleSrcSelfDeepValidator;
impl Default for CspStyleSrcSelfDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CspStyleSrcSelfDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CspStyleSrcSelfDeepValidator {
    fn name(&self) -> &str {
        "csp-style-src-self-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let csp = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Content-Security-Policy"))
            .map(|(_, v)| v.as_str())
            .unwrap_or("");
        if csp.is_empty() {
            return findings;
        }
        let mut found = false;
        for directive in csp.split(';') {
            let d = directive.trim();
            if d.starts_with("style-src") {
                found = true;
                if !d.contains("'self'") {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Security,
                        code: "CSPSTY-V2001".to_string(),
                        title: "CSP style-src missing 'self' (deep)".to_string(),
                        description:
                            "style-src directive does not include 'self' in deep analysis."
                                .to_string(),
                        url: url.clone(),
                        recommendation: "Add 'self' to style-src directive.".to_string(),
                    });
                }
                if d.contains("'unsafe-inline'") {
                    findings.push(Finding {
                        severity: Severity::Info,
                        category: IssueCategory::Security,
                        code: "CSPSTY-V2002".to_string(),
                        title: "CSP style-src allows unsafe-inline".to_string(),
                        description: "style-src directive includes 'unsafe-inline'.".to_string(),
                        url: url.clone(),
                        recommendation: "Remove 'unsafe-inline' from style-src if possible."
                            .to_string(),
                    });
                }
                break;
            }
        }
        if !found {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Security,
                code: "CSPSTY-V2001".to_string(),
                title: "CSP missing style-src directive".to_string(),
                description: "No style-src directive found in CSP.".to_string(),
                url: url.clone(),
                recommendation: "Add style-src directive to Content-Security-Policy.".to_string(),
            });
        }
        findings
    }
}

pub struct CspConnectSrcDeepValidator;
impl Default for CspConnectSrcDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CspConnectSrcDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CspConnectSrcDeepValidator {
    fn name(&self) -> &str {
        "csp-connect-src-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let csp = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Content-Security-Policy"))
            .map(|(_, v)| v.as_str())
            .unwrap_or("");
        if csp.is_empty() {
            return findings;
        }
        for directive in csp.split(';') {
            let d = directive.trim();
            if d.starts_with("connect-src") {
                if d.contains("*") {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Security,
                        code: "CSPCON-V2001".to_string(),
                        title: "CSP connect-src allows wildcard (deep)".to_string(),
                        description:
                            "connect-src directive includes wildcard source in deep analysis."
                                .to_string(),
                        url: url.clone(),
                        recommendation: "Replace '*' with specific origins in connect-src."
                            .to_string(),
                    });
                }
                break;
            }
        }
        findings
    }
}

pub struct CspFontSrcDeepValidator;
impl Default for CspFontSrcDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CspFontSrcDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CspFontSrcDeepValidator {
    fn name(&self) -> &str {
        "csp-font-src-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let csp = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Content-Security-Policy"))
            .map(|(_, v)| v.as_str())
            .unwrap_or("");
        if csp.is_empty() {
            return findings;
        }
        for directive in csp.split(';') {
            let d = directive.trim();
            if d.starts_with("font-src") {
                if d.contains("*") {
                    findings.push(Finding {
                        severity: Severity::Info,
                        category: IssueCategory::Security,
                        code: "CSPFNT-V2001".to_string(),
                        title: "CSP font-src allows wildcard (deep)".to_string(),
                        description:
                            "font-src directive includes wildcard source in deep analysis."
                                .to_string(),
                        url: url.clone(),
                        recommendation: "Replace '*' with specific origins in font-src."
                            .to_string(),
                    });
                }
                break;
            }
        }
        findings
    }
}

pub struct CspObjectSrcNoneDeepDeepValidator;
impl Default for CspObjectSrcNoneDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CspObjectSrcNoneDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CspObjectSrcNoneDeepDeepValidator {
    fn name(&self) -> &str {
        "csp-object-src-none-deep-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let csp = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Content-Security-Policy"))
            .map(|(_, v)| v.as_str())
            .unwrap_or("");
        if csp.is_empty() {
            return findings;
        }
        let mut found = false;
        for directive in csp.split(';') {
            let d = directive.trim();
            if d.starts_with("object-src") {
                found = true;
                if !d.contains("'none'") {
                    findings.push(Finding {
                        severity: Severity::Critical,
                        category: IssueCategory::Security,
                        code: "CSPOBJ-V2001".to_string(),
                        title: "CSP object-src not set to 'none' (deep-deep)".to_string(),
                        description:
                            "object-src should be 'none' to block plugins in deep-deep analysis."
                                .to_string(),
                        url: url.clone(),
                        recommendation: "Set object-src to 'none'.".to_string(),
                    });
                }
                break;
            }
        }
        if !found {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Security,
                code: "CSPOBJ-V2001".to_string(),
                title: "CSP missing object-src directive".to_string(),
                description: "No object-src or default-src directive found.".to_string(),
                url: url.clone(),
                recommendation: "Add object-src 'none' to CSP.".to_string(),
            });
        }
        findings
    }
}

pub struct CspBaseUriSelfDeepDeepValidator;
impl Default for CspBaseUriSelfDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CspBaseUriSelfDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CspBaseUriSelfDeepDeepValidator {
    fn name(&self) -> &str {
        "csp-base-uri-self-deep-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let csp = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Content-Security-Policy"))
            .map(|(_, v)| v.as_str())
            .unwrap_or("");
        if csp.is_empty() {
            return findings;
        }
        let mut found = false;
        for directive in csp.split(';') {
            let d = directive.trim();
            if d.starts_with("base-uri") {
                found = true;
                if !d.contains("'self'") {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Security,
                        code: "CSPBASE-V2001".to_string(),
                        title: "CSP base-uri missing 'self' (deep-deep)".to_string(),
                        description:
                            "base-uri directive does not include 'self' in deep-deep analysis."
                                .to_string(),
                        url: url.clone(),
                        recommendation: "Add 'self' to base-uri directive.".to_string(),
                    });
                }
                break;
            }
        }
        if !found {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Security,
                code: "CSPBASE-V2001".to_string(),
                title: "CSP missing base-uri directive".to_string(),
                description: "No base-uri directive found. An attacker could inject a base tag."
                    .to_string(),
                url: url.clone(),
                recommendation: "Add base-uri 'self' to CSP.".to_string(),
            });
        }
        findings
    }
}

pub struct CspFormActionSelfDeepDeepValidator;
impl Default for CspFormActionSelfDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CspFormActionSelfDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CspFormActionSelfDeepDeepValidator {
    fn name(&self) -> &str {
        "csp-form-action-self-deep-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let csp = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Content-Security-Policy"))
            .map(|(_, v)| v.as_str())
            .unwrap_or("");
        if csp.is_empty() {
            return findings;
        }
        let mut found = false;
        for directive in csp.split(';') {
            let d = directive.trim();
            if d.starts_with("form-action") {
                found = true;
                if !d.contains("'self'") {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Security,
                        code: "CSPFORM-V2001".to_string(),
                        title: "CSP form-action missing 'self' (deep-deep)".to_string(),
                        description:
                            "form-action directive does not include 'self' in deep-deep analysis."
                                .to_string(),
                        url: url.clone(),
                        recommendation: "Add 'self' to form-action directive.".to_string(),
                    });
                }
                break;
            }
        }
        if !found {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Security,
                code: "CSPFORM-V2001".to_string(),
                title: "CSP missing form-action directive".to_string(),
                description: "No form-action directive found.".to_string(),
                url: url.clone(),
                recommendation: "Add form-action 'self' to CSP.".to_string(),
            });
        }
        findings
    }
}

pub struct HstsPreloadReadyDeepDeepValidator;
impl Default for HstsPreloadReadyDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl HstsPreloadReadyDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for HstsPreloadReadyDeepDeepValidator {
    fn name(&self) -> &str {
        "hsts-preload-ready-deep-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(val) = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Strict-Transport-Security"))
            .map(|(_, v)| v.as_str())
        {
            let lower = val.to_lowercase();
            let has_isd = lower.contains("includesubdomains");
            let has_preload = lower.contains("preload");
            let max_age = lower
                .find("max-age=")
                .map(|pos| {
                    let after = &lower[pos + 8..];
                    after
                        .chars()
                        .take_while(|c| c.is_ascii_digit())
                        .collect::<String>()
                        .parse::<u64>()
                        .unwrap_or(0)
                })
                .unwrap_or(0);
            if !has_isd || !has_preload || max_age < 31536000 {
                let mut missing = Vec::new();
                if !has_isd {
                    missing.push("includeSubDomains");
                }
                if !has_preload {
                    missing.push("preload");
                }
                if max_age < 31536000 {
                    missing.push("max-age>=31536000");
                }
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Security,
                    code: "HSTSPR-V2001".to_string(),
                    title: "HSTS preload readiness incomplete (deep-deep)".to_string(),
                    description: format!("Missing: {}.", missing.join(", ")),
                    url: url.clone(),
                    recommendation: "Add includeSubDomains; preload; max-age=31536000.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct HstsMaxAgeDeepDeepValidator;
impl Default for HstsMaxAgeDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl HstsMaxAgeDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for HstsMaxAgeDeepDeepValidator {
    fn name(&self) -> &str {
        "hsts-max-age-deep-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(val) = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Strict-Transport-Security"))
            .map(|(_, v)| v.as_str())
        {
            let lower = val.to_lowercase();
            if let Some(pos) = lower.find("max-age=") {
                let after = &lower[pos + 8..];
                if let Ok(age) = after
                    .chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect::<String>()
                    .parse::<u64>()
                {
                    if age < 31536000 {
                        findings.push(Finding {
                            severity: Severity::Warning,
                            category: IssueCategory::Security,
                            code: "HSTSMAX-V2001".to_string(),
                            title: "HSTS max-age below 1 year (deep-deep)".to_string(),
                            description: format!(
                                "max-age is {age}, recommended >= 31536000 (1 year)."
                            ),
                            url: url.clone(),
                            recommendation: "Set max-age to at least 31536000.".to_string(),
                        });
                    }
                }
            }
        }
        findings
    }
}

pub struct CookieSecureDeepDeepDeepValidator;
impl Default for CookieSecureDeepDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CookieSecureDeepDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CookieSecureDeepDeepDeepValidator {
    fn name(&self) -> &str {
        "cookie-secure-deep-deep-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for (k, v) in ctx.headers {
            if !k.eq_ignore_ascii_case("set-cookie") {
                continue;
            }
            let lower = v.to_lowercase();
            let name = v.split('=').next().unwrap_or("cookie").trim().to_string();
            let is_session = lower.contains("session");
            if !lower.contains("secure") && !is_session {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "COOKIESEC-V2001".to_string(),
                    title: format!("Cookie '{name}' missing Secure flag (deep-deep-deep)"),
                    description:
                        "Non-session cookie transmitted without Secure flag in deep analysis."
                            .to_string(),
                    url: url.clone(),
                    recommendation: "Add Secure flag to cookie.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct CookieHttpOnlyDeepDeepDeepValidator;
impl Default for CookieHttpOnlyDeepDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CookieHttpOnlyDeepDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CookieHttpOnlyDeepDeepDeepValidator {
    fn name(&self) -> &str {
        "cookie-httponly-deep-deep-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for (k, v) in ctx.headers {
            if !k.eq_ignore_ascii_case("set-cookie") {
                continue;
            }
            let lower = v.to_lowercase();
            let name = v.split('=').next().unwrap_or("cookie").trim().to_string();
            if !lower.contains("httponly") {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "COOKIEHTTP-V2001".to_string(),
                    title: format!("Cookie '{name}' missing HttpOnly flag (deep-deep-deep)"),
                    description:
                        "Cookie accessible to JavaScript without HttpOnly in deep analysis."
                            .to_string(),
                    url: url.clone(),
                    recommendation: "Add HttpOnly flag to cookie.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct CookieSameSiteDeepDeepDeepValidator;
impl Default for CookieSameSiteDeepDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CookieSameSiteDeepDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CookieSameSiteDeepDeepDeepValidator {
    fn name(&self) -> &str {
        "cookie-samesite-deep-deep-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for (k, v) in ctx.headers {
            if !k.eq_ignore_ascii_case("set-cookie") {
                continue;
            }
            let lower = v.to_lowercase();
            let name = v.split('=').next().unwrap_or("cookie").trim().to_string();
            if !lower.contains("samesite") {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "COOKIESAME-V2001".to_string(),
                    title: format!("Cookie '{name}' missing SameSite (deep-deep-deep)"),
                    description: "Without SameSite, cookie is vulnerable to CSRF in deep analysis."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add SameSite=Strict or SameSite=Lax.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct MixedContentIframeDeepDeepValidator;
impl Default for MixedContentIframeDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl MixedContentIframeDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for MixedContentIframeDeepDeepValidator {
    fn name(&self) -> &str {
        "mixed-content-iframe-deep-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if !url.starts_with("https://") {
            return findings;
        }
        if let Some(body) = ctx.body {
            let lower = body.to_lowercase();
            let http_iframe_count = lower.matches("<iframe").count();
            let http_in_iframes = lower.matches("http://").count();
            if http_iframe_count > 0 && http_in_iframes > 0 {
                findings.push(Finding { severity: Severity::Critical, category: IssueCategory::Security, code: "MIXIFRAME-V2001".to_string(), title: "Mixed content in iframes (deep-deep)".to_string(), description: format!("Found {http_iframe_count} iframe(s) with HTTP resources on HTTPS page in deep analysis."), url: url.clone(), recommendation: "Change all iframe sources to HTTPS.".to_string() });
            }
        }
        findings
    }
}

pub struct CorsWildcardDeepDeepValidator;
impl Default for CorsWildcardDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CorsWildcardDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CorsWildcardDeepDeepValidator {
    fn name(&self) -> &str {
        "cors-wildcard-deep-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(val) = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Access-Control-Allow-Origin"))
            .map(|(_, v)| v.as_str())
        {
            if val == "*" {
                let has_credentials = ctx.headers.iter().any(|(k, v)| {
                    k.eq_ignore_ascii_case("Access-Control-Allow-Credentials")
                        && v.eq_ignore_ascii_case("true")
                });
                if has_credentials {
                    findings.push(Finding {
                        severity: Severity::Critical,
                        category: IssueCategory::Security,
                        code: "CORSWILD-V2001".to_string(),
                        title: "CORS wildcard with credentials (deep-deep)".to_string(),
                        description:
                            "Access-Control-Allow-Origin is '*' with credentials in deep analysis."
                                .to_string(),
                        url: url.clone(),
                        recommendation:
                            "Use a specific origin instead of '*' when credentials are allowed."
                                .to_string(),
                    });
                } else {
                    findings.push(Finding {
                        severity: Severity::Info,
                        category: IssueCategory::Security,
                        code: "CORSWILD-V2001".to_string(),
                        title: "CORS allows all origins (deep-deep)".to_string(),
                        description: "Access-Control-Allow-Origin is set to '*' in deep analysis."
                            .to_string(),
                        url: url.clone(),
                        recommendation: "Consider restricting CORS to specific origins."
                            .to_string(),
                    });
                }
            }
        }
        findings
    }
}

pub struct ReferrerPolicyStrictDeepDeepValidator;
impl Default for ReferrerPolicyStrictDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl ReferrerPolicyStrictDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for ReferrerPolicyStrictDeepDeepValidator {
    fn name(&self) -> &str {
        "referrer-policy-strict-deep-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(val) = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Referrer-Policy"))
            .map(|(_, v)| v.as_str())
        {
            let lower = val.to_lowercase();
            let is_strict = lower == "no-referrer"
                || lower == "same-origin"
                || lower == "strict-origin"
                || lower == "strict-origin-when-cross-origin"
                || lower == "no-referrer-when-downgrade";
            if !is_strict {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Security,
                    code: "RPSTRICT-V2001".to_string(),
                    title: "Referrer-Policy not using strict policy (deep-deep)".to_string(),
                    description: format!(
                        "Current policy is '{val}'. Consider a stricter policy in deep analysis."
                    ),
                    url: url.clone(),
                    recommendation:
                        "Use no-referrer, same-origin, or strict-origin-when-cross-origin."
                            .to_string(),
                });
            }
        }
        findings
    }
}

pub struct XFrameOptionsDeepDeepValidator;
impl Default for XFrameOptionsDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl XFrameOptionsDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for XFrameOptionsDeepDeepValidator {
    fn name(&self) -> &str {
        "x-frame-options-deep-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let has_csp_frame = ctx.headers.iter().any(|(k, v)| {
            k.eq_ignore_ascii_case("Content-Security-Policy") && v.contains("frame-ancestors")
        });
        if has_csp_frame {
            return findings;
        }
        if let Some(val) = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("X-Frame-Options"))
            .map(|(_, v)| v.as_str())
        {
            let lower = val.to_lowercase();
            if lower != "deny" && lower != "sameorigin" {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "XFODEEP-V2001".to_string(),
                    title: "X-Frame-Options has invalid value (deep-deep)".to_string(),
                    description: format!("Value is '{val}'. Must be DENY or SAMEORIGIN."),
                    url: url.clone(),
                    recommendation: "Set X-Frame-Options to DENY or SAMEORIGIN.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct PermissionsPolicyDeepDeepValidator;
impl Default for PermissionsPolicyDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl PermissionsPolicyDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for PermissionsPolicyDeepDeepValidator {
    fn name(&self) -> &str {
        "permissions-policy-deep-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if !ctx.headers.iter().any(|(k, _)| {
            k.eq_ignore_ascii_case("Permissions-Policy") || k.eq_ignore_ascii_case("Feature-Policy")
        }) {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Security,
                code: "PERMP-V2001-DEEP-DEEP".to_string(),
                title: "Missing Permissions-Policy header (deep-deep)".to_string(),
                description:
                    "No Permissions-Policy or Feature-Policy header found in deep analysis."
                        .to_string(),
                url: url.clone(),
                recommendation: "Add a Permissions-Policy header to restrict browser features."
                    .to_string(),
            });
        }
        findings
    }
}

pub struct CrossOriginIsolationDeepDeepValidator;
impl Default for CrossOriginIsolationDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CrossOriginIsolationDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for CrossOriginIsolationDeepDeepValidator {
    fn name(&self) -> &str {
        "cross-origin-isolation-deep-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let has_coep = ctx
            .headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("Cross-Origin-Embedder-Policy"));
        let has_coop = ctx
            .headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("Cross-Origin-Opener-Policy"));
        if !has_coep {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Security,
                code: "COISO-V2001-DEEP-DEEP".to_string(),
                title: "Missing Cross-Origin-Embedder-Policy (deep-deep)".to_string(),
                description: "No COEP header found in deep analysis.".to_string(),
                url: url.clone(),
                recommendation: "Add Cross-Origin-Embedder-Policy: require-corp.".to_string(),
            });
        }
        if !has_coop {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Security,
                code: "COISO-V2002".to_string(),
                title: "Missing Cross-Origin-Opener-Policy (deep-deep)".to_string(),
                description: "No COOP header found in deep analysis.".to_string(),
                url: url.clone(),
                recommendation: "Add Cross-Origin-Opener-Policy: same-origin.".to_string(),
            });
        }
        findings
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::meta::MetaTags;

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

    fn make_ctx_headers<'a>(
        page: &'a crate::parser::ParsedPage,
        headers: &'a [(String, String)],
    ) -> AnalysisContext<'a> {
        AnalysisContext {
            page,
            body: None,
            status_code: Some(200),
            headers,
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
    fn test_csp_v2_empty() {
        assert!(CspDirectiveAnalyzerV2::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_cors_v2() {
        assert!(CorsPolicyAnalyzerV2::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_cookie_v2() {
        assert!(CookieSecurityFlagAnalyzerV2::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_mixed_content_v2() {
        assert!(MixedContentDetectionAnalyzerV2::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_hsts_v2() {
        assert!(HstsPreloadReadinessAnalyzerV2::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_xcto_v2() {
        let f = XContentTypeOptionsDeepAnalyzerV2::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None));
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].code, "XCTO-V2001-DEEP");
    }
    #[test]
    fn test_rp_v2() {
        let f = ReferrerPolicyDeepAnalyzerV2::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None));
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].code, "RPDEEP-V2001");
    }
    #[test]
    fn test_xfo_v2() {
        let f = XFrameOptionsDeepAnalyzerV2::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None));
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].code, "XFODEEP-V2001");
    }
    #[test]
    fn test_pp_v2() {
        let f = PermissionsPolicyDeepAnalyzerV2::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None));
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].code, "PERMP-V2001");
    }
    #[test]
    fn test_coi_v2() {
        let f = CrossOriginIsolationDeepAnalyzerV2::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None));
        assert_eq!(f.len(), 2);
    }
    #[test]
    fn test_csp_script_src_empty() {
        assert!(CspScriptSrcValidator::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_csp_script_src_unsafe_inline() {
        let p = make_page("https://example.com");
        let headers = vec![(
            "Content-Security-Policy".into(),
            "script-src 'unsafe-inline'".into(),
        )];
        let ctx = AnalysisContext {
            page: &p,
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let f = CspScriptSrcValidator::new().analyze(&ctx);
        assert!(f.iter().any(|x| x.code == "CSPSSRC-V5001"));
    }
    #[test]
    fn test_csp_script_src_wildcard() {
        let p = make_page("https://example.com");
        let headers = vec![("Content-Security-Policy".into(), "script-src *".into())];
        let ctx = AnalysisContext {
            page: &p,
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let f = CspScriptSrcValidator::new().analyze(&ctx);
        assert!(f.iter().any(|x| x.code == "CSPSSRC-V5003"));
    }
    #[test]
    fn test_csp_script_src_missing() {
        let p = make_page("https://example.com");
        let headers = vec![(
            "Content-Security-Policy".into(),
            "default-src 'self'".into(),
        )];
        let ctx = AnalysisContext {
            page: &p,
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let f = CspScriptSrcValidator::new().analyze(&ctx);
        assert!(f.iter().any(|x| x.code == "CSPSSRC-V5004"));
    }
    #[test]
    fn test_csp_style_src_empty() {
        assert!(CspStyleSrcValidator::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_csp_frame_ancestors_missing() {
        let p = make_page("https://example.com");
        let headers = vec![("Content-Security-Policy".into(), "script-src 'self'".into())];
        let ctx = AnalysisContext {
            page: &p,
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let f = CspFrameAncestorsValidator::new().analyze(&ctx);
        assert!(!f.is_empty());
    }
    #[test]
    fn test_hsts_max_age_missing() {
        let p = make_page("https://example.com");
        let f = HstsMaxAgeValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_hsts_max_age_low() {
        let p = make_page("https://example.com");
        let headers = vec![("Strict-Transport-Security".into(), "max-age=100".into())];
        let ctx = AnalysisContext {
            page: &p,
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let f = HstsMaxAgeValidator::new().analyze(&ctx);
        assert!(f.iter().any(|x| x.code == "HSTSMAX-V5002"));
    }
    #[test]
    fn test_hsts_max_age_ok() {
        let p = make_page("https://example.com");
        let headers = vec![(
            "Strict-Transport-Security".into(),
            "max-age=31536000".into(),
        )];
        let ctx = AnalysisContext {
            page: &p,
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        assert!(HstsMaxAgeValidator::new().analyze(&ctx).is_empty());
    }
    #[test]
    fn test_hsts_include_subdomains() {
        let p = make_page("https://example.com");
        let headers = vec![(
            "Strict-Transport-Security".into(),
            "max-age=31536000".into(),
        )];
        let ctx = AnalysisContext {
            page: &p,
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let f = HstsIncludeSubDomainsValidator::new().analyze(&ctx);
        assert!(!f.is_empty());
    }
    #[test]
    fn test_hsts_preload_missing() {
        let p = make_page("https://example.com");
        let headers = vec![(
            "Strict-Transport-Security".into(),
            "max-age=31536000; includeSubDomains".into(),
        )];
        let ctx = AnalysisContext {
            page: &p,
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let f = HstsPreloadValidatorV5::new().analyze(&ctx);
        assert!(!f.is_empty());
    }
    #[test]
    fn test_xcto_v5_missing() {
        let f = XContentTypeOptionsValidatorV5::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_xcto_v5_ok() {
        let p = make_page("https://example.com");
        let headers = vec![("X-Content-Type-Options".into(), "nosniff".into())];
        let ctx = AnalysisContext {
            page: &p,
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        assert!(XContentTypeOptionsValidatorV5::new()
            .analyze(&ctx)
            .is_empty());
    }
    #[test]
    fn test_rp_v5_missing() {
        let f = ReferrerPolicyValidatorV5::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_rp_v5_unsafe_url() {
        let p = make_page("https://example.com");
        let headers = vec![("Referrer-Policy".into(), "unsafe-url".into())];
        let ctx = AnalysisContext {
            page: &p,
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let f = ReferrerPolicyValidatorV5::new().analyze(&ctx);
        assert!(f.iter().any(|x| x.code == "RP-V5002"));
    }
    #[test]
    fn test_xfo_v5_missing() {
        let f = XFrameOptionsValidatorV5::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_xfo_v5_invalid() {
        let p = make_page("https://example.com");
        let headers = vec![("X-Frame-Options".into(), "ALLOW".into())];
        let ctx = AnalysisContext {
            page: &p,
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let f = XFrameOptionsValidatorV5::new().analyze(&ctx);
        assert!(f.iter().any(|x| x.code == "XFO-V5002"));
    }
    #[test]
    fn test_pp_camera() {
        let p = make_page("https://example.com");
        let headers = vec![("Permissions-Policy".into(), "camera=(self)".into())];
        let ctx = AnalysisContext {
            page: &p,
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let f = PermissionsPolicyCameraValidator::new().analyze(&ctx);
        assert!(!f.is_empty());
    }
    #[test]
    fn test_pp_microphone() {
        let p = make_page("https://example.com");
        let headers = vec![("Permissions-Policy".into(), "microphone=(self)".into())];
        let ctx = AnalysisContext {
            page: &p,
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let f = PermissionsPolicyMicrophoneValidator::new().analyze(&ctx);
        assert!(!f.is_empty());
    }
    #[test]
    fn test_pp_geolocation() {
        let p = make_page("https://example.com");
        let headers = vec![("Permissions-Policy".into(), "geolocation=(self)".into())];
        let ctx = AnalysisContext {
            page: &p,
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let f = PermissionsPolicyGeolocationValidator::new().analyze(&ctx);
        assert!(!f.is_empty());
    }
    #[test]
    fn test_coep_missing() {
        let f = CoepValidator::new().analyze(&make_ctx(&make_page("https://example.com"), None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_coop_missing() {
        let f = CoopValidator::new().analyze(&make_ctx(&make_page("https://example.com"), None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_cookie_secure_v5() {
        let p = make_page("https://example.com");
        let headers = vec![("Set-Cookie".into(), "session=abc123; HttpOnly".into())];
        let ctx = AnalysisContext {
            page: &p,
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let f = CookieSecureFlagValidatorV5::new().analyze(&ctx);
        assert!(!f.is_empty());
    }
    #[test]
    fn test_cookie_httponly_v5() {
        let p = make_page("https://example.com");
        let headers = vec![("Set-Cookie".into(), "session=abc123; Secure".into())];
        let ctx = AnalysisContext {
            page: &p,
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let f = CookieHttpOnlyFlagValidatorV5::new().analyze(&ctx);
        assert!(!f.is_empty());
    }
    #[test]
    fn test_cookie_samesite() {
        let p = make_page("https://example.com");
        let headers = vec![(
            "Set-Cookie".into(),
            "session=abc123; Secure; HttpOnly".into(),
        )];
        let ctx = AnalysisContext {
            page: &p,
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let f = CookieSameSiteValidator::new().analyze(&ctx);
        assert!(!f.is_empty());
    }
    #[test]
    fn test_mixed_script_v5() {
        assert!(MixedContentScriptValidatorV5::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_mixed_script_v5_with_content() {
        let f = MixedContentScriptValidatorV5::new().analyze(&make_ctx(
            &make_page("https://example.com"),
            Some("<script src=\"http://evil.com/x.js\"></script>"),
        ));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_mixed_css_v5() {
        let f = MixedContentStylesheetValidator::new().analyze(&make_ctx(
            &make_page("https://example.com"),
            Some("<link href=\"http://evil.com/style.css\" rel=\"stylesheet\">"),
        ));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_sri_v5() {
        let mut p = make_page("https://example.com");
        p.scripts = vec![crate::parser::ScriptInfo {
            src: Some("https://cdn.com/lib.js".into()),
            r#async: false,
            defer: false,
            script_type: None,
            has_integrity: false,
            is_module: false,
        }];
        let f = SriValidator::new().analyze(&make_ctx(
            &p,
            Some("<script src=\"https://cdn.com/lib.js\"></script>"),
        ));
        assert!(!f.is_empty());
    }

    // ===== SEO V5 Tests =====
    #[test]
    fn test_csp_connect_src() {
        assert!(CspConnectSrcValidator::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_csp_connect_src_present() {
        let p = make_page("https://example.com");
        let headers = vec![(
            "Content-Security-Policy".into(),
            "connect-src 'self'".into(),
        )];
        let ctx = AnalysisContext {
            page: &p,
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        assert!(CspConnectSrcValidator::new().analyze(&ctx).is_empty());
    }
    #[test]
    fn test_csp_font_src() {
        assert!(CspFontSrcValidator::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_hsts_threshold_low() {
        let p = make_page("https://example.com");
        let headers = vec![("Strict-Transport-Security".into(), "max-age=100".into())];
        let ctx = AnalysisContext {
            page: &p,
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let f = HstsMaxAgeThresholdValidator::new().analyze(&ctx);
        assert!(!f.is_empty());
    }
    #[test]
    fn test_hsts_threshold_ok() {
        let p = make_page("https://example.com");
        let headers = vec![(
            "Strict-Transport-Security".into(),
            "max-age=31536000".into(),
        )];
        let ctx = AnalysisContext {
            page: &p,
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        assert!(HstsMaxAgeThresholdValidator::new().analyze(&ctx).is_empty());
    }
    #[test]
    fn test_hsts_preload_check() {
        let p = make_page("https://example.com");
        let headers = vec![(
            "Strict-Transport-Security".into(),
            "max-age=31536000; includeSubDomains; preload".into(),
        )];
        let ctx = AnalysisContext {
            page: &p,
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        assert!(HstsPreloadListCheckValidator::new()
            .analyze(&ctx)
            .is_empty());
    }
    #[test]
    fn test_cookie_secure_deep() {
        let p = make_page("https://example.com");
        let headers = vec![("Set-Cookie".into(), "session=abc".into())];
        let ctx = AnalysisContext {
            page: &p,
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let f = CookieSecureFlagDeepValidator::new().analyze(&ctx);
        assert!(!f.is_empty());
    }
    #[test]
    fn test_cookie_httponly_deep() {
        let p = make_page("https://example.com");
        let headers = vec![("Set-Cookie".into(), "session=abc; Secure".into())];
        let ctx = AnalysisContext {
            page: &p,
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let f = CookieHttpOnlyFlagDeepValidator::new().analyze(&ctx);
        assert!(!f.is_empty());
    }
    #[test]
    fn test_cookie_samesite_deep() {
        let p = make_page("https://example.com");
        let headers = vec![("Set-Cookie".into(), "session=abc; Secure; HttpOnly".into())];
        let ctx = AnalysisContext {
            page: &p,
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let f = CookieSameSiteDeepValidator::new().analyze(&ctx);
        assert!(!f.is_empty());
    }
    #[test]
    fn test_mixed_content_iframe() {
        assert!(MixedContentIframeValidator::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_cors_wildcard() {
        let p = make_page("https://example.com");
        let headers = vec![("Access-Control-Allow-Origin".into(), "*".into())];
        let ctx = AnalysisContext {
            page: &p,
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let f = CorsWildcardValidator::new().analyze(&ctx);
        assert!(!f.is_empty());
    }
    #[test]
    fn test_cors_missing() {
        let f = CorsMissingHeaderValidator::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_rp_strict() {
        let p = make_page("https://example.com");
        let headers = vec![("Referrer-Policy".into(), "unsafe-url".into())];
        let ctx = AnalysisContext {
            page: &p,
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let f = ReferrerPolicyStrictValidator::new().analyze(&ctx);
        assert!(!f.is_empty());
    }
    #[test]
    fn test_xfo_missing() {
        let f = XFrameOptionsMissingValidator::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_pp_payment() {
        assert!(PermissionsPolicyPaymentValidator::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_pp_fullscreen() {
        assert!(PermissionsPolicyFullscreenValidator::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_pp_xr_vr() {
        assert!(PermissionsPolicyXrVrValidator::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_coep_require_corp() {
        assert!(CoepRequireCorpValidator::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_coop_same_origin() {
        assert!(CoopSameOriginValidator::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_csp_obj_src() {
        assert!(CspObjectSrcNoneValidator::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_csp_base_uri() {
        assert!(CspBaseUriSelfValidator::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_csp_form_action() {
        assert!(CspFormActionSelfValidator::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }

    // ===== V6 SEO Validators Tests =====
    #[test]
    fn test_csp_ss_self_missing() {
        let headers = vec![("Content-Security-Policy".into(), "script-src 'none'".into())];
        let ctx = AnalysisContext {
            page: &make_page("https://example.com"),
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let f = CspScriptSrcSelfValidator::new().analyze(&ctx);
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "CSPSS001");
    }
    #[test]
    fn test_csp_ss_self_ok() {
        let headers = vec![("Content-Security-Policy".into(), "script-src 'self'".into())];
        let ctx = AnalysisContext {
            page: &make_page("https://example.com"),
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        assert!(CspScriptSrcSelfValidator::new().analyze(&ctx).is_empty());
    }
    #[test]
    fn test_csp_sty_self_missing() {
        let headers = vec![("Content-Security-Policy".into(), "style-src 'none'".into())];
        let ctx = AnalysisContext {
            page: &make_page("https://example.com"),
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let f = CspStyleSrcSelfValidator::new().analyze(&ctx);
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "CSPSTY001");
    }
    #[test]
    fn test_csp_sty_self_ok() {
        let headers = vec![("Content-Security-Policy".into(), "style-src 'self'".into())];
        let ctx = AnalysisContext {
            page: &make_page("https://example.com"),
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        assert!(CspStyleSrcSelfValidator::new().analyze(&ctx).is_empty());
    }
    #[test]
    fn test_csp_connect_wildcard() {
        let headers = vec![("Content-Security-Policy".into(), "connect-src *".into())];
        let ctx = AnalysisContext {
            page: &make_page("https://example.com"),
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let f = CspConnectSrcAnalysisValidator::new().analyze(&ctx);
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "CSPCON001");
    }
    #[test]
    fn test_csp_connect_ok() {
        let headers = vec![(
            "Content-Security-Policy".into(),
            "connect-src https://api.example.com".into(),
        )];
        let ctx = AnalysisContext {
            page: &make_page("https://example.com"),
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        assert!(CspConnectSrcAnalysisValidator::new()
            .analyze(&ctx)
            .is_empty());
    }
    #[test]
    fn test_csp_font_wildcard() {
        let headers = vec![("Content-Security-Policy".into(), "font-src *".into())];
        let ctx = AnalysisContext {
            page: &make_page("https://example.com"),
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let f = CspFontSrcAnalysisValidator::new().analyze(&ctx);
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "CSPFNT001");
    }
    #[test]
    fn test_csp_font_ok() {
        let headers = vec![(
            "Content-Security-Policy".into(),
            "font-src https://fonts.example.com".into(),
        )];
        let ctx = AnalysisContext {
            page: &make_page("https://example.com"),
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        assert!(CspFontSrcAnalysisValidator::new().analyze(&ctx).is_empty());
    }
    #[test]
    fn test_csp_obj_not_none() {
        let headers = vec![("Content-Security-Policy".into(), "object-src 'self'".into())];
        let ctx = AnalysisContext {
            page: &make_page("https://example.com"),
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let f = CspObjectSrcNoneDeepValidator::new().analyze(&ctx);
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "CSPOBJ001");
    }
    #[test]
    fn test_csp_obj_none_ok() {
        let headers = vec![("Content-Security-Policy".into(), "object-src 'none'".into())];
        let ctx = AnalysisContext {
            page: &make_page("https://example.com"),
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        assert!(CspObjectSrcNoneDeepValidator::new()
            .analyze(&ctx)
            .is_empty());
    }
    #[test]
    fn test_csp_base_uri_missing() {
        let headers = vec![("Content-Security-Policy".into(), "script-src 'self'".into())];
        let ctx = AnalysisContext {
            page: &make_page("https://example.com"),
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let f = CspBaseUriSelfDeepValidator::new().analyze(&ctx);
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "CSPBASE001");
    }
    #[test]
    fn test_csp_base_uri_ok() {
        let headers = vec![("Content-Security-Policy".into(), "base-uri 'self'".into())];
        let ctx = AnalysisContext {
            page: &make_page("https://example.com"),
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        assert!(CspBaseUriSelfDeepValidator::new().analyze(&ctx).is_empty());
    }
    #[test]
    fn test_csp_form_action_missing() {
        let headers = vec![("Content-Security-Policy".into(), "script-src 'self'".into())];
        let ctx = AnalysisContext {
            page: &make_page("https://example.com"),
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let f = CspFormActionSelfDeepValidator::new().analyze(&ctx);
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "CSPFORM001");
    }
    #[test]
    fn test_csp_form_action_ok() {
        let headers = vec![(
            "Content-Security-Policy".into(),
            "form-action 'self'".into(),
        )];
        let ctx = AnalysisContext {
            page: &make_page("https://example.com"),
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        assert!(CspFormActionSelfDeepValidator::new()
            .analyze(&ctx)
            .is_empty());
    }
    #[test]
    fn test_hsts_preload_ready_incomplete() {
        let headers = vec![(
            "Strict-Transport-Security".into(),
            "max-age=31536000".into(),
        )];
        let ctx = AnalysisContext {
            page: &make_page("https://example.com"),
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let f = HstsPreloadReadyDeepValidator::new().analyze(&ctx);
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "HSTSPR001");
    }
    #[test]
    fn test_hsts_preload_ready_ok() {
        let headers = vec![(
            "Strict-Transport-Security".into(),
            "max-age=31536000; includeSubDomains; preload".into(),
        )];
        let ctx = AnalysisContext {
            page: &make_page("https://example.com"),
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        assert!(HstsPreloadReadyDeepValidator::new()
            .analyze(&ctx)
            .is_empty());
    }
    #[test]
    fn test_hsts_max_age_deep_low() {
        let headers = vec![("Strict-Transport-Security".into(), "max-age=86400".into())];
        let ctx = AnalysisContext {
            page: &make_page("https://example.com"),
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let f = HstsMaxAgeDeepValidator::new().analyze(&ctx);
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "HSTSMAX001");
    }
    #[test]
    fn test_hsts_max_age_deep_ok() {
        let headers = vec![(
            "Strict-Transport-Security".into(),
            "max-age=31536000".into(),
        )];
        let ctx = AnalysisContext {
            page: &make_page("https://example.com"),
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        assert!(HstsMaxAgeDeepValidator::new().analyze(&ctx).is_empty());
    }
    #[test]
    fn test_cookie_secure_deep_missing() {
        let headers = vec![("Set-Cookie".into(), "token=abc123; HttpOnly".into())];
        let ctx = AnalysisContext {
            page: &make_page("https://example.com"),
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let f = CookieSecureDeepDeepValidator::new().analyze(&ctx);
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "COOKIESEC001");
    }
    #[test]
    fn test_cookie_secure_deep_ok() {
        let headers = vec![("Set-Cookie".into(), "token=abc123; Secure; HttpOnly".into())];
        let ctx = AnalysisContext {
            page: &make_page("https://example.com"),
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        assert!(CookieSecureDeepDeepValidator::new()
            .analyze(&ctx)
            .is_empty());
    }
    #[test]
    fn test_cookie_httponly_deep_missing() {
        let headers = vec![("Set-Cookie".into(), "token=abc123; Secure".into())];
        let ctx = AnalysisContext {
            page: &make_page("https://example.com"),
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let f = CookieHttpOnlyDeepDeepValidator::new().analyze(&ctx);
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "COOKIEHTTP001");
    }
    #[test]
    fn test_cookie_httponly_deep_ok() {
        let headers = vec![("Set-Cookie".into(), "token=abc123; Secure; HttpOnly".into())];
        let ctx = AnalysisContext {
            page: &make_page("https://example.com"),
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        assert!(CookieHttpOnlyDeepDeepValidator::new()
            .analyze(&ctx)
            .is_empty());
    }
    #[test]
    fn test_cookie_samesite_deep_missing() {
        let headers = vec![("Set-Cookie".into(), "token=abc123; Secure; HttpOnly".into())];
        let ctx = AnalysisContext {
            page: &make_page("https://example.com"),
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let f = CookieSameSiteDeepDeepValidator::new().analyze(&ctx);
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "COOKIESAME001");
    }
    #[test]
    fn test_cookie_samesite_deep_ok() {
        let headers = vec![(
            "Set-Cookie".into(),
            "token=abc123; Secure; HttpOnly; SameSite=Strict".into(),
        )];
        let ctx = AnalysisContext {
            page: &make_page("https://example.com"),
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        assert!(CookieSameSiteDeepDeepValidator::new()
            .analyze(&ctx)
            .is_empty());
    }
    #[test]
    fn test_mixed_iframe_deep() {
        let p = make_page("https://example.com");
        let body = "<html><body><iframe src=\"http://evil.com/frame\"></iframe></body></html>";
        let ctx = AnalysisContext {
            page: &p,
            body: Some(body),
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
        };
        let f = MixedContentIframeDeepValidator::new().analyze(&ctx);
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "MIXIFRAME001");
    }
    #[test]
    fn test_mixed_iframe_deep_ok() {
        let p = make_page("https://example.com");
        let body = "<html><body><iframe src=\"https://safe.com/frame\"></iframe></body></html>";
        let ctx = AnalysisContext {
            page: &p,
            body: Some(body),
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
        };
        assert!(MixedContentIframeDeepValidator::new()
            .analyze(&ctx)
            .is_empty());
    }
    #[test]
    fn test_cors_wildcard_deep() {
        let headers = vec![("Access-Control-Allow-Origin".into(), "*".into())];
        let ctx = AnalysisContext {
            page: &make_page("https://example.com"),
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let f = CorsWildcardDeepValidator::new().analyze(&ctx);
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "CORSWILD001");
    }
    #[test]
    fn test_cors_wildcard_creds() {
        let headers = vec![
            ("Access-Control-Allow-Origin".into(), "*".into()),
            ("Access-Control-Allow-Credentials".into(), "true".into()),
        ];
        let ctx = AnalysisContext {
            page: &make_page("https://example.com"),
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let f = CorsWildcardDeepValidator::new().analyze(&ctx);
        assert!(!f.is_empty());
        assert_eq!(f[0].severity, Severity::Critical);
    }
    #[test]
    fn test_cors_specific_ok() {
        let headers = vec![(
            "Access-Control-Allow-Origin".into(),
            "https://example.com".into(),
        )];
        let ctx = AnalysisContext {
            page: &make_page("https://example.com"),
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        assert!(CorsWildcardDeepValidator::new().analyze(&ctx).is_empty());
    }
    #[test]
    fn test_rp_strict_unsafe() {
        let headers = vec![("Referrer-Policy".into(), "unsafe-url".into())];
        let ctx = AnalysisContext {
            page: &make_page("https://example.com"),
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let f = ReferrerPolicyStrictDeepValidator::new().analyze(&ctx);
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "RPSTRICT001");
    }
    #[test]
    fn test_rp_strict_ok() {
        let headers = vec![(
            "Referrer-Policy".into(),
            "strict-origin-when-cross-origin".into(),
        )];
        let ctx = AnalysisContext {
            page: &make_page("https://example.com"),
            body: None,
            status_code: Some(200),
            headers: &headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        assert!(ReferrerPolicyStrictDeepValidator::new()
            .analyze(&ctx)
            .is_empty());
    }

    // ===== V7 SEO Validators Tests =====
    #[test]
    fn test_pp_payment_missing() {
        let p = make_page("https://example.com");
        assert!(PermissionsPolicyPaymentDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_pp_payment_explicit_deny() {
        let p = make_page("https://example.com");
        let h = vec![("Permissions-Policy".into(), "payment=()".into())];
        assert!(PermissionsPolicyPaymentDeepValidator::new()
            .analyze(&make_ctx_headers(&p, &h))
            .is_empty());
    }
    #[test]
    fn test_pp_payment_not_denied() {
        let p = make_page("https://example.com");
        let h = vec![("Permissions-Policy".into(), "payment=(self)".into())];
        let f = PermissionsPolicyPaymentDeepValidator::new().analyze(&make_ctx_headers(&p, &h));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "PPAYDP001");
    }
    #[test]
    fn test_pp_payment_feature_policy() {
        let p = make_page("https://example.com");
        let h = vec![("Feature-Policy".into(), "payment (self)".into())];
        assert!(PermissionsPolicyPaymentDeepValidator::new()
            .analyze(&make_ctx_headers(&p, &h))
            .is_empty());
    }
    #[test]
    fn test_pp_payment_empty_header() {
        let p = make_page("https://example.com");
        let h = vec![("Permissions-Policy".into(), "".into())];
        assert!(PermissionsPolicyPaymentDeepValidator::new()
            .analyze(&make_ctx_headers(&p, &h))
            .is_empty());
    }
    #[test]
    fn test_pp_payment_with_other_policies() {
        let p = make_page("https://example.com");
        let h = vec![(
            "Permissions-Policy".into(),
            "camera=(), microphone=(), payment=(self)".into(),
        )];
        assert!(!PermissionsPolicyPaymentDeepValidator::new()
            .analyze(&make_ctx_headers(&p, &h))
            .is_empty());
    }
    #[test]
    fn test_pp_payment_case_insensitive() {
        let p = make_page("https://example.com");
        let h = vec![("permissions-policy".into(), "Payment=(self)".into())];
        assert!(!PermissionsPolicyPaymentDeepValidator::new()
            .analyze(&make_ctx_headers(&p, &h))
            .is_empty());
    }
    #[test]
    fn test_pp_payment_no_value() {
        let p = make_page("https://example.com");
        let h = vec![("Permissions-Policy".into(), "payment".into())];
        assert!(PermissionsPolicyPaymentDeepValidator::new()
            .analyze(&make_ctx_headers(&p, &h))
            .is_empty());
    }
    #[test]
    fn test_pp_payment_multiple_directives() {
        let p = make_page("https://example.com");
        let h = vec![(
            "Permissions-Policy".into(),
            "camera=(), payment=(self), microphone=()".into(),
        )];
        assert!(!PermissionsPolicyPaymentDeepValidator::new()
            .analyze(&make_ctx_headers(&p, &h))
            .is_empty());
    }

    // ===== PermissionsPolicyFullscreenDeepValidator tests =====
    #[test]
    fn test_pp_fullscreen_missing() {
        let p = make_page("https://example.com");
        assert!(PermissionsPolicyFullscreenDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_pp_fullscreen_explicit_deny() {
        let p = make_page("https://example.com");
        let h = vec![("Permissions-Policy".into(), "fullscreen=()".into())];
        assert!(PermissionsPolicyFullscreenDeepValidator::new()
            .analyze(&make_ctx_headers(&p, &h))
            .is_empty());
    }
    #[test]
    fn test_pp_fullscreen_not_restricted() {
        let p = make_page("https://example.com");
        let h = vec![("Permissions-Policy".into(), "fullscreen=(self)".into())];
        let f = PermissionsPolicyFullscreenDeepValidator::new().analyze(&make_ctx_headers(&p, &h));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "PPFULLDP001");
    }
    #[test]
    fn test_pp_fullscreen_no_value() {
        let p = make_page("https://example.com");
        let h = vec![("Permissions-Policy".into(), "fullscreen".into())];
        assert!(PermissionsPolicyFullscreenDeepValidator::new()
            .analyze(&make_ctx_headers(&p, &h))
            .is_empty());
    }
    #[test]
    fn test_pp_fullscreen_empty_header() {
        let p = make_page("https://example.com");
        let h = vec![("Permissions-Policy".into(), "".into())];
        assert!(PermissionsPolicyFullscreenDeepValidator::new()
            .analyze(&make_ctx_headers(&p, &h))
            .is_empty());
    }
    #[test]
    fn test_pp_fullscreen_case_insensitive() {
        let p = make_page("https://example.com");
        let h = vec![("permissions-policy".into(), "Fullscreen=(self)".into())];
        assert!(!PermissionsPolicyFullscreenDeepValidator::new()
            .analyze(&make_ctx_headers(&p, &h))
            .is_empty());
    }
    #[test]
    fn test_pp_fullscreen_multiple_directives() {
        let p = make_page("https://example.com");
        let h = vec![(
            "Permissions-Policy".into(),
            "camera=(), fullscreen=(self), microphone=()".into(),
        )];
        assert!(!PermissionsPolicyFullscreenDeepValidator::new()
            .analyze(&make_ctx_headers(&p, &h))
            .is_empty());
    }
    #[test]
    fn test_pp_fullscreen_with_others() {
        let p = make_page("https://example.com");
        let h = vec![(
            "Permissions-Policy".into(),
            "camera=(), microphone=()".into(),
        )];
        assert!(PermissionsPolicyFullscreenDeepValidator::new()
            .analyze(&make_ctx_headers(&p, &h))
            .is_empty());
    }
    #[test]
    fn test_pp_fullscreen_wildcard() {
        let p = make_page("https://example.com");
        let h = vec![("Permissions-Policy".into(), "fullscreen *".into())];
        assert!(PermissionsPolicyFullscreenDeepValidator::new()
            .analyze(&make_ctx_headers(&p, &h))
            .is_empty());
    }

    // ===== PermissionsPolicyXrVrDeepValidator tests =====
    #[test]
    fn test_pp_xrvr_missing() {
        let p = make_page("https://example.com");
        assert!(PermissionsPolicyXrVrDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_pp_xrvr_explicit_deny() {
        let p = make_page("https://example.com");
        let h = vec![(
            "Permissions-Policy".into(),
            "xr-spatial-tracking=(), vr=()".into(),
        )];
        assert!(PermissionsPolicyXrVrDeepValidator::new()
            .analyze(&make_ctx_headers(&p, &h))
            .is_empty());
    }
    #[test]
    fn test_pp_xrvr_not_denied() {
        let p = make_page("https://example.com");
        let h = vec![(
            "Permissions-Policy".into(),
            "xr-spatial-tracking=(self), vr=(self)".into(),
        )];
        let f = PermissionsPolicyXrVrDeepValidator::new().analyze(&make_ctx_headers(&p, &h));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "PPXRDP001");
    }
    #[test]
    fn test_pp_xr_only() {
        let p = make_page("https://example.com");
        let h = vec![(
            "Permissions-Policy".into(),
            "xr-spatial-tracking=(self)".into(),
        )];
        assert!(!PermissionsPolicyXrVrDeepValidator::new()
            .analyze(&make_ctx_headers(&p, &h))
            .is_empty());
    }
    #[test]
    fn test_pp_vr_only() {
        let p = make_page("https://example.com");
        let h = vec![("Permissions-Policy".into(), "vr=(self)".into())];
        assert!(!PermissionsPolicyXrVrDeepValidator::new()
            .analyze(&make_ctx_headers(&p, &h))
            .is_empty());
    }
    #[test]
    fn test_pp_xrvr_empty_header() {
        let p = make_page("https://example.com");
        let h = vec![("Permissions-Policy".into(), "".into())];
        assert!(PermissionsPolicyXrVrDeepValidator::new()
            .analyze(&make_ctx_headers(&p, &h))
            .is_empty());
    }
    #[test]
    fn test_pp_xrvr_case_insensitive() {
        let p = make_page("https://example.com");
        let h = vec![(
            "permissions-policy".into(),
            "XR-Spatial-Tracking=(self), VR=(self)".into(),
        )];
        assert!(!PermissionsPolicyXrVrDeepValidator::new()
            .analyze(&make_ctx_headers(&p, &h))
            .is_empty());
    }
    #[test]
    fn test_pp_xrvr_no_value() {
        let p = make_page("https://example.com");
        let h = vec![(
            "Permissions-Policy".into(),
            "xr-spatial-tracking, vr".into(),
        )];
        assert!(PermissionsPolicyXrVrDeepValidator::new()
            .analyze(&make_ctx_headers(&p, &h))
            .is_empty());
    }
    #[test]
    fn test_pp_xrvr_feature_policy() {
        let p = make_page("https://example.com");
        let h = vec![("Feature-Policy".into(), "xr-spatial-tracking (self)".into())];
        assert!(PermissionsPolicyXrVrDeepValidator::new()
            .analyze(&make_ctx_headers(&p, &h))
            .is_empty());
    }
    #[test]
    fn test_pp_xrvr_multiple_directives() {
        let p = make_page("https://example.com");
        let h = vec![(
            "Permissions-Policy".into(),
            "camera=(), xr-spatial-tracking=(self), vr=(self)".into(),
        )];
        assert!(!PermissionsPolicyXrVrDeepValidator::new()
            .analyze(&make_ctx_headers(&p, &h))
            .is_empty());
    }

    // ===== InternalLinksDepthDeepValidator tests =====
    #[test]
    fn test_csp_ss_deep_v8_no_csp() {
        let p = make_page("https://example.com");
        assert!(CspScriptSrcSelfDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_csp_ss_deep_v8_missing_self() {
        let p = make_page("https://example.com");
        let h = vec![(
            "Content-Security-Policy".into(),
            "script-src 'unsafe-inline'".into(),
        )];
        let f = CspScriptSrcSelfDeepValidator::new().analyze(&make_ctx_headers(&p, &h));
        assert!(f.iter().any(|x| x.code == "CSPSS-V2001"));
    }
    #[test]
    fn test_csp_ss_deep_v8_has_self() {
        let p = make_page("https://example.com");
        let h = vec![("Content-Security-Policy".into(), "script-src 'self'".into())];
        assert!(CspScriptSrcSelfDeepValidator::new()
            .analyze(&make_ctx_headers(&p, &h))
            .is_empty());
    }
    #[test]
    fn test_csp_ss_deep_v8_unsafe_inline() {
        let p = make_page("https://example.com");
        let h = vec![(
            "Content-Security-Policy".into(),
            "script-src 'self' 'unsafe-inline'".into(),
        )];
        let f = CspScriptSrcSelfDeepValidator::new().analyze(&make_ctx_headers(&p, &h));
        assert!(f.iter().any(|x| x.code == "CSPSS-V2002"));
    }
    #[test]
    fn test_csp_ss_deep_v8_unsafe_eval() {
        let p = make_page("https://example.com");
        let h = vec![(
            "Content-Security-Policy".into(),
            "script-src 'self' 'unsafe-eval'".into(),
        )];
        let f = CspScriptSrcSelfDeepValidator::new().analyze(&make_ctx_headers(&p, &h));
        assert!(f.iter().any(|x| x.code == "CSPSS-V2003"));
    }
    #[test]
    fn test_csp_ss_deep_v8_no_directive() {
        let p = make_page("https://example.com");
        let h = vec![(
            "Content-Security-Policy".into(),
            "default-src 'self'".into(),
        )];
        let f = CspScriptSrcSelfDeepValidator::new().analyze(&make_ctx_headers(&p, &h));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_csp_sty_deep_v8_no_csp() {
        let p = make_page("https://example.com");
        assert!(CspStyleSrcSelfDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_csp_sty_deep_v8_missing_self() {
        let p = make_page("https://example.com");
        let h = vec![("Content-Security-Policy".into(), "style-src *".into())];
        let f = CspStyleSrcSelfDeepValidator::new().analyze(&make_ctx_headers(&p, &h));
        assert!(f.iter().any(|x| x.code == "CSPSTY-V2001"));
    }
    #[test]
    fn test_csp_sty_deep_v8_has_self() {
        let p = make_page("https://example.com");
        let h = vec![("Content-Security-Policy".into(), "style-src 'self'".into())];
        assert!(CspStyleSrcSelfDeepValidator::new()
            .analyze(&make_ctx_headers(&p, &h))
            .is_empty());
    }
    #[test]
    fn test_csp_con_deep_v8_no_csp() {
        let p = make_page("https://example.com");
        assert!(CspConnectSrcDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_csp_con_deep_v8_wildcard() {
        let p = make_page("https://example.com");
        let h = vec![("Content-Security-Policy".into(), "connect-src *".into())];
        let f = CspConnectSrcDeepValidator::new().analyze(&make_ctx_headers(&p, &h));
        assert!(f.iter().any(|x| x.code == "CSPCON-V2001"));
    }
    #[test]
    fn test_csp_con_deep_v8_no_directive() {
        let p = make_page("https://example.com");
        let h = vec![(
            "Content-Security-Policy".into(),
            "default-src 'self'".into(),
        )];
        assert!(CspConnectSrcDeepValidator::new()
            .analyze(&make_ctx_headers(&p, &h))
            .is_empty());
    }
    #[test]
    fn test_csp_fnt_deep_v8_no_csp() {
        let p = make_page("https://example.com");
        assert!(CspFontSrcDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_csp_fnt_deep_v8_wildcard() {
        let p = make_page("https://example.com");
        let h = vec![("Content-Security-Policy".into(), "font-src *".into())];
        let f = CspFontSrcDeepValidator::new().analyze(&make_ctx_headers(&p, &h));
        assert!(f.iter().any(|x| x.code == "CSPFNT-V2001"));
    }
    #[test]
    fn test_csp_obj_dd_v8_no_csp() {
        let p = make_page("https://example.com");
        assert!(CspObjectSrcNoneDeepDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_csp_obj_dd_v8_not_none() {
        let p = make_page("https://example.com");
        let h = vec![("Content-Security-Policy".into(), "object-src 'self'".into())];
        let f = CspObjectSrcNoneDeepDeepValidator::new().analyze(&make_ctx_headers(&p, &h));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_csp_obj_dd_v8_is_none() {
        let p = make_page("https://example.com");
        let h = vec![("Content-Security-Policy".into(), "object-src 'none'".into())];
        assert!(CspObjectSrcNoneDeepDeepValidator::new()
            .analyze(&make_ctx_headers(&p, &h))
            .is_empty());
    }
    #[test]
    fn test_csp_base_dd_v8_no_csp() {
        let p = make_page("https://example.com");
        assert!(CspBaseUriSelfDeepDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_csp_base_dd_v8_missing() {
        let p = make_page("https://example.com");
        let h = vec![(
            "Content-Security-Policy".into(),
            "default-src 'self'".into(),
        )];
        let f = CspBaseUriSelfDeepDeepValidator::new().analyze(&make_ctx_headers(&p, &h));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_csp_base_dd_v8_has_self() {
        let p = make_page("https://example.com");
        let h = vec![("Content-Security-Policy".into(), "base-uri 'self'".into())];
        assert!(CspBaseUriSelfDeepDeepValidator::new()
            .analyze(&make_ctx_headers(&p, &h))
            .is_empty());
    }
    #[test]
    fn test_csp_form_dd_v8_no_csp() {
        let p = make_page("https://example.com");
        assert!(CspFormActionSelfDeepDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_csp_form_dd_v8_missing() {
        let p = make_page("https://example.com");
        let h = vec![(
            "Content-Security-Policy".into(),
            "default-src 'self'".into(),
        )];
        let f = CspFormActionSelfDeepDeepValidator::new().analyze(&make_ctx_headers(&p, &h));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_hsts_pr_dd_v8_no_hsts() {
        let p = make_page("https://example.com");
        assert!(HstsPreloadReadyDeepDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_hsts_pr_dd_v8_complete() {
        let p = make_page("https://example.com");
        let h = vec![(
            "Strict-Transport-Security".into(),
            "max-age=31536000; includeSubDomains; preload".into(),
        )];
        assert!(HstsPreloadReadyDeepDeepValidator::new()
            .analyze(&make_ctx_headers(&p, &h))
            .is_empty());
    }
    #[test]
    fn test_hsts_pr_dd_v8_incomplete() {
        let p = make_page("https://example.com");
        let h = vec![(
            "Strict-Transport-Security".into(),
            "max-age=31536000".into(),
        )];
        let f = HstsPreloadReadyDeepDeepValidator::new().analyze(&make_ctx_headers(&p, &h));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_hsts_max_dd_v8_no_hsts() {
        let p = make_page("https://example.com");
        assert!(HstsMaxAgeDeepDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_hsts_max_dd_v8_low() {
        let p = make_page("https://example.com");
        let h = vec![("Strict-Transport-Security".into(), "max-age=3600".into())];
        let f = HstsMaxAgeDeepDeepValidator::new().analyze(&make_ctx_headers(&p, &h));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "HSTSMAX-V2001");
    }
    #[test]
    fn test_hsts_max_dd_v8_ok() {
        let p = make_page("https://example.com");
        let h = vec![(
            "Strict-Transport-Security".into(),
            "max-age=31536000".into(),
        )];
        assert!(HstsMaxAgeDeepDeepValidator::new()
            .analyze(&make_ctx_headers(&p, &h))
            .is_empty());
    }
    #[test]
    fn test_cookie_sec_ddd_v8_no_cookies() {
        let p = make_page("https://example.com");
        assert!(CookieSecureDeepDeepDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_cookie_sec_ddd_v8_missing() {
        let p = make_page("https://example.com");
        let h = vec![("Set-Cookie".into(), "user=abc123".into())];
        let f = CookieSecureDeepDeepDeepValidator::new().analyze(&make_ctx_headers(&p, &h));
        assert!(f.iter().any(|x| x.code == "COOKIESEC-V2001"));
    }
    #[test]
    fn test_cookie_sec_ddd_v8_secure() {
        let p = make_page("https://example.com");
        let h = vec![("Set-Cookie".into(), "session=abc123; Secure".into())];
        assert!(CookieSecureDeepDeepDeepValidator::new()
            .analyze(&make_ctx_headers(&p, &h))
            .is_empty());
    }
    #[test]
    fn test_cookie_http_ddd_v8_no_cookies() {
        let p = make_page("https://example.com");
        assert!(CookieHttpOnlyDeepDeepDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_cookie_http_ddd_v8_missing() {
        let p = make_page("https://example.com");
        let h = vec![("Set-Cookie".into(), "session=abc123".into())];
        let f = CookieHttpOnlyDeepDeepDeepValidator::new().analyze(&make_ctx_headers(&p, &h));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_cookie_ss_ddd_v8_no_cookies() {
        let p = make_page("https://example.com");
        assert!(CookieSameSiteDeepDeepDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_cookie_ss_ddd_v8_missing() {
        let p = make_page("https://example.com");
        let h = vec![("Set-Cookie".into(), "session=abc123".into())];
        let f = CookieSameSiteDeepDeepDeepValidator::new().analyze(&make_ctx_headers(&p, &h));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_mixed_iframe_dd_v8_http() {
        let p = make_page("http://example.com");
        assert!(MixedContentIframeDeepDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_mixed_iframe_dd_v8_no_iframe() {
        let p = make_page("https://example.com");
        let h = MixedContentIframeDeepDeepValidator::new()
            .analyze(&make_ctx(&p, Some("<html><body>No iframes</body></html>")));
        assert!(h.is_empty());
    }
    #[test]
    fn test_mixed_iframe_dd_v8_with_iframe() {
        let p = make_page("https://example.com");
        let f = MixedContentIframeDeepDeepValidator::new().analyze(&make_ctx(
            &p,
            Some("<html><body><iframe src=\"http://bad.com\"></iframe></body></html>"),
        ));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_cors_wild_dd_v8_no_header() {
        let p = make_page("https://example.com");
        assert!(CorsWildcardDeepDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_cors_wild_dd_v8_wildcard() {
        let p = make_page("https://example.com");
        let h = vec![("Access-Control-Allow-Origin".into(), "*".into())];
        let f = CorsWildcardDeepDeepValidator::new().analyze(&make_ctx_headers(&p, &h));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_cors_wild_dd_v8_specific() {
        let p = make_page("https://example.com");
        let h = vec![(
            "Access-Control-Allow-Origin".into(),
            "https://other.com".into(),
        )];
        assert!(CorsWildcardDeepDeepValidator::new()
            .analyze(&make_ctx_headers(&p, &h))
            .is_empty());
    }
    #[test]
    fn test_rp_strict_dd_v8_no_header() {
        let p = make_page("https://example.com");
        assert!(ReferrerPolicyStrictDeepDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_rp_strict_dd_v8_strict() {
        let p = make_page("https://example.com");
        let h = vec![(
            "Referrer-Policy".into(),
            "strict-origin-when-cross-origin".into(),
        )];
        assert!(ReferrerPolicyStrictDeepDeepValidator::new()
            .analyze(&make_ctx_headers(&p, &h))
            .is_empty());
    }
    #[test]
    fn test_rp_strict_dd_v8_not_strict() {
        let p = make_page("https://example.com");
        let h = vec![("Referrer-Policy".into(), "unsafe-url".into())];
        let f = ReferrerPolicyStrictDeepDeepValidator::new().analyze(&make_ctx_headers(&p, &h));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_xfo_dd_v8_no_header_no_csp() {
        let p = make_page("https://example.com");
        assert!(XFrameOptionsDeepDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_xfo_dd_v8_valid() {
        let p = make_page("https://example.com");
        let h = vec![("X-Frame-Options".into(), "DENY".into())];
        assert!(XFrameOptionsDeepDeepValidator::new()
            .analyze(&make_ctx_headers(&p, &h))
            .is_empty());
    }
    #[test]
    fn test_xfo_dd_v8_invalid() {
        let p = make_page("https://example.com");
        let h = vec![("X-Frame-Options".into(), "ALLOW".into())];
        let f = XFrameOptionsDeepDeepValidator::new().analyze(&make_ctx_headers(&p, &h));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_pp_dd_v8_missing() {
        let p = make_page("https://example.com");
        let f = PermissionsPolicyDeepDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "PERMP-V2001-DEEP-DEEP");
    }
    #[test]
    fn test_pp_dd_v8_present() {
        let p = make_page("https://example.com");
        let h = vec![("Permissions-Policy".into(), "camera=()".into())];
        assert!(PermissionsPolicyDeepDeepValidator::new()
            .analyze(&make_ctx_headers(&p, &h))
            .is_empty());
    }
    #[test]
    fn test_coi_dd_v8_no_headers() {
        let p = make_page("https://example.com");
        let f = CrossOriginIsolationDeepDeepValidator::new().analyze(&make_ctx(&p, None));
        assert_eq!(f.len(), 2);
    }
    #[test]
    fn test_coi_dd_v8_both() {
        let p = make_page("https://example.com");
        let h = vec![
            ("Cross-Origin-Embedder-Policy".into(), "require-corp".into()),
            ("Cross-Origin-Opener-Policy".into(), "same-origin".into()),
        ];
        assert!(CrossOriginIsolationDeepDeepValidator::new()
            .analyze(&make_ctx_headers(&p, &h))
            .is_empty());
    }

    // ===== V8 SEO Validators (25) tests =====
}
