//! WASM export macros for plugins.

/// Export an analyzer as a WASM plugin.
///
/// This macro generates the required WASM export functions:
/// - `crawlkit_plugin_init`
/// - `crawlkit_plugin_analyze`
/// - `crawlkit_plugin_free_string`
///
/// # Example
///
/// ```rust,no_run
/// use crawlkit_plugin_sdk::{Analyzer, Finding, Severity, AnalysisContext};
///
/// pub struct MyAnalyzer;
/// impl MyAnalyzer { pub fn new() -> Self { Self } }
/// impl Default for MyAnalyzer { fn default() -> Self { Self::new() } }
///
/// impl Analyzer for MyAnalyzer {
///     fn name(&self) -> &str { "my-analyzer" }
///     fn analyze(&self, _ctx: &AnalysisContext) -> Vec<Finding> { vec![] }
/// }
///
/// crawlkit_plugin_sdk::export_analyzer!(MyAnalyzer);
/// ```
#[macro_export]
macro_rules! export_analyzer {
    ($analyzer_type:ty) => {
        static mut ANALYZER: Option<$analyzer_type> = None;

        #[no_mangle]
        pub extern "C" fn crawlkit_plugin_init() -> i32 {
            // SAFETY: WASM modules are single-threaded. This static is only
            // accessed from the single execution thread of the WASM runtime.
            #[allow(static_mut_refs)]
            unsafe {
                ANALYZER = Some(<$analyzer_type>::new());
            }
            0
        }

        /// # Safety
        ///
        /// This function reads from raw pointers provided by the host.
        /// The host must ensure the pointers are valid and point to valid UTF-8 data.
        #[no_mangle]
        pub unsafe extern "C" fn crawlkit_plugin_analyze(
            html_ptr: *const u8,
            html_len: usize,
            url_ptr: *const u8,
            url_len: usize,
        ) -> *mut u8 {
            let html = String::from_utf8_unchecked(
                std::slice::from_raw_parts(html_ptr, html_len).to_vec(),
            );
            let url =
                String::from_utf8_unchecked(std::slice::from_raw_parts(url_ptr, url_len).to_vec());

            let ctx = $crate::AnalysisContext {
                url,
                html,
                status_code: None,
                headers: Vec::new(),
                response_time_ms: None,
            };

            // SAFETY: WASM modules are single-threaded.
            #[allow(static_mut_refs)]
            let analyzer = ANALYZER.as_ref().expect("Analyzer not initialized");
            let findings = analyzer.analyze(&ctx);

            let json = serde_json::to_string(&findings).unwrap();
            let bytes = json.into_bytes();
            let ptr = bytes.as_ptr();
            std::mem::forget(bytes);
            ptr as *mut u8
        }

        /// # Safety
        ///
        /// This function frees memory allocated by the plugin.
        /// The pointer must have been returned by `crawlkit_plugin_analyze`.
        #[no_mangle]
        pub unsafe extern "C" fn crawlkit_plugin_free_string(ptr: *mut u8) {
            if !ptr.is_null() {
                let _ = Box::from_raw(ptr);
            }
        }

        #[no_mangle]
        pub extern "C" fn crawlkit_plugin_api_version() -> *mut u8 {
            let version = "1.0";
            let bytes = version.as_bytes().to_vec();
            let ptr = bytes.as_ptr();
            std::mem::forget(bytes);
            ptr as *mut u8
        }
    };
}

#[cfg(test)]
mod tests {
    use crate::{AnalysisContext, Analyzer, Finding, Severity};

    struct TestAnalyzer;

    impl TestAnalyzer {
        fn new() -> Self {
            Self
        }
    }

    impl Analyzer for TestAnalyzer {
        fn name(&self) -> &str {
            "test-analyzer"
        }

        fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
            if ctx.html.contains("<h1>") {
                vec![Finding {
                    severity: Severity::Info,
                    category: "structure".into(),
                    code: "STRUCT01".into(),
                    title: "Has heading".into(),
                    description: "Page contains an h1 tag".into(),
                    url: ctx.url.clone(),
                    recommendation: "None needed".into(),
                }]
            } else {
                vec![]
            }
        }
    }

    export_analyzer!(TestAnalyzer);

    #[test]
    fn plugin_init_returns_zero() {
        let result = crawlkit_plugin_init();
        assert_eq!(result, 0);
    }

    #[test]
    fn plugin_api_version_returns_1_0() {
        let ptr = crawlkit_plugin_api_version();
        assert!(!ptr.is_null());
        unsafe {
            let slice = std::slice::from_raw_parts(ptr, 3);
            let version = std::str::from_utf8_unchecked(slice);
            assert_eq!(version, "1.0");
        }
    }

    #[test]
    fn plugin_analyze_with_heading() {
        let _ = crawlkit_plugin_init();
        let html = "<html><body><h1>Title</h1></body></html>";
        let url = "https://example.com";
        let html_bytes = html.as_bytes();
        let url_bytes = url.as_bytes();

        let result_ptr = unsafe {
            crawlkit_plugin_analyze(
                html_bytes.as_ptr(),
                html_bytes.len(),
                url_bytes.as_ptr(),
                url_bytes.len(),
            )
        };
        assert!(!result_ptr.is_null());

        let expected = vec![Finding {
            severity: Severity::Info,
            category: "structure".into(),
            code: "STRUCT01".into(),
            title: "Has heading".into(),
            description: "Page contains an h1 tag".into(),
            url: "https://example.com".into(),
            recommendation: "None needed".into(),
        }];
        let expected_json = serde_json::to_string(&expected).unwrap();
        let expected_bytes = expected_json.as_bytes();

        unsafe {
            let result_slice = std::slice::from_raw_parts(result_ptr, expected_bytes.len());
            assert_eq!(result_slice, expected_bytes);
            crawlkit_plugin_free_string(result_ptr);
        }
    }

    #[test]
    fn plugin_analyze_no_findings() {
        let _ = crawlkit_plugin_init();
        let html = "<html><body>Plain text</body></html>";
        let url = "https://example.com/other";
        let html_bytes = html.as_bytes();
        let url_bytes = url.as_bytes();

        let result_ptr = unsafe {
            crawlkit_plugin_analyze(
                html_bytes.as_ptr(),
                html_bytes.len(),
                url_bytes.as_ptr(),
                url_bytes.len(),
            )
        };
        assert!(!result_ptr.is_null());

        let expected_json = serde_json::to_string(&Vec::<Finding>::new()).unwrap();
        let expected_bytes = expected_json.as_bytes();

        unsafe {
            let result_slice = std::slice::from_raw_parts(result_ptr, expected_bytes.len());
            assert_eq!(result_slice, expected_bytes);
            crawlkit_plugin_free_string(result_ptr);
        }
    }

    #[test]
    fn plugin_free_null_pointer_is_safe() {
        unsafe {
            crawlkit_plugin_free_string(std::ptr::null_mut());
        }
    }

    #[test]
    fn plugin_free_string_after_analyze() {
        let _ = crawlkit_plugin_init();
        let html = "<h1>Hi</h1>";
        let url = "https://a.com";
        let html_bytes = html.as_bytes();
        let url_bytes = url.as_bytes();

        let result_ptr = unsafe {
            crawlkit_plugin_analyze(
                html_bytes.as_ptr(),
                html_bytes.len(),
                url_bytes.as_ptr(),
                url_bytes.len(),
            )
        };
        unsafe {
            crawlkit_plugin_free_string(result_ptr);
        }
    }
}
