//! Shared types for crawlkit engine and plugin SDK.
//!
//! This crate defines the canonical [`Severity`], [`IssueCategory`], and
//! [`Finding`] types used by both the host engine and WASM plugin guests.
//! Keeping them in a single crate eliminates the three-way type duplication
//! that previously existed between engine, SDK, and JSON mirror types.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use serde::{Deserialize, Serialize};

/// Severity level for an issue/finding.
///
/// Used by analyzers to classify the importance of detected issues.
/// Stored in the database as a lowercase string for querying.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Critical issue requiring immediate attention.
    Critical,
    /// Error that should be fixed.
    Error,
    /// Warning suggesting improvement.
    Warning,
    /// Informational note.
    Info,
}

impl Severity {
    /// Convert to the string representation used in the database.
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Critical => "critical",
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        }
    }

    /// Parse from the database string representation.
    pub fn parse_severity(s: &str) -> Option<Self> {
        match s {
            "critical" => Some(Severity::Critical),
            "error" => Some(Severity::Error),
            "warning" => Some(Severity::Warning),
            "info" => Some(Severity::Info),
            _ => None,
        }
    }
}

/// Category of an analysis finding.
///
/// Groups related issues for filtering and reporting. Stored in the
/// database as a lowercase string. Custom categories use a `custom:` prefix.
///
/// Serializes to the same lowercase string form used by the database and
/// every CLI/API surface (`"http"`, `"seo"`, `"custom:my-plugin"`), so the
/// wire format matches `docs/schema/findings.schema.json`. Deserialization
/// additionally accepts the pre-5.2 map form (`{"custom": "name"}`) from
/// older plugin payloads.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum IssueCategory {
    /// HTTP-related issues (status codes, redirects, headers).
    Http,
    /// SEO issues (title, meta, canonical, robots).
    Seo,
    /// Content issues (word count, thin content, readability).
    Content,
    /// Link issues (broken links, redirect links, nofollow).
    Links,
    /// Image issues (missing alt, oversized, format).
    Images,
    /// Structured data issues (JSON-LD, microdata).
    Schema,
    /// Security issues (mixed content, headers).
    Security,
    /// Performance issues (page size, load time).
    Performance,
    /// Mobile-friendliness issues.
    Mobile,
    /// Accessibility issues (alt text, ARIA, contrast).
    Accessibility,
    /// Social metadata issues (Open Graph, Twitter Cards).
    Social,
    /// Custom analyzer issue.
    Custom(String),
}

impl IssueCategory {
    /// Convert to the string representation used in the database.
    pub fn as_str(&self) -> String {
        match self {
            IssueCategory::Http => "http".to_string(),
            IssueCategory::Seo => "seo".to_string(),
            IssueCategory::Content => "content".to_string(),
            IssueCategory::Links => "links".to_string(),
            IssueCategory::Images => "images".to_string(),
            IssueCategory::Schema => "schema".to_string(),
            IssueCategory::Security => "security".to_string(),
            IssueCategory::Performance => "performance".to_string(),
            IssueCategory::Mobile => "mobile".to_string(),
            IssueCategory::Accessibility => "accessibility".to_string(),
            IssueCategory::Social => "social".to_string(),
            IssueCategory::Custom(name) => format!("custom:{name}"),
        }
    }

    /// Parse from the database string representation.
    pub fn parse_category(s: &str) -> Self {
        match s {
            "http" => IssueCategory::Http,
            "seo" => IssueCategory::Seo,
            "content" => IssueCategory::Content,
            "links" => IssueCategory::Links,
            "images" => IssueCategory::Images,
            "schema" => IssueCategory::Schema,
            "security" => IssueCategory::Security,
            "performance" => IssueCategory::Performance,
            "mobile" => IssueCategory::Mobile,
            "accessibility" => IssueCategory::Accessibility,
            "social" => IssueCategory::Social,
            other => {
                let name = other.strip_prefix("custom:").unwrap_or(other);
                IssueCategory::Custom(name.to_string())
            }
        }
    }
}

impl From<&str> for IssueCategory {
    /// Parse a category from a string. Handles both fixed variants and `custom:` prefixed names.
    fn from(s: &str) -> Self {
        Self::parse_category(s)
    }
}

impl Serialize for IssueCategory {
    /// Canonical wire form: the lowercase category string, `custom:` prefixed
    /// for plugin categories. Matches `as_str()` exactly.
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.as_str())
    }
}

impl<'de> Deserialize<'de> for IssueCategory {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        /// Untagged input form: canonical string, or the pre-5.2 derived map
        /// form (`{"custom": "name"}`).
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Str(String),
            Map(std::collections::BTreeMap<String, serde_json::Value>),
        }

        match Raw::deserialize(deserializer)? {
            Raw::Str(s) => Ok(Self::parse_category(&s)),
            Raw::Map(map) => {
                let first = map
                    .into_iter()
                    .next()
                    .ok_or_else(|| serde::de::Error::custom("empty category object"))?;
                // Old derived form: {"custom": "<name>"} for Custom payloads;
                // unit variants serialized as {"<variant>": null}.
                if first.0 == "custom" {
                    let name = first.1.as_str().ok_or_else(|| {
                        serde::de::Error::custom("custom category name must be a string")
                    })?;
                    Ok(Self::Custom(name.to_string()))
                } else {
                    Ok(Self::parse_category(&first.0))
                }
            }
        }
    }
}

/// A single finding from an analyzer.
///
/// Represents a SEO or technical issue found during page analysis.
/// Each finding has a severity, category, machine-readable code, and
/// a human-readable recommendation for fixing the issue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    /// Issue severity (Critical, Error, Warning, Info).
    pub severity: Severity,
    /// Issue category (SEO, HTTP, Links, etc.).
    pub category: IssueCategory,
    /// Machine-readable issue code (e.g., "META001", "HTTP005").
    pub code: String,
    /// Short human-readable title.
    pub title: String,
    /// Detailed description of the issue.
    pub description: String,
    /// URL of the page where the issue was found.
    pub url: String,
    /// Recommendation for fixing the issue.
    pub recommendation: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_as_str_all_variants() {
        assert_eq!(Severity::Critical.as_str(), "critical");
        assert_eq!(Severity::Error.as_str(), "error");
        assert_eq!(Severity::Warning.as_str(), "warning");
        assert_eq!(Severity::Info.as_str(), "info");
    }

    #[test]
    fn severity_parse_roundtrip() {
        for s in ["critical", "error", "warning", "info"] {
            let sev = Severity::parse_severity(s).unwrap();
            assert_eq!(sev.as_str(), s);
        }
        assert!(Severity::parse_severity("bogus").is_none());
    }

    #[test]
    fn severity_copy() {
        let s = Severity::Error;
        let s2 = s;
        assert_eq!(s, s2);
    }

    // --- Wire-format pins (findings JSON schema contract) ---

    #[test]
    fn issue_category_serializes_to_canonical_string() {
        assert_eq!(
            serde_json::to_value(IssueCategory::Seo).unwrap(),
            serde_json::json!("seo")
        );
        let custom = IssueCategory::Custom("my-plugin".to_string());
        assert_eq!(
            serde_json::to_value(&custom).unwrap(),
            serde_json::json!("custom:my-plugin")
        );
    }

    #[test]
    fn issue_category_deserializes_from_canonical_string() {
        let v: IssueCategory = serde_json::from_value(serde_json::json!("links")).unwrap();
        assert_eq!(v, IssueCategory::Links);
    }

    #[test]
    fn issue_category_deserializes_legacy_map_form() {
        let v: IssueCategory =
            serde_json::from_value(serde_json::json!({"custom": "my-plugin"})).unwrap();
        assert_eq!(v, IssueCategory::Custom("my-plugin".to_string()));
    }

    #[test]
    fn issue_category_rejects_empty_object() {
        let result: Result<IssueCategory, _> = serde_json::from_value(serde_json::json!({}));
        assert!(result.is_err());
    }

    #[test]
    fn finding_json_has_exactly_schema_keys() {
        let f = Finding {
            severity: Severity::Warning,
            category: IssueCategory::Seo,
            code: "SEO001".to_string(),
            title: "Title".to_string(),
            description: "Description".to_string(),
            url: "https://example.com/".to_string(),
            recommendation: "Fix it".to_string(),
        };
        let v = serde_json::to_value(&f).unwrap();
        let obj = v.as_object().unwrap();
        let expected: std::collections::BTreeSet<&str> = [
            "severity",
            "category",
            "code",
            "title",
            "description",
            "url",
            "recommendation",
        ]
        .into_iter()
        .collect();
        let actual: std::collections::BTreeSet<&str> = obj.keys().map(String::as_str).collect();
        assert_eq!(actual, expected, "Finding wire shape drifted from schema");
        assert_eq!(obj["severity"], "warning");
        assert_eq!(obj["category"], "seo");
    }

    #[test]
    fn finding_json_roundtrip() {
        let f = Finding {
            severity: Severity::Critical,
            category: IssueCategory::Custom("my-plugin".to_string()),
            code: "MY001".to_string(),
            title: "T".to_string(),
            description: "D".to_string(),
            url: "https://example.com/p".to_string(),
            recommendation: "R".to_string(),
        };
        let json = serde_json::to_string(&f).unwrap();
        let f2: Finding = serde_json::from_str(&json).unwrap();
        assert_eq!(f2.code, f.code);
        assert_eq!(f2.category, f.category);
        assert_eq!(f2.severity, f.severity);
    }

    #[test]
    fn issue_category_as_str_roundtrip() {
        let cats = [
            IssueCategory::Http,
            IssueCategory::Seo,
            IssueCategory::Content,
            IssueCategory::Links,
            IssueCategory::Images,
            IssueCategory::Schema,
            IssueCategory::Security,
            IssueCategory::Performance,
            IssueCategory::Mobile,
            IssueCategory::Accessibility,
            IssueCategory::Social,
        ];
        for cat in cats {
            let s = cat.as_str();
            let parsed = IssueCategory::parse_category(&s);
            assert_eq!(cat, parsed);
        }
    }

    #[test]
    fn issue_category_custom_roundtrip() {
        let cat = IssueCategory::Custom("plugin:my-check".to_string());
        let s = cat.as_str();
        assert_eq!(s, "custom:plugin:my-check");
        let parsed = IssueCategory::parse_category(&s);
        assert_eq!(cat, parsed);
    }

    #[test]
    fn issue_category_from_str() {
        let cat: IssueCategory = "seo".into();
        assert_eq!(cat, IssueCategory::Seo);
        let cat: IssueCategory = "custom:foo".into();
        assert_eq!(cat, IssueCategory::Custom("foo".to_string()));
    }

    #[test]
    fn finding_construction_and_field_access() {
        let f = Finding {
            severity: Severity::Error,
            category: IssueCategory::Seo,
            code: "SEO001".into(),
            title: "Missing title".into(),
            description: "Page has no title tag".into(),
            url: "https://example.com".into(),
            recommendation: "Add a title tag".into(),
        };
        assert_eq!(f.severity, Severity::Error);
        assert_eq!(f.category, IssueCategory::Seo);
        assert_eq!(f.code, "SEO001");
    }

    #[test]
    fn finding_serialization_roundtrip() {
        let f = Finding {
            severity: Severity::Critical,
            category: IssueCategory::Security,
            code: "SEC001".into(),
            title: "XSS".into(),
            description: "Unsafe innerHTML".into(),
            url: "https://example.com/page".into(),
            recommendation: "Sanitize input".into(),
        };
        let json = serde_json::to_string(&f).expect("serialize");
        let deserialized: Finding = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(f.severity, deserialized.severity);
        assert_eq!(f.category, deserialized.category);
        assert_eq!(f.code, deserialized.code);
    }

    #[test]
    fn finding_deserialize_from_json() {
        let json = r#"{
            "severity": "warning",
            "category": "accessibility",
            "code": "A11Y01",
            "title": "Missing alt text",
            "description": "Image without alt attribute",
            "url": "https://example.com",
            "recommendation": "Add alt text to images"
        }"#;
        let f: Finding = serde_json::from_str(json).expect("deserialize from json");
        assert_eq!(f.severity, Severity::Warning);
        assert_eq!(f.category, IssueCategory::Accessibility);
        assert_eq!(f.code, "A11Y01");
    }
}
