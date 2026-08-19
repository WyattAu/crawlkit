//! WASM export macros for plugins.
//!
//! The generated exports implement the host-side ABI expected by
//! `crawlkit-engine`'s wasmtime loader:
//!
//! - `crawlkit_plugin_init(reserved: usize) -> i32` — 0 on success
//! - `crawlkit_plugin_alloc(size: usize) -> usize` — allocate a buffer the
//!   host can write into; returns a pointer (0 on failure)
//! - `crawlkit_plugin_analyze(html_ptr, html_len, url_ptr, url_len) -> usize`
//!   — returns a pointer to a NUL-terminated JSON array of findings
//! - `crawlkit_plugin_free(ptr: usize)` — release any pointer returned by
//!   `alloc`/`analyze`
//!
//! Parameters and return values use `usize`, which is 32-bit on
//! `wasm32-unknown-unknown` (ABI-identical to the host's `i32` signatures)
//! and 64-bit on native targets, allowing the exports to be unit-tested
//! directly.
//!
//! # Memory layout
//!
//! Allocations carry a 4-byte little-endian size header immediately before
//! the returned pointer so that `free` can reconstruct the original
//! `Layout` (fixing the previous `Box::from_raw` mismatched-layout bug).

/// Size of the allocation header in bytes.
const HEADER_SIZE: usize = 4;

/// Allocate `size` bytes of guest memory with a size header.
///
/// Returns 0 on allocation failure (the host treats 0 as a null pointer).
///
/// # Safety
///
/// The returned pointer must only be released through [`free_raw`].
pub unsafe fn alloc_raw(size: usize) -> usize {
    let total = match size.checked_add(HEADER_SIZE) {
        Some(t) => t,
        None => return 0,
    };
    let layout = match std::alloc::Layout::from_size_align(total, 8) {
        Ok(l) => l,
        Err(_) => return 0,
    };
    let base = unsafe { std::alloc::alloc(layout) };
    if base.is_null() {
        return 0;
    }
    // Write the little-endian size header.
    let size_bytes = (size as u32).to_le_bytes();
    unsafe { std::ptr::copy_nonoverlapping(size_bytes.as_ptr(), base, HEADER_SIZE) };
    (unsafe { base.add(HEADER_SIZE) }) as usize
}

/// Release a pointer returned by [`alloc_raw`] (or by `analyze`).
///
/// # Safety
///
/// `ptr` must have originated from [`alloc_raw`] and must not be freed twice.
pub unsafe fn free_raw(ptr: usize) {
    if ptr == 0 {
        return;
    }
    let user_ptr = ptr as *mut u8;
    // SAFETY: caller guarantees the header precedes the pointer.
    unsafe {
        let base = user_ptr.sub(HEADER_SIZE);
        let size = u32::from_le_bytes(base.cast::<[u8; 4]>().read()) as usize;
        let total = size + HEADER_SIZE;
        if let Ok(layout) = std::alloc::Layout::from_size_align(total, 8) {
            std::alloc::dealloc(base, layout);
        }
    }
}

/// Export an analyzer as a WASM plugin implementing the host ABI.
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
        pub extern "C" fn crawlkit_plugin_init(_reserved: usize) -> i32 {
            // SAFETY: WASM modules are single-threaded. This static is only
            // accessed from the single execution thread of the WASM runtime.
            #[allow(static_mut_refs)]
            unsafe {
                ANALYZER = Some(<$analyzer_type>::new());
            }
            0
        }

        #[no_mangle]
        pub extern "C" fn crawlkit_plugin_alloc(size: usize) -> usize {
            // SAFETY: allocation is paired with crawlkit_plugin_free by the host.
            unsafe { $crate::exported::alloc_raw(size) }
        }

        #[no_mangle]
        pub extern "C" fn crawlkit_plugin_free(ptr: usize) {
            // SAFETY: the host only passes pointers from alloc/analyze.
            unsafe { $crate::exported::free_raw(ptr) }
        }

        /// # Safety
        ///
        /// This function reads from raw pointers provided by the host.
        /// The host must ensure the pointers are valid for `html_len` /
        /// `url_len` bytes and point to valid UTF-8 data.
        #[no_mangle]
        pub unsafe extern "C" fn crawlkit_plugin_analyze(
            html_ptr: usize,
            html_len: usize,
            url_ptr: usize,
            url_len: usize,
        ) -> usize {
            // SAFETY: caller (host) guarantees pointer/length validity.
            unsafe {
                let html_slice = std::slice::from_raw_parts(html_ptr as *const u8, html_len);
                let url_slice = std::slice::from_raw_parts(url_ptr as *const u8, url_len);
                let html = String::from_utf8_lossy(html_slice).into_owned();
                let url = String::from_utf8_lossy(url_slice).into_owned();

                let ctx = $crate::AnalysisContext {
                    url,
                    html,
                    status_code: None,
                    headers: Vec::new(),
                    response_time_ms: None,
                };

                // SAFETY: WASM modules are single-threaded.
                #[allow(static_mut_refs)]
                let analyzer = match ANALYZER.as_ref() {
                    Some(a) => a,
                    None => return 0,
                };
                let findings = analyzer.analyze(&ctx);

                // Serialization cannot panic: fall back to an empty finding
                // list rather than aborting the guest (which would trap).
                let json = serde_json::to_string(&findings).unwrap_or_else(|_| "[]".to_string());

                // NUL-terminated result buffer: [header][json bytes][0x00]
                let payload_len = json.len() + 1;
                let ptr = $crate::exported::alloc_raw(payload_len);
                if ptr == 0 {
                    return 0;
                }
                let mut cursor = ptr as *mut u8;
                std::ptr::copy_nonoverlapping(json.as_ptr(), cursor, json.len());
                cursor = cursor.add(json.len());
                cursor.write(0);
                ptr
            }
        }

        /// Legacy alias kept for older hosts. Header-aware like
        /// `crawlkit_plugin_free`.
        ///
        /// # Safety
        ///
        /// `ptr` must originate from this module's allocator.
        #[no_mangle]
        pub unsafe extern "C" fn crawlkit_plugin_free_string(ptr: usize) {
            // SAFETY: same contract as crawlkit_plugin_free.
            unsafe { $crate::exported::free_raw(ptr) }
        }

        #[no_mangle]
        pub extern "C" fn crawlkit_plugin_api_version() -> usize {
            // Leaks a small static string; only called for diagnostics.
            let version = b"1.0\0";
            version.as_ptr() as usize
        }
    };
}

// Host-ABI internals are re-exported from the crate root as
// `crawlkit_plugin_sdk::exported` (see `lib.rs`); the macro references
// them there.

#[cfg(test)]
mod tests {
    use super::{alloc_raw, free_raw};
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

    /// Read a NUL-terminated string starting at `ptr` (test helper).
    fn read_c_string(ptr: usize) -> String {
        assert_ne!(ptr, 0, "null pointer returned");
        let mut len = 0usize;
        unsafe {
            while *(ptr as *const u8).add(len) != 0 {
                len += 1;
            }
            String::from_utf8(std::slice::from_raw_parts(ptr as *const u8, len).to_vec()).unwrap()
        }
    }

    #[test]
    fn alloc_free_roundtrip_is_sound() {
        let ptr = unsafe { alloc_raw(1024) };
        assert_ne!(ptr, 0);
        unsafe {
            std::ptr::write_bytes(ptr as *mut u8, 0xAB, 1024);
            free_raw(ptr);
        }
    }

    #[test]
    fn free_of_null_is_safe() {
        unsafe { free_raw(0) };
    }

    #[test]
    fn plugin_init_returns_zero() {
        let result = crawlkit_plugin_init(0);
        assert_eq!(result, 0);
    }

    #[test]
    fn plugin_alloc_returns_writable_memory() {
        let ptr = crawlkit_plugin_alloc(64);
        assert_ne!(ptr, 0);
        unsafe {
            std::ptr::write_bytes(ptr as *mut u8, 0x42, 64);
        }
        crawlkit_plugin_free(ptr);
    }

    #[test]
    fn plugin_analyze_matches_host_read_protocol() {
        let _ = crawlkit_plugin_init(0);
        let html = "<html><body><h1>Title</h1></body></html>";
        let url = "https://example.com";
        let html_bytes = html.as_bytes();
        let url_bytes = url.as_bytes();

        // Simulate the host: allocate, copy input, call analyze, read the
        // NUL-terminated result, then free all three pointers.
        let html_ptr = crawlkit_plugin_alloc(html_bytes.len());
        let url_ptr = crawlkit_plugin_alloc(url_bytes.len());
        unsafe {
            std::ptr::copy_nonoverlapping(
                html_bytes.as_ptr(),
                html_ptr as *mut u8,
                html_bytes.len(),
            );
            std::ptr::copy_nonoverlapping(url_bytes.as_ptr(), url_ptr as *mut u8, url_bytes.len());
        }

        let result_ptr = unsafe {
            crawlkit_plugin_analyze(html_ptr, html_bytes.len(), url_ptr, url_bytes.len())
        };
        assert_ne!(result_ptr, 0, "analyze returned null pointer");

        let result = read_c_string(result_ptr);
        crawlkit_plugin_free(html_ptr);
        crawlkit_plugin_free(url_ptr);
        crawlkit_plugin_free(result_ptr);

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
        assert_eq!(result, expected_json);
    }

    #[test]
    fn plugin_analyze_empty_findings_is_valid_json() {
        let _ = crawlkit_plugin_init(0);
        let html = "<html><body>Plain text</body></html>";
        let url = "https://example.com/other";
        let html_bytes = html.as_bytes();
        let url_bytes = url.as_bytes();

        let html_ptr = crawlkit_plugin_alloc(html_bytes.len());
        let url_ptr = crawlkit_plugin_alloc(url_bytes.len());
        unsafe {
            std::ptr::copy_nonoverlapping(
                html_bytes.as_ptr(),
                html_ptr as *mut u8,
                html_bytes.len(),
            );
            std::ptr::copy_nonoverlapping(url_bytes.as_ptr(), url_ptr as *mut u8, url_bytes.len());
        }

        let result_ptr = unsafe {
            crawlkit_plugin_analyze(html_ptr, html_bytes.len(), url_ptr, url_bytes.len())
        };
        assert_ne!(result_ptr, 0);
        let result = read_c_string(result_ptr);
        crawlkit_plugin_free(html_ptr);
        crawlkit_plugin_free(url_ptr);
        crawlkit_plugin_free(result_ptr);

        assert_eq!(result, "[]");
    }

    #[test]
    fn api_version_is_nul_terminated() {
        let ptr = crawlkit_plugin_api_version();
        let version = read_c_string(ptr);
        assert_eq!(version, "1.0");
    }
}
