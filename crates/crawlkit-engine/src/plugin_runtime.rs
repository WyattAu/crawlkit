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

#[cfg(test)]
mod dedupe_tests {
    use crate::analyzers::dedupe::collapse_duplicates;
    use crate::types::{IssueCategory, Severity};
    use crate::Finding;

    fn f(code: &str, title: &str, severity: Severity) -> Finding {
        Finding {
            severity,
            category: IssueCategory::Seo,
            code: code.to_string(),
            title: title.to_string(),
            description: "d".to_string(),
            url: "https://example.com/p".to_string(),
            recommendation: "r".to_string(),
        }
    }

    /// Plugin output must collapse against the built-in family, not only against
    /// itself.
    ///
    /// `AnalyzerRegistry::analyze` deduplicates before plugins run, and
    /// `crawl_engine::pipeline` used to append plugin findings afterwards, so a
    /// plugin reporting an already-covered defect produced a second row that no
    /// aggregate collapsed.
    #[test]
    fn plugin_finding_collapses_with_builtin_family_member() {
        let findings = vec![
            f("A11Y004", "Multiple H1 headings", Severity::Error),
            f("HEADING-MULTIH1", "Multiple H1 headings", Severity::Warning),
        ];
        let out = collapse_duplicates(findings);
        assert_eq!(
            out.len(),
            1,
            "a plugin reporting a built-in defect must not add a second row: {:?}",
            out.iter().map(|x| &x.code).collect::<Vec<_>>()
        );
    }

    /// A plugin must not be able to absorb an unrelated built-in defect.
    ///
    /// This is the false negative the first-party plugins were causing.
    /// `meta-description-checker` emitted `META002` for *"Meta description too
    /// short"*, but `META002` is registered in the **title** family ("title too
    /// short"). `defect_key` resolves by code, so the description finding was
    /// keyed as a title defect and merged into the built-in `TITLE001` — silently
    /// dropping a real description problem on any page that also had a short
    /// title. Codes now say what they mean, so the two stay distinct.
    #[test]
    fn description_and_title_defects_do_not_collapse_into_one() {
        let findings = vec![
            f("TITLE001", "Title too short", Severity::Warning),
            f(
                "METADESC-SHORT",
                "Meta description too short",
                Severity::Warning,
            ),
        ];
        let out = collapse_duplicates(findings);
        assert_eq!(
            out.len(),
            2,
            "a short title and a short description are two defects: {:?}",
            out.iter().map(|x| &x.code).collect::<Vec<_>>()
        );
    }

    /// The specific regression: with the plugin's old code, one of these two
    /// vanished. Pinned so re-using a built-in code in a plugin fails loudly.
    #[test]
    fn reusing_a_builtin_code_would_absorb_an_unrelated_defect() {
        // `META002` resolves to the title family regardless of the title passed.
        let key_desc = crate::analyzers::defect_family::defect_key(
            "META002",
            &crate::analyzers::dedupe::defect_signature("Meta description too short"),
        );
        let key_title = crate::analyzers::defect_family::defect_key(
            "TITLE001",
            &crate::analyzers::dedupe::defect_signature("Title too short"),
        );
        assert_eq!(
            key_desc, key_title,
            "if these ever diverge, `META002` is no longer a title-family code and this test should be revisited"
        );
    }

    /// A plugin reporting the same code once per element is left alone: several
    /// analyzers legitimately report one finding per offending element, and those
    /// are distinct defects.
    #[test]
    fn same_code_repeats_from_a_plugin_are_not_collapsed() {
        let findings = vec![
            f("MYP001", "Widget alignment is off", Severity::Warning),
            f("MYP001", "Widget alignment is off", Severity::Warning),
        ];
        assert_eq!(collapse_duplicates(findings).len(), 2);
    }

    /// A plugin using a *different* code for a genuinely different defect is
    /// untouched — the collapse is keyed on defect identity, not on "came from a
    /// plugin".
    #[test]
    fn unrelated_plugin_finding_survives() {
        let findings = vec![
            f("A11Y004", "Multiple H1 headings", Severity::Error),
            f(
                "MYP001",
                "Widget alignment is off by 3px",
                Severity::Warning,
            ),
        ];
        assert_eq!(collapse_duplicates(findings).len(), 2);
    }
}
