//! Security header analyzers extracted from `security_analyzers.rs`.
//!
//! Phase 2 SRP step: each analyzer group moves to its own module so the
//! 13.5k-line monolith shrinks incrementally. No public API change —
//! types are re-exported from `analyzers::mod` identically.

use crate::types::{IssueCategory, Severity};

use super::{AnalysisContext, Analyzer, Finding};

// ---------------------------------------------------------------------------
// Subresource Integrity (SRI) Analyzer
// ---------------------------------------------------------------------------

pub struct SriAnalyzer;

impl SriAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SriAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for SriAnalyzer {
    fn name(&self) -> &str {
        "sri"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        // Check external scripts without integrity
        let external_scripts_without_integrity: Vec<&str> = ctx
            .page
            .scripts
            .iter()
            .filter(|s| {
                s.src.as_ref().is_some_and(|src| {
                    let is_external = src.starts_with("http://")
                        || src.starts_with("https://")
                        || src.starts_with("//");
                    is_external && !s.has_integrity
                })
            })
            .filter_map(|s| s.src.as_deref())
            .collect();

        if !external_scripts_without_integrity.is_empty() {
            let examples: Vec<&str> = external_scripts_without_integrity
                .iter()
                .take(5)
                .copied()
                .collect();
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Security,
                code: "SRI001".to_string(),
                title: "External scripts missing integrity attribute".to_string(),
                description: format!(
                    "{} external script(s) lack the integrity attribute: {}. Without SRI, \
                     compromised CDNs or man-in-the-middle attacks could inject malicious code.",
                    external_scripts_without_integrity.len(),
                    examples.join(", ")
                ),
                url: url.to_string(),
                recommendation: "Add an integrity attribute with the expected hash to all \
                                 external scripts. Use a tool like SRI Hash Generator to \
                                 compute the hash."
                    .into(),
            });
        }

        // Check external stylesheets without integrity
        let external_styles_without_integrity: Vec<&str> = ctx
            .page
            .styles
            .iter()
            .filter(|s| {
                s.href.as_ref().is_some_and(|href| {
                    let is_external = href.starts_with("http://")
                        || href.starts_with("https://")
                        || href.starts_with("//");
                    is_external && !s.has_integrity
                })
            })
            .filter_map(|s| s.href.as_deref())
            .collect();

        if !external_styles_without_integrity.is_empty() {
            let examples: Vec<&str> = external_styles_without_integrity
                .iter()
                .take(5)
                .copied()
                .collect();
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Security,
                code: "SRI002".to_string(),
                title: "External stylesheets missing integrity attribute".to_string(),
                description: format!(
                    "{} external stylesheet(s) lack the integrity attribute: {}. Without SRI, \
                     a compromised stylesheet could alter page layout or inject CSS-based attacks.",
                    external_styles_without_integrity.len(),
                    examples.join(", ")
                ),
                url: url.to_string(),
                recommendation: "Add an integrity attribute with the expected hash to all \
                                 external stylesheets."
                    .into(),
            });
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// Permission Policy Analyzer
// ---------------------------------------------------------------------------

pub struct PermissionPolicyAnalyzer;

impl PermissionPolicyAnalyzer {
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

impl Default for PermissionPolicyAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for PermissionPolicyAnalyzer {
    fn name(&self) -> &str {
        "permission-policy"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        match Self::get_header(ctx.headers, "Permissions-Policy") {
            None => {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "PERM001".to_string(),
                    title: "Missing Permissions-Policy header".to_string(),
                    description: "No Permissions-Policy header was found. This header controls \
                                  which browser features and APIs can be used. Without it, all \
                                  features are available by default."
                        .to_string(),
                    url: url.to_string(),
                    recommendation: "Set a Permissions-Policy header to restrict access to \
                                     sensitive features like camera, microphone, and geolocation."
                        .into(),
                });
            }
            Some(policy) => {
                let lower = policy.to_lowercase();
                for feature in &["camera", "microphone"] {
                    // If the feature is mentioned but not restricted to ()
                    if lower.contains(feature)
                        && !lower.contains(&format!("{feature}=()"))
                        && !lower.contains(&format!("{feature}=(self)"))
                    {
                        findings.push(Finding {
                            severity: Severity::Warning,
                            category: IssueCategory::Security,
                            code: "PERM002".to_string(),
                            title: format!("Permissions-Policy: {feature} not restricted"),
                            description: format!(
                                "The {feature} feature in Permissions-Policy is not explicitly \
                                 restricted. Allowing {feature} access increases the attack \
                                 surface for microphone/camera-based attacks."
                            ),
                            url: url.to_string(),
                            recommendation: format!(
                                "Add {feature}=() to Permissions-Policy to disable it if not \
                                 needed, or {feature}=(self) to restrict to same-origin."
                            ),
                        });
                    }
                }
            }
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// Cross-Origin Isolation Analyzer
// ---------------------------------------------------------------------------

pub struct CrossOriginIsolationAnalyzer;

impl CrossOriginIsolationAnalyzer {
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

impl Default for CrossOriginIsolationAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for CrossOriginIsolationAnalyzer {
    fn name(&self) -> &str {
        "cross-origin-isolation"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        if Self::get_header(ctx.headers, "Cross-Origin-Embedder-Policy").is_none() {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Security,
                code: "COEP001".to_string(),
                title: "Missing Cross-Origin-Embedder-Policy header".to_string(),
                description: "No Cross-Origin-Embedder-Policy (COEP) header was found. COEP \
                              prevents loading cross-origin resources without explicit opt-in, \
                              enabling cross-origin isolation for high-precision timing \
                              mitigation (Spectre)."
                    .to_string(),
                url: url.to_string(),
                recommendation: "Set Cross-Origin-Embedder-Policy: require-corp to enable \
                                 cross-origin isolation."
                    .into(),
            });
        }

        if Self::get_header(ctx.headers, "Cross-Origin-Opener-Policy").is_none() {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Security,
                code: "COOP002".to_string(),
                title: "Missing Cross-Origin-Opener-Policy header".to_string(),
                description: "No Cross-Origin-Opener-Policy (COOP) header was found. COOP \
                              isolates your browsing context from cross-origin popups, preventing \
                              cross-origin window references that could be exploited."
                    .to_string(),
                url: url.to_string(),
                recommendation: "Set Cross-Origin-Opener-Policy: same-origin to isolate your \
                                 browsing context."
                    .into(),
            });
        }

        findings
    }
}
