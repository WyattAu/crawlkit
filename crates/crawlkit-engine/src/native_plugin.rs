//! Native plugin loading via dynamic linking (libloading).
//!
//! Provides [`NativePlugin`] for loading shared libraries (`.so`/`.dylib`/`.dll`)
//! that implement the crawlkit plugin ABI (`crawlkit_plugin_analyze` and
//! `crawlkit_plugin_free`).
//!
//! # Safety
//!
//! The FFI boundaries (`analyze` and `free` symbols) must adhere to the ABI
//! contract: allocate with `alloc` from the host, return null-terminated UTF-8,
//! and free only pointers returned by the plugin.

#![allow(unsafe_code)]

use std::path::Path;

use libloading::Library;

use crate::plugin::PluginError;

type AnalyzeFn = unsafe extern "C" fn(*const u8, usize) -> *mut u8;
type FreeFn = unsafe extern "C" fn(*mut u8);

/// Native plugin loaded via dynamic linking.
///
/// Wraps a shared library that exposes `crawlkit_plugin_analyze` and
/// `crawlkit_plugin_free` symbols. The library must conform to the
/// crawlkit native plugin ABI.
#[derive(Debug)]
pub struct NativePlugin {
    _library: Library,
    analyze: AnalyzeFn,
    free: FreeFn,
}

impl NativePlugin {
    /// Load a native plugin from a shared library (`.so`/`.dylib`/`.dll`).
    ///
    /// The library must export the following symbols:
    /// - `crawlkit_plugin_analyze(input_ptr: *const u8, input_len: usize) -> *mut u8`
    /// - `crawlkit_plugin_free(ptr: *mut u8)`
    ///
    /// # Errors
    ///
    /// Returns [`PluginError::LoadFailed`] if the library cannot be loaded or
    /// the required symbols are missing.
    pub fn load(path: &Path) -> Result<Self, PluginError> {
        // SAFETY: Library::new loads the shared library. Library handle keeps it loaded.
        let library = unsafe {
            Library::new(path).map_err(|e| {
                PluginError::LoadFailed(format!("Failed to load native plugin: {e}"))
            })?
        };

        // SAFETY: Symbol name is fixed, types match ABI, library kept alive by _library.
        let analyze: AnalyzeFn = unsafe {
            *library
                .get::<AnalyzeFn>(b"crawlkit_plugin_analyze")
                .map_err(|e| PluginError::LoadFailed(format!("Missing analyze symbol: {e}")))?
        };

        // SAFETY: Same as analyze symbol resolution.
        let free: FreeFn = unsafe {
            *library
                .get::<FreeFn>(b"crawlkit_plugin_free")
                .map_err(|e| PluginError::LoadFailed(format!("Missing free symbol: {e}")))?
        };

        Ok(Self {
            _library: library,
            analyze,
            free,
        })
    }

    /// Analyze content using the loaded native plugin.
    ///
    /// # Safety
    ///
    /// This calls into FFI code. The plugin must:
    /// 1. Only read from the input buffer (up to `input_len` bytes)
    /// 2. Return a pointer to a null-terminated UTF-8 string allocated by the host
    /// 3. Not retain references to the input buffer after returning
    pub fn analyze(&self, input: &str) -> Result<String, PluginError> {
        let input_bytes = input.as_bytes();

        // SAFETY: Plugin reads input_ptr..input_ptr+input_len, returns null-terminated UTF-8.
        let result_ptr = unsafe { (self.analyze)(input_bytes.as_ptr(), input_bytes.len()) };

        if result_ptr.is_null() {
            return Err(PluginError::AnalysisFailed(
                "Native plugin returned null pointer".to_string(),
            ));
        }

        // SAFETY: Read null-terminated C string from plugin output, then free.
        let result = unsafe {
            let c_str = std::ffi::CStr::from_ptr(result_ptr.cast::<std::ffi::c_char>());
            let s = c_str
                .to_str()
                .map_err(|e| {
                    PluginError::AnalysisFailed(format!("Invalid UTF-8 from plugin: {e}"))
                })?
                .to_string();
            (self.free)(result_ptr);
            s
        };

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Create a minimal shared library for testing.
    ///
    /// On Linux, we compile a small C file into a .so and load it.
    /// This test is skipped on platforms where we can't easily compile C.
    #[test]
    #[cfg(target_os = "linux")]
    fn test_native_plugin_load_and_analyze() {
        let dir = tempfile::tempdir().expect("Failed to create temp dir");
        let so_path = dir.path().join("libtest_plugin.so");
        let c_path = dir.path().join("plugin.c");

        // Write a minimal C plugin
        let c_source = r#"
#include <stdlib.h>
#include <string.h>

char* crawlkit_plugin_analyze(const unsigned char* input, size_t len) {
    // Simple echo plugin: return a fixed JSON result
    const char* result = "{\"status\":\"ok\",\"analyzer\":\"native-test\"}";
    char* out = (char*)malloc(strlen(result) + 1);
    if (out) {
        strcpy(out, result);
    }
    return out;
}

void crawlkit_plugin_free(char* ptr) {
    free(ptr);
}
"#;

        let mut f = std::fs::File::create(&c_path).expect("Failed to create C file");
        f.write_all(c_source.as_bytes())
            .expect("Failed to write C source");
        drop(f);

        // Compile to shared library
        let output = std::process::Command::new("gcc")
            .args([
                "-shared",
                "-fPIC",
                "-o",
                so_path.to_str().expect("Invalid path"),
                c_path.to_str().expect("Invalid path"),
            ])
            .output()
            .expect("Failed to run gcc");

        if !output.status.success() {
            // gcc not available, skip test
            eprintln!(
                "gcc compilation failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }

        let plugin = NativePlugin::load(&so_path).expect("Failed to load native plugin");
        let result = plugin
            .analyze("<html>test</html>")
            .expect("Analysis failed");
        assert!(result.contains("native-test"));
        assert!(result.contains("ok"));
    }

    #[test]
    fn test_native_plugin_load_nonexistent() {
        let path = Path::new("/nonexistent/libplugin.so");
        let result = NativePlugin::load(path);
        assert!(result.is_err());
        match result.unwrap_err() {
            PluginError::LoadFailed(msg) => {
                assert!(msg.contains("Failed to load native plugin"));
            }
            other => panic!("Expected LoadFailed, got: {other:?}"),
        }
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_native_plugin_missing_symbols() {
        let dir = tempfile::tempdir().expect("Failed to create temp dir");
        let so_path = dir.path().join("libempty_plugin.so");
        let c_path = dir.path().join("empty.c");

        let c_source = r#"
// Empty plugin - no required symbols
int not_the_right_function(void) {
    return 0;
}
"#;

        let mut f = std::fs::File::create(&c_path).expect("Failed to create C file");
        f.write_all(c_source.as_bytes())
            .expect("Failed to write C source");
        drop(f);

        let output = std::process::Command::new("gcc")
            .args([
                "-shared",
                "-fPIC",
                "-o",
                so_path.to_str().expect("Invalid path"),
                c_path.to_str().expect("Invalid path"),
            ])
            .output()
            .expect("Failed to run gcc");

        if !output.status.success() {
            eprintln!(
                "gcc compilation failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }

        let result = NativePlugin::load(&so_path);
        assert!(result.is_err());
        match result.unwrap_err() {
            PluginError::LoadFailed(msg) => {
                assert!(msg.contains("Missing"));
            }
            other => panic!("Expected LoadFailed, got: {other:?}"),
        }
    }
}
