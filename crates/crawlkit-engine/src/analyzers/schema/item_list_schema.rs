#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct ItemListSchemaValidator;

impl Default for ItemListSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ItemListSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for ItemListSchemaValidator {
    fn name(&self) -> &str {
        "itemlist-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("ItemList") {
                continue;
            }
            let data = &sd.data;

            match data.get("itemListElement") {
                None | Some(serde_json::Value::Null) => {
                    findings.push(Finding {
                        severity: Severity::Error,
                        category: IssueCategory::Schema,
                        code: "ITEMLIST001".to_string(),
                        title: "ItemList schema missing itemListElement".to_string(),
                        description: "An ItemList structured data block is missing the required \
                                      \"itemListElement\" property."
                            .to_string(),
                        url: url.clone(),
                        recommendation: "Add \"itemListElement\" with an array of ListItem objects."
                            .to_string(),
                    });
                }
                Some(val) => {
                    match val.as_array() {
                        None => {
                            // Not an array at all (e.g. string, number)
                            findings.push(Finding {
                                severity: Severity::Error,
                                category: IssueCategory::Schema,
                                code: "ITEMLIST001".to_string(),
                                title: "ItemList schema missing itemListElement".to_string(),
                                description: "An ItemList structured data block has an \
                                              \"itemListElement\" that is not an array."
                                    .to_string(),
                                url: url.clone(),
                                recommendation:
                                    "Change itemListElement to an array of ListItem objects."
                                        .to_string(),
                            });
                        }
                        Some(arr) if arr.is_empty() => {
                            findings.push(Finding {
                                severity: Severity::Error,
                                category: IssueCategory::Schema,
                                code: "ITEMLIST002".to_string(),
                                title: "ItemList schema itemListElement is empty".to_string(),
                                description: "An ItemList structured data block has an empty \
                                              \"itemListElement\" array."
                                    .to_string(),
                                url: url.clone(),
                                recommendation:
                                    "Populate the itemListElement array with ListItem objects."
                                        .to_string(),
                            });
                        }
                        _ => {}
                    }
                }
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
    fn test_itemlist_missing_item_list_element() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("ItemList".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "ItemList"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = ItemListSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ITEMLIST001"));
    }

    #[test]
    fn test_itemlist_item_list_element_empty() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("ItemList".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "ItemList",
                "itemListElement": []
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = ItemListSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ITEMLIST002"));
    }

    #[test]
    fn test_itemlist_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("ItemList".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "ItemList",
                "itemListElement": [
                    {"@type": "ListItem", "position": 1, "name": "Item 1"}
                ]
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = ItemListSchemaValidator::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "ITEMLIST001"));
        assert!(!findings.iter().any(|f| f.code == "ITEMLIST002"));
    }

    #[test]
    fn test_itemlist_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, None);
        let findings = ItemListSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_itemlist_non_itemlist_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("BreadcrumbList".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "BreadcrumbList"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = ItemListSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_itemlist_both_issues() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("ItemList".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "ItemList"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = ItemListSchemaValidator::new().analyze(&ctx);
        // Missing itemListElement entirely fires only ITEMLIST001
        assert!(findings.iter().any(|f| f.code == "ITEMLIST001"));
        assert!(!findings.iter().any(|f| f.code == "ITEMLIST002"));
    }

    #[test]
    fn test_itemlist_multiple_itemlists() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("ItemList".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "ItemList",
                    "itemListElement": [{"@type": "ListItem", "position": 1}]
                }),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("ItemList".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "ItemList"
                }),
            },
        ];
        let ctx = make_ctx(&page, None);
        let findings = ItemListSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ITEMLIST001"));
    }

    #[test]
    fn test_itemlist_null_item_list_element() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("ItemList".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "ItemList",
                "itemListElement": null
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = ItemListSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ITEMLIST001"));
    }

    #[test]
    fn test_itemlist_item_list_element_string_instead_of_array() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("ItemList".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "ItemList",
                "itemListElement": "not-an-array"
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = ItemListSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ITEMLIST001"));
    }

    #[test]
    fn test_itemlist_single_item_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("ItemList".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "ItemList",
                "itemListElement": [
                    {"@type": "ListItem", "position": 1, "name": "Only Item"}
                ]
            }),
        }];
        let ctx = make_ctx(&page, None);
        let findings = ItemListSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }
}
