//! Finding types for plugins.

use serde::{Deserialize, Serialize};

/// Severity of a finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Critical,
    Error,
    Warning,
    Info,
}

impl Severity {
    /// Convert to string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Critical => "critical",
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        }
    }
}

/// A single finding from an analyzer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    /// Issue severity.
    pub severity: Severity,
    /// Issue category.
    pub category: String,
    /// Machine-readable issue code.
    pub code: String,
    /// Short human-readable title.
    pub title: String,
    /// Detailed description.
    pub description: String,
    /// URL where the issue was found.
    pub url: String,
    /// Recommendation for fixing.
    pub recommendation: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finding_construction_and_field_access() {
        let f = Finding {
            severity: Severity::Error,
            category: "seo".into(),
            code: "SEO001".into(),
            title: "Missing title".into(),
            description: "Page has no title tag".into(),
            url: "https://example.com".into(),
            recommendation: "Add a title tag".into(),
        };

        assert_eq!(f.severity, Severity::Error);
        assert_eq!(f.category, "seo");
        assert_eq!(f.code, "SEO001");
        assert_eq!(f.title, "Missing title");
        assert_eq!(f.description, "Page has no title tag");
        assert_eq!(f.url, "https://example.com");
        assert_eq!(f.recommendation, "Add a title tag");
    }

    #[test]
    fn finding_clone() {
        let f = Finding {
            severity: Severity::Warning,
            category: "perf".into(),
            code: "PERF01".into(),
            title: "Slow".into(),
            description: "Slow page".into(),
            url: "https://a.com".into(),
            recommendation: "Optimize".into(),
        };
        let cloned = f.clone();
        assert_eq!(f.severity, cloned.severity);
        assert_eq!(f.title, cloned.title);
    }

    #[test]
    fn finding_debug() {
        let f = Finding {
            severity: Severity::Info,
            category: "test".into(),
            code: "T1".into(),
            title: "t".into(),
            description: "d".into(),
            url: "u".into(),
            recommendation: "r".into(),
        };
        let dbg = format!("{:?}", f);
        assert!(dbg.contains("Finding"));
        assert!(dbg.contains("Info"));
    }

    #[test]
    fn severity_as_str_all_variants() {
        assert_eq!(Severity::Critical.as_str(), "critical");
        assert_eq!(Severity::Error.as_str(), "error");
        assert_eq!(Severity::Warning.as_str(), "warning");
        assert_eq!(Severity::Info.as_str(), "info");
    }

    #[test]
    fn severity_equality() {
        assert_eq!(Severity::Critical, Severity::Critical);
        assert_ne!(Severity::Critical, Severity::Error);
        assert_ne!(Severity::Warning, Severity::Info);
    }

    #[test]
    fn severity_clone_and_copy() {
        let s = Severity::Error;
        let s2 = s;
        assert_eq!(s, s2);
    }

    #[test]
    fn severity_debug() {
        let dbg = format!("{:?}", Severity::Critical);
        assert_eq!(dbg, "Critical");
    }

    #[test]
    fn finding_serialization_roundtrip() {
        let f = Finding {
            severity: Severity::Critical,
            category: "security".into(),
            code: "SEC001".into(),
            title: "XSS vulnerability".into(),
            description: "Unsafe innerHTML usage".into(),
            url: "https://example.com/page".into(),
            recommendation: "Sanitize input".into(),
        };

        let json = serde_json::to_string(&f).expect("serialize");
        let deserialized: Finding = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(f.severity, deserialized.severity);
        assert_eq!(f.category, deserialized.category);
        assert_eq!(f.code, deserialized.code);
        assert_eq!(f.title, deserialized.title);
        assert_eq!(f.description, deserialized.description);
        assert_eq!(f.url, deserialized.url);
        assert_eq!(f.recommendation, deserialized.recommendation);
    }

    #[test]
    fn finding_deserialize_from_json() {
        let json = r#"{
            "severity": "Warning",
            "category": "accessibility",
            "code": "A11Y01",
            "title": "Missing alt text",
            "description": "Image without alt attribute",
            "url": "https://example.com",
            "recommendation": "Add alt text to images"
        }"#;
        let f: Finding = serde_json::from_str(json).expect("deserialize from json");
        assert_eq!(f.severity, Severity::Warning);
        assert_eq!(f.code, "A11Y01");
    }

    #[test]
    fn finding_json_output_is_object() {
        let f = Finding {
            severity: Severity::Info,
            category: "c".into(),
            code: "C".into(),
            title: "t".into(),
            description: "d".into(),
            url: "u".into(),
            recommendation: "r".into(),
        };
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&f).unwrap()).unwrap();
        assert!(json.is_object());
        assert_eq!(json["severity"], "Info");
        assert_eq!(json["code"], "C");
    }

    #[test]
    fn severity_serialization_roundtrip() {
        let severities = vec![
            Severity::Critical,
            Severity::Error,
            Severity::Warning,
            Severity::Info,
        ];
        for s in severities {
            let json = serde_json::to_string(&s).unwrap();
            let deserialized: Severity = serde_json::from_str(&json).unwrap();
            assert_eq!(s, deserialized);
        }
    }
}
