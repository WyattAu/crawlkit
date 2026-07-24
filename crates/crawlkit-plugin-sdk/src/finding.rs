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
