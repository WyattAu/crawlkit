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
use std::collections::HashMap;

use regex::Regex;

use crate::types::{IssueCategory, Severity};

use super::{AnalysisContext, Analyzer, Finding};
use crate::parser::ExtractedImage;

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

// ---------------------------------------------------------------------------

pub struct MobileFriendlinessChecker;

impl MobileFriendlinessChecker {
    pub fn new() -> Self {
        Self
    }

    /// Parse viewport meta content into a map of directives.
    pub(crate) fn parse_viewport(viewport: &str) -> HashMap<String, String> {
        let mut map = HashMap::new();
        for part in viewport.split(',') {
            let part = part.trim();
            if let Some((key, value)) = part.split_once('=') {
                map.insert(key.trim().to_lowercase(), value.trim().to_lowercase());
            }
        }
        map
    }
}

impl Default for MobileFriendlinessChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for MobileFriendlinessChecker {
    fn name(&self) -> &str {
        "mobile-friendliness"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        // --- Viewport meta tag presence ---
        let viewport = match &ctx.page.meta.viewport {
            Some(v) => v,
            None => {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Mobile,
                    code: "MOB001".to_string(),
                    title: "Missing viewport meta tag".to_string(),
                    description: "No viewport meta tag was found. Without it, the page will not \
                                  scale properly on mobile devices."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add <meta name=\"viewport\" content=\"width=device-width, \
                                     initial-scale=1\"> to the <head>."
                        .to_string(),
                });
                return findings;
            }
        };

        let directives = Self::parse_viewport(viewport);

        // --- width=device-width ---
        match directives.get("width") {
            None => {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Mobile,
                    code: "MOB002".to_string(),
                    title: "Viewport missing width directive".to_string(),
                    description: "The viewport meta tag does not specify width=device-width."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Set width=device-width in the viewport meta tag.".to_string(),
                });
            }
            Some(w) if w != "device-width" => {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Mobile,
                    code: "MOB003".to_string(),
                    title: "Viewport width is not device-width".to_string(),
                    description: format!(
                        "Viewport width is set to \"{w}\" instead of device-width. This forces \
                         a fixed layout."
                    ),
                    url: url.clone(),
                    recommendation: "Change viewport width to device-width for responsive layout."
                        .to_string(),
                });
            }
            _ => {} // device-width — correct
        }

        // --- user-scalable=no or maximum-scale=1.0 ---
        if directives.get("user-scalable") == Some(&"no".to_string()) {
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Mobile,
                code: "MOB004".to_string(),
                title: "Zooming is disabled (user-scalable=no)".to_string(),
                description: "The viewport meta tag disables user zooming with \
                              user-scalable=no. This is a critical accessibility issue."
                    .to_string(),
                url: url.clone(),
                recommendation: "Remove user-scalable=no to allow pinch-to-zoom. This is \
                                 required for WCAG 2.1 compliance."
                    .to_string(),
            });
        }

        if let Some(max_scale) = directives.get("maximum-scale") {
            if max_scale == "1" || max_scale == "1.0" {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Mobile,
                    code: "MOB005".to_string(),
                    title: "Maximum scale restricted".to_string(),
                    description: format!(
                        "maximum-scale={max_scale} limits zooming. While not as severe as \
                         user-scalable=no, it can still hinder accessibility."
                    ),
                    url: url.clone(),
                    recommendation: "Remove the maximum-scale constraint or set it to at least \
                                     5.0."
                        .to_string(),
                });
            }
        }

        // --- initial-scale ---
        if let Some(scale) = directives.get("initial-scale") {
            if scale != "1" && scale != "1.0" {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Mobile,
                    code: "MOB009".to_string(),
                    title: "Non-standard initial scale".to_string(),
                    description: format!(
                        "initial-scale is set to \"{scale}\" instead of the standard 1.0. This \
                         may cause unexpected zoom behavior on page load."
                    ),
                    url: url.clone(),
                    recommendation: "Set initial-scale=1 for a consistent mobile experience."
                        .to_string(),
                });
            }
        }

        // MOB006-008 removed: placeholder findings that fire unconditionally on every page.
        // Touch targets, font sizes, and horizontal scrolling require CSS layout
        // computation or runtime testing that is not implemented. Emitting these
        // as Info on every page corrupts issue counts and undermines trust.

        findings
    }
}

// ---------------------------------------------------------------------------
// 17. Accessibility Analyzer (WCAG 2.1 AA)
// ---------------------------------------------------------------------------

pub struct AccessibilityAnalyzer;

impl AccessibilityAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Generic link text patterns that are not descriptive.
    const VAGUE_LINK_TEXTS: &[&str] = &[
        "click here",
        "here",
        "read more",
        "more",
        "learn more",
        "click",
        "link",
        "this",
        "go",
        "continue",
    ];
    /// WCAG 1.1.1: only a MISSING alt attribute is a failure.
    ///
    /// `alt=""` (present but empty) is the WCAG H67 mechanism for
    /// decorative images and must not be flagged — axe-core and Lighthouse
    /// treat it identically. `aria-hidden="true"` further removes the
    /// image from the accessibility tree (common trust-badge pattern
    /// where adjacent text carries the meaning).
    fn check_images_alt(&self, ctx: &AnalysisContext, url: &str, f: &mut Vec<Finding>) {
        let images_without_alt: Vec<&ExtractedImage> =
            ctx.page.images.iter().filter(|img| !img.has_alt).collect();
        if images_without_alt.is_empty() {
            return;
        }
        let srcs: Vec<&str> = images_without_alt
            .iter()
            .map(|img| img.src.as_str())
            .collect();
        f.push(Finding {
            severity: Severity::Error,
            category: IssueCategory::Accessibility,
            code: "A11Y001".to_string(),
            title: "Images missing alt attribute".into(),
            description: format!(
                "{} image(s) have no alt attribute at all: {}. Decorative \
                 images should use alt=\"\" (and optionally aria-hidden); \
                 meaningful images need descriptive alt text.",
                images_without_alt.len(),
                srcs.join(", ")
            ),
            url: url.to_string(),
            recommendation: "Add an alt attribute to every img. Use descriptive \
                             text for meaningful images and alt=\"\" for \
                             decorative ones."
                .into(),
        });
    }

    fn check_headings(&self, ctx: &AnalysisContext, url: &str, f: &mut Vec<Finding>) {
        if ctx.page.headings.is_empty() {
            f.push(Finding {
                severity: Severity::Warning, category: IssueCategory::Accessibility,
                code: "A11Y002".to_string(), title: "No headings found".into(),
                description: "The page has no heading elements. Headings provide structure for screen reader users.".into(),
                url: url.to_string(), recommendation: "Add heading elements (H1-H6) to provide page structure.".into(),
            });
            return;
        }
        let h1_count = ctx.page.headings.iter().filter(|h| h.level == 1).count();
        if h1_count == 0 {
            f.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Accessibility,
                code: "A11Y003".to_string(),
                title: "Missing H1 heading".into(),
                description:
                    "No H1 heading found. Screen readers use H1 to identify the main page topic."
                        .into(),
                url: url.to_string(),
                recommendation: "Add exactly one H1 heading per page.".into(),
            });
        } else if h1_count > 1 {
            f.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "A11Y004".to_string(),
                title: "Multiple H1 headings".into(),
                description: format!(
                    "Page has {h1_count} H1 headings. Use a single H1 for the main topic."
                ),
                url: url.to_string(),
                recommendation: "Use one H1 for the page title and H2+ for sections.".into(),
            });
        }
        let mut prev_level: Option<u8> = None;
        for heading in &ctx.page.headings {
            if let Some(prev) = prev_level {
                if heading.level > prev + 1 {
                    f.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Accessibility,
                        code: "A11Y005".to_string(),
                        title: "Skipped heading level".into(),
                        description: format!(
                            "Heading jumps from H{prev} to H{}, skipping intermediate levels.",
                            heading.level
                        ),
                        url: url.to_string(),
                        recommendation: format!(
                            "Use H{} after H{prev} to maintain document outline.",
                            prev + 1
                        ),
                    });
                    break;
                }
            }
            prev_level = Some(heading.level);
        }
    }

    fn check_landmarks(&self, ctx: &AnalysisContext, url: &str, f: &mut Vec<Finding>) {
        if !ctx.page.has_main_landmark {
            f.push(Finding {
                severity: Severity::Warning, category: IssueCategory::Accessibility,
                code: "A11Y006".to_string(), title: "Missing main landmark".into(),
                description: "No main element or role=main found. Screen readers use landmarks for page navigation.".into(),
                url: url.to_string(), recommendation: "Wrap primary content in a <main> element.".into(),
            });
        }
        if !ctx.page.has_nav_landmark {
            f.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Accessibility,
                code: "A11Y007".to_string(),
                title: "No navigation landmark".into(),
                description: "No nav element or role=navigation found.".into(),
                url: url.to_string(),
                recommendation: "Wrap navigation links in a <nav> element.".into(),
            });
        }
    }

    fn check_skip_link(&self, ctx: &AnalysisContext, url: &str, f: &mut Vec<Finding>) {
        if !ctx.page.has_skip_link && ctx.page.has_nav_landmark {
            f.push(Finding {
                severity: Severity::Warning, category: IssueCategory::Accessibility,
                code: "A11Y008".to_string(), title: "Missing skip navigation link".into(),
                description: "No skip-to-content link found. Keyboard users must tab through all navigation links to reach main content.".into(),
                url: url.to_string(), recommendation: "Add a skip link as the first focusable element pointing to the main content area.".into(),
            });
        }
    }

    fn check_link_text(&self, ctx: &AnalysisContext, url: &str, f: &mut Vec<Finding>) {
        for link in &ctx.page.links {
            let text_lower = link.text.trim().to_lowercase();
            let has_accessible_name = !text_lower.is_empty()
                || link
                    .aria_label
                    .as_ref()
                    .is_some_and(|l| !l.trim().is_empty())
                || link.img_alt.as_ref().is_some_and(|a| !a.trim().is_empty());
            if !has_accessible_name {
                f.push(Finding {
                    severity: Severity::Error, category: IssueCategory::Accessibility,
                    code: "A11Y009".to_string(), title: "Empty link text".into(),
                    description: format!("Link to {} has no text. Screen readers announce the URL, which is not descriptive.", link.href),
                    url: url.to_string(), recommendation: "Add descriptive text or an aria-label to the link.".into(),
                });
            } else if Self::VAGUE_LINK_TEXTS.contains(&text_lower.as_str()) {
                f.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Accessibility,
                    code: "A11Y010".to_string(),
                    title: "Non-descriptive link text".into(),
                    description: format!(
                        "Link text {} is vague and does not describe the destination.",
                        link.text
                    ),
                    url: url.to_string(),
                    recommendation: "Use descriptive text that explains the link purpose.".into(),
                });
            }
        }
    }

    fn check_form_labels(&self, ctx: &AnalysisContext, url: &str, f: &mut Vec<Finding>) {
        for form in &ctx.page.forms {
            for input in &form.inputs {
                if !input.has_label {
                    let desc = match (&input.name, &input.input_type) {
                        (Some(n), Some(t)) => format!("input (name={n}, type={t})"),
                        (Some(n), None) => format!("input (name={n})"),
                        (None, Some(t)) => format!("input (type={t})"),
                        (None, None) => "input".to_string(),
                    };
                    f.push(Finding {
                        severity: Severity::Error,
                        category: IssueCategory::Accessibility,
                        code: "A11Y011".to_string(),
                        title: "Form input missing label".into(),
                        description: format!(
                            "{desc} has no associated label, aria-label, or aria-labelledby."
                        ),
                        url: url.to_string(),
                        recommendation:
                            "Add a label element or an aria-label attribute to the input.".into(),
                    });
                }
            }
        }
    }

    fn check_keyboard_aria(&self, ctx: &AnalysisContext, url: &str, f: &mut Vec<Finding>) {
        if ctx.page.has_positive_tabindex {
            f.push(Finding {
                severity: Severity::Error, category: IssueCategory::Accessibility,
                code: "A11Y012".to_string(), title: "Positive tabindex values detected".into(),
                description: "Elements with tabindex > 0 alter the natural tab order, making keyboard navigation unpredictable.".into(),
                url: url.to_string(), recommendation: "Use tabindex=0 to add elements to the natural tab order or tabindex=-1 for programmatic focus only.".into(),
            });
        }
        if ctx.page.aria_role_count > 0 && ctx.page.aria_label_count == 0 {
            f.push(Finding {
                severity: Severity::Warning, category: IssueCategory::Accessibility,
                code: "A11Y013".to_string(), title: "ARIA roles without labels".into(),
                description: format!("{} ARIA role(s) found but no aria-label or aria-labelledby attributes. Custom roles require accessible names.", ctx.page.aria_role_count),
                url: url.to_string(), recommendation: "Add aria-label or aria-labelledby to elements with custom ARIA roles.".into(),
            });
        }
    }

    fn check_tables_lang(&self, ctx: &AnalysisContext, url: &str, f: &mut Vec<Finding>) {
        if ctx.page.tables_total > 0 {
            let without_headers = ctx.page.tables_total - ctx.page.tables_with_headers;
            if without_headers > 0 {
                f.push(Finding {
                    severity: Severity::Warning, category: IssueCategory::Accessibility,
                    code: "A11Y014".to_string(), title: "Table missing header cells".into(),
                    description: format!("{without_headers} of {} table(s) have no <th> header cells.", ctx.page.tables_total),
                    url: url.to_string(), recommendation: "Use <th> elements for header cells and add scope attributes for complex tables.".into(),
                });
            }
            let without_captions = ctx.page.tables_total - ctx.page.tables_with_captions;
            if without_captions > 0 {
                f.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Accessibility,
                    code: "A11Y015".to_string(),
                    title: "Table missing caption".into(),
                    description: format!(
                        "{without_captions} of {} table(s) have no <caption> element.",
                        ctx.page.tables_total
                    ),
                    url: url.to_string(),
                    recommendation: "Add a <caption> to describe the table purpose.".into(),
                });
            }
        }
        if !ctx.page.has_lang_attribute {
            f.push(Finding {
                severity: Severity::Error, category: IssueCategory::Accessibility,
                code: "A11Y016".to_string(), title: "Missing html lang attribute".into(),
                description: "The html element has no lang attribute. Screen readers use this to select the correct pronunciation engine.".into(),
                url: url.to_string(), recommendation: "Add lang=en (or the appropriate language code) to the html element.".into(),
            });
        }
    }
}

impl Default for AccessibilityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for AccessibilityAnalyzer {
    fn name(&self) -> &str {
        "accessibility"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut f = Vec::new();
        let url = &ctx.page.url;
        self.check_images_alt(ctx, url, &mut f);
        self.check_headings(ctx, url, &mut f);
        self.check_landmarks(ctx, url, &mut f);
        self.check_skip_link(ctx, url, &mut f);
        self.check_link_text(ctx, url, &mut f);
        self.check_form_labels(ctx, url, &mut f);
        self.check_keyboard_aria(ctx, url, &mut f);
        self.check_tables_lang(ctx, url, &mut f);
        f
    }
}

// ---------------------------------------------------------------------------
// Font Size Analyzer (WCAG 1.4.4)
// ---------------------------------------------------------------------------

pub struct FontSizeAnalyzer;

impl FontSizeAnalyzer {
    pub fn new() -> Self {
        Self
    }

    fn parse_font_size_px(value: &str) -> Option<f64> {
        let trimmed = value.trim().to_lowercase();
        if let Some(px) = trimmed.strip_suffix("px") {
            return px.trim().parse::<f64>().ok();
        }
        if let Some(pt) = trimmed.strip_suffix("pt") {
            return pt.trim().parse::<f64>().ok().map(|v| v * 96.0 / 72.0);
        }
        None
    }

    fn parse_line_height(value: &str) -> Option<f64> {
        let trimmed = value.trim();
        trimmed.parse::<f64>().ok()
    }

    fn extract_inline_font_sizes(html: &str) -> Vec<f64> {
        let re = Regex::new(r#"style\s*=\s*["'][^"']*font-size\s*:\s*([^;"']+)["']"#)
            .unwrap_or_else(|_| Regex::new("x^").unwrap());
        let mut sizes = Vec::new();
        for cap in re.captures_iter(html) {
            if let Some(px) = Self::parse_font_size_px(&cap[1]) {
                sizes.push(px);
            }
        }
        sizes
    }

    fn extract_style_block_font_sizes(html: &str) -> Vec<f64> {
        let re =
            Regex::new(r#"font-size\s*:\s*([^;}]+)"#).unwrap_or_else(|_| Regex::new("x^").unwrap());
        let mut sizes = Vec::new();
        for cap in re.captures_iter(html) {
            if let Some(px) = Self::parse_font_size_px(&cap[1]) {
                sizes.push(px);
            }
        }
        sizes
    }

    fn extract_inline_line_heights(html: &str) -> Vec<f64> {
        let re = Regex::new(r#"style\s*=\s*["'][^"']*line-height\s*:\s*([^;"']+)["']"#)
            .unwrap_or_else(|_| Regex::new("x^").unwrap());
        let mut heights = Vec::new();
        for cap in re.captures_iter(html) {
            if let Some(lh) = Self::parse_line_height(&cap[1]) {
                heights.push(lh);
            }
        }
        heights
    }

    fn extract_style_block_line_heights(html: &str) -> Vec<f64> {
        let re = Regex::new(r#"line-height\s*:\s*([^;}]+)"#)
            .unwrap_or_else(|_| Regex::new("x^").unwrap());
        let mut heights = Vec::new();
        for cap in re.captures_iter(html) {
            if let Some(lh) = Self::parse_line_height(&cap[1]) {
                heights.push(lh);
            }
        }
        heights
    }

    fn check_small_font_sizes(&self, ctx: &AnalysisContext, url: &str, f: &mut Vec<Finding>) {
        let body = ctx.body.unwrap_or("");
        let mut all_sizes: Vec<f64> = Self::extract_inline_font_sizes(body);
        all_sizes.extend(Self::extract_style_block_font_sizes(body));
        let small: Vec<f64> = all_sizes
            .into_iter()
            .filter(|&s| s > 0.0 && s < 12.0)
            .collect();
        if !small.is_empty() {
            let examples: Vec<String> = small.iter().take(5).map(|s| format!("{s:.0}px")).collect();
            f.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "FSIZE001".to_string(),
                title: "Text smaller than 12px detected".to_string(),
                description: format!(
                    "{} element(s) have font-size below 12px (e.g., {}). WCAG 1.4.4 requires \
                     text to be resizable up to 200% without loss of content or functionality.",
                    small.len(),
                    examples.join(", ")
                ),
                url: url.to_string(),
                recommendation: "Use a minimum font size of 12px (0.75rem) for body text. Use \
                                 relative units (rem, em) so text can be resized by the user."
                    .to_string(),
            });
        }
    }

    fn check_line_height(&self, ctx: &AnalysisContext, url: &str, f: &mut Vec<Finding>) {
        let body = ctx.body.unwrap_or("");
        let mut all_heights: Vec<f64> = Self::extract_inline_line_heights(body);
        all_heights.extend(Self::extract_style_block_line_heights(body));
        let insufficient: Vec<f64> = all_heights
            .into_iter()
            .filter(|&lh| lh > 0.0 && lh < 1.5)
            .collect();
        if !insufficient.is_empty() {
            let examples: Vec<String> = insufficient
                .iter()
                .take(5)
                .map(|lh| format!("{lh:.1}"))
                .collect();
            f.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "FSIZE002".to_string(),
                title: "Insufficient line-height for body text".to_string(),
                description: format!(
                    "{} element(s) have line-height below 1.5 (e.g., {}). WCAG 1.4.12 recommends \
                     a line-height of at least 1.5 times the font size for body text.",
                    insufficient.len(),
                    examples.join(", ")
                ),
                url: url.to_string(),
                recommendation: "Set line-height to at least 1.5 for body text and 1.5 times \
                                 the font size for headings."
                    .to_string(),
            });
        }
    }
}

impl Default for FontSizeAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for FontSizeAnalyzer {
    fn name(&self) -> &str {
        "font-size"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut f = Vec::new();
        let url = &ctx.page.url;
        self.check_small_font_sizes(ctx, url, &mut f);
        self.check_line_height(ctx, url, &mut f);
        f
    }
}

// ---------------------------------------------------------------------------
// Color Contrast Analyzer (WCAG 1.4.3)
// ---------------------------------------------------------------------------

pub struct ColorContrastAnalyzer;

impl ColorContrastAnalyzer {
    pub fn new() -> Self {
        Self
    }

    fn parse_hex_color(hex: &str) -> Option<(u8, u8, u8)> {
        let hex = hex.trim().trim_start_matches('#');
        match hex.len() {
            3 => {
                let r = u8::from_str_radix(&hex[0..1], 16).ok()?;
                let g = u8::from_str_radix(&hex[1..2], 16).ok()?;
                let b = u8::from_str_radix(&hex[2..3], 16).ok()?;
                Some((r * 17, g * 17, b * 17))
            }
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                Some((r, g, b))
            }
            _ => None,
        }
    }

    fn parse_named_color(name: &str) -> Option<(u8, u8, u8)> {
        match name.trim().to_lowercase().as_str() {
            "black" => Some((0, 0, 0)),
            "white" => Some((255, 255, 255)),
            "red" => Some((255, 0, 0)),
            "green" => Some((0, 128, 0)),
            "blue" => Some((0, 0, 255)),
            "yellow" => Some((255, 255, 0)),
            "gray" | "grey" => Some((128, 128, 128)),
            "silver" => Some((192, 192, 192)),
            "navy" => Some((0, 0, 128)),
            "maroon" => Some((128, 0, 0)),
            "olive" => Some((128, 128, 0)),
            "teal" => Some((0, 128, 128)),
            "aqua" | "cyan" => Some((0, 255, 255)),
            "fuchsia" | "magenta" => Some((255, 0, 255)),
            "lime" => Some((0, 255, 0)),
            "orange" => Some((255, 165, 0)),
            "pink" => Some((255, 192, 203)),
            "purple" => Some((128, 0, 128)),
            _ => None,
        }
    }

    fn parse_rgb_function(val: &str) -> Option<(u8, u8, u8)> {
        let re = Regex::new(r"rgb\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*\)")
            .unwrap_or_else(|_| Regex::new("x^").unwrap());
        let caps = re.captures(val)?;
        let r: u8 = caps[1].parse().ok()?;
        let g: u8 = caps[2].parse().ok()?;
        let b: u8 = caps[3].parse().ok()?;
        Some((r, g, b))
    }

    fn parse_color_value(val: &str) -> Option<(u8, u8, u8)> {
        let trimmed = val.trim();
        if trimmed.starts_with('#') {
            return Self::parse_hex_color(trimmed);
        }
        if trimmed.starts_with("rgb(") {
            return Self::parse_rgb_function(trimmed);
        }
        Self::parse_named_color(trimmed)
    }

    fn relative_luminance(r: u8, g: u8, b: u8) -> f64 {
        let fn_channel = |c: u8| -> f64 {
            let s = c as f64 / 255.0;
            if s <= 0.03928 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * fn_channel(r) + 0.7152 * fn_channel(g) + 0.0722 * fn_channel(b)
    }

    pub(crate) fn contrast_ratio(fg: (u8, u8, u8), bg: (u8, u8, u8)) -> f64 {
        let l1 = Self::relative_luminance(fg.0, fg.1, fg.2);
        let l2 = Self::relative_luminance(bg.0, bg.1, bg.2);
        let lighter = l1.max(l2);
        let darker = l1.min(l2);
        (lighter + 0.05) / (darker + 0.05)
    }

    fn extract_color_pairs(html: &str) -> Vec<((u8, u8, u8), (u8, u8, u8))> {
        let re = Regex::new(
            r#"style\s*=\s*["'][^"']*color\s*:\s*([^;"']+)[^"']*background(?:-color)?\s*:\s*([^;"']+)["']"#,
        )
        .unwrap_or_else(|_| Regex::new("x^").unwrap());
        let mut pairs = Vec::new();
        for cap in re.captures_iter(html) {
            if let (Some(fg), Some(bg)) = (
                Self::parse_color_value(&cap[1]),
                Self::parse_color_value(&cap[2]),
            ) {
                pairs.push((fg, bg));
            }
        }

        let re2 = Regex::new(
            r#"style\s*=\s*["'][^"']*background(?:-color)?\s*:\s*([^;"']+)[^"']*color\s*:\s*([^;"']+)["']"#,
        )
        .unwrap_or_else(|_| Regex::new("x^").unwrap());
        for cap in re2.captures_iter(html) {
            if let (Some(bg), Some(fg)) = (
                Self::parse_color_value(&cap[1]),
                Self::parse_color_value(&cap[2]),
            ) {
                pairs.push((fg, bg));
            }
        }
        pairs
    }
}

impl Default for ColorContrastAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for ColorContrastAnalyzer {
    fn name(&self) -> &str {
        "color-contrast"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let body = ctx.body.unwrap_or("");

        let pairs = Self::extract_color_pairs(body);
        let mut low_contrast_count = 0;
        let mut similar_count = 0;

        for (fg, bg) in &pairs {
            let ratio = Self::contrast_ratio(*fg, *bg);
            if ratio < 3.0 {
                similar_count += 1;
            } else if ratio < 4.5 {
                low_contrast_count += 1;
            }
        }

        if similar_count > 0 {
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Accessibility,
                code: "CONTR001".to_string(),
                title: "Text color too similar to background color".to_string(),
                description: format!(
                    "{} inline style(s) have a contrast ratio below 3:1, making text \
                     extremely difficult to read. WCAG 1.4.3 requires a minimum contrast ratio \
                     of 4.5:1 for normal text.",
                    similar_count
                ),
                url: url.to_string(),
                recommendation: "Ensure text color contrasts sufficiently with its background. \
                                 Use a contrast checker tool to verify WCAG AA compliance."
                    .to_string(),
            });
        }

        if low_contrast_count > 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "CONTR002".to_string(),
                title: "Low color contrast ratio (below 4.5:1)".to_string(),
                description: format!(
                    "{} inline style(s) have a contrast ratio between 3:1 and 4.5:1. WCAG 1.4.3 \
                     requires at least 4.5:1 for normal text and 3:1 for large text.",
                    low_contrast_count
                ),
                url: url.to_string(),
                recommendation: "Increase the contrast ratio to at least 4.5:1 for normal text \
                                 and 3:1 for large text (18px+ or 14px bold)."
                    .to_string(),
            });
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// Focus Order Analyzer
// ---------------------------------------------------------------------------

pub struct FocusOrderAnalyzer;

impl FocusOrderAnalyzer {
    pub fn new() -> Self {
        Self
    }

    fn check_positive_tabindex(&self, ctx: &AnalysisContext, url: &str, f: &mut Vec<Finding>) {
        if ctx.page.has_positive_tabindex {
            f.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Accessibility,
                code: "FOCUS001".to_string(),
                title: "Positive tabindex values disrupt tab order".to_string(),
                description: "Elements with tabindex > 0 alter the natural tab order, making \
                              keyboard navigation unpredictable. Users expect a sequential tab \
                              flow matching the visual layout."
                    .to_string(),
                url: url.to_string(),
                recommendation: "Remove positive tabindex values. Use tabindex=\"0\" to add \
                                 elements to the natural tab order or tabindex=\"-1\" for \
                                 programmatic focus only."
                    .to_string(),
            });
        }
    }

    fn check_focus_styles(&self, ctx: &AnalysisContext, url: &str, f: &mut Vec<Finding>) {
        let body = ctx.body.unwrap_or("");
        let has_focus_style = body.contains(":focus")
            || body.contains(":focus-visible")
            || body.contains(":focus-within");
        let interactive_count = ctx
            .page
            .links
            .iter()
            .filter(|l| !l.text.trim().is_empty())
            .count()
            + ctx.page.forms.len();

        if interactive_count > 0 && !has_focus_style {
            f.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "FOCUS002".to_string(),
                title: "No visible focus indicators found".to_string(),
                description: format!(
                    "Page has {} interactive element(s) but no :focus or :focus-visible CSS \
                     rules were detected. Keyboard users rely on visible focus indicators to \
                     know which element is active.",
                    interactive_count
                ),
                url: url.to_string(),
                recommendation: "Add :focus and/or :focus-visible CSS rules with a visible \
                                 outline or background change. Ensure the indicator has \
                                 sufficient contrast (3:1 minimum)."
                    .to_string(),
            });
        }
    }
}

impl Default for FocusOrderAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for FocusOrderAnalyzer {
    fn name(&self) -> &str {
        "focus-order"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut f = Vec::new();
        let url = &ctx.page.url;
        self.check_positive_tabindex(ctx, url, &mut f);
        self.check_focus_styles(ctx, url, &mut f);
        f
    }
}

// ---------------------------------------------------------------------------
// HSTS Preload Analyzer
// ---------------------------------------------------------------------------

// HstsPreloadAnalyzer extracted to hsts_analyzer.rs (Phase 2 SRP step).

// SRI, PermissionPolicy, CrossOriginIsolation, CSP, ReferrerPolicy,
// XFrameOptions, MixedContent analyzers extracted to
// security_header_analyzers.rs (Phase 2 SRP step).

// {name} extracted to cookies.rs (Phase 2 SRP step).

// =========================================================================
// XContentTypeOptionsAnalyzer, XPermittedCrossDomainPoliciesAnalyzer, and
// CrossOriginResourcePolicyAnalyzer extracted to x_header_analyzers.rs
// (Phase 2 SRP step).

// ---------------------------------------------------------------------------
// Landmark Regions Analyzer (WCAG landmark navigation)
// ---------------------------------------------------------------------------

pub struct LandmarkRegionsAnalyzer;

impl LandmarkRegionsAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LandmarkRegionsAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for LandmarkRegionsAnalyzer {
    fn name(&self) -> &str {
        "landmark-regions"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        if !ctx.page.has_main_landmark {
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Accessibility,
                code: "LAND001".to_string(),
                title: "Missing main landmark region".to_string(),
                description: "No <main> element or role=\"main\" found. Screen reader users rely on landmark regions to quickly navigate to the primary content of a page."
                    .to_string(),
                url: url.to_string(),
                recommendation: "Wrap the primary page content in a <main> element or add role=\"main\" to the primary content container."
                    .to_string(),
            });
        }

        if !ctx.page.has_nav_landmark {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "LAND002".to_string(),
                title: "Missing navigation landmark".to_string(),
                description: "No <nav> element or role=\"navigation\" found. Navigation landmarks allow screen reader users to jump directly to site navigation."
                    .to_string(),
                url: url.to_string(),
                recommendation: "Wrap primary navigation links in a <nav> element or add role=\"navigation\" to the navigation container."
                    .to_string(),
            });
        }

        let has_banner = ctx
            .page
            .landmarks
            .iter()
            .any(|l| l == "banner" || l == "header");
        if !has_banner {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Accessibility,
                code: "LAND003".to_string(),
                title: "Missing banner/header landmark".to_string(),
                description: "No <header> element or role=\"banner\" found. The banner landmark typically contains the site logo, search, and primary navigation."
                    .to_string(),
                url: url.to_string(),
                recommendation: "Wrap the site header in a <header> element. For the banner role, ensure it is a direct child of <body>."
                    .to_string(),
            });
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// Heading Order Analyzer (WCAG 1.3.1 heading hierarchy)
// ---------------------------------------------------------------------------

pub struct HeadingOrderAnalyzer;

impl HeadingOrderAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for HeadingOrderAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for HeadingOrderAnalyzer {
    fn name(&self) -> &str {
        "heading-order"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        if ctx.page.headings.len() < 2 {
            return findings;
        }

        let mut prev_level: Option<u8> = None;
        let mut found_descent = false;

        for heading in &ctx.page.headings {
            if let Some(prev) = prev_level {
                if heading.level > prev + 1 {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Accessibility,
                        code: "HORDER001".to_string(),
                        title: "Heading level skip detected".to_string(),
                        description: format!(
                            "Heading jumps from H{prev} to H{}, skipping intermediate levels. \
                             Screen readers and outline tools rely on sequential heading levels.",
                            heading.level
                        ),
                        url: url.to_string(),
                        recommendation: format!(
                            "Use H{} after H{prev} to maintain a proper document outline.",
                            prev + 1
                        ),
                    });
                }
                if heading.level < prev && !found_descent {
                    found_descent = true;
                }
                if found_descent && heading.level > prev {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Accessibility,
                        code: "HORDER002".to_string(),
                        title: "Non-sequential heading order".to_string(),
                        description: format!(
                            "Heading level decreased from H{prev} to H{} and then increased again. \
                             Heading levels should follow a strictly non-increasing pattern within sections.",
                            heading.level
                        ),
                        url: url.to_string(),
                        recommendation: "Ensure heading levels descend sequentially (H1 > H2 > H3) and do not increase within a section."
                            .to_string(),
                    });
                    break;
                }
            }
            prev_level = Some(heading.level);
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// Form Label Analyzer (WCAG 1.3.1, 4.1.2 form accessibility)
// ---------------------------------------------------------------------------

pub struct FormLabelAnalyzer;

impl FormLabelAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FormLabelAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for FormLabelAnalyzer {
    fn name(&self) -> &str {
        "form-labels"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for form in &ctx.page.forms {
            for input in &form.inputs {
                if !input.has_label {
                    let aria_has_name = input
                        .aria_label
                        .as_ref()
                        .is_some_and(|l| !l.trim().is_empty())
                        || input
                            .aria_labelledby
                            .as_ref()
                            .is_some_and(|l| !l.trim().is_empty());
                    if !aria_has_name {
                        let desc = match (&input.name, &input.input_type) {
                            (Some(n), Some(t)) => format!("input (name=\"{n}\", type=\"{t}\")"),
                            (Some(n), None) => format!("input (name=\"{n}\")"),
                            (None, Some(t)) => format!("input (type=\"{t}\")"),
                            (None, None) => "input".to_string(),
                        };
                        findings.push(Finding {
                            severity: Severity::Error,
                            category: IssueCategory::Accessibility,
                            code: "FLABEL001".to_string(),
                            title: "Form input missing associated label".to_string(),
                            description: format!(
                                "{desc} has no associated <label> element, aria-label, or aria-labelledby attribute. \
                                 Screen readers cannot announce the purpose of unlabeled inputs."
                            ),
                            url: url.to_string(),
                            recommendation: "Associate a <label> element with the input using the for/id attributes, or add an aria-label attribute."
                                .to_string(),
                        });
                    }
                } else if let Some(placeholder) = &input.placeholder {
                    if !placeholder.trim().is_empty() && input.aria_label.is_none() {
                        let label_text = input.name.as_deref().unwrap_or("input");
                        if label_text.trim().is_empty() {
                            findings.push(Finding {
                                severity: Severity::Info,
                                category: IssueCategory::Accessibility,
                                code: "FLABEL002".to_string(),
                                title: "Form input with empty label text".to_string(),
                                description: format!(
                                    "input (name=\"{}\") has a <label> element but the label text may be empty. \
                                     Placeholder text is not a substitute for a proper label.",
                                    input.name.as_deref().unwrap_or("")
                                ),
                                url: url.to_string(),
                                recommendation: "Ensure the <label> element contains descriptive text explaining the input purpose."
                                    .to_string(),
                            });
                        }
                    }
                }
            }
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// Table Accessibility Analyzer (WCAG 1.3.1 table semantics)
// ---------------------------------------------------------------------------

pub struct TableAccessibilityAnalyzer;

impl TableAccessibilityAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TableAccessibilityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for TableAccessibilityAnalyzer {
    fn name(&self) -> &str {
        "table-accessibility"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        if ctx.page.tables_total == 0 {
            return findings;
        }

        let without_headers = ctx
            .page
            .tables_total
            .saturating_sub(ctx.page.tables_with_headers);
        if without_headers > 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "TACC001".to_string(),
                title: "Table missing header cells".to_string(),
                description: format!(
                    "{} of {} table(s) have no <th> header cells. Header cells help screen reader \
                     users understand the structure and relationships of tabular data.",
                    without_headers, ctx.page.tables_total
                ),
                url: url.to_string(),
                recommendation: "Use <th> elements for header cells and add scope=\"col\" or scope=\"row\" attributes for complex tables."
                    .to_string(),
            });
        }

        let without_captions = ctx
            .page
            .tables_total
            .saturating_sub(ctx.page.tables_with_captions);
        if without_captions > 0 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Accessibility,
                code: "TACC002".to_string(),
                title: "Table missing caption".to_string(),
                description: format!(
                    "{} of {} table(s) have no <caption> element. Captions provide a summary \
                     of the table purpose for screen reader users.",
                    without_captions, ctx.page.tables_total
                ),
                url: url.to_string(),
                recommendation: "Add a <caption> element to each data table describing its content."
                    .to_string(),
            });
        }

        let tables_needing_scope = ctx
            .page
            .tables_total
            .saturating_sub(ctx.page.tables_with_headers);
        if tables_needing_scope > 10 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "TACC003".to_string(),
                title: "Large number of tables missing scope attributes".to_string(),
                description: format!(
                    "{} table(s) with more than 10 rows are missing scope attributes on header cells. \
                     The scope attribute clarifies whether a header applies to a row or column.",
                    tables_needing_scope
                ),
                url: url.to_string(),
                recommendation: "Add scope=\"col\" to column headers and scope=\"row\" to row headers."
                    .to_string(),
            });
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// Link Accessibility Analyzer (WCAG 2.4.4 link purpose)
// ---------------------------------------------------------------------------

pub struct LinkAccessibilityAnalyzer;

impl LinkAccessibilityAnalyzer {
    pub fn new() -> Self {
        Self
    }

    const GENERIC_TEXTS: &[&str] = &[
        "click here",
        "read more",
        "more",
        "learn more",
        "click",
        "go",
        "continue",
    ];

    const NON_DESCRIPTIVE_TEXTS: &[&str] = &["link", "here"];
}

impl Default for LinkAccessibilityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for LinkAccessibilityAnalyzer {
    fn name(&self) -> &str {
        "link-accessibility"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for link in &ctx.page.links {
            let text_lower = link.text.trim().to_lowercase();
            let has_accessible_name = !text_lower.is_empty()
                || link
                    .aria_label
                    .as_ref()
                    .is_some_and(|l| !l.trim().is_empty())
                || link.img_alt.as_ref().is_some_and(|a| !a.trim().is_empty());

            if !has_accessible_name {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Accessibility,
                    code: "LNKACC001".to_string(),
                    title: "Link with empty text content".to_string(),
                    description: format!(
                        "Link to \"{}\" has no accessible text. Screen readers announce the raw URL, \
                         which is not descriptive for users.",
                        link.href
                    ),
                    url: url.to_string(),
                    recommendation: "Add descriptive text content, an aria-label, or an image with alt text inside the link."
                        .to_string(),
                });
            } else if Self::GENERIC_TEXTS.contains(&text_lower.as_str()) {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Accessibility,
                    code: "LNKACC002".to_string(),
                    title: "Link with generic text".to_string(),
                    description: format!(
                        "Link text \"{}\" is generic and does not describe the destination. \
                         Screen reader users navigating by links hear a list of identical labels.",
                        link.text.trim()
                    ),
                    url: url.to_string(),
                    recommendation: "Use descriptive link text that explains the purpose or destination of the link."
                        .to_string(),
                });
            } else if Self::NON_DESCRIPTIVE_TEXTS.contains(&text_lower.as_str()) {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Accessibility,
                    code: "LNKACC003".to_string(),
                    title: "Link with non-descriptive text".to_string(),
                    description: format!(
                        "Link text \"{}\" is too short to convey meaning. Users navigating by links \
                         cannot determine the destination.",
                        link.text.trim()
                    ),
                    url: url.to_string(),
                    recommendation: "Replace the link text with a phrase that describes the link destination."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// Image Accessibility Analyzer (WCAG 1.1.1 non-text content)
// ---------------------------------------------------------------------------

pub struct ImageAccessibilityAnalyzer;

impl ImageAccessibilityAnalyzer {
    pub fn new() -> Self {
        Self
    }

    fn filename_from_src(src: &str) -> Option<&str> {
        src.rsplit('/').next().and_then(|s| {
            let s = s.trim_start_matches('\\');
            if s.is_empty() {
                None
            } else {
                Some(s)
            }
        })
    }
}

impl Default for ImageAccessibilityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for ImageAccessibilityAnalyzer {
    fn name(&self) -> &str {
        "image-accessibility"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for img in &ctx.page.images {
            if !img.has_alt {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Accessibility,
                    code: "IMGACC001".to_string(),
                    title: "Image missing alt attribute".to_string(),
                    description: format!(
                        "Image \"{}\" has no alt attribute. Screen readers cannot convey \
                         the image content to visually impaired users.",
                        img.src
                    ),
                    url: url.to_string(),
                    recommendation: "Add an alt attribute to every <img> element. Use descriptive text for meaningful images and alt=\"\" for decorative ones."
                        .to_string(),
                });
            } else if img.alt.is_empty() && !img.aria_hidden {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Accessibility,
                    code: "IMGACC002".to_string(),
                    title: "Image with empty alt on non-decorative image".to_string(),
                    description: format!(
                        "Image \"{}\" has alt=\"\" but is not marked as aria-hidden. If this image \
                         conveys meaningful content, it needs descriptive alt text. If decorative, add aria-hidden=\"true\".",
                        img.src
                    ),
                    url: url.to_string(),
                    recommendation: "For meaningful images, provide descriptive alt text. For decorative images, use alt=\"\" AND aria-hidden=\"true\"."
                        .to_string(),
                });
            } else if !img.alt.is_empty() {
                if let Some(filename) = Self::filename_from_src(&img.src) {
                    let filename_no_ext = filename.split('.').next().unwrap_or(filename);
                    let alt_lower = img.alt.trim().to_lowercase();
                    let filename_lower = filename_no_ext.to_lowercase();
                    if alt_lower == filename_lower {
                        findings.push(Finding {
                            severity: Severity::Warning,
                            category: IssueCategory::Accessibility,
                            code: "IMGACC003".to_string(),
                            title: "Image alt text identical to filename".to_string(),
                            description: format!(
                                "Image \"{}\" has alt text \"{}\" which matches the filename. \
                                 Alt text should describe the image content, not repeat the file name.",
                                img.src, img.alt
                            ),
                            url: url.to_string(),
                            recommendation: "Replace the filename-based alt text with a description of what the image shows."
                                .to_string(),
                        });
                    }
                }
            }
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// ARIA Roles Analyzer (WCAG 4.1.2 name, role, value)
// ---------------------------------------------------------------------------

pub struct AriaRolesAnalyzer;

impl AriaRolesAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AriaRolesAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for AriaRolesAnalyzer {
    fn name(&self) -> &str {
        "aria-roles"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        if ctx.page.aria_role_count > 0 && ctx.page.aria_label_count == 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "ARIA001".to_string(),
                title: "ARIA roles without accessible names".to_string(),
                description: format!(
                    "{} ARIA role(s) found but no aria-label or aria-labelledby attributes. \
                     Custom ARIA roles require accessible names so screen readers can announce \
                     the element purpose.",
                    ctx.page.aria_role_count
                ),
                url: url.to_string(),
                recommendation:
                    "Add aria-label or aria-labelledby to all elements with custom ARIA roles."
                        .to_string(),
            });
        }

        if ctx.page.aria_role_count > 0
            && ctx.page.aria_label_count > 0
            && ctx.page.aria_role_count > ctx.page.aria_label_count
        {
            let unlabeled = ctx
                .page
                .aria_role_count
                .saturating_sub(ctx.page.aria_label_count);
            if unlabeled > 0 {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Accessibility,
                    code: "ARIA002".to_string(),
                    title: "ARIA roles may need accessible names on non-semantic elements".to_string(),
                    description: format!(
                        "{} ARIA role(s) are used but not all have associated accessible names. \
                         When adding ARIA roles to non-semantic elements like <div> or <span>, \
                         ensure each has an aria-label or aria-labelledby.",
                        unlabeled
                    ),
                    url: url.to_string(),
                    recommendation: "Every element with a role attribute should have an accessible name via aria-label, aria-labelledby, or visible text content."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// Focus Management Analyzer (WCAG 2.4.3 focus order)
// ---------------------------------------------------------------------------

pub struct FocusManagementAnalyzer;

impl FocusManagementAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FocusManagementAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for FocusManagementAnalyzer {
    fn name(&self) -> &str {
        "focus-management"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        if ctx.page.has_positive_tabindex {
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Accessibility,
                code: "FOCUS001".to_string(),
                title: "Positive tabindex values disrupt focus order".to_string(),
                description: "Elements with tabindex > 0 alter the natural tab order, causing \
                              keyboard navigation to skip elements or follow an unpredictable sequence. \
                              This violates WCAG 2.4.3 Focus Order."
                    .to_string(),
                url: url.to_string(),
                recommendation: "Remove positive tabindex values. Use tabindex=\"0\" to add elements to the natural tab order or tabindex=\"-1\" for programmatic focus only."
                    .to_string(),
            });
        }

        let body = ctx.body.unwrap_or("");
        let has_focus_style = body.contains(":focus")
            || body.contains(":focus-visible")
            || body.contains(":focus-within");
        let interactive_count = ctx
            .page
            .links
            .iter()
            .filter(|l| !l.text.trim().is_empty())
            .count()
            + ctx.page.forms.len();

        if interactive_count > 0 && !has_focus_style {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "FOCUS002".to_string(),
                title: "No visible focus indicators found".to_string(),
                description: format!(
                    "Page has {} interactive element(s) but no :focus or :focus-visible CSS rules \
                     were detected. Keyboard users rely on visible focus indicators to know which \
                     element is active.",
                    interactive_count
                ),
                url: url.to_string(),
                recommendation: "Add :focus and/or :focus-visible CSS rules with a visible outline or background change. Ensure the indicator has sufficient contrast (3:1 minimum)."
                    .to_string(),
            });
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// Language Attribute Analyzer (WCAG 3.1.1 language of page)
// ---------------------------------------------------------------------------

pub struct LanguageAttributeAnalyzer;

impl LanguageAttributeAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LanguageAttributeAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for LanguageAttributeAnalyzer {
    fn name(&self) -> &str {
        "language-attribute"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        if !ctx.page.has_lang_attribute {
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Accessibility,
                code: "LANGACC001".to_string(),
                title: "Missing html lang attribute".to_string(),
                description: "The <html> element has no lang attribute. Screen readers use this \
                              attribute to select the correct pronunciation engine and hyphenation rules."
                    .to_string(),
                url: url.to_string(),
                recommendation: "Add lang=\"en\" (or the appropriate language code) to the <html> element."
                    .to_string(),
            });
        }

        if let Some(lang) = &ctx.page.html_lang {
            if lang.len() < 2 {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Accessibility,
                    code: "LANGACC002".to_string(),
                    title: "Lang attribute value too short".to_string(),
                    description: format!(
                        "The html lang attribute is set to \"{}\", which is shorter than the minimum \
                         2-character language code. Valid examples: \"en\", \"fr\", \"de\", \"zh-CN\".",
                        lang
                    ),
                    url: url.to_string(),
                    recommendation: "Use a valid BCP 47 language tag (e.g., \"en\", \"fr-CA\", \"zh-CN\")."
                        .to_string(),
                });
            }

            let has_hreflang = ctx.page.meta.hreflang.iter().any(|h| h.lang == *lang);
            let has_content = ctx.page.word_count > 0;
            if has_content && !has_hreflang && !ctx.page.meta.hreflang.is_empty() {
                let hreflang_langs: Vec<&str> = ctx
                    .page
                    .meta
                    .hreflang
                    .iter()
                    .map(|h| h.lang.as_str())
                    .collect();
                if !hreflang_langs.contains(&lang.as_str()) {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Accessibility,
                        code: "LANGACC002".to_string(),
                        title: "Lang attribute doesn't match hreflang declarations".to_string(),
                        description: format!(
                            "The html lang=\"{}\" but hreflang tags declare: {}. The page language \
                             should match one of the declared hreflang values.",
                            lang,
                            hreflang_langs.join(", ")
                        ),
                        url: url.to_string(),
                        recommendation: "Ensure the html lang attribute matches the content language declared in hreflang tags."
                            .to_string(),
                    });
                }
            }
        }

        findings
    }
}
// StrictTransportSecurityAnalyzer, CrossOriginOpenerPolicyAnalyzer,
// CrossOriginEmbedderPolicyAnalyzer, FeaturePolicyAnalyzer, ExpectCTAnalyzer, and
// CertificateTransparencyAnalyzer extracted to sts_analyzers.rs (Phase 2 SRP step).
// StrictTransportSecurityAnalyzer, XSSProtectionAnalyzer, ContentTypeSniffingAnalyzer,
// PermissionsPolicyAnalyzerNew, CrossOriginEmbedderPolicyAnalyzer, CrossOriginOpenerPolicyAnalyzer,
// FeaturePolicyAnalyzer, ExpectCTAnalyzer, and CertificateTransparencyAnalyzer
// extracted to sts_analyzers.rs (Phase 2 SRP step).

// CorsPolicyAnalyzer extracted to cors_analyzers.rs (Phase 2 SRP step).

// {name} extracted to cookies.rs (Phase 2 SRP step).

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

// =========================================================================
// XContentTypeOptionsDeepAnalyzer
// =========================================================================

pub struct XContentTypeOptionsDeepAnalyzer;

impl Default for XContentTypeOptionsDeepAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl XContentTypeOptionsDeepAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for XContentTypeOptionsDeepAnalyzer {
    fn name(&self) -> &str {
        "x-content-type-options-deep"
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
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Security, code: "XCTODEEP001".to_string(), title: "Missing X-Content-Type-Options header".to_string(), description: "No X-Content-Type-Options header found. Without nosniff, browsers may MIME-sniff responses.".to_string(), url: url.clone(), recommendation: "Add X-Content-Type-Options: nosniff.".to_string() });
            }
            Some(val) if !val.eq_ignore_ascii_case("nosniff") => {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "XCTODEEP002".to_string(),
                    title: "Invalid X-Content-Type-Options value".to_string(),
                    description: format!(
                        "X-Content-Type-Options is \"{val}\" but should be \"nosniff\"."
                    ),
                    url: url.clone(),
                    recommendation: "Set X-Content-Type-Options to nosniff.".to_string(),
                });
            }
            _ => {}
        }
        findings
    }
}

// =========================================================================
// ReferrerPolicyDeepAnalyzer
// =========================================================================

pub struct ReferrerPolicyDeepAnalyzer;

impl Default for ReferrerPolicyDeepAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl ReferrerPolicyDeepAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for ReferrerPolicyDeepAnalyzer {
    fn name(&self) -> &str {
        "referrer-policy-deep"
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
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Security, code: "RPDEEP001".to_string(), title: "Missing Referrer-Policy header".to_string(), description: "No Referrer-Policy header found. Browser default may leak referrer information.".to_string(), url: url.clone(), recommendation: "Add Referrer-Policy: strict-origin-when-cross-origin.".to_string() });
            }
            Some(val) if val.eq_ignore_ascii_case("unsafe-url") => {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Security, code: "RPDEEP002".to_string(), title: "Referrer-Policy set to unsafe-url".to_string(), description: "Referrer-Policy is 'unsafe-url', leaking full URL including path and query on cross-origin requests.".to_string(), url: url.clone(), recommendation: "Use strict-origin-when-cross-origin instead of unsafe-url.".to_string() });
            }
            _ => {}
        }
        findings
    }
}

// =========================================================================
// XFrameOptionsDeepAnalyzer
// =========================================================================

pub struct XFrameOptionsDeepAnalyzer;

impl Default for XFrameOptionsDeepAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl XFrameOptionsDeepAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for XFrameOptionsDeepAnalyzer {
    fn name(&self) -> &str {
        "x-frame-options-deep"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let xfo = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("X-Frame-Options"))
            .map(|(_, v)| v.as_str());
        let csp_frame = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Content-Security-Policy"))
            .map(|(_, v)| v.as_str())
            .unwrap_or("")
            .contains("frame-ancestors");

        match xfo {
            None => {
                if !csp_frame {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Security,
                        code: "XFODEEP001".to_string(),
                        title: "No clickjacking protection".to_string(),
                        description: "Neither X-Frame-Options nor CSP frame-ancestors is set."
                            .to_string(),
                        url: url.clone(),
                        recommendation:
                            "Add X-Frame-Options: DENY or CSP frame-ancestors directive."
                                .to_string(),
                    });
                }
            }
            Some(val) => {
                let upper = val.to_uppercase().trim().to_string();
                if upper != "DENY" && upper != "SAMEORIGIN" {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Security,
                        code: "XFODEEP002".to_string(),
                        title: "Invalid X-Frame-Options value".to_string(),
                        description: format!(
                            "X-Frame-Options is \"{val}\", must be DENY or SAMEORIGIN."
                        ),
                        url: url.clone(),
                        recommendation: "Set X-Frame-Options to DENY or SAMEORIGIN.".to_string(),
                    });
                }
            }
        }
        findings
    }
}

// =========================================================================
// PermissionsPolicyDeepAnalyzer
// =========================================================================

pub struct PermissionsPolicyDeepAnalyzer;

impl Default for PermissionsPolicyDeepAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl PermissionsPolicyDeepAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for PermissionsPolicyDeepAnalyzer {
    fn name(&self) -> &str {
        "permissions-policy-deep"
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
        match pp {
            None => {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Security, code: "PERMPDEEP001".to_string(), title: "Missing Permissions-Policy header".to_string(), description: "No Permissions-Policy header found. Browsers may allow access to sensitive APIs.".to_string(), url: url.clone(), recommendation: "Add Permissions-Policy to restrict sensitive browser features.".to_string() });
            }
            Some(val) => {
                let lower = val.to_lowercase();
                for feature in &["camera", "microphone", "geolocation", "payment"] {
                    if !lower.contains(feature) {
                        findings.push(Finding { severity: Severity::Info, category: IssueCategory::Security, code: "PERMPDEEP002".to_string(), title: format!("Permissions-Policy missing {feature}"), description: format!("The {feature} feature is not explicitly restricted in Permissions-Policy."), url: url.clone(), recommendation: format!("Add {feature}=() to restrict {feature} access.") });
                    }
                }
            }
        }
        findings
    }
}

// =========================================================================
// CrossOriginIsolationDeepAnalyzer
// =========================================================================

pub struct CrossOriginIsolationDeepAnalyzer;

impl Default for CrossOriginIsolationDeepAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl CrossOriginIsolationDeepAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for CrossOriginIsolationDeepAnalyzer {
    fn name(&self) -> &str {
        "cross-origin-isolation-deep"
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
                code: "COISODEEP001".to_string(),
                title: "Missing Cross-Origin-Embedder-Policy".to_string(),
                description: "COEP header is not set, preventing cross-origin isolation."
                    .to_string(),
                url: url.clone(),
                recommendation:
                    "Add Cross-Origin-Embedder-Policy: require-corp for cross-origin isolation."
                        .to_string(),
            });
        }
        if coop.is_none() {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Security,
                code: "COISODEEP002".to_string(),
                title: "Missing Cross-Origin-Opener-Policy".to_string(),
                description: "COOP header is not set.".to_string(),
                url: url.clone(),
                recommendation:
                    "Add Cross-Origin-Opener-Policy: same-origin for cross-origin isolation."
                        .to_string(),
            });
        }

        if let (Some(coep_val), Some(coop_val)) = (coep, coop) {
            if coep_val.eq_ignore_ascii_case("require-corp")
                && coop_val.eq_ignore_ascii_case("same-origin")
            {
                // Full cross-origin isolation achieved
            } else {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Security, code: "COISODEEP003".to_string(), title: "Partial cross-origin isolation".to_string(), description: format!("COEP={coep_val}, COOP={coop_val}. Full isolation requires COEP=require-corp and COOP=same-origin."), url: url.clone(), recommendation: "Set COEP=require-corp and COOP=same-origin for full cross-origin isolation.".to_string() });
            }
        }

        findings
    }
}

// =========================================================================
// AriaLandmarksAnalyzer
// =========================================================================

pub struct AriaLandmarksAnalyzer;

impl Default for AriaLandmarksAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl AriaLandmarksAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for AriaLandmarksAnalyzer {
    fn name(&self) -> &str {
        "aria-landmarks"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        if !ctx.page.has_main_landmark {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "ARIALAND001".to_string(),
                title: "Missing main landmark".to_string(),
                description: "No <main> or role=\"main\" landmark found.".to_string(),
                url: url.clone(),
                recommendation: "Add a main landmark to identify the primary content.".to_string(),
            });
        }
        if !ctx.page.has_nav_landmark && ctx.page.links.len() > 3 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Accessibility,
                code: "ARIALAND002".to_string(),
                title: "Missing navigation landmark".to_string(),
                description: "Multiple links found but no nav landmark.".to_string(),
                url: url.clone(),
                recommendation: "Wrap navigation links in <nav> or role=\"nav\".".to_string(),
            });
        }

        let landmark_count = ctx.page.landmarks.len();
        if landmark_count > 0 {
            let mut landmark_types: std::collections::HashMap<String, usize> =
                std::collections::HashMap::new();
            for lm in &ctx.page.landmarks {
                *landmark_types.entry(lm.clone()).or_default() += 1;
            }
            for (lm_type, count) in &landmark_types {
                if *count > 1 && lm_type != "navigation" {
                    findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Accessibility, code: "ARIALAND003".to_string(), title: format!("Duplicate landmark: {lm_type}"), description: format!("Landmark '{lm_type}' appears {count} times. Each landmark type should typically appear once."), url: url.clone(), recommendation: "Use unique landmark types or label duplicates with aria-label.".to_string() });
                }
            }
        }

        findings
    }
}

// =========================================================================
// HeadingHierarchyDeepAnalyzer
// =========================================================================

pub struct HeadingHierarchyDeepAnalyzer;

impl Default for HeadingHierarchyDeepAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl HeadingHierarchyDeepAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for HeadingHierarchyDeepAnalyzer {
    fn name(&self) -> &str {
        "heading-hierarchy-deep"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.headings.is_empty() {
            return findings;
        }

        let mut prev_level: u8 = 0;
        for h in &ctx.page.headings {
            if prev_level > 0 && h.level > prev_level + 1 {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Accessibility,
                    code: "HHIERDEEP001".to_string(),
                    title: "Heading hierarchy skip".to_string(),
                    description: format!("Heading jumped from H{prev_level} to H{}.", h.level),
                    url: url.clone(),
                    recommendation: format!(
                        "Use H{} after H{} for proper heading hierarchy.",
                        prev_level + 1,
                        prev_level
                    ),
                });
            }
            prev_level = h.level;
        }

        let first_level = ctx.page.headings[0].level;
        if first_level != 1 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "HHIERDEEP002".to_string(),
                title: "First heading is not H1".to_string(),
                description: format!(
                    "First heading is H{}, but should be H1 for accessibility.",
                    first_level
                ),
                url: url.clone(),
                recommendation: "Start the heading hierarchy with H1.".to_string(),
            });
        }

        let h1_count = ctx.page.headings.iter().filter(|h| h.level == 1).count();
        if h1_count > 1 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "HHIERDEEP003".to_string(),
                title: "Multiple H1 headings".to_string(),
                description: format!(
                    "Page has {h1_count} H1 headings. Screen readers expect a single H1."
                ),
                url: url.clone(),
                recommendation: "Use exactly one H1 per page.".to_string(),
            });
        }

        findings
    }
}

// =========================================================================
// FormLabelsDeepAnalyzer
// =========================================================================

pub struct FormLabelsDeepAnalyzer;

impl Default for FormLabelsDeepAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl FormLabelsDeepAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for FormLabelsDeepAnalyzer {
    fn name(&self) -> &str {
        "form-labels-deep"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let form_count = ctx.page.forms.len();
        let aria_label_count = ctx.page.aria_label_count;

        if form_count > 0 && aria_label_count == 0 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Accessibility,
                code: "FORMLBLDEEP001".to_string(),
                title: "Forms present but no ARIA labels".to_string(),
                description: format!("Page has {form_count} form(s) but no ARIA labels detected."),
                url: url.clone(),
                recommendation:
                    "Add aria-label or aria-labelledby to form elements for screen readers."
                        .to_string(),
            });
        }

        if form_count > 3 && aria_label_count < form_count {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "FORMLBLDEEP002".to_string(),
                title: "Insufficient ARIA labels for forms".to_string(),
                description: format!(
                    "Page has {form_count} forms but only {aria_label_count} ARIA labels."
                ),
                url: url.clone(),
                recommendation: "Each form should have an aria-label or aria-labelledby attribute."
                    .to_string(),
            });
        }

        findings
    }
}

// =========================================================================
// TableAccessibilityDeepAnalyzer
// =========================================================================

pub struct TableAccessibilityDeepAnalyzer;

impl Default for TableAccessibilityDeepAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl TableAccessibilityDeepAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for TableAccessibilityDeepAnalyzer {
    fn name(&self) -> &str {
        "table-accessibility-deep"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        if ctx.page.tables_total == 0 {
            return findings;
        }

        let tables_without_headers = ctx.page.tables_total - ctx.page.tables_with_headers;
        if tables_without_headers > 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "TABACCDEEP001".to_string(),
                title: "Tables without headers".to_string(),
                description: format!(
                    "{tables_without_headers}/{} table(s) lack header cells (th).",
                    ctx.page.tables_total
                ),
                url: url.clone(),
                recommendation: "Add <th> elements to identify column/row headers in data tables."
                    .to_string(),
            });
        }

        let tables_without_captions = ctx.page.tables_total - ctx.page.tables_with_captions;
        if tables_without_captions > 0 && ctx.page.tables_total > 1 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Accessibility,
                code: "TABACCDEEP002".to_string(),
                title: "Tables missing captions".to_string(),
                description: format!(
                    "{tables_without_captions}/{} table(s) lack <caption> elements.",
                    ctx.page.tables_total
                ),
                url: url.clone(),
                recommendation:
                    "Add <caption> elements to describe table purpose for screen readers."
                        .to_string(),
            });
        }

        findings
    }
}

// =========================================================================
// LinkTextQualityAnalyzer
// =========================================================================

pub struct LinkTextQualityAnalyzer;

impl Default for LinkTextQualityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl LinkTextQualityAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for LinkTextQualityAnalyzer {
    fn name(&self) -> &str {
        "link-text-quality"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let generic_texts = [
            "click here",
            "here",
            "read more",
            "learn more",
            "more",
            "link",
            "this",
        ];

        let generic_count: usize = ctx
            .page
            .links
            .iter()
            .filter(|l| {
                let text_lower = l.text.to_lowercase();
                generic_texts.iter().any(|g| text_lower == *g)
            })
            .count();

        if generic_count > 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "LINKTQ001".to_string(),
                title: "Generic link text detected".to_string(),
                description: format!(
                    "{generic_count} link(s) use generic text like 'click here' or 'read more'."
                ),
                url: url.clone(),
                recommendation: "Use descriptive link text that indicates the link destination."
                    .to_string(),
            });
        }

        let empty_text_count = ctx
            .page
            .links
            .iter()
            .filter(|l| l.text.trim().is_empty() && l.aria_label.is_none())
            .count();
        if empty_text_count > 0 {
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Accessibility,
                code: "LINKTQ002".to_string(),
                title: "Links without accessible text".to_string(),
                description: format!("{empty_text_count} link(s) have no text or aria-label."),
                url: url.clone(),
                recommendation: "Add visible text or aria-label to all links for screen readers."
                    .to_string(),
            });
        }

        findings
    }
}

// =========================================================================
// ImageAltTextDeepAnalyzer
// =========================================================================

pub struct ImageAltTextDeepAnalyzer;

impl Default for ImageAltTextDeepAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageAltTextDeepAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for ImageAltTextDeepAnalyzer {
    fn name(&self) -> &str {
        "image-alt-text-deep"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let total_images = ctx.page.images.len();
        if total_images == 0 {
            return findings;
        }

        let missing_alt: usize = ctx
            .page
            .images
            .iter()
            .filter(|img| !img.has_alt || img.alt.trim().is_empty())
            .count();
        if missing_alt > 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "IMGALTDEEP001".to_string(),
                title: "Images missing alt text".to_string(),
                description: format!(
                    "{missing_alt}/{total_images} image(s) have missing or empty alt text."
                ),
                url: url.clone(),
                recommendation:
                    "Add descriptive alt text to all images. Use alt=\"\" for decorative images."
                        .to_string(),
            });
        }

        let alt_too_long: usize = ctx
            .page
            .images
            .iter()
            .filter(|img| img.alt.len() > 125)
            .count();
        if alt_too_long > 0 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Accessibility,
                code: "IMGALTDEEP002".to_string(),
                title: "Alt text too long".to_string(),
                description: format!(
                    "{alt_too_long} image(s) have alt text exceeding 125 characters."
                ),
                url: url.clone(),
                recommendation:
                    "Keep alt text concise (under 125 characters). Use longdesc for complex images."
                        .to_string(),
            });
        }

        findings
    }
}

// =========================================================================
// FocusManagementDeepAnalyzer
// =========================================================================

pub struct FocusManagementDeepAnalyzer;

impl Default for FocusManagementDeepAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl FocusManagementDeepAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for FocusManagementDeepAnalyzer {
    fn name(&self) -> &str {
        "focus-management-deep"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        if ctx.page.has_positive_tabindex {
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Accessibility,
                code: "FOCUSDEEP001".to_string(),
                title: "Positive tabindex detected".to_string(),
                description: "A positive tabindex value disrupts natural tab order.".to_string(),
                url: url.clone(),
                recommendation: "Use tabindex=\"0\" or tabindex=\"-1\" instead of positive values."
                    .to_string(),
            });
        }

        if ctx.page.tabindex_negative_count > 3 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Accessibility,
                code: "FOCUSDEEP002".to_string(),
                title: "Many elements with tabindex=-1".to_string(),
                description: format!(
                    "{} elements have tabindex=\"-1\", removing them from tab order.",
                    ctx.page.tabindex_negative_count
                ),
                url: url.clone(),
                recommendation:
                    "Ensure elements with tabindex=-1 are intentionally removed from tab order."
                        .to_string(),
            });
        }

        findings
    }
}

// =========================================================================
// LanguageAttributesDeepAnalyzer
// =========================================================================

pub struct LanguageAttributesDeepAnalyzer;

impl Default for LanguageAttributesDeepAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageAttributesDeepAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for LanguageAttributesDeepAnalyzer {
    fn name(&self) -> &str {
        "language-attributes-deep"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        if !ctx.page.has_lang_attribute {
            findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Accessibility, code: "LANGATTRDEEP001".to_string(), title: "Missing html lang attribute".to_string(), description: "The <html> element lacks a lang attribute, affecting screen reader pronunciation.".to_string(), url: url.clone(), recommendation: "Add lang=\"en\" (or appropriate language code) to the <html> element.".to_string() });
        }

        if let Some(lang) = &ctx.page.html_lang {
            if !lang.contains('-') && lang.len() > 2 {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Accessibility, code: "LANGATTRDEEP002".to_string(), title: "Language code may be too specific".to_string(), description: format!("html lang=\"{lang}\" is unusually long. Standard codes are 2-letter (en) or 4-letter (en-US)."), url: url.clone(), recommendation: "Verify the language code follows BCP 47 format.".to_string() });
            }
        }

        if ctx.page.has_lang_attribute && ctx.page.meta.language.is_some() {
            let html_lang = ctx.page.html_lang.as_deref().unwrap_or("");
            let meta_lang = ctx.page.meta.language.as_deref().unwrap_or("");
            let html_base = html_lang.split('-').next().unwrap_or("");
            let meta_base = meta_lang.split('-').next().unwrap_or("");
            if !html_base.is_empty() && !meta_base.is_empty() && html_base != meta_base {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Accessibility,
                    code: "LANGATTRDEEP003".to_string(),
                    title: "Language mismatch between HTML and meta".to_string(),
                    description: format!(
                        "HTML lang is \"{html_lang}\" but meta language is \"{meta_lang}\"."
                    ),
                    url: url.clone(),
                    recommendation: "Ensure html lang and meta language are consistent."
                        .to_string(),
                });
            }
        }

        findings
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzers::CookieAnalyzer;
    use crate::analyzers::{
        ContentSecurityPolicyAnalyzer, ContentTypeSniffingAnalyzer,
        CrossOriginEmbedderPolicyAnalyzer, CrossOriginOpenerPolicyAnalyzer,
        CrossOriginResourcePolicyAnalyzer, MixedContentAnalyzer, PermissionsPolicyAnalyzerNew,
        ReferrerPolicyAnalyzer, StrictTransportSecurityAnalyzer, XContentTypeOptionsAnalyzer,
        XFrameOptionsAnalyzer, XPermittedCrossDomainPoliciesAnalyzer, XSSProtectionAnalyzer,
    };
    use crate::meta::MetaTags;
    use crate::parser::{ExtractedImage, ExtractedLink, Heading, ParsedPage};
    use url::Url;

    fn make_page(url: &str) -> ParsedPage {
        ParsedPage {
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
        page: &'a ParsedPage,
        status: Option<u16>,
        headers: &'a [(String, String)],
        content_type: Option<&'a str>,
    ) -> AnalysisContext<'a> {
        AnalysisContext {
            page,
            body: None,
            status_code: status,
            headers,
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type,
            rendered: None,
        }
    }

    // ===== ContentSecurityPolicyAnalyzer tests =====

    #[test]
    fn test_csp_no_header() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(ContentSecurityPolicyAnalyzer::new()
            .analyze(&ctx)
            .is_empty());
    }

    #[test]
    fn test_csp_unsafe_inline() {
        let headers = vec![(
            "Content-Security-Policy".to_string(),
            "script-src 'self' 'unsafe-inline'".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = ContentSecurityPolicyAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "CSP001"));
    }

    #[test]
    fn test_csp_no_frame_ancestors() {
        let headers = vec![(
            "Content-Security-Policy".to_string(),
            "default-src 'self'".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = ContentSecurityPolicyAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "CSP002"));
    }

    #[test]
    fn test_csp_valid() {
        let headers = vec![(
            "Content-Security-Policy".to_string(),
            "default-src 'self'; script-src 'self'; frame-ancestors 'self'".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = ContentSecurityPolicyAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_csp_both_issues() {
        let headers = vec![(
            "Content-Security-Policy".to_string(),
            "script-src 'self' 'unsafe-inline'".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = ContentSecurityPolicyAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "CSP001"));
        assert!(findings.iter().any(|f| f.code == "CSP002"));
    }

    #[test]
    fn test_csp_frame_ancestors_none() {
        let headers = vec![(
            "Content-Security-Policy".to_string(),
            "default-src 'self'; frame-ancestors 'none'".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = ContentSecurityPolicyAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "CSP002"));
    }

    #[test]
    fn test_csp_case_insensitive_header_lookup() {
        let headers = vec![(
            "content-security-policy".to_string(),
            "default-src 'self'; frame-ancestors 'none'".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = ContentSecurityPolicyAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_csp_script_src_in_other_directive_not_flagged() {
        let headers = vec![(
            "Content-Security-Policy".to_string(),
            "style-src 'self' 'unsafe-inline'; frame-ancestors 'self'".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = ContentSecurityPolicyAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "CSP001"));
    }

    #[test]
    fn test_csp_empty_script_src_value() {
        let headers = vec![(
            "Content-Security-Policy".to_string(),
            "script-src; frame-ancestors 'self'".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = ContentSecurityPolicyAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "CSP001"));
    }

    #[test]
    fn test_csp_multiple_script_src_directives() {
        let headers = vec![(
            "Content-Security-Policy".to_string(),
            "default-src 'self'; script-src 'self' 'unsafe-inline'; script-src-elem 'self'"
                .to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = ContentSecurityPolicyAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "CSP001"));
    }

    #[test]
    fn test_csp_nonce_instead_of_unsafe_inline() {
        let headers = vec![(
            "Content-Security-Policy".to_string(),
            "script-src 'self' 'nonce-abc123'; frame-ancestors 'self'".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = ContentSecurityPolicyAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_csp_empty_csp_value() {
        let headers = vec![("Content-Security-Policy".to_string(), "".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = ContentSecurityPolicyAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "CSP002"));
    }

    // ===== ReferrerPolicyAnalyzer tests =====

    #[test]
    fn test_referrer_no_header() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = ReferrerPolicyAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "REF001"));
    }

    #[test]
    fn test_referrer_unsafe_url() {
        let headers = vec![("Referrer-Policy".to_string(), "unsafe-url".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = ReferrerPolicyAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "REF002"));
    }

    #[test]
    fn test_referrer_valid() {
        let headers = vec![(
            "Referrer-Policy".to_string(),
            "strict-origin-when-cross-origin".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = ReferrerPolicyAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_referrer_no_referrer() {
        let headers = vec![("Referrer-Policy".to_string(), "no-referrer".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = ReferrerPolicyAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_referrer_case_insensitive() {
        let headers = vec![("referrer-policy".to_string(), "unsafe-url".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = ReferrerPolicyAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "REF002"));
    }

    #[test]
    fn test_referrer_origin() {
        let headers = vec![("Referrer-Policy".to_string(), "origin".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = ReferrerPolicyAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_referrer_same_origin() {
        let headers = vec![("Referrer-Policy".to_string(), "same-origin".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = ReferrerPolicyAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_referrer_strict_origin() {
        let headers = vec![("Referrer-Policy".to_string(), "strict-origin".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = ReferrerPolicyAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_referrer_no_referrer_when_downgrade() {
        let headers = vec![(
            "Referrer-Policy".to_string(),
            "no-referrer-when-downgrade".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = ReferrerPolicyAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_referrer_unsafe_url_with_whitespace() {
        let headers = vec![("Referrer-Policy".to_string(), "  unsafe-url  ".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = ReferrerPolicyAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "REF002"));
    }

    #[test]
    fn test_referrer_empty_value() {
        let headers = vec![("Referrer-Policy".to_string(), "".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = ReferrerPolicyAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_referrer_both_findings() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = ReferrerPolicyAnalyzer::new().analyze(&ctx);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "REF001");
    }

    // ===== XFrameOptionsAnalyzer tests =====

    #[test]
    fn test_xfo_no_header() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], Some("text/html"));
        let findings = XFrameOptionsAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "XFO001"));
    }

    #[test]
    fn test_xfo_allowall() {
        let headers = vec![("X-Frame-Options".to_string(), "ALLOWALL".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, Some("text/html"));
        let findings = XFrameOptionsAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "XFO002"));
    }

    #[test]
    fn test_xfo_deny() {
        let headers = vec![("X-Frame-Options".to_string(), "DENY".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, Some("text/html"));
        let findings = XFrameOptionsAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_xfo_sameorigin() {
        let headers = vec![("X-Frame-Options".to_string(), "SAMEORIGIN".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, Some("text/html"));
        let findings = XFrameOptionsAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_xfo_non_html_page_ignored() {
        let page = make_page("https://example.com/image.png");
        let ctx = make_ctx(&page, Some(200), &[], Some("image/png"));
        let findings = XFrameOptionsAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_xfo_case_insensitive_header() {
        let headers = vec![("x-frame-options".to_string(), "DENY".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, Some("text/html"));
        let findings = XFrameOptionsAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_xfo_allowall_case_insensitive() {
        let headers = vec![("x-frame-options".to_string(), "allowall".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, Some("text/html"));
        let findings = XFrameOptionsAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "XFO002"));
    }

    #[test]
    fn test_xfo_no_content_type_treated_as_html() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = XFrameOptionsAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "XFO001"));
    }

    #[test]
    fn test_xfo_xml_content_type_ignored() {
        let page = make_page("https://example.com/feed.xml");
        let ctx = make_ctx(&page, Some(200), &[], Some("application/xml"));
        let findings = XFrameOptionsAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_xfo_json_content_type_ignored() {
        let page = make_page("https://example.com/api/data");
        let ctx = make_ctx(&page, Some(200), &[], Some("application/json"));
        let findings = XFrameOptionsAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_xfo_javascript_content_type_ignored() {
        let page = make_page("https://example.com/app.js");
        let ctx = make_ctx(&page, Some(200), &[], Some("application/javascript"));
        let findings = XFrameOptionsAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_xfo_css_content_type_ignored() {
        let page = make_page("https://example.com/style.css");
        let ctx = make_ctx(&page, Some(200), &[], Some("text/css"));
        let findings = XFrameOptionsAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    // ===== MixedContentAnalyzer tests =====

    #[test]
    fn test_mixed_no_resources() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = MixedContentAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_mixed_http_resources_on_https() {
        let body = r#"<img src="http://cdn.example.com/photo.jpg">"#;
        let page = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &page,
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
        let findings = MixedContentAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "MIXED001"));
    }

    #[test]
    fn test_mixed_http_form_on_https() {
        let body = r#"<form action="http://example.com/submit">"#;
        let page = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &page,
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
        let findings = MixedContentAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "MIXED002"));
    }

    #[test]
    fn test_mixed_all_https_no_finding() {
        let body = r#"<img src="https://cdn.example.com/photo.jpg">"#;
        let page = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &page,
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
        let findings = MixedContentAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_mixed_http_page_not_checked() {
        let body = r#"<img src="http://cdn.example.com/photo.jpg">"#;
        let page = make_page("http://example.com");
        let ctx = AnalysisContext {
            page: &page,
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
        let findings = MixedContentAnalyzer::new().analyze(&ctx);
        // HTTP pages don't get mixed content warnings
        assert!(findings.is_empty());
    }

    #[test]
    fn test_mixed_multiple_http_resources() {
        let body = r#"
            <img src="http://cdn.example.com/photo1.jpg">
            <img src="http://cdn.example.com/photo2.jpg">
            <script src="http://cdn.example.com/app.js"></script>
            <link href="http://cdn.example.com/style.css" rel="stylesheet">
        "#;
        let page = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &page,
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
        let findings = MixedContentAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "MIXED001"));
        let f = findings.iter().find(|f| f.code == "MIXED001").unwrap();
        assert!(f.description.contains("4"));
    }

    #[test]
    fn test_mixed_relative_urls_not_flagged() {
        let body = r#"<img src="/photo.jpg">"#;
        let page = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &page,
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
        let findings = MixedContentAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_mixed_both_resource_and_form() {
        let body = r#"
            <img src="http://cdn.example.com/photo.jpg">
            <form action="http://example.com/submit">
        "#;
        let page = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &page,
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
        let findings = MixedContentAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "MIXED001"));
        assert!(findings.iter().any(|f| f.code == "MIXED002"));
    }

    #[test]
    fn test_mixed_form_with_single_quotes() {
        let body = r#"<form action='http://example.com/submit'>"#;
        let page = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &page,
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
        let findings = MixedContentAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "MIXED002"));
    }

    #[test]
    fn test_mixed_data_uris_not_flagged() {
        let body = r#"<img src="data:image/png;base64,abc123">"#;
        let page = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &page,
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
        let findings = MixedContentAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    // ===== CookieAnalyzer tests =====

    #[test]
    fn test_cookie_no_cookies() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = CookieAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_cookie_missing_secure() {
        let headers = vec![(
            "Set-Cookie".to_string(),
            "session=abc123; HttpOnly; Path=/".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &page,
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
        let findings = CookieAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "COOKIE001"));
    }

    #[test]
    fn test_cookie_missing_httponly() {
        let headers = vec![(
            "Set-Cookie".to_string(),
            "session=abc123; Secure; Path=/".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &page,
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
        let findings = CookieAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "COOKIE002"));
    }

    #[test]
    fn test_cookie_both_flags_missing() {
        let headers = vec![(
            "Set-Cookie".to_string(),
            "session=abc123; Path=/".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &page,
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
        let findings = CookieAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "COOKIE001"));
        assert!(findings.iter().any(|f| f.code == "COOKIE002"));
    }

    #[test]
    fn test_cookie_all_flags_present() {
        let headers = vec![(
            "Set-Cookie".to_string(),
            "session=abc123; Secure; HttpOnly; Path=/".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &page,
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
        let findings = CookieAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_cookie_http_page_not_checked() {
        let headers = vec![(
            "Set-Cookie".to_string(),
            "session=abc123; Path=/".to_string(),
        )];
        let page = make_page("http://example.com");
        let ctx = AnalysisContext {
            page: &page,
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
        let findings = CookieAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_cookie_multiple_cookies() {
        let headers = vec![
            (
                "Set-Cookie".to_string(),
                "session=abc123; Path=/".to_string(),
            ),
            ("Set-Cookie".to_string(), "token=xyz789; Path=/".to_string()),
        ];
        let page = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &page,
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
        let findings = CookieAnalyzer::new().analyze(&ctx);
        // Both cookies missing both flags = 4 findings
        assert_eq!(findings.len(), 4);
    }

    #[test]
    fn test_cookie_case_insensitive_flags() {
        let headers = vec![(
            "Set-Cookie".to_string(),
            "session=abc123; secure; httponly; Path=/".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &page,
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
        let findings = CookieAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_cookie_session_cookie_name_extracted() {
        let headers = vec![(
            "Set-Cookie".to_string(),
            "session_id=abc123; Path=/".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &page,
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
        let findings = CookieAnalyzer::new().analyze(&ctx);
        let f = findings.iter().find(|f| f.code == "COOKIE001").unwrap();
        assert!(f.description.contains("session_id"));
    }

    // ===== XContentTypeOptionsAnalyzer tests =====

    #[test]
    fn test_xcto_missing() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = XContentTypeOptionsAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "XCTO001"));
    }

    #[test]
    fn test_xcto_nosniff() {
        let headers = vec![("X-Content-Type-Options".to_string(), "nosniff".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = XContentTypeOptionsAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_xcto_wrong_value() {
        let headers = vec![("X-Content-Type-Options".to_string(), "sniff".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = XContentTypeOptionsAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "XCTO002"));
    }

    #[test]
    fn test_xcto_case_insensitive() {
        let headers = vec![("x-content-type-options".to_string(), "NOSNIFF".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = XContentTypeOptionsAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_xcto_whitespace_around_nosniff() {
        let headers = vec![(
            "X-Content-Type-Options".to_string(),
            "  nosniff  ".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = XContentTypeOptionsAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    // ===== XPermittedCrossDomainPoliciesAnalyzer tests =====

    #[test]
    fn test_xpcdp_missing() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = XPermittedCrossDomainPoliciesAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "XPCDP001"));
    }

    #[test]
    fn test_xpcdp_none() {
        let headers = vec![(
            "X-Permitted-Cross-Domain-Policies".to_string(),
            "none".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = XPermittedCrossDomainPoliciesAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_xpcdp_all() {
        let headers = vec![(
            "X-Permitted-Cross-Domain-Policies".to_string(),
            "all".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = XPermittedCrossDomainPoliciesAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "XPCDP002"));
    }

    #[test]
    fn test_xpcdp_case_insensitive() {
        let headers = vec![(
            "x-permitted-cross-domain-policies".to_string(),
            "ALL".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = XPermittedCrossDomainPoliciesAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "XPCDP002"));
    }

    #[test]
    fn test_xpcdp_master_only() {
        let headers = vec![(
            "X-Permitted-Cross-Domain-Policies".to_string(),
            "master-only".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = XPermittedCrossDomainPoliciesAnalyzer::new().analyze(&ctx);
        // master-only is not "all" and not missing
        assert!(!findings.iter().any(|f| f.code == "XPCDP002"));
        assert!(!findings.iter().any(|f| f.code == "XPCDP001"));
    }

    // ===== CrossOriginResourcePolicyAnalyzer tests =====

    #[test]
    fn test_corp_missing() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = CrossOriginResourcePolicyAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "CORP001"));
    }

    #[test]
    fn test_corp_same_origin() {
        let headers = vec![(
            "Cross-Origin-Resource-Policy".to_string(),
            "same-origin".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = CrossOriginResourcePolicyAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_corp_cross_origin() {
        let headers = vec![(
            "Cross-Origin-Resource-Policy".to_string(),
            "cross-origin".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = CrossOriginResourcePolicyAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_corp_case_insensitive() {
        let headers = vec![(
            "cross-origin-resource-policy".to_string(),
            "same-origin".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = CrossOriginResourcePolicyAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    // ===== LandmarkRegionsAnalyzer tests =====

    #[test]
    fn test_landmark_missing_all() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = LandmarkRegionsAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "LAND001"));
        assert!(findings.iter().any(|f| f.code == "LAND002"));
        assert!(findings.iter().any(|f| f.code == "LAND003"));
    }

    #[test]
    fn test_landmark_has_main() {
        let mut page = make_page("https://example.com");
        page.has_main_landmark = true;
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = LandmarkRegionsAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "LAND001"));
    }

    #[test]
    fn test_landmark_has_nav() {
        let mut page = make_page("https://example.com");
        page.has_nav_landmark = true;
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = LandmarkRegionsAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "LAND002"));
    }

    #[test]
    fn test_landmark_has_banner() {
        let mut page = make_page("https://example.com");
        page.landmarks.push("banner".to_string());
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = LandmarkRegionsAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "LAND003"));
    }

    #[test]
    fn test_landmark_has_header_role() {
        let mut page = make_page("https://example.com");
        page.landmarks.push("header".to_string());
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = LandmarkRegionsAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "LAND003"));
    }

    #[test]
    fn test_landmark_all_present_no_findings() {
        let mut page = make_page("https://example.com");
        page.has_main_landmark = true;
        page.has_nav_landmark = true;
        page.landmarks.push("banner".to_string());
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = LandmarkRegionsAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_landmark_only_main_missing() {
        let mut page = make_page("https://example.com");
        page.has_nav_landmark = true;
        page.landmarks.push("banner".to_string());
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = LandmarkRegionsAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "LAND001"));
        assert!(!findings.iter().any(|f| f.code == "LAND002"));
        assert!(!findings.iter().any(|f| f.code == "LAND003"));
    }

    #[test]
    fn test_landmark_severity_levels() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = LandmarkRegionsAnalyzer::new().analyze(&ctx);
        let land001 = findings.iter().find(|f| f.code == "LAND001").unwrap();
        assert_eq!(land001.severity, Severity::Error);
        let land002 = findings.iter().find(|f| f.code == "LAND002").unwrap();
        assert_eq!(land002.severity, Severity::Warning);
        let land003 = findings.iter().find(|f| f.code == "LAND003").unwrap();
        assert_eq!(land003.severity, Severity::Info);
    }

    #[test]
    fn test_landmark_all_use_accessibility_category() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = LandmarkRegionsAnalyzer::new().analyze(&ctx);
        for f in &findings {
            assert_eq!(f.category, IssueCategory::Accessibility);
        }
    }

    #[test]
    fn test_landmark_analyzer_name() {
        assert_eq!(LandmarkRegionsAnalyzer::new().name(), "landmark-regions");
    }

    // ===== HeadingOrderAnalyzer tests =====

    #[test]
    fn test_heading_order_skip_level() {
        let mut page = make_page("https://example.com");
        page.headings = vec![
            Heading {
                level: 1,
                text: "H1".to_string(),
                length: 2,
            },
            Heading {
                level: 3,
                text: "H3".to_string(),
                length: 2,
            },
        ];
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = HeadingOrderAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "HORDER001"));
    }

    #[test]
    fn test_heading_order_non_sequential() {
        let mut page = make_page("https://example.com");
        page.headings = vec![
            Heading {
                level: 1,
                text: "H1".to_string(),
                length: 2,
            },
            Heading {
                level: 2,
                text: "H2".to_string(),
                length: 2,
            },
            Heading {
                level: 3,
                text: "H3".to_string(),
                length: 2,
            },
            Heading {
                level: 2,
                text: "H2b".to_string(),
                length: 3,
            },
            Heading {
                level: 4,
                text: "H4".to_string(),
                length: 2,
            },
        ];
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = HeadingOrderAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "HORDER002"));
    }

    #[test]
    fn test_heading_order_valid_sequence() {
        let mut page = make_page("https://example.com");
        page.headings = vec![
            Heading {
                level: 1,
                text: "H1".to_string(),
                length: 2,
            },
            Heading {
                level: 2,
                text: "H2".to_string(),
                length: 2,
            },
            Heading {
                level: 3,
                text: "H3".to_string(),
                length: 2,
            },
        ];
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = HeadingOrderAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_heading_order_same_level_repeated() {
        let mut page = make_page("https://example.com");
        page.headings = vec![
            Heading {
                level: 2,
                text: "H2a".to_string(),
                length: 3,
            },
            Heading {
                level: 2,
                text: "H2b".to_string(),
                length: 3,
            },
            Heading {
                level: 2,
                text: "H2c".to_string(),
                length: 3,
            },
        ];
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = HeadingOrderAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_heading_order_single_heading() {
        let mut page = make_page("https://example.com");
        page.headings = vec![Heading {
            level: 1,
            text: "Only".to_string(),
            length: 4,
        }];
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = HeadingOrderAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_heading_order_no_headings() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = HeadingOrderAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_heading_order_skip_from_h2_to_h4() {
        let mut page = make_page("https://example.com");
        page.headings = vec![
            Heading {
                level: 2,
                text: "H2".to_string(),
                length: 2,
            },
            Heading {
                level: 4,
                text: "H4".to_string(),
                length: 2,
            },
        ];
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = HeadingOrderAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "HORDER001"));
    }

    #[test]
    fn test_heading_order_use_accessibility_category() {
        let mut page = make_page("https://example.com");
        page.headings = vec![
            Heading {
                level: 1,
                text: "H1".to_string(),
                length: 2,
            },
            Heading {
                level: 3,
                text: "H3".to_string(),
                length: 2,
            },
        ];
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = HeadingOrderAnalyzer::new().analyze(&ctx);
        for f in &findings {
            assert_eq!(f.category, IssueCategory::Accessibility);
        }
    }

    #[test]
    fn test_heading_order_analyzer_name() {
        assert_eq!(HeadingOrderAnalyzer::new().name(), "heading-order");
    }

    #[test]
    fn test_heading_order_descend_then_ascent() {
        let mut page = make_page("https://example.com");
        page.headings = vec![
            Heading {
                level: 3,
                text: "H3".to_string(),
                length: 2,
            },
            Heading {
                level: 2,
                text: "H2".to_string(),
                length: 2,
            },
            Heading {
                level: 3,
                text: "H3b".to_string(),
                length: 3,
            },
        ];
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = HeadingOrderAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "HORDER002"));
    }

    // ===== FormLabelAnalyzer tests =====

    #[test]
    fn test_form_label_missing_label() {
        use crate::parser::{ExtractedForm, ExtractedInput};
        let mut page = make_page("https://example.com");
        page.forms = vec![ExtractedForm {
            action: None,
            method: "post".to_string(),
            input_count: 1,
            has_file_input: false,
            has_search_input: false,
            inputs: vec![ExtractedInput {
                input_type: Some("text".to_string()),
                name: Some("email".to_string()),
                id: None,
                has_label: false,
                aria_label: None,
                aria_labelledby: None,
                aria_describedby: None,
                placeholder: None,
                required: false,
            }],
            has_fieldset: false,
            has_legend: false,
        }];
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = FormLabelAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "FLABEL001"));
    }

    #[test]
    fn test_form_label_with_aria_label() {
        use crate::parser::{ExtractedForm, ExtractedInput};
        let mut page = make_page("https://example.com");
        page.forms = vec![ExtractedForm {
            action: None,
            method: "post".to_string(),
            input_count: 1,
            has_file_input: false,
            has_search_input: false,
            inputs: vec![ExtractedInput {
                input_type: Some("text".to_string()),
                name: Some("email".to_string()),
                id: None,
                has_label: false,
                aria_label: Some("Email address".to_string()),
                aria_labelledby: None,
                aria_describedby: None,
                placeholder: None,
                required: false,
            }],
            has_fieldset: false,
            has_legend: false,
        }];
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = FormLabelAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "FLABEL001"));
    }

    #[test]
    fn test_form_label_with_label_element() {
        use crate::parser::{ExtractedForm, ExtractedInput};
        let mut page = make_page("https://example.com");
        page.forms = vec![ExtractedForm {
            action: None,
            method: "post".to_string(),
            input_count: 1,
            has_file_input: false,
            has_search_input: false,
            inputs: vec![ExtractedInput {
                input_type: Some("email".to_string()),
                name: Some("user_email".to_string()),
                id: Some("email".to_string()),
                has_label: true,
                aria_label: None,
                aria_labelledby: None,
                aria_describedby: None,
                placeholder: None,
                required: true,
            }],
            has_fieldset: false,
            has_legend: false,
        }];
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = FormLabelAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_form_label_multiple_inputs_mixed() {
        use crate::parser::{ExtractedForm, ExtractedInput};
        let mut page = make_page("https://example.com");
        page.forms = vec![ExtractedForm {
            action: None,
            method: "post".to_string(),
            input_count: 2,
            has_file_input: false,
            has_search_input: false,
            inputs: vec![
                ExtractedInput {
                    input_type: Some("text".to_string()),
                    name: Some("name".to_string()),
                    id: None,
                    has_label: true,
                    aria_label: None,
                    aria_labelledby: None,
                    aria_describedby: None,
                    placeholder: None,
                    required: false,
                },
                ExtractedInput {
                    input_type: Some("email".to_string()),
                    name: Some("email".to_string()),
                    id: None,
                    has_label: false,
                    aria_label: None,
                    aria_labelledby: None,
                    aria_describedby: None,
                    placeholder: None,
                    required: false,
                },
            ],
            has_fieldset: false,
            has_legend: false,
        }];
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = FormLabelAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "FLABEL001"));
        let f = findings.iter().find(|f| f.code == "FLABEL001").unwrap();
        assert!(f.description.contains("email"));
    }

    #[test]
    fn test_form_label_with_aria_labelledby() {
        use crate::parser::{ExtractedForm, ExtractedInput};
        let mut page = make_page("https://example.com");
        page.forms = vec![ExtractedForm {
            action: None,
            method: "post".to_string(),
            input_count: 1,
            has_file_input: false,
            has_search_input: false,
            inputs: vec![ExtractedInput {
                input_type: Some("text".to_string()),
                name: Some("search".to_string()),
                id: None,
                has_label: false,
                aria_label: None,
                aria_labelledby: Some("search-label".to_string()),
                aria_describedby: None,
                placeholder: None,
                required: false,
            }],
            has_fieldset: false,
            has_legend: false,
        }];
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = FormLabelAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "FLABEL001"));
    }

    #[test]
    fn test_form_label_no_forms() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = FormLabelAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_form_label_use_accessibility_category() {
        use crate::parser::{ExtractedForm, ExtractedInput};
        let mut page = make_page("https://example.com");
        page.forms = vec![ExtractedForm {
            action: None,
            method: "post".to_string(),
            input_count: 1,
            has_file_input: false,
            has_search_input: false,
            inputs: vec![ExtractedInput {
                input_type: Some("text".to_string()),
                name: Some("field".to_string()),
                id: None,
                has_label: false,
                aria_label: None,
                aria_labelledby: None,
                aria_describedby: None,
                placeholder: None,
                required: false,
            }],
            has_fieldset: false,
            has_legend: false,
        }];
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = FormLabelAnalyzer::new().analyze(&ctx);
        for f in &findings {
            assert_eq!(f.category, IssueCategory::Accessibility);
        }
    }

    #[test]
    fn test_form_label_analyzer_name() {
        assert_eq!(FormLabelAnalyzer::new().name(), "form-labels");
    }

    #[test]
    fn test_form_label_unnamed_input() {
        use crate::parser::{ExtractedForm, ExtractedInput};
        let mut page = make_page("https://example.com");
        page.forms = vec![ExtractedForm {
            action: None,
            method: "post".to_string(),
            input_count: 1,
            has_file_input: false,
            has_search_input: false,
            inputs: vec![ExtractedInput {
                input_type: Some("text".to_string()),
                name: None,
                id: None,
                has_label: false,
                aria_label: None,
                aria_labelledby: None,
                aria_describedby: None,
                placeholder: None,
                required: false,
            }],
            has_fieldset: false,
            has_legend: false,
        }];
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = FormLabelAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "FLABEL001"));
        let f = findings.iter().find(|f| f.code == "FLABEL001").unwrap();
        assert!(f.description.contains("input (type=\"text\")"));
    }

    // ===== TableAccessibilityAnalyzer tests =====

    #[test]
    fn test_table_acc_missing_headers() {
        let mut page = make_page("https://example.com");
        page.tables_total = 3;
        page.tables_with_headers = 1;
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = TableAccessibilityAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "TACC001"));
    }

    #[test]
    fn test_table_acc_missing_caption() {
        let mut page = make_page("https://example.com");
        page.tables_total = 2;
        page.tables_with_captions = 0;
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = TableAccessibilityAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "TACC002"));
    }

    #[test]
    fn test_table_acc_all_have_headers_and_captions() {
        let mut page = make_page("https://example.com");
        page.tables_total = 5;
        page.tables_with_headers = 5;
        page.tables_with_captions = 5;
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = TableAccessibilityAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_table_acc_no_tables() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = TableAccessibilityAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_table_acc_large_table_missing_scope() {
        let mut page = make_page("https://example.com");
        page.tables_total = 15;
        page.tables_with_headers = 0;
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = TableAccessibilityAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "TACC003"));
    }

    #[test]
    fn test_table_acc_small_table_no_scope_finding() {
        let mut page = make_page("https://example.com");
        page.tables_total = 5;
        page.tables_with_headers = 0;
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = TableAccessibilityAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "TACC003"));
    }

    #[test]
    fn test_table_acc_use_accessibility_category() {
        let mut page = make_page("https://example.com");
        page.tables_total = 1;
        page.tables_with_headers = 0;
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = TableAccessibilityAnalyzer::new().analyze(&ctx);
        for f in &findings {
            assert_eq!(f.category, IssueCategory::Accessibility);
        }
    }

    #[test]
    fn test_table_acc_analyzer_name() {
        assert_eq!(
            TableAccessibilityAnalyzer::new().name(),
            "table-accessibility"
        );
    }

    #[test]
    fn test_table_acc_all_have_captions_no_headers() {
        let mut page = make_page("https://example.com");
        page.tables_total = 3;
        page.tables_with_headers = 0;
        page.tables_with_captions = 3;
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = TableAccessibilityAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "TACC001"));
        assert!(!findings.iter().any(|f| f.code == "TACC002"));
    }

    #[test]
    fn test_table_acc_description_contains_counts() {
        let mut page = make_page("https://example.com");
        page.tables_total = 5;
        page.tables_with_headers = 2;
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = TableAccessibilityAnalyzer::new().analyze(&ctx);
        let tacc001 = findings.iter().find(|f| f.code == "TACC001").unwrap();
        assert!(tacc001.description.contains("3 of 5"));
    }

    // ===== LinkAccessibilityAnalyzer tests =====

    #[test]
    fn test_link_acc_empty_text() {
        let mut page = make_page("https://example.com");
        page.links = vec![ExtractedLink {
            href: "/page".to_string(),
            text: String::new(),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = LinkAccessibilityAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "LNKACC001"));
    }

    #[test]
    fn test_link_acc_generic_text() {
        let mut page = make_page("https://example.com");
        page.links = vec![ExtractedLink {
            href: "/page".to_string(),
            text: "click here".to_string(),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = LinkAccessibilityAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "LNKACC002"));
    }

    #[test]
    fn test_link_acc_nondescriptive_text() {
        let mut page = make_page("https://example.com");
        page.links = vec![ExtractedLink {
            href: "/page".to_string(),
            text: "link".to_string(),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = LinkAccessibilityAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "LNKACC003"));
    }

    #[test]
    fn test_link_acc_good_text() {
        let mut page = make_page("https://example.com");
        page.links = vec![ExtractedLink {
            href: "/pricing".to_string(),
            text: "View our pricing plans".to_string(),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = LinkAccessibilityAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_link_acc_with_aria_label() {
        let mut page = make_page("https://example.com");
        page.links = vec![ExtractedLink {
            href: "/page".to_string(),
            text: String::new(),
            rel: vec![],
            is_external: false,
            aria_label: Some("Go to page".to_string()),
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = LinkAccessibilityAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "LNKACC001"));
    }

    #[test]
    fn test_link_acc_with_img_alt() {
        let mut page = make_page("https://example.com");
        page.links = vec![ExtractedLink {
            href: "/page".to_string(),
            text: String::new(),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: Some("Logo link".to_string()),
        }];
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = LinkAccessibilityAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "LNKACC001"));
    }

    #[test]
    fn test_link_acc_multiple_generic_texts() {
        let mut page = make_page("https://example.com");
        page.links = vec![
            ExtractedLink {
                href: "/a".to_string(),
                text: "read more".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "/b".to_string(),
                text: "click here".to_string(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            },
        ];
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = LinkAccessibilityAnalyzer::new().analyze(&ctx);
        let generic = findings.iter().filter(|f| f.code == "LNKACC002").count();
        assert_eq!(generic, 2);
    }

    #[test]
    fn test_link_acc_use_accessibility_category() {
        let mut page = make_page("https://example.com");
        page.links = vec![ExtractedLink {
            href: "/page".to_string(),
            text: String::new(),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = LinkAccessibilityAnalyzer::new().analyze(&ctx);
        for f in &findings {
            assert_eq!(f.category, IssueCategory::Accessibility);
        }
    }

    #[test]
    fn test_link_acc_analyzer_name() {
        assert_eq!(
            LinkAccessibilityAnalyzer::new().name(),
            "link-accessibility"
        );
    }

    #[test]
    fn test_link_acc_here_text() {
        let mut page = make_page("https://example.com");
        page.links = vec![ExtractedLink {
            href: "/page".to_string(),
            text: "here".to_string(),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = LinkAccessibilityAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "LNKACC003"));
    }

    // ===== ImageAccessibilityAnalyzer tests =====

    #[test]
    fn test_img_acc_missing_alt() {
        let mut page = make_page("https://example.com");
        page.images = vec![ExtractedImage {
            src: "/photo.jpg".to_string(),
            alt: String::new(),
            width: None,
            height: None,
            has_alt: false,
            is_lazy_loaded: false,
            aria_hidden: false,
        }];
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = ImageAccessibilityAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "IMGACC001"));
    }

    #[test]
    fn test_img_acc_empty_alt_non_decorative() {
        let mut page = make_page("https://example.com");
        page.images = vec![ExtractedImage {
            src: "/photo.jpg".to_string(),
            alt: String::new(),
            width: None,
            height: None,
            has_alt: true,
            is_lazy_loaded: false,
            aria_hidden: false,
        }];
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = ImageAccessibilityAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "IMGACC002"));
    }

    #[test]
    fn test_img_acc_empty_alt_decorative() {
        let mut page = make_page("https://example.com");
        page.images = vec![ExtractedImage {
            src: "/photo.jpg".to_string(),
            alt: String::new(),
            width: None,
            height: None,
            has_alt: true,
            is_lazy_loaded: false,
            aria_hidden: true,
        }];
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = ImageAccessibilityAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_img_acc_alt_equals_filename() {
        let mut page = make_page("https://example.com");
        page.images = vec![ExtractedImage {
            src: "/images/sunset.jpg".to_string(),
            alt: "sunset".to_string(),
            width: None,
            height: None,
            has_alt: true,
            is_lazy_loaded: false,
            aria_hidden: false,
        }];
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = ImageAccessibilityAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "IMGACC003"));
    }

    #[test]
    fn test_img_acc_good_alt_text() {
        let mut page = make_page("https://example.com");
        page.images = vec![ExtractedImage {
            src: "/images/sunset.jpg".to_string(),
            alt: "Beautiful sunset over the ocean".to_string(),
            width: None,
            height: None,
            has_alt: true,
            is_lazy_loaded: false,
            aria_hidden: false,
        }];
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = ImageAccessibilityAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_img_acc_no_images() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = ImageAccessibilityAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_img_acc_use_accessibility_category() {
        let mut page = make_page("https://example.com");
        page.images = vec![ExtractedImage {
            src: "/a.png".to_string(),
            alt: String::new(),
            width: None,
            height: None,
            has_alt: false,
            is_lazy_loaded: false,
            aria_hidden: false,
        }];
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = ImageAccessibilityAnalyzer::new().analyze(&ctx);
        for f in &findings {
            assert_eq!(f.category, IssueCategory::Accessibility);
        }
    }

    #[test]
    fn test_img_acc_analyzer_name() {
        assert_eq!(
            ImageAccessibilityAnalyzer::new().name(),
            "image-accessibility"
        );
    }

    #[test]
    fn test_img_acc_multiple_missing_alt() {
        let mut page = make_page("https://example.com");
        page.images = vec![
            ExtractedImage {
                src: "/a.png".to_string(),
                alt: String::new(),
                width: None,
                height: None,
                has_alt: false,
                is_lazy_loaded: false,
                aria_hidden: false,
            },
            ExtractedImage {
                src: "/b.jpg".to_string(),
                alt: String::new(),
                width: None,
                height: None,
                has_alt: false,
                is_lazy_loaded: false,
                aria_hidden: false,
            },
        ];
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = ImageAccessibilityAnalyzer::new().analyze(&ctx);
        let imgacc001 = findings.iter().filter(|f| f.code == "IMGACC001").count();
        assert_eq!(imgacc001, 2);
    }

    #[test]
    fn test_img_acc_filename_from_src() {
        assert_eq!(
            ImageAccessibilityAnalyzer::filename_from_src("/images/photo.jpg"),
            Some("photo.jpg")
        );
        assert_eq!(
            ImageAccessibilityAnalyzer::filename_from_src("https://cdn.com/img.png"),
            Some("img.png")
        );
        assert_eq!(
            ImageAccessibilityAnalyzer::filename_from_src("/noext"),
            Some("noext")
        );
    }

    // ===== AriaRolesAnalyzer tests =====

    #[test]
    fn test_aria_roles_with_no_labels() {
        let mut page = make_page("https://example.com");
        page.aria_role_count = 3;
        page.aria_label_count = 0;
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = AriaRolesAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ARIA001"));
    }

    #[test]
    fn test_aria_roles_with_labels() {
        let mut page = make_page("https://example.com");
        page.aria_role_count = 3;
        page.aria_label_count = 3;
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = AriaRolesAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_aria_roles_partial_labels() {
        let mut page = make_page("https://example.com");
        page.aria_role_count = 5;
        page.aria_label_count = 2;
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = AriaRolesAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ARIA002"));
    }

    #[test]
    fn test_aria_roles_no_roles() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = AriaRolesAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_aria_roles_more_labels_than_roles() {
        let mut page = make_page("https://example.com");
        page.aria_role_count = 2;
        page.aria_label_count = 5;
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = AriaRolesAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_aria_roles_use_accessibility_category() {
        let mut page = make_page("https://example.com");
        page.aria_role_count = 1;
        page.aria_label_count = 0;
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = AriaRolesAnalyzer::new().analyze(&ctx);
        for f in &findings {
            assert_eq!(f.category, IssueCategory::Accessibility);
        }
    }

    #[test]
    fn test_aria_roles_analyzer_name() {
        assert_eq!(AriaRolesAnalyzer::new().name(), "aria-roles");
    }

    #[test]
    fn test_aria_roles_description_contains_count() {
        let mut page = make_page("https://example.com");
        page.aria_role_count = 7;
        page.aria_label_count = 0;
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = AriaRolesAnalyzer::new().analyze(&ctx);
        let aria001 = findings.iter().find(|f| f.code == "ARIA001").unwrap();
        assert!(aria001.description.contains("7"));
    }

    #[test]
    fn test_aria_roles_single_role_no_label() {
        let mut page = make_page("https://example.com");
        page.aria_role_count = 1;
        page.aria_label_count = 0;
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = AriaRolesAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ARIA001"));
        assert!(!findings.iter().any(|f| f.code == "ARIA002"));
    }

    // ===== FocusManagementAnalyzer tests =====

    #[test]
    fn test_focus_positive_tabindex() {
        let mut page = make_page("https://example.com");
        page.has_positive_tabindex = true;
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = FocusManagementAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "FOCUS001"));
    }

    #[test]
    fn test_focus_no_positive_tabindex() {
        let mut page = make_page("https://example.com");
        page.has_positive_tabindex = false;
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = FocusManagementAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "FOCUS001"));
    }

    #[test]
    fn test_focus_no_focus_styles() {
        let mut page = make_page("https://example.com");
        page.has_positive_tabindex = false;
        page.links = vec![ExtractedLink {
            href: "/page".to_string(),
            text: "Go".to_string(),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = FocusManagementAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "FOCUS002"));
    }

    #[test]
    fn test_focus_has_focus_visible_style() {
        let mut page = make_page("https://example.com");
        page.has_positive_tabindex = false;
        page.links = vec![ExtractedLink {
            href: "/page".to_string(),
            text: "Go".to_string(),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: None,
        }];
        let body = "<style>:focus-visible { outline: 2px solid blue; }</style>";
        let ctx = AnalysisContext {
            page: &page,
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
        let findings = FocusManagementAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "FOCUS002"));
    }

    #[test]
    fn test_focus_has_focus_style() {
        let mut page = make_page("https://example.com");
        page.has_positive_tabindex = false;
        page.links = vec![ExtractedLink {
            href: "/page".to_string(),
            text: "Go".to_string(),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: None,
        }];
        let body = "<style>:focus { outline: 2px solid blue; }</style>";
        let ctx = AnalysisContext {
            page: &page,
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
        let findings = FocusManagementAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "FOCUS002"));
    }

    #[test]
    fn test_focus_no_interactive_elements() {
        let mut page = make_page("https://example.com");
        page.has_positive_tabindex = false;
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = FocusManagementAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "FOCUS002"));
    }

    #[test]
    fn test_focus_use_accessibility_category() {
        let mut page = make_page("https://example.com");
        page.has_positive_tabindex = true;
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = FocusManagementAnalyzer::new().analyze(&ctx);
        for f in &findings {
            assert_eq!(f.category, IssueCategory::Accessibility);
        }
    }

    #[test]
    fn test_focus_analyzer_name() {
        assert_eq!(FocusManagementAnalyzer::new().name(), "focus-management");
    }

    #[test]
    fn test_focus_both_issues() {
        let mut page = make_page("https://example.com");
        page.has_positive_tabindex = true;
        page.links = vec![ExtractedLink {
            href: "/page".to_string(),
            text: "Go".to_string(),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = FocusManagementAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "FOCUS001"));
        assert!(findings.iter().any(|f| f.code == "FOCUS002"));
    }

    #[test]
    fn test_focus_severity_levels() {
        let mut page = make_page("https://example.com");
        page.has_positive_tabindex = true;
        page.links = vec![ExtractedLink {
            href: "/page".to_string(),
            text: "Go".to_string(),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = FocusManagementAnalyzer::new().analyze(&ctx);
        let focus001 = findings.iter().find(|f| f.code == "FOCUS001").unwrap();
        assert_eq!(focus001.severity, Severity::Error);
        let focus002 = findings.iter().find(|f| f.code == "FOCUS002").unwrap();
        assert_eq!(focus002.severity, Severity::Warning);
    }

    // ===== LanguageAttributeAnalyzer (security) tests =====

    #[test]
    fn test_lang_acc_missing_lang() {
        let mut page = make_page("https://example.com");
        page.has_lang_attribute = false;
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = LanguageAttributeAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "LANGACC001"));
    }

    #[test]
    fn test_lang_acc_has_lang() {
        let mut page = make_page("https://example.com");
        page.has_lang_attribute = true;
        page.html_lang = Some("en".to_string());
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = LanguageAttributeAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "LANGACC001"));
    }

    #[test]
    fn test_lang_acc_too_short_value() {
        let mut page = make_page("https://example.com");
        page.has_lang_attribute = true;
        page.html_lang = Some("e".to_string());
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = LanguageAttributeAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "LANGACC002"));
    }

    #[test]
    fn test_lang_acc_valid_value() {
        let mut page = make_page("https://example.com");
        page.has_lang_attribute = true;
        page.html_lang = Some("en".to_string());
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = LanguageAttributeAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "LANGACC002"));
    }

    #[test]
    fn test_lang_acc_hreflang_mismatch() {
        let mut page = make_page("https://example.com");
        page.has_lang_attribute = true;
        page.html_lang = Some("fr".to_string());
        page.word_count = 100;
        page.meta.hreflang = vec![crate::meta::HreflangTag {
            lang: "en".to_string(),
            url: Url::parse("https://example.com/en").unwrap(),
        }];
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = LanguageAttributeAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "LANGACC002"));
    }

    #[test]
    fn test_lang_acc_hreflang_match() {
        let mut page = make_page("https://example.com");
        page.has_lang_attribute = true;
        page.html_lang = Some("en".to_string());
        page.word_count = 100;
        page.meta.hreflang = vec![crate::meta::HreflangTag {
            lang: "en".to_string(),
            url: Url::parse("https://example.com/en").unwrap(),
        }];
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = LanguageAttributeAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "LANGACC002"));
    }

    #[test]
    fn test_lang_acc_use_accessibility_category() {
        let mut page = make_page("https://example.com");
        page.has_lang_attribute = false;
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = LanguageAttributeAnalyzer::new().analyze(&ctx);
        for f in &findings {
            assert_eq!(f.category, IssueCategory::Accessibility);
        }
    }

    #[test]
    fn test_lang_acc_analyzer_name() {
        assert_eq!(
            LanguageAttributeAnalyzer::new().name(),
            "language-attribute"
        );
    }

    #[test]
    fn test_lang_acc_empty_hreflang_no_mismatch() {
        let mut page = make_page("https://example.com");
        page.has_lang_attribute = true;
        page.html_lang = Some("de".to_string());
        page.word_count = 100;
        page.meta.hreflang = vec![];
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = LanguageAttributeAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "LANGACC002"));
    }

    #[test]
    fn test_lang_acc_zero_words_no_mismatch() {
        let mut page = make_page("https://example.com");
        page.has_lang_attribute = true;
        page.html_lang = Some("fr".to_string());
        page.word_count = 0;
        page.meta.hreflang = vec![crate::meta::HreflangTag {
            lang: "en".to_string(),
            url: Url::parse("https://example.com/en").unwrap(),
        }];
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = LanguageAttributeAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "LANGACC002"));
    }

    // ===== StrictTransportSecurityAnalyzer tests =====

    #[test]
    fn test_hsts_missing() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = StrictTransportSecurityAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "STRICT001"));
    }

    #[test]
    fn test_hsts_valid() {
        let headers = vec![(
            "Strict-Transport-Security".to_string(),
            "max-age=31536000; includeSubDomains".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = StrictTransportSecurityAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "STRICT001"));
        assert!(!findings.iter().any(|f| f.code == "STRICT002"));
    }

    #[test]
    fn test_hsts_too_short() {
        let headers = vec![(
            "Strict-Transport-Security".to_string(),
            "max-age=300".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = StrictTransportSecurityAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "STRICT002"));
    }

    #[test]
    fn test_hsts_case_insensitive() {
        let headers = vec![(
            "strict-transport-security".to_string(),
            "max-age=63072000".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = StrictTransportSecurityAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_hsts_with_preload() {
        let headers = vec![(
            "Strict-Transport-Security".to_string(),
            "max-age=31536000; includeSubDomains; preload".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = StrictTransportSecurityAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_hsts_exact_boundary() {
        let headers = vec![(
            "Strict-Transport-Security".to_string(),
            "max-age=31536000".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = StrictTransportSecurityAnalyzer::new().analyze(&ctx);
        // Exactly 31536000 is valid
        assert!(!findings.iter().any(|f| f.code == "STRICT002"));
    }

    #[test]
    fn test_hsts_whitespace_around_max_age() {
        let headers = vec![(
            "Strict-Transport-Security".to_string(),
            "  max-age=31536000  ".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = StrictTransportSecurityAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_hsts_missing_max_age_param() {
        let headers = vec![(
            "Strict-Transport-Security".to_string(),
            "includeSubDomains".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = StrictTransportSecurityAnalyzer::new().analyze(&ctx);
        // No max-age parsed → treated as missing/valid (no STRICT002)
        assert!(!findings.iter().any(|f| f.code == "STRICT002"));
    }

    // ===== XSSProtectionAnalyzer tests =====

    #[test]
    fn test_xss_missing() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = XSSProtectionAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "XSS001"));
    }

    #[test]
    fn test_xss_mode_block() {
        let headers = vec![("X-XSS-Protection".to_string(), "1; mode=block".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = XSSProtectionAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "XSS002"));
    }

    #[test]
    fn test_xss_enabled_no_mode_block() {
        let headers = vec![("X-XSS-Protection".to_string(), "1".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = XSSProtectionAnalyzer::new().analyze(&ctx);
        // Present and not mode=block → no findings
        assert!(findings.is_empty());
    }

    #[test]
    fn test_xss_case_insensitive() {
        let headers = vec![("x-xss-protection".to_string(), "1; mode=block".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = XSSProtectionAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "XSS002"));
    }

    #[test]
    fn test_xss_zero_disabled() {
        let headers = vec![("X-XSS-Protection".to_string(), "0".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = XSSProtectionAnalyzer::new().analyze(&ctx);
        // Present, not mode=block → no findings
        assert!(findings.is_empty());
    }

    #[test]
    fn test_xss_whitespace_around_value() {
        let headers = vec![(
            "X-XSS-Protection".to_string(),
            "  1; mode=block  ".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = XSSProtectionAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "XSS002"));
    }

    #[test]
    fn test_xss_no_header_and_no_csp() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = XSSProtectionAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "XSS001"));
    }

    #[test]
    fn test_xss_multiple_headers_last_wins() {
        let headers = vec![
            ("X-XSS-Protection".to_string(), "1".to_string()),
            ("X-XSS-Protection".to_string(), "1; mode=block".to_string()),
        ];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = XSSProtectionAnalyzer::new().analyze(&ctx);
        // Our get_header returns first match, which is "1" (no mode=block)
        assert!(findings.is_empty());
    }

    // ===== ContentTypeSniffingAnalyzer tests =====

    #[test]
    fn test_ctsniff_missing() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = ContentTypeSniffingAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "CTSNIFF001"));
    }

    #[test]
    fn test_ctsniff_nosniff() {
        let headers = vec![("X-Content-Type-Options".to_string(), "nosniff".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = ContentTypeSniffingAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_ctsniff_wrong_value() {
        let headers = vec![("X-Content-Type-Options".to_string(), "sniff".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = ContentTypeSniffingAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "CTSNIFF002"));
    }

    #[test]
    fn test_ctsniff_case_insensitive() {
        let headers = vec![("x-content-type-options".to_string(), "NOSNIFF".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = ContentTypeSniffingAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_ctsniff_whitespace_around_nosniff() {
        let headers = vec![(
            "X-Content-Type-Options".to_string(),
            "  nosniff  ".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = ContentTypeSniffingAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_ctsniff_empty_value() {
        let headers = vec![("X-Content-Type-Options".to_string(), "".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = ContentTypeSniffingAnalyzer::new().analyze(&ctx);
        // Empty string is not "nosniff"
        assert!(findings.iter().any(|f| f.code == "CTSNIFF002"));
    }

    #[test]
    fn test_ctsniff_uppercase() {
        let headers = vec![("X-Content-Type-Options".to_string(), "NOSNIFF".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = ContentTypeSniffingAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_ctsniff_no_header_implies_vulnerable() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = ContentTypeSniffingAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "CTSNIFF001"));
        assert!(!findings.iter().any(|f| f.code == "CTSNIFF002"));
    }

    // =========================================================================
    // PermissionsPolicyAnalyzerNew tests
    // =========================================================================

    #[test]
    fn test_pperm_missing_header() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = PermissionsPolicyAnalyzerNew::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PPERM001"));
    }

    #[test]
    fn test_pperm_camera_not_restricted() {
        let headers = vec![("Permissions-Policy".to_string(), "camera=self".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = PermissionsPolicyAnalyzerNew::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PPERM002"));
    }

    #[test]
    fn test_pperm_camera_restricted() {
        let headers = vec![("Permissions-Policy".to_string(), "camera=()".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = PermissionsPolicyAnalyzerNew::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_pperm_valid_with_multiple_features() {
        let headers = vec![(
            "Permissions-Policy".to_string(),
            "camera=(), microphone=(), geolocation=()".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = PermissionsPolicyAnalyzerNew::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_pperm_camera_self_restricted() {
        let headers = vec![(
            "Permissions-Policy".to_string(),
            "camera=(self)".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = PermissionsPolicyAnalyzerNew::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_pperm_no_camera_feature() {
        let headers = vec![(
            "Permissions-Policy".to_string(),
            "microphone=()".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = PermissionsPolicyAnalyzerNew::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_pperm_empty_header_value() {
        let headers = vec![("Permissions-Policy".to_string(), "".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = PermissionsPolicyAnalyzerNew::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "PPERM001"));
        assert!(!findings.iter().any(|f| f.code == "PPERM002"));
    }

    #[test]
    fn test_pperm_case_insensitive_camera() {
        let headers = vec![("Permissions-Policy".to_string(), "Camera=self".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = PermissionsPolicyAnalyzerNew::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PPERM002"));
    }

    // =========================================================================
    // CrossOriginEmbedderPolicyAnalyzer tests
    // =========================================================================

    #[test]
    fn test_coep_missing_header() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = CrossOriginEmbedderPolicyAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "COEP001"));
    }

    #[test]
    fn test_coep_not_require_corp() {
        let headers = vec![(
            "Cross-Origin-Embedder-Policy".to_string(),
            "credentialless".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = CrossOriginEmbedderPolicyAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "COEP002"));
    }

    #[test]
    fn test_coep_require_corp_valid() {
        let headers = vec![(
            "Cross-Origin-Embedder-Policy".to_string(),
            "require-corp".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = CrossOriginEmbedderPolicyAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_coep_unsafe_none() {
        let headers = vec![(
            "Cross-Origin-Embedder-Policy".to_string(),
            "unsafe-none".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = CrossOriginEmbedderPolicyAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "COEP002"));
    }

    #[test]
    fn test_coep_case_sensitive() {
        let headers = vec![(
            "Cross-Origin-Embedder-Policy".to_string(),
            "Require-Corp".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = CrossOriginEmbedderPolicyAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "COEP002"));
    }

    #[test]
    fn test_coep_empty_value() {
        let headers = vec![("Cross-Origin-Embedder-Policy".to_string(), "".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = CrossOriginEmbedderPolicyAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "COEP001"));
        assert!(findings.iter().any(|f| f.code == "COEP002"));
    }

    #[test]
    fn test_coep_with_whitespace() {
        let headers = vec![(
            "Cross-Origin-Embedder-Policy".to_string(),
            " require-corp ".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = CrossOriginEmbedderPolicyAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_coep_no_headers() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = CrossOriginEmbedderPolicyAnalyzer::new().analyze(&ctx);
        assert_eq!(findings.len(), 1);
        assert!(findings.iter().any(|f| f.code == "COEP001"));
    }

    // =========================================================================
    // CrossOriginOpenerPolicyAnalyzer tests
    // =========================================================================

    #[test]
    fn test_coop_missing_header() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = CrossOriginOpenerPolicyAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "COOP001"));
    }

    #[test]
    fn test_coop_not_same_origin() {
        let headers = vec![(
            "Cross-Origin-Opener-Policy".to_string(),
            "same-origin-allow-popups".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = CrossOriginOpenerPolicyAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "COOP002"));
    }

    #[test]
    fn test_coop_same_origin_valid() {
        let headers = vec![(
            "Cross-Origin-Opener-Policy".to_string(),
            "same-origin".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = CrossOriginOpenerPolicyAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_coop_unsafe_none() {
        let headers = vec![(
            "Cross-Origin-Opener-Policy".to_string(),
            "unsafe-none".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = CrossOriginOpenerPolicyAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "COOP002"));
    }

    #[test]
    fn test_coop_case_sensitive() {
        let headers = vec![(
            "Cross-Origin-Opener-Policy".to_string(),
            "Same-Origin".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = CrossOriginOpenerPolicyAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "COOP002"));
    }

    #[test]
    fn test_coop_empty_value() {
        let headers = vec![("Cross-Origin-Opener-Policy".to_string(), "".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = CrossOriginOpenerPolicyAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "COOP001"));
        assert!(findings.iter().any(|f| f.code == "COOP002"));
    }

    #[test]
    fn test_coop_with_whitespace() {
        let headers = vec![(
            "Cross-Origin-Opener-Policy".to_string(),
            " same-origin ".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = CrossOriginOpenerPolicyAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_coop_no_headers() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = CrossOriginOpenerPolicyAnalyzer::new().analyze(&ctx);
        assert_eq!(findings.len(), 1);
        assert!(findings.iter().any(|f| f.code == "COOP001"));
    }
}

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
                    code: "CORS001".to_string(),
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
                        code: "CORS002".to_string(),
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

// =========================================================================
// AriaLabelAnalyzer
// =========================================================================

pub struct AriaLabelAnalyzer;

impl Default for AriaLabelAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl AriaLabelAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for AriaLabelAnalyzer {
    fn name(&self) -> &str {
        "aria-label"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        // Check if there are ARIA roles but no labels
        if ctx.page.aria_role_count > 0 && ctx.page.aria_label_count == 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "ARIALABEL001".to_string(),
                title: "ARIA roles without labels".to_string(),
                description: format!(
                    "{} ARIA role(s) found but no aria-label or aria-labelledby \
                     attributes. Interactive elements need accessible names.",
                    ctx.page.aria_role_count
                ),
                url: url.clone(),
                recommendation: "Add aria-label or aria-labelledby to elements with \
                                 ARIA roles."
                    .to_string(),
            });
        }

        findings
    }
}

// =========================================================================
// TableCaptionAnalyzer
// =========================================================================

pub struct TableCaptionAnalyzer;

impl Default for TableCaptionAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl TableCaptionAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for TableCaptionAnalyzer {
    fn name(&self) -> &str {
        "table-caption"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        if ctx.page.tables_total == 0 {
            return findings;
        }

        let without_captions = ctx.page.tables_total - ctx.page.tables_with_captions;
        if without_captions > 0 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Accessibility,
                code: "TABLECAP001".to_string(),
                title: "Table missing caption element".to_string(),
                description: format!(
                    "{without_captions} of {} table(s) have no <caption> element. \
                     Captions help screen reader users understand table purpose.",
                    ctx.page.tables_total
                ),
                url: url.clone(),
                recommendation: "Add a <caption> element to each table to describe its \
                                 purpose."
                    .to_string(),
            });
        }

        findings
    }
}

// =========================================================================
// SkipLinkAnalyzer
// =========================================================================

pub struct SkipLinkAnalyzer;

impl Default for SkipLinkAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl SkipLinkAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for SkipLinkAnalyzer {
    fn name(&self) -> &str {
        "skip-link"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        if ctx.page.has_nav_landmark && !ctx.page.has_skip_link {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "SKIPLINK001".to_string(),
                title: "No skip navigation link".to_string(),
                description: "The page has a navigation landmark but no skip-to-content \
                              link. Keyboard users must tab through all navigation links \
                              to reach main content."
                    .to_string(),
                url: url.clone(),
                recommendation: "Add a skip link as the first focusable element pointing \
                                 to the main content area."
                    .to_string(),
            });
        }

        findings
    }
}

// =========================================================================
// TabindexAnalyzer
// =========================================================================

pub struct TabindexAnalyzer;

impl Default for TabindexAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl TabindexAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for TabindexAnalyzer {
    fn name(&self) -> &str {
        "tabindex"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        if ctx.page.has_positive_tabindex {
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Accessibility,
                code: "TABINDEX001".to_string(),
                title: "Positive tabindex values detected".to_string(),
                description: "Elements with tabindex > 0 alter the natural tab order, \
                              making keyboard navigation unpredictable. Users expect a \
                              sequential tab flow matching the visual layout."
                    .to_string(),
                url: url.clone(),
                recommendation: "Use tabindex=\"0\" to add elements to the natural tab \
                                 order or tabindex=\"-1\" for programmatic focus only."
                    .to_string(),
            });
        }

        if ctx.page.tabindex_negative_count > 0 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Accessibility,
                code: "TABINDEX002".to_string(),
                title: "Elements removed from tab order with tabindex=-1".to_string(),
                description: format!(
                    "{} element(s) use tabindex=-1, removing them from the tab \
                         order. This is acceptable for programmatically focused elements \
                         but should not be used to hide interactive content.",
                    ctx.page.tabindex_negative_count
                ),
                url: url.clone(),
                recommendation: "Ensure elements with tabindex=-1 are not interactive \
                                     elements that users need to reach."
                    .to_string(),
            });
        }

        findings
    }
}

// =========================================================================
// PermissionsPolicyAnalyzerV2
// =========================================================================

pub struct PermissionsPolicyAnalyzerV2;

impl Default for PermissionsPolicyAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl PermissionsPolicyAnalyzerV2 {
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

impl Analyzer for PermissionsPolicyAnalyzerV2 {
    fn name(&self) -> &str {
        "permissions-policy-v2"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        match Self::get_header(ctx.headers, "Permissions-Policy") {
            None => {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "PERM-V2001".to_string(),
                    title: "Permissions-Policy header missing".to_string(),
                    description: "No Permissions-Policy header was found.".to_string(),
                    url: url.to_string(),
                    recommendation: "Set a Permissions-Policy header.".into(),
                });
            }
            Some(policy) => {
                let lower = policy.to_lowercase();
                for feature in &["camera", "microphone", "geolocation", "payment"] {
                    if !lower.contains(&format!("{feature}=()"))
                        && !lower.contains(&format!("{feature}=(self)"))
                    {
                        findings.push(Finding {
                            severity: Severity::Info,
                            category: IssueCategory::Security,
                            code: "PERM-V2002".to_string(),
                            title: format!("Permissions-Policy does not restrict {feature}"),
                            description: format!(
                                "The Permissions-Policy header does not explicitly restrict \
                                 {feature} access."
                            ),
                            url: url.to_string(),
                            recommendation: format!("Add {feature}=() to Permissions-Policy."),
                        });
                    }
                }
            }
        }

        findings
    }
}

// =========================================================================
// FormInputLabelAnalyzer
// =========================================================================

pub struct FormInputLabelAnalyzer;

impl Default for FormInputLabelAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl FormInputLabelAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for FormInputLabelAnalyzer {
    fn name(&self) -> &str {
        "form-input-label"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for form in &ctx.page.forms {
            for input in &form.inputs {
                if !input.has_label {
                    let aria_has_name = input
                        .aria_label
                        .as_ref()
                        .is_some_and(|l| !l.trim().is_empty())
                        || input
                            .aria_labelledby
                            .as_ref()
                            .is_some_and(|l| !l.trim().is_empty());
                    if !aria_has_name {
                        let desc = match (&input.name, &input.input_type) {
                            (Some(n), Some(t)) => format!("input (name=\"{n}\", type=\"{t}\")"),
                            (Some(n), None) => format!("input (name=\"{n}\")"),
                            (None, Some(t)) => format!("input (type=\"{t}\")"),
                            (None, None) => "input".to_string(),
                        };
                        findings.push(Finding {
                            severity: Severity::Error,
                            category: IssueCategory::Accessibility,
                            code: "FILABEL001".to_string(),
                            title: "Form input missing associated label".to_string(),
                            description: format!(
                                "{desc} has no associated <label> element, aria-label, or \
                                 aria-labelledby attribute."
                            ),
                            url: url.to_string(),
                            recommendation: "Associate a <label> element with the input."
                                .to_string(),
                        });
                    }
                }
            }
        }

        findings
    }
}

// =========================================================================
// LinkTextAnalyzer
// =========================================================================

pub struct LinkTextAnalyzer;

impl Default for LinkTextAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl LinkTextAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for LinkTextAnalyzer {
    fn name(&self) -> &str {
        "link-text"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for link in &ctx.page.links {
            let text = link.text.trim();
            if text.is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Accessibility,
                    code: "LINKTEXT001".to_string(),
                    title: "Link with empty text".to_string(),
                    description: format!(
                        "A link to \"{}\" has no visible text content.",
                        link.href
                    ),
                    url: url.to_string(),
                    recommendation: "Add descriptive text content inside the <a> tag.".to_string(),
                });
                continue;
            }

            let lower = text.to_lowercase();
            let generic_texts = [
                "click here",
                "read more",
                "learn more",
                "here",
                "link",
                "more",
                "this",
                "continue",
            ];
            for generic in &generic_texts {
                if lower == *generic {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Accessibility,
                        code: "LINKTEXT002".to_string(),
                        title: "Link with generic text".to_string(),
                        description: format!(
                            "Link text \"{text}\" is generic and does not describe the destination."
                        ),
                        url: url.to_string(),
                        recommendation: "Replace generic text with descriptive text.".to_string(),
                    });
                    break;
                }
            }
        }

        findings
    }
}

// =========================================================================
// ImageAltTextAnalyzer
// =========================================================================

pub struct ImageAltTextAnalyzer;

impl Default for ImageAltTextAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageAltTextAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for ImageAltTextAnalyzer {
    fn name(&self) -> &str {
        "image-alt-text"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for image in &ctx.page.images {
            if !image.has_alt || image.alt.trim().is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Accessibility,
                    code: "IMGALT001".to_string(),
                    title: "Image missing alt text".to_string(),
                    description: format!(
                        "Image \"{}\" is missing an alt attribute or has empty alt text.",
                        image.src
                    ),
                    url: url.to_string(),
                    recommendation: "Add a descriptive alt attribute to the image.".to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// AriaRoleAnalyzer
// =========================================================================

pub struct AriaRoleAnalyzer;

impl Default for AriaRoleAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl AriaRoleAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for AriaRoleAnalyzer {
    fn name(&self) -> &str {
        "aria-role"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        if ctx.page.aria_role_count > 0 && ctx.page.aria_label_count == 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "ARIAROLE001".to_string(),
                title: "ARIA roles without accessible names".to_string(),
                description: format!(
                    "{} ARIA role(s) found but no aria-label or aria-labelledby attributes.",
                    ctx.page.aria_role_count
                ),
                url: url.to_string(),
                recommendation: "Add aria-label or aria-labelledby to all elements with ARIA roles."
                    .to_string(),
            });
        }

        findings
    }
}

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
                    description: "No Strict-Transport-Security header was found."
                        .to_string(),
                    url: url.to_string(),
                    recommendation: "Set Strict-Transport-Security: max-age=31536000; includeSubDomains; preload."
                        .to_string(),
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
                            recommendation: "Set max-age to at least 31536000 (1 year)."
                                .to_string(),
                        });
                    }
                }

                if !value.to_lowercase().contains("includesubdomains") {
                    findings.push(Finding {
                        severity: Severity::Info,
                        category: IssueCategory::Security,
                        code: "HSTS-V2003".to_string(),
                        title: "HSTS missing includeSubDomains".to_string(),
                        description: "The Strict-Transport-Security header does not include the includeSubDomains directive."
                            .to_string(),
                        url: url.to_string(),
                        recommendation: "Add includeSubDomains to the HSTS header."
                            .to_string(),
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
            None => {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Security,
                    code: "XSS-V2001".to_string(),
                    title: "Missing X-XSS-Protection header".to_string(),
                    description: "No X-XSS-Protection header was found.".to_string(),
                    url: url.to_string(),
                    recommendation: "Set X-XSS-Protection: 1; mode=block.".to_string(),
                });
            }
            Some(value) => {
                if value.trim() == "0" {
                    findings.push(Finding {
                        severity: Severity::Info,
                        category: IssueCategory::Security,
                        code: "XSS-V2002".to_string(),
                        title: "X-XSS-Protection explicitly disabled".to_string(),
                        description:
                            "X-XSS-Protection is set to 0, explicitly disabling the XSS auditor."
                                .to_string(),
                        url: url.to_string(),
                        recommendation: "Ensure Content-Security-Policy is properly configured."
                            .to_string(),
                    });
                }
            }
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
            None => {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "CT-V2001".to_string(),
                    title: "Missing X-Content-Type-Options header".to_string(),
                    description: "No X-Content-Type-Options header was found.".to_string(),
                    url: url.to_string(),
                    recommendation: "Set X-Content-Type-Options: nosniff.".to_string(),
                });
            }
            Some(value) => {
                if value.trim().to_lowercase() != "nosniff" {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Security,
                        code: "CT-V2002".to_string(),
                        title: "Invalid X-Content-Type-Options value".to_string(),
                        description: format!(
                            "X-Content-Type-Options is \"{value}\" instead of \"nosniff\"."
                        ),
                        url: url.to_string(),
                        recommendation: "Set X-Content-Type-Options: nosniff.".to_string(),
                    });
                }
            }
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
                let max_age_ok = lower.find("max-age=").map_or(false, |pos| {
                    let after = &lower[pos + 8..];
                    let num: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
                    num.parse::<u64>().map_or(false, |a| a >= 31536000)
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

// =========================================================================
// CspDirectiveValidator — CSPDIR001
// =========================================================================

pub struct CspDirectiveValidator;
impl Default for CspDirectiveValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl CspDirectiveValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for CspDirectiveValidator {
    fn name(&self) -> &str {
        "csp-directive-validator"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let csp = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Content-Security-Policy"))
            .map(|(_, v)| v.as_str());
        if let Some(value) = csp {
            let lower = value.to_lowercase();
            let recommended = [
                "default-src",
                "script-src",
                "style-src",
                "img-src",
                "connect-src",
            ];
            for dir in &recommended {
                if !lower.contains(dir) {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Security,
                        code: "CSPDIR001".to_string(),
                        title: format!("CSP missing {dir} directive"),
                        description: format!("The Content-Security-Policy header does not include a '{dir}' directive. Without it, resources of this type are governed by default-src or are unrestricted."),
                        url: url.to_string(),
                        recommendation: format!("Add a '{dir}' directive to the Content-Security-Policy header."),
                    });
                }
            }
        }
        findings
    }
}

// {name} extracted to cookies.rs (Phase 2 SRP step).

// =========================================================================

pub struct MixedContentFormValidator;
impl Default for MixedContentFormValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl MixedContentFormValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for MixedContentFormValidator {
    fn name(&self) -> &str {
        "mixed-content-form"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if !url.starts_with("https://") {
            return findings;
        }
        if let Some(body) = ctx.body {
            let prefix_dq = "action=\"http://";
            let prefix_sq = "action='http://";
            // For double-quote prefix
            {
                let mut remaining = body;
                while let Some(pos) = remaining.find(prefix_dq) {
                    let start = pos + prefix_dq.len();
                    if let Some(end) = remaining[start..].find('"') {
                        findings.push(Finding {
                            severity: Severity::Error,
                            category: IssueCategory::Security,
                            code: "MIXFRM001".to_string(),
                            title: "HTTP form action on HTTPS page".to_string(),
                            description: "A form has an action attribute using HTTP on an HTTPS page. User data will be transmitted in plaintext.".to_string(),
                            url: url.to_string(),
                            recommendation: "Change the form action URL to HTTPS.".to_string(),
                        });
                        remaining = &remaining[start + end + 1..];
                    } else {
                        break;
                    }
                }
            }
            // For single-quote prefix
            {
                let mut remaining = body;
                while let Some(pos) = remaining.find(prefix_sq) {
                    let start = pos + prefix_sq.len();
                    if let Some(end) = remaining[start..].find('\'') {
                        findings.push(Finding {
                            severity: Severity::Error,
                            category: IssueCategory::Security,
                            code: "MIXFRM001".to_string(),
                            title: "HTTP form action on HTTPS page".to_string(),
                            description: "A form has an action attribute using HTTP on an HTTPS page. User data will be transmitted in plaintext.".to_string(),
                            url: url.to_string(),
                            recommendation: "Change the form action URL to HTTPS.".to_string(),
                        });
                        remaining = &remaining[start + end + 1..];
                    } else {
                        break;
                    }
                }
            }
        }
        findings
    }
}

// =========================================================================
// MixedContentScriptValidator — MIXSCR001
// =========================================================================

pub struct MixedContentScriptValidator;
impl Default for MixedContentScriptValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl MixedContentScriptValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for MixedContentScriptValidator {
    fn name(&self) -> &str {
        "mixed-content-script"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if !url.starts_with("https://") {
            return findings;
        }
        if let Some(body) = ctx.body {
            let prefix_dq = "src=\"http://";
            let prefix_sq = "src='http://";
            let mut count = 0;
            // For double-quote prefix
            {
                let mut remaining = body;
                while let Some(pos) = remaining.find(prefix_dq) {
                    let start = pos + prefix_dq.len();
                    if let Some(end) = remaining[start..].find('"') {
                        let res = remaining[start..start + end].to_lowercase();
                        if res.ends_with(".js") || res.contains("/script") {
                            count += 1;
                        }
                        remaining = &remaining[start + end + 1..];
                    } else {
                        break;
                    }
                }
            }
            // For single-quote prefix
            {
                let mut remaining = body;
                while let Some(pos) = remaining.find(prefix_sq) {
                    let start = pos + prefix_sq.len();
                    if let Some(end) = remaining[start..].find('\'') {
                        let res = remaining[start..start + end].to_lowercase();
                        if res.ends_with(".js") || res.contains("/script") {
                            count += 1;
                        }
                        remaining = &remaining[start + end + 1..];
                    } else {
                        break;
                    }
                }
            }
            if count > 0 {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Security,
                    code: "MIXSCR001".to_string(),
                    title: "HTTP script sources on HTTPS page".to_string(),
                    description: format!("{count} script(s) loaded over HTTP on an HTTPS page. Browsers may block mixed active content."),
                    url: url.to_string(),
                    recommendation: "Update script URLs to use HTTPS.".to_string(),
                });
            }
        }
        findings
    }
}

// =========================================================================
// MixedContentImageValidator — MIXIMG001
// =========================================================================

pub struct MixedContentImageValidator;
impl Default for MixedContentImageValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl MixedContentImageValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for MixedContentImageValidator {
    fn name(&self) -> &str {
        "mixed-content-image"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if !url.starts_with("https://") {
            return findings;
        }
        if let Some(body) = ctx.body {
            let prefix_dq = "src=\"http://";
            let prefix_sq = "src='http://";
            let img_exts = [
                ".jpg", ".jpeg", ".png", ".gif", ".svg", ".webp", ".ico", ".bmp", ".tiff",
            ];
            let mut count = 0;
            // For double-quote prefix
            {
                let mut remaining = body;
                while let Some(pos) = remaining.find(prefix_dq) {
                    let start = pos + prefix_dq.len();
                    if let Some(end) = remaining[start..].find('"') {
                        let res = remaining[start..start + end].to_lowercase();
                        if img_exts.iter().any(|ext| res.ends_with(ext)) {
                            count += 1;
                        }
                        remaining = &remaining[start + end + 1..];
                    } else {
                        break;
                    }
                }
            }
            // For single-quote prefix
            {
                let mut remaining = body;
                while let Some(pos) = remaining.find(prefix_sq) {
                    let start = pos + prefix_sq.len();
                    if let Some(end) = remaining[start..].find('\'') {
                        let res = remaining[start..start + end].to_lowercase();
                        if img_exts.iter().any(|ext| res.ends_with(ext)) {
                            count += 1;
                        }
                        remaining = &remaining[start + end + 1..];
                    } else {
                        break;
                    }
                }
            }
            if count > 0 {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "MIXIMG001".to_string(),
                    title: "HTTP image sources on HTTPS page".to_string(),
                    description: format!("{count} image(s) loaded over HTTP on an HTTPS page. Mixed passive content degrades HTTPS security."),
                    url: url.to_string(),
                    recommendation: "Update image URLs to use HTTPS.".to_string(),
                });
            }
        }
        findings
    }
}

// =========================================================================
// LandmarkMainAnalyzer — LANDMAIN001
// =========================================================================

pub struct LandmarkMainAnalyzer;
impl Default for LandmarkMainAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
impl LandmarkMainAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for LandmarkMainAnalyzer {
    fn name(&self) -> &str {
        "landmark-main"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        if !ctx.page.has_main_landmark {
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Accessibility,
                code: "LANDMAIN001".to_string(),
                title: "Page missing main landmark".to_string(),
                description: "No <main> element or role=\"main\" found. Screen readers use landmarks for quick navigation.".to_string(),
                url: ctx.page.url.to_string(),
                recommendation: "Wrap primary content in a <main> element.".to_string(),
            });
        }
        findings
    }
}

// =========================================================================
// LandmarkNavAnalyzer — LANDNAV001
// =========================================================================

pub struct LandmarkNavAnalyzer;
impl Default for LandmarkNavAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
impl LandmarkNavAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for LandmarkNavAnalyzer {
    fn name(&self) -> &str {
        "landmark-nav"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        if !ctx.page.has_nav_landmark {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "LANDNAV001".to_string(),
                title: "Page missing navigation landmark".to_string(),
                description: "No <nav> element or role=\"navigation\" found. Navigation landmarks help screen reader users.".to_string(),
                url: ctx.page.url.to_string(),
                recommendation: "Wrap navigation links in a <nav> element.".to_string(),
            });
        }
        findings
    }
}

// =========================================================================
// LandmarkBannerAnalyzer — LANDBAN001
// =========================================================================

pub struct LandmarkBannerAnalyzer;
impl Default for LandmarkBannerAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
impl LandmarkBannerAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for LandmarkBannerAnalyzer {
    fn name(&self) -> &str {
        "landmark-banner"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let has_banner = ctx
            .page
            .landmarks
            .iter()
            .any(|l| l == "banner" || l == "header");
        if !has_banner {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Accessibility,
                code: "LANDBAN001".to_string(),
                title: "Page missing banner/header landmark".to_string(),
                description: "No <header> element or role=\"banner\" found. The banner landmark contains site-wide content like logo and navigation.".to_string(),
                url: ctx.page.url.to_string(),
                recommendation: "Wrap the site header in a <header> element.".to_string(),
            });
        }
        findings
    }
}

// =========================================================================
// HeadingLevelSkipAnalyzer — HEADSKIP001
// =========================================================================

pub struct HeadingLevelSkipAnalyzer;
impl Default for HeadingLevelSkipAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
impl HeadingLevelSkipAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for HeadingLevelSkipAnalyzer {
    fn name(&self) -> &str {
        "heading-level-skip"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.headings.len() < 2 {
            return findings;
        }
        let mut prev_level: Option<u8> = None;
        for heading in &ctx.page.headings {
            if let Some(prev) = prev_level {
                if heading.level > prev + 1 {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Accessibility,
                        code: "HEADSKIP001".to_string(),
                        title: "Heading level skip detected".to_string(),
                        description: format!(
                            "Heading jumps from H{prev} to H{}, skipping intermediate levels.",
                            heading.level
                        ),
                        url: url.to_string(),
                        recommendation: format!(
                            "Use H{} after H{prev} to maintain document outline.",
                            prev + 1
                        ),
                    });
                    break;
                }
            }
            prev_level = Some(heading.level);
        }
        findings
    }
}

// =========================================================================
// FormLabelAssociationAnalyzer — FORMLAB001
// =========================================================================

pub struct FormLabelAssociationAnalyzer;
impl Default for FormLabelAssociationAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
impl FormLabelAssociationAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for FormLabelAssociationAnalyzer {
    fn name(&self) -> &str {
        "form-label-association"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let mut unlabeled = 0;
        for form in &ctx.page.forms {
            for input in &form.inputs {
                if !input.has_label
                    && input
                        .aria_label
                        .as_ref()
                        .is_none_or(|l| l.trim().is_empty())
                    && input
                        .aria_labelledby
                        .as_ref()
                        .is_none_or(|l| l.trim().is_empty())
                {
                    unlabeled += 1;
                }
            }
        }
        if unlabeled > 0 {
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Accessibility,
                code: "FORMLAB001".to_string(),
                title: "Form inputs missing label associations".to_string(),
                description: format!("{unlabeled} form input(s) have no associated <label>, aria-label, or aria-labelledby."),
                url: url.to_string(),
                recommendation: "Associate a <label> element with each input using for/id attributes, or add aria-label.".to_string(),
            });
        }
        findings
    }
}

// =========================================================================
// TableHeaderScopeAnalyzer — TBLSCOP001
// =========================================================================

pub struct TableHeaderScopeAnalyzer;
impl Default for TableHeaderScopeAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
impl TableHeaderScopeAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for TableHeaderScopeAnalyzer {
    fn name(&self) -> &str {
        "table-header-scope"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.tables_total == 0 {
            return findings;
        }
        let without = ctx
            .page
            .tables_total
            .saturating_sub(ctx.page.tables_with_headers);
        if without > 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "TBLSCOP001".to_string(),
                title: "Tables missing header cells with scope".to_string(),
                description: format!("{without} of {} table(s) have no <th> header cells. Header cells with scope attributes clarify data relationships.", ctx.page.tables_total),
                url: url.to_string(),
                recommendation: "Use <th scope=\"col\"> for column headers and <th scope=\"row\"> for row headers.".to_string(),
            });
        }
        findings
    }
}

// =========================================================================
// TableCaptionPresenceAnalyzer — TBLCAP001
// =========================================================================

pub struct TableCaptionPresenceAnalyzer;
impl Default for TableCaptionPresenceAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
impl TableCaptionPresenceAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for TableCaptionPresenceAnalyzer {
    fn name(&self) -> &str {
        "table-caption-presence"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.tables_total == 0 {
            return findings;
        }
        let without = ctx
            .page
            .tables_total
            .saturating_sub(ctx.page.tables_with_captions);
        if without > 0 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Accessibility,
                code: "TBLCAP001".to_string(),
                title: "Tables missing caption element".to_string(),
                description: format!("{without} of {} table(s) have no <caption>. Captions describe table purpose for screen readers.", ctx.page.tables_total),
                url: url.to_string(),
                recommendation: "Add a <caption> element to each data table.".to_string(),
            });
        }
        findings
    }
}

// =========================================================================
// AnchorTextGenericAnalyzer — ANCHGEN001
// =========================================================================

pub struct AnchorTextGenericAnalyzer;
impl Default for AnchorTextGenericAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
impl AnchorTextGenericAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for AnchorTextGenericAnalyzer {
    fn name(&self) -> &str {
        "anchor-text-generic"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let generic = [
            "click here",
            "read more",
            "learn more",
            "here",
            "more",
            "link",
            "this",
            "continue",
            "go",
        ];
        for link in &ctx.page.links {
            let text = link.text.trim().to_lowercase();
            if !text.is_empty() && generic.contains(&text.as_str()) {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Accessibility,
                    code: "ANCHGEN001".to_string(),
                    title: "Link with generic anchor text".to_string(),
                    description: format!(
                        "Link text \"{}\" is generic and does not describe the destination.",
                        link.text.trim()
                    ),
                    url: url.to_string(),
                    recommendation: "Use descriptive text that explains the link purpose."
                        .to_string(),
                });
            }
        }
        findings
    }
}

// =========================================================================
// AriaRequiredAttributesAnalyzer — ARIAREQ001
// =========================================================================

pub struct AriaRequiredAttributesAnalyzer;
impl Default for AriaRequiredAttributesAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
impl AriaRequiredAttributesAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for AriaRequiredAttributesAnalyzer {
    fn name(&self) -> &str {
        "aria-required-attributes"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.aria_role_count > 0 && ctx.page.aria_label_count == 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "ARIAREQ001".to_string(),
                title: "ARIA roles missing required accessible name attributes".to_string(),
                description: format!("{} ARIA role(s) found without aria-label or aria-labelledby. Roles require accessible names for screen readers.", ctx.page.aria_role_count),
                url: url.to_string(),
                recommendation: "Add aria-label or aria-labelledby to all elements with ARIA roles.".to_string(),
            });
        }
        findings
    }
}

// =========================================================================
// FocusOrderPositiveTabindexAnalyzer — TABPOS001
// =========================================================================

pub struct FocusOrderPositiveTabindexAnalyzer;
impl Default for FocusOrderPositiveTabindexAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
impl FocusOrderPositiveTabindexAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for FocusOrderPositiveTabindexAnalyzer {
    fn name(&self) -> &str {
        "focus-order-positive-tabindex"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        if ctx.page.has_positive_tabindex {
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Accessibility,
                code: "TABPOS001".to_string(),
                title: "Positive tabindex values disrupt focus order".to_string(),
                description: "Elements with tabindex > 0 alter the natural tab order, making keyboard navigation unpredictable.".to_string(),
                url: ctx.page.url.to_string(),
                recommendation: "Use tabindex=\"0\" for natural order or tabindex=\"-1\" for programmatic focus.".to_string(),
            });
        }
        findings
    }
}

// =========================================================================
// ColorContrastTextAnalyzer — COLRCT001
// =========================================================================

pub struct ColorContrastTextAnalyzer;
impl Default for ColorContrastTextAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
impl ColorContrastTextAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl ColorContrastTextAnalyzer {
    fn parse_hex(hex: &str) -> Option<(u8, u8, u8)> {
        let h = hex.trim().trim_start_matches('#');
        match h.len() {
            3 => {
                let r = u8::from_str_radix(&h[0..1], 16).ok()?;
                let g = u8::from_str_radix(&h[1..2], 16).ok()?;
                let b = u8::from_str_radix(&h[2..3], 16).ok()?;
                Some((r * 17, g * 17, b * 17))
            }
            6 => {
                let r = u8::from_str_radix(&h[0..2], 16).ok()?;
                let g = u8::from_str_radix(&h[2..4], 16).ok()?;
                let b = u8::from_str_radix(&h[4..6], 16).ok()?;
                Some((r, g, b))
            }
            _ => None,
        }
    }

    fn relative_luminance(r: u8, g: u8, b: u8) -> f64 {
        let f = |c: u8| -> f64 {
            let s = c as f64 / 255.0;
            if s <= 0.03928 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b)
    }

    fn contrast_ratio(fg: (u8, u8, u8), bg: (u8, u8, u8)) -> f64 {
        let l1 = Self::relative_luminance(fg.0, fg.1, fg.2);
        let l2 = Self::relative_luminance(bg.0, bg.1, bg.2);
        let (lighter, darker) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
        (lighter + 0.05) / (darker + 0.05)
    }

    fn extract_text_color_pairs(body: &str) -> Vec<((u8, u8, u8), (u8, u8, u8))> {
        use regex::Regex;
        let re = Regex::new(r#"style\s*=\s*["'][^"']*color\s*:\s*(#[0-9a-fA-F]{3,6})[^"']*background(?:-color)?\s*:\s*(#[0-9a-fA-F]{3,6})["']"#)
            .unwrap_or_else(|_| Regex::new("x^").unwrap());
        let mut pairs = Vec::new();
        for cap in re.captures_iter(body) {
            if let (Some(fg), Some(bg)) = (Self::parse_hex(&cap[1]), Self::parse_hex(&cap[2])) {
                pairs.push((fg, bg));
            }
        }
        let re2 = Regex::new(r#"style\s*=\s*["'][^"']*background(?:-color)?\s*:\s*(#[0-9a-fA-F]{3,6})[^"']*color\s*:\s*(#[0-9a-fA-F]{3,6})["']"#)
            .unwrap_or_else(|_| Regex::new("x^").unwrap());
        for cap in re2.captures_iter(body) {
            if let (Some(bg), Some(fg)) = (Self::parse_hex(&cap[1]), Self::parse_hex(&cap[2])) {
                pairs.push((fg, bg));
            }
        }
        pairs
    }
}

impl Analyzer for ColorContrastTextAnalyzer {
    fn name(&self) -> &str {
        "color-contrast-text"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let body = ctx.body.unwrap_or("");
        let pairs = Self::extract_text_color_pairs(body);
        let mut low = 0;
        for (fg, bg) in &pairs {
            let ratio = Self::contrast_ratio(*fg, *bg);
            if ratio < 4.5 {
                low += 1;
            }
        }
        if low > 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "COLRCT001".to_string(),
                title: "Low text color contrast ratio".to_string(),
                description: format!("{low} inline style(s) have a contrast ratio below 4.5:1. WCAG 1.4.3 requires at least 4.5:1 for normal text."),
                url: url.to_string(),
                recommendation: "Ensure text color contrasts at least 4.5:1 with its background.".to_string(),
            });
        }
        findings
    }
}

// =========================================================================
// ColorContrastLinkAnalyzer — COLRCL001
// =========================================================================

pub struct ColorContrastLinkAnalyzer;
impl Default for ColorContrastLinkAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
impl ColorContrastLinkAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl ColorContrastLinkAnalyzer {
    fn parse_hex(hex: &str) -> Option<(u8, u8, u8)> {
        let h = hex.trim().trim_start_matches('#');
        match h.len() {
            3 => {
                let r = u8::from_str_radix(&h[0..1], 16).ok()?;
                let g = u8::from_str_radix(&h[1..2], 16).ok()?;
                let b = u8::from_str_radix(&h[2..3], 16).ok()?;
                Some((r * 17, g * 17, b * 17))
            }
            6 => {
                let r = u8::from_str_radix(&h[0..2], 16).ok()?;
                let g = u8::from_str_radix(&h[2..4], 16).ok()?;
                let b = u8::from_str_radix(&h[4..6], 16).ok()?;
                Some((r, g, b))
            }
            _ => None,
        }
    }

    fn relative_luminance(r: u8, g: u8, b: u8) -> f64 {
        let f = |c: u8| -> f64 {
            let s = c as f64 / 255.0;
            if s <= 0.03928 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b)
    }

    fn contrast_ratio(fg: (u8, u8, u8), bg: (u8, u8, u8)) -> f64 {
        let l1 = Self::relative_luminance(fg.0, fg.1, fg.2);
        let l2 = Self::relative_luminance(bg.0, bg.1, bg.2);
        let (lighter, darker) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
        (lighter + 0.05) / (darker + 0.05)
    }

    fn extract_link_color_pairs(body: &str) -> Vec<((u8, u8, u8), (u8, u8, u8))> {
        use regex::Regex;
        let re = Regex::new(r#"style\s*=\s*["'][^"']*color\s*:\s*(#[0-9a-fA-F]{3,6})[^"']*background(?:-color)?\s*:\s*(#[0-9a-fA-F]{3,6})["']"#)
            .unwrap_or_else(|_| Regex::new("x^").unwrap());
        let mut pairs = Vec::new();
        for cap in re.captures_iter(body) {
            if let (Some(fg), Some(bg)) = (Self::parse_hex(&cap[1]), Self::parse_hex(&cap[2])) {
                pairs.push((fg, bg));
            }
        }
        pairs
    }
}

impl Analyzer for ColorContrastLinkAnalyzer {
    fn name(&self) -> &str {
        "color-contrast-link"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let body = ctx.body.unwrap_or("");
        let pairs = Self::extract_link_color_pairs(body);
        let mut low = 0;
        for (fg, bg) in &pairs {
            let ratio = Self::contrast_ratio(*fg, *bg);
            if ratio < 3.0 {
                low += 1;
            }
        }
        if low > 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "COLRCL001".to_string(),
                title: "Link color contrast too low".to_string(),
                description: format!("{low} color pair(s) have a contrast ratio below 3:1, making links difficult to distinguish from surrounding text."),
                url: url.to_string(),
                recommendation: "Ensure link colors contrast at least 3:1 with the background and 3:1 with surrounding text.".to_string(),
            });
        }
        findings
    }
}

// ---------------------------------------------------------------------------
// Security: Referrer-Policy V2 — missing header
// ---------------------------------------------------------------------------

pub struct ReferrerPolicyAnalyzerV2;

impl Default for ReferrerPolicyAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl ReferrerPolicyAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for ReferrerPolicyAnalyzerV2 {
    fn name(&self) -> &str {
        "referrer-policy-v2"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let has = ctx
            .headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("Referrer-Policy"));
        if !has {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Security,
                code: "REF-V2001".to_string(),
                title: "Missing Referrer-Policy header".to_string(),
                description: "No Referrer-Policy header was found. This header controls how much referrer information is sent with requests.".into(),
                url: url.clone(),
                recommendation: "Add Referrer-Policy: strict-origin-when-cross-origin or no-referrer.".into(),
            });
        }
        findings
    }
}

// ---------------------------------------------------------------------------
// Security: X-Frame-Options V2 — missing header
// ---------------------------------------------------------------------------

pub struct XFrameOptionsAnalyzerV2;

impl Default for XFrameOptionsAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl XFrameOptionsAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for XFrameOptionsAnalyzerV2 {
    fn name(&self) -> &str {
        "x-frame-options-v2"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let has = ctx
            .headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("X-Frame-Options"));
        if !has {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Security,
                code: "XFO-V2001".to_string(),
                title: "Missing X-Frame-Options header".to_string(),
                description: "No X-Frame-Options header was found. This header prevents clickjacking by controlling frame embedding.".into(),
                url: url.clone(),
                recommendation: "Set X-Frame-Options to DENY or SAMEORIGIN.".into(),
            });
        }
        findings
    }
}

// ---------------------------------------------------------------------------
// Security: CSP V2 — missing script-src
// ---------------------------------------------------------------------------

pub struct ContentSecurityPolicyAnalyzerV2;

impl Default for ContentSecurityPolicyAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl ContentSecurityPolicyAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for ContentSecurityPolicyAnalyzerV2 {
    fn name(&self) -> &str {
        "content-security-policy-v2"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let csp = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Content-Security-Policy"))
            .map(|(_, v)| v.as_str());
        match csp {
            None => {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Security,
                    code: "CSP-V2001".to_string(),
                    title: "CSP missing script-src directive".to_string(),
                    description: "No Content-Security-Policy header with script-src was found. CSP script-src helps prevent XSS attacks.".into(),
                    url: url.clone(),
                    recommendation: "Add Content-Security-Policy with a script-src directive (e.g., script-src 'self').".into(),
                });
            }
            Some(val) => {
                if !val.contains("script-src") {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Security,
                        code: "CSP-V2001".to_string(),
                        title: "CSP missing script-src directive".to_string(),
                        description: "Content-Security-Policy header is present but does not include a script-src directive.".into(),
                        url: url.clone(),
                        recommendation: "Add script-src directive to the Content-Security-Policy header.".into(),
                    });
                }
            }
        }
        findings
    }
}

// ---------------------------------------------------------------------------
// Security: HSTS V3 — missing includeSubDomains
// ---------------------------------------------------------------------------

pub struct StrictTransportSecurityAnalyzerV3;

impl Default for StrictTransportSecurityAnalyzerV3 {
    fn default() -> Self {
        Self::new()
    }
}

impl StrictTransportSecurityAnalyzerV3 {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for StrictTransportSecurityAnalyzerV3 {
    fn name(&self) -> &str {
        "strict-transport-security-v3"
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
                    code: "HSTS-V3001".to_string(),
                    title: "HSTS missing includeSubDomains".to_string(),
                    description: "No Strict-Transport-Security header with includeSubDomains was found.".into(),
                    url: url.clone(),
                    recommendation: "Add Strict-Transport-Security: max-age=31536000; includeSubDomains; preload.".into(),
                });
            }
            Some(val) => {
                if !val.to_lowercase().contains("includesubdomains") {
                    findings.push(Finding {
                        severity: Severity::Info,
                        category: IssueCategory::Security,
                        code: "HSTS-V3001".to_string(),
                        title: "HSTS missing includeSubDomains".to_string(),
                        description: "The Strict-Transport-Security header does not include the includeSubDomains directive.".into(),
                        url: url.clone(),
                        recommendation: "Add includeSubDomains to protect all subdomains.".into(),
                    });
                }
            }
        }
        findings
    }
}

// ---------------------------------------------------------------------------
// Security: X-Content-Type-Options V2 — missing header
// ---------------------------------------------------------------------------

pub struct XContentTypeOptionsAnalyzerV2;

impl Default for XContentTypeOptionsAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl XContentTypeOptionsAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for XContentTypeOptionsAnalyzerV2 {
    fn name(&self) -> &str {
        "x-content-type-options-v2"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let has = ctx
            .headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("X-Content-Type-Options"));
        if !has {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Security,
                code: "XCTO-V2001".to_string(),
                title: "Missing X-Content-Type-Options header".to_string(),
                description: "No X-Content-Type-Options header was found. This header prevents MIME-type sniffing.".into(),
                url: url.clone(),
                recommendation: "Set X-Content-Type-Options to nosniff.".into(),
            });
        }
        findings
    }
}

// ---------------------------------------------------------------------------
// Security: Permissions-Policy V3 — missing camera
// ---------------------------------------------------------------------------

pub struct PermissionsPolicyAnalyzerV3;

impl Default for PermissionsPolicyAnalyzerV3 {
    fn default() -> Self {
        Self::new()
    }
}

impl PermissionsPolicyAnalyzerV3 {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for PermissionsPolicyAnalyzerV3 {
    fn name(&self) -> &str {
        "permissions-policy-v3"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let pp = ctx
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Permissions-Policy"))
            .map(|(_, v)| v.as_str());
        match pp {
            None => {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Security,
                    code: "PERM-V3001".to_string(),
                    title: "Permissions-Policy missing camera restriction".to_string(),
                    description: "No Permissions-Policy header was found. Without it, the camera API may be accessible by default.".into(),
                    url: url.clone(),
                    recommendation: "Add Permissions-Policy header with camera=() to disable camera access if not needed.".into(),
                });
            }
            Some(val) => {
                let lower = val.to_lowercase();
                if !lower.contains("camera=()") {
                    findings.push(Finding {
                        severity: Severity::Info,
                        category: IssueCategory::Security,
                        code: "PERM-V3001".to_string(),
                        title: "Permissions-Policy missing camera restriction".to_string(),
                        description: "The Permissions-Policy header does not explicitly restrict camera access.".into(),
                        url: url.clone(),
                        recommendation: "Add camera=() to Permissions-Policy to disable camera access if not needed.".into(),
                    });
                }
            }
        }
        findings
    }
}

// ---------------------------------------------------------------------------
// Security: COEP V2 — missing header
// ---------------------------------------------------------------------------

pub struct CrossOriginIsolationAnalyzerV2;

impl Default for CrossOriginIsolationAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl CrossOriginIsolationAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for CrossOriginIsolationAnalyzerV2 {
    fn name(&self) -> &str {
        "cross-origin-isolation-v2"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let has = ctx
            .headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("Cross-Origin-Embedder-Policy"));
        if !has {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Security,
                code: "COEP-V2001".to_string(),
                title: "Missing Cross-Origin-Embedder-Policy header".to_string(),
                description: "No Cross-Origin-Embedder-Policy header was found. COEP prevents resources from loading cross-origin without explicit permission.".into(),
                url: url.clone(),
                recommendation: "Set Cross-Origin-Embedder-Policy to require-corp for stricter cross-origin isolation.".into(),
            });
        }
        findings
    }
}

// ---------------------------------------------------------------------------
// Security: COOP V2 — missing header
// ---------------------------------------------------------------------------

pub struct CrossOriginOpenerPolicyAnalyzerV2;

impl Default for CrossOriginOpenerPolicyAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl CrossOriginOpenerPolicyAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for CrossOriginOpenerPolicyAnalyzerV2 {
    fn name(&self) -> &str {
        "cross-origin-opener-policy-v2"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let has = ctx
            .headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("Cross-Origin-Opener-Policy"));
        if !has {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Security,
                code: "COOP-V2001".to_string(),
                title: "Missing Cross-Origin-Opener-Policy header".to_string(),
                description: "No Cross-Origin-Opener-Policy header was found. COOP isolates your browsing context from cross-origin popups.".into(),
                url: url.clone(),
                recommendation: "Set Cross-Origin-Opener-Policy to same-origin for stricter isolation.".into(),
            });
        }
        findings
    }
}

// ---------------------------------------------------------------------------
// Accessibility: Tabindex V2 — positive tabindex values
// ---------------------------------------------------------------------------

pub struct TabindexAnalyzerV2;

impl Default for TabindexAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl TabindexAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for TabindexAnalyzerV2 {
    fn name(&self) -> &str {
        "tabindex-v2"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.has_positive_tabindex {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "TAB-V2001".to_string(),
                title: "Positive tabindex values found".to_string(),
                description: "Elements with positive tabindex values alter the natural tab order, which can confuse keyboard users.".into(),
                url: url.clone(),
                recommendation: "Use tabindex=\"0\" or tabindex=\"-1\" instead of positive values. Restructure DOM order to achieve the desired tab sequence.".into(),
            });
        }
        findings
    }
}

// ---------------------------------------------------------------------------
// Accessibility: Link Accessibility V2 — links with empty text
// ---------------------------------------------------------------------------

pub struct LinkAccessibilityAnalyzerV2;

impl Default for LinkAccessibilityAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl LinkAccessibilityAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for LinkAccessibilityAnalyzerV2 {
    fn name(&self) -> &str {
        "link-accessibility-v2"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let empty_text_links: Vec<&str> = ctx
            .page
            .links
            .iter()
            .filter(|l| {
                l.text.trim().is_empty() && l.aria_label.as_deref().unwrap_or("").is_empty()
            })
            .map(|l| l.href.as_str())
            .collect();
        if !empty_text_links.is_empty() {
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Accessibility,
                code: "LINK-V2001".to_string(),
                title: "Links with empty text".to_string(),
                description: format!("{} link(s) have no visible text or aria-label: {}.", empty_text_links.len(), empty_text_links.iter().take(3).cloned().collect::<Vec<_>>().join(", ")),
                url: url.clone(),
                recommendation: "Add descriptive link text, an aria-label, or an img with alt text inside each link.".into(),
            });
        }
        findings
    }
}

// ---------------------------------------------------------------------------
// Accessibility: Image Accessibility V2 — images missing alt
// ---------------------------------------------------------------------------

pub struct ImageAccessibilityAnalyzerV2;

impl Default for ImageAccessibilityAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageAccessibilityAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for ImageAccessibilityAnalyzerV2 {
    fn name(&self) -> &str {
        "image-accessibility-v2"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let missing_alt: Vec<&str> = ctx
            .page
            .images
            .iter()
            .filter(|i| !i.has_alt)
            .map(|i| i.src.as_str())
            .collect();
        if !missing_alt.is_empty() {
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Accessibility,
                code: "IMG-V2001".to_string(),
                title: "Images missing alt attribute".to_string(),
                description: format!("{} image(s) have no alt attribute: {}.", missing_alt.len(), missing_alt.iter().take(3).cloned().collect::<Vec<_>>().join(", ")),
                url: url.clone(),
                recommendation: "Add an alt attribute to every img. Use descriptive text for meaningful images and alt=\"\" for decorative ones.".into(),
            });
        }
        findings
    }
}

// ---------------------------------------------------------------------------
// Accessibility: Form Accessibility V2 — forms without labels
// ---------------------------------------------------------------------------

pub struct FormAccessibilityAnalyzerV2;

impl Default for FormAccessibilityAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl FormAccessibilityAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for FormAccessibilityAnalyzerV2 {
    fn name(&self) -> &str {
        "form-accessibility-v2"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let body = ctx.body.unwrap_or("");
        let has_labels = body.contains("<label") || body.contains("aria-label");
        let has_inputs = ctx.page.forms.iter().any(|f| !f.inputs.is_empty());
        if has_inputs && !has_labels {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "FORM-V2001".to_string(),
                title: "Forms without labels".to_string(),
                description: "Form inputs were found but no <label> or aria-label attributes were detected. Labels are essential for screen reader users.".into(),
                url: url.clone(),
                recommendation: "Add <label> elements associated via for/id, or use aria-label/aria-labelledby on each input.".into(),
            });
        }
        findings
    }
}

// ---------------------------------------------------------------------------
// Accessibility: Table Accessibility V2 — tables without headers
// ---------------------------------------------------------------------------

pub struct TableAccessibilityAnalyzerV2;

impl Default for TableAccessibilityAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl TableAccessibilityAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for TableAccessibilityAnalyzerV2 {
    fn name(&self) -> &str {
        "table-accessibility-v2"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.tables_total > 0 && ctx.page.tables_with_headers == 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "TBL-V2001".to_string(),
                title: "Tables without headers".to_string(),
                description: format!("{} table(s) found but none have <th> header cells. Screen readers use headers to describe cell relationships.", ctx.page.tables_total),
                url: url.clone(),
                recommendation: "Add <th> elements for row and/or column headers in data tables.".into(),
            });
        }
        findings
    }
}

// ---------------------------------------------------------------------------
// Accessibility: ARIA Roles V2 — roles without names
// ---------------------------------------------------------------------------

pub struct AriaRolesAnalyzerV2;

impl Default for AriaRolesAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl AriaRolesAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for AriaRolesAnalyzerV2 {
    fn name(&self) -> &str {
        "aria-roles-v2"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.aria_role_count > 0 && ctx.page.aria_label_count == 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "ARIA-V2001".to_string(),
                title: "ARIA roles without names".to_string(),
                description: format!("{} ARIA role(s) found but no aria-label or aria-labelledby attributes. Roles need names for screen reader context.", ctx.page.aria_role_count),
                url: url.clone(),
                recommendation: "Add aria-label or aria-labelledby to elements with ARIA roles.".into(),
            });
        }
        findings
    }
}

// ---------------------------------------------------------------------------
// Accessibility: Heading Hierarchy V2 — heading levels skip
// ---------------------------------------------------------------------------

pub struct HeadingHierarchyAnalyzerV2;

impl Default for HeadingHierarchyAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl HeadingHierarchyAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for HeadingHierarchyAnalyzerV2 {
    fn name(&self) -> &str {
        "heading-hierarchy-v2"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.headings.is_empty() {
            return findings;
        }
        let mut prev_level: Option<u8> = None;
        for heading in &ctx.page.headings {
            if let Some(prev) = prev_level {
                if heading.level > prev + 1 {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Accessibility,
                        code: "HEAD-V2001".to_string(),
                        title: "Heading levels skip".to_string(),
                        description: format!("Heading jumps from H{prev} to H{}. Skipping levels breaks the document outline for screen readers.", heading.level),
                        url: url.clone(),
                        recommendation: "Use heading levels in sequential order (H1 -> H2 -> H3).".into(),
                    });
                    break;
                }
            }
            prev_level = Some(heading.level);
        }
        findings
    }
}

// ---------------------------------------------------------------------------
// Accessibility: Language Attribute V2 — missing lang
// ---------------------------------------------------------------------------

pub struct LanguageAttributeAnalyzerV2;

impl Default for LanguageAttributeAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageAttributeAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for LanguageAttributeAnalyzerV2 {
    fn name(&self) -> &str {
        "language-attribute-v2"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if !ctx.page.has_lang_attribute {
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Accessibility,
                code: "LANG-V2001".to_string(),
                title: "Missing lang attribute".to_string(),
                description: "No lang attribute was found on the <html> element. Screen readers need this to use the correct pronunciation rules.".into(),
                url: url.clone(),
                recommendation: "Add lang=\"en\" (or the appropriate language code) to the <html> element.".into(),
            });
        }
        findings
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod new_analyzer_tests {
    use super::*;
    use crate::analyzers::{
        CertificateTransparencyAnalyzer, CookieHttpOnlyFlagValidator, CookieSecureFlagValidator,
        ExpectCTAnalyzer, FeaturePolicyAnalyzer,
    };
    use crate::meta::MetaTags;
    use crate::parser::ParsedPage;

    fn make_page(url: &str) -> ParsedPage {
        ParsedPage {
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
        page: &'a ParsedPage,
        status: Option<u16>,
        headers: &'a [(String, String)],
        body: Option<&'a str>,
    ) -> AnalysisContext<'a> {
        AnalysisContext {
            page,
            body,
            status_code: status,
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

    // DnsRebindingAnalyzer tests

    #[test]
    fn test_dns_rebinding_no_cors() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(DnsRebindingAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_dns_rebinding_wildcard_with_creds() {
        let headers = vec![
            ("Access-Control-Allow-Origin".to_string(), "*".to_string()),
            (
                "Access-Control-Allow-Credentials".to_string(),
                "true".to_string(),
            ),
        ];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        assert!(DnsRebindingAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "DNSREBIND001"));
    }

    #[test]
    fn test_dns_rebinding_wildcard_without_creds() {
        let headers = vec![("Access-Control-Allow-Origin".to_string(), "*".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        assert!(DnsRebindingAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_dns_rebinding_wildcard_with_local_refs() {
        let headers = vec![("Access-Control-Allow-Origin".to_string(), "*".to_string())];
        let page = make_page("https://example.com");
        let body = "Connect to 127.0.0.1 for local access";
        let ctx = make_ctx(&page, Some(200), &headers, Some(body));
        assert!(DnsRebindingAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "DNSREBIND002"));
    }

    #[test]
    fn test_dns_rebinding_name() {
        assert_eq!(DnsRebindingAnalyzer::new().name(), "dns-rebinding");
    }

    #[test]
    fn test_dns_rebinding_default() {
        let _ = DnsRebindingAnalyzer::default();
    }

    #[test]
    fn test_dns_rebinding_specific_origin_no_finding() {
        let headers = vec![(
            "Access-Control-Allow-Origin".to_string(),
            "https://other.com".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        assert!(DnsRebindingAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_dns_rebinding_body_with_localhost() {
        let headers = vec![("Access-Control-Allow-Origin".to_string(), "*".to_string())];
        let page = make_page("https://example.com");
        let body = "Visit localhost:8080 for admin panel";
        let ctx = make_ctx(&page, Some(200), &headers, Some(body));
        assert!(DnsRebindingAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "DNSREBIND002"));
    }

    #[test]
    fn test_dns_rebinding_body_with_192_168() {
        let headers = vec![("Access-Control-Allow-Origin".to_string(), "*".to_string())];
        let page = make_page("https://example.com");
        let body = "Connect to 192.168.1.1 for local access";
        let ctx = make_ctx(&page, Some(200), &headers, Some(body));
        assert!(DnsRebindingAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "DNSREBIND002"));
    }

    #[test]
    fn test_dns_rebinding_body_no_local_refs() {
        let headers = vec![("Access-Control-Allow-Origin".to_string(), "*".to_string())];
        let page = make_page("https://example.com");
        let body = "This is a normal page with no local IPs";
        let ctx = make_ctx(&page, Some(200), &headers, Some(body));
        assert!(DnsRebindingAnalyzer::new().analyze(&ctx).is_empty());
    }

    // SubresourceIntegrityAnalyzer tests

    #[test]
    fn test_sri_no_body() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(SubresourceIntegrityAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_sri_name() {
        assert_eq!(
            SubresourceIntegrityAnalyzer::new().name(),
            "subresource-integrity"
        );
    }

    #[test]
    fn test_sri_default() {
        let _ = SubresourceIntegrityAnalyzer::default();
    }

    #[test]
    fn test_sri_external_script_without_sri() {
        let mut page = make_page("https://example.com");
        page.scripts = vec![crate::parser::ScriptInfo {
            src: Some("https://cdn.example.com/lib.js".to_string()),
            r#async: false,
            defer: false,
            script_type: None,
            has_integrity: false,
        }];
        let body =
            r#"<html><head><script src="https://cdn.example.com/lib.js"></script></head></html>"#;
        let ctx = make_ctx(&page, Some(200), &[], Some(body));
        assert!(SubresourceIntegrityAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "SRISCRIPT001"));
    }

    #[test]
    fn test_sri_external_script_with_sri() {
        let mut page = make_page("https://example.com");
        page.scripts = vec![crate::parser::ScriptInfo {
            src: Some("https://cdn.example.com/lib.js".to_string()),
            r#async: false,
            defer: false,
            script_type: None,
            has_integrity: true,
        }];
        let body = r#"<html><head><script src="https://cdn.example.com/lib.js" integrity="sha384-abc" crossorigin="anonymous"></script></head></html>"#;
        let ctx = make_ctx(&page, Some(200), &[], Some(body));
        assert!(SubresourceIntegrityAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_sri_internal_script_no_finding() {
        let mut page = make_page("https://example.com");
        page.scripts = vec![crate::parser::ScriptInfo {
            src: Some("/app.js".to_string()),
            r#async: false,
            defer: false,
            script_type: None,
            has_integrity: false,
        }];
        let body = r#"<html><head><script src="/app.js"></script></head></html>"#;
        let ctx = make_ctx(&page, Some(200), &[], Some(body));
        assert!(SubresourceIntegrityAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_sri_no_scripts() {
        let page = make_page("https://example.com");
        let body = "<html><head></head></html>";
        let ctx = make_ctx(&page, Some(200), &[], Some(body));
        assert!(SubresourceIntegrityAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_sri_multiple_external_without_sri() {
        let mut page = make_page("https://example.com");
        page.scripts = vec![
            crate::parser::ScriptInfo {
                src: Some("https://cdn.example.com/a.js".to_string()),
                r#async: false,
                defer: false,
                script_type: None,
                has_integrity: false,
            },
            crate::parser::ScriptInfo {
                src: Some("https://cdn.example.com/b.js".to_string()),
                r#async: false,
                defer: false,
                script_type: None,
                has_integrity: false,
            },
        ];
        let body = r#"<html><head><script src="https://cdn.example.com/a.js"></script><script src="https://cdn.example.com/b.js"></script></head></html>"#;
        let ctx = make_ctx(&page, Some(200), &[], Some(body));
        let findings = SubresourceIntegrityAnalyzer::new().analyze(&ctx);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].description.contains("2"));
    }

    #[test]
    fn test_sri_mixed_sri_and_no_sri() {
        let mut page = make_page("https://example.com");
        page.scripts = vec![crate::parser::ScriptInfo {
            src: Some("https://cdn.example.com/bad.js".to_string()),
            r#async: false,
            defer: false,
            script_type: None,
            has_integrity: false,
        }];
        let body =
            r#"<html><head><script src="https://cdn.example.com/bad.js"></script></head></html>"#;
        let ctx = make_ctx(&page, Some(200), &[], Some(body));
        assert!(SubresourceIntegrityAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "SRISCRIPT001"));
    }

    #[test]
    fn test_sri_protocol_relative_url() {
        let mut page = make_page("https://example.com");
        page.scripts = vec![crate::parser::ScriptInfo {
            src: Some("//cdn.example.com/lib.js".to_string()),
            r#async: false,
            defer: false,
            script_type: None,
            has_integrity: false,
        }];
        let body = r#"<html><head><script src="//cdn.example.com/lib.js"></script></head></html>"#;
        let ctx = make_ctx(&page, Some(200), &[], Some(body));
        assert!(SubresourceIntegrityAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "SRISCRIPT001"));
    }

    #[test]
    fn test_sri_no_script_src() {
        let mut page = make_page("https://example.com");
        page.scripts = vec![crate::parser::ScriptInfo {
            src: None,
            r#async: false,
            defer: false,
            script_type: None,
            has_integrity: false,
        }];
        let body = "<html><head><script>console.log('hi')</script></head></html>";
        let ctx = make_ctx(&page, Some(200), &[], Some(body));
        assert!(SubresourceIntegrityAnalyzer::new().analyze(&ctx).is_empty());
    }

    // FeaturePolicyAnalyzer tests

    #[test]
    fn test_feature_policy_no_headers() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(FeaturePolicyAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "FP001"));
    }

    #[test]
    fn test_feature_policy_has_feature_policy() {
        let headers = vec![("Feature-Policy".to_string(), "camera 'none'".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        assert!(FeaturePolicyAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_feature_policy_has_permissions_policy() {
        let headers = vec![("Permissions-Policy".to_string(), "camera=()".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        assert!(FeaturePolicyAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_feature_policy_case_insensitive() {
        let headers = vec![("permissions-policy".to_string(), "camera=()".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        assert!(FeaturePolicyAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_feature_policy_empty_value() {
        let headers = vec![("Permissions-Policy".to_string(), "".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        // Header exists even if empty — analyzer only checks presence
        assert!(FeaturePolicyAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_feature_policy_name() {
        assert_eq!(FeaturePolicyAnalyzer::new().name(), "feature-policy");
    }

    #[test]
    fn test_feature_policy_default() {
        let _ = FeaturePolicyAnalyzer::default();
    }

    #[test]
    fn test_feature_policy_404_still_checks() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(404), &[], None);
        assert!(FeaturePolicyAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "FP001"));
    }

    #[test]
    fn test_feature_policy_both_headers() {
        let headers = vec![
            ("Feature-Policy".to_string(), "camera 'none'".to_string()),
            ("Permissions-Policy".to_string(), "camera=()".to_string()),
        ];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        assert!(FeaturePolicyAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_feature_policy_info_severity() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = FeaturePolicyAnalyzer::new().analyze(&ctx);
        assert_eq!(findings[0].severity, Severity::Info);
    }

    // ExpectCTAnalyzer tests

    #[test]
    fn test_expect_ct_no_header() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(ExpectCTAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "ECT001"));
    }

    #[test]
    fn test_expect_ct_has_header() {
        let headers = vec![(
            "Expect-CT".to_string(),
            "max-age=86400, enforce".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        assert!(ExpectCTAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_expect_ct_non_200_skipped() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(301), &[], None);
        assert!(ExpectCTAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_expect_ct_case_insensitive() {
        let headers = vec![("expect-ct".to_string(), "max-age=86400".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        assert!(ExpectCTAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_expect_ct_name() {
        assert_eq!(ExpectCTAnalyzer::new().name(), "expect-ct");
    }

    #[test]
    fn test_expect_ct_default() {
        let _ = ExpectCTAnalyzer::default();
    }

    #[test]
    fn test_expect_ct_404_no_finding() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(404), &[], None);
        assert!(ExpectCTAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_expect_ct_info_severity() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = ExpectCTAnalyzer::new().analyze(&ctx);
        assert_eq!(findings[0].severity, Severity::Info);
    }

    #[test]
    fn test_expect_ct_200_with_empty_header() {
        let headers = vec![("Expect-CT".to_string(), "".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        assert!(ExpectCTAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_expect_ct_500_no_finding() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(500), &[], None);
        assert!(ExpectCTAnalyzer::new().analyze(&ctx).is_empty());
    }

    // CertificateTransparencyAnalyzer tests

    #[test]
    fn test_ct_no_header() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(CertificateTransparencyAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "CT001"));
    }

    #[test]
    fn test_ct_has_enforce() {
        let headers = vec![(
            "Expect-CT".to_string(),
            "max-age=86400, enforce".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        assert!(CertificateTransparencyAnalyzer::new()
            .analyze(&ctx)
            .is_empty());
    }

    #[test]
    fn test_ct_report_only() {
        let headers = vec![(
            "Expect-CT".to_string(),
            "max-age=86400, enforce".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        assert!(CertificateTransparencyAnalyzer::new()
            .analyze(&ctx)
            .is_empty());
    }

    #[test]
    fn test_ct_non_200_skipped() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(301), &[], None);
        assert!(CertificateTransparencyAnalyzer::new()
            .analyze(&ctx)
            .is_empty());
    }

    #[test]
    fn test_ct_name() {
        assert_eq!(
            CertificateTransparencyAnalyzer::new().name(),
            "certificate-transparency"
        );
    }

    #[test]
    fn test_ct_default() {
        let _ = CertificateTransparencyAnalyzer::default();
    }

    #[test]
    fn test_ct_404_no_finding() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(404), &[], None);
        assert!(CertificateTransparencyAnalyzer::new()
            .analyze(&ctx)
            .is_empty());
    }

    #[test]
    fn test_ct_info_severity() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = CertificateTransparencyAnalyzer::new().analyze(&ctx);
        assert_eq!(findings[0].severity, Severity::Info);
    }

    #[test]
    fn test_ct_header_without_enforce() {
        let headers = vec![("Expect-CT".to_string(), "max-age=0".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        // Has Expect-CT header but without "enforce" — CT analyzer requires enforce
        assert!(CertificateTransparencyAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "CT001"));
    }

    #[test]
    fn test_ct_500_no_finding() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(500), &[], None);
        assert!(CertificateTransparencyAnalyzer::new()
            .analyze(&ctx)
            .is_empty());
    }

    // CorsMisconfigurationAnalyzer tests

    #[test]
    fn test_cors_no_headers() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(CorsMisconfigurationAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_cors_wildcard_with_creds() {
        let headers = vec![
            ("Access-Control-Allow-Origin".to_string(), "*".to_string()),
            (
                "Access-Control-Allow-Credentials".to_string(),
                "true".to_string(),
            ),
        ];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        assert!(CorsMisconfigurationAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "CORS001"));
    }

    #[test]
    fn test_cors_wildcard_on_sensitive() {
        let headers = vec![("Access-Control-Allow-Origin".to_string(), "*".to_string())];
        let page = make_page("https://example.com/api/data");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        assert!(CorsMisconfigurationAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "CORS002"));
    }

    #[test]
    fn test_cors_wildcard_on_non_sensitive() {
        let headers = vec![("Access-Control-Allow-Origin".to_string(), "*".to_string())];
        let page = make_page("https://example.com/page");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        assert!(CorsMisconfigurationAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_cors_specific_origin() {
        let headers = vec![(
            "Access-Control-Allow-Origin".to_string(),
            "https://other.com".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        assert!(CorsMisconfigurationAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_cors_name() {
        assert_eq!(
            CorsMisconfigurationAnalyzer::new().name(),
            "cors-misconfiguration"
        );
    }

    #[test]
    fn test_cors_default() {
        let _ = CorsMisconfigurationAnalyzer::default();
    }

    // AriaLabelAnalyzer tests

    #[test]
    fn test_aria_label_no_roles() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(AriaLabelAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_aria_label_roles_with_labels() {
        let mut page = make_page("https://example.com");
        page.aria_role_count = 3;
        page.aria_label_count = 3;
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(AriaLabelAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_aria_label_roles_without_labels() {
        let mut page = make_page("https://example.com");
        page.aria_role_count = 3;
        page.aria_label_count = 0;
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(AriaLabelAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "ARIALABEL001"));
    }

    #[test]
    fn test_aria_label_name() {
        assert_eq!(AriaLabelAnalyzer::new().name(), "aria-label");
    }

    #[test]
    fn test_aria_label_default() {
        let _ = AriaLabelAnalyzer::default();
    }

    #[test]
    fn test_aria_label_one_role_one_label() {
        let mut page = make_page("https://example.com");
        page.aria_role_count = 1;
        page.aria_label_count = 1;
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(AriaLabelAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_aria_label_multiple_roles_fewer_labels() {
        let mut page = make_page("https://example.com");
        page.aria_role_count = 5;
        page.aria_label_count = 0;
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(AriaLabelAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "ARIALABEL001"));
    }

    #[test]
    fn test_aria_label_more_labels_than_roles() {
        let mut page = make_page("https://example.com");
        page.aria_role_count = 2;
        page.aria_label_count = 5;
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(AriaLabelAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_aria_label_equal_counts() {
        let mut page = make_page("https://example.com");
        page.aria_role_count = 3;
        page.aria_label_count = 3;
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(AriaLabelAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_aria_label_warning_severity() {
        let mut page = make_page("https://example.com");
        page.aria_role_count = 2;
        page.aria_label_count = 0;
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = AriaLabelAnalyzer::new().analyze(&ctx);
        assert_eq!(findings[0].severity, Severity::Warning);
    }

    // TableCaptionAnalyzer tests

    #[test]
    fn test_table_caption_no_tables() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(TableCaptionAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_table_caption_all_have_captions() {
        let mut page = make_page("https://example.com");
        page.tables_total = 3;
        page.tables_with_captions = 3;
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(TableCaptionAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_table_caption_missing() {
        let mut page = make_page("https://example.com");
        page.tables_total = 3;
        page.tables_with_captions = 1;
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(TableCaptionAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "TABLECAP001"));
    }

    #[test]
    fn test_table_caption_name() {
        assert_eq!(TableCaptionAnalyzer::new().name(), "table-caption");
    }

    #[test]
    fn test_table_caption_default() {
        let _ = TableCaptionAnalyzer::default();
    }

    #[test]
    fn test_table_caption_one_table_with_caption() {
        let mut page = make_page("https://example.com");
        page.tables_total = 1;
        page.tables_with_captions = 1;
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(TableCaptionAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_table_caption_one_table_no_caption() {
        let mut page = make_page("https://example.com");
        page.tables_total = 1;
        page.tables_with_captions = 0;
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(TableCaptionAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "TABLECAP001"));
    }

    #[test]
    fn test_table_caption_all_missing() {
        let mut page = make_page("https://example.com");
        page.tables_total = 5;
        page.tables_with_captions = 0;
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(TableCaptionAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "TABLECAP001"));
    }

    #[test]
    fn test_table_caption_half_have_captions() {
        let mut page = make_page("https://example.com");
        page.tables_total = 4;
        page.tables_with_captions = 2;
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(TableCaptionAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "TABLECAP001"));
    }

    #[test]
    fn test_table_caption_warning_severity() {
        let mut page = make_page("https://example.com");
        page.tables_total = 2;
        page.tables_with_captions = 0;
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = TableCaptionAnalyzer::new().analyze(&ctx);
        assert_eq!(findings[0].severity, Severity::Info);
    }

    // SkipLinkAnalyzer tests

    #[test]
    fn test_skip_link_no_nav() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(SkipLinkAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_skip_link_has_nav_with_skip() {
        let mut page = make_page("https://example.com");
        page.has_nav_landmark = true;
        page.has_skip_link = true;
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(SkipLinkAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_skip_link_has_nav_without_skip() {
        let mut page = make_page("https://example.com");
        page.has_nav_landmark = true;
        page.has_skip_link = false;
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(SkipLinkAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "SKIPLINK001"));
    }

    #[test]
    fn test_skip_link_name() {
        assert_eq!(SkipLinkAnalyzer::new().name(), "skip-link");
    }

    #[test]
    fn test_skip_link_default() {
        let _ = SkipLinkAnalyzer::default();
    }

    #[test]
    fn test_skip_link_main_landmark_with_skip() {
        let mut page = make_page("https://example.com");
        page.has_main_landmark = true;
        page.has_skip_link = true;
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(SkipLinkAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_skip_link_main_landmark_without_skip() {
        let mut page = make_page("https://example.com");
        page.has_nav_landmark = true;
        page.has_main_landmark = false;
        page.has_skip_link = false;
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(SkipLinkAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "SKIPLINK001"));
    }

    #[test]
    fn test_skip_link_no_landmarks_no_finding() {
        let mut page = make_page("https://example.com");
        page.has_nav_landmark = false;
        page.has_main_landmark = false;
        page.has_skip_link = false;
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(SkipLinkAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_skip_link_warning_severity() {
        let mut page = make_page("https://example.com");
        page.has_nav_landmark = true;
        page.has_skip_link = false;
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = SkipLinkAnalyzer::new().analyze(&ctx);
        assert_eq!(findings[0].severity, Severity::Warning);
    }

    #[test]
    fn test_skip_link_both_nav_and_main_without_skip() {
        let mut page = make_page("https://example.com");
        page.has_nav_landmark = true;
        page.has_main_landmark = true;
        page.has_skip_link = false;
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(SkipLinkAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "SKIPLINK001"));
    }

    // TabindexAnalyzer tests

    #[test]
    fn test_tabindex_no_positive() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(TabindexAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_tabindex_positive() {
        let mut page = make_page("https://example.com");
        page.has_positive_tabindex = true;
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(TabindexAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "TABINDEX001"));
    }

    #[test]
    fn test_tabindex_negative() {
        let mut page = make_page("https://example.com");
        page.tabindex_negative_count = 5;
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(TabindexAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "TABINDEX002"));
    }

    #[test]
    fn test_tabindex_both() {
        let mut page = make_page("https://example.com");
        page.has_positive_tabindex = true;
        page.tabindex_negative_count = 2;
        let ctx = make_ctx(&page, Some(200), &[], None);
        let findings = TabindexAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "TABINDEX001"));
        assert!(findings.iter().any(|f| f.code == "TABINDEX002"));
    }

    #[test]
    fn test_tabindex_name() {
        assert_eq!(TabindexAnalyzer::new().name(), "tabindex");
    }

    #[test]
    fn test_tabindex_default() {
        let _ = TabindexAnalyzer::default();
    }

    // PermissionsPolicyAnalyzerV2 tests

    #[test]
    fn test_perm_v2_missing() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(PermissionsPolicyAnalyzerV2::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "PERM-V2001"));
    }

    #[test]
    fn test_perm_v2_present() {
        let page = make_page("https://example.com");
        let headers = vec![(
            "Permissions-Policy".to_string(),
            "camera=(), microphone=(), geolocation=(), payment=()".to_string(),
        )];
        let ctx = make_ctx(&page, Some(200), &headers, None);
        assert!(PermissionsPolicyAnalyzerV2::new().analyze(&ctx).is_empty());
    }

    // FormInputLabelAnalyzer tests

    #[test]
    fn test_form_input_label_no_forms() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(FormInputLabelAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_form_input_label_with_label() {
        let mut page = make_page("https://example.com");
        page.forms = vec![crate::parser::ExtractedForm {
            action: None,
            method: "post".to_string(),
            input_count: 1,
            has_file_input: false,
            has_search_input: false,
            inputs: vec![crate::parser::ExtractedInput {
                input_type: Some("text".to_string()),
                name: Some("email".to_string()),
                id: None,
                has_label: true,
                aria_label: None,
                aria_labelledby: None,
                aria_describedby: None,
                placeholder: None,
                required: false,
            }],
            has_fieldset: false,
            has_legend: false,
        }];
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(FormInputLabelAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_form_input_label_missing_label() {
        let mut page = make_page("https://example.com");
        page.forms = vec![crate::parser::ExtractedForm {
            action: None,
            method: "post".to_string(),
            input_count: 1,
            has_file_input: false,
            has_search_input: false,
            inputs: vec![crate::parser::ExtractedInput {
                input_type: Some("text".to_string()),
                name: Some("email".to_string()),
                id: None,
                has_label: false,
                aria_label: None,
                aria_labelledby: None,
                aria_describedby: None,
                placeholder: None,
                required: false,
            }],
            has_fieldset: false,
            has_legend: false,
        }];
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(FormInputLabelAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "FILABEL001"));
    }

    // LinkTextAnalyzer tests

    #[test]
    fn test_link_text_no_links() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(LinkTextAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_link_text_empty() {
        let mut page = make_page("https://example.com");
        page.links = vec![crate::parser::ExtractedLink {
            href: "https://example.com/target".to_string(),
            text: "".to_string(),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(LinkTextAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "LINKTEXT001"));
    }

    #[test]
    fn test_link_text_generic() {
        let mut page = make_page("https://example.com");
        page.links = vec![crate::parser::ExtractedLink {
            href: "https://example.com/target".to_string(),
            text: "click here".to_string(),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(LinkTextAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "LINKTEXT002"));
    }

    #[test]
    fn test_link_text_good() {
        let mut page = make_page("https://example.com");
        page.links = vec![crate::parser::ExtractedLink {
            href: "https://example.com/target".to_string(),
            text: "About our company".to_string(),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(LinkTextAnalyzer::new().analyze(&ctx).is_empty());
    }

    // ImageAltTextAnalyzer tests

    #[test]
    fn test_image_alt_no_images() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(ImageAltTextAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_image_alt_missing() {
        let mut page = make_page("https://example.com");
        page.images = vec![crate::parser::ExtractedImage {
            src: "https://example.com/photo.jpg".to_string(),
            alt: String::new(),
            width: None,
            height: None,
            has_alt: false,
            is_lazy_loaded: false,
            aria_hidden: false,
        }];
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(ImageAltTextAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "IMGALT001"));
    }

    #[test]
    fn test_image_alt_empty() {
        let mut page = make_page("https://example.com");
        page.images = vec![crate::parser::ExtractedImage {
            src: "https://example.com/photo.jpg".to_string(),
            alt: String::new(),
            width: None,
            height: None,
            has_alt: false,
            is_lazy_loaded: false,
            aria_hidden: false,
        }];
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(ImageAltTextAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "IMGALT001"));
    }

    #[test]
    fn test_image_alt_present() {
        let mut page = make_page("https://example.com");
        page.images = vec![crate::parser::ExtractedImage {
            src: "https://example.com/photo.jpg".to_string(),
            alt: "A scenic mountain view".to_string(),
            width: None,
            height: None,
            has_alt: true,
            is_lazy_loaded: false,
            aria_hidden: false,
        }];
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(ImageAltTextAnalyzer::new().analyze(&ctx).is_empty());
    }

    // AriaRoleAnalyzer tests

    #[test]
    fn test_aria_role_no_roles() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(AriaRoleAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_aria_role_without_labels() {
        let mut page = make_page("https://example.com");
        page.aria_role_count = 3;
        page.aria_label_count = 0;
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(AriaRoleAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "ARIAROLE001"));
    }

    #[test]
    fn test_aria_role_with_labels() {
        let mut page = make_page("https://example.com");
        page.aria_role_count = 3;
        page.aria_label_count = 3;
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(AriaRoleAnalyzer::new().analyze(&ctx).is_empty());
    }

    // StrictTransportSecurityAnalyzerV2 tests

    #[test]
    fn test_hsts_v2_missing() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(StrictTransportSecurityAnalyzerV2::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "HSTS-V2001"));
    }

    #[test]
    fn test_hsts_v2_low_max_age() {
        let page = make_page("https://example.com");
        let headers = vec![(
            "Strict-Transport-Security".to_string(),
            "max-age=3600".to_string(),
        )];
        let ctx = make_ctx(&page, Some(200), &headers, None);
        assert!(StrictTransportSecurityAnalyzerV2::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "HSTS-V2002"));
    }

    #[test]
    fn test_hsts_v2_no_include_subdomains() {
        let page = make_page("https://example.com");
        let headers = vec![(
            "Strict-Transport-Security".to_string(),
            "max-age=31536000".to_string(),
        )];
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = StrictTransportSecurityAnalyzerV2::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "HSTS-V2003"));
    }

    #[test]
    fn test_hsts_v2_valid() {
        let page = make_page("https://example.com");
        let headers = vec![(
            "Strict-Transport-Security".to_string(),
            "max-age=31536000; includeSubDomains; preload".to_string(),
        )];
        let ctx = make_ctx(&page, Some(200), &headers, None);
        assert!(StrictTransportSecurityAnalyzerV2::new()
            .analyze(&ctx)
            .is_empty());
    }

    // XssProtectionAnalyzerV2 tests

    #[test]
    fn test_xss_v2_missing() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(XssProtectionAnalyzerV2::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "XSS-V2001"));
    }

    #[test]
    fn test_xss_v2_disabled() {
        let page = make_page("https://example.com");
        let headers = vec![("X-XSS-Protection".to_string(), "0".to_string())];
        let ctx = make_ctx(&page, Some(200), &headers, None);
        assert!(XssProtectionAnalyzerV2::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "XSS-V2002"));
    }

    // ContentTypeSniffingAnalyzerV2 tests

    #[test]
    fn test_ct_v2_missing() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(ContentTypeSniffingAnalyzerV2::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "CT-V2001"));
    }

    #[test]
    fn test_ct_v2_invalid_value() {
        let page = make_page("https://example.com");
        let headers = vec![("X-Content-Type-Options".to_string(), "sniff".to_string())];
        let ctx = make_ctx(&page, Some(200), &headers, None);
        assert!(ContentTypeSniffingAnalyzerV2::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "CT-V2002"));
    }

    #[test]
    fn test_ct_v2_valid() {
        let page = make_page("https://example.com");
        let headers = vec![("X-Content-Type-Options".to_string(), "nosniff".to_string())];
        let ctx = make_ctx(&page, Some(200), &headers, None);
        assert!(ContentTypeSniffingAnalyzerV2::new()
            .analyze(&ctx)
            .is_empty());
    }

    // === HstsPreloadListValidator tests ===

    #[test]
    fn test_hsts_preload_valid() {
        let headers = vec![(
            "Strict-Transport-Security".to_string(),
            "max-age=31536000; includeSubDomains; preload".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        assert!(HstsPreloadListValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_hsts_preload_no_preload() {
        let headers = vec![(
            "Strict-Transport-Security".to_string(),
            "max-age=31536000; includeSubDomains".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        assert!(HstsPreloadListValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_hsts_preload_low_max_age() {
        let headers = vec![(
            "Strict-Transport-Security".to_string(),
            "max-age=300; includeSubDomains; preload".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let f = HstsPreloadListValidator::new().analyze(&ctx);
        assert!(f.iter().any(|f| f.code == "HSTSPRELOAD001"));
    }

    #[test]
    fn test_hsts_preload_no_isd() {
        let headers = vec![(
            "Strict-Transport-Security".to_string(),
            "max-age=63072000; preload".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        assert!(HstsPreloadListValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "HSTSPRELOAD001"));
    }

    #[test]
    fn test_hsts_preload_no_hsts() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(HstsPreloadListValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_hsts_preload_name() {
        assert_eq!(
            HstsPreloadListValidator::new().name(),
            "hsts-preload-list-validator"
        );
    }

    #[test]
    fn test_hsts_preload_default() {
        let _ = HstsPreloadListValidator::default();
    }

    #[test]
    fn test_hsts_preload_category() {
        let headers = vec![(
            "Strict-Transport-Security".to_string(),
            "max-age=300; preload".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let findings = HstsPreloadListValidator::new().analyze(&ctx);
        for f in &findings {
            assert_eq!(f.category, IssueCategory::Security);
        }
    }

    #[test]
    fn test_hsts_preload_severity() {
        let headers = vec![(
            "Strict-Transport-Security".to_string(),
            "max-age=300; preload".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        assert_eq!(
            HstsPreloadListValidator::new().analyze(&ctx)[0].severity,
            Severity::Warning
        );
    }

    // === CspDirectiveValidator tests ===

    #[test]
    fn test_csp_dir_missing_default_src() {
        let headers = vec![(
            "Content-Security-Policy".to_string(),
            "script-src 'self'".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let f = CspDirectiveValidator::new().analyze(&ctx);
        assert!(f.iter().any(|f| f.code == "CSPDIR001"));
    }

    #[test]
    fn test_csp_dir_all_present() {
        let headers = vec![("Content-Security-Policy".to_string(), "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self'; connect-src 'self'".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        assert!(CspDirectiveValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_csp_dir_no_csp() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(CspDirectiveValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_csp_dir_name() {
        assert_eq!(
            CspDirectiveValidator::new().name(),
            "csp-directive-validator"
        );
    }

    #[test]
    fn test_csp_dir_default() {
        let _ = CspDirectiveValidator::default();
    }

    #[test]
    fn test_csp_dir_category() {
        let headers = vec![(
            "Content-Security-Policy".to_string(),
            "script-src 'self'".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        for f in CspDirectiveValidator::new().analyze(&ctx) {
            assert_eq!(f.category, IssueCategory::Security);
        }
    }

    #[test]
    fn test_csp_dir_multiple_missing() {
        let headers = vec![(
            "Content-Security-Policy".to_string(),
            "script-src 'self'".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let f = CspDirectiveValidator::new().analyze(&ctx);
        assert!(f.len() >= 3);
    }

    #[test]
    fn test_csp_dir_severity() {
        let headers = vec![(
            "Content-Security-Policy".to_string(),
            "script-src 'self'".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        assert_eq!(
            CspDirectiveValidator::new().analyze(&ctx)[0].severity,
            Severity::Warning
        );
    }

    #[test]
    fn test_csp_dir_empty_value() {
        let headers = vec![("Content-Security-Policy".to_string(), "".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        let f = CspDirectiveValidator::new().analyze(&ctx);
        assert!(!f.is_empty());
    }

    // === CookieSecureFlagValidator tests ===

    #[test]
    fn test_cookie_secure_missing() {
        let headers = vec![(
            "Set-Cookie".to_string(),
            "session=abc; HttpOnly".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        assert!(CookieSecureFlagValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "COOKIESEC001"));
    }

    #[test]
    fn test_cookie_secure_present() {
        let headers = vec![("Set-Cookie".to_string(), "session=abc; Secure".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        assert!(CookieSecureFlagValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_cookie_secure_http_page_skipped() {
        let headers = vec![("Set-Cookie".to_string(), "session=abc".to_string())];
        let page = make_page("http://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        assert!(CookieSecureFlagValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_cookie_secure_no_cookies() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(CookieSecureFlagValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_cookie_secure_name() {
        assert_eq!(
            CookieSecureFlagValidator::new().name(),
            "cookie-secure-flag"
        );
    }

    #[test]
    fn test_cookie_secure_default() {
        let _ = CookieSecureFlagValidator::default();
    }

    #[test]
    fn test_cookie_secure_category() {
        let headers = vec![("Set-Cookie".to_string(), "session=abc".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        for f in CookieSecureFlagValidator::new().analyze(&ctx) {
            assert_eq!(f.category, IssueCategory::Security);
        }
    }

    #[test]
    fn test_cookie_secure_severity() {
        let headers = vec![("Set-Cookie".to_string(), "session=abc".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        assert_eq!(
            CookieSecureFlagValidator::new().analyze(&ctx)[0].severity,
            Severity::Warning
        );
    }

    // === CookieHttpOnlyFlagValidator tests ===

    #[test]
    fn test_cookie_httponly_missing() {
        let headers = vec![("Set-Cookie".to_string(), "session=abc; Secure".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        assert!(CookieHttpOnlyFlagValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "COOKIEHTTP001"));
    }

    #[test]
    fn test_cookie_httponly_present() {
        let headers = vec![(
            "Set-Cookie".to_string(),
            "session=abc; HttpOnly".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        assert!(CookieHttpOnlyFlagValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_cookie_httponly_both_flags() {
        let headers = vec![(
            "Set-Cookie".to_string(),
            "session=abc; Secure; HttpOnly".to_string(),
        )];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        assert!(CookieHttpOnlyFlagValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_cookie_httponly_no_cookies() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(CookieHttpOnlyFlagValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_cookie_httponly_name() {
        assert_eq!(
            CookieHttpOnlyFlagValidator::new().name(),
            "cookie-httponly-flag"
        );
    }

    #[test]
    fn test_cookie_httponly_default() {
        let _ = CookieHttpOnlyFlagValidator::default();
    }

    #[test]
    fn test_cookie_httponly_category() {
        let headers = vec![("Set-Cookie".to_string(), "session=abc".to_string())];
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &headers, None);
        for f in CookieHttpOnlyFlagValidator::new().analyze(&ctx) {
            assert_eq!(f.category, IssueCategory::Security);
        }
    }

    // === MixedContentFormValidator tests ===

    #[test]
    fn test_mixed_form_http_action() {
        let body = r#"<form action="http://example.com/submit">"#;
        let page = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &page,
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
        assert!(MixedContentFormValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "MIXFRM001"));
    }

    #[test]
    fn test_mixed_form_https_action() {
        let body = r#"<form action="https://example.com/submit">"#;
        let page = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &page,
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
        assert!(MixedContentFormValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_mixed_form_http_page_skipped() {
        let body = r#"<form action="http://example.com/submit">"#;
        let page = make_page("http://example.com");
        let ctx = AnalysisContext {
            page: &page,
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
        assert!(MixedContentFormValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_mixed_form_no_body() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(MixedContentFormValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_mixed_form_name() {
        assert_eq!(
            MixedContentFormValidator::new().name(),
            "mixed-content-form"
        );
    }

    #[test]
    fn test_mixed_form_default() {
        let _ = MixedContentFormValidator::default();
    }

    #[test]
    fn test_mixed_form_category() {
        let body = r#"<form action="http://example.com/submit">"#;
        let page = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &page,
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
        for f in MixedContentFormValidator::new().analyze(&ctx) {
            assert_eq!(f.category, IssueCategory::Security);
        }
    }

    #[test]
    fn test_mixed_form_severity() {
        let body = r#"<form action="http://example.com/submit">"#;
        let page = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &page,
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
        assert_eq!(
            MixedContentFormValidator::new().analyze(&ctx)[0].severity,
            Severity::Error
        );
    }

    // === MixedContentScriptValidator tests ===

    #[test]
    fn test_mixed_script_http() {
        let body = r#"<script src="http://cdn.example.com/app.js"></script>"#;
        let page = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &page,
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
        assert!(MixedContentScriptValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "MIXSCR001"));
    }

    #[test]
    fn test_mixed_script_https() {
        let body = r#"<script src="https://cdn.example.com/app.js"></script>"#;
        let page = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &page,
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
        assert!(MixedContentScriptValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_mixed_script_http_page_skipped() {
        let body = r#"<script src="http://cdn.example.com/app.js"></script>"#;
        let page = make_page("http://example.com");
        let ctx = AnalysisContext {
            page: &page,
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
        assert!(MixedContentScriptValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_mixed_script_no_body() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(MixedContentScriptValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_mixed_script_name() {
        assert_eq!(
            MixedContentScriptValidator::new().name(),
            "mixed-content-script"
        );
    }

    #[test]
    fn test_mixed_script_default() {
        let _ = MixedContentScriptValidator::default();
    }

    #[test]
    fn test_mixed_script_category() {
        let body = r#"<script src="http://cdn.example.com/app.js"></script>"#;
        let page = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &page,
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
        for f in MixedContentScriptValidator::new().analyze(&ctx) {
            assert_eq!(f.category, IssueCategory::Security);
        }
    }

    // === MixedContentImageValidator tests ===

    #[test]
    fn test_mixed_img_http() {
        let body = r#"<img src="http://cdn.example.com/photo.jpg">"#;
        let page = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &page,
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
        assert!(MixedContentImageValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "MIXIMG001"));
    }

    #[test]
    fn test_mixed_img_https() {
        let body = r#"<img src="https://cdn.example.com/photo.jpg">"#;
        let page = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &page,
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
        assert!(MixedContentImageValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_mixed_img_http_page_skipped() {
        let body = r#"<img src="http://cdn.example.com/photo.jpg">"#;
        let page = make_page("http://example.com");
        let ctx = AnalysisContext {
            page: &page,
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
        assert!(MixedContentImageValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_mixed_img_no_body() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(MixedContentImageValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_mixed_img_name() {
        assert_eq!(
            MixedContentImageValidator::new().name(),
            "mixed-content-image"
        );
    }

    #[test]
    fn test_mixed_img_default() {
        let _ = MixedContentImageValidator::default();
    }

    #[test]
    fn test_mixed_img_category() {
        let body = r#"<img src="http://cdn.example.com/photo.jpg">"#;
        let page = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &page,
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
        for f in MixedContentImageValidator::new().analyze(&ctx) {
            assert_eq!(f.category, IssueCategory::Security);
        }
    }

    // === LandmarkMainAnalyzer tests ===

    #[test]
    fn test_landmark_main_missing() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(LandmarkMainAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "LANDMAIN001"));
    }

    #[test]
    fn test_landmark_main_present() {
        let mut page = make_page("https://example.com");
        page.has_main_landmark = true;
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(LandmarkMainAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_landmark_main_name() {
        assert_eq!(LandmarkMainAnalyzer::new().name(), "landmark-main");
    }

    #[test]
    fn test_landmark_main_default() {
        let _ = LandmarkMainAnalyzer::default();
    }

    #[test]
    fn test_landmark_main_category() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        for f in LandmarkMainAnalyzer::new().analyze(&ctx) {
            assert_eq!(f.category, IssueCategory::Accessibility);
        }
    }

    #[test]
    fn test_landmark_main_severity() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert_eq!(
            LandmarkMainAnalyzer::new().analyze(&ctx)[0].severity,
            Severity::Error
        );
    }

    // === LandmarkNavAnalyzer tests ===

    #[test]
    fn test_landmark_nav_missing() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(LandmarkNavAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "LANDNAV001"));
    }

    #[test]
    fn test_landmark_nav_present() {
        let mut page = make_page("https://example.com");
        page.has_nav_landmark = true;
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(LandmarkNavAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_landmark_nav_name() {
        assert_eq!(LandmarkNavAnalyzer::new().name(), "landmark-nav");
    }

    #[test]
    fn test_landmark_nav_default() {
        let _ = LandmarkNavAnalyzer::default();
    }

    #[test]
    fn test_landmark_nav_category() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        for f in LandmarkNavAnalyzer::new().analyze(&ctx) {
            assert_eq!(f.category, IssueCategory::Accessibility);
        }
    }

    // === LandmarkBannerAnalyzer tests ===

    #[test]
    fn test_landmark_banner_missing() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(LandmarkBannerAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "LANDBAN001"));
    }

    #[test]
    fn test_landmark_banner_present() {
        let mut page = make_page("https://example.com");
        page.landmarks.push("banner".to_string());
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(LandmarkBannerAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_landmark_banner_header_role() {
        let mut page = make_page("https://example.com");
        page.landmarks.push("header".to_string());
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(LandmarkBannerAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_landmark_banner_name() {
        assert_eq!(LandmarkBannerAnalyzer::new().name(), "landmark-banner");
    }

    #[test]
    fn test_landmark_banner_default() {
        let _ = LandmarkBannerAnalyzer::default();
    }

    #[test]
    fn test_landmark_banner_category() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        for f in LandmarkBannerAnalyzer::new().analyze(&ctx) {
            assert_eq!(f.category, IssueCategory::Accessibility);
        }
    }

    // === HeadingLevelSkipAnalyzer tests ===

    #[test]
    fn test_heading_skip_h1_to_h3() {
        let mut page = make_page("https://example.com");
        page.headings = vec![
            crate::parser::Heading {
                level: 1,
                text: "H1".into(),
                length: 2,
            },
            crate::parser::Heading {
                level: 3,
                text: "H3".into(),
                length: 2,
            },
        ];
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(HeadingLevelSkipAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "HEADSKIP001"));
    }

    #[test]
    fn test_heading_skip_no_skip() {
        let mut page = make_page("https://example.com");
        page.headings = vec![
            crate::parser::Heading {
                level: 1,
                text: "H1".into(),
                length: 2,
            },
            crate::parser::Heading {
                level: 2,
                text: "H2".into(),
                length: 2,
            },
        ];
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(HeadingLevelSkipAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_heading_skip_single() {
        let mut page = make_page("https://example.com");
        page.headings = vec![crate::parser::Heading {
            level: 1,
            text: "H1".into(),
            length: 2,
        }];
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(HeadingLevelSkipAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_heading_skip_empty() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(HeadingLevelSkipAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_heading_skip_name() {
        assert_eq!(HeadingLevelSkipAnalyzer::new().name(), "heading-level-skip");
    }

    #[test]
    fn test_heading_skip_default() {
        let _ = HeadingLevelSkipAnalyzer::default();
    }

    #[test]
    fn test_heading_skip_category() {
        let mut page = make_page("https://example.com");
        page.headings = vec![
            crate::parser::Heading {
                level: 1,
                text: "H1".into(),
                length: 2,
            },
            crate::parser::Heading {
                level: 3,
                text: "H3".into(),
                length: 2,
            },
        ];
        let ctx = make_ctx(&page, Some(200), &[], None);
        for f in HeadingLevelSkipAnalyzer::new().analyze(&ctx) {
            assert_eq!(f.category, IssueCategory::Accessibility);
        }
    }

    // === FormLabelAssociationAnalyzer tests ===

    #[test]
    fn test_form_label_assoc_missing() {
        let mut page = make_page("https://example.com");
        page.forms = vec![crate::parser::ExtractedForm {
            action: None,
            method: "post".into(),
            input_count: 1,
            has_file_input: false,
            has_search_input: false,
            inputs: vec![crate::parser::ExtractedInput {
                input_type: Some("text".into()),
                name: Some("email".into()),
                id: None,
                has_label: false,
                aria_label: None,
                aria_labelledby: None,
                aria_describedby: None,
                placeholder: None,
                required: false,
            }],
            has_fieldset: false,
            has_legend: false,
        }];
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(FormLabelAssociationAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "FORMLAB001"));
    }

    #[test]
    fn test_form_label_assoc_with_label() {
        let mut page = make_page("https://example.com");
        page.forms = vec![crate::parser::ExtractedForm {
            action: None,
            method: "post".into(),
            input_count: 1,
            has_file_input: false,
            has_search_input: false,
            inputs: vec![crate::parser::ExtractedInput {
                input_type: Some("text".into()),
                name: Some("email".into()),
                id: None,
                has_label: true,
                aria_label: None,
                aria_labelledby: None,
                aria_describedby: None,
                placeholder: None,
                required: false,
            }],
            has_fieldset: false,
            has_legend: false,
        }];
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(FormLabelAssociationAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_form_label_assoc_with_aria() {
        let mut page = make_page("https://example.com");
        page.forms = vec![crate::parser::ExtractedForm {
            action: None,
            method: "post".into(),
            input_count: 1,
            has_file_input: false,
            has_search_input: false,
            inputs: vec![crate::parser::ExtractedInput {
                input_type: Some("text".into()),
                name: Some("email".into()),
                id: None,
                has_label: false,
                aria_label: Some("Email".into()),
                aria_labelledby: None,
                aria_describedby: None,
                placeholder: None,
                required: false,
            }],
            has_fieldset: false,
            has_legend: false,
        }];
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(FormLabelAssociationAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_form_label_assoc_no_forms() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(FormLabelAssociationAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_form_label_assoc_name() {
        assert_eq!(
            FormLabelAssociationAnalyzer::new().name(),
            "form-label-association"
        );
    }

    #[test]
    fn test_form_label_assoc_default() {
        let _ = FormLabelAssociationAnalyzer::default();
    }

    #[test]
    fn test_form_label_assoc_category() {
        let mut page = make_page("https://example.com");
        page.forms = vec![crate::parser::ExtractedForm {
            action: None,
            method: "post".into(),
            input_count: 1,
            has_file_input: false,
            has_search_input: false,
            inputs: vec![crate::parser::ExtractedInput {
                input_type: Some("text".into()),
                name: Some("email".into()),
                id: None,
                has_label: false,
                aria_label: None,
                aria_labelledby: None,
                aria_describedby: None,
                placeholder: None,
                required: false,
            }],
            has_fieldset: false,
            has_legend: false,
        }];
        let ctx = make_ctx(&page, Some(200), &[], None);
        for f in FormLabelAssociationAnalyzer::new().analyze(&ctx) {
            assert_eq!(f.category, IssueCategory::Accessibility);
        }
    }

    // === TableHeaderScopeAnalyzer tests ===

    #[test]
    fn test_tbl_scope_missing() {
        let mut page = make_page("https://example.com");
        page.tables_total = 3;
        page.tables_with_headers = 1;
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(TableHeaderScopeAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "TBLSCOP001"));
    }

    #[test]
    fn test_tbl_scope_all_have() {
        let mut page = make_page("https://example.com");
        page.tables_total = 3;
        page.tables_with_headers = 3;
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(TableHeaderScopeAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_tbl_scope_no_tables() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(TableHeaderScopeAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_tbl_scope_name() {
        assert_eq!(TableHeaderScopeAnalyzer::new().name(), "table-header-scope");
    }

    #[test]
    fn test_tbl_scope_default() {
        let _ = TableHeaderScopeAnalyzer::default();
    }

    #[test]
    fn test_tbl_scope_category() {
        let mut page = make_page("https://example.com");
        page.tables_total = 1;
        page.tables_with_headers = 0;
        let ctx = make_ctx(&page, Some(200), &[], None);
        for f in TableHeaderScopeAnalyzer::new().analyze(&ctx) {
            assert_eq!(f.category, IssueCategory::Accessibility);
        }
    }

    // === TableCaptionPresenceAnalyzer tests ===

    #[test]
    fn test_tbl_cap_missing() {
        let mut page = make_page("https://example.com");
        page.tables_total = 2;
        page.tables_with_captions = 0;
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(TableCaptionPresenceAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "TBLCAP001"));
    }

    #[test]
    fn test_tbl_cap_all_have() {
        let mut page = make_page("https://example.com");
        page.tables_total = 3;
        page.tables_with_captions = 3;
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(TableCaptionPresenceAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_tbl_cap_no_tables() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(TableCaptionPresenceAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_tbl_cap_name() {
        assert_eq!(
            TableCaptionPresenceAnalyzer::new().name(),
            "table-caption-presence"
        );
    }

    #[test]
    fn test_tbl_cap_default() {
        let _ = TableCaptionPresenceAnalyzer::default();
    }

    #[test]
    fn test_tbl_cap_category() {
        let mut page = make_page("https://example.com");
        page.tables_total = 1;
        page.tables_with_captions = 0;
        let ctx = make_ctx(&page, Some(200), &[], None);
        for f in TableCaptionPresenceAnalyzer::new().analyze(&ctx) {
            assert_eq!(f.category, IssueCategory::Accessibility);
        }
    }

    // === AnchorTextGenericAnalyzer tests ===

    #[test]
    fn test_anch_gen_click_here() {
        let mut page = make_page("https://example.com");
        page.links = vec![crate::parser::ExtractedLink {
            href: "/page".into(),
            text: "click here".into(),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(AnchorTextGenericAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "ANCHGEN001"));
    }

    #[test]
    fn test_anch_gen_read_more() {
        let mut page = make_page("https://example.com");
        page.links = vec![crate::parser::ExtractedLink {
            href: "/page".into(),
            text: "read more".into(),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(AnchorTextGenericAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "ANCHGEN001"));
    }

    #[test]
    fn test_anch_gen_good_text() {
        let mut page = make_page("https://example.com");
        page.links = vec![crate::parser::ExtractedLink {
            href: "/page".into(),
            text: "About our pricing".into(),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(AnchorTextGenericAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_anch_gen_empty_text() {
        let mut page = make_page("https://example.com");
        page.links = vec![crate::parser::ExtractedLink {
            href: "/page".into(),
            text: "".into(),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(AnchorTextGenericAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_anch_gen_no_links() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(AnchorTextGenericAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_anch_gen_name() {
        assert_eq!(
            AnchorTextGenericAnalyzer::new().name(),
            "anchor-text-generic"
        );
    }

    #[test]
    fn test_anch_gen_default() {
        let _ = AnchorTextGenericAnalyzer::default();
    }

    #[test]
    fn test_anch_gen_category() {
        let mut page = make_page("https://example.com");
        page.links = vec![crate::parser::ExtractedLink {
            href: "/page".into(),
            text: "click here".into(),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200), &[], None);
        for f in AnchorTextGenericAnalyzer::new().analyze(&ctx) {
            assert_eq!(f.category, IssueCategory::Accessibility);
        }
    }

    // === AriaRequiredAttributesAnalyzer tests ===

    #[test]
    fn test_aria_req_roles_no_labels() {
        let mut page = make_page("https://example.com");
        page.aria_role_count = 3;
        page.aria_label_count = 0;
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(AriaRequiredAttributesAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "ARIAREQ001"));
    }

    #[test]
    fn test_aria_req_with_labels() {
        let mut page = make_page("https://example.com");
        page.aria_role_count = 3;
        page.aria_label_count = 3;
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(AriaRequiredAttributesAnalyzer::new()
            .analyze(&ctx)
            .is_empty());
    }

    #[test]
    fn test_aria_req_no_roles() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(AriaRequiredAttributesAnalyzer::new()
            .analyze(&ctx)
            .is_empty());
    }

    #[test]
    fn test_aria_req_name() {
        assert_eq!(
            AriaRequiredAttributesAnalyzer::new().name(),
            "aria-required-attributes"
        );
    }

    #[test]
    fn test_aria_req_default() {
        let _ = AriaRequiredAttributesAnalyzer::default();
    }

    #[test]
    fn test_aria_req_category() {
        let mut page = make_page("https://example.com");
        page.aria_role_count = 2;
        page.aria_label_count = 0;
        let ctx = make_ctx(&page, Some(200), &[], None);
        for f in AriaRequiredAttributesAnalyzer::new().analyze(&ctx) {
            assert_eq!(f.category, IssueCategory::Accessibility);
        }
    }

    // === FocusOrderPositiveTabindexAnalyzer tests ===

    #[test]
    fn test_tabpos_positive() {
        let mut page = make_page("https://example.com");
        page.has_positive_tabindex = true;
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(FocusOrderPositiveTabindexAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "TABPOS001"));
    }

    #[test]
    fn test_tabpos_no_positive() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(FocusOrderPositiveTabindexAnalyzer::new()
            .analyze(&ctx)
            .is_empty());
    }

    #[test]
    fn test_tabpos_name() {
        assert_eq!(
            FocusOrderPositiveTabindexAnalyzer::new().name(),
            "focus-order-positive-tabindex"
        );
    }

    #[test]
    fn test_tabpos_default() {
        let _ = FocusOrderPositiveTabindexAnalyzer::default();
    }

    #[test]
    fn test_tabpos_category() {
        let mut page = make_page("https://example.com");
        page.has_positive_tabindex = true;
        let ctx = make_ctx(&page, Some(200), &[], None);
        for f in FocusOrderPositiveTabindexAnalyzer::new().analyze(&ctx) {
            assert_eq!(f.category, IssueCategory::Accessibility);
        }
    }

    #[test]
    fn test_tabpos_severity() {
        let mut page = make_page("https://example.com");
        page.has_positive_tabindex = true;
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert_eq!(
            FocusOrderPositiveTabindexAnalyzer::new().analyze(&ctx)[0].severity,
            Severity::Error
        );
    }

    // === ColorContrastTextAnalyzer tests ===

    #[test]
    fn test_colrct_no_body() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(ColorContrastTextAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_colrct_good_contrast() {
        let body = r#"<p style="color: #000000; background-color: #ffffff">Text</p>"#;
        let page = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &page,
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
        assert!(ColorContrastTextAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_colrct_low_contrast() {
        let body = r#"<p style="color: #888888; background-color: #999999">Text</p>"#;
        let page = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &page,
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
        assert!(ColorContrastTextAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "COLRCT001"));
    }

    #[test]
    fn test_colrct_name() {
        assert_eq!(
            ColorContrastTextAnalyzer::new().name(),
            "color-contrast-text"
        );
    }

    #[test]
    fn test_colrct_default() {
        let _ = ColorContrastTextAnalyzer::default();
    }

    #[test]
    fn test_colrct_category() {
        let body = r#"<p style="color: #888888; background-color: #999999">Text</p>"#;
        let page = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &page,
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
        for f in ColorContrastTextAnalyzer::new().analyze(&ctx) {
            assert_eq!(f.category, IssueCategory::Accessibility);
        }
    }

    // === ColorContrastLinkAnalyzer tests ===

    #[test]
    fn test_colrcl_no_body() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200), &[], None);
        assert!(ColorContrastLinkAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_colrcl_good_contrast() {
        let body = r#"<a style="color: #0000ff; background-color: #ffffff">Link</a>"#;
        let page = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &page,
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
        assert!(ColorContrastLinkAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_colrcl_low_contrast() {
        let body = r#"<a style="color: #cccccc; background-color: #dddddd">Link</a>"#;
        let page = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &page,
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
        assert!(ColorContrastLinkAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "COLRCL001"));
    }

    #[test]
    fn test_colrcl_name() {
        assert_eq!(
            ColorContrastLinkAnalyzer::new().name(),
            "color-contrast-link"
        );
    }

    #[test]
    fn test_colrcl_default() {
        let _ = ColorContrastLinkAnalyzer::default();
    }

    #[test]
    fn test_colrcl_category() {
        let body = r#"<a style="color: #cccccc; background-color: #dddddd">Link</a>"#;
        let page = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &page,
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
        for f in ColorContrastLinkAnalyzer::new().analyze(&ctx) {
            assert_eq!(f.category, IssueCategory::Accessibility);
        }
    }
}
