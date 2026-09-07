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
    clippy::manual_contains,
    clippy::collapsible_match,
    clippy::redundant_clone,
    clippy::useless_conversion
)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct AriaLandmarksAnalyzerV2;
impl Default for AriaLandmarksAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl AriaLandmarksAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for AriaLandmarksAnalyzerV2 {
    fn name(&self) -> &str {
        "aria-landmarks-v2"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.landmarks.is_empty() {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "ARIALAND-V2006".to_string(),
                title: "No ARIA landmarks found".to_string(),
                description: "No ARIA landmarks on page.".to_string(),
                url: url.clone(),
                recommendation: "Add landmark roles (banner, navigation, main, contentinfo)."
                    .to_string(),
            });
            return findings;
        }
        for &role in &["banner", "navigation", "main", "contentinfo"] {
            if !ctx.page.landmarks.iter().any(|l| l.to_lowercase() == role) {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Accessibility,
                    code: format!(
                        "ARIALAND-V200{}",
                        ["banner", "navigation", "main", "contentinfo"]
                            .iter()
                            .position(|&r| r == role)
                            .unwrap_or(0)
                            + 1
                    ),
                    title: format!("Missing {role} landmark"),
                    description: format!("No ARIA landmark with role '{role}' found."),
                    url: url.clone(),
                    recommendation: format!("Add a <div role=\"{role}\"> or HTML5 element."),
                });
            }
        }
        let main_count = ctx
            .page
            .landmarks
            .iter()
            .filter(|l| l.to_lowercase() == "main")
            .count();
        if main_count > 1 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "ARIALAND-V2005".to_string(),
                title: "Duplicate main landmarks".to_string(),
                description: format!("{main_count} main landmarks found. Only one is allowed."),
                url: url.clone(),
                recommendation: "Use a single main landmark.".to_string(),
            });
        }
        findings
    }
}

pub struct HeadingHierarchyDeepAnalyzerV2;
impl Default for HeadingHierarchyDeepAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl HeadingHierarchyDeepAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for HeadingHierarchyDeepAnalyzerV2 {
    fn name(&self) -> &str {
        "heading-hierarchy-deep-v2"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.headings.is_empty() {
            return findings;
        }
        let mut prev_level = 0u8;
        let mut skip_count = 0;
        let mut empty_count = 0;
        for h in &ctx.page.headings {
            if h.text.trim().is_empty() {
                empty_count += 1;
            }
            if prev_level > 0 && h.level > prev_level + 1 {
                skip_count += 1;
            }
            prev_level = h.level;
        }
        if empty_count > 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "HHIER-V2002".to_string(),
                title: "Empty headings found".to_string(),
                description: format!("{empty_count} heading(s) have no text."),
                url: url.clone(),
                recommendation: "Add meaningful text to all headings.".to_string(),
            });
        }
        if skip_count > 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "HHIER-V2003".to_string(),
                title: "Heading levels skipped".to_string(),
                description: format!("{skip_count} heading level skip(s)."),
                url: url.clone(),
                recommendation: "Use heading levels sequentially.".to_string(),
            });
        }
        findings
    }
}

pub struct FormLabelsDeepAnalyzerV2;
impl Default for FormLabelsDeepAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl FormLabelsDeepAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for FormLabelsDeepAnalyzerV2 {
    fn name(&self) -> &str {
        "form-labels-deep-v2"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.forms.is_empty() {
            return findings;
        }
        let mut unlabeled = 0;
        for form in &ctx.page.forms {
            for input in &form.inputs {
                let t = input.input_type.as_deref().unwrap_or("text");
                if matches!(t, "hidden" | "submit" | "button" | "image" | "reset") {
                    continue;
                }
                if !input.has_label
                    && input.aria_label.is_none()
                    && input.aria_labelledby.is_none()
                    && input.placeholder.is_none()
                {
                    unlabeled += 1;
                }
            }
        }
        if unlabeled > 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "FORMLBL-V2001".to_string(),
                title: "Form elements without labels".to_string(),
                description: format!("{unlabeled} input(s) lack labels."),
                url: url.clone(),
                recommendation: "Associate inputs with <label> elements.".to_string(),
            });
        }
        findings
    }
}

pub struct TableAccessibilityDeepAnalyzerV2;
impl Default for TableAccessibilityDeepAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl TableAccessibilityDeepAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for TableAccessibilityDeepAnalyzerV2 {
    fn name(&self) -> &str {
        "table-accessibility-deep-v2"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.tables_total == 0 {
            return findings;
        }
        if ctx.page.tables_with_headers == 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "TABACC-V2001".to_string(),
                title: "Tables without headers".to_string(),
                description: format!("{} table(s) lack <th> headers.", ctx.page.tables_total),
                url: url.clone(),
                recommendation: "Add <th> elements.".to_string(),
            });
        }
        if ctx.page.tables_with_captions == 0 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Accessibility,
                code: "TABACC-V2002".to_string(),
                title: "Tables without captions".to_string(),
                description: format!("{} table(s) lack <caption>.", ctx.page.tables_total),
                url: url.clone(),
                recommendation: "Add <caption> to each table.".to_string(),
            });
        }
        findings
    }
}

pub struct LinkTextQualityAnalyzerV2;
impl Default for LinkTextQualityAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl LinkTextQualityAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for LinkTextQualityAnalyzerV2 {
    fn name(&self) -> &str {
        "link-text-quality-v2"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let generic = [
            "click here",
            "here",
            "read more",
            "more",
            "link",
            "learn more",
        ];
        let mut generic_count = 0;
        let mut empty_count = 0;
        for link in &ctx.page.links {
            let text = link.text.trim().to_lowercase();
            // aria-label (or an image alt) provides the accessible name, so
            // a link with either is not "empty" even without visible text.
            if text.is_empty() && link.aria_label.is_none() && link.img_alt.is_none() {
                empty_count += 1;
            }
            // aria-label overrides visible text for assistive technology,
            // so a link with an aria-label is not flagged as generic.
            if link.aria_label.is_none() && generic.contains(&text.as_str()) {
                generic_count += 1;
            }
        }
        if generic_count > 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "LINKTQ-V2001".to_string(),
                title: "Generic link text".to_string(),
                description: format!("{generic_count} link(s) use generic text."),
                url: url.clone(),
                recommendation: "Use descriptive link text.".to_string(),
            });
        }
        if empty_count > 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "LINKTQ-V2002-V2".to_string(),
                title: "Empty link text".to_string(),
                description: format!("{empty_count} link(s) have no text or aria-label."),
                url: url.clone(),
                recommendation: "Add descriptive text or aria-label.".to_string(),
            });
        }
        findings
    }
}

pub struct ImageAltTextDeepAnalyzerV2;
impl Default for ImageAltTextDeepAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl ImageAltTextDeepAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for ImageAltTextDeepAnalyzerV2 {
    fn name(&self) -> &str {
        "image-alt-text-deep-v2"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.images.is_empty() {
            return findings;
        }
        let mut missing = 0;
        let mut generic = 0;
        for img in &ctx.page.images {
            // aria-hidden="true" removes the image from the accessibility
            // tree entirely, so alt text is irrelevant for such images.
            if img.aria_hidden {
                continue;
            }
            if !img.has_alt && img.alt.is_empty() {
                missing += 1;
            } else if ["image", "photo", "picture", "img"]
                .contains(&img.alt.to_lowercase().as_str())
            {
                generic += 1;
            }
        }
        if missing > 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "IMGALT-V2001".to_string(),
                title: "Images missing alt".to_string(),
                description: format!("{missing} image(s) have no alt attribute."),
                url: url.clone(),
                recommendation: "Add alt attributes.".to_string(),
            });
        }
        if generic > 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "IMGALT-V2003".to_string(),
                title: "Generic alt text".to_string(),
                description: format!("{generic} image(s) use generic alt text."),
                url: url.clone(),
                recommendation: "Use descriptive alt text.".to_string(),
            });
        }
        findings
    }
}

pub struct FocusManagementDeepAnalyzerV2;
impl Default for FocusManagementDeepAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl FocusManagementDeepAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for FocusManagementDeepAnalyzerV2 {
    fn name(&self) -> &str {
        "focus-management-deep-v2"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.has_positive_tabindex {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "FOCUS-V2001".to_string(),
                title: "Positive tabindex found".to_string(),
                description: "Positive tabindex reorders tab sequence confusingly.".to_string(),
                url: url.clone(),
                recommendation: "Use tabindex=\"0\" or tabindex=\"-1\".".to_string(),
            });
        }
        findings
    }
}

pub struct LanguageAttributesDeepAnalyzerV2;
impl Default for LanguageAttributesDeepAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl LanguageAttributesDeepAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for LanguageAttributesDeepAnalyzerV2 {
    fn name(&self) -> &str {
        "language-attributes-deep-v2"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if !ctx.page.has_lang_attribute {
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Accessibility,
                code: "LANGATTR-V2001".to_string(),
                title: "Missing html lang".to_string(),
                description: "Screen readers cannot determine page language.".to_string(),
                url: url.clone(),
                recommendation: "Add lang=\"en\" to <html>.".to_string(),
            });
        }
        findings
    }
}

pub struct ColorContrastTextAnalyzerV2;
impl Default for ColorContrastTextAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl ColorContrastTextAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for ColorContrastTextAnalyzerV2 {
    fn name(&self) -> &str {
        "color-contrast-text-v2"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(body) = ctx.body {
            let lower = body.to_lowercase();
            let hidden =
                lower.matches("opacity:0").count() + lower.matches("visibility:hidden").count();
            if hidden > 0 {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Accessibility,
                    code: "COLRCT-V2003".to_string(),
                    title: "Hidden text detected".to_string(),
                    description: format!("{hidden} CSS rule(s) hide text."),
                    url: url.clone(),
                    recommendation: "Avoid hiding text with CSS.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct ColorContrastLinkAnalyzerV2;
impl Default for ColorContrastLinkAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl ColorContrastLinkAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for ColorContrastLinkAnalyzerV2 {
    fn name(&self) -> &str {
        "color-contrast-link-v2"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(body) = ctx.body {
            let lower = body.to_lowercase();
            if lower.contains("text-decoration: none") && lower.contains("color:") {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Accessibility,
                    code: "COLRCL-V2001-UNDERLINE".to_string(),
                    title: "Links without underline".to_string(),
                    description: "Links may be indistinguishable from text.".to_string(),
                    url: url.clone(),
                    recommendation: "Provide non-color visual indicator for links.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct AnchorTextGenericAnalyzerV2;
impl Default for AnchorTextGenericAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl AnchorTextGenericAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for AnchorTextGenericAnalyzerV2 {
    fn name(&self) -> &str {
        "anchor-text-generic-v2"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let generic = [
            "click here",
            "here",
            "read more",
            "more info",
            "learn more",
            "link",
        ];
        let mut generic_count = 0;
        for link in &ctx.page.links {
            let text = link.text.trim().to_lowercase();
            if generic.contains(&text.as_str()) {
                generic_count += 1;
            }
        }
        if generic_count > 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "ANCHGEN-V2001".to_string(),
                title: "Generic anchor text".to_string(),
                description: format!("{generic_count} link(s) use generic text."),
                url: url.clone(),
                recommendation: "Use descriptive, keyword-rich anchor text.".to_string(),
            });
        }
        findings
    }
}

pub struct TableCaptionPresenceAnalyzerV2;
impl Default for TableCaptionPresenceAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl TableCaptionPresenceAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for TableCaptionPresenceAnalyzerV2 {
    fn name(&self) -> &str {
        "table-caption-presence-v2"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.tables_total > 0 && ctx.page.tables_with_captions == 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "TBLCAP-V2001".to_string(),
                title: "No table captions".to_string(),
                description: format!("{} table(s) lack <caption>.", ctx.page.tables_total),
                url: url.clone(),
                recommendation: "Add <caption> to every data table.".to_string(),
            });
        }
        findings
    }
}

pub struct TableHeaderScopeAnalyzerV2;
impl Default for TableHeaderScopeAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl TableHeaderScopeAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for TableHeaderScopeAnalyzerV2 {
    fn name(&self) -> &str {
        "table-header-scope-v2"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.tables_total == 0 || ctx.page.tables_with_headers == 0 {
            return findings;
        }
        if let Some(body) = ctx.body {
            let lower = body.to_lowercase();
            let th_count = lower.matches("<th").count();
            let th_with_scope = lower.matches("scope=\"").count();
            if th_count > 0 && th_with_scope == 0 {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Accessibility,
                    code: "TBLSCOP-V2001".to_string(),
                    title: "Table headers missing scope".to_string(),
                    description: format!("{th_count} <th> elements without scope."),
                    url: url.clone(),
                    recommendation: "Add scope=\"col\" or scope=\"row\" to <th> elements."
                        .to_string(),
                });
            }
        }
        findings
    }
}

pub struct FormLabelAssociationAnalyzerV2;
impl Default for FormLabelAssociationAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl FormLabelAssociationAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for FormLabelAssociationAnalyzerV2 {
    fn name(&self) -> &str {
        "form-label-association-v2"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.forms.is_empty() {
            return findings;
        }
        let mut dup_ids = 0;
        for form in &ctx.page.forms {
            let ids: std::collections::HashSet<&str> =
                form.inputs.iter().filter_map(|i| i.id.as_deref()).collect();
            let mut counts: std::collections::HashMap<&str, usize> =
                std::collections::HashMap::new();
            for id in &ids {
                *counts.entry(id).or_insert(0) += 1;
            }
            for &c in counts.values() {
                if c > 1 {
                    dup_ids += 1;
                }
            }
        }
        if dup_ids > 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "FORMLAB-V2001".to_string(),
                title: "Duplicate input IDs".to_string(),
                description: format!("{dup_ids} duplicate ID(s) found."),
                url: url.clone(),
                recommendation: "Ensure all input IDs are unique.".to_string(),
            });
        }
        findings
    }
}

pub struct AriaRequiredAttributesAnalyzerV2;
impl Default for AriaRequiredAttributesAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}
impl AriaRequiredAttributesAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for AriaRequiredAttributesAnalyzerV2 {
    fn name(&self) -> &str {
        "aria-required-attributes-v2"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(body) = ctx.body {
            let lower = body.to_lowercase();
            for role in &["progressbar", "slider", "combobox", "listbox", "tab"] {
                let role_pattern = format!("role=\"{role}\"");
                if let Some(pos) = lower.find(&role_pattern) {
                    let after = &lower[pos + role_pattern.len()..];
                    let segment = &after[..after.len().min(300)];
                    match *role {
                        "progressbar" => {
                            if !segment.contains("aria-valuenow") {
                                findings.push(Finding {
                                    severity: Severity::Warning,
                                    category: IssueCategory::Accessibility,
                                    code: "ARIAREQ-V2002".to_string(),
                                    title: "progressbar missing aria-valuenow".to_string(),
                                    description: "Required for assistive technologies.".to_string(),
                                    url: url.clone(),
                                    recommendation: "Add aria-valuenow.".to_string(),
                                });
                            }
                        }
                        "slider" => {
                            if !segment.contains("aria-valuenow")
                                || !segment.contains("aria-valuemin")
                            {
                                findings.push(Finding {
                                    severity: Severity::Warning,
                                    category: IssueCategory::Accessibility,
                                    code: "ARIAREQ-V2003".to_string(),
                                    title: "slider missing required attributes".to_string(),
                                    description: "Requires aria-valuenow and aria-valuemin."
                                        .to_string(),
                                    url: url.clone(),
                                    recommendation:
                                        "Add aria-valuenow, aria-valuemin, aria-valuemax."
                                            .to_string(),
                                });
                            }
                        }
                        "combobox" | "listbox" => {
                            if !segment.contains("aria-expanded") {
                                findings.push(Finding {
                                    severity: Severity::Warning,
                                    category: IssueCategory::Accessibility,
                                    code: "ARIAREQ-V2004".to_string(),
                                    title: format!("{role} missing aria-expanded"),
                                    description: "Should indicate expanded/collapsed state."
                                        .to_string(),
                                    url: url.clone(),
                                    recommendation: "Add aria-expanded.".to_string(),
                                });
                            }
                        }
                        "tab" => {
                            if !segment.contains("aria-selected") {
                                findings.push(Finding {
                                    severity: Severity::Info,
                                    category: IssueCategory::Accessibility,
                                    code: "ARIAREQ-V2005".to_string(),
                                    title: "tab missing aria-selected".to_string(),
                                    description: "Should indicate active tab state.".to_string(),
                                    url: url.clone(),
                                    recommendation: "Add aria-selected.".to_string(),
                                });
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        findings
    }
}

// =========================================================================
// SEO V2/V3/V4 Analyzers
// =========================================================================

pub struct LandmarkMainValidatorV5;
impl Default for LandmarkMainValidatorV5 {
    fn default() -> Self {
        Self::new()
    }
}
impl LandmarkMainValidatorV5 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for LandmarkMainValidatorV5 {
    fn name(&self) -> &str {
        "landmark-main-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if !ctx
            .page
            .landmarks
            .iter()
            .any(|l| l.to_lowercase() == "main")
            && !ctx.page.has_main_landmark
        {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "LANDMAIN-V5001".to_string(),
                title: "Missing main landmark".to_string(),
                description: "No <main> or role='main' found.".to_string(),
                url: url.clone(),
                recommendation: "Add a main landmark for primary content.".to_string(),
            });
        }
        let main_count = ctx
            .page
            .landmarks
            .iter()
            .filter(|l| l.to_lowercase() == "main")
            .count();
        if main_count > 1 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "LANDMAIN-V5002".to_string(),
                title: "Multiple main landmarks".to_string(),
                description: format!("{main_count} main landmarks. Only one allowed."),
                url: url.clone(),
                recommendation: "Use a single main landmark.".to_string(),
            });
        }
        findings
    }
}

pub struct LandmarkNavValidatorV5;
impl Default for LandmarkNavValidatorV5 {
    fn default() -> Self {
        Self::new()
    }
}
impl LandmarkNavValidatorV5 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for LandmarkNavValidatorV5 {
    fn name(&self) -> &str {
        "landmark-nav-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if !ctx
            .page
            .landmarks
            .iter()
            .any(|l| l.to_lowercase() == "navigation")
            && !ctx.page.has_nav_landmark
        {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Accessibility,
                code: "LANDNAV-V5001".to_string(),
                title: "Missing navigation landmark".to_string(),
                description: "No <nav> or role='navigation' found.".to_string(),
                url: url.clone(),
                recommendation: "Add a navigation landmark.".to_string(),
            });
        }
        findings
    }
}

pub struct LandmarkBannerValidatorV5;
impl Default for LandmarkBannerValidatorV5 {
    fn default() -> Self {
        Self::new()
    }
}
impl LandmarkBannerValidatorV5 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for LandmarkBannerValidatorV5 {
    fn name(&self) -> &str {
        "landmark-banner-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if !ctx
            .page
            .landmarks
            .iter()
            .any(|l| l.to_lowercase() == "banner")
        {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Accessibility,
                code: "LANDBANNER-V5001".to_string(),
                title: "Missing banner landmark".to_string(),
                description: "No <header> or role='banner' found.".to_string(),
                url: url.clone(),
                recommendation: "Add a banner landmark for site header.".to_string(),
            });
        }
        findings
    }
}

pub struct HeadingSkipLevelsValidator;
impl Default for HeadingSkipLevelsValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl HeadingSkipLevelsValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for HeadingSkipLevelsValidator {
    fn name(&self) -> &str {
        "heading-skip-levels-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.headings.is_empty() {
            return findings;
        }
        let mut prev_level = 0u8;
        let mut skip_count = 0;
        for h in &ctx.page.headings {
            if prev_level > 0 && h.level > prev_level + 1 {
                skip_count += 1;
            }
            prev_level = h.level;
        }
        if skip_count > 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "HEADSKIP-V5001".to_string(),
                title: "Heading levels skipped".to_string(),
                description: format!("{skip_count} heading level skip(s)."),
                url: url.clone(),
                recommendation: "Use heading levels sequentially (H1 > H2 > H3).".to_string(),
            });
        }
        findings
    }
}

pub struct HeadingMultipleH1Validator;
impl Default for HeadingMultipleH1Validator {
    fn default() -> Self {
        Self::new()
    }
}
impl HeadingMultipleH1Validator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for HeadingMultipleH1Validator {
    fn name(&self) -> &str {
        "heading-multiple-h1-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let h1_count = ctx.page.headings.iter().filter(|h| h.level == 1).count();
        if h1_count == 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "HEADH1-V5001".to_string(),
                title: "Missing H1 heading".to_string(),
                description: "No H1 heading found.".to_string(),
                url: url.clone(),
                recommendation: "Add a single H1 heading.".to_string(),
            });
        } else if h1_count > 1 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "HEADH1-V5002".to_string(),
                title: "Multiple H1 headings".to_string(),
                description: format!("{h1_count} H1 headings found."),
                url: url.clone(),
                recommendation: "Use only one H1 per page.".to_string(),
            });
        }
        findings
    }
}

pub struct FormLabelAssociationValidatorV5;
impl Default for FormLabelAssociationValidatorV5 {
    fn default() -> Self {
        Self::new()
    }
}
impl FormLabelAssociationValidatorV5 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for FormLabelAssociationValidatorV5 {
    fn name(&self) -> &str {
        "form-label-association-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.forms.is_empty() {
            return findings;
        }
        let mut unlabeled = 0;
        for form in &ctx.page.forms {
            for input in &form.inputs {
                let t = input.input_type.as_deref().unwrap_or("text");
                if matches!(t, "hidden" | "submit" | "button" | "image" | "reset") {
                    continue;
                }
                if !input.has_label && input.aria_label.is_none() && input.aria_labelledby.is_none()
                {
                    unlabeled += 1;
                }
            }
        }
        if unlabeled > 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "FORMLBLASSOC-V5001".to_string(),
                title: "Form inputs without labels".to_string(),
                description: format!("{unlabeled} input(s) lack associated labels."),
                url: url.clone(),
                recommendation: "Associate inputs with <label> elements.".to_string(),
            });
        }
        findings
    }
}

pub struct FormRequiredFieldsValidator;
impl Default for FormRequiredFieldsValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl FormRequiredFieldsValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for FormRequiredFieldsValidator {
    fn name(&self) -> &str {
        "form-required-fields-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.forms.is_empty() {
            return findings;
        }
        let mut missing_required = 0;
        for form in &ctx.page.forms {
            for input in &form.inputs {
                if input.required && input.aria_label.is_none() && !input.has_label {
                    missing_required += 1;
                }
            }
        }
        if missing_required > 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "FORMREQ-V5001".to_string(),
                title: "Required fields missing labels".to_string(),
                description: format!("{missing_required} required input(s) lack labels."),
                url: url.clone(),
                recommendation: "Add labels to required form fields.".to_string(),
            });
        }
        findings
    }
}

pub struct TableHeadersValidatorV5;
impl Default for TableHeadersValidatorV5 {
    fn default() -> Self {
        Self::new()
    }
}
impl TableHeadersValidatorV5 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for TableHeadersValidatorV5 {
    fn name(&self) -> &str {
        "table-headers-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.tables_total == 0 {
            return findings;
        }
        if ctx.page.tables_with_headers == 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "TBLHDR-V5001".to_string(),
                title: "All tables missing headers".to_string(),
                description: format!("{} table(s) lack <th> headers.", ctx.page.tables_total),
                url: url.clone(),
                recommendation: "Add <th> elements to data tables.".to_string(),
            });
        } else if ctx.page.tables_with_headers < ctx.page.tables_total {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Accessibility,
                code: "TBLHDR-V5002".to_string(),
                title: "Some tables missing headers".to_string(),
                description: format!(
                    "{}/{} tables have headers.",
                    ctx.page.tables_with_headers, ctx.page.tables_total
                ),
                url: url.clone(),
                recommendation: "Add headers to all data tables.".to_string(),
            });
        }
        findings
    }
}

pub struct TableCaptionValidatorV5;
impl Default for TableCaptionValidatorV5 {
    fn default() -> Self {
        Self::new()
    }
}
impl TableCaptionValidatorV5 {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for TableCaptionValidatorV5 {
    fn name(&self) -> &str {
        "table-caption-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.tables_total == 0 {
            return findings;
        }
        if ctx.page.tables_with_captions == 0 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Accessibility,
                code: "TBLCAPT-V5001".to_string(),
                title: "Tables missing captions".to_string(),
                description: format!("{} table(s) lack <caption>.", ctx.page.tables_total),
                url: url.clone(),
                recommendation: "Add <caption> to data tables.".to_string(),
            });
        }
        findings
    }
}

pub struct TableScopeValidator;
impl Default for TableScopeValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl TableScopeValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for TableScopeValidator {
    fn name(&self) -> &str {
        "table-scope-v5"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.tables_total == 0 {
            return findings;
        }
        if let Some(body) = ctx.body {
            let lower = body.to_lowercase();
            let th_count = lower.matches("<th").count();
            let scope_count = lower.matches("scope=\"").count();
            if th_count > 0 && scope_count == 0 {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Accessibility,
                    code: "TBLSCOPE-V5001".to_string(),
                    title: "Table headers missing scope".to_string(),
                    description: format!("{th_count} <th> without scope attribute."),
                    url: url.clone(),
                    recommendation: "Add scope='col' or scope='row'.".to_string(),
                });
            }
        }
        findings
    }
}

// =========================================================================
// Performance V5 Analyzers (76-80)
// =========================================================================

pub struct HeadingH1CountValidator;
impl Default for HeadingH1CountValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl HeadingH1CountValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for HeadingH1CountValidator {
    fn name(&self) -> &str {
        "heading-h1-count-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let h1_count = ctx.page.headings.iter().filter(|h| h.level == 1).count();
        if h1_count == 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "H1COUNT-V6107".to_string(),
                title: "Missing H1 heading".to_string(),
                description: "No H1 heading found.".to_string(),
                url: url.clone(),
                recommendation: "Add a single H1 heading.".to_string(),
            });
        } else if h1_count > 1 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "H1MULTI-V6108".to_string(),
                title: "Multiple H1 headings".to_string(),
                description: format!("{h1_count} H1 headings found."),
                url: url.clone(),
                recommendation: "Use a single H1 heading per page.".to_string(),
            });
        }
        findings
    }
}

pub struct HeadingDepthAnalysisValidator;
impl Default for HeadingDepthAnalysisValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl HeadingDepthAnalysisValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for HeadingDepthAnalysisValidator {
    fn name(&self) -> &str {
        "heading-depth-analysis-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(max_level) = ctx.page.headings.iter().map(|h| h.level).max() {
            if max_level >= 6 {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Accessibility,
                    code: "HEADDEPTH-V6109".to_string(),
                    title: "Headings reach H6 level".to_string(),
                    description: "H6 headings may indicate overly deep content structure."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Consider flattening heading hierarchy.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct FormRequiredFieldsDeepValidator;
impl Default for FormRequiredFieldsDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl FormRequiredFieldsDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for FormRequiredFieldsDeepValidator {
    fn name(&self) -> &str {
        "form-required-fields-deep-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for form in &ctx.page.forms {
            let required_inputs: Vec<_> = form.inputs.iter().filter(|i| i.required).collect();
            let labeled: Vec<_> = required_inputs
                .iter()
                .filter(|i| i.has_label || i.aria_label.is_some() || i.aria_labelledby.is_some())
                .collect();
            if !required_inputs.is_empty() && labeled.len() < required_inputs.len() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Accessibility,
                    code: "FORMREQ-V6110".to_string(),
                    title: "Required fields missing labels".to_string(),
                    description: format!(
                        "{}/{} required inputs lack labels.",
                        required_inputs.len() - labeled.len(),
                        required_inputs.len()
                    ),
                    url: url.clone(),
                    recommendation: "Add labels to all required form fields.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct TableHeadersScopeValidator;
impl Default for TableHeadersScopeValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl TableHeadersScopeValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for TableHeadersScopeValidator {
    fn name(&self) -> &str {
        "table-headers-scope-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.tables_total > 0 && ctx.page.tables_with_headers == 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "TABSCOPE-V6111".to_string(),
                title: "Tables missing header scope".to_string(),
                description: format!(
                    "{} table(s) lack header cells with scope.",
                    ctx.page.tables_total
                ),
                url: url.clone(),
                recommendation: "Add scope attribute to <th> elements.".to_string(),
            });
        }
        findings
    }
}

pub struct TableCaptionMissingDeepValidator;
impl Default for TableCaptionMissingDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl TableCaptionMissingDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for TableCaptionMissingDeepValidator {
    fn name(&self) -> &str {
        "table-caption-missing-deep-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.tables_total > 0 && ctx.page.tables_with_captions == 0 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Accessibility,
                code: "TABCAP-V6112".to_string(),
                title: "Tables missing captions".to_string(),
                description: format!("{} table(s) lack <caption>.", ctx.page.tables_total),
                url: url.clone(),
                recommendation: "Add <caption> to each table.".to_string(),
            });
        }
        findings
    }
}

pub struct LinkTextGenericDeepValidator;
impl Default for LinkTextGenericDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl LinkTextGenericDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for LinkTextGenericDeepValidator {
    fn name(&self) -> &str {
        "link-text-generic-deep-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let generic = [
            "click here",
            "here",
            "read more",
            "more",
            "link",
            "learn more",
            "go",
            "this page",
        ];
        let generic_count = ctx
            .page
            .links
            .iter()
            .filter(|l| {
                // aria-label overrides the visible text for screen readers,
                // so a link with a descriptive aria-label is not generic.
                l.aria_label.is_none() && {
                    let text = l.text.trim().to_lowercase();
                    generic.contains(&text.as_str())
                }
            })
            .count();
        if generic_count > 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "LINKGEN-V6113".to_string(),
                title: "Generic link text".to_string(),
                description: format!("{generic_count} link(s) use generic text."),
                url: url.clone(),
                recommendation: "Use descriptive link text.".to_string(),
            });
        }
        findings
    }
}

pub struct LinkTextEmptyDeepValidator;
impl Default for LinkTextEmptyDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl LinkTextEmptyDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for LinkTextEmptyDeepValidator {
    fn name(&self) -> &str {
        "link-text-empty-deep-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let empty_count = ctx
            .page
            .links
            .iter()
            .filter(|l| {
                l.text.trim().is_empty()
                    && l.aria_label.is_none()
                    // A link wrapping an image gets its accessible name
                    // from the image's alt attribute.
                    && l.img_alt.is_none()
            })
            .count();
        if empty_count > 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "LINKEEMPTY-V6114".to_string(),
                title: "Empty link text".to_string(),
                description: format!("{empty_count} link(s) have no text or aria-label."),
                url: url.clone(),
                recommendation: "Add descriptive text or aria-label to links.".to_string(),
            });
        }
        findings
    }
}

pub struct LinkTextDuplicateValidator;
impl Default for LinkTextDuplicateValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl LinkTextDuplicateValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for LinkTextDuplicateValidator {
    fn name(&self) -> &str {
        "link-text-duplicate-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let texts: Vec<&str> = ctx
            .page
            .links
            .iter()
            .filter(|l| !l.text.trim().is_empty())
            .map(|l| l.text.trim())
            .collect();
        if texts.len() > 5 {
            let mut counts: std::collections::HashMap<&str, usize> =
                std::collections::HashMap::new();
            for t in &texts {
                *counts.entry(t).or_insert(0) += 1;
            }
            let dupes: Vec<(&&str, &usize)> = counts.iter().filter(|(_, &c)| c > 3).collect();
            if !dupes.is_empty() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Accessibility,
                    code: "LINKDUP-V6115".to_string(),
                    title: "Duplicate link text".to_string(),
                    description: format!("{} link text(s) appear 3+ times.", dupes.len()),
                    url: url.clone(),
                    recommendation: "Use unique, descriptive text for each link.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct ImageAltMissingDeepValidator;
impl Default for ImageAltMissingDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl ImageAltMissingDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for ImageAltMissingDeepValidator {
    fn name(&self) -> &str {
        "image-alt-missing-deep-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let missing = ctx
            .page
            .images
            .iter()
            // aria-hidden images are removed from the accessibility tree and
            // do not require alt text.
            .filter(|i| !i.aria_hidden && !i.has_alt && i.alt.is_empty())
            .count();
        if missing > 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "IMGALTMISS-V6116".to_string(),
                title: "Images missing alt text".to_string(),
                description: format!("{missing} image(s) lack alt text."),
                url: url.clone(),
                recommendation: "Add descriptive alt text to images.".to_string(),
            });
        }
        findings
    }
}

pub struct ImageAltEmptyDeepValidator;
impl Default for ImageAltEmptyDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl ImageAltEmptyDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for ImageAltEmptyDeepValidator {
    fn name(&self) -> &str {
        "image-alt-empty-deep-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let empty = ctx
            .page
            .images
            .iter()
            // An aria-hidden image is author-declared decorative, so an
            // empty alt is intentional rather than a gap.
            .filter(|i| !i.aria_hidden && i.has_alt && i.alt.is_empty())
            .count();
        if empty > 0 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Accessibility,
                code: "IMGALTEMPTY-V6117".to_string(),
                title: "Empty alt attributes".to_string(),
                description: format!("{empty} image(s) have empty alt attributes."),
                url: url.clone(),
                recommendation: "Add meaningful alt text or mark as decorative.".to_string(),
            });
        }
        findings
    }
}

pub struct ImageAltDecorativePatternValidator;
impl Default for ImageAltDecorativePatternValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl ImageAltDecorativePatternValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for ImageAltDecorativePatternValidator {
    fn name(&self) -> &str {
        "image-alt-decorative-pattern-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let decorative_patterns = ["image", "photo", "picture", "img", "decorative"];
        let suspicious = ctx
            .page
            .images
            .iter()
            .filter(|i| {
                // Generic alt text is only a problem if the image is actually
                // exposed to assistive technology.
                !i.aria_hidden && i.has_alt && !i.alt.is_empty() && {
                    let lower = i.alt.to_lowercase();
                    decorative_patterns.iter().any(|p| lower == *p)
                }
            })
            .count();
        if suspicious > 2 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Accessibility,
                code: "IMGDECPAT-V6118".to_string(),
                title: "Suspiciously generic alt text".to_string(),
                description: format!("{suspicious} image(s) have generic alt text."),
                url: url.clone(),
                recommendation: "Provide descriptive alt text instead of generic words."
                    .to_string(),
            });
        }
        findings
    }
}

pub struct FocusTabindexPositiveValidator;
impl Default for FocusTabindexPositiveValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl FocusTabindexPositiveValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for FocusTabindexPositiveValidator {
    fn name(&self) -> &str {
        "focus-tabindex-positive-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.has_positive_tabindex {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "FOCUSTABPOS-V6119".to_string(),
                title: "Positive tabindex detected".to_string(),
                description: "Positive tabindex disrupts natural tab order.".to_string(),
                url: url.clone(),
                recommendation: "Remove positive tabindex and use DOM order.".to_string(),
            });
        }
        findings
    }
}

pub struct FocusTrapMissingValidator;
impl Default for FocusTrapMissingValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl FocusTrapMissingValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for FocusTrapMissingValidator {
    fn name(&self) -> &str {
        "focus-trap-missing-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(body) = ctx.body {
            // Match actual dialog elements via the DOM; a raw `contains` would
            // also match `role="dialog"` inside comments, scripts, or text.
            let doc = scraper::Html::parse_document(body);
            let (Ok(dialog_sel), Ok(modal_sel)) = (
                scraper::Selector::parse("[role=\"dialog\"], [role=\"alertdialog\"]"),
                scraper::Selector::parse(
                    "[role=\"dialog\"][aria-modal=\"true\"], [role=\"alertdialog\"][aria-modal=\"true\"]",
                ),
            ) else {
                return findings;
            };
            let has_dialog = doc.select(&dialog_sel).next().is_some();
            let has_aria_modal = doc.select(&modal_sel).next().is_some();
            if has_dialog && !has_aria_modal {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Accessibility,
                    code: "FOCTR001".to_string(),
                    title: "Dialog without aria-modal".to_string(),
                    description: "A dialog or alertdialog element is missing aria-modal=\"true\", which means keyboard focus may escape the dialog.".to_string(),
                    url: url.clone(),
                    recommendation: "Add aria-modal=\"true\" to dialog elements to trap focus.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct HeadingSkipLevelsDeepValidator;
impl Default for HeadingSkipLevelsDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl HeadingSkipLevelsDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for HeadingSkipLevelsDeepValidator {
    fn name(&self) -> &str {
        "heading-skip-levels-deep-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.headings.len() < 2 {
            return findings;
        }
        let mut skip_count = 0;
        let mut prev_level = 0u8;
        for h in &ctx.page.headings {
            if prev_level > 0 && h.level > prev_level + 1 {
                skip_count += 1;
            }
            prev_level = h.level;
        }
        if skip_count > 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "HEADSKIP-V6121".to_string(),
                title: "Heading levels skipped".to_string(),
                description: format!("{skip_count} heading level skip(s) detected."),
                url: url.clone(),
                recommendation: "Use heading levels sequentially.".to_string(),
            });
        }
        findings
    }
}

pub struct HeadingEmptyValidator;
impl Default for HeadingEmptyValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl HeadingEmptyValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for HeadingEmptyValidator {
    fn name(&self) -> &str {
        "heading-empty-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let empty = ctx
            .page
            .headings
            .iter()
            .filter(|h| h.text.trim().is_empty())
            .count();
        if empty > 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "HEADEMPTY-V6122".to_string(),
                title: "Empty headings".to_string(),
                description: format!("{empty} heading(s) have no text."),
                url: url.clone(),
                recommendation: "Add meaningful text to all headings.".to_string(),
            });
        }
        findings
    }
}

pub struct FormFieldsetLegendValidator;
impl Default for FormFieldsetLegendValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl FormFieldsetLegendValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for FormFieldsetLegendValidator {
    fn name(&self) -> &str {
        "form-fieldset-legend-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let multi_input_forms = ctx.page.forms.iter().filter(|f| f.inputs.len() > 3).count();
        if multi_input_forms > 0 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Accessibility,
                code: "FORMFSLG-V6123".to_string(),
                title: "Complex forms without fieldset".to_string(),
                description: format!(
                    "{multi_input_forms} form(s) with 3+ inputs may benefit from fieldset/legend."
                ),
                url: url.clone(),
                recommendation: "Group related inputs with <fieldset> and <legend>.".to_string(),
            });
        }
        findings
    }
}

pub struct LandmarkContentinfoValidator;
impl Default for LandmarkContentinfoValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl LandmarkContentinfoValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for LandmarkContentinfoValidator {
    fn name(&self) -> &str {
        "landmark-contentinfo-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if !ctx
            .page
            .landmarks
            .iter()
            .any(|l| l.to_lowercase() == "contentinfo")
        {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "LANDCINFO-V6124".to_string(),
                title: "Missing contentinfo landmark".to_string(),
                description: "No contentinfo (footer) landmark found.".to_string(),
                url: url.clone(),
                recommendation: "Add a <footer> or role='contentinfo' landmark.".to_string(),
            });
        }
        findings
    }
}

pub struct LandmarkComplementaryValidator;
impl Default for LandmarkComplementaryValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl LandmarkComplementaryValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for LandmarkComplementaryValidator {
    fn name(&self) -> &str {
        "landmark-complementary-v6"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if !ctx
            .page
            .landmarks
            .iter()
            .any(|l| l.to_lowercase() == "complementary")
        {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Accessibility,
                code: "LANDCOMP-V6125".to_string(),
                title: "No complementary landmark".to_string(),
                description: "No complementary (aside) landmark found.".to_string(),
                url: url.clone(),
                recommendation: "Consider adding <aside> or role='complementary'.".to_string(),
            });
        }
        findings
    }
}

// =========================================================================
// V7 Content Validators (Schema.org type validators, 1-20)
// =========================================================================

pub struct AriaLandmarksDeepValidator;
impl Default for AriaLandmarksDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl AriaLandmarksDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for AriaLandmarksDeepValidator {
    fn name(&self) -> &str {
        "aria-landmarks-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.landmarks.is_empty() {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "ARIALAND-V2001".to_string(),
                title: "No ARIA landmarks found (deep)".to_string(),
                description: "Page has no landmark regions in deep analysis.".to_string(),
                url: url.clone(),
                recommendation: "Add semantic HTML5 landmarks or ARIA roles.".to_string(),
            });
        } else {
            let has_main = ctx
                .page
                .landmarks
                .iter()
                .any(|l| l.to_lowercase() == "main");
            if !has_main {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Accessibility,
                    code: "ARIALAND-V2002".to_string(),
                    title: "Missing main landmark (deep)".to_string(),
                    description: "No main landmark found in deep analysis.".to_string(),
                    url: url.clone(),
                    recommendation: "Add <main> or role='main' landmark.".to_string(),
                });
            }
            let has_nav = ctx
                .page
                .landmarks
                .iter()
                .any(|l| l.to_lowercase() == "navigation");
            if !has_nav {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Accessibility,
                    code: "ARIALAND-V2003".to_string(),
                    title: "Missing navigation landmark (deep)".to_string(),
                    description: "No navigation landmark found in deep analysis.".to_string(),
                    url: url.clone(),
                    recommendation: "Add <nav> or role='navigation' landmark.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct HeadingHierarchyDeepDeepValidator;
impl Default for HeadingHierarchyDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl HeadingHierarchyDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for HeadingHierarchyDeepDeepValidator {
    fn name(&self) -> &str {
        "heading-hierarchy-deep-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.headings.is_empty() {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "HHIER-V2001".to_string(),
                title: "No headings found (deep-deep)".to_string(),
                description: "Page has no heading elements in deep analysis.".to_string(),
                url: url.clone(),
                recommendation: "Add at least one H1 heading.".to_string(),
            });
            return findings;
        }
        let h1_count = ctx.page.headings.iter().filter(|h| h.level == 1).count();
        if h1_count == 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "HHIER-V2002-DEEP-DEEP".to_string(),
                title: "Missing H1 heading (deep-deep)".to_string(),
                description: "No H1 heading found in deep analysis.".to_string(),
                url: url.clone(),
                recommendation: "Add a single H1 heading.".to_string(),
            });
        } else if h1_count > 1 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "HHIER-V2003-DEEP-DEEP".to_string(),
                title: "Multiple H1 headings (deep-deep)".to_string(),
                description: format!("{h1_count} H1 headings found in deep analysis."),
                url: url.clone(),
                recommendation: "Use a single H1 heading per page.".to_string(),
            });
        }
        let mut prev_level = 0u8;
        for h in &ctx.page.headings {
            if prev_level > 0 && h.level > prev_level + 1 {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Accessibility,
                    code: "HHIER-V2004".to_string(),
                    title: "Heading level skipped (deep-deep)".to_string(),
                    description: format!(
                        "Heading jumps from H{} to H{} in deep analysis.",
                        prev_level, h.level
                    ),
                    url: url.clone(),
                    recommendation: "Use heading levels sequentially.".to_string(),
                });
            }
            prev_level = h.level;
        }
        findings
    }
}

pub struct FormLabelsDeepDeepValidator;
impl Default for FormLabelsDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl FormLabelsDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for FormLabelsDeepDeepValidator {
    fn name(&self) -> &str {
        "form-labels-deep-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let unlabeled = ctx
            .page
            .forms
            .iter()
            .flat_map(|f| &f.inputs)
            .filter(|i| {
                // Hidden inputs are not user-visible and cannot receive a
                // visible label; treating them as unlabeled is a false
                // positive. Submit/button inputs likewise carry no label
                // expectation beyond their own text.
                !matches!(
                    i.input_type.as_deref(),
                    Some("hidden")
                        | Some("submit")
                        | Some("button")
                        | Some("reset")
                        | Some("image")
                ) && !i.has_label
                    && i.aria_label.is_none()
                    && i.aria_labelledby.is_none()
                    && i.id.is_none()
            })
            .count();
        if unlabeled > 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "FORMLBL-V2001-DEEP-DEEP".to_string(),
                title: "Form inputs without labels (deep-deep)".to_string(),
                description: format!(
                    "{unlabeled} input(s) have no associated label in deep analysis."
                ),
                url: url.clone(),
                recommendation: "Associate each input with a <label> element.".to_string(),
            });
        }
        findings
    }
}

pub struct TableAccessibilityDeepDeepValidator;
impl Default for TableAccessibilityDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl TableAccessibilityDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for TableAccessibilityDeepDeepValidator {
    fn name(&self) -> &str {
        "table-accessibility-deep-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.tables_total > 0 && ctx.page.tables_with_headers == 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "TABACC-V2001-DEEP-DEEP".to_string(),
                title: "Tables without headers (deep-deep)".to_string(),
                description: format!(
                    "{} table(s) found but none have header cells in deep analysis.",
                    ctx.page.tables_total
                ),
                url: url.clone(),
                recommendation: "Add <th> elements to tables for screen reader accessibility."
                    .to_string(),
            });
        }
        findings
    }
}

pub struct LinkTextQualityDeepValidator;
impl Default for LinkTextQualityDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl LinkTextQualityDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for LinkTextQualityDeepValidator {
    fn name(&self) -> &str {
        "link-text-quality-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let empty_text = ctx
            .page
            .links
            .iter()
            .filter(|l| l.text.trim().is_empty() && l.aria_label.is_none() && l.img_alt.is_none())
            .count();
        if empty_text > 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "LINKTQ-V2001-DEEP".to_string(),
                title: "Links without text (deep)".to_string(),
                description: format!(
                    "{empty_text} link(s) have no text, aria-label, or alt text in deep analysis."
                ),
                url: url.clone(),
                recommendation: "Add descriptive text to all links.".to_string(),
            });
        }
        let generic = [
            "click here",
            "read more",
            "learn more",
            "more",
            "here",
            "link",
        ];
        let generic_count = ctx
            .page
            .links
            .iter()
            .filter(|l| {
                l.aria_label.is_none() && generic.contains(&l.text.trim().to_lowercase().as_str())
            })
            .count();
        if generic_count > 0 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Accessibility,
                code: "LINKTQ-V2002-DEEP".to_string(),
                title: "Generic link text (deep)".to_string(),
                description: format!(
                    "{generic_count} link(s) use generic anchor text in deep analysis."
                ),
                url: url.clone(),
                recommendation: "Use descriptive, specific anchor text.".to_string(),
            });
        }
        findings
    }
}

pub struct ImageAltTextDeepDeepValidator;
impl Default for ImageAltTextDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl ImageAltTextDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for ImageAltTextDeepDeepValidator {
    fn name(&self) -> &str {
        "image-alt-text-deep-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        // aria-hidden images are removed from the accessibility tree and do
        // not require alt text.
        let no_alt = ctx
            .page
            .images
            .iter()
            .filter(|i| !i.aria_hidden && !i.has_alt)
            .count();
        if no_alt > 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "IMGALT-V2001-DEEP-DEEP".to_string(),
                title: "Images missing alt attribute (deep-deep)".to_string(),
                description: format!("{no_alt} image(s) lack an alt attribute in deep analysis."),
                url: url.clone(),
                recommendation: "Add descriptive alt text to all images.".to_string(),
            });
        }
        let empty_alt = ctx
            .page
            .images
            .iter()
            .filter(|i| !i.aria_hidden && i.has_alt && i.alt.trim().is_empty())
            .count();
        if empty_alt > 0 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Accessibility,
                code: "IMGALT-V2002-DEEP-DEEP".to_string(),
                title: "Images with empty alt text (deep-deep)".to_string(),
                description: format!("{empty_alt} image(s) have alt='' in deep analysis."),
                url: url.clone(),
                recommendation: "Ensure empty alt is intentional for decorative images."
                    .to_string(),
            });
        }
        findings
    }
}

pub struct FocusManagementDeepDeepValidator;
impl Default for FocusManagementDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl FocusManagementDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for FocusManagementDeepDeepValidator {
    fn name(&self) -> &str {
        "focus-management-deep-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.has_positive_tabindex {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "FOCUS-V2001-DEEP-DEEP".to_string(),
                title: "Positive tabindex found (deep-deep)".to_string(),
                description:
                    "Elements with positive tabindex disrupt natural tab order in deep analysis."
                        .to_string(),
                url: url.clone(),
                recommendation: "Remove positive tabindex values and use DOM order.".to_string(),
            });
        }
        findings
    }
}

// =========================================================================
// V8 SEO Validators (25)
// =========================================================================

pub struct LandmarkMainDeepValidator;
impl Default for LandmarkMainDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl LandmarkMainDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for LandmarkMainDeepValidator {
    fn name(&self) -> &str {
        "landmark-main-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if !ctx.page.has_main_landmark {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "LANDMAIN-V2001".to_string(),
                title: "Missing main landmark (deep)".to_string(),
                description: "No main landmark found in deep analysis.".to_string(),
                url: url.clone(),
                recommendation: "Add <main> or role='main' landmark.".to_string(),
            });
        }
        findings
    }
}

pub struct LandmarkNavDeepValidator;
impl Default for LandmarkNavDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl LandmarkNavDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for LandmarkNavDeepValidator {
    fn name(&self) -> &str {
        "landmark-nav-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if !ctx.page.has_nav_landmark
            && !ctx
                .page
                .landmarks
                .iter()
                .any(|l| l.to_lowercase() == "navigation")
        {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Accessibility,
                code: "LANDNAV-V2001".to_string(),
                title: "Missing navigation landmark (deep)".to_string(),
                description: "No navigation landmark found in deep analysis.".to_string(),
                url: url.clone(),
                recommendation: "Add <nav> or role='navigation' landmark.".to_string(),
            });
        }
        findings
    }
}

pub struct LandmarkBannerDeepValidator;
impl Default for LandmarkBannerDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl LandmarkBannerDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for LandmarkBannerDeepValidator {
    fn name(&self) -> &str {
        "landmark-banner-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if !ctx
            .page
            .landmarks
            .iter()
            .any(|l| l.to_lowercase() == "banner")
        {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Accessibility,
                code: "LANDBAN-V2001".to_string(),
                title: "Missing banner landmark (deep)".to_string(),
                description: "No banner landmark found in deep analysis.".to_string(),
                url: url.clone(),
                recommendation: "Add <header> or role='banner' landmark.".to_string(),
            });
        }
        findings
    }
}

pub struct HeadingSkipLevelsDeepDeepValidator;
impl Default for HeadingSkipLevelsDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl HeadingSkipLevelsDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for HeadingSkipLevelsDeepDeepValidator {
    fn name(&self) -> &str {
        "heading-skip-levels-deep-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.headings.len() < 2 {
            return findings;
        }
        let mut skip_count = 0;
        let mut prev_level = 0u8;
        for h in &ctx.page.headings {
            if prev_level > 0 && h.level > prev_level + 1 {
                skip_count += 1;
            }
            prev_level = h.level;
        }
        if skip_count > 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "HEADSKIP-V2001".to_string(),
                title: "Heading levels skipped (deep-deep)".to_string(),
                description: format!(
                    "{skip_count} heading level skip(s) detected in deep analysis."
                ),
                url: url.clone(),
                recommendation: "Use heading levels sequentially.".to_string(),
            });
        }
        findings
    }
}

pub struct FormLabelsDeepDeepDeepValidator;
impl Default for FormLabelsDeepDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl FormLabelsDeepDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for FormLabelsDeepDeepDeepValidator {
    fn name(&self) -> &str {
        "form-labels-deep-deep-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let unlabeled = ctx
            .page
            .forms
            .iter()
            .flat_map(|f| &f.inputs)
            .filter(|i| {
                !i.has_label
                    && i.aria_label.is_none()
                    && i.aria_labelledby.is_none()
                    && i.id.is_none()
                    && i.input_type.as_deref() != Some("hidden")
            })
            .count();
        if unlabeled > 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "FORMLBL-V2001-DEEP-DEEP-DEEP".to_string(),
                title: "Form inputs without labels (deep-deep-deep)".to_string(),
                description: format!(
                    "{unlabeled} input(s) have no associated label in deep-deep analysis."
                ),
                url: url.clone(),
                recommendation: "Associate each input with a <label> element.".to_string(),
            });
        }
        findings
    }
}

pub struct TableAccessibilityDeepDeepDeepValidator;
impl Default for TableAccessibilityDeepDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl TableAccessibilityDeepDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for TableAccessibilityDeepDeepDeepValidator {
    fn name(&self) -> &str {
        "table-accessibility-deep-deep-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.tables_total > 0 && ctx.page.tables_with_headers == 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "TABACC-V2001-DEEP-DEEP-DEEP".to_string(),
                title: "Tables without headers (deep-deep-deep)".to_string(),
                description: format!(
                    "{} table(s) but none have header cells in deep-deep analysis.",
                    ctx.page.tables_total
                ),
                url: url.clone(),
                recommendation: "Add <th> elements to tables.".to_string(),
            });
        }
        if ctx.page.tables_total > 0 && ctx.page.tables_with_captions == 0 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Accessibility,
                code: "TABACC-V2002-DEEP-DEEP-DEEP".to_string(),
                title: "Tables without captions (deep-deep-deep)".to_string(),
                description: format!(
                    "{} table(s) but none have captions in deep-deep analysis.",
                    ctx.page.tables_total
                ),
                url: url.clone(),
                recommendation: "Add <caption> elements to tables.".to_string(),
            });
        }
        findings
    }
}

pub struct LinkTextQualityDeepDeepValidator;
impl Default for LinkTextQualityDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl LinkTextQualityDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for LinkTextQualityDeepDeepValidator {
    fn name(&self) -> &str {
        "link-text-quality-deep-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let empty_text = ctx
            .page
            .links
            .iter()
            .filter(|l| l.text.trim().is_empty() && l.aria_label.is_none() && l.img_alt.is_none())
            .count();
        if empty_text > 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "LINKTQ-V2001-DEEP-DEEP".to_string(),
                title: "Links without text (deep-deep)".to_string(),
                description: format!(
                    "{empty_text} link(s) have no text or aria-label in deep-deep analysis."
                ),
                url: url.clone(),
                recommendation: "Add descriptive text to all links.".to_string(),
            });
        }
        findings
    }
}

pub struct ImageAltTextDeepDeepDeepValidator;
impl Default for ImageAltTextDeepDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl ImageAltTextDeepDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for ImageAltTextDeepDeepDeepValidator {
    fn name(&self) -> &str {
        "image-alt-text-deep-deep-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        // aria-hidden images are removed from the accessibility tree and do
        // not require alt text.
        let no_alt = ctx
            .page
            .images
            .iter()
            .filter(|i| !i.aria_hidden && !i.has_alt)
            .count();
        if no_alt > 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "IMGALT-V2001-DEEP-DEEP-DEEP".to_string(),
                title: "Images missing alt attribute (deep-deep-deep)".to_string(),
                description: format!(
                    "{no_alt} image(s) lack an alt attribute in deep-deep analysis."
                ),
                url: url.clone(),
                recommendation: "Add descriptive alt text to all images.".to_string(),
            });
        }
        findings
    }
}

pub struct FocusManagementDeepDeepDeepValidator;
impl Default for FocusManagementDeepDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl FocusManagementDeepDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for FocusManagementDeepDeepDeepValidator {
    fn name(&self) -> &str {
        "focus-management-deep-deep-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.has_positive_tabindex {
            findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Accessibility, code: "FOCUS-V2001-DEEP-DEEP-DEEP".to_string(), title: "Positive tabindex found (deep-deep-deep)".to_string(), description: "Elements with positive tabindex disrupt natural tab order in deep-deep analysis.".to_string(), url: url.clone(), recommendation: "Remove positive tabindex values and use DOM order.".to_string() });
        }
        findings
    }
}

pub struct LanguageAttributesDeepDeepValidator;
impl Default for LanguageAttributesDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl LanguageAttributesDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for LanguageAttributesDeepDeepValidator {
    fn name(&self) -> &str {
        "language-attributes-deep-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if !ctx.page.has_lang_attribute {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "LANGATTR-V2001-DEEP-DEEP".to_string(),
                title: "Missing lang attribute (deep-deep)".to_string(),
                description: "HTML element has no lang attribute in deep analysis.".to_string(),
                url: url.clone(),
                recommendation: "Add lang attribute to <html> element.".to_string(),
            });
        }
        findings
    }
}

pub struct ColorContrastTextDeepValidator;
impl Default for ColorContrastTextDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl ColorContrastTextDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for ColorContrastTextDeepValidator {
    fn name(&self) -> &str {
        "color-contrast-text-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(body) = ctx.body {
            let lower = body.to_lowercase();
            let light_on_light = lower.contains("color: #fff")
                && lower.contains("background-color: #fff")
                || lower.contains("color: white") && lower.contains("background-color: white");
            if light_on_light {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Accessibility,
                    code: "COLRCT-V2001".to_string(),
                    title: "Possible light-on-light contrast issue (deep)".to_string(),
                    description: "Detected white text on white background in deep analysis."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Ensure sufficient color contrast for text.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct ColorContrastLinkDeepValidator;
impl Default for ColorContrastLinkDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl ColorContrastLinkDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for ColorContrastLinkDeepValidator {
    fn name(&self) -> &str {
        "color-contrast-link-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(body) = ctx.body {
            let lower = body.to_lowercase();
            let light_link = (lower.contains("a { color: #fff")
                || lower.contains("a {color: white"))
                && (lower.contains("background-color: #fff")
                    || lower.contains("background-color: white"));
            if light_link {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Accessibility,
                    code: "COLRCL-V2001-DEEP".to_string(),
                    title: "Possible link contrast issue (deep)".to_string(),
                    description: "Detected white links on white background in deep analysis."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Ensure link colors have sufficient contrast.".to_string(),
                });
            }
        }
        findings
    }
}

pub struct AnchorTextGenericDeepValidator;
impl Default for AnchorTextGenericDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl AnchorTextGenericDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for AnchorTextGenericDeepValidator {
    fn name(&self) -> &str {
        "anchor-text-generic-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let generic = [
            "click here",
            "read more",
            "learn more",
            "more",
            "here",
            "link",
            "this page",
        ];
        let generic_count = ctx
            .page
            .links
            .iter()
            .filter(|l| {
                l.aria_label.is_none() && generic.contains(&l.text.trim().to_lowercase().as_str())
            })
            .count();
        if generic_count > 0 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Accessibility,
                code: "ANCHGEN-V2001-DEEP".to_string(),
                title: "Generic anchor text (deep)".to_string(),
                description: format!(
                    "{generic_count} link(s) use generic anchor text in deep analysis."
                ),
                url: url.clone(),
                recommendation: "Use descriptive, specific anchor text.".to_string(),
            });
        }
        findings
    }
}

pub struct TableCaptionPresenceDeepValidator;
impl Default for TableCaptionPresenceDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl TableCaptionPresenceDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for TableCaptionPresenceDeepValidator {
    fn name(&self) -> &str {
        "table-caption-presence-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.tables_total > 0 && ctx.page.tables_with_captions == 0 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Accessibility,
                code: "TBLCAP-V2001".to_string(),
                title: "Tables without captions (deep)".to_string(),
                description: format!(
                    "{} table(s) but none have captions in deep analysis.",
                    ctx.page.tables_total
                ),
                url: url.clone(),
                recommendation: "Add <caption> elements to tables for screen readers.".to_string(),
            });
        }
        findings
    }
}

pub struct TableHeaderScopeDeepValidator;
impl Default for TableHeaderScopeDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl TableHeaderScopeDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for TableHeaderScopeDeepValidator {
    fn name(&self) -> &str {
        "table-header-scope-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.tables_total > 0 && ctx.page.tables_with_headers == 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "TBLSCOP-V2001".to_string(),
                title: "Tables without proper headers (deep)".to_string(),
                description: format!(
                    "{} table(s) but none have properly scoped header cells in deep analysis.",
                    ctx.page.tables_total
                ),
                url: url.clone(),
                recommendation: "Add <th scope='col'> or <th scope='row'> elements.".to_string(),
            });
        }
        findings
    }
}

pub struct FormLabelAssociationDeepValidator;
impl Default for FormLabelAssociationDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl FormLabelAssociationDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for FormLabelAssociationDeepValidator {
    fn name(&self) -> &str {
        "form-label-association-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let unlabeled = ctx
            .page
            .forms
            .iter()
            .flat_map(|f| &f.inputs)
            .filter(|i| {
                !i.has_label && i.aria_label.is_none() && i.input_type.as_deref() != Some("hidden")
            })
            .count();
        if unlabeled > 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "FORMLAB-V2001".to_string(),
                title: "Form inputs without label association (deep)".to_string(),
                description: format!(
                    "{unlabeled} input(s) have no associated label in deep analysis."
                ),
                url: url.clone(),
                recommendation: "Use <label for='id'> to associate labels with inputs.".to_string(),
            });
        }
        findings
    }
}

pub struct AriaRequiredAttributesDeepValidator;
impl Default for AriaRequiredAttributesDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl AriaRequiredAttributesDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for AriaRequiredAttributesDeepValidator {
    fn name(&self) -> &str {
        "aria-required-attributes-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.aria_role_count > 0 && ctx.page.aria_label_count == 0 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Accessibility,
                code: "ARIAREQ-V2001".to_string(),
                title: "ARIA roles without labels (deep)".to_string(),
                description: "ARIA roles found but no aria-label attributes in deep analysis."
                    .to_string(),
                url: url.clone(),
                recommendation: "Add aria-label or aria-labelledby to interactive ARIA roles."
                    .to_string(),
            });
        }
        findings
    }
}

pub struct HeadingHierarchyDeepDeepDeepValidator;
impl Default for HeadingHierarchyDeepDeepDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl HeadingHierarchyDeepDeepDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for HeadingHierarchyDeepDeepDeepValidator {
    fn name(&self) -> &str {
        "heading-hierarchy-deep-deep-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.headings.is_empty() {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "HHIER-V2001-DEEP-DEEP-DEEP".to_string(),
                title: "No headings found (deep-deep-deep)".to_string(),
                description: "Page has no heading elements in deep-deep analysis.".to_string(),
                url: url.clone(),
                recommendation: "Add at least one H1 heading.".to_string(),
            });
            return findings;
        }
        let h1_count = ctx.page.headings.iter().filter(|h| h.level == 1).count();
        if h1_count == 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "HHIER-V2002-DEEP-DEEP-DEEP".to_string(),
                title: "Missing H1 heading (deep-deep-deep)".to_string(),
                description: "No H1 heading found in deep-deep analysis.".to_string(),
                url: url.clone(),
                recommendation: "Add a single H1 heading.".to_string(),
            });
        } else if h1_count > 1 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "HHIER-V2003-DEEP-DEEP-DEEP".to_string(),
                title: "Multiple H1 headings (deep-deep-deep)".to_string(),
                description: format!("{h1_count} H1 headings found in deep-deep analysis."),
                url: url.clone(),
                recommendation: "Use a single H1 heading per page.".to_string(),
            });
        }
        findings
    }
}

pub struct LandmarkContentinfoDeepValidator;
impl Default for LandmarkContentinfoDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl LandmarkContentinfoDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for LandmarkContentinfoDeepValidator {
    fn name(&self) -> &str {
        "landmark-contentinfo-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if !ctx
            .page
            .landmarks
            .iter()
            .any(|l| l.to_lowercase() == "contentinfo")
        {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Accessibility,
                code: "LANDCINFO-V2001".to_string(),
                title: "Missing contentinfo landmark (deep)".to_string(),
                description: "No contentinfo (footer) landmark found in deep analysis.".to_string(),
                url: url.clone(),
                recommendation: "Add a <footer> or role='contentinfo' landmark.".to_string(),
            });
        }
        findings
    }
}

pub struct LandmarkComplementaryDeepValidator;
impl Default for LandmarkComplementaryDeepValidator {
    fn default() -> Self {
        Self::new()
    }
}
impl LandmarkComplementaryDeepValidator {
    pub fn new() -> Self {
        Self
    }
}
impl Analyzer for LandmarkComplementaryDeepValidator {
    fn name(&self) -> &str {
        "landmark-complementary-deep-v8"
    }
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if !ctx
            .page
            .landmarks
            .iter()
            .any(|l| l.to_lowercase() == "complementary")
        {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Accessibility,
                code: "LANDCOMP-V2001".to_string(),
                title: "No complementary landmark (deep)".to_string(),
                description: "No complementary (aside) landmark found in deep analysis."
                    .to_string(),
                url: url.clone(),
                recommendation: "Consider adding <aside> or role='complementary'.".to_string(),
            });
        }
        findings
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::meta::MetaTags;
    use crate::parser::Heading;

    fn make_page(url: &str) -> crate::parser::ParsedPage {
        crate::parser::ParsedPage {
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
        page: &'a crate::parser::ParsedPage,
        body: Option<&'a str>,
    ) -> AnalysisContext<'a> {
        AnalysisContext {
            page,
            body,
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
        }
    }

    #[test]
    fn test_aria_landmarks_v2_empty() {
        let p = make_page("https://example.com");
        let f = AriaLandmarksAnalyzerV2::new().analyze(&make_ctx(&p, None));
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].code, "ARIALAND-V2006");
    }
    #[test]
    fn test_heading_deep_v2() {
        assert!(HeadingHierarchyDeepAnalyzerV2::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_form_labels_v2() {
        assert!(FormLabelsDeepAnalyzerV2::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_table_acc_v2() {
        assert!(TableAccessibilityDeepAnalyzerV2::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_link_text_v2() {
        assert!(LinkTextQualityAnalyzerV2::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_img_alt_v2() {
        assert!(ImageAltTextDeepAnalyzerV2::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_focus_v2() {
        assert!(FocusManagementDeepAnalyzerV2::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_lang_v2() {
        let p = make_page("https://example.com");
        let f = LanguageAttributesDeepAnalyzerV2::new().analyze(&make_ctx(&p, None));
        assert_eq!(f.len(), 1);
    }
    #[test]
    fn test_color_text_v2() {
        assert!(ColorContrastTextAnalyzerV2::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_color_link_v2() {
        assert!(ColorContrastLinkAnalyzerV2::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_anchor_v2() {
        assert!(AnchorTextGenericAnalyzerV2::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_table_caption_v2() {
        assert!(TableCaptionPresenceAnalyzerV2::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_table_scope_v2() {
        assert!(TableHeaderScopeAnalyzerV2::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_form_label_v2() {
        assert!(FormLabelAssociationAnalyzerV2::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_aria_req_v2() {
        assert!(AriaRequiredAttributesAnalyzerV2::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_landmark_main_v5_missing() {
        let f = LandmarkMainValidatorV5::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_landmark_main_v5_ok() {
        let mut p = make_page("https://example.com");
        p.landmarks = vec!["main".into()];
        assert!(LandmarkMainValidatorV5::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_landmark_main_v5_multi() {
        let mut p = make_page("https://example.com");
        p.landmarks = vec!["main".into(), "main".into()];
        let f = LandmarkMainValidatorV5::new().analyze(&make_ctx(&p, None));
        assert!(f.iter().any(|x| x.code == "LANDMAIN-V5002"));
    }
    #[test]
    fn test_landmark_nav_v5_missing() {
        let f = LandmarkNavValidatorV5::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_landmark_nav_v5_ok() {
        let mut p = make_page("https://example.com");
        p.landmarks = vec!["navigation".into()];
        assert!(LandmarkNavValidatorV5::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_landmark_banner_v5_missing() {
        let f = LandmarkBannerValidatorV5::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_heading_skip_v5() {
        let mut p = make_page("https://example.com");
        p.headings = vec![
            Heading {
                level: 1,
                text: "H1".into(),
                length: 2,
            },
            Heading {
                level: 3,
                text: "H3".into(),
                length: 2,
            },
        ];
        let f = HeadingSkipLevelsValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_heading_skip_v5_ok() {
        let mut p = make_page("https://example.com");
        p.headings = vec![
            Heading {
                level: 1,
                text: "H1".into(),
                length: 2,
            },
            Heading {
                level: 2,
                text: "H2".into(),
                length: 2,
            },
        ];
        assert!(HeadingSkipLevelsValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_heading_h1_v5_missing() {
        let mut p = make_page("https://example.com");
        p.headings = vec![Heading {
            level: 2,
            text: "H2".into(),
            length: 2,
        }];
        let f = HeadingMultipleH1Validator::new().analyze(&make_ctx(&p, None));
        assert_eq!(f[0].code, "HEADH1-V5001");
    }
    #[test]
    fn test_heading_h1_v5_multi() {
        let mut p = make_page("https://example.com");
        p.headings = vec![
            Heading {
                level: 1,
                text: "H1a".into(),
                length: 3,
            },
            Heading {
                level: 1,
                text: "H1b".into(),
                length: 3,
            },
        ];
        let f = HeadingMultipleH1Validator::new().analyze(&make_ctx(&p, None));
        assert_eq!(f[0].code, "HEADH1-V5002");
    }
    #[test]
    fn test_form_label_v5() {
        assert!(FormLabelAssociationValidatorV5::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_form_required_v5() {
        assert!(FormRequiredFieldsValidator::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None))
            .is_empty());
    }
    #[test]
    fn test_table_headers_v5_none() {
        let mut p = make_page("https://example.com");
        p.tables_total = 3;
        p.tables_with_headers = 0;
        let f = TableHeadersValidatorV5::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_table_headers_v5_partial() {
        let mut p = make_page("https://example.com");
        p.tables_total = 3;
        p.tables_with_headers = 1;
        let f = TableHeadersValidatorV5::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_table_headers_v5_ok() {
        let mut p = make_page("https://example.com");
        p.tables_total = 3;
        p.tables_with_headers = 3;
        assert!(TableHeadersValidatorV5::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_table_caption_v5() {
        let mut p = make_page("https://example.com");
        p.tables_total = 1;
        p.tables_with_captions = 0;
        let f = TableCaptionValidatorV5::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_table_scope_v5() {
        let body =
            "<table><tr><th>Name</th><th>Age</th></tr><tr><td>John</td><td>30</td></tr></table>";
        let mut p = make_page("https://example.com");
        p.tables_total = 1;
        p.tables_with_headers = 1;
        let f = TableScopeValidator::new().analyze(&make_ctx(&p, Some(body)));
        assert!(!f.is_empty());
    }

    // ===== Performance V5 Tests =====
    #[test]
    fn test_h1_count() {
        let mut p = make_page("https://example.com");
        p.headings = vec![Heading {
            level: 1,
            text: "Title".into(),
            length: 5,
        }];
        assert!(HeadingH1CountValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_h1_count_missing() {
        let mut p = make_page("https://example.com");
        p.headings = vec![Heading {
            level: 2,
            text: "Subtitle".into(),
            length: 8,
        }];
        let f = HeadingH1CountValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_heading_depth() {
        let p = make_page("https://example.com");
        assert!(HeadingDepthAnalysisValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_form_required() {
        let p = make_page("https://example.com");
        assert!(FormRequiredFieldsDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_table_scope() {
        let mut p = make_page("https://example.com");
        p.tables_total = 2;
        p.tables_with_headers = 0;
        let f = TableHeadersScopeValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_table_caption_deep() {
        let mut p = make_page("https://example.com");
        p.tables_total = 2;
        p.tables_with_captions = 0;
        let f = TableCaptionMissingDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_link_text_generic_deep() {
        let mut p = make_page("https://example.com");
        p.links = vec![crate::parser::ExtractedLink {
            href: "https://example.com/1".into(),
            text: "click here".into(),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: None,
        }];
        let f = LinkTextGenericDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_link_text_empty_deep() {
        let mut p = make_page("https://example.com");
        p.links = vec![crate::parser::ExtractedLink {
            href: "https://example.com/1".into(),
            text: "".into(),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: None,
        }];
        let f = LinkTextEmptyDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_link_text_dup() {
        let mut p = make_page("https://example.com");
        for i in 0..10 {
            p.links.push(crate::parser::ExtractedLink {
                href: format!("https://example.com/{i}"),
                text: "same".into(),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            });
        }
        let f = LinkTextDuplicateValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_img_alt_missing_deep() {
        let mut p = make_page("https://example.com");
        p.images = vec![crate::parser::ExtractedImage {
            src: "https://example.com/1.jpg".into(),
            alt: "".into(),
            has_alt: false,
            width: None,
            height: None,
            is_lazy_loaded: false,
            aria_hidden: false,
        }];
        let f = ImageAltMissingDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_img_alt_empty_deep() {
        let mut p = make_page("https://example.com");
        p.images = vec![crate::parser::ExtractedImage {
            src: "https://example.com/1.jpg".into(),
            alt: "".into(),
            has_alt: true,
            width: None,
            height: None,
            is_lazy_loaded: false,
            aria_hidden: false,
        }];
        let f = ImageAltEmptyDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_img_alt_decorative() {
        let mut p = make_page("https://example.com");
        for _ in 0..5 {
            p.images.push(crate::parser::ExtractedImage {
                src: "https://example.com/1.jpg".into(),
                alt: "image".into(),
                has_alt: true,
                width: None,
                height: None,
                is_lazy_loaded: false,
                aria_hidden: false,
            });
        }
        let f = ImageAltDecorativePatternValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_focus_tabindex() {
        let mut p = make_page("https://example.com");
        p.has_positive_tabindex = true;
        let f = FocusTabindexPositiveValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_focus_trap_no_body() {
        let p = make_page("https://example.com");
        assert!(FocusTrapMissingValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_focus_trap_dialog_without_aria_modal() {
        let p = make_page("https://example.com");
        let body = r#"<div role="dialog"><p>Content</p></div>"#;
        let f = FocusTrapMissingValidator::new().analyze(&make_ctx(&p, Some(body)));
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].code, "FOCTR001");
    }
    #[test]
    fn test_focus_trap_dialog_with_aria_modal() {
        let p = make_page("https://example.com");
        let body = r#"<div role="dialog" aria-modal="true"><p>Content</p></div>"#;
        assert!(FocusTrapMissingValidator::new()
            .analyze(&make_ctx(&p, Some(body)))
            .is_empty());
    }
    #[test]
    fn test_focus_trap_alertdialog_without_aria_modal() {
        let p = make_page("https://example.com");
        let body = r#"<div role="alertdialog"><p>Alert</p></div>"#;
        let f = FocusTrapMissingValidator::new().analyze(&make_ctx(&p, Some(body)));
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].code, "FOCTR001");
    }
    #[test]
    fn test_heading_skip_deep() {
        let mut p = make_page("https://example.com");
        p.headings = vec![
            Heading {
                level: 1,
                text: "H1".into(),
                length: 2,
            },
            Heading {
                level: 3,
                text: "H3".into(),
                length: 2,
            },
        ];
        let f = HeadingSkipLevelsDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_heading_empty() {
        let mut p = make_page("https://example.com");
        p.headings = vec![Heading {
            level: 1,
            text: "".into(),
            length: 0,
        }];
        let f = HeadingEmptyValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_form_fieldset() {
        let p = make_page("https://example.com");
        assert!(FormFieldsetLegendValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_landmark_contentinfo() {
        let f = LandmarkContentinfoValidator::new()
            .analyze(&make_ctx(&make_page("https://example.com"), None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_landmark_contentinfo_ok() {
        let mut p = make_page("https://example.com");
        p.landmarks = vec!["contentinfo".into()];
        assert!(LandmarkContentinfoValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_landmark_complementary() {
        let p = make_page("https://example.com");
        let f = LandmarkComplementaryValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }

    // ===== V7 Content Validators Tests =====
    #[test]
    fn test_land_main_deep_v8_missing() {
        let p = make_page("https://example.com");
        let f = LandmarkMainDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "LANDMAIN-V2001");
    }
    #[test]
    fn test_land_main_deep_v8_present() {
        let mut p = make_page("https://example.com");
        p.has_main_landmark = true;
        assert!(LandmarkMainDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_land_nav_deep_v8_missing() {
        let p = make_page("https://example.com");
        let f = LandmarkNavDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "LANDNAV-V2001");
    }
    #[test]
    fn test_land_nav_deep_v8_present() {
        let mut p = make_page("https://example.com");
        p.has_nav_landmark = true;
        assert!(LandmarkNavDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_land_ban_deep_v8_missing() {
        let p = make_page("https://example.com");
        let f = LandmarkBannerDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "LANDBAN-V2001");
    }
    #[test]
    fn test_land_ban_deep_v8_present() {
        let mut p = make_page("https://example.com");
        p.landmarks = vec!["Banner".into()];
        assert!(LandmarkBannerDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_head_skip_dd_v8_few() {
        let mut p = make_page("https://example.com");
        p.headings = vec![
            Heading {
                level: 1,
                text: "H1".into(),
                length: 2,
            },
            Heading {
                level: 2,
                text: "H2".into(),
                length: 2,
            },
        ];
        assert!(HeadingSkipLevelsDeepDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_head_skip_dd_v8_skip() {
        let mut p = make_page("https://example.com");
        p.headings = vec![
            Heading {
                level: 1,
                text: "H1".into(),
                length: 2,
            },
            Heading {
                level: 3,
                text: "H3".into(),
                length: 2,
            },
        ];
        let f = HeadingSkipLevelsDeepDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_form_lbl_dd_ddd_v8_ok() {
        let p = make_page("https://example.com");
        assert!(FormLabelsDeepDeepDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_tbl_acc_dd_ddd_v8_no_tables() {
        let p = make_page("https://example.com");
        assert!(TableAccessibilityDeepDeepDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_tbl_acc_dd_ddd_v8_no_headers() {
        let mut p = make_page("https://example.com");
        p.tables_total = 2;
        p.tables_with_headers = 0;
        let f = TableAccessibilityDeepDeepDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_tbl_acc_dd_ddd_v8_no_captions() {
        let mut p = make_page("https://example.com");
        p.tables_total = 2;
        p.tables_with_headers = 2;
        p.tables_with_captions = 0;
        let f = TableAccessibilityDeepDeepDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_link_tq_dd_v8_few() {
        let mut p = make_page("https://example.com");
        p.links = (0..3)
            .map(|i| crate::parser::ExtractedLink {
                href: format!("https://example.com/page{i}"),
                text: format!("L{i}"),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            })
            .collect();
        assert!(LinkTextQualityDeepDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_link_tq_dd_v8_empty() {
        let mut p = make_page("https://example.com");
        p.links = vec![crate::parser::ExtractedLink {
            href: "https://example.com/page".into(),
            text: "".into(),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: None,
        }];
        let f = LinkTextQualityDeepDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_img_alt_ddd_v8_ok() {
        let mut p = make_page("https://example.com");
        p.images = vec![crate::parser::ExtractedImage {
            src: "img.jpg".into(),
            alt: "Alt".into(),
            has_alt: true,
            width: None,
            height: None,
            is_lazy_loaded: false,
            aria_hidden: false,
        }];
        assert!(ImageAltTextDeepDeepDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_img_alt_ddd_v8_missing() {
        let mut p = make_page("https://example.com");
        p.images = vec![crate::parser::ExtractedImage {
            src: "img.jpg".into(),
            alt: "".into(),
            has_alt: false,
            width: None,
            height: None,
            is_lazy_loaded: false,
            aria_hidden: false,
        }];
        let f = ImageAltTextDeepDeepDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_focus_ddd_v8_ok() {
        let p = make_page("https://example.com");
        assert!(FocusManagementDeepDeepDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_focus_ddd_v8_positive() {
        let mut p = make_page("https://example.com");
        p.has_positive_tabindex = true;
        let f = FocusManagementDeepDeepDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_lang_dd_v8_missing() {
        let p = make_page("https://example.com");
        let f = LanguageAttributesDeepDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "LANGATTR-V2001-DEEP-DEEP");
    }
    #[test]
    fn test_lang_dd_v8_present() {
        let mut p = make_page("https://example.com");
        p.has_lang_attribute = true;
        assert!(LanguageAttributesDeepDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_colr_txt_deep_v8_ok() {
        let p = make_page("https://example.com");
        assert!(ColorContrastTextDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_colr_link_deep_v8_ok() {
        let p = make_page("https://example.com");
        assert!(ColorContrastLinkDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_anch_gen_deep_v8_ok() {
        let mut p = make_page("https://example.com");
        p.links = (0..3)
            .map(|i| crate::parser::ExtractedLink {
                href: format!("https://example.com/page{i}"),
                text: format!("Descriptive link {i}"),
                rel: vec![],
                is_external: false,
                aria_label: None,
                img_alt: None,
            })
            .collect();
        assert!(AnchorTextGenericDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_anch_gen_deep_v8_generic() {
        let mut p = make_page("https://example.com");
        p.links = vec![crate::parser::ExtractedLink {
            href: "https://example.com/page".into(),
            text: "click here".into(),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: None,
        }];
        let f = AnchorTextGenericDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_tbl_cap_deep_v8_no_tables() {
        let p = make_page("https://example.com");
        assert!(TableCaptionPresenceDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_tbl_cap_deep_v8_no_captions() {
        let mut p = make_page("https://example.com");
        p.tables_total = 1;
        p.tables_with_captions = 0;
        let f = TableCaptionPresenceDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_tbl_scop_deep_v8_no_tables() {
        let p = make_page("https://example.com");
        assert!(TableHeaderScopeDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_tbl_scop_deep_v8_no_headers() {
        let mut p = make_page("https://example.com");
        p.tables_total = 1;
        p.tables_with_headers = 0;
        let f = TableHeaderScopeDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_form_lab_deep_v8_ok() {
        let p = make_page("https://example.com");
        assert!(FormLabelAssociationDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_aria_req_deep_v8_no_roles() {
        let p = make_page("https://example.com");
        assert!(AriaRequiredAttributesDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_aria_req_deep_v8_roles_no_labels() {
        let mut p = make_page("https://example.com");
        p.aria_role_count = 3;
        p.aria_label_count = 0;
        let f = AriaRequiredAttributesDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_head_hier_ddd_v8_no_headings() {
        let p = make_page("https://example.com");
        let f = HeadingHierarchyDeepDeepDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "HHIER-V2001-DEEP-DEEP-DEEP");
    }
    #[test]
    fn test_head_hier_ddd_v8_no_h1() {
        let mut p = make_page("https://example.com");
        p.headings = vec![Heading {
            level: 2,
            text: "H2".into(),
            length: 2,
        }];
        let f = HeadingHierarchyDeepDeepDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(f.iter().any(|x| x.code == "HHIER-V2002-DEEP-DEEP-DEEP"));
    }
    #[test]
    fn test_head_hier_ddd_v8_multi_h1() {
        let mut p = make_page("https://example.com");
        p.headings = vec![
            Heading {
                level: 1,
                text: "H1a".into(),
                length: 3,
            },
            Heading {
                level: 1,
                text: "H1b".into(),
                length: 3,
            },
        ];
        let f = HeadingHierarchyDeepDeepDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(f.iter().any(|x| x.code == "HHIER-V2003-DEEP-DEEP-DEEP"));
    }
    #[test]
    fn test_head_hier_ddd_v8_good() {
        let mut p = make_page("https://example.com");
        p.headings = vec![Heading {
            level: 1,
            text: "H1".into(),
            length: 2,
        }];
        assert!(HeadingHierarchyDeepDeepDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_land_cinfo_deep_v8_missing() {
        let p = make_page("https://example.com");
        let f = LandmarkContentinfoDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "LANDCINFO-V2001");
    }
    #[test]
    fn test_land_cinfo_deep_v8_present() {
        let mut p = make_page("https://example.com");
        p.landmarks = vec!["Contentinfo".into()];
        assert!(LandmarkContentinfoDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_land_comp_deep_v8_missing() {
        let p = make_page("https://example.com");
        let f = LandmarkComplementaryDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "LANDCOMP-V2001");
    }
    #[test]
    fn test_land_comp_deep_v8_present() {
        let mut p = make_page("https://example.com");
        p.landmarks = vec!["Complementary".into()];
        assert!(LandmarkComplementaryDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }

    // ---------------------------------------------------------------------------
    // Regression tests: context-aware accessibility checks
    // ---------------------------------------------------------------------------

    fn make_link(
        text: &str,
        aria_label: Option<&str>,
        img_alt: Option<&str>,
    ) -> crate::parser::ExtractedLink {
        crate::parser::ExtractedLink {
            href: "https://example.com/page".into(),
            text: text.into(),
            rel: vec![],
            is_external: false,
            aria_label: aria_label.map(String::from),
            img_alt: img_alt.map(String::from),
        }
    }

    fn make_image(has_alt: bool, alt: &str, aria_hidden: bool) -> crate::parser::ExtractedImage {
        crate::parser::ExtractedImage {
            src: "img.jpg".into(),
            alt: alt.into(),
            has_alt,
            width: None,
            height: None,
            is_lazy_loaded: false,
            aria_hidden,
        }
    }

    #[test]
    fn test_focus_trap_ignores_role_in_comments_and_scripts() {
        let p = make_page("https://example.com");
        // `role="dialog"` appears only in a comment, an inline script, and
        // plain text — no actual dialog element exists.
        let body = r#"<html><body>
            <!-- <div role="dialog"></div> -->
            <script>var tpl = '<div role="dialog"></div>';</script>
            <p>The role="dialog" pattern opens modals.</p>
        </body></html>"#;
        assert!(FocusTrapMissingValidator::new()
            .analyze(&make_ctx(&p, Some(body)))
            .is_empty());
    }
    #[test]
    fn test_focus_trap_detects_real_dialog_element() {
        let p = make_page("https://example.com");
        let body = r#"<div role="dialog"><p>Content</p></div>"#;
        let f = FocusTrapMissingValidator::new().analyze(&make_ctx(&p, Some(body)));
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].code, "FOCTR001");
    }
    #[test]
    fn test_focus_trap_aria_modal_must_be_on_dialog() {
        let p = make_page("https://example.com");
        // aria-modal on an unrelated element does not trap dialog focus.
        let body = r#"<div role="dialog"></div><span aria-modal="true"></span>"#;
        let f = FocusTrapMissingValidator::new().analyze(&make_ctx(&p, Some(body)));
        assert!(!f.is_empty());
    }
    #[test]
    fn test_image_alt_missing_ignores_aria_hidden_images() {
        let mut p = make_page("https://example.com");
        p.images = vec![make_image(false, "", true)];
        assert!(ImageAltTextDeepAnalyzerV2::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
        assert!(ImageAltMissingDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
        assert!(ImageAltTextDeepDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
        assert!(ImageAltTextDeepDeepDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_image_alt_missing_still_flags_visible_images() {
        let mut p = make_page("https://example.com");
        p.images = vec![make_image(false, "", false)];
        let f = ImageAltMissingDeepValidator::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "IMGALTMISS-V6116");
    }
    #[test]
    fn test_image_alt_empty_ignores_aria_hidden_images() {
        let mut p = make_page("https://example.com");
        p.images = vec![make_image(true, "", true)];
        assert!(ImageAltEmptyDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_link_generic_text_with_aria_label_is_not_flagged() {
        let mut p = make_page("https://example.com");
        p.links = vec![make_link(
            "read more",
            Some("Read the full pricing guide"),
            None,
        )];
        assert!(LinkTextQualityAnalyzerV2::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
        assert!(LinkTextGenericDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
        assert!(LinkTextQualityDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_link_generic_text_without_aria_label_is_still_flagged() {
        let mut p = make_page("https://example.com");
        p.links = vec![make_link("read more", None, None)];
        let f = LinkTextQualityAnalyzerV2::new().analyze(&make_ctx(&p, None));
        assert!(!f.is_empty());
        assert_eq!(f[0].code, "LINKTQ-V2001");
    }
    #[test]
    fn test_link_empty_text_with_img_alt_is_not_flagged() {
        let mut p = make_page("https://example.com");
        p.links = vec![make_link("", None, Some("Home"))];
        assert!(LinkTextQualityAnalyzerV2::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
        assert!(LinkTextEmptyDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
    #[test]
    fn test_form_labels_aria_labelledby_counts_as_label() {
        let input = |aria_labelledby: Option<&str>| crate::parser::ExtractedInput {
            input_type: Some("text".into()),
            name: Some("email".into()),
            id: None,
            has_label: false,
            aria_label: None,
            aria_labelledby: aria_labelledby.map(String::from),
            aria_describedby: None,
            placeholder: None,
            required: false,
        };
        let form = crate::parser::ExtractedForm {
            action: None,
            method: "post".into(),
            input_count: 1,
            has_file_input: false,
            has_search_input: false,
            inputs: vec![input(Some("hint-id"))],
            has_fieldset: false,
            has_legend: false,
        };
        let mut p = make_page("https://example.com");
        p.forms = vec![form];
        assert!(FormLabelsDeepDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
        assert!(FormLabelsDeepDeepDeepValidator::new()
            .analyze(&make_ctx(&p, None))
            .is_empty());
    }
}
