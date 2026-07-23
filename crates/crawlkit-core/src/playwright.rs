use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::RwLock;
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

    #[error("context creation failed: {0}")]
    ContextCreationFailed(String),

    #[error("resource limit exceeded: {0}")]
    ResourceLimitExceeded(String),
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
    /// Maximum memory per context (bytes).
    pub max_memory_per_context: u64,
    /// Maximum CPU time per page (seconds).
    pub max_cpu_seconds: u64,
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
                "--disable-gpu".to_string(),
                "--disable-extensions".to_string(),
                "--disable-background-networking".to_string(),
            ],
            max_memory_per_context: 512 * 1024 * 1024, // 512 MB
            max_cpu_seconds: 30,
        }
    }
}

/// Browser type for rendering.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BrowserType {
    Chromium,
    Firefox,
    WebKit,
}

impl BrowserType {
    /// Get the Playwright browser name.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            BrowserType::Chromium => "chromium",
            BrowserType::Firefox => "firefox",
            BrowserType::WebKit => "webkit",
        }
    }
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
    /// Memory used during render (bytes).
    pub memory_used: u64,
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
    pub size: Option<u64>,
}

/// WASM error detected during rendering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmError {
    pub error_type: String,
    pub message: String,
    pub source: Option<String>,
    pub timestamp: u64,
}

/// Browser context with resource isolation.
#[derive(Clone)]
pub struct BrowserContext {
    /// Context ID.
    pub id: String,
    /// Memory limit.
    pub memory_limit: u64,
    /// CPU time limit.
    pub cpu_limit: Duration,
    /// Creation time.
    pub created_at: Instant,
    /// Whether context is active.
    pub active: bool,
}

impl BrowserContext {
    /// Create a new browser context.
    #[must_use]
    pub fn new(id: String, memory_limit: u64, cpu_limit: Duration) -> Self {
        Self {
            id,
            memory_limit,
            cpu_limit,
            created_at: Instant::now(),
            active: true,
        }
    }

    /// Check if context has exceeded resource limits.
    #[must_use]
    pub fn is_over_limit(&self, memory_used: u64) -> bool {
        let elapsed = self.created_at.elapsed();
        memory_used > self.memory_limit || elapsed > self.cpu_limit
    }
}

/// Playwright binary detection and management.
pub struct PlaywrightDetector {
    /// Path to Playwright binary.
    binary_path: Option<PathBuf>,
    /// Playwright version.
    version: Option<String>,
    /// Available browsers.
    available_browsers: Vec<BrowserType>,
}

impl PlaywrightDetector {
    /// Detect Playwright installation.
    #[must_use]
    pub fn detect() -> Self {
        let binary_path = Self::find_binary();
        let version = binary_path.as_ref().and_then(Self::get_version);
        let available_browsers = binary_path
            .as_ref()
            .map(Self::get_browsers)
            .unwrap_or_default();

        Self {
            binary_path,
            version,
            available_browsers,
        }
    }

    /// Find Playwright binary in PATH.
    fn find_binary() -> Option<PathBuf> {
        // Check common locations
        let candidates = [
            "npx playwright",
            "playwright",
            "/usr/local/bin/playwright",
            "/usr/bin/playwright",
        ];

        for candidate in &candidates {
            if let Ok(output) = Command::new("which").arg(candidate).output() {
                if output.status.success() {
                    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !path.is_empty() {
                        return Some(PathBuf::from(path));
                    }
                }
            }
        }

        None
    }

    /// Get Playwright version.
    fn get_version(binary: &PathBuf) -> Option<String> {
        Command::new(binary)
            .arg("--version")
            .output()
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    String::from_utf8(o.stdout)
                        .ok()
                        .map(|s| s.trim().to_string())
                } else {
                    None
                }
            })
    }

    /// Get available browsers.
    fn get_browsers(binary: &PathBuf) -> Vec<BrowserType> {
        Command::new(binary)
            .arg("install")
            .arg("--dry-run")
            .output()
            .ok()
            .map(|o| {
                let output = String::from_utf8_lossy(&o.stdout);
                let mut browsers = Vec::new();
                if output.contains("chromium") {
                    browsers.push(BrowserType::Chromium);
                }
                if output.contains("firefox") {
                    browsers.push(BrowserType::Firefox);
                }
                if output.contains("webkit") {
                    browsers.push(BrowserType::WebKit);
                }
                browsers
            })
            .unwrap_or_default()
    }

    /// Check if Playwright is available.
    #[must_use]
    pub fn is_available(&self) -> bool {
        self.binary_path.is_some()
    }

    /// Get binary path.
    #[must_use]
    pub fn binary_path(&self) -> Option<&PathBuf> {
        self.binary_path.as_ref()
    }

    /// Get version.
    #[must_use]
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    /// Check if browser is available.
    #[must_use]
    pub fn has_browser(&self, browser: BrowserType) -> bool {
        self.available_browsers.contains(&browser)
    }
}

/// Playwright renderer with browser context isolation.
pub struct PlaywrightRenderer {
    config: PlaywrightConfig,
    detector: PlaywrightDetector,
    contexts: Arc<RwLock<Vec<BrowserContext>>>,
}

impl PlaywrightRenderer {
    /// Create a new Playwright renderer.
    #[must_use]
    pub fn new(config: PlaywrightConfig) -> Self {
        let detector = PlaywrightDetector::detect();
        Self {
            config,
            detector,
            contexts: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Create with default config.
    #[must_use]
    pub fn with_default_config() -> Self {
        Self::new(PlaywrightConfig::default())
    }

    /// Check if Playwright is available.
    #[must_use]
    pub fn is_available(&self) -> bool {
        self.config.enabled && self.detector.is_available()
    }

    /// Get detector reference.
    #[must_use]
    pub fn detector(&self) -> &PlaywrightDetector {
        &self.detector
    }

    /// Create a new browser context with resource isolation.
    ///
    /// # Errors
    /// Returns error if context creation fails.
    pub fn create_context(&self) -> Result<BrowserContext, PlaywrightError> {
        if !self.is_available() {
            return Err(PlaywrightError::NotAvailable(
                "Playwright not available".to_string(),
            ));
        }

        let contexts = self.contexts.read();
        if contexts.len() >= self.config.max_concurrent {
            return Err(PlaywrightError::ResourceLimitExceeded(
                "Maximum concurrent contexts reached".to_string(),
            ));
        }

        let context = BrowserContext::new(
            uuid::Uuid::new_v4().to_string(),
            self.config.max_memory_per_context,
            Duration::from_secs(self.config.max_cpu_seconds),
        );

        drop(contexts);
        self.contexts.write().push(context.clone());

        Ok(context)
    }

    /// Render a page with JavaScript.
    ///
    /// # Errors
    /// Returns error if rendering fails.
    pub async fn render(&self, url: &str) -> Result<RenderedPage, PlaywrightError> {
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

        let start = Instant::now();

        // Create isolated context
        let context = self.create_context()?;

        // Launch browser and render page via Playwright CLI
        let rendered = self.render_via_cli(url).await?;

        let render_time = start.elapsed();

        // Remove context
        self.contexts.write().retain(|c| c.id != context.id);

        Ok(RenderedPage {
            render_time,
            ..rendered
        })
    }

    /// Render page via Playwright CLI subprocess.
    async fn render_via_cli(&self, url: &str) -> Result<RenderedPage, PlaywrightError> {
        let _binary = self.detector.binary_path().ok_or_else(|| {
            PlaywrightError::NotAvailable("Playwright binary not found".to_string())
        })?;

        // Create a temporary script for Playwright execution
        let script = format!(
            r#"
const {{ chromium }} = require('playwright');

(async () => {{
    const browser = await chromium.launch({{
        headless: {},
        args: {}
    }});
    
    const context = await browser.newContext({{
        viewport: {{ width: 1920, height: 1080 }},
        userAgent: 'crawlkit/0.4.0'
    }});
    
    const page = await context.newPage();
    
    const consoleMessages = [];
    const networkRequests = [];
    
    page.on('console', msg => {{
        consoleMessages.push({{
            level: msg.type(),
            text: msg.text(),
            source: msg.location().url || null,
            line: msg.location().lineNumber || null
        }});
    }});
    
    page.on('request', req => {{
        networkRequests.push({{
            url: req.url(),
            method: req.method(),
            status: null,
            resourceType: req.resourceType(),
            size: null
        }});
    }});
    
    page.on('response', res => {{
        const req = networkRequests.find(r => r.url === res.url());
        if (req) {{
            req.status = res.status();
        }}
    }});
    
    try {{
        await page.goto('{}', {{ waitUntil: 'networkidle', timeout: {} }});
    }} catch (e) {{
        console.error('Navigation error:', e.message);
    }}
    
    const html = await page.content();
    const finalUrl = page.url();
    
    // Detect WASM errors
    const wasmErrors = [];
    for (const msg of consoleMessages) {{
        if (msg.level === 'error' && 
            (msg.text.toLowerCase().includes('webassembly') || 
             msg.text.toLowerCase().includes('wasm'))) {{
            wasmErrors.push({{
                error_type: 'runtime',
                message: msg.text,
                source: msg.source,
                timestamp: Date.now()
            }});
        }}
    }}
    
    const result = {{
        final_url: finalUrl,
        html: html,
        console_messages: consoleMessages,
        network_requests: networkRequests,
        wasm_errors: wasmErrors,
        memory_used: process.memoryUsage().heapUsed
    }};
    
    console.log(JSON.stringify(result));
    
    await browser.close();
}})();
"#,
            self.config.headless,
            serde_json::to_string(&self.config.args).unwrap_or_else(|_| "[]".to_string()),
            url,
            self.config.timeout.as_millis()
        );

        // Write script to temporary file
        let temp_dir = std::env::temp_dir();
        let script_path = temp_dir.join(format!("crawlkit_playwright_{}.js", uuid::Uuid::new_v4()));
        std::fs::write(&script_path, &script)
            .map_err(|e| PlaywrightError::BrowserLaunchFailed(e.to_string()))?;

        // Execute Playwright script. Allow NODE_PATH override via environment variable.
        let node_path = std::env::var("NODE_PATH").unwrap_or_default();
        let mut cmd = tokio::process::Command::new("node");
        cmd.arg(&script_path);
        if !node_path.is_empty() {
            cmd.env("NODE_PATH", &node_path);
        }
        let output = cmd
            .output()
            .await
            .map_err(|e| PlaywrightError::BrowserLaunchFailed(e.to_string()))?;

        // Clean up temporary script
        let _ = std::fs::remove_file(&script_path);

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(PlaywrightError::BrowserLaunchFailed(stderr.to_string()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let result: serde_json::Value = serde_json::from_str(&stdout)
            .map_err(|e| PlaywrightError::JsEvaluationFailed(e.to_string()))?;

        Ok(RenderedPage {
            final_url: result["final_url"].as_str().unwrap_or(url).to_string(),
            html: result["html"].as_str().unwrap_or("").to_string(),
            console_messages: serde_json::from_value(result["console_messages"].clone())
                .unwrap_or_default(),
            network_requests: serde_json::from_value(result["network_requests"].clone())
                .unwrap_or_default(),
            wasm_errors: serde_json::from_value(result["wasm_errors"].clone()).unwrap_or_default(),
            render_time: Duration::from_millis(0), // Will be set by caller
            memory_used: result["memory_used"].as_u64().unwrap_or(0),
        })
    }

    /// Get active context count.
    #[must_use]
    pub fn active_contexts(&self) -> usize {
        self.contexts.read().len()
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
        assert_eq!(config.max_memory_per_context, 512 * 1024 * 1024);
    }

    #[test]
    fn test_browser_type_as_str() {
        assert_eq!(BrowserType::Chromium.as_str(), "chromium");
        assert_eq!(BrowserType::Firefox.as_str(), "firefox");
        assert_eq!(BrowserType::WebKit.as_str(), "webkit");
    }

    #[test]
    fn test_playwright_detector() {
        let detector = PlaywrightDetector::detect();
        // May or may not be available depending on environment
        let _ = detector.is_available();
    }

    #[test]
    fn test_browser_context_resource_limits() {
        let context = BrowserContext::new(
            "test".to_string(),
            1024 * 1024, // 1 MB
            Duration::from_secs(10),
        );

        assert!(!context.is_over_limit(512 * 1024)); // 512 KB - OK
        assert!(context.is_over_limit(2 * 1024 * 1024)); // 2 MB - Over limit
    }

    #[tokio::test]
    async fn test_playwright_render_disabled() {
        let renderer = PlaywrightRenderer::with_default_config();
        let result = renderer.render("https://example.com").await;
        assert!(result.is_err());
    }

    #[test]
    fn test_playwright_renderer_not_available() {
        let renderer = PlaywrightRenderer::with_default_config();
        assert!(!renderer.is_available());
    }
}
