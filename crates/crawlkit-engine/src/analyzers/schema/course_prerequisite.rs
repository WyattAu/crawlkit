#![allow(clippy::default_constructed_unit_structs)]
use crate::analyzers::{AnalysisContext, Analyzer, Finding};
use crate::types::{IssueCategory, Severity};

pub struct CoursePrerequisiteValidator;

impl Default for CoursePrerequisiteValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl CoursePrerequisiteValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for CoursePrerequisiteValidator {
    fn name(&self) -> &str {
        "course-prerequisite"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Course") {
                continue;
            }
            let data = &sd.data;

            if data.get("prerequisite").is_none() && data.get("hasCoursePrerequisite").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "COURSEPRE001".to_string(),
                    title: "Course missing prerequisites".to_string(),
                    description: "A Course structured data block is missing the \"prerequisite\" \
                                  or \"hasCoursePrerequisite\" property. Specifying prerequisites \
                                  helps search engines understand course progression and \
                                  skill requirements."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"prerequisite\" or \"hasCoursePrerequisite\" with a \
                                     text description, URL, or Course reference."
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
    fn test_missing_prerequisite() {
        let mut page = make_page("https://example.com/course/1");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Course".to_string()),
            data: serde_json::json!({"@type": "Course", "name": "Advanced Rust"}),
        }];
        let findings = CoursePrerequisiteValidator::new().analyze(&make_ctx(&page));
        assert!(findings.iter().any(|f| f.code == "COURSEPRE001"));
    }

    #[test]
    fn test_with_prerequisite() {
        let mut page = make_page("https://example.com/course/1");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Course".to_string()),
            data: serde_json::json!({"@type": "Course", "name": "Advanced Rust", "prerequisite": "Basic Rust"}),
        }];
        let findings = CoursePrerequisiteValidator::new().analyze(&make_ctx(&page));
        assert!(findings.is_empty());
    }

    #[test]
    fn test_with_has_course_prerequisite() {
        let mut page = make_page("https://example.com/course/1");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Course".to_string()),
            data: serde_json::json!({"@type": "Course", "name": "Advanced Rust", "hasCoursePrerequisite": {"@type": "Course", "name": "Basic Rust"}}),
        }];
        let findings = CoursePrerequisiteValidator::new().analyze(&make_ctx(&page));
        assert!(findings.is_empty());
    }

    #[test]
    fn test_non_course_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product", "name": "Widget"}),
        }];
        let findings = CoursePrerequisiteValidator::new().analyze(&make_ctx(&page));
        assert!(findings.is_empty());
    }

    #[test]
    fn test_no_structured_data() {
        let page = make_page("https://example.com");
        let findings = CoursePrerequisiteValidator::new().analyze(&make_ctx(&page));
        assert!(findings.is_empty());
    }

    #[test]
    fn test_multiple_courses() {
        let mut page = make_page("https://example.com/courses");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Course".to_string()),
                data: serde_json::json!({"@type": "Course", "name": "Advanced Rust"}),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Course".to_string()),
                data: serde_json::json!({"@type": "Course", "name": "Basic Rust"}),
            },
        ];
        let findings = CoursePrerequisiteValidator::new().analyze(&make_ctx(&page));
        assert_eq!(findings.len(), 2);
    }

    #[test]
    fn test_one_course_with_one_without() {
        let mut page = make_page("https://example.com/courses");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Course".to_string()),
                data: serde_json::json!({"@type": "Course", "name": "Advanced Rust", "prerequisite": "Basic Rust"}),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Course".to_string()),
                data: serde_json::json!({"@type": "Course", "name": "Basic Rust"}),
            },
        ];
        let findings = CoursePrerequisiteValidator::new().analyze(&make_ctx(&page));
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn test_name() {
        assert_eq!(CoursePrerequisiteValidator::new().name(), "course-prerequisite");
    }
}
