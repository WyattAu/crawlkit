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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_construction_minimal() {
        let ctx = AnalysisContext {
            url: "https://example.com".into(),
            html: "<html></html>".into(),
            status_code: None,
            headers: Vec::new(),
            response_time_ms: None,
        };
        assert_eq!(ctx.url, "https://example.com");
        assert_eq!(ctx.html, "<html></html>");
        assert!(ctx.status_code.is_none());
        assert!(ctx.headers.is_empty());
        assert!(ctx.response_time_ms.is_none());
    }

    #[test]
    fn context_construction_full() {
        let ctx = AnalysisContext {
            url: "https://example.com/page".into(),
            html: "<html><body>Hi</body></html>".into(),
            status_code: Some(200),
            headers: vec![
                ("content-type".into(), "text/html".into()),
                ("cache-control".into(), "no-cache".into()),
            ],
            response_time_ms: Some(150),
        };
        assert_eq!(ctx.status_code, Some(200));
        assert_eq!(ctx.headers.len(), 2);
        assert_eq!(ctx.headers[0].1, "text/html");
        assert_eq!(ctx.response_time_ms, Some(150));
    }

    #[test]
    fn context_clone() {
        let ctx = AnalysisContext {
            url: "u".into(),
            html: "h".into(),
            status_code: Some(404),
            headers: vec![("k".into(), "v".into())],
            response_time_ms: Some(50),
        };
        let cloned = ctx.clone();
        assert_eq!(ctx.url, cloned.url);
        assert_eq!(ctx.status_code, cloned.status_code);
        assert_eq!(ctx.headers, cloned.headers);
    }

    #[test]
    fn context_debug() {
        let ctx = AnalysisContext {
            url: "u".into(),
            html: "h".into(),
            status_code: None,
            headers: Vec::new(),
            response_time_ms: None,
        };
        let dbg = format!("{:?}", ctx);
        assert!(dbg.contains("AnalysisContext"));
    }

    #[test]
    fn context_serialization_roundtrip() {
        let ctx = AnalysisContext {
            url: "https://example.com".into(),
            html: "<h1>Hello</h1>".into(),
            status_code: Some(200),
            headers: vec![("x-powered-by".into(), "rust".into())],
            response_time_ms: Some(42),
        };
        let json = serde_json::to_string(&ctx).unwrap();
        let deserialized: AnalysisContext = serde_json::from_str(&json).unwrap();
        assert_eq!(ctx.url, deserialized.url);
        assert_eq!(ctx.html, deserialized.html);
        assert_eq!(ctx.status_code, deserialized.status_code);
        assert_eq!(ctx.headers, deserialized.headers);
        assert_eq!(ctx.response_time_ms, deserialized.response_time_ms);
    }

    #[test]
    fn context_deserialize_from_json() {
        let json = r#"{
            "url": "https://test.com",
            "html": "<p>test</p>",
            "status_code": 301,
            "headers": [["location", "https://new.com"]],
            "response_time_ms": 100
        }"#;
        let ctx: AnalysisContext = serde_json::from_str(json).unwrap();
        assert_eq!(ctx.status_code, Some(301));
        assert_eq!(ctx.headers[0].0, "location");
        assert_eq!(ctx.response_time_ms, Some(100));
    }
}
