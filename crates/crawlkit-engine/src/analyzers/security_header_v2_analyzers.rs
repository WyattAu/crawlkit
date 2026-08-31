//! Versioned security-header analyzers.
//!
//! Extracted from `security_analyzers.rs` as a focused Phase 2 module
//! decomposition. Public names and behavior are preserved through
//! re-exports from `analyzers::mod` and `security_analyzers`.

use crate::types::{IssueCategory, Severity};

use super::{AnalysisContext, Analyzer, Finding};

// =========================================================================
// StrictTransportSecurityAnalyzerV2
// =========================================================================

pub struct StrictTransportSecurityAnalyzerV2;

impl Default for StrictTransportSecurityAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl StrictTransportSecurityAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }

    fn get_header<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
        headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    fn parse_max_age(value: &str) -> Option<u64> {
        for part in value.split(';') {
            let trimmed = part.trim();
            if let Some(val) = trimmed.strip_prefix("max-age=") {
                if let Ok(n) = val.trim().parse::<u64>() {
                    return Some(n);
                }
            }
        }
        None
    }
}

impl Analyzer for StrictTransportSecurityAnalyzerV2 {
    fn name(&self) -> &str {
        "strict-transport-security-v2"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        match Self::get_header(ctx.headers, "Strict-Transport-Security") {
            None => {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "HSTS-V2001".to_string(),
                    title: "Missing Strict-Transport-Security header".to_string(),
                    description: "No Strict-Transport-Security header was found.".to_string(),
                    url: url.to_string(),
                    recommendation: "Set Strict-Transport-Security: max-age=31536000; includeSubDomains; preload.".to_string(),
                });
            }
            Some(value) => {
                if let Some(max_age) = Self::parse_max_age(value) {
                    if max_age < 31536000 {
                        findings.push(Finding {
                            severity: Severity::Warning,
                            category: IssueCategory::Security,
                            code: "HSTS-V2002".to_string(),
                            title: "HSTS max-age is too short".to_string(),
                            description: format!(
                                "Strict-Transport-Security max-age is {max_age} seconds, which is below the recommended minimum of 31536000."
                            ),
                            url: url.to_string(),
                            recommendation: "Set max-age to at least 31536000 (1 year).".to_string(),
                        });
                    }
                }

                if !value.to_lowercase().contains("includesubdomains") {
                    findings.push(Finding {
                        severity: Severity::Info,
                        category: IssueCategory::Security,
                        code: "HSTS-V2003".to_string(),
                        title: "HSTS missing includeSubDomains".to_string(),
                        description: "The Strict-Transport-Security header does not include the includeSubDomains directive.".to_string(),
                        url: url.to_string(),
                        recommendation: "Add includeSubDomains to the HSTS header.".to_string(),
                    });
                }
            }
        }

        findings
    }
}

// =========================================================================
// XssProtectionAnalyzerV2
// =========================================================================

pub struct XssProtectionAnalyzerV2;

impl Default for XssProtectionAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl XssProtectionAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }

    fn get_header<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
        headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

impl Analyzer for XssProtectionAnalyzerV2 {
    fn name(&self) -> &str {
        "xss-protection-v2"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        match Self::get_header(ctx.headers, "X-XSS-Protection") {
            None => findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Security,
                code: "XSS-V2001".to_string(),
                title: "Missing X-XSS-Protection header".to_string(),
                description: "No X-XSS-Protection header was found.".to_string(),
                url: url.to_string(),
                recommendation: "Set X-XSS-Protection: 1; mode=block.".to_string(),
            }),
            Some(value) if value.trim() == "0" => findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Security,
                code: "XSS-V2002".to_string(),
                title: "X-XSS-Protection explicitly disabled".to_string(),
                description: "X-XSS-Protection is set to 0, explicitly disabling the XSS auditor."
                    .to_string(),
                url: url.to_string(),
                recommendation: "Ensure Content-Security-Policy is properly configured."
                    .to_string(),
            }),
            Some(_) => {}
        }

        findings
    }
}

// =========================================================================
// ContentTypeSniffingAnalyzerV2
// =========================================================================

pub struct ContentTypeSniffingAnalyzerV2;

impl Default for ContentTypeSniffingAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl ContentTypeSniffingAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }

    fn get_header<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
        headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

impl Analyzer for ContentTypeSniffingAnalyzerV2 {
    fn name(&self) -> &str {
        "content-type-sniffing-v2"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        match Self::get_header(ctx.headers, "X-Content-Type-Options") {
            None => findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Security,
                code: "CT-V2001".to_string(),
                title: "Missing X-Content-Type-Options header".to_string(),
                description: "No X-Content-Type-Options header was found.".to_string(),
                url: url.to_string(),
                recommendation: "Set X-Content-Type-Options: nosniff.".to_string(),
            }),
            Some(value) if value.trim().to_lowercase() != "nosniff" => findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Security,
                code: "CT-V2002".to_string(),
                title: "Invalid X-Content-Type-Options value".to_string(),
                description: format!(
                    "X-Content-Type-Options is \"{value}\" instead of \"nosniff\"."
                ),
                url: url.to_string(),
                recommendation: "Set X-Content-Type-Options: nosniff.".to_string(),
            }),
            Some(_) => {}
        }

        findings
    }
}

// =========================================================================
// HstsPreloadListValidator — HSTSPRELOAD001
// =========================================================================

pub struct HstsPreloadListValidator;

impl Default for HstsPreloadListValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl HstsPreloadListValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for HstsPreloadListValidator {
    fn name(&self) -> &str {
        "hsts-preload-list-validator"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let hsts = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Strict-Transport-Security"))
            .map(|(_, v)| v.as_str());
        if let Some(value) = hsts {
            let lower = value.to_lowercase();
            if lower.contains("preload") {
                let max_age_ok = lower.find("max-age=").is_some_and(|pos| {
                    let after = &lower[pos + 8..];
                    let num: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
                    num.parse::<u64>().is_ok_and(|a| a >= 31536000)
                });
                let has_isd = lower.contains("includesubdomains");
                if !max_age_ok || !has_isd {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Security,
                        code: "HSTSPRELOAD001".to_string(),
                        title: "HSTS preload directive present without meeting preload requirements".to_string(),
                        description: "The HSTS header includes the preload directive but does not meet the requirements for HSTS preload list inclusion (max-age >= 31536000 and includeSubDomains required).".to_string(),
                        url: url.to_string(),
                        recommendation: "Set max-age to at least 31536000 and include includeSubDomains to be eligible for the HSTS preload list.".to_string(),
                    });
                }
            }
        }
        findings
    }
}
