#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct RecipeNutritionValidator;

impl RecipeNutritionValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RecipeNutritionValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for RecipeNutritionValidator {
    fn name(&self) -> &str {
        "recipe-nutrition"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Recipe") {
                continue;
            }
            let data = &sd.data;

            if data.get("nutrition").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "RNUT001".to_string(),
                    title: "Recipe missing nutrition information".to_string(),
                    description: "A Recipe structured data block is missing the \"nutrition\" \
                                  property. Nutrition info improves eligibility for rich results."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"nutrition\" with a NutritionInformation object \
                                     containing at least \"calories\"."
                        .to_string(),
                });
                continue;
            }

            if let Some(nutrition) = data.get("nutrition") {
                if nutrition.get("calories").is_none() {
                    findings.push(Finding {
                        severity: Severity::Info,
                        category: IssueCategory::Schema,
                        code: "RNUT002".to_string(),
                        title: "Recipe nutrition missing calories".to_string(),
                        description: "The \"nutrition\" property in Recipe schema is missing \
                                      \"calories\"."
                            .to_string(),
                        url: url.clone(),
                        recommendation: "Add \"calories\" with a string value (e.g., \"240 cal\")."
                            .to_string(),
                    });
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
fn test_rnut_missing_nutrition() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Recipe".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Recipe",
            "name": "Chocolate Cake"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = RecipeNutritionValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "RNUT001"));
}


    #[test]
fn test_rnut_missing_calories() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Recipe".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Recipe",
            "name": "Chocolate Cake",
            "nutrition": {"@type": "NutritionInformation"}
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = RecipeNutritionValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "RNUT002"));
}


    #[test]
fn test_rnut_valid() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Recipe".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Recipe",
            "name": "Chocolate Cake",
            "nutrition": {
                "@type": "NutritionInformation",
                "calories": "240 calories"
            }
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = RecipeNutritionValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_rnut_no_structured_data() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, None);
    let findings = RecipeNutritionValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_rnut_non_recipe_ignored() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Product".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Product",
            "name": "Widget"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = RecipeNutritionValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_rnut_empty_nutrition_object() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Recipe".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Recipe",
            "name": "Chocolate Cake",
            "nutrition": {}
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = RecipeNutritionValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "RNUT002"));
}


    #[test]
fn test_rnut_both_missing() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Recipe".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Recipe",
            "name": "Chocolate Cake"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = RecipeNutritionValidator::new().analyze(&ctx);
    assert_eq!(findings.len(), 1);
    assert!(findings.iter().any(|f| f.code == "RNUT001"));
}


    #[test]
fn test_rnut_nutrition_with_other_fields() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Recipe".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Recipe",
            "name": "Chocolate Cake",
            "nutrition": {
                "@type": "NutritionInformation",
                "fatContent": "10g",
                "proteinContent": "5g"
            }
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = RecipeNutritionValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "RNUT002"));
}


}
