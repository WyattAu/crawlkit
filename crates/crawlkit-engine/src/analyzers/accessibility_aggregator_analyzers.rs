//! AccessibilityAnalyzer — the original aggregate WCAG 2.1 AA checker.
//!
//! One check per page-level signal (images, headings, landmarks, skip link,
//! link text, form labels, keyboard/ARIA, tables, lang). Extracted verbatim
//! from `security_analyzers.rs` (Phase 2 SRP step); the legacy module path
//! re-exports it, so the public name is unchanged.

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
use crate::parser::ExtractedImage;

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
