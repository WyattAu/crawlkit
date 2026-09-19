//! JavaScript error findings (6.0.0-alpha.2 "JS-error findings class").
//!
//! Client-side failures are invisible to static analysis: a page can return
//! HTTP 200 with perfect markup and still be broken because its hydration
//! script threw. This analyzer turns the renderer's error capture into
//! findings so broken-by-JS pages surface in reports and exports alongside
//! every other issue class.
//!
//! Codes:
//!
//! - `JSERR001` — uncaught exceptions (`pageerror` events). These are the
//!   strongest signal: an exception that escaped every handler can abort
//!   hydration and leave the page half-rendered.
//! - `JSERR002` — elevated console error volume (≥ 3 error-level console
//!   messages). Individually noisy but collectively a reliability smell.
//!
//! The analyzer runs only when rendering data is present; pages crawled
//! statically produce no findings here (the render-budget degradation path
//! already reports that absence as `RENDER001`/`RENDER002`).

use crate::analyzers::{AnalysisContext, Analyzer};
use crate::{Finding, IssueCategory, Severity};

/// Console error-level message count at or above which `JSERR002` fires.
const CONSOLE_ERROR_THRESHOLD: usize = 3;

/// Detects JavaScript errors captured during rendering.
pub struct JsErrorAnalyzer;

impl JsErrorAnalyzer {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for JsErrorAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for JsErrorAnalyzer {
    fn name(&self) -> &str {
        "JsErrorAnalyzer"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        #[cfg(feature = "full")]
        {
            let Some(rendered) = ctx.rendered else {
                return Vec::new();
            };
            let url = rendered.final_url.clone();
            let mut findings = Vec::new();

            if !rendered.page_errors.is_empty() {
                let count = rendered.page_errors.len();
                let first_message = rendered.page_errors[0].message.clone();
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Custom("Reliability".to_string()),
                    code: "JSERR001".to_string(),
                    title: "Uncaught JavaScript error during rendering".to_string(),
                    description: format!(
                        "The page threw {count} uncaught exception(s) while it was being \
                         rendered; the first was: \"{first_message}\". Uncaught exceptions \
                         can abort hydration, leaving content missing or interactive \
                         elements dead even though the HTML looks complete."
                    ),
                    url: url.clone(),
                    recommendation: "Fix the uncaught exception (see the browser console for \
                         the full stack). Pages that fail hydration are crawled as their \
                         server-rendered HTML only, which may not match what users see."
                        .to_string(),
                });
            }

            let console_errors = rendered
                .console_messages
                .iter()
                .filter(|m| m.level == "error")
                .count();
            if console_errors >= CONSOLE_ERROR_THRESHOLD {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Custom("Reliability".to_string()),
                    code: "JSERR002".to_string(),
                    title: "Elevated JavaScript console errors".to_string(),
                    description: format!(
                        "The page logged {console_errors} error-level console messages \
                         during rendering. Recurring console errors often indicate broken \
                         requests, failing integrations, or code paths that degrade the \
                         user experience."
                    ),
                    url,
                    recommendation: "Review the console errors captured during rendering and \
                         fix or silence the underlying causes."
                        .to_string(),
                });
            }

            findings
        }
        #[cfg(not(feature = "full"))]
        {
            let _ = ctx;
            Vec::new()
        }
    }
}

#[cfg(all(test, feature = "full"))]
mod tests {
    use super::*;
    use crate::analyzers::Analyzer;
    use crate::playwright::{ConsoleMessage, PageError, RenderedPage};
    use std::time::Duration;

    fn ctx_with(rendered: RenderedPage) -> AnalysisContext<'static> {
        // A minimal empty page: the JS-error analyzer reads only the
        // rendered data, so no parsed markup is needed.
        let page = Box::leak(Box::new(crate::ParsedPage {
            url: "https://example.com/".to_string(),
            ..test_page()
        }));
        AnalysisContext {
            page: &*page,
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
            rendered: Some(Box::leak(Box::new(rendered))),
        }
    }

    fn test_page() -> crate::ParsedPage {
        let url = url::Url::parse("https://example.com/").expect("valid url");
        crate::parser::HtmlParser::parse("<html><body><p>hi</p></body></html>", &url)
    }

    fn clean_page() -> RenderedPage {
        RenderedPage {
            final_url: "https://example.com/".to_string(),
            html: "<html></html>".to_string(),
            console_messages: Vec::new(),
            network_requests: Vec::new(),
            wasm_errors: Vec::new(),
            page_errors: Vec::new(),
            render_time: Duration::from_millis(10),
            memory_used: 0,
        }
    }

    #[test]
    fn clean_render_produces_no_findings() {
        let ctx = ctx_with(clean_page());
        assert!(JsErrorAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn uncaught_page_error_emits_jserr001() {
        let mut page = clean_page();
        page.page_errors.push(PageError {
            message: "TypeError: x is not a function".to_string(),
            source: Some("https://example.com/app.js".to_string()),
            line: Some(42),
        });
        let ctx = ctx_with(page);
        let findings = JsErrorAnalyzer::new().analyze(&ctx);
        let f = findings
            .iter()
            .find(|f| f.code == "JSERR001")
            .expect("JSERR001 for uncaught exception");
        assert_eq!(f.severity, Severity::Error);
        assert_eq!(f.category.as_str(), "custom:Reliability");
        assert!(f.description.contains("TypeError"));
    }

    #[test]
    fn console_error_volume_emits_jserr002_at_threshold() {
        let mut page = clean_page();
        for i in 0..3 {
            page.console_messages.push(ConsoleMessage {
                level: "error".to_string(),
                text: format!("error {i}"),
                source: None,
                line: None,
            });
        }
        let ctx = ctx_with(page);
        let findings = JsErrorAnalyzer::new().analyze(&ctx);
        assert!(
            findings.iter().any(|f| f.code == "JSERR002"),
            "3 console errors must trip the threshold"
        );
    }

    #[test]
    fn below_threshold_console_errors_stay_silent() {
        let mut page = clean_page();
        for i in 0..2 {
            page.console_messages.push(ConsoleMessage {
                level: "error".to_string(),
                text: format!("error {i}"),
                source: None,
                line: None,
            });
        }
        let ctx = ctx_with(page);
        assert!(JsErrorAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn warnings_do_not_count_toward_console_error_volume() {
        let mut page = clean_page();
        for i in 0..5 {
            page.console_messages.push(ConsoleMessage {
                level: "warning".to_string(),
                text: format!("warn {i}"),
                source: None,
                line: None,
            });
        }
        let ctx = ctx_with(page);
        assert!(JsErrorAnalyzer::new().analyze(&ctx).is_empty());
    }
}
