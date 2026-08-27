#![allow(clippy::unwrap_used, clippy::manual_range_contains, clippy::redundant_closure, clippy::collapsible_if, clippy::unnecessary_map_or, clippy::default_constructed_unit_structs, clippy::needless_return)]
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
        let re = Regex::new(r#"font-size\s*:\s*([^;}]+)"#)
            .unwrap_or_else(|_| Regex::new("x^").unwrap());
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
        let small: Vec<f64> = all_sizes.into_iter().filter(|&s| s > 0.0 && s < 12.0).collect();
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
        let re =
            Regex::new(r"rgb\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*\)").unwrap_or_else(|_| {
                Regex::new("x^").unwrap()
            });
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

    pub(crate) fn contrast_ratio(
        fg: (u8, u8, u8),
        bg: (u8, u8, u8),
    ) -> f64 {
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

pub struct HstsPreloadAnalyzer;

impl HstsPreloadAnalyzer {
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

impl Default for HstsPreloadAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for HstsPreloadAnalyzer {
    fn name(&self) -> &str {
        "hsts-preload"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let hsts_value = match Self::get_header(ctx.headers, "Strict-Transport-Security") {
            Some(v) => v,
            None => return findings,
        };

        let lower = hsts_value.to_lowercase();

        // Check max-age >= 31536000
        if let Some(ma_pos) = lower.find("max-age=") {
            let after = &lower[ma_pos + 8..];
            let num_str: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(age) = num_str.parse::<u64>() {
                if age < 31536000 {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Security,
                        code: "HSTS003".to_string(),
                        title: "HSTS max-age below recommended minimum".to_string(),
                        description: format!(
                            "HSTS max-age is {age}, which is below the recommended minimum of \
                             31536000 (1 year). Short max-age values provide less protection \
                             against protocol downgrade attacks."
                        ),
                        url: url.to_string(),
                        recommendation: "Set max-age to at least 31536000 (1 year). Ideally use \
                                         63072000 (2 years) for preload list eligibility."
                            .into(),
                    });
                }
            }
        }

        // Check includeSubDomains
        if !lower.contains("includesubdomains") {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Security,
                code: "HSTS001".to_string(),
                title: "HSTS missing includeSubDomains directive".to_string(),
                description: "The Strict-Transport-Security header does not include the \
                              includeSubDomains directive. Without it, subdomains are not \
                              protected by HSTS and may be vulnerable to downgrade attacks."
                    .to_string(),
                url: url.to_string(),
                recommendation: "Add includeSubDomains to the HSTS header to extend protection \
                                 to all subdomains."
                    .into(),
            });
        }

        // Check preload
        if !lower.contains("preload") {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Security,
                code: "HSTS002".to_string(),
                title: "HSTS missing preload directive".to_string(),
                description: "The Strict-Transport-Security header does not include the preload \
                              directive. The preload directive signals intent to be included in \
                              browser HSTS preload lists."
                    .to_string(),
                url: url.to_string(),
                recommendation: "Add preload to the HSTS header and submit your domain to the \
                                 HSTS preload list at hstspreload.org."
                    .into(),
            });
        }

        findings
    }
}

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
                    // Only flag cross-origin scripts (different origin from page)
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
                            title: format!(
                                "Permissions-Policy: {feature} not restricted"
                            ),
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
