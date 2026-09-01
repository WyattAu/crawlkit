//! DNS rebinding, Subresource Integrity, and CORS security analyzers.
//!
//! Extracted from `security_analyzers.rs` as a focused Phase 2 module
//! decomposition. The original public names and behavior are preserved by
//! re-exports from `analyzers::mod` and `security_analyzers`.

#![allow(clippy::useless_format)]

use crate::types::{IssueCategory, Severity};

use super::{AnalysisContext, Analyzer, Finding};

// =========================================================================
// DnsRebindingAnalyzer
// =========================================================================

pub struct DnsRebindingAnalyzer;

impl Default for DnsRebindingAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl DnsRebindingAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for DnsRebindingAnalyzer {
    fn name(&self) -> &str {
        "dns-rebinding"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        // Check for CORS headers that could enable DNS rebinding
        let has_cors_wildcard = ctx
            .headers
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("Access-Control-Allow-Origin") && v.trim() == "*");

        if has_cors_wildcard {
            // Check if the page also sets credentials
            let has_credentials = ctx.headers.iter().any(|(k, v)| {
                k.eq_ignore_ascii_case("Access-Control-Allow-Credentials")
                    && v.trim().eq_ignore_ascii_case("true")
            });

            if has_credentials {
                findings.push(Finding {
                    severity: Severity::Critical,
                    category: IssueCategory::Security,
                    code: "DNSREBIND001".to_string(),
                    title: "CORS wildcard with credentials enabled".to_string(),
                    description: "The page sets Access-Control-Allow-Origin: * and \
                                  Access-Control-Allow-Credentials: true, which is an \
                                  invalid and dangerous configuration that could enable \
                                  DNS rebinding attacks."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Remove the wildcard Access-Control-Allow-Origin or \
                                     disable credentials. Never combine * origin with \
                                     credentials."
                        .to_string(),
                });
            }

            // Check if the page has local/internal IP patterns in body
            if let Some(body) = ctx.body {
                let local_patterns = [
                    "127.0.0.1",
                    "localhost",
                    "0.0.0.0",
                    "192.168.",
                    "10.0.",
                    "172.16.",
                ];
                let has_local_ref = local_patterns.iter().any(|p| body.contains(p));
                if has_local_ref {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Security,
                        code: "DNSREBIND002".to_string(),
                        title: "CORS wildcard with local network references".to_string(),
                        description: "The page uses Access-Control-Allow-Origin: * and \
                                      references local network addresses, which could \
                                      indicate a DNS rebinding vulnerability."
                            .to_string(),
                        url: url.clone(),
                        recommendation: "Restrict CORS to specific trusted origins \
                                         instead of using wildcard."
                            .to_string(),
                    });
                }
            }
        }

        findings
    }
}

// =========================================================================
// SubresourceIntegrityAnalyzer
// =========================================================================

pub struct SubresourceIntegrityAnalyzer;

impl Default for SubresourceIntegrityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl SubresourceIntegrityAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for SubresourceIntegrityAnalyzer {
    fn name(&self) -> &str {
        "subresource-integrity"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let body = match ctx.body {
            Some(b) => b,
            None => return findings,
        };

        // Count external script tags with src but without integrity
        let scripts_without_sri = ctx
            .page
            .scripts
            .iter()
            .filter(|s| {
                s.src.as_ref().is_some_and(|src| {
                    let is_external = src.starts_with("http://")
                        || src.starts_with("https://")
                        || src.starts_with("//");
                    let has_integrity =
                        body.contains(&format!("integrity=\"")) && body.contains(src);
                    is_external && !has_integrity
                })
            })
            .count();

        if scripts_without_sri > 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Security,
                code: "SRISCRIPT001".to_string(),
                title: "External scripts missing Subresource Integrity".to_string(),
                description: format!(
                    "{scripts_without_sri} external script(s) are loaded without \
                     Subresource Integrity (SRI). Without SRI, compromised CDNs \
                     could inject malicious code."
                ),
                url: url.clone(),
                recommendation: "Add integrity and crossorigin attributes to external \
                                 script tags. Generate SRI hashes with: openssl dgst \
                                 -sha384 -binary file.js | openssl base64 -A"
                    .to_string(),
            });
        }

        findings
    }
}

// =========================================================================
// CorsMisconfigurationAnalyzer
// =========================================================================

pub struct CorsMisconfigurationAnalyzer;

impl Default for CorsMisconfigurationAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl CorsMisconfigurationAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for CorsMisconfigurationAnalyzer {
    fn name(&self) -> &str {
        "cors-misconfiguration"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let cors_origin = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Access-Control-Allow-Origin"))
            .map(|(_, v)| v.as_str());

        let cors_creds = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Access-Control-Allow-Credentials"))
            .map(|(_, v)| v.as_str());

        if let Some(origin) = cors_origin {
            if origin.trim() == "*" && cors_creds == Some("true") {
                findings.push(Finding {
                    severity: Severity::Critical,
                    category: IssueCategory::Security,
                    code: "CORS001-MISCONFIG".to_string(),
                    title: "CORS allows all origins with credentials".to_string(),
                    description: "Access-Control-Allow-Origin is set to \"*\" with \
                                  Access-Control-Allow-Credentials: true. This is an \
                                  invalid and insecure configuration."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Replace wildcard origin with the specific allowed \
                                     origin or remove credentials support."
                        .to_string(),
                });
            } else if origin.trim() == "*" {
                // Wildcard without credentials is less severe but still notable
                // for sensitive endpoints
                let is_sensitive = url.contains("/api/")
                    || url.contains("/admin")
                    || url.contains("/auth")
                    || url.contains("/login");
                if is_sensitive {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Security,
                        code: "CORS002-MISCONFIG".to_string(),
                        title: "CORS wildcard on sensitive endpoint".to_string(),
                        description: "Access-Control-Allow-Origin is set to \"*\" on a \
                                      sensitive endpoint. While technically valid without \
                                      credentials, this allows any site to read responses."
                            .to_string(),
                        url: url.clone(),
                        recommendation: "Restrict CORS to specific trusted origins for \
                                         sensitive endpoints."
                            .to_string(),
                    });
                }
            }
        }

        findings
    }
}
