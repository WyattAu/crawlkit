//! Analyzer trait for plugins.

use crate::context::AnalysisContext;
use crate::finding::Finding;

/// Trait for page analyzers.
///
/// Implement this trait to create a custom SEO analyzer.
/// The analyzer receives page content and returns findings.
pub trait Analyzer {
    /// Get the analyzer name.
    fn name(&self) -> &str;

    /// Analyze the page content and return findings.
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Finding, Severity};

    struct MockAnalyzer;

    impl Analyzer for MockAnalyzer {
        fn name(&self) -> &str {
            "mock-analyzer"
        }

        fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
            let mut findings = Vec::new();
            if ctx.html.contains("<img") && !ctx.html.contains("alt=") {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: "accessibility".into(),
                    code: "A11Y01".into(),
                    title: "Missing alt text".into(),
                    description: "Image tag found without alt attribute".into(),
                    url: ctx.url.clone(),
                    recommendation: "Add alt attribute to image tags".into(),
                });
            }
            findings
        }
    }

    #[test]
    fn mock_analyzer_name() {
        let a = MockAnalyzer;
        assert_eq!(a.name(), "mock-analyzer");
    }

    #[test]
    fn mock_analyzer_no_findings() {
        let a = MockAnalyzer;
        let ctx = AnalysisContext {
            url: "https://example.com".into(),
            html: "<html><body>Text only</body></html>".into(),
            status_code: Some(200),
            headers: Vec::new(),
            response_time_ms: None,
        };
        let findings = a.analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn mock_analyzer_finds_issue() {
        let a = MockAnalyzer;
        let ctx = AnalysisContext {
            url: "https://example.com".into(),
            html: r#"<html><body><img src="photo.jpg"></body></html>"#.into(),
            status_code: Some(200),
            headers: Vec::new(),
            response_time_ms: None,
        };
        let findings = a.analyze(&ctx);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Warning);
        assert_eq!(findings[0].code, "A11Y01");
        assert_eq!(findings[0].url, "https://example.com");
    }

    #[test]
    fn mock_analyzer_ignores_img_with_alt() {
        let a = MockAnalyzer;
        let ctx = AnalysisContext {
            url: "https://example.com".into(),
            html: r#"<html><body><img src="photo.jpg" alt="A photo"></body></html>"#.into(),
            status_code: Some(200),
            headers: Vec::new(),
            response_time_ms: None,
        };
        let findings = a.analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn analyzer_trait_is_object_safe() {
        fn _assert_object_safe(_: &dyn Analyzer) {}
    }
}
