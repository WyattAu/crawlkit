//! V2 accessibility analyzers for tabindex, links, images, forms, tables,
//! ARIA roles, heading hierarchy, and language attributes.
//!
//! Extracted from `security_analyzers.rs` as a focused Phase 2 module
//! decomposition. Public names and behavior are preserved through re-exports.

use crate::types::{IssueCategory, Severity};

use super::{AnalysisContext, Analyzer, Finding};

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
                code: "A11Y-LINK-V2001".to_string(),
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
