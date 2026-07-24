//! Analysis context for plugins.

use serde::{Deserialize, Serialize};

/// Context passed to analyzers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisContext {
    /// The page URL.
    pub url: String,
    /// The HTML content.
    pub html: String,
    /// HTTP status code.
    pub status_code: Option<u16>,
    /// Response headers.
    pub headers: Vec<(String, String)>,
    /// Response time in milliseconds.
    pub response_time_ms: Option<u64>,
}
