#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct DatasetSchemaValidator;

impl Default for DatasetSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl DatasetSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for DatasetSchemaValidator {
    fn name(&self) -> &str {
        "dataset-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Dataset") {
                continue;
            }
            let data = &sd.data;

            // DATA001: Missing name
            if data.get("name").is_none() {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Schema,
                    code: "DATA001".to_string(),
                    title: "Dataset schema missing name".to_string(),
                    description: "A Dataset structured data block is missing the required \
                                  \"name\" property. The name identifies the dataset."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with a descriptive title for the dataset."
                        .to_string(),
                });
            }

            // DATA002: Missing distribution
            if data.get("distribution").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "DATA002".to_string(),
                    title: "Dataset schema missing distribution".to_string(),
                    description: "A Dataset structured data block is missing the \"distribution\" \
                                  property. Distribution specifies how to access the dataset."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"distribution\" with a DataDownload object specifying \
                                     the download URL and format."
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
fn test_dataset_missing_name() {
    let mut page = make_page("https://example.com/data");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Dataset".to_string()),
        data: serde_json::json!({
            "@type": "Dataset",
            "description": "A dataset about weather"
        }),
    }];
    let ctx = make_ctx(&page, Some(200));
    assert!(DatasetSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "DATA001"));
}


    #[test]
fn test_dataset_missing_description() {
    let mut page = make_page("https://example.com/data");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Dataset".to_string()),
        data: serde_json::json!({
            "@type": "Dataset",
            "name": "Weather Data"
        }),
    }];
    let ctx = make_ctx(&page, Some(200));
    assert!(DatasetSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "DATA002"));
}


    #[test]
fn test_dataset_missing_distribution() {
    let mut page = make_page("https://example.com/data");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Dataset".to_string()),
        data: serde_json::json!({
            "@type": "Dataset",
            "name": "Weather Data",
            "description": "Daily weather data"
        }),
    }];
    let ctx = make_ctx(&page, Some(200));
    assert!(DatasetSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "DATA002"));
}


    #[test]
fn test_dataset_valid() {
    let mut page = make_page("https://example.com/data");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Dataset".to_string()),
        data: serde_json::json!({
            "@type": "Dataset",
            "name": "Weather Data",
            "description": "Daily weather data",
            "distribution": {"@type": "DataDownload", "contentUrl": "https://example.com/data.csv"}
        }),
    }];
    let ctx = make_ctx(&page, Some(200));
    assert!(DatasetSchemaValidator::new().analyze(&ctx).is_empty());
}


    #[test]
fn test_dataset_non_dataset_type_ignored() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Article".to_string()),
        data: serde_json::json!({"@type": "Article", "headline": "News"}),
    }];
    let ctx = make_ctx(&page, Some(200));
    assert!(DatasetSchemaValidator::new().analyze(&ctx).is_empty());
}


    #[test]
fn test_dataset_no_structured_data() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200));
    assert!(DatasetSchemaValidator::new().analyze(&ctx).is_empty());
}


    #[test]
fn test_dataset_missing_all_fields() {
    let mut page = make_page("https://example.com/data");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Dataset".to_string()),
        data: serde_json::json!({"@type": "Dataset"}),
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = DatasetSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "DATA001"));
    assert!(findings.iter().any(|f| f.code == "DATA002"));
}


    #[test]
fn test_dataset_multiple_datasets() {
    let mut page = make_page("https://example.com/data");
    page.structured_data = vec![
        StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Dataset".to_string()),
            data: serde_json::json!({"@type": "Dataset"}),
        },
        StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Dataset".to_string()),
            data: serde_json::json!({"@type": "Dataset"}),
        },
    ];
    let ctx = make_ctx(&page, Some(200));
    let findings = DatasetSchemaValidator::new().analyze(&ctx);
    let data001_count = findings.iter().filter(|f| f.code == "DATA001").count();
    assert_eq!(data001_count, 2);
}


}
