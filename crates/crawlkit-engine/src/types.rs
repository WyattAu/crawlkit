//! Core types shared across all feature gates.
//!
//! `Severity` and `IssueCategory` live here so analyzers can import them
//! without pulling in the gated `storage` module.

/// Severity level for an issue/finding.
///
/// Used by analyzers to classify the importance of detected issues.
/// Stored in the database as a lowercase string for querying.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
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
