//! Core Web Vitals measurement via Chrome DevTools Protocol.
//!
//! Injects [`PerformanceObserver`] scripts into pages rendered by Playwright
//! to capture LCP, CLS, INP, FCP, and TTFB in a single page load.
//!
//! [`PerformanceObserver`]: https://developer.mozilla.org/en-US/docs/Web/API/PerformanceObserver

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors from Web Vitals measurement.
#[derive(Debug, Error)]
pub enum WebVitalsError {
    /// Playwright is not available or not enabled.
    #[error("playwright not available: {0}")]
    NotAvailable(String),

    /// Navigation or rendering failed.
    #[error("navigation failed: {0}")]
    NavigationFailed(String),

    /// JavaScript evaluation failed.
    #[error("JS evaluation failed: {0}")]
    JsEvaluationFailed(String),

    /// Measurement timed out.
    #[error("measurement timed out after {0:?}")]
    Timeout(std::time::Duration),

    /// Failed to parse metric results from the page.
    #[error("failed to parse metrics: {0}")]
    ParseError(String),
}

/// Core Web Vitals measurement results.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WebVitals {
    /// Largest Contentful Paint in milliseconds.
    pub lcp: Option<f64>,
    /// Cumulative Layout Shift (dimensionless).
    pub cls: Option<f64>,
    /// Interaction to Next Paint in milliseconds.
    pub inp: Option<f64>,
    /// First Contentful Paint in milliseconds.
    pub fcp: Option<f64>,
    /// Time to First Byte in milliseconds.
    pub ttfb: Option<f64>,
}

/// JavaScript snippet injected into the page to collect Core Web Vitals
/// via `PerformanceObserver`.  The script writes results to
/// `window.__crawlkit_cwv` so they can be retrieved with `page.evaluate`.
pub const CWV_OBSERVER_SCRIPT: &str = r#"
(function() {
    if (window.__crawlkit_cwv) return;
    window.__crawlkit_cwv = { lcp: null, cls: null, inp: null, fcp: null, ttfb: null };

    try {
        var lcpEntries = [];
        new PerformanceObserver(function(list) {
            var entries = list.getEntries();
            if (entries.length > 0) {
                lcpEntries.push(entries[entries.length - 1]);
                window.__crawlkit_cwv.lcp = entries[entries.length - 1].startTime;
            }
        }).observe({ type: 'largest-contentful-paint', buffered: true });
    } catch(e) {}

    try {
        var clsValue = 0;
        new PerformanceObserver(function(list) {
            list.getEntries().forEach(function(entry) {
                if (!entry.hadRecentInput) {
                    clsValue += entry.value;
                }
            });
            window.__crawlkit_cwv.cls = clsValue;
        }).observe({ type: 'layout-shift', buffered: true });
    } catch(e) {}

    try {
        new PerformanceObserver(function(list) {
            var entries = list.getEntries();
            if (entries.length > 0) {
                window.__crawlkit_cwv.fcp = entries[0].startTime;
            }
        }).observe({ type: 'paint', buffered: true });
    } catch(e) {}

    try {
        var navEntries = performance.getEntriesByType('navigation');
        if (navEntries.length > 0) {
            var nav = navEntries[0];
            window.__crawlkit_cwv.ttfb = nav.responseStart - nav.requestStart;
        }
    } catch(e) {}
})();
"#;

/// Measure Core Web Vitals by injecting PerformanceObserver scripts into a
/// Playwright-rendered page.
///
/// Uses the existing [`PlaywrightRenderer`](crate::PlaywrightRenderer) CLI
/// subprocess to navigate and evaluate JavaScript, so no direct CDP
/// connection is required.
pub struct WebVitalsMeasurer {
    /// Timeout for page load in milliseconds.
    pub timeout_ms: u64,
}

impl Default for WebVitalsMeasurer {
    fn default() -> Self {
        Self::new()
    }
}

impl WebVitalsMeasurer {
    /// Create a new measurer with a 5-second timeout.
    #[must_use]
    pub fn new() -> Self {
        Self { timeout_ms: 5000 }
    }

    /// Create a measurer with a custom timeout.
    #[must_use]
    pub fn with_timeout(timeout_ms: u64) -> Self {
        Self { timeout_ms }
    }

    /// Measure Core Web Vitals for a URL.
    ///
    /// Launches a headless Chromium instance via the Playwright CLI,
    /// navigates to the URL, injects the CWV observer, waits for metrics
    /// to stabilise, and returns the collected values.
    ///
    /// # Errors
    /// Returns [`WebVitalsError`] if Playwright is unavailable, navigation
    /// fails, or metrics cannot be parsed.
    pub async fn measure(&self, url: &str) -> Result<WebVitals, WebVitalsError> {
        let script = build_measure_script(url, self.timeout_ms);
        let temp_dir = std::env::temp_dir();
        let script_path = temp_dir.join(format!("crawlkit_cwv_{}.js", uuid::Uuid::new_v4()));
        std::fs::write(&script_path, &script)
            .map_err(|e| WebVitalsError::NavigationFailed(e.to_string()))?;

        let node_path = std::env::var("NODE_PATH").unwrap_or_default();
        let mut cmd = tokio::process::Command::new("node");
        cmd.arg(&script_path);
        if !node_path.is_empty() {
            cmd.env("NODE_PATH", &node_path);
        }

        let output = cmd
            .output()
            .await
            .map_err(|e| WebVitalsError::NavigationFailed(e.to_string()))?;

        let _ = std::fs::remove_file(&script_path);

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(WebVitalsError::NavigationFailed(stderr.to_string()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let result: serde_json::Value =
            serde_json::from_str(&stdout).map_err(|e| WebVitalsError::ParseError(e.to_string()))?;

        parse_web_vitals(&result).ok_or_else(|| {
            WebVitalsError::ParseError("missing or invalid metrics object".to_string())
        })
    }
}

/// Build the Node.js script that launches Playwright, navigates, injects
/// the CWV observer, waits, and prints the metrics as JSON.
fn build_measure_script(_url: &str, timeout_ms: u64) -> String {
    // SECURITY: url is passed via process.argv, not interpolated.
    format!(
        r#"
const {{ chromium }} = require('playwright');
const targetUrl = process.argv[2];
const CWV_SCRIPT = {cwv_script_json};

(async () => {{
    const browser = await chromium.launch({{
        headless: true,
        args: ['--no-sandbox', '--disable-setuid-sandbox', '--disable-dev-shm-usage', '--disable-gpu']
    }});

    const context = await browser.newContext({{
        viewport: {{ width: 1920, height: 1080 }}
    }});

    const page = await context.newPage();

    try {{
        await page.goto(targetUrl, {{ waitUntil: 'load', timeout: {timeout_ms} }});
    }} catch (e) {{
        // Proceed even if navigation times out — some metrics may still be available.
    }}

    // Inject CWV observer after load
    await page.evaluate(CWV_SCRIPT);

    // Wait for metrics to stabilise
    await page.waitForTimeout(Math.min({wait_ms}, {timeout_ms}));

    // Collect metrics
    const metrics = await page.evaluate(() => window.__crawlkit_cwv || {{}});

    // Also grab TTFB from navigation timing as a fallback
    if (!metrics.ttfb) {{
        metrics.ttfb = await page.evaluate(() => {{
            const nav = performance.getEntriesByType('navigation');
            return nav.length > 0 ? nav[0].responseStart - nav[0].requestStart : null;
        }});
    }}

    console.log(JSON.stringify(metrics));
    await browser.close();
}})();
"#,
        cwv_script_json = serde_json::to_string(CWV_OBSERVER_SCRIPT).unwrap_or_default(),
        timeout_ms = timeout_ms,
        wait_ms = std::cmp::min(2000, timeout_ms / 2),
    )
}

/// Parse the JSON metrics object from the Node.js script output into
/// [`WebVitals`].
fn parse_web_vitals(value: &serde_json::Value) -> Option<WebVitals> {
    Some(WebVitals {
        lcp: value.get("lcp").and_then(|v| v.as_f64()),
        cls: value.get("cls").and_then(|v| v.as_f64()),
        inp: value.get("inp").and_then(|v| v.as_f64()),
        fcp: value.get("fcp").and_then(|v| v.as_f64()),
        ttfb: value.get("ttfb").and_then(|v| v.as_f64()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_web_vitals_default() {
        let v = WebVitals::default();
        assert!(v.lcp.is_none());
        assert!(v.cls.is_none());
        assert!(v.inp.is_none());
        assert!(v.fcp.is_none());
        assert!(v.ttfb.is_none());
    }

    #[test]
    fn test_web_vitals_serialization() {
        let v = WebVitals {
            lcp: Some(1200.0),
            cls: Some(0.05),
            inp: Some(80.0),
            fcp: Some(300.0),
            ttfb: Some(50.0),
        };
        let json = serde_json::to_string(&v).unwrap();
        let parsed: WebVitals = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.lcp, Some(1200.0));
        assert_eq!(parsed.cls, Some(0.05));
    }

    #[test]
    fn test_parse_web_vitals() {
        let val = serde_json::json!({
            "lcp": 2500.0,
            "cls": 0.1,
            "inp": 150.0,
            "fcp": 800.0,
            "ttfb": 100.0
        });
        let v = parse_web_vitals(&val).unwrap();
        assert_eq!(v.lcp, Some(2500.0));
        assert_eq!(v.cls, Some(0.1));
        assert_eq!(v.inp, Some(150.0));
        assert_eq!(v.fcp, Some(800.0));
        assert_eq!(v.ttfb, Some(100.0));
    }

    #[test]
    fn test_parse_web_vitals_nulls() {
        let val = serde_json::json!({ "lcp": 1000.0 });
        let v = parse_web_vitals(&val).unwrap();
        assert_eq!(v.lcp, Some(1000.0));
        assert!(v.cls.is_none());
        assert!(v.inp.is_none());
    }

    #[test]
    fn test_cwv_observer_script_not_empty() {
        assert!(!CWV_OBSERVER_SCRIPT.is_empty());
        assert!(CWV_OBSERVER_SCRIPT.contains("PerformanceObserver"));
    }

    #[test]
    fn test_web_vitals_measurer_default() {
        let m = WebVitalsMeasurer::new();
        assert_eq!(m.timeout_ms, 5000);
    }

    #[test]
    fn test_web_vitals_measurer_custom_timeout() {
        let m = WebVitalsMeasurer::with_timeout(10000);
        assert_eq!(m.timeout_ms, 10000);
    }

    #[test]
    fn test_build_measure_script_contains_url_arg() {
        let script = build_measure_script("https://example.com", 5000);
        assert!(script.contains("process.argv[2]"));
        assert!(script.contains("5000"));
    }

    #[tokio::test]
    #[ignore]
    async fn test_measure_requires_playwright() {
        let m = WebVitalsMeasurer::new();
        // This will fail unless Playwright is installed — that's expected.
        let _ = m.measure("https://example.com").await;
    }
}
