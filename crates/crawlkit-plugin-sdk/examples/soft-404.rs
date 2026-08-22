//! Example crawlkit plugin: soft-404 detector using the host context API.
//!
//! Demonstrates `crawlkit_plugin_sdk::host` (B4): reads structured
//! response metadata via `crawlkit_host.get_context` instead of inferring
//! anything from raw HTML, and flags 4xx/5xx pages that were still
//! analyzed.

use crawlkit_plugin_sdk::host::{self, HostContext};
use crawlkit_plugin_sdk::{AnalysisContext, Analyzer, Finding, Severity};

pub struct Soft404Detector;

impl Soft404Detector {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Soft404Detector {
    fn default() -> Self {
        Self::new()
    }
}

fn findings_for(ctx: &HostContext) -> Vec<Finding> {
    let (code, title) = match ctx.status_code {
        Some(code @ 400..=599) => (code, "Error page analyzed"),
        Some(301) | Some(302) | Some(308) => (301, "Redirect analyzed as content"),
        _ => return Vec::new(),
    };
    vec![Finding {
        severity: Severity::Warning,
        category: "seo".into(),
        code: "SOFT404".into(),
        title: title.into(),
        description: format!(
            "This URL returned HTTP {code}; the page was still analyzed. \
             Error responses reachable from internal links are soft-404s."
        ),
        url: ctx.url.clone(),
        recommendation: "Fix internal links pointing at error responses.".into(),
    }]
}

impl Analyzer for Soft404Detector {
    fn name(&self) -> &str {
        "soft-404"
    }

    fn analyze(&self, _ctx: &AnalysisContext) -> Vec<Finding> {
        match host::context() {
            Some(Ok(host_ctx)) => findings_for(&host_ctx),
            // No context available (plain analyze): nothing to check.
            Some(Err(e)) => {
                let _ = e; // version skew; degrade silently
                Vec::new()
            }
            None => Vec::new(),
        }
    }
}

crawlkit_plugin_sdk::export_analyzer!(Soft404Detector);

fn main() {
    // WASM library target; the exported ABI symbols are the entry points.
}
