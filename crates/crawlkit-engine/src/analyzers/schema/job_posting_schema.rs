use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct JobPostingSchemaValidator;

impl Default for JobPostingSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl JobPostingSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for JobPostingSchemaValidator {
    fn name(&self) -> &str {
        "jobposting-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            let schema_type = sd.data.get("@type").and_then(|t| t.as_str()).unwrap_or("");
            if schema_type != "JobPosting" {
                continue;
            }
            let data = &sd.data;

            // JOB001: Missing title (job title)
            if data.get("title").is_none() {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Schema,
                    code: "JOB001".to_string(),
                    title: "JobPosting schema missing title".to_string(),
                    description: "A JobPosting structured data block is missing the required \
                                  \"title\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"title\" with the job position title.".to_string(),
                });
            }

            // JOB002: Missing datePosted
            if data.get("datePosted").is_none() {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Schema,
                    code: "JOB002".to_string(),
                    title: "JobPosting schema missing datePosted".to_string(),
                    description: "A JobPosting structured data block is missing the required \
                                  \"datePosted\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"datePosted\" with an ISO 8601 date."
                        .to_string(),
                });
            }

            // JOB003: Missing validThrough
            if data.get("validThrough").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "JOB003".to_string(),
                    title: "JobPosting schema missing validThrough".to_string(),
                    description: "A JobPosting structured data block is missing the \"validThrough\" \
                                  property. This tells search engines when the job posting expires."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"validThrough\" with an ISO 8601 date/time when the \
                                     posting expires."
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
fn test_job_missing_title() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("JobPosting".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "JobPosting"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = JobPostingSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "JOB001"));
}


    #[test]
fn test_job_missing_date_posted() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("JobPosting".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "JobPosting",
            "title": "Engineer"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = JobPostingSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "JOB002"));
}


    #[test]
fn test_job_missing_valid_through() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("JobPosting".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "JobPosting",
            "title": "Engineer",
            "datePosted": "2024-01-01"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = JobPostingSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "JOB003"));
}


    #[test]
fn test_job_all_present() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("JobPosting".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "JobPosting",
            "title": "Engineer",
            "datePosted": "2024-01-01",
            "validThrough": "2024-12-31"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = JobPostingSchemaValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_job_missing_all() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("JobPosting".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "JobPosting"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = JobPostingSchemaValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "JOB001"));
    assert!(findings.iter().any(|f| f.code == "JOB002"));
    assert!(findings.iter().any(|f| f.code == "JOB003"));
}


    #[test]
fn test_job_non_job_type_ignored() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Product".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Product"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = JobPostingSchemaValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_job_no_schema_no_findings() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, None);
    let findings = JobPostingSchemaValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


}
