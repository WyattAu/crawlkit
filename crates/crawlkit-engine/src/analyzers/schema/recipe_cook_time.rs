#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct RecipeCookTimeValidator;

impl Default for RecipeCookTimeValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl RecipeCookTimeValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for RecipeCookTimeValidator {
    fn name(&self) -> &str {
        "recipe-cook-time"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Recipe") {
                continue;
            }
            let data = &sd.data;

            if data.get("cookTime").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "RECIPECOOK001".to_string(),
                    title: "Recipe missing cookTime".to_string(),
                    description: "A Recipe structured data block is missing the \"cookTime\" \
                                  property. Cook time helps search engines display recipe rich \
                                  results with timing information."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"cookTime\" with an ISO 8601 duration value \
                                     (e.g., \"PT30M\")."
                        .to_string(),
                });
            }

            if data.get("prepTime").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "RECIPECOOK002".to_string(),
                    title: "Recipe missing prepTime".to_string(),
                    description: "A Recipe structured data block is missing the \"prepTime\" \
                                  property. Prep time helps search engines understand the \
                                  total effort required for the recipe."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"prepTime\" with an ISO 8601 duration value \
                                     (e.g., \"PT15M\")."
                        .to_string(),
                });
            }

            if data.get("totalTime").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "RECIPECOOK003".to_string(),
                    title: "Recipe missing totalTime".to_string(),
                    description: "A Recipe structured data block is missing the \"totalTime\" \
                                  property. Total time provides a complete time estimate \
                                  for the recipe."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"totalTime\" with an ISO 8601 duration value \
                                     (e.g., \"PT45M\")."
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
    use crate::parser::{ParsedPage, StructuredData};

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

    fn make_ctx<'a>(page: &'a ParsedPage) -> AnalysisContext<'a> {
        AnalysisContext {
            page,
            body: None,
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
    fn test_missing_cook_time() {
        let mut page = make_page("https://example.com/recipe/1");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Recipe".to_string()),
            data: serde_json::json!({"@type": "Recipe", "name": "Pasta"}),
        }];
        let findings = RecipeCookTimeValidator::new().analyze(&make_ctx(&page));
        assert!(findings.iter().any(|f| f.code == "RECIPECOOK001"));
    }

    #[test]
    fn test_missing_prep_time() {
        let mut page = make_page("https://example.com/recipe/1");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Recipe".to_string()),
            data: serde_json::json!({"@type": "Recipe", "name": "Pasta", "cookTime": "PT30M"}),
        }];
        let findings = RecipeCookTimeValidator::new().analyze(&make_ctx(&page));
        assert!(findings.iter().any(|f| f.code == "RECIPECOOK002"));
    }

    #[test]
    fn test_all_times_present() {
        let mut page = make_page("https://example.com/recipe/1");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Recipe".to_string()),
            data: serde_json::json!({"@type": "Recipe", "name": "Pasta", "cookTime": "PT30M", "prepTime": "PT15M", "totalTime": "PT45M"}),
        }];
        let findings = RecipeCookTimeValidator::new().analyze(&make_ctx(&page));
        assert!(findings.is_empty());
    }

    #[test]
    fn test_non_recipe_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product", "name": "Widget"}),
        }];
        let findings = RecipeCookTimeValidator::new().analyze(&make_ctx(&page));
        assert!(findings.is_empty());
    }

    #[test]
    fn test_no_structured_data() {
        let page = make_page("https://example.com");
        let findings = RecipeCookTimeValidator::new().analyze(&make_ctx(&page));
        assert!(findings.is_empty());
    }

    #[test]
    fn test_all_three_times_missing() {
        let mut page = make_page("https://example.com/recipe/1");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Recipe".to_string()),
            data: serde_json::json!({"@type": "Recipe", "name": "Pasta"}),
        }];
        let findings = RecipeCookTimeValidator::new().analyze(&make_ctx(&page));
        assert_eq!(findings.len(), 3);
    }

    #[test]
    fn test_multiple_recipes() {
        let mut page = make_page("https://example.com/recipes");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Recipe".to_string()),
                data: serde_json::json!({"@type": "Recipe", "name": "Pasta", "cookTime": "PT30M", "prepTime": "PT15M", "totalTime": "PT45M"}),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Recipe".to_string()),
                data: serde_json::json!({"@type": "Recipe", "name": "Soup"}),
            },
        ];
        let findings = RecipeCookTimeValidator::new().analyze(&make_ctx(&page));
        assert_eq!(findings.len(), 3);
    }

    #[test]
    fn test_name() {
        assert_eq!(RecipeCookTimeValidator::new().name(), "recipe-cook-time");
    }
}
