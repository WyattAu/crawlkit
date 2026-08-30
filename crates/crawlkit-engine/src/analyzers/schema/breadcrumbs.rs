#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct BreadcrumbsValidator;

impl Default for BreadcrumbsValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl BreadcrumbsValidator {
    pub fn new() -> Self {
        Self
    }

    fn find_breadcrumb_schema(data: &serde_json::Value) -> Option<&serde_json::Value> {
        let schemas = data.get("@graph");
        if let Some(graph) = schemas.and_then(|g| g.as_array()) {
            for item in graph {
                if item.get("@type").and_then(|t| t.as_str()) == Some("BreadcrumbList") {
                    return Some(item);
                }
            }
        }
        if data.get("@type").and_then(|t| t.as_str()) == Some("BreadcrumbList") {
            return Some(data);
        }
        None
    }
}

impl Analyzer for BreadcrumbsValidator {
    fn name(&self) -> &str {
        "breadcrumbs-validator"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if let Some(breadcrumb) = Self::find_breadcrumb_schema(&sd.data) {
                // BREAD001: BreadcrumbList present but empty or single item
                if let Some(items) = breadcrumb.get("itemListElement").and_then(|i| i.as_array()) {
                    if items.len() <= 1 {
                        findings.push(Finding {
                            severity: Severity::Warning,
                            category: IssueCategory::Content,
                            code: "BREAD001".to_string(),
                            title: "BreadcrumbList has too few items".to_string(),
                            description: format!(
                                "BreadcrumbList contains only {} item(s). A complete breadcrumb \
                                 trail should have at least 2 items (home + current page).",
                                items.len()
                            ),
                            url: url.clone(),
                            recommendation: "Add all intermediate pages to the BreadcrumbList \
                                             schema to help search engines understand your site hierarchy."
                                .to_string(),
                        });
                    }

                    // BREAD002: Breadcrumb URLs don't match page hierarchy
                    if let Some(last_item) = items.last() {
                        if let Some(item_url) = last_item.get("item").and_then(|i| i.as_str()) {
                            let page_path = url
                                .trim_start_matches("https://")
                                .trim_start_matches("http://")
                                .trim_start_matches(|c: char| c.is_alphanumeric());
                            if !item_url.contains(page_path) && !page_path.is_empty() {
                                findings.push(Finding {
                                    severity: Severity::Info,
                                    category: IssueCategory::Content,
                                    code: "BREAD002".to_string(),
                                    title: "Breadcrumb URL doesn't match page URL".to_string(),
                                    description: format!(
                                        "The last breadcrumb item points to \"{}\" but the current \
                                         page URL is \"{}\".",
                                        item_url, url
                                    ),
                                    url: url.clone(),
                                    recommendation:
                                        "Ensure the last breadcrumb item's URL matches \
                                                     the current page URL."
                                            .to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }

        // BREAD003: No BreadcrumbList on deep pages (depth > 2)
        let path_segments: Vec<&str> = url
            .split('/')
            .filter(|s| !s.is_empty() && !s.contains(':'))
            .collect();
        if path_segments.len() > 2 {
            let has_breadcrumb =
                ctx.page.structured_data.iter().any(|sd| {
                    sd.data.get("@type").and_then(|t| t.as_str()) == Some("BreadcrumbList")
                });
            if !has_breadcrumb {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Content,
                    code: "BREAD003".to_string(),
                    title: "Deep page missing BreadcrumbList schema".to_string(),
                    description: format!(
                        "This page is {} levels deep but has no BreadcrumbList structured data. \
                         Breadcrumbs help search engines understand site hierarchy.",
                        path_segments.len()
                    ),
                    url: url.clone(),
                    recommendation: "Add a BreadcrumbList schema showing the full navigation \
                                     path from home to this page."
                        .to_string(),
                });
            }
        }

        findings
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::meta::MetaTags;
    use crate::parser::StructuredData;

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
        status: Option<u16>,
    ) -> AnalysisContext<'a> {
        AnalysisContext {
            page,
            body: None,
            status_code: status,
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
    fn test_breadcrumbs_empty_list() {
        let mut page = make_page("https://example.com/products");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("BreadcrumbList".to_string()),
            data: serde_json::json!({"@type": "BreadcrumbList", "itemListElement": []}),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(BreadcrumbsValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "BREAD001"));
    }

    #[test]
    fn test_breadcrumbs_single_item() {
        let mut page = make_page("https://example.com/products");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("BreadcrumbList".to_string()),
            data: serde_json::json!({"@type": "BreadcrumbList", "itemListElement": [{"@type": "ListItem", "position": 1, "item": "https://example.com"}]}),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(BreadcrumbsValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "BREAD001"));
    }

    #[test]
    fn test_breadcrumbs_valid() {
        let mut page = make_page("https://example.com/products/widget");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("BreadcrumbList".to_string()),
            data: serde_json::json!({"@type": "BreadcrumbList", "itemListElement": [
                {"@type": "ListItem", "position": 1, "item": "https://example.com"},
                {"@type": "ListItem", "position": 2, "item": "https://example.com/products"},
                {"@type": "ListItem", "position": 3, "item": "https://example.com/products/widget"}
            ]}),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = BreadcrumbsValidator::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "BREAD001"));
    }

    #[test]
    fn test_breadcrumbs_missing_on_deep_page() {
        let page = make_page("https://example.com/products/widget");
        let ctx = make_ctx(&page, Some(200));
        assert!(BreadcrumbsValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "BREAD003"));
    }

    #[test]
    fn test_breadcrumbs_no_bread003_on_shallow_page() {
        let page = make_page("https://example.com/products");
        let ctx = make_ctx(&page, Some(200));
        assert!(!BreadcrumbsValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "BREAD003"));
    }
}
