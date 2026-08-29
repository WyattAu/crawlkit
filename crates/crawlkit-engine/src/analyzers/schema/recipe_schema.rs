use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct RecipeSchemaValidator;

impl Default for RecipeSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl RecipeSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for RecipeSchemaValidator {
    fn name(&self) -> &str {
        "recipe-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            let schema_type = sd.data.get("@type").and_then(|t| t.as_str()).unwrap_or("");
            if schema_type != "Recipe" {
                continue;
            }
            let data = &sd.data;

            // RECIPE001: Missing name
            if data.get("name").is_none() {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Schema,
                    code: "RECIPE001".to_string(),
                    title: "Recipe schema missing name".to_string(),
                    description: "A Recipe structured data block is missing the required \
                                  \"name\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with the recipe title.".to_string(),
                });
            }

            // RECIPE002: Missing cookTime
            if data.get("cookTime").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "RECIPE002".to_string(),
                    title: "Recipe schema missing cookTime".to_string(),
                    description: "A Recipe structured data block is missing the \"cookTime\" \
                                  property. cookTime helps search engines display cooking \
                                  duration in rich results."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"cookTime\" with an ISO 8601 duration (e.g., PT30M)."
                        .to_string(),
                });
            }

            // RECIPE003: Missing recipeIngredient
            if data.get("recipeIngredient").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "RECIPE003".to_string(),
                    title: "Recipe schema missing recipeIngredient".to_string(),
                    description: "A Recipe structured data block is missing the \
                                  \"recipeIngredient\" property. Ingredients are required for \
                                  Recipe rich results."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"recipeIngredient\" with an array of ingredient strings."
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

    fn make_ctx_with_body<'a>(
        page: &'a crate::parser::ParsedPage,
        status: Option<u16>,
        body: &'a str,
    ) -> AnalysisContext<'a> {
        AnalysisContext {
            page,
            body: Some(body),
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
fn test_recipe_missing_name() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Recipe".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Recipe"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = RecipeSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "RECIPE001"));
}


    #[test]
fn test_recipe_missing_cook_time() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Recipe".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Recipe",
            "name": "Cake"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = RecipeSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "RECIPE002"));
}


    #[test]
fn test_recipe_missing_ingredients() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Recipe".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Recipe",
            "name": "Cake",
            "cookTime": "PT30M"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = RecipeSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "RECIPE003"));
}


    #[test]
fn test_recipe_all_present() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Recipe".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Recipe",
            "name": "Cake",
            "cookTime": "PT30M",
            "recipeIngredient": ["flour", "sugar"]
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = RecipeSchemaValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_recipe_missing_all() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Recipe".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Recipe"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = RecipeSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "RECIPE001"));
    assert!(findings.iter().any(|f| f.code == "RECIPE002"));
    assert!(findings.iter().any(|f| f.code == "RECIPE003"));
}


    #[test]
fn test_recipe_no_schema_no_findings() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, None);
    let findings = RecipeSchemaValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


}
