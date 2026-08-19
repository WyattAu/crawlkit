use std::collections::HashMap;

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
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        // --- WCAG 1.1.1: Image alt text ---
        let images_without_alt: Vec<&str> = ctx
            .page
            .images
            .iter()
            .filter(|img| !img.has_alt || img.alt.trim().is_empty())
            .map(|img| img.src.as_str())
            .collect();
        if !images_without_alt.is_empty() {
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Accessibility,
                code: "A11Y001".to_string(),
                title: "Images missing alt text".to_string(),
                description: format!(
                    "{} image(s) missing alt attribute or have empty alt text: {}.",
                    images_without_alt.len(),
                    images_without_alt.join(", ")
                ),
                url: url.clone(),
                recommendation: "Add descriptive alt text to all images. Use alt=\"\" for \
                                 decorative images."
                    .to_string(),
            });
        }

        // --- WCAG 1.3.1: Heading hierarchy ---
        if ctx.page.headings.is_empty() {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "A11Y002".to_string(),
                title: "No headings found".to_string(),
                description: "The page has no heading elements. Headings provide structure \
                              for screen reader users."
                    .to_string(),
                url: url.clone(),
                recommendation: "Add heading elements (H1-H6) to provide page structure."
                    .to_string(),
            });
        } else {
            let h1_count = ctx.page.headings.iter().filter(|h| h.level == 1).count();
            if h1_count == 0 {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Accessibility,
                    code: "A11Y003".to_string(),
                    title: "Missing H1 heading".to_string(),
                    description: "No H1 heading found. Screen readers use H1 to identify \
                                  the main page topic."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add exactly one H1 heading per page.".to_string(),
                });
            } else if h1_count > 1 {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Accessibility,
                    code: "A11Y004".to_string(),
                    title: "Multiple H1 headings".to_string(),
                    description: format!(
                        "Page has {h1_count} H1 headings. Use a single H1 for the main topic."
                    ),
                    url: url.clone(),
                    recommendation: "Use one H1 for the page title and H2+ for sections."
                        .to_string(),
                });
            }

            // Skipped heading levels
            let mut prev_level: Option<u8> = None;
            for heading in &ctx.page.headings {
                if let Some(prev) = prev_level {
                    if heading.level > prev + 1 {
                        findings.push(Finding {
                            severity: Severity::Warning,
                            category: IssueCategory::Accessibility,
                            code: "A11Y005".to_string(),
                            title: "Skipped heading level".to_string(),
                            description: format!(
                                "Heading jumps from H{prev} to H{}, skipping intermediate \
                                 levels.",
                                heading.level
                            ),
                            url: url.clone(),
                            recommendation: format!(
                                "Use H{} after H{prev} to maintain document outline.",
                                prev + 1
                            ),
                        });
                        break; // Report first skip only
                    }
                }
                prev_level = Some(heading.level);
            }
        }

        // --- WCAG 1.3.1: Landmark roles ---
        if !ctx.page.has_main_landmark {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "A11Y006".to_string(),
                title: "Missing main landmark".to_string(),
                description: "No <main> element or role=\"main\" found. Screen readers use \
                              landmarks for page navigation."
                    .to_string(),
                url: url.clone(),
                recommendation: "Wrap primary content in a <main> element.".to_string(),
            });
        }

        if !ctx.page.has_nav_landmark {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Accessibility,
                code: "A11Y007".to_string(),
                title: "No navigation landmark".to_string(),
                description: "No <nav> element or role=\"navigation\" found.".to_string(),
                url: url.clone(),
                recommendation: "Wrap navigation links in a <nav> element.".to_string(),
            });
        }

        // --- WCAG 2.4.1: Skip navigation ---
        if !ctx.page.has_skip_link && ctx.page.has_nav_landmark {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "A11Y008".to_string(),
                title: "Missing skip navigation link".to_string(),
                description: "No skip-to-content link found. Keyboard users must tab through \
                              all navigation links to reach main content."
                    .to_string(),
                url: url.clone(),
                recommendation: "Add a skip link as the first focusable element: \
                                 <a href=\"#main\" class=\"skip-link\">Skip to content</a>."
                    .to_string(),
            });
        }

        // --- WCAG 2.4.4: Link text quality ---
        for link in &ctx.page.links {
            let text_lower = link.text.trim().to_lowercase();
            // Check for accessible name: text content, aria-label, or img alt
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
                    code: "A11Y009".to_string(),
                    title: "Empty link text".to_string(),
                    description: format!(
                        "Link to \"{}\" has no text. Screen readers announce the URL, \
                         which is not descriptive.",
                        link.href
                    ),
                    url: url.clone(),
                    recommendation: "Add descriptive text or an aria-label to the link."
                        .to_string(),
                });
            } else if Self::VAGUE_LINK_TEXTS.contains(&text_lower.as_str()) {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Accessibility,
                    code: "A11Y010".to_string(),
                    title: "Non-descriptive link text".to_string(),
                    description: format!(
                        "Link text \"{}\" is vague and does not describe the destination.",
                        link.text
                    ),
                    url: url.clone(),
                    recommendation: "Use descriptive text that explains the link purpose \
                                     (e.g., \"View pricing details\" instead of \"click here\")."
                        .to_string(),
                });
            }
        }

        // --- WCAG 1.3.1: Form label association ---
        for form in &ctx.page.forms {
            for input in &form.inputs {
                if !input.has_label {
                    let desc = match (&input.name, &input.input_type) {
                        (Some(n), Some(t)) => format!("input (name=\"{n}\", type=\"{t}\")"),
                        (Some(n), None) => format!("input (name=\"{n}\")"),
                        (None, Some(t)) => format!("input (type=\"{t}\")"),
                        (None, None) => "input".to_string(),
                    };
                    findings.push(Finding {
                        severity: Severity::Error,
                        category: IssueCategory::Accessibility,
                        code: "A11Y011".to_string(),
                        title: "Form input missing label".to_string(),
                        description: format!(
                            "{desc} has no associated <label>, aria-label, or aria-labelledby."
                        ),
                        url: url.clone(),
                        recommendation: "Add a <label for=\"id\"> element or an aria-label \
                                         attribute to the input."
                            .to_string(),
                    });
                }
            }
        }

        // --- WCAG 2.1.1: Keyboard navigation (tabindex) ---
        if ctx.page.has_positive_tabindex {
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Accessibility,
                code: "A11Y012".to_string(),
                title: "Positive tabindex values detected".to_string(),
                description: "Elements with tabindex > 0 alter the natural tab order, \
                              making keyboard navigation unpredictable."
                    .to_string(),
                url: url.clone(),
                recommendation: "Use tabindex=\"0\" to add elements to the natural tab order \
                                 or tabindex=\"-1\" for programmatic focus only."
                    .to_string(),
            });
        }

        // --- WCAG 4.1.2: ARIA usage ---
        if ctx.page.aria_role_count == 0 && !ctx.page.landmarks.is_empty() {
            // No ARIA roles used but landmarks exist via HTML — that's fine
        } else if ctx.page.aria_role_count > 0 && ctx.page.aria_label_count == 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "A11Y013".to_string(),
                title: "ARIA roles without labels".to_string(),
                description: format!(
                    "{} ARIA role(s) found but no aria-label or aria-labelledby attributes. \
                     Custom roles require accessible names.",
                    ctx.page.aria_role_count
                ),
                url: url.clone(),
                recommendation: "Add aria-label or aria-labelledby to elements with custom \
                                 ARIA roles."
                    .to_string(),
            });
        }

        // --- WCAG 1.3.1: Table accessibility ---
        if ctx.page.tables_total > 0 {
            let tables_without_headers = ctx.page.tables_total - ctx.page.tables_with_headers;
            if tables_without_headers > 0 {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Accessibility,
                    code: "A11Y014".to_string(),
                    title: "Table missing header cells".to_string(),
                    description: format!(
                        "{} of {} table(s) have no <th> header cells.",
                        tables_without_headers, ctx.page.tables_total
                    ),
                    url: url.clone(),
                    recommendation: "Use <th> elements for header cells and add scope \
                                     attributes for complex tables."
                        .to_string(),
                });
            }

            let tables_without_captions = ctx.page.tables_total - ctx.page.tables_with_captions;
            if tables_without_captions > 0 {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Accessibility,
                    code: "A11Y015".to_string(),
                    title: "Table missing caption".to_string(),
                    description: format!(
                        "{} of {} table(s) have no <caption> element.",
                        tables_without_captions, ctx.page.tables_total
                    ),
                    url: url.clone(),
                    recommendation: "Add a <caption> to describe the table purpose.".to_string(),
                });
            }
        }

        // --- WCAG 1.4.4: Language attribute ---
        if !ctx.page.has_lang_attribute {
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Accessibility,
                code: "A11Y016".to_string(),
                title: "Missing html lang attribute".to_string(),
                description: "The <html> element has no lang attribute. Screen readers use \
                              this to select the correct pronunciation engine."
                    .to_string(),
                url: url.clone(),
                recommendation: "Add lang=\"en\" (or the appropriate language code) to the \
                                 <html> element."
                    .to_string(),
            });
        }

        findings
    }
}
