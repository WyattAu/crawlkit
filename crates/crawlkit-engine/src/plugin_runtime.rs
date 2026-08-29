//! WASM plugin execution during crawls.
//!
//! Loads installed plugins (the directory layout produced by
//! [`crate::plugin_index::install_plugin`]) once per crawl and runs them
//! against every fetched page alongside the built-in analyzers. Plugin
//! findings are converted into engine [`Finding`]s with a `custom:`
//! category namespace so they store/export identically.
//!
//! Failure semantics: a plugin that fails to load is skipped with a
//! logged error (crawl proceeds); a plugin that fails at runtime on a
//! page contributes no findings for that page (the crawl never aborts on
//! plugin errors — they are third-party code running in a sandbox).

use std::path::{Path, PathBuf};

use parking_lot::Mutex;

use crate::analyzers::Finding;
use crate::plugin::{PluginError, PluginInstance, WasmConfig};
use crate::types::IssueCategory;

/// A plugin loaded and ready to execute during a crawl.
pub struct CrawlPlugin {
    pub name: String,
    plugin: Mutex<PluginInstance>,
}

impl CrawlPlugin {
    /// Analyze one page, returning engine findings (empty on error).
    ///
    /// Errors are logged and swallowed by design: plugin failures must
    /// never abort a crawl.
    pub fn analyze(&self, html: &str, url: &str, context_json: Option<&str>) -> Vec<Finding> {
        let mut plugin = self.plugin.lock();
        match plugin.analyze(html, url, context_json) {
            Ok(json) => parse_plugin_findings(&json),
            Err(e) => {
                tracing::warn!(plugin = %self.name, error = %e, "plugin analysis failed");
                Vec::new()
            }
        }
    }
}

/// Load every valid plugin under `dir` (one subdirectory per plugin, the
/// layout `install_plugin` produces). Invalid plugins are skipped with
/// logged errors; an empty/missing directory yields no plugins.
#[must_use]
pub fn load_plugins_from_dir(dir: &Path, config: &WasmConfig) -> Vec<CrawlPlugin> {
    let mut out = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            tracing::info!(dir = %dir.display(), error = %e, "no plugin directory; plugin execution disabled");
            return out;
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.join("crawlkit-plugin.toml").exists() {
            continue;
        }
        match crate::plugin::load_plugin_from_dir(&path, config) {
            Ok(instance) => {
                let name = instance.metadata().name.clone();
                tracing::info!(plugin = %name, "loaded crawl plugin");
                out.push(CrawlPlugin {
                    name,
                    plugin: Mutex::new(instance),
                });
            }
            Err(e) => {
                tracing::warn!(dir = %path.display(), error = %e, "skipping unloadable plugin");
            }
        }
    }
    out
}

/// Convert a plugin's JSON findings payload into engine findings.
/// Malformed entries are skipped; an invalid payload as a whole yields
/// an empty vec (never panics on third-party output).
#[must_use]
pub fn parse_plugin_findings(json: &str) -> Vec<Finding> {
    let parsed: Vec<Finding> = match serde_json::from_str(json) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(error = %e, "plugin returned malformed findings JSON");
            return Vec::new();
        }
    };
    parsed
        .into_iter()
        .map(|f| Finding {
            category: IssueCategory::Custom(format!("plugin:{}", f.category.as_str())),
            ..f
        })
        .collect()
}

/// Build the B4 host-context JSON for a page being analyzed.
#[must_use]
pub fn build_context_json(
    url: &str,
    status_code: Option<u16>,
    headers: &[(String, String)],
    response_time_ms: Option<u64>,
    parsed: Option<&crate::ParsedPage>,
) -> String {
    use serde_json::json;
    let parsed_json = parsed.map(|p| {
        json!({
            "title": p.meta.title,
            "description": p.meta.description,
            "canonical": p.meta.canonical.as_ref().map(|u| u.to_string()),
            "word_count": p.word_count,
            "sentence_count": p.sentence_count,
            "headings": p.headings.iter().map(|h| json!({
                "level": h.level,
                "text": h.text,
            })).collect::<Vec<_>>(),
            "link_count": p.links.len(),
            "image_count": p.images.len(),
            "lang": p.html_lang,
        })
    });
    json!({
        "url": url,
        "status_code": status_code,
        "response_time_ms": response_time_ms,
        "headers": headers,
        "parsed": parsed_json,
    })
    .to_string()
}

/// Errors surfaced by crawl-plugin loading (unused variants reserved).
#[derive(Debug, thiserror::Error)]
pub enum PluginRuntimeError {
    #[error("plugin error: {0}")]
    Plugin(#[from] PluginError),
}

/// Default plugin install roots checked by the CLI: `~/.crawlkit/plugins`
/// plus every directory in `CRAWLKIT_PLUGIN_DIRS` (colon-separated).
#[must_use]
pub fn default_plugin_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(home) = dirs_home() {
        dirs.push(home.join(".crawlkit").join("plugins"));
    }
    if let Ok(extra) = std::env::var("CRAWLKIT_PLUGIN_DIRS") {
        for part in extra.split(':').filter(|s| !s.is_empty()) {
            dirs.push(PathBuf::from(part));
        }
    }
    dirs
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
}
