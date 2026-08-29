#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct JobPostingValidThroughValidator;

impl Default for JobPostingValidThroughValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl JobPostingValidThroughValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for JobPostingValidThroughValidator {
    fn name(&self) -> &str {
        "job-posting-valid-through"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("JobPosting") {
                continue;
            }
            let data = &sd.data;

            if data.get("validThrough").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "JOBVALID001".to_string(),
                    title: "JobPosting missing validThrough".to_string(),
                    description: "A JobPosting structured data block is missing the \
                                  \"validThrough\" property. Without validThrough, search \
                                  engines cannot determine when the job listing expires."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"validThrough\" with an ISO 8601 date-time value \
                                     (e.g., \"2024-12-31T23:59:59Z\") to indicate when the \
                                     posting expires."
                        .to_string(),
                });
            }

            if let Some(valid_through) = data.get("validThrough") {
                if let Some(vt_str) = valid_through.as_str() {
                    if vt_str.is_empty() {
                        findings.push(Finding {
                            severity: Severity::Warning,
                            category: IssueCategory::Schema,
                            code: "JOBVALID001".to_string(),
                            title: "JobPosting empty validThrough".to_string(),
                            description: "The \"validThrough\" property in JobPosting schema \
                                          is empty. It should contain a valid ISO 8601 date-time."
                                .to_string(),
                            url: url.clone(),
                            recommendation: "Add a valid ISO 8601 date-time value to \
                                             validThrough."
                                .to_string(),
                        });
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
    fn test_missing_valid_through() {
        let mut page = make_page("https://example.com/jobs/1");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("JobPosting".to_string()),
            data: serde_json::json!({"@type": "JobPosting", "title": "Engineer", "datePosted": "2024-01-01"}),
        }];
        let findings = JobPostingValidThroughValidator::new().analyze(&make_ctx(&page));
        assert!(findings.iter().any(|f| f.code == "JOBVALID001"));
    }

    #[test]
    fn test_valid_valid_through() {
        let mut page = make_page("https://example.com/jobs/1");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("JobPosting".to_string()),
            data: serde_json::json!({"@type": "JobPosting", "title": "Engineer", "datePosted": "2024-01-01", "validThrough": "2024-12-31T23:59:59Z"}),
        }];
        let findings = JobPostingValidThroughValidator::new().analyze(&make_ctx(&page));
        assert!(findings.is_empty());
    }

    #[test]
    fn test_empty_valid_through() {
        let mut page = make_page("https://example.com/jobs/1");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("JobPosting".to_string()),
            data: serde_json::json!({"@type": "JobPosting", "title": "Engineer", "datePosted": "2024-01-01", "validThrough": ""}),
        }];
        let findings = JobPostingValidThroughValidator::new().analyze(&make_ctx(&page));
        assert!(findings.iter().any(|f| f.code == "JOBVALID001"));
    }

    #[test]
    fn test_non_job_posting_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product", "name": "Widget"}),
        }];
        let findings = JobPostingValidThroughValidator::new().analyze(&make_ctx(&page));
        assert!(findings.is_empty());
    }

    #[test]
    fn test_no_structured_data() {
        let page = make_page("https://example.com");
        let findings = JobPostingValidThroughValidator::new().analyze(&make_ctx(&page));
        assert!(findings.is_empty());
    }

    #[test]
    fn test_multiple_job_postings_each_missing() {
        let mut page = make_page("https://example.com/jobs");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("JobPosting".to_string()),
                data: serde_json::json!({"@type": "JobPosting", "title": "Engineer"}),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("JobPosting".to_string()),
                data: serde_json::json!({"@type": "JobPosting", "title": "Designer"}),
            },
        ];
        let findings = JobPostingValidThroughValidator::new().analyze(&make_ctx(&page));
        let valid_through_findings: Vec<_> = findings.iter().filter(|f| f.code == "JOBVALID001").collect();
        assert_eq!(valid_through_findings.len(), 2);
    }

    #[test]
    fn test_valid_through_with_date_posted() {
        let mut page = make_page("https://example.com/jobs/1");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("JobPosting".to_string()),
            data: serde_json::json!({"@type": "JobPosting", "title": "Engineer", "datePosted": "2024-01-01", "validThrough": "2024-06-30"}),
        }];
        let findings = JobPostingValidThroughValidator::new().analyze(&make_ctx(&page));
        assert!(findings.is_empty());
    }

    #[test]
    fn test_mixed_valid_and_invalid() {
        let mut page = make_page("https://example.com/jobs");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("JobPosting".to_string()),
                data: serde_json::json!({"@type": "JobPosting", "title": "Engineer", "validThrough": "2024-12-31"}),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("JobPosting".to_string()),
                data: serde_json::json!({"@type": "JobPosting", "title": "Designer"}),
            },
        ];
        let findings = JobPostingValidThroughValidator::new().analyze(&make_ctx(&page));
        assert_eq!(findings.len(), 1);
        assert!(findings[0].code == "JOBVALID001");
    }

    #[test]
    fn test_name() {
        assert_eq!(JobPostingValidThroughValidator::new().name(), "job-posting-valid-through");
    }
}
