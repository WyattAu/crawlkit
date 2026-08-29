use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct JobPostingSalaryValidator;

impl JobPostingSalaryValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for JobPostingSalaryValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for JobPostingSalaryValidator {
    fn name(&self) -> &str {
        "job-posting-salary"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("JobPosting") {
                continue;
            }
            let data = &sd.data;

            if data.get("baseSalary").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "JSAL001".to_string(),
                    title: "JobPosting missing baseSalary".to_string(),
                    description: "A JobPosting structured data block is missing the \"baseSalary\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"baseSalary\" with a MonetaryAmount or QuantitativeValue \
                                     to show salary information in search results."
                        .to_string(),
                });
            }

            if data.get("employmentType").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "JSAL002".to_string(),
                    title: "JobPosting missing employmentType".to_string(),
                    description: "A JobPosting structured data block is missing the \
                                  \"employmentType\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"employmentType\" with one of: FULL_TIME, PART_TIME, \
                                     CONTRACTOR, TEMPORARY, INTERN, VOLUNTEER, PER_DIEM, OTHER."
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
fn test_jsal_missing_base_salary() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("JobPosting".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "JobPosting",
            "title": "Software Engineer"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = JobPostingSalaryValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "JSAL001"));
}


    #[test]
fn test_jsal_missing_employment_type() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("JobPosting".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "JobPosting",
            "title": "Software Engineer",
            "baseSalary": {
                "@type": "MonetaryAmount",
                "currency": "USD",
                "value": {"@type": "QuantitativeValue", "value": 100000}
            }
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = JobPostingSalaryValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "JSAL002"));
}


    #[test]
fn test_jsal_valid() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("JobPosting".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "JobPosting",
            "title": "Software Engineer",
            "baseSalary": {
                "@type": "MonetaryAmount",
                "currency": "USD",
                "value": {"@type": "QuantitativeValue", "value": 100000}
            },
            "employmentType": "FULL_TIME"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = JobPostingSalaryValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_jsal_no_structured_data() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, None);
    let findings = JobPostingSalaryValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_jsal_non_job_posting_ignored() {
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
    let findings = JobPostingSalaryValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}


    #[test]
fn test_jsal_both_missing() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("JobPosting".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "JobPosting",
            "title": "Software Engineer"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = JobPostingSalaryValidator::new().analyze(&ctx);
    assert_eq!(findings.len(), 2);
}


    #[test]
fn test_jsal_with_salary_no_employment_type() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("JobPosting".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "JobPosting",
            "title": "Software Engineer",
            "baseSalary": {
                "@type": "MonetaryAmount",
                "currency": "USD",
                "value": {"@type": "QuantitativeValue", "value": 100000}
            }
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = JobPostingSalaryValidator::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "JSAL001"));
    assert!(findings.iter().any(|f| f.code == "JSAL002"));
}


    #[test]
fn test_jsal_with_employment_type_no_salary() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("JobPosting".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "JobPosting",
            "title": "Software Engineer",
            "employmentType": "FULL_TIME"
        }),
    }];
    let ctx = make_ctx(&page, None);
    let findings = JobPostingSalaryValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "JSAL001"));
    assert!(!findings.iter().any(|f| f.code == "JSAL002"));
}


}
