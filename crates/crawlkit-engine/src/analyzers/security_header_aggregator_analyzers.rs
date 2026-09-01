//! SecurityHeaderAnalyzer — the original aggregate security-header checker.
//!
//! One check per header (CSP, HSTS, X-Frame-Options, X-Content-Type-Options,
//! Referrer-Policy, Permissions-Policy, cross-origin family) plus a posture
//! score. Extracted verbatim from `security_analyzers.rs` (Phase 2 SRP step);
//! the legacy module path re-exports it, so the public name is unchanged.

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
    clippy::manual_contains
)]

use crate::types::{IssueCategory, Severity};

use super::{AnalysisContext, Analyzer, Finding};

// ---------------------------------------------------------------------------
// 14. Security Header Analyzer
// ---------------------------------------------------------------------------

pub struct SecurityHeaderAnalyzer;

impl SecurityHeaderAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Look up a header value by name (case-insensitive).
    fn get_header<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
        headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    /// Validate a CSP directive string (basic syntax check).
    fn is_valid_csp(value: &str) -> bool {
        if value.trim().is_empty() {
            return false;
        }
        // CSP must contain at least one directive (e.g. "default-src 'self'")
        let directives = [
            "default-src",
            "script-src",
            "style-src",
            "img-src",
            "font-src",
            "connect-src",
            "frame-src",
            "object-src",
            "media-src",
            "child-src",
            "worker-src",
            "manifest-src",
            "form-action",
            "frame-ancestors",
            "base-uri",
            "upgrade-insecure-requests",
            "block-all-mixed-content",
        ];
        value.split(';').any(|part| {
            let trimmed = part.trim();
            directives.iter().any(|d| trimmed.starts_with(d))
        })
    }

    /// Validate HSTS value (max-age, includeSubDomains, preload).
    fn validate_hsts(value: &str) -> Vec<String> {
        let mut issues = Vec::new();
        let lower = value.to_lowercase();

        // Must contain max-age
        if !lower.contains("max-age=") {
            issues.push("missing max-age directive".to_string());
        } else if let Some(ma_pos) = lower.find("max-age=") {
            // SECURITY FIX: Use `lower` for slicing, not `value`.
            // `ma_pos` is the byte position in `lower`; slicing the
            // original `value` at that index is incorrect when the
            // original contains multi-byte or case-changing characters.
            let after = &lower[ma_pos + 8..];
            let num_str: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
            match num_str.parse::<u64>() {
                Ok(age) if age < 31536000 => {
                    issues.push(format!(
                        "max-age ({age}) is below recommended minimum of 31536000 (1 year)"
                    ));
                }
                Ok(_) => {} // acceptable
                Err(_) => {
                    issues.push("max-age is not a valid integer".to_string());
                }
            }
        }

        issues
    }

    /// Compute a security posture score (0-100) from the findings.
    fn compute_score(findings: &[Finding]) -> u32 {
        let mut score: i32 = 100;
        for f in findings {
            if f.code == "SEC012" {
                continue; // Don't count the score finding itself
            }
            match f.severity {
                Severity::Critical => score -= 20,
                Severity::Error => score -= 10,
                Severity::Warning => score -= 5,
                Severity::Info => {}
            }
        }
        score.max(0) as u32
    }
    // ---- Individual header checks (single responsibility each) ----

    fn check_csp(&self, h: &[(String, String)], url: &str, f: &mut Vec<Finding>) {
        match Self::get_header(h, "Content-Security-Policy") {
            None => f.push(Finding {
                severity: Severity::Warning, category: IssueCategory::Security,
                code: "SEC001".to_string(),
                title: "Missing Content-Security-Policy header".to_string(),
                description: "No Content-Security-Policy header was found. CSP helps prevent XSS, clickjacking, and other code injection attacks.".into(),
                url: url.to_string(),
                recommendation: "Implement a Content-Security-Policy header. Start with default-src \'self\' and refine as needed.".into(),
            }),
            Some(csp) if !Self::is_valid_csp(csp) => f.push(Finding {
                severity: Severity::Warning, category: IssueCategory::Security,
                code: "SEC013".to_string(),
                title: "Invalid Content-Security-Policy syntax".to_string(),
                description: "The CSP header value does not appear to contain valid directive syntax.".into(),
                url: url.to_string(),
                recommendation: "Ensure CSP contains at least one valid directive (e.g. default-src, script-src).".into(),
            }),
            _ => {}
        }
    }

    fn check_hsts(&self, h: &[(String, String)], url: &str, f: &mut Vec<Finding>) {
        match Self::get_header(h, "Strict-Transport-Security") {
            None => f.push(Finding {
                severity: Severity::Warning, category: IssueCategory::Security,
                code: "SEC002".to_string(),
                title: "Missing Strict-Transport-Security header".to_string(),
                description: "No Strict-Transport-Security (HSTS) header was found. HSTS forces browsers to use HTTPS.".into(),
                url: url.to_string(),
                recommendation: "Add Strict-Transport-Security: max-age=31536000; includeSubDomains; preload.".into(),
            }),
            Some(hsts) => {
                for issue in Self::validate_hsts(hsts) {
                    f.push(Finding {
                        severity: Severity::Warning, category: IssueCategory::Security,
                        code: "SEC014".to_string(), title: "HSTS configuration issue".into(),
                        description: format!("HSTS header: {issue}."), url: url.to_string(),
                        recommendation: "Set max-age to at least 31536000 (1 year). Add includeSubDomains and preload.".into(),
                    });
                }
                if !hsts.to_lowercase().contains("includesubdomains") {
                    f.push(Finding {
                        severity: Severity::Info, category: IssueCategory::Security,
                        code: "SEC015".to_string(), title: "HSTS missing includeSubDomains".into(),
                        description: "The HSTS header does not include the includeSubDomains directive.".into(),
                        url: url.to_string(), recommendation: "Add includeSubDomains to protect all subdomains.".into(),
                    });
                }
                if !hsts.to_lowercase().contains("preload") {
                    f.push(Finding {
                        severity: Severity::Info, category: IssueCategory::Security,
                        code: "SEC016".to_string(), title: "HSTS missing preload".into(),
                        description: "The HSTS header does not include the preload directive.".into(),
                        url: url.to_string(),
                        recommendation: "Consider adding preload for browser HSTS preload list inclusion.".into(),
                    });
                }
            }
        }
    }

    fn check_xfo(&self, h: &[(String, String)], url: &str, f: &mut Vec<Finding>) {
        match Self::get_header(h, "X-Frame-Options") {
            None => f.push(Finding {
                severity: Severity::Warning, category: IssueCategory::Security,
                code: "SEC003".to_string(), title: "Missing X-Frame-Options header".into(),
                description: "No X-Frame-Options header was found. This header prevents clickjacking by controlling frame embedding.".into(),
                url: url.to_string(), recommendation: "Set X-Frame-Options to DENY or SAMEORIGIN.".into(),
            }),
            Some(value) => {
                if value.to_uppercase().trim() != "DENY" && value.to_uppercase().trim() != "SAMEORIGIN" {
                    f.push(Finding {
                        severity: Severity::Warning, category: IssueCategory::Security,
                        code: "SEC004".to_string(), title: "Invalid X-Frame-Options value".into(),
                        description: format!("X-Frame-Options is \"{value}\" but must be DENY or SAMEORIGIN."),
                        url: url.to_string(), recommendation: "Set X-Frame-Options to DENY (preferred) or SAMEORIGIN.".into(),
                    });
                }
            }
        }
    }

    fn check_xcto(&self, h: &[(String, String)], url: &str, f: &mut Vec<Finding>) {
        match Self::get_header(h, "X-Content-Type-Options") {
            None => f.push(Finding {
                severity: Severity::Warning, category: IssueCategory::Security,
                code: "SEC005".to_string(), title: "Missing X-Content-Type-Options header".into(),
                description: "No X-Content-Type-Options header was found. This header prevents MIME-type sniffing.".into(),
                url: url.to_string(), recommendation: "Set X-Content-Type-Options to nosniff.".into(),
            }),
            Some(value) if value.trim().to_lowercase() != "nosniff" => f.push(Finding {
                severity: Severity::Warning, category: IssueCategory::Security,
                code: "SEC006".to_string(), title: "Invalid X-Content-Type-Options value".into(),
                description: format!("X-Content-Type-Options is \"{value}\" but must be nosniff."),
                url: url.to_string(), recommendation: "Set X-Content-Type-Options to nosniff.".into(),
            }),
            _ => {}
        }
    }

    fn check_referrer(&self, h: &[(String, String)], url: &str, f: &mut Vec<Finding>) {
        const RECOMMENDED: &[&str] = &[
            "no-referrer",
            "no-referrer-when-downgrade",
            "origin",
            "origin-when-cross-origin",
            "same-origin",
            "strict-origin",
            "strict-origin-when-cross-origin",
            "unsafe-url",
        ];
        match Self::get_header(h, "Referrer-Policy") {
            None => f.push(Finding {
                severity: Severity::Info, category: IssueCategory::Security,
                code: "SEC007".to_string(), title: "Missing Referrer-Policy header".into(),
                description: "No Referrer-Policy header was found. This header controls how much referrer information is sent with requests.".into(),
                url: url.to_string(), recommendation: "Set Referrer-Policy to strict-origin-when-cross-origin or no-referrer for maximum privacy.".into(),
            }),
            Some(value) if !RECOMMENDED.contains(&value.trim()) => f.push(Finding {
                severity: Severity::Info, category: IssueCategory::Security,
                code: "SEC017".to_string(), title: "Uncommon Referrer-Policy value".into(),
                description: format!("Referrer-Policy \"{value}\" is not in the list of commonly used policies."),
                url: url.to_string(), recommendation: "Consider using strict-origin-when-cross-origin or no-referrer.".into(),
            }),
            _ => {}
        }
    }

    fn check_permissions(&self, h: &[(String, String)], url: &str, f: &mut Vec<Finding>) {
        match Self::get_header(h, "Permissions-Policy") {
            None => f.push(Finding {
                severity: Severity::Info, category: IssueCategory::Security,
                code: "SEC008".to_string(), title: "Missing Permissions-Policy header".into(),
                description: "No Permissions-Policy header was found. This header controls which browser features APIs can be used.".into(),
                url: url.to_string(),
                recommendation: "Consider setting Permissions-Policy to disable unused features like camera, microphone, geolocation.".into(),
            }),
            Some(pp) => {
                let pp_lower = pp.to_lowercase();
                for feature in &["camera", "microphone", "geolocation"] {
                    if pp_lower.contains(feature) && !pp_lower.contains(&format!("{feature}=()")) {
                        f.push(Finding {
                            severity: Severity::Info, category: IssueCategory::Security,
                            code: "SEC018".to_string(), title: format!("Permissions-Policy: {feature} not restricted"),
                            description: format!("The {feature} feature in Permissions-Policy is not explicitly restricted."),
                            url: url.to_string(), recommendation: format!("Add {feature}=() to Permissions-Policy to disable it if not needed."),
                        });
                    }
                }
            }
        }
    }

    fn check_cross_origin(&self, h: &[(String, String)], url: &str, f: &mut Vec<Finding>) {
        let checks = [
            (
                "Cross-Origin-Embedder-Policy",
                "SEC009",
                "Cross-Origin-Embedder-Policy",
                "COEP prevents resources from loading cross-origin without explicit permission.",
                "Set COEP to require-corp for stricter cross-origin isolation.",
            ),
            (
                "Cross-Origin-Opener-Policy",
                "SEC010",
                "Cross-Origin-Opener-Policy",
                "COOP isolates your browsing context from cross-origin popups.",
                "Set COOP to same-origin for stricter isolation.",
            ),
            (
                "Cross-Origin-Resource-Policy",
                "SEC011",
                "Cross-Origin-Resource-Policy",
                "CORP prevents cross-origin reads of embedded resources.",
                "Set CORP to same-origin if the resource should only be used by the same origin.",
            ),
        ];
        for (header, code, name, desc, rec) in checks {
            if Self::get_header(h, header).is_none() {
                f.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Security,
                    code: code.to_string(),
                    title: format!("Missing {name} header"),
                    description: format!("No {name} header was found. {desc}"),
                    url: url.to_string(),
                    recommendation: rec.into(),
                });
            }
        }
    }
}

impl Default for SecurityHeaderAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for SecurityHeaderAnalyzer {
    fn name(&self) -> &str {
        "security-headers"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut f = Vec::new();
        let url = &ctx.page.url;
        let h = ctx.headers;
        self.check_csp(h, url, &mut f);
        self.check_hsts(h, url, &mut f);
        self.check_xfo(h, url, &mut f);
        self.check_xcto(h, url, &mut f);
        self.check_referrer(h, url, &mut f);
        self.check_permissions(h, url, &mut f);
        self.check_cross_origin(h, url, &mut f);
        let score = Self::compute_score(&f);
        f.push(Finding {
            severity: Severity::Info, category: IssueCategory::Security,
            code: "SEC012".to_string(), title: "Security posture score".to_string(),
            description: format!("Security header score: {score}/100."), url: url.clone(),
            recommendation: if score < 50 {
                "Security posture is weak. Prioritize adding CSP, HSTS, and frame-protecting headers.".into()
            } else if score < 80 {
                "Security posture is moderate. Address remaining missing headers.".into()
            } else {
                "Security posture is strong. Minor improvements possible.".into()
            },
        });
        f
    }
}
