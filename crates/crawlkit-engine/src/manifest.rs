//! Machine-readable capability manifest (Phase 0.1).
//!
//! Generates the project manifest required by `ROADMAP.md` §0.1: analyzer
//! and finding-code counts derived from the live [`AnalyzerRegistry`] rather
//! than hand-copied into documentation. The CI drift check compares the
//! generated TOML against the committed `docs/capabilities.toml` `[counts]`
//! table; a mismatch fails the build.
//!
//! Every numeric public claim must come from this module. Documents that
//! state counts without citing a generated manifest value are violations of
//! the claims policy (ROADMAP §0.3).
//!
//! # Examples
//!
//! ```rust
//! use crawlkit_engine::manifest;
//!
//! let m = manifest::generate();
//! assert!(m.analyzer_count > 0);
//! assert!(m.unique_analyzer_types > 0);
//! assert!(m.unique_analyzer_types <= m.analyzer_count);
//! let toml = manifest::render_toml();
//! assert!(toml.contains("[counts]"));
//! ```

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::analyzers::AnalyzerRegistry;
use crate::CrawlConfig;

/// Counts derived from a fully-populated [`AnalyzerRegistry`].
///
/// "Analyzer" and "finding code" are distinct metrics (ROADMAP §0.1):
/// analyzers are components; finding codes are the check identifiers they
/// emit. Neither count is derived from documentation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CapabilityManifest {
    /// Total registered analyzer instances in the default registry.
    pub analyzer_count: usize,
    /// Distinct concrete analyzer types (deduplicated by type name).
    pub unique_analyzer_types: usize,
    /// Distinct finding codes emitted across a representative analysis.
    pub finding_code_count: usize,
    /// Distinct analyzer profiles exposed by the registry.
    pub profile_count: usize,
    /// Per-analyzer-type instance counts, sorted by type name.
    pub analyzer_type_counts: BTreeMap<String, usize>,
    /// Sorted finding codes observed for the representative context.
    pub finding_codes: BTreeSet<String>,
}

/// Analyze a representative page with the full registry and collect finding
/// codes plus type-level analyzer counts.
///
/// The representative context deliberately exercises the common analysis
/// surface (headers, body, redirect chain, robots.txt) so the code census
/// covers analyzers that key off each input. It is a census, not an
/// exhaustive proof: analyzers are counted by registration; codes are
/// counted from what the default registry emits on this input.
///
/// # Panics-free parsing
///
/// The inline fixture is static and well-formed; parse failures are
/// impossible, so no error path is required.
pub fn generate() -> CapabilityManifest {
    let config = CrawlConfig::default();
    let registry = AnalyzerRegistry::new(&config);

    let analyzer_count = registry.len();

    // Type-name census: one entry per concrete analyzer type.
    let mut analyzer_type_counts: BTreeMap<String, usize> = BTreeMap::new();
    for name in registry.analyzer_type_names() {
        *analyzer_type_counts.entry(name).or_default() += 1;
    }
    let unique_analyzer_types = analyzer_type_counts.len();

    // Finding-code census from a representative page analysis.
    let url = match url::Url::parse("https://example.com/") {
        Ok(u) => u,
        Err(_) => return fallback_manifest(analyzer_type_counts),
    };
    let html = concat!(
        "<!DOCTYPE html><html lang=\"en\"><head>",
        "<title>Example Domain — manifest census</title>",
        "<meta name=\"description\" content=\"Manifest census page\">",
        "<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">",
        "<link rel=\"canonical\" href=\"https://example.com/\">",
        "<meta property=\"og:title\" content=\"Example\">",
        "<meta name=\"twitter:card\" content=\"summary\">",
        "</head><body>",
        "<h1>Example Domain</h1>",
        "<main><p>",
        "<a href=\"/about\">About</a>",
        "<a href=\"https://external.example/\">External</a>",
        "<img src=\"/logo.png\" alt=\"Logo\" width=\"80\" height=\"80\">",
        "<form action=\"/search\" method=\"get\"><label for=\"q\">Query</label>",
        "<input id=\"q\" name=\"q\" type=\"text\"></form>",
        "<table><caption>Nav</caption><tr><th scope=\"col\">H</th></tr>",
        "<tr><td>D</td></tr></table>",
        "<script type=\"application/ld+json\">{\"@context\":\"https://schema.org\",",
        "\"@type\":\"WebPage\",\"name\":\"Example\"}</script>",
        "</main></body></html>"
    );
    let body_words = "word ".repeat(120);
    let html = format!("{html}{body_words}</p>");
    let page = crate::parser::HtmlParser::parse(&html, &url);
    let headers: Vec<(String, String)> = vec![
        ("content-type".to_string(), "text/html; charset=utf-8".to_string()),
        ("cache-control".to_string(), "max-age=3600".to_string()),
    ];
    let redirect_chain: Vec<crate::RedirectHop> = Vec::new();
    let ctx = crate::AnalysisContext {
        page: &page,
        body: Some(&html),
        status_code: Some(200),
        headers: &headers,
        response_time: Some(std::time::Duration::from_millis(120)),
        redirect_chain: &redirect_chain,
        robots_txt: Some("User-agent: *\nDisallow: /private\nSitemap: https://example.com/sitemap.xml"),
        body_size: Some(html.len()),
        compressed_size: Some(html.len()),
        server: Some("example"),
        content_type: Some("text/html; charset=utf-8"),
        rendered: None,
    };
    let findings = registry.analyze(&ctx);
    let finding_codes: BTreeSet<String> = findings.iter().map(|f| f.code.clone()).collect();
    let finding_code_count = finding_codes.len();

    CapabilityManifest {
        analyzer_count,
        unique_analyzer_types,
        finding_code_count,
        profile_count: 4, // core, standard, deep, full (see AnalyzerRegistry profiles)
        analyzer_type_counts,
        finding_codes,
    }
}

/// Render the manifest as a TOML fragment matching the `docs/capabilities.toml`
/// `[counts]` table format.
///
/// # Examples
///
/// ```rust
/// use crawlkit_engine::manifest;
///
/// let toml = manifest::render_toml();
/// assert!(toml.contains("[counts]"));
/// assert!(toml.contains("analyzer_count"));
/// ```
pub fn render_toml() -> String {
    let m = generate();
    let mut out = String::with_capacity(2048);
    out.push_str("[counts]\n");
    out.push_str(&format!("analyzer_count = {}\n", m.analyzer_count));
    out.push_str(&format!(
        "unique_analyzer_types = {}\n",
        m.unique_analyzer_types
    ));
    out.push_str(&format!(
        "finding_code_count = {}\n",
        m.finding_code_count
    ));
    out.push_str(&format!("profile_count = {}\n", m.profile_count));
    out
}

/// Compare the committed `[counts]` table with the generated one.
///
/// Returns `Err` listing each drifted key (`committed != generated`) so CI
/// can fail with an actionable diff instead of a bare boolean.
pub fn check_drift(committed_counts: &str) -> Result<(), Vec<String>> {
    let generated = generate();

    let mut expected: BTreeMap<&str, usize> = BTreeMap::new();
    expected.insert("analyzer_count", generated.analyzer_count);
    expected.insert("unique_analyzer_types", generated.unique_analyzer_types);
    expected.insert("finding_code_count", generated.finding_code_count);
    expected.insert("profile_count", generated.profile_count);

    let mut drift = Vec::new();
    for (key, want) in &expected {
        match extract_count(committed_counts, key) {
            Some(got) if &got == want => {}
            Some(got) => drift.push(format!("{key}: committed {got} != generated {want}")),
            None => drift.push(format!("{key}: missing from committed [counts] table")),
        }
    }
    if drift.is_empty() {
        Ok(())
    } else {
        Err(drift)
    }
}

/// Type-count-only manifest used when the static census URL cannot parse.
/// The URL is a compile-time constant, so this path is unreachable in
/// practice; it exists to keep `generate` panic-free (workspace lint).
fn fallback_manifest(analyzer_type_counts: BTreeMap<String, usize>) -> CapabilityManifest {
    CapabilityManifest {
        analyzer_count: analyzer_type_counts.values().sum(),
        unique_analyzer_types: analyzer_type_counts.len(),
        finding_code_count: 0,
        profile_count: 4,
        analyzer_type_counts,
        finding_codes: BTreeSet::new(),
    }
}

/// Extract an integer value for `key` from a `[counts]` TOML table body.
fn extract_count(toml_text: &str, key: &str) -> Option<usize> {
    let mut in_counts = false;
    for line in toml_text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_counts = trimmed == "[counts]";
            continue;
        }
        if !in_counts {
            continue;
        }
        let mut parts = trimmed.splitn(2, '=');
        if parts.next()?.trim() == key {
            return parts
                .next()?
                .split('#')
                .next()?
                .trim()
                .parse::<usize>()
                .ok();
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_counts_are_consistent() {
        let m = generate();
        assert!(m.analyzer_count > 0);
        assert!(m.unique_analyzer_types > 0);
        assert!(m.unique_analyzer_types <= m.analyzer_count);
        assert!(m.finding_code_count > 0);
    }

    #[test]
    fn render_toml_is_parseable_shape() {
        let toml = render_toml();
        assert!(toml.starts_with("[counts]\n"));
        assert!(toml.contains("analyzer_count = "));
        assert!(toml.contains("unique_analyzer_types = "));
        assert!(toml.contains("finding_code_count = "));
    }

    #[test]
    fn drift_check_passes_on_matching_input() {
        let toml = render_toml();
        assert!(check_drift(&toml).is_ok());
    }

    #[test]
    fn drift_check_fails_on_stale_count() {
        let stale = format!("[counts]\nanalyzer_count = {}\n", generate().analyzer_count);
        let toml = stale.replace(
            &format!("analyzer_count = {}", generate().analyzer_count),
            "analyzer_count = 1",
        );
        let errs = check_drift(&toml).unwrap_err();
        assert!(errs.iter().any(|e| e.contains("analyzer_count")));
    }

    #[test]
    fn drift_check_reports_missing_key() {
        let errs = check_drift("[counts]\nanalyzer_count = 5\n").unwrap_err();
        assert!(errs.len() >= 3, "missing keys must be reported: {errs:?}");
    }

    #[test]
    fn extract_count_reads_simple_table() {
        let toml = "[counts]\nanalyzer_count = 42\n[other]\nanalyzer_count = 99\n";
        assert_eq!(extract_count(toml, "analyzer_count"), Some(42));
    }

    #[test]
    fn extract_count_ignores_inline_comments() {
        let toml = "[counts]\nanalyzer_count = 42 # kept current by CI\n";
        assert_eq!(extract_count(toml, "analyzer_count"), Some(42));
    }

    #[test]
    fn extract_count_rejects_non_numeric_value() {
        let toml = "[counts]\nanalyzer_count = \"many\"\n";
        assert_eq!(extract_count(toml, "analyzer_count"), None);
    }

    #[test]
    fn drift_check_fails_when_section_missing() {
        let errs = check_drift("[commands]\nnames = [\"crawl\"]\n").unwrap_err();
        assert_eq!(errs.len(), 4, "all four keys must be reported: {errs:?}");
        assert!(errs.iter().all(|e| e.contains("missing from committed")));
    }

    #[test]
    fn drift_check_fails_on_empty_input() {
        let errs = check_drift("").unwrap_err();
        assert_eq!(errs.len(), 4);
    }

    #[test]
    fn drift_check_accepts_extra_committed_keys() {
        // Extra keys in the committed table (e.g. hand-added notes) must not
        // fail the check; only the four generated keys are authoritative.
        let toml = format!(
            "[counts]\n{}extra_note = 1\n",
            render_toml().strip_prefix("[counts]\n").unwrap()
        );
        assert!(check_drift(&toml).is_ok());
    }

    #[test]
    fn drift_check_error_names_both_values() {
        let toml = "[counts]\nanalyzer_count = 999999\n";
        let errs = check_drift(toml).unwrap_err();
        let line = errs.iter().find(|e| e.contains("analyzer_count")).unwrap();
        assert!(line.contains("committed 999999"), "must show committed value: {line}");
        assert!(line.contains("generated"), "must show generated value: {line}");
    }
}
