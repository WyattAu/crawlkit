use std::time::Duration;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors from Playwright operations.
#[derive(Debug, Error)]
pub enum PlaywrightError {
    #[error("playwright not available: {0}")]
    NotAvailable(String),

    #[error("page navigation failed: {0}")]
    NavigationFailed(String),

    #[error("page timeout after {0:?}")]
    Timeout(Duration),

    #[error("JavaScript evaluation failed: {0}")]
    JsEvaluationFailed(String),

    #[error("browser launch failed: {0}")]
    BrowserLaunchFailed(String),
}

/// Configuration for Playwright browser rendering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaywrightConfig {
    /// Enable JavaScript rendering.
    pub enabled: bool,
    /// Browser type.
    pub browser_type: BrowserType,
    /// Page load timeout.
    pub timeout: Duration,
    /// Maximum concurrent browser contexts.
    pub max_concurrent: usize,
    /// Headless mode.
    pub headless: bool,
    /// Extra arguments to pass to the browser.
    pub args: Vec<String>,
}

impl Default for PlaywrightConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            browser_type: BrowserType::Chromium,
            timeout: Duration::from_secs(30),
            max_concurrent: 5,
            headless: true,
            args: vec![
                "--no-sandbox".to_string(),
                "--disable-setuid-sandbox".to_string(),
                "--disable-dev-shm-usage".to_string(),
            ],
        }
    }
}

/// Browser type for rendering.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum BrowserType {
    Chromium,
    Firefox,
    WebKit,
}

/// Result of JavaScript rendering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderedPage {
    /// Final URL after JS rendering.
    pub final_url: String,
    /// Rendered HTML content.
    pub html: String,
    /// Console messages from the page.
    pub console_messages: Vec<ConsoleMessage>,
    /// Network requests made during rendering.
    pub network_requests: Vec<NetworkRequest>,
    /// WASM-related errors detected.
    pub wasm_errors: Vec<WasmError>,
    /// Render time.
    pub render_time: Duration,
}

/// Console message from browser.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleMessage {
    pub level: String,
    pub text: String,
    pub source: Option<String>,
    pub line: Option<u32>,
}

/// Network request during rendering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkRequest {
    pub url: String,
    pub method: String,
    pub status: Option<u16>,
    pub resource_type: String,
}

/// WASM error detected during rendering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmError {
    pub error_type: String,
    pub message: String,
    pub source: Option<String>,
    pub timestamp: u64,
}

/// Playwright renderer (placeholder for actual Playwright integration).
///
/// When Playwright is not available, falls back to HTTP-only mode.
pub struct PlaywrightRenderer {
    config: PlaywrightConfig,
}

impl PlaywrightRenderer {
    /// Create a new Playwright renderer.
    #[must_use]
    pub fn new(config: PlaywrightConfig) -> Self {
        Self { config }
    }

    /// Create with default config.
    #[must_use]
    pub fn with_default_config() -> Self {
        Self::new(PlaywrightConfig::default())
    }

    /// Check if Playwright is available.
    #[must_use]
    pub fn is_available(&self) -> bool {
        // In real implementation, check if playwright binary exists
        // For now, always return false (HTTP-only mode)
        false
    }

    /// Render a page with JavaScript.
    ///
    /// # Errors
    /// Returns error if rendering fails.
    pub async fn render(&self, _url: &str) -> Result<RenderedPage, PlaywrightError> {
        if !self.config.enabled {
            return Err(PlaywrightError::NotAvailable(
                "JavaScript rendering is disabled".to_string(),
            ));
        }

        if !self.is_available() {
            return Err(PlaywrightError::NotAvailable(
                "Playwright binary not found".to_string(),
            ));
        }

        // Placeholder: actual implementation would launch browser,
        // navigate to URL, wait for network idle, extract content
        Err(PlaywrightError::NotAvailable(
            "Playwright integration not yet implemented".to_string(),
        ))
    }

    /// Get configuration.
    #[must_use]
    pub fn config(&self) -> &PlaywrightConfig {
        &self.config
    }
}

impl Default for PlaywrightRenderer {
    fn default() -> Self {
        Self::with_default_config()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_playwright_config_default() {
        let config = PlaywrightConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.max_concurrent, 5);
        assert!(config.headless);
    }

    #[test]
    fn test_playwright_renderer_not_available() {
        let renderer = PlaywrightRenderer::with_default_config();
        assert!(!renderer.is_available());
    }

    #[tokio::test]
    async fn test_playwright_render_disabled() {
        let renderer = PlaywrightRenderer::with_default_config();
        let result = renderer.render("https://example.com").await;
        assert!(result.is_err());
    }
}
