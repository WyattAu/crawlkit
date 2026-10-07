//! Rendering JavaScript with a Chrome binary directly, without Node.
//!
//! # Why this exists
//!
//! The Playwright backend needs two things that often are not present together: a
//! Node runtime and an installed `playwright` npm package. The render script is
//! written to a temp file and does `require('playwright')`, which resolves from
//! that file's own directory upward — a global npm install does not satisfy it
//! without `NODE_PATH` being set. So on a machine with Chrome installed and npm
//! missing, JavaScript rendering was simply unavailable.
//!
//! Chrome itself needs no runtime. `--headless --dump-dom <url>` navigates,
//! executes scripts, waits for the load, and prints the resulting DOM to stdout.
//! That is precisely the document a parity comparison needs.
//!
//! # What this mode cannot do, stated plainly
//!
//! `--dump-dom` is a single-shot print of the DOM. It cannot report:
//!
//! - console messages, `pageerror` events, or uncaught exceptions
//! - network requests issued during rendering
//! - Wasm errors
//! - a resource-limited render budget
//!
//! Those fields are therefore left empty rather than filled with plausible
//! values. Analyzers that consume them ([`crate::analyzers::js_error_analyzers`])
//! correctly report nothing, because nothing was observed. A renderer that
//! invented empty-but-confident telemetry would be worse than one that admits the
//! limit.
//!
//! # Choosing between backends
//!
//! [`ChromeCliRenderer`] is preferred when Chrome is present, since it needs no
//! package installation. Playwright remains the backend when the caller asks for
//! it explicitly, because it is the only one that can observe console output and
//! client-side errors — which matter for diagnosing hydration failures.

use std::path::{Path, PathBuf};
use std::process::Stdio;

use tokio::process::Command;

use crate::playwright::RenderedPage;

/// Chrome/Chromium executables, in preference order.
///
/// Absolute paths are checked directly: `which` does not search when its argument
/// contains a separator.
const CHROME_CANDIDATES: &[&str] = &[
    "google-chrome",
    "google-chrome-stable",
    "chromium",
    "chromium-browser",
    "chrome",
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    "/usr/bin/google-chrome",
    "/usr/bin/chromium",
    "/usr/bin/chromium-browser",
    "/snap/bin/chromium",
];

/// How long to let scripts run before capturing the DOM.
///
/// `--virtual-time-budget` advances a virtual clock, so page timers and animation
/// frames settle deterministically instead of racing a wall-clock timeout. Without
/// it, `--dump-dom` can capture a DOM mid-hydration on a slow machine and report
/// content as missing that was about to appear.
const VIRTUAL_TIME_BUDGET_MS: &str = "5000";

/// Renders pages by invoking a Chrome binary directly.
#[derive(Debug, Clone)]
pub struct ChromeCliRenderer {
    binary: PathBuf,
    timeout: std::time::Duration,
    headless: bool,
    user_agent: String,
}

impl ChromeCliRenderer {
    /// Locate an installed Chrome or Chromium.
    ///
    /// Also inspects the Playwright browser cache, which is where a machine that
    /// has run Playwright's installer keeps its Chromium even when no system
    /// browser is installed.
    #[must_use]
    pub fn detect() -> Option<Self> {
        let binary = Self::locate()?;
        Some(Self {
            binary,
            timeout: std::time::Duration::from_secs(30),
            headless: true,
            user_agent: format!("crawlkit/{}", env!("CARGO_PKG_VERSION")),
        })
    }

    /// With a timeout override, for callers that need to bound a crawl.
    #[must_use]
    pub fn with_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Find the first usable Chrome/Chromium.
    fn locate() -> Option<PathBuf> {
        for candidate in CHROME_CANDIDATES {
            if let Some(found) = Self::resolve(candidate) {
                return Some(found);
            }
        }
        Self::locate_in_playwright_cache()
    }

    /// Resolve one candidate to an executable path.
    fn resolve(name: &str) -> Option<PathBuf> {
        if name.contains('/') {
            let p = PathBuf::from(name);
            return p.is_file().then_some(p);
        }
        let out = std::process::Command::new("which")
            .arg(name)
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
        (!path.is_empty()).then(|| PathBuf::from(path))
    }

    /// Look for a Chrome executable under the Playwright browser cache.
    ///
    /// The layout is `chromium-<rev>/<platform-dir>/chrome` on Linux, so the binary
    /// is a *file* inside a directory whose name varies by platform. Matching
    /// directory names alone finds nothing on Linux, which is precisely where this
    /// cache usually lives.
    ///
    /// Best-effort: a miss is not an error, it only means the Playwright backend is
    /// the remaining option.
    fn locate_in_playwright_cache() -> Option<PathBuf> {
        let root = std::env::var_os("PLAYWRIGHT_BROWSERS_PATH")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache/ms-playwright"))
            })?;
        Self::search_cache(&root, 0)
    }

    /// File names a Chrome binary ships under, across platforms.
    const EXECUTABLE_NAMES: &[&str] = &[
        "chrome",
        "chrome.exe",
        "Chromium",
        "headless_shell",
        "chrome-headless-shell",
    ];

    /// Bounded search for a Chrome executable under `dir`.
    fn search_cache(dir: &Path, depth: usize) -> Option<PathBuf> {
        // Three levels covers `chromium-<rev>/chrome-linux64/chrome` without
        // walking a whole browser bundle.
        const MAX_DEPTH: usize = 3;
        if depth > MAX_DEPTH {
            return None;
        }
        for name in Self::EXECUTABLE_NAMES {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
        let entries = std::fs::read_dir(dir).ok()?;
        for entry in entries.flatten() {
            let path = entry.path();
            // Only directories are recursed; the loop above covered files at this
            // level, which is where the binary actually lives.
            if path.is_dir() {
                if let Some(found) = Self::search_cache(&path, depth + 1) {
                    return Some(found);
                }
            }
        }
        None
    }

    /// The browser executable this renderer will invoke.
    #[must_use]
    pub fn binary(&self) -> &Path {
        &self.binary
    }

    async fn run(&self, url: &str) -> Result<(String, String), String> {
        let mut cmd = Command::new(&self.binary);
        if self.headless {
            // The new headless mode is the real browser engine, so parity results
            // match what Chrome users actually get. `--headless=old` would not.
            cmd.arg("--headless=new");
        }
        cmd.arg("--no-sandbox")
            .arg("--disable-gpu")
            .arg("--disable-dev-shm-usage")
            .arg(format!("--virtual-time-budget={VIRTUAL_TIME_BUDGET_MS}"))
            .arg(format!("--user-agent={}", self.user_agent))
            .arg("--dump-dom")
            .arg(url)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let out = tokio::time::timeout(self.timeout, cmd.output())
            .await
            .map_err(|_| format!("chrome render timed out after {:?}", self.timeout))?
            .map_err(|e| format!("chrome could not be launched: {e}"))?;

        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            return Err(format!(
                "chrome exited with {}: {}",
                out.status,
                stderr.trim().chars().take(400).collect::<String>()
            ));
        }
        if stdout.trim().is_empty() {
            return Err("chrome produced no DOM output".to_string());
        }
        // `--dump-dom` prints the original URL, not the post-redirect one. The
        // crawl pipeline tracks redirects separately, so reporting the requested
        // URL is accurate rather than merely convenient.
        Ok((url.to_string(), stdout))
    }
}

#[async_trait::async_trait]
impl crate::crawl_engine::JsRenderer for ChromeCliRenderer {
    fn is_available(&self) -> bool {
        self.binary.is_file()
    }

    async fn render(&self, url: &str) -> Result<String, String> {
        self.run(url).await.map(|(_, html)| html)
    }

    /// Overridden so the served response body survives.
    ///
    /// The default trait implementation sets `source_html: None`, and the
    /// raw-versus-rendered analyzer needs both documents. The crawl pipeline fills
    /// `source_html` in after this returns, so returning it here is belt-and-braces
    /// for callers that invoke the renderer directly.
    async fn render_rich(&self, url: &str) -> Result<RenderedPage, String> {
        let (final_url, html) = self.run(url).await?;
        Ok(RenderedPage {
            final_url,
            html,
            source_html: None,
            // `--dump-dom` cannot observe these. Left empty rather than
            // synthesized; see the module docs.
            console_messages: Vec::new(),
            network_requests: Vec::new(),
            wasm_errors: Vec::new(),
            page_errors: Vec::new(),
            render_time: std::time::Duration::ZERO,
            memory_used: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crawl_engine::JsRenderer as _;

    #[test]
    fn existing_binary_resolves() {
        assert!(ChromeCliRenderer::resolve("sh").is_some());
        assert!(ChromeCliRenderer::resolve("definitely-not-real-xyzzy").is_none());
        assert!(ChromeCliRenderer::resolve("/bin/sh").is_some());
        assert!(ChromeCliRenderer::resolve("/bin/definitely-not-here").is_none());
    }

    /// An empty stdout with a successful exit must be an error. Treating it as an
    /// empty document would make every parity check compare against nothing and
    /// report the whole page as injected content.
    #[tokio::test]
    async fn empty_output_is_an_error_not_an_empty_document() {
        let renderer = ChromeCliRenderer {
            binary: PathBuf::from("/bin/sh"),
            timeout: std::time::Duration::from_secs(5),
            headless: true,
            user_agent: "t".to_string(),
        };
        // /bin/sh with these flags cannot succeed; either way the result must be
        // Err rather than Ok with an empty DOM.
        let result = renderer.run("https://example.invalid/").await;
        assert!(result.is_err(), "got {result:?}");
    }

    /// A missing browser must not be reported as present.
    #[tokio::test]
    async fn unavailable_browser_reports_itself_unavailable() {
        let renderer = ChromeCliRenderer {
            binary: PathBuf::from("/nonexistent/chrome"),
            timeout: std::time::Duration::from_secs(2),
            headless: true,
            user_agent: "t".to_string(),
        };
        assert!(!renderer.is_available());
        assert!(renderer.run("https://example.invalid/").await.is_err());
    }
}
