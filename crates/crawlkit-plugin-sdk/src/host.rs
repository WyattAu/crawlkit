//! Host-provided structured context (API v1.1 surface, B4).
//!
//! The engine links `crawlkit_host.get_context` for every plugin. Calling
//! it inside [`analyze`](crate::Analyzer::analyze) returns the analysis
//! context the host captured for this page — URL, status, headers, and a
//! parsed-page summary — as a JSON string, without re-parsing the raw
//! HTML. Plugins that never call it are unchanged (the import is only
//! present if you reference this module).
//!
//! ```
//! use crawlkit_plugin_sdk::{host, AnalysisContext, Analyzer, Finding};
//! # use crawlkit_plugin_sdk::Severity;
//! # pub struct MyAnalyzer;
//! # impl MyAnalyzer { pub fn new() -> Self { Self } }
//! # impl Analyzer for MyAnalyzer {
//! #     fn name(&self) -> &str { "my-analyzer" }
//! #     fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> { vec![] }
//! # }
//! // Inside your analyze():
//! # fn demo(ctx: &AnalysisContext) -> Vec<Finding> {
//! if let Some(Ok(host_ctx)) = host::context() {
//!     if host_ctx.status_code == Some(404) {
//!         return vec![Finding {
//!             severity: Severity::Warning,
//!             category: "seo".into(),
//!             code: "SOFT404".into(),
//!             title: "Analyzed an error page".into(),
//!             description: "This URL returned 404".into(),
//!             url: host_ctx.url.clone(),
//!             recommendation: "Fix the broken link".into(),
//!         }];
//!     }
//! }
//! # vec![]
//! # }
//! ```

use serde::Deserialize;

// Raw wasm import. Returns a NUL-terminated JSON string pointer, or 0
// (null) when the host has no context for this call.
//
// The pointer, when non-zero, points to host-written guest memory that
// the guest owns after the call. [`context_json`] is the safe wrapper.
#[link(wasm_import_module = "crawlkit_host")]
unsafe extern "C" {
    fn get_context() -> i32;
}

/// Parsed-page summary fields the host precomputes.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ParsedSummary {
    pub title: Option<String>,
    pub description: Option<String>,
    pub canonical: Option<String>,
    pub word_count: usize,
    pub sentence_count: usize,
    #[serde(default)]
    pub headings: Vec<HeadingSummary>,
    pub link_count: usize,
    pub image_count: usize,
    pub lang: Option<String>,
}

/// One heading in the parsed summary.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct HeadingSummary {
    pub level: u8,
    pub text: String,
}

/// The structured analysis context returned by [`context`].
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct HostContext {
    pub url: String,
    pub status_code: Option<u16>,
    pub response_time_ms: Option<u64>,
    #[serde(default)]
    pub headers: Vec<(String, String)>,
    #[serde(default)]
    pub parsed: Option<ParsedSummary>,
}

/// Read the host context as a raw JSON string.
///
/// Returns `None` when the host provided no context for this call (e.g.
/// the plugin is run through a plain `analyze` without context).
#[must_use]
pub fn context_json() -> Option<String> {
    // SAFETY: the host writes into guest-owned memory at a pointer it
    // allocated via this module's alloc protocol; reading up to the NUL
    // terminator is the documented contract.
    let ptr = unsafe { get_context() };
    if ptr == 0 {
        return None;
    }
    let mut len = 0usize;
    // SAFETY: pointer validity and NUL termination are guaranteed by the
    // host before it returns a non-zero pointer.
    unsafe {
        while *((ptr as *const u8).add(len)) != 0 {
            len += 1;
        }
        let slice = std::slice::from_raw_parts(ptr as *const u8, len);
        String::from_utf8(slice.to_vec()).ok()
    }
}

/// Read and parse the host context.
///
/// # Errors
///
/// Returns `serde_json::Error` when the host blob is present but not
/// valid [`HostContext`] JSON (an engine/plugin version skew).
pub fn context() -> Option<Result<HostContext, serde_json::Error>> {
    context_json().map(|json| serde_json::from_str(&json))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_context_parses_full_shape() {
        let json = r#"{
            "url": "https://example.com/page",
            "status_code": 200,
            "response_time_ms": 412,
            "headers": [["content-type", "text/html"], ["x-foo", "bar"]],
            "parsed": {
                "title": "Example",
                "description": "A page",
                "canonical": "https://example.com/page",
                "word_count": 517,
                "sentence_count": 31,
                "headings": [{"level": 1, "text": "Example"}],
                "link_count": 23,
                "image_count": 7,
                "lang": "en"
            }
        }"#;
        let ctx: HostContext = serde_json::from_str(json).unwrap();
        assert_eq!(ctx.url, "https://example.com/page");
        assert_eq!(ctx.status_code, Some(200));
        assert_eq!(ctx.response_time_ms, Some(412));
        assert_eq!(ctx.headers.len(), 2);
        let parsed = ctx.parsed.as_ref().unwrap();
        assert_eq!(parsed.title.as_deref(), Some("Example"));
        assert_eq!(parsed.word_count, 517);
        assert_eq!(parsed.headings[0].level, 1);
        assert_eq!(parsed.lang.as_deref(), Some("en"));
    }

    #[test]
    fn host_context_minimal_shape() {
        // Only url is required; everything else degrades to None/default.
        let ctx: HostContext = serde_json::from_str(r#"{"url":"https://example.com"}"#).unwrap();
        assert_eq!(ctx.url, "https://example.com");
        assert_eq!(ctx.status_code, None);
        assert!(ctx.headers.is_empty());
        assert!(ctx.parsed.is_none());
    }

    #[test]
    fn host_context_rejects_missing_url() {
        assert!(serde_json::from_str::<HostContext>("{}").is_err());
    }
}
