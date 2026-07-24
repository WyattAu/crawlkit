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
