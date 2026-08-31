//! Deep accessibility analyzers for landmarks, headings, forms, tables, links,
//! images, focus management, and language metadata.
//!
//! Extracted from `security_analyzers.rs` as a Phase 2 SRP step. Public
//! analyzer names and behavior are preserved through re-exports in `mod.rs`.

use crate::types::{IssueCategory, Severity};

use super::{AnalysisContext, Analyzer, Finding};

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
