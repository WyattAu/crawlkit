use anyhow::{Context, Result};
use crawlkit_engine::storage::{PageData, Storage};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// CI quality-gate options for the `report` command (GAP G4).
///
/// Exit codes when [`CiGateOptions::enabled`]: `0` pass, `10` errors exceeded
/// (or new errors versus the baseline), `11` warnings exceeded, `12` score
/// below floor. Decided by [`ci_exit_code`].
#[derive(Debug, Clone, Default)]
pub struct CiGateOptions {
    /// Enable the gate: print a one-line summary and exit non-zero on
    /// failure. Without it the report always exits 0.
    pub enabled: bool,
    /// Upper bound on error-severity findings; critical-severity findings
    /// count as errors. Default 0: any error fails.
    pub max_errors: usize,
    /// Upper bound on warning-severity findings. `None` = unbounded.
    pub max_warnings: Option<usize>,
    /// Lower bound on the report score.
    ///
    /// Reserved: crawlkit has no scoring model yet (GAP G2 is unimplemented),
    /// so the CLI cannot set this and the gate always receives `None`. The
    /// decision path is implemented and unit-tested so wiring `--min-score`
    /// later is a one-flag change.
    pub min_score: Option<f64>,
    /// Crawl to diff against (database file or directory). `None` = no
    /// baseline comparison.
    pub baseline: Option<PathBuf>,
    /// Error-severity regressions found by diffing against `baseline` —
    /// filled in by [`run`] after the comparison. Any positive value fails
    /// the gate regardless of `max_errors`: a new error is a regression even
    /// under a lenient ceiling.
    pub new_errors_vs_baseline: usize,
}

/// Rendering options for `report` beyond the crawl itself (GAP G9 + G10).
#[derive(Debug, Clone, Default)]
pub struct ReportRenderOptions {
    /// Sections to render (GAP G9): repeatable or comma-separated tokens
    /// (`summary`, `backlinks`, `previews`, `all`), parsed by
    /// [`parse_sections`]. Empty selects the pre-G9 default sections so
    /// existing invocations keep byte-identical output.
    pub sections: Vec<String>,
    /// Print the HTML report to this PDF file (GAP G10) using a PDF engine
    /// found in `PATH`. The textual report is still produced per `--format`.
    pub pdf: Option<PathBuf>,
}

/// Decide the `report --ci` exit code. Pure and unit-tested; no I/O.
///
/// `errors`/`warnings` are finding counts by severity (critical counts as an
/// error), `score` the report score when one exists. Returns `None` for a
/// pass (the caller exits 0), or the exit code:
///
/// - `10` — errors exceed `max_errors`, or `new_errors_vs_baseline` is positive
/// - `11` — warnings exceed `max_warnings`
/// - `12` — score below `min_score`
///
/// Checked in that order: a hard error failure is reported even when the
/// warning budget is also blown.
pub fn ci_exit_code(
    errors: usize,
    warnings: usize,
    score: Option<f64>,
    opts: &CiGateOptions,
) -> Option<u8> {
    if errors > opts.max_errors || opts.new_errors_vs_baseline > 0 {
        return Some(10);
    }
    if let Some(max) = opts.max_warnings {
        if warnings > max {
            return Some(11);
        }
    }
    if let (Some(score), Some(floor)) = (score, opts.min_score) {
        if score < floor {
            return Some(12);
        }
    }
    None
}

/// Resolve a crawl path (directory or database file) to its database.
fn db_path_for(crawl_path: &Path) -> PathBuf {
    if crawl_path.is_dir() {
        crawl_path.join("crawlkit.db")
    } else {
        crawl_path.to_path_buf()
    }
}

/// Sum the stored issue counts for the given severity keys, as stored by
/// `Severity::as_str` ("critical", "error", "warning", "info").
fn severity_total(stats: &crawlkit_engine::storage::CrawlStats, severities: &[&str]) -> usize {
    severities
        .iter()
        .map(|s| stats.issues_by_severity.get(*s).copied().unwrap_or(0))
        .sum()
}

/// Terminate with a CI gate exit code.
///
/// `clippy::exit` is denied workspace-wide because `process::exit` skips
/// destructors — a real hazard in library code. Here the non-zero exit code
/// *is* the feature (GAP G4): the call runs after the report is written and
/// the progress bar finished, and a CLI handing control to a CI pipeline
/// loses nothing by skipping the process-global teardown.
#[allow(clippy::exit)]
fn exit_with_ci_code(code: u8) -> ! {
    std::process::exit(i32::from(code));
}

// ---------------------------------------------------------------------------
// Report sections (GAP G9)
// ---------------------------------------------------------------------------

/// A selectable report section (`report --section`, GAP G9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    /// Crawl-level counters: pages, issues, severity breakdown, timings.
    Summary,
    /// Backlink summary and top pages by PageRank. Data only exists when the
    /// crawl was run with the backlink-analysis feature flag.
    Backlinks,
    /// Per-platform social share preview frames built from stored page meta.
    Previews,
}

/// The sections rendered when `--section` is not passed: exactly the sections
/// that existed before G9. `previews` is new and opt-in, so default output
/// stays byte-identical with earlier crawlkit versions.
const DEFAULT_SECTIONS: &[Section] = &[Section::Summary, Section::Backlinks];

/// Parse repeatable/comma-separated `--section` values into a deduplicated
/// list that preserves first-mention order. Empty input selects
/// [`DEFAULT_SECTIONS`]. The token `all` selects every section, including the
/// previews section that the default omits.
///
/// Pure and unit-tested; unknown tokens are a hard error so a typo cannot
/// silently render an empty report.
fn parse_sections(values: &[String]) -> Result<Vec<Section>> {
    if values.is_empty() {
        return Ok(DEFAULT_SECTIONS.to_vec());
    }
    let mut selected: Vec<Section> = Vec::new();
    for value in values {
        for token in value.split(',') {
            let token = token.trim();
            let section = match token {
                "" => continue,
                "summary" => Section::Summary,
                "backlinks" => Section::Backlinks,
                "previews" => Section::Previews,
                "all" => {
                    for section in [Section::Summary, Section::Backlinks, Section::Previews] {
                        if !selected.contains(&section) {
                            selected.push(section);
                        }
                    }
                    continue;
                }
                other => {
                    return Err(anyhow::anyhow!(
                        "Unknown report section '{other}'. Valid: summary, backlinks, previews, all."
                    ));
                }
            };
            if !selected.contains(&section) {
                selected.push(section);
            }
        }
    }
    Ok(selected)
}

/// Cap on pages rendered in the share-previews section. A report is a
/// summary artifact, not a per-page dump; the limit keeps HTML output bounded
/// and is applied to the storage query itself.
const PREVIEW_PAGE_LIMIT: usize = 20;

/// Escape text for interpolation into HTML element content and attributes.
/// Applied to every value derived from crawl data, so stored titles or
/// descriptions containing markup cannot inject HTML into the report.
fn escape_html(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

/// Collapse all whitespace runs to single spaces. Stored meta can contain
/// newlines that would break markdown list items; the HTML renderer lets the
/// browser collapse whitespace, so this is only used on the markdown path.
fn collapse_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// One page's share preview, derived from the meta data the crawler stored.
///
/// `og:image`/`og:image:width`/`og:image:height` are parsed at crawl time but
/// not persisted to the report database, so there is deliberately no image
/// field here: the frames render a fixed placeholder instead of a plausible
/// URL that was never observed.
struct SharePreview {
    url: String,
    title: String,
    description: String,
    site_name: String,
}

/// Derive one [`SharePreview`] from a stored page: canonical URL when the
/// crawl recorded one (what sharers should display), else the final URL.
/// Title/description come from the stored `<title>`/meta description, which
/// is the same data `og:` tags fall back to on every major platform.
fn share_preview_from_page(page: &PageData) -> SharePreview {
    let url = page
        .canonical_url
        .as_ref()
        .unwrap_or(&page.final_url)
        .as_str()
        .to_string();
    let site_name = url::Url::parse(&url)
        .ok()
        .and_then(|parsed| parsed.host_str().map(str::to_string))
        .unwrap_or_default();
    SharePreview {
        url,
        title: page
            .title
            .clone()
            .unwrap_or_else(|| "Untitled page".to_string()),
        description: page
            .description
            .clone()
            .unwrap_or_else(|| "No meta description stored for this page.".to_string()),
        site_name,
    }
}

/// `https://example.com/a/b` → `example.com › a › b`, the breadcrumb form
/// search engines display above a result title.
fn breadcrumb_url(url: &str) -> String {
    let parsed = url::Url::parse(url).ok();
    let host = parsed
        .as_ref()
        .and_then(|u| u.host_str())
        .unwrap_or(url)
        .to_string();
    let path = parsed.map(|u| u.path().to_string()).unwrap_or_default();
    let segments: Vec<&str> = path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();
    if segments.is_empty() {
        host
    } else {
        format!("{host} › {}", segments.join(" › "))
    }
}

/// Render the share-preview section HTML: one group of mock card frames per
/// stored page (Google result snippet, Facebook, X, LinkedIn, iMessage).
///
/// Pure HTML/CSS — no external services, no images fetched, no timestamps —
/// so the same stored pages always produce byte-identical markup.
fn render_share_previews_html(pages: &[PageData]) -> String {
    let mut html = String::from("<h2>Social share previews</h2>\n");
    html.push_str(
        "<p class=\"ck-meta\">Card text is derived from the stored page meta. \
         og:image is not persisted by the crawler, so image slots are placeholders.</p>\n",
    );
    for page in pages.iter().take(PREVIEW_PAGE_LIMIT) {
        let preview = share_preview_from_page(page);
        let url = escape_html(&preview.url);
        let title = escape_html(&preview.title);
        let description = escape_html(&preview.description);
        let site = escape_html(&preview.site_name);
        let crumb = escape_html(&breadcrumb_url(&preview.url));
        let image_box = "<div class=\"ck-image-box\">og:image not stored</div>";
        html.push_str(&format!(
            "<h3>{url}</h3>\n\
             <div class=\"ck-frame\"><span class=\"ck-platform\">Google</span>\n\
             <div class=\"ck-google-snippet\"><div class=\"ck-google-site\">{crumb}</div>\n\
             <div class=\"ck-google-title ck-title\">{title}</div>\n\
             <div class=\"ck-google-desc\">{description}</div></div></div>\n\
             <div class=\"ck-frame\"><span class=\"ck-platform\">Facebook</span>\n\
             <div class=\"ck-card\">{image_box}\n\
             <div class=\"ck-card-domain\">{site}</div>\n\
             <div class=\"ck-card-title ck-title\">{title}</div>\n\
             <div class=\"ck-card-desc\">{description}</div></div></div>\n\
             <div class=\"ck-frame\"><span class=\"ck-platform\">X</span>\n\
             <div class=\"ck-card\">{image_box}\n\
             <div class=\"ck-card-domain\">{site}</div>\n\
             <div class=\"ck-card-title ck-title\">{title}</div>\n\
             <div class=\"ck-card-desc\">{description}</div></div></div>\n\
             <div class=\"ck-frame\"><span class=\"ck-platform\">LinkedIn</span>\n\
             <div class=\"ck-card\">{image_box}\n\
             <div class=\"ck-card-domain\">{site}</div>\n\
             <div class=\"ck-card-title ck-title\">{title}</div>\n\
             <div class=\"ck-card-desc\">{description}</div></div></div>\n\
             <div class=\"ck-frame\"><span class=\"ck-platform\">iMessage</span>\n\
             <div class=\"ck-bubble\"><div class=\"ck-bubble-title ck-title\">{title}</div>\n\
             <div class=\"ck-bubble-domain\">{site}</div>\n\
             <div class=\"ck-bubble-desc\">{description}</div></div></div>\n",
            url = url,
            crumb = crumb,
            title = title,
            description = description,
            site = site,
            image_box = image_box,
        ));
    }
    html
}

/// Render the share-preview section for the markdown report. Markdown cannot
/// show styled mock frames, so it lists the per-page card text the frames
/// would use; the frames themselves are only rendered by the html format.
fn render_share_previews_markdown(pages: &[PageData]) -> String {
    let mut md = String::from(
        "\n## Social share previews\n\n\
         Card text derived from stored page meta. Styled mock frames for\n\
         Google, Facebook, X, LinkedIn, and iMessage are rendered by the\n\
         html format (and `--pdf`); og:image is not persisted by the crawler.\n",
    );
    if pages.is_empty() {
        md.push_str("\nNo pages stored for this crawl.\n");
        return md;
    }
    for page in pages.iter().take(PREVIEW_PAGE_LIMIT) {
        let preview = share_preview_from_page(page);
        md.push_str(&format!(
            "\n### {}\n\n\
             - **Title:** {}\n\
             - **Description:** {}\n\
             - **Site:** {}\n",
            collapse_whitespace(&preview.url),
            collapse_whitespace(&preview.title),
            collapse_whitespace(&preview.description),
            collapse_whitespace(&preview.site_name),
        ));
    }
    md
}

// ---------------------------------------------------------------------------
// HTML report (GAP G10)
// ---------------------------------------------------------------------------

/// Fixed stylesheet for the HTML report, including the `@media print` rules
/// that make it print-ready for PDF export. Deliberately versionless and
/// environment-independent: the same crawl data must produce byte-identical
/// HTML on every machine (determinism rules).
const REPORT_CSS: &str = "\
:root { color-scheme: light; }
body { font-family: system-ui, sans-serif; margin: 2rem; color: #1a1a1a; background: #ffffff; }
h1 { font-size: 1.5rem; margin: 0 0 0.25rem; }
h2 { font-size: 1.2rem; margin-top: 2rem; }
h3 { font-size: 1rem; margin: 1.5rem 0 0.5rem; }
.ck-meta { color: #555555; font-size: 0.85rem; }
table.ck-table { border-collapse: collapse; margin-top: 0.5rem; }
.ck-table th, .ck-table td { border: 1px solid #cccccc; padding: 0.3rem 0.6rem; text-align: left; }
.ck-table th { background: #f2f2f2; }
.ck-platform { display: block; font-size: 0.7rem; letter-spacing: 0.08em; text-transform: uppercase; color: #666666; margin-bottom: 0.35rem; }
.ck-frame { border: 1px solid #d0d0d0; border-radius: 6px; padding: 0.75rem; margin: 0.75rem 0; max-width: 34rem; }
.ck-title { font-weight: 600; }
.ck-card-domain, .ck-google-site, .ck-bubble-domain { color: #556677; font-size: 0.75rem; }
.ck-card-desc, .ck-google-desc, .ck-bubble-desc { font-size: 0.8rem; color: #444444; }
.ck-google-snippet { padding: 0.25rem 0; }
.ck-google-title { color: #1a0dab; }
.ck-image-box { background: #e8eef5; border: 1px dashed #b9c6d2; border-radius: 4px; height: 158px; display: flex; align-items: center; justify-content: center; font-size: 0.8rem; color: #5a6b7d; margin-bottom: 0.5rem; }
.ck-bubble { background: #e9e9eb; border-radius: 12px; padding: 0.6rem 0.8rem; }
@media print { body { margin: 0; } .ck-frame, .ck-table { page-break-inside: avoid; } }
";

/// Severity counts in a fixed order (critical, error, warning, info, then any
/// unrecognized keys alphabetically) so HTML output is byte-identical between
/// runs despite `HashMap`'s unordered iteration.
fn severity_breakdown(counts: &std::collections::HashMap<String, usize>) -> Vec<(&str, usize)> {
    const KNOWN: [&str; 4] = ["critical", "error", "warning", "info"];
    let mut rows: Vec<(&str, usize)> = KNOWN
        .iter()
        .filter_map(|key| counts.get(*key).map(|count| (*key, *count)))
        .collect();
    let mut extras: Vec<(&str, usize)> = counts
        .iter()
        .filter(|(key, _)| !KNOWN.contains(&key.as_str()))
        .map(|(key, count)| (key.as_str(), *count))
        .collect();
    extras.sort_unstable_by(|a, b| a.0.cmp(b.0));
    rows.extend(extras);
    rows
}

/// Render the crawl-summary section of the HTML report.
fn render_summary_html(stats: &crawlkit_engine::storage::CrawlStats) -> String {
    let mut html = String::from("<h2>Summary</h2>\n<table class=\"ck-table\">\n");
    html.push_str(&format!(
        "<tr><th>Total pages</th><td>{}</td></tr>\n\
         <tr><th>Total issues</th><td>{}</td></tr>\n",
        stats.total_pages, stats.total_issues,
    ));
    for (severity, count) in severity_breakdown(&stats.issues_by_severity) {
        html.push_str(&format!(
            "<tr><th>{severity} findings</th><td>{count}</td></tr>\n"
        ));
    }
    if let Some(avg) = stats.avg_response_time_ms {
        html.push_str(&format!(
            "<tr><th>Avg response time</th><td>{avg:.2}ms</td></tr>\n"
        ));
    }
    if let Some(size) = stats.total_body_size {
        html.push_str(&format!(
            "<tr><th>Total body size</th><td>{size} bytes</td></tr>\n"
        ));
    }
    html.push_str("</table>\n");
    html
}

/// Render the backlinks section of the HTML report (top pages by PageRank).
fn render_backlinks_html(summary: &crawlkit_engine::BacklinkSummary) -> String {
    let mut html = format!(
        "<h2>Backlinks</h2>\n<table class=\"ck-table\">\n\
         <tr><th>Internal links</th><td>{}</td></tr>\n\
         <tr><th>External links</th><td>{}</td></tr>\n\
         <tr><th>Referring domains</th><td>{}</td></tr>\n\
         <tr><th>Orphan pages</th><td>{}</td></tr>\n</table>\n",
        summary.total_internal_links,
        summary.total_external_links,
        summary.total_referring_domains,
        summary.orphan_pages.len(),
    );
    html.push_str(
        "<h3>Top pages by PageRank</h3>\n<table class=\"ck-table\">\n\
         <tr><th>URL</th><th>PageRank</th><th>Inbound</th><th>Outbound</th>\
         <th>Referring domains</th></tr>\n",
    );
    for page in summary.pages.iter().take(10) {
        html.push_str(&format!(
            "<tr><td>{}</td><td>{:.4}</td><td>{}</td><td>{}</td><td>{}</td></tr>\n",
            escape_html(&page.url),
            page.pagerank,
            page.inbound_links,
            page.outbound_links,
            page.referring_domains,
        ));
    }
    html.push_str("</table>\n");
    html
}

/// Build the standalone HTML report. Fixed CSS, escaped interpolation only,
/// no timestamps: the same inputs always produce byte-identical output, which
/// is what makes the PDF export reproducible.
fn build_html_report(
    crawl_id: &str,
    stats: &crawlkit_engine::storage::CrawlStats,
    backlink_data: &Option<(
        crawlkit_engine::BacklinkSummary,
        std::collections::HashMap<String, f64>,
    )>,
    sections: &[Section],
    preview_pages: &[PageData],
) -> String {
    let mut body = String::new();
    if sections.contains(&Section::Summary) {
        body.push_str(&render_summary_html(stats));
    }
    if sections.contains(&Section::Backlinks) {
        if let Some((summary, _pagerank)) = backlink_data {
            body.push_str(&render_backlinks_html(summary));
        }
    }
    if sections.contains(&Section::Previews) {
        body.push_str(&render_share_previews_html(preview_pages));
    }
    format!(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
         <title>Crawl Report {cid}</title>\n<style>\n{css}</style>\n</head>\n\
         <body>\n<h1>Crawl Report</h1>\n<p class=\"ck-meta\">Crawl ID: {cid}</p>\n\
         {body}</body>\n</html>\n",
        cid = escape_html(crawl_id),
        css = REPORT_CSS,
        body = body,
    )
}

// ---------------------------------------------------------------------------
// PDF export (GAP G10)
// ---------------------------------------------------------------------------

/// External programs that can print HTML to PDF, in preference order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PdfEngine {
    /// `weasyprint <input.html> <output.pdf>` — purpose-built, no browser.
    WeasyPrint,
    /// `chromium --headless --print-to-pdf=... file://...` — any Chromium.
    Chromium,
}

impl PdfEngine {
    /// Candidate binary names searched in `PATH`, in order.
    fn binary_names(self) -> &'static [&'static str] {
        match self {
            PdfEngine::WeasyPrint => &["weasyprint"],
            PdfEngine::Chromium => &["chromium", "chromium-browser"],
        }
    }
}

impl std::fmt::Display for PdfEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            PdfEngine::WeasyPrint => "weasyprint",
            PdfEngine::Chromium => "chromium",
        })
    }
}

/// Wall-clock cap on the print subprocess (GAP G10).
const PDF_TIMEOUT: Duration = Duration::from_secs(10);

/// Exit code when `--pdf` is requested but no PDF engine exists in `PATH`.
/// Documented in the `report` command's help.
pub const EXIT_PDF_ENGINE_MISSING: u8 = 3;

/// Whether `path` is an existing file with an execute bit set. A directory on
/// `PATH` may hold a same-named data file; executing that would be wrong.
#[cfg(unix)]
fn is_executable_file(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.is_file()
        && std::fs::metadata(path)
            .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable_file(path: &Path) -> bool {
    path.is_file()
}

/// Walk a `PATH`-style string (directories joined by `:`) and return the first
/// directory containing an executable file named `name`. Pure given the
/// filesystem state; unit-tested with fake binaries so no real chromium is
/// needed.
fn find_executable_in_path(search_path: &str, name: &str) -> Option<PathBuf> {
    for dir in search_path.split(':') {
        if dir.is_empty() {
            continue;
        }
        let candidate = Path::new(dir).join(name);
        if is_executable_file(&candidate) {
            return Some(candidate);
        }
    }
    None
}

/// Resolve a PDF engine from an explicit `PATH` string: weasyprint first, then
/// chromium/chromium-browser. Pure function over `(PATH string, filesystem)`;
/// unit-tested without any real browser installed.
fn resolve_pdf_engine_from_path(search_path: &str) -> Option<(PdfEngine, PathBuf)> {
    for engine in [PdfEngine::WeasyPrint, PdfEngine::Chromium] {
        for name in engine.binary_names() {
            if let Some(exe) = find_executable_in_path(search_path, name) {
                return Some((engine, exe));
            }
        }
    }
    None
}

/// Resolve a PDF engine from the process environment. Non-UTF-8 `PATH` values
/// are matched lossily rather than discarding a valid engine directory.
fn resolve_pdf_engine() -> Option<(PdfEngine, PathBuf)> {
    let search_path = std::env::var_os("PATH")?;
    resolve_pdf_engine_from_path(&search_path.to_string_lossy())
}

/// Build the print command for `engine`. Kept pure (no I/O) so the exact
/// flags each engine receives are unit-tested.
fn pdf_print_command(
    engine: PdfEngine,
    exe: &Path,
    html_path: &Path,
    pdf_path: &Path,
) -> std::process::Command {
    let mut command = std::process::Command::new(exe);
    match engine {
        PdfEngine::WeasyPrint => {
            command.arg(html_path).arg(pdf_path);
        }
        PdfEngine::Chromium => {
            command
                .arg("--headless")
                .arg(format!("--print-to-pdf={}", pdf_path.display()))
                .arg(format!("file://{}", html_path.display()));
        }
    }
    command
}

/// Run `command` to completion under a wall-clock timeout, killing the child
/// when it expires. Never panics: spawn/poll failures become errors, and the
/// timeout path kills, reaps, and reports instead of leaking the process.
fn run_with_timeout(
    mut command: std::process::Command,
    timeout: Duration,
) -> Result<std::process::ExitStatus> {
    use std::process::Stdio;
    // The report HTML is already in the temp file and engines write the PDF
    // to `--print-to-pdf`/argv; their streams are noise we cannot drain
    // without risking a full-pipe deadlock while waiting on the timeout.
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let mut child = command
        .spawn()
        .with_context(|| format!("failed to spawn {}", command.get_program().display()))?;
    let started = Instant::now();
    loop {
        match child
            .try_wait()
            .context("failed to poll the PDF subprocess")?
        {
            Some(status) => return Ok(status),
            None => {
                if started.elapsed() >= timeout {
                    // Kill first, then reap so no zombie survives; secondary
                    // errors on an already-timeout path are not the story.
                    let _ = child.kill();
                    let _ = child.wait();
                    anyhow::bail!(
                        "PDF export timed out after {timeout:?} and the subprocess was killed"
                    );
                }
                std::thread::sleep(Duration::from_millis(25));
            }
        }
    }
}

/// Print `html_path` to `pdf_path` with `engine`, enforcing [`PDF_TIMEOUT`]
/// and requiring a real output file afterwards.
fn print_html_to_pdf(
    engine: PdfEngine,
    exe: &Path,
    html_path: &Path,
    pdf_path: &Path,
) -> Result<()> {
    let command = pdf_print_command(engine, exe, html_path, pdf_path);
    let status = run_with_timeout(command, PDF_TIMEOUT)?;
    if !status.success() {
        anyhow::bail!("{engine} failed to print the PDF (exit status: {status})");
    }
    if !pdf_path.is_file() {
        anyhow::bail!(
            "{engine} exited successfully but created no PDF at {}",
            pdf_path.display()
        );
    }
    Ok(())
}

/// Terminate with [`EXIT_PDF_ENGINE_MISSING`].
///
/// `clippy::exit` is denied workspace-wide because `process::exit` skips
/// destructors. Here the non-zero exit code *is* the documented contract
/// (GAP G10): the report has already been rendered and written, and a CLI
/// handing a distinguishable "engine missing" status to a pipeline loses
/// nothing by skipping process-global teardown.
#[allow(clippy::exit)]
fn exit_with_pdf_engine_missing() -> ! {
    std::process::exit(i32::from(EXIT_PDF_ENGINE_MISSING));
}

/// Print the generated report HTML to `pdf_path`, or exit `3` when no engine
/// is installed (GAP G10).
#[allow(clippy::exit)]
fn export_pdf(html: &str, pdf_path: &Path) -> Result<()> {
    let Some((engine, exe)) = resolve_pdf_engine() else {
        eprintln!(
            "error: --pdf needs a PDF engine, but none of weasyprint, chromium, \
             chromium-browser was found in PATH (exit code {EXIT_PDF_ENGINE_MISSING})."
        );
        eprintln!("Install one of them and re-run, or omit --pdf to keep the textual report only.");
        exit_with_pdf_engine_missing();
    };

    // Fixed name, fixed content: the HTML is deterministic, so overwriting a
    // previous run's temp file is safe and needs no timestamped paths.
    let temp_html = std::env::temp_dir().join("crawlkit-report-print.html");
    std::fs::write(&temp_html, html)
        .with_context(|| format!("failed to write temporary HTML to {}", temp_html.display()))?;
    let html_abs = std::path::absolute(&temp_html)
        .with_context(|| format!("failed to resolve {}", temp_html.display()))?;

    print_html_to_pdf(engine, &exe, &html_abs, pdf_path)?;
    println!("PDF written to {} (using {engine})", pdf_path.display());
    Ok(())
}

/// Generate a report from an existing crawl.
///
/// `options.sections` (GAP G9) filters the rendered sections; empty selects
/// the pre-G9 defaults so existing invocations keep byte-identical output.
/// `options.pdf` (GAP G10) additionally prints the HTML report to a PDF via
/// an external engine, exiting with code `3` when none is installed.
pub fn run(
    crawl_path: &Path,
    output: Option<&Path>,
    format: &str,
    _theme: &str,
    feature_flags: &crawlkit_engine::FeatureFlags,
    ci: &CiGateOptions,
    options: &ReportRenderOptions,
) -> Result<()> {
    let selected = parse_sections(&options.sections)?;

    // json/csv have fixed layouts with no section structure; a silent ignore
    // would look like the feature is broken. (markdown/html — and the page
    // `--pdf` prints, which is always HTML — honor the selection.)
    if !options.sections.is_empty() && matches!(format, "json" | "csv") {
        return Err(anyhow::anyhow!(
            "--section applies to markdown and html output; the '{format}' format has a fixed layout."
        ));
    }

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner()),
    );
    pb.set_message("Generating report...");

    let db_path = db_path_for(crawl_path);

    let storage = Storage::new(&db_path)
        .with_context(|| format!("Failed to open crawl database: {}", db_path.display()))?;

    let _encryption = crawlkit_engine::EncryptionManager::default();

    let crawl_id = storage
        .get_latest_crawl_id()
        .context("Failed to get latest crawl ID")?
        .ok_or_else(|| anyhow::anyhow!("No crawls found in database"))?;

    let stats = storage
        .get_stats(&crawl_id)
        .context("Failed to get crawl statistics")?;

    let backlink_data = if feature_flags.get(crawlkit_engine::feature_flags::FLAG_BACKLINK_ANALYSIS)
        && selected.contains(&Section::Backlinks)
    {
        let link_pairs = storage.get_links_for_crawl(&crawl_id).unwrap_or_default();
        let external_links = storage.get_external_links(&crawl_id).unwrap_or_default();
        let page_urls = storage.get_page_urls(&crawl_id).unwrap_or_default();

        let mut analyzer = crawlkit_engine::BacklinkAnalyzer::new();
        analyzer.load_from_crawl_data(&link_pairs);

        for (source, target) in &external_links {
            analyzer.add_backlink(crawlkit_engine::Backlink {
                source_url: source.clone(),
                target_url: target.clone(),
                anchor_text: String::new(),
                is_followed: true,
                is_internal: false,
            });
        }

        let _known_urls: std::collections::HashSet<String> = page_urls.into_iter().collect();
        let pagerank = analyzer.compute_pagerank(0.85, 20);
        let summary = analyzer.summarize();

        Some((summary, pagerank))
    } else {
        None
    };

    pb.set_message("Generating report...");

    // The previews section is the only consumer of per-page rows, and only
    // the markdown/html renderers (plus the PDF, which prints the HTML) can
    // show it — so the extra storage query runs just for those combinations.
    let preview_pages = if selected.contains(&Section::Previews)
        && (options.pdf.is_some() || matches!(format, "html" | "htm" | "markdown" | "md"))
    {
        storage
            .get_pages(&crawl_id, PREVIEW_PAGE_LIMIT)
            .context("Failed to load pages for share previews")?
    } else {
        Vec::new()
    };

    let report = match format {
        "json" => build_json_report(&crawl_id, &stats, &backlink_data)?,
        "markdown" | "md" => {
            build_markdown_report(&crawl_id, &stats, &backlink_data, &selected, &preview_pages)
        }
        "csv" => build_csv_report(&crawl_id, &stats),
        "html" | "htm" => {
            build_html_report(&crawl_id, &stats, &backlink_data, &selected, &preview_pages)
        }
        other => {
            // Name the values that exist. `--help` previously advertised `html`,
            // which had no implementation, so the error a user hit listed no
            // way forward.
            return Err(anyhow::anyhow!(
                "Unsupported report format '{other}'. Supported: json, markdown (md), html, csv."
            ));
        }
    };

    pb.finish_with_message("Report generated");

    if let Some(out) = output {
        std::fs::write(out, &report)?;
        tracing::info!("Wrote report to {}", out.display());
    } else {
        println!("{}", report);
    }

    if let Some(pdf_path) = &options.pdf {
        let html = build_html_report(&crawl_id, &stats, &backlink_data, &selected, &preview_pages);
        export_pdf(&html, pdf_path)?;
    }

    if ci.enabled {
        run_ci_gate(&stats, ci)?;
    }

    Ok(())
}

/// Apply the CI quality gate after the report has rendered (GAP G4).
///
/// Prints a one-line summary, then exits `10`/`11`/`12` on the first failed
/// threshold via [`ci_exit_code`]. Baseline comparison reuses the engine's
/// stored crawl data: error-severity finding counts on both sides give the
/// "new errors" signal.
fn run_ci_gate(stats: &crawlkit_engine::storage::CrawlStats, ci: &CiGateOptions) -> Result<()> {
    // Critical findings count as errors: a gate that lets criticals pass
    // while failing errors would hold the page to a weaker standard than
    // the findings themselves do.
    let errors = severity_total(stats, &["critical", "error"]);
    let warnings = severity_total(stats, &["warning"]);

    let mut gate = ci.clone();
    gate.new_errors_vs_baseline = 0;
    let mut baseline_summary = String::from("no baseline");
    if let Some(baseline) = &ci.baseline {
        let baseline_db = db_path_for(baseline);
        let baseline_storage = Storage::new(&baseline_db).with_context(|| {
            format!(
                "Failed to open baseline crawl database: {}",
                baseline_db.display()
            )
        })?;
        let baseline_id = baseline_storage
            .get_latest_crawl_id()
            .context("Failed to get latest crawl ID from baseline database")?
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "No crawls found in baseline database: {}",
                    baseline_db.display()
                )
            })?;
        let baseline_stats = baseline_storage
            .get_stats(&baseline_id)
            .context("Failed to get baseline crawl statistics")?;
        let baseline_errors = severity_total(&baseline_stats, &["critical", "error"]);
        gate.new_errors_vs_baseline = errors.saturating_sub(baseline_errors);
        baseline_summary = if gate.new_errors_vs_baseline > 0 {
            format!(
                "baseline: {baseline_errors} errors (+{} new)",
                gate.new_errors_vs_baseline
            )
        } else {
            format!("baseline: {baseline_errors} errors (+0 new)")
        };
    }

    let verdict = match ci_exit_code(errors, warnings, None, &gate) {
        None => "PASS".to_string(),
        Some(code) => format!(
            "FAIL exit {code} ({})",
            match code {
                10 => "error budget exceeded or new errors vs baseline",
                11 => "warning budget exceeded",
                12 => "score below floor",
                _ => "unknown gate condition",
            }
        ),
    };

    println!(
        "CI gate: {} pages, {errors} errors, {warnings} warnings, \
         max-errors {}, max-warnings {}, {baseline_summary}; {verdict}",
        stats.total_pages,
        ci.max_errors,
        ci.max_warnings
            .map_or_else(|| "unbounded".to_string(), |m| m.to_string()),
    );

    if let Some(code) = ci_exit_code(errors, warnings, None, &gate) {
        exit_with_ci_code(code);
    }
    Ok(())
}

fn build_json_report(
    crawl_id: &str,
    stats: &crawlkit_engine::storage::CrawlStats,
    backlink_data: &Option<(
        crawlkit_engine::BacklinkSummary,
        std::collections::HashMap<String, f64>,
    )>,
) -> Result<String> {
    let mut data = serde_json::json!({
        "crawl_id": crawl_id,
        "total_pages": stats.total_pages,
        "total_issues": stats.total_issues,
        "issues_by_severity": stats.issues_by_severity,
        "issues_by_category": stats.issues_by_category,
        "avg_response_time_ms": stats.avg_response_time_ms,
        "total_body_size": stats.total_body_size,
        "status": "completed",
    });

    if let Some((summary, _pagerank)) = backlink_data {
        let top_pages: Vec<serde_json::Value> = summary
            .pages
            .iter()
            .take(20)
            .map(|p| {
                serde_json::json!({
                    "url": p.url,
                    "pagerank": p.pagerank,
                    "inbound_links": p.inbound_links,
                    "outbound_links": p.outbound_links,
                    "referring_domains": p.referring_domains,
                })
            })
            .collect();

        data["backlinks"] = serde_json::json!({
            "total_internal_links": summary.total_internal_links,
            "total_external_links": summary.total_external_links,
            "total_referring_domains": summary.total_referring_domains,
            "orphan_pages": summary.orphan_pages,
            "top_pages_by_pagerank": top_pages,
        });
    }

    Ok(serde_json::to_string_pretty(&data)?)
}

fn build_markdown_report(
    crawl_id: &str,
    stats: &crawlkit_engine::storage::CrawlStats,
    backlink_data: &Option<(
        crawlkit_engine::BacklinkSummary,
        std::collections::HashMap<String, f64>,
    )>,
    sections: &[Section],
    preview_pages: &[PageData],
) -> String {
    let mut md = format!(
        "# Crawl Report\n\n\
        - **Crawl ID:** {}\n\
        - **Total Pages:** {}\n\
        - **Total Issues:** {}\n\
        - **Avg Response Time:** {:.2}ms\n\
        - **Total Body Size:** {} bytes\n\
        - **Status:** Completed\n",
        crawl_id,
        stats.total_pages,
        stats.total_issues,
        stats.avg_response_time_ms.unwrap_or(0.0),
        stats.total_body_size.unwrap_or(0)
    );

    if sections.contains(&Section::Backlinks) {
        if let Some((summary, _)) = backlink_data {
            md.push_str(&format!(
                "\n## Backlinks\n\n\
            - **Total Internal Links:** {}\n\
            - **Total External Links:** {}\n\
            - **Total Referring Domains:** {}\n\
            - **Orphan Pages:** {}\n",
                summary.total_internal_links,
                summary.total_external_links,
                summary.total_referring_domains,
                summary.orphan_pages.len()
            ));

            if !summary.orphan_pages.is_empty() {
                md.push_str("\n### Orphan Pages\n\n");
                for url in summary.orphan_pages.iter().take(10) {
                    md.push_str(&format!("- {url}\n"));
                }
                if summary.orphan_pages.len() > 10 {
                    md.push_str(&format!(
                        "\n... and {} more\n",
                        summary.orphan_pages.len() - 10
                    ));
                }
            }

            md.push_str("\n### Top Pages by PageRank\n\n");
            md.push_str("| URL | PageRank | Inbound | Outbound | Referring Domains |\n");
            md.push_str("|-----|----------|---------|----------|------------------|\n");
            for page in summary.pages.iter().take(10) {
                md.push_str(&format!(
                    "| {} | {:.4} | {} | {} | {} |\n",
                    page.url,
                    page.pagerank,
                    page.inbound_links,
                    page.outbound_links,
                    page.referring_domains
                ));
            }
        }
    }

    if sections.contains(&Section::Previews) {
        md.push_str(&render_share_previews_markdown(preview_pages));
    }

    md
}

fn build_csv_report(crawl_id: &str, stats: &crawlkit_engine::storage::CrawlStats) -> String {
    let mut csv = String::from("crawl_id,total_pages,total_issues,status\n");
    csv.push_str(&format!(
        "{},{},{},completed\n",
        crawl_id, stats.total_pages, stats.total_issues
    ));
    csv
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    /// Default CLI settings: gate on, zero tolerance for errors, warnings
    /// unbounded, no score floor, no baseline.
    fn opts() -> CiGateOptions {
        CiGateOptions {
            enabled: true,
            max_errors: 0,
            max_warnings: None,
            min_score: None,
            baseline: None,
            new_errors_vs_baseline: 0,
        }
    }

    #[test]
    fn everything_within_budget_passes() {
        assert_eq!(ci_exit_code(0, 0, None, &opts()), None);
        assert_eq!(ci_exit_code(0, 5_000, None, &opts()), None);
    }

    #[test]
    fn any_error_fails_the_default_budget() {
        assert_eq!(ci_exit_code(1, 0, None, &opts()), Some(10));
    }

    #[test]
    fn error_budget_is_inclusive() {
        let mut o = opts();
        o.max_errors = 5;
        assert_eq!(ci_exit_code(5, 0, None, &o), None, "at the budget passes");
        assert_eq!(ci_exit_code(6, 0, None, &o), Some(10));
    }

    #[test]
    fn warnings_unbounded_without_a_ceiling() {
        assert_eq!(ci_exit_code(0, usize::MAX, None, &opts()), None);
    }

    #[test]
    fn warning_budget_fails_with_11() {
        let mut o = opts();
        o.max_warnings = Some(3);
        assert_eq!(ci_exit_code(0, 3, None, &o), None, "at the budget passes");
        assert_eq!(ci_exit_code(0, 4, None, &o), Some(11));
    }

    #[test]
    fn error_failure_takes_precedence_over_warning_failure() {
        let mut o = opts();
        o.max_warnings = Some(0);
        assert_eq!(ci_exit_code(1, 1, None, &o), Some(10));
    }

    #[test]
    fn score_floor_fails_with_12() {
        let mut o = opts();
        o.min_score = Some(80.0);
        assert_eq!(ci_exit_code(0, 0, Some(79.999), &o), Some(12));
        assert_eq!(
            ci_exit_code(0, 0, Some(80.0), &o),
            None,
            "at the floor passes"
        );
    }

    #[test]
    fn score_is_ignored_without_a_floor_and_a_missing_score_cannot_fail_one() {
        assert_eq!(ci_exit_code(0, 0, Some(1.0), &opts()), None);
        let mut o = opts();
        o.min_score = Some(80.0);
        assert_eq!(ci_exit_code(0, 0, None, &o), None);
    }

    #[test]
    fn baseline_regressions_fail_even_under_lenient_budgets() {
        let mut o = opts();
        o.max_errors = 100;
        o.new_errors_vs_baseline = 1;
        assert_eq!(ci_exit_code(4, 0, None, &o), Some(10));
        o.new_errors_vs_baseline = 0;
        assert_eq!(
            ci_exit_code(4, 0, None, &o),
            None,
            "no regression: budget governs"
        );
    }

    #[test]
    fn severity_totals_sum_critical_into_errors() {
        let stats = crawlkit_engine::storage::CrawlStats {
            total_pages: 10,
            total_issues: 17,
            issues_by_severity: [
                ("critical".to_string(), 2),
                ("error".to_string(), 4),
                ("warning".to_string(), 8),
                ("info".to_string(), 3),
            ]
            .into_iter()
            .collect(),
            issues_by_category: Default::default(),
            avg_response_time_ms: None,
            total_body_size: None,
        };
        assert_eq!(severity_total(&stats, &["critical", "error"]), 6);
        assert_eq!(severity_total(&stats, &["warning"]), 8);
        assert_eq!(severity_total(&stats, &["nonexistent"]), 0);
    }

    // ---- GAP G9: share previews ---------------------------------------

    /// A stored page with only the fields the previews read; everything else
    /// stays at `PageData::default`.
    fn preview_page(url: &str, title: Option<&str>, description: Option<&str>) -> PageData {
        PageData {
            url: url::Url::parse(url).unwrap(),
            final_url: url::Url::parse(url).unwrap(),
            title: title.map(str::to_string),
            description: description.map(str::to_string),
            ..PageData::default()
        }
    }

    fn empty_backlink_data() -> Option<(
        crawlkit_engine::BacklinkSummary,
        std::collections::HashMap<String, f64>,
    )> {
        None
    }

    fn stats() -> crawlkit_engine::storage::CrawlStats {
        crawlkit_engine::storage::CrawlStats {
            total_pages: 3,
            total_issues: 4,
            ..Default::default()
        }
    }

    #[test]
    fn previews_html_names_every_platform() {
        let pages = vec![preview_page(
            "https://example.com/a",
            Some("A"),
            Some("Desc"),
        )];
        let html = render_share_previews_html(&pages);
        for platform in ["Google", "Facebook", "X", "LinkedIn", "iMessage"] {
            assert!(
                html.contains(&format!(">{platform}</span>")),
                "missing platform frame: {platform}"
            );
        }
    }

    #[test]
    fn previews_html_uses_stored_meta() {
        let pages = vec![preview_page(
            "https://example.com/post",
            Some("Stored title"),
            Some("Stored description"),
        )];
        let html = render_share_previews_html(&pages);
        assert!(html.contains("Stored title"), "stored title must render");
        assert!(
            html.contains("Stored description"),
            "stored description must render"
        );
        assert!(html.contains("example.com"), "site name comes from the URL");
        assert!(
            html.contains("https://example.com/post"),
            "page URL anchors the preview group"
        );
    }

    #[test]
    fn previews_html_falls_back_when_meta_is_missing() {
        let pages = vec![preview_page("https://example.com/bare", None, None)];
        let html = render_share_previews_html(&pages);
        assert!(html.contains("Untitled page"));
        assert!(html.contains("No meta description stored"));
    }

    /// The previews renderer must be a pure function of its input: two calls
    /// on the same stored pages produce byte-identical HTML (determinism
    /// rules), so the PDF built from it is reproducible too.
    #[test]
    fn previews_html_is_byte_identical_across_calls() {
        let pages = vec![
            preview_page("https://example.com/one", Some("One"), Some("d1")),
            preview_page("https://example.com/two", Some("Two"), Some("d2")),
        ];
        assert_eq!(
            render_share_previews_html(&pages),
            render_share_previews_html(&pages)
        );
    }

    #[test]
    fn previews_html_escapes_untrusted_meta() {
        let pages = vec![preview_page(
            "https://example.com/x",
            Some("<script>alert(1)</script>"),
            Some("a & b"),
        )];
        let html = render_share_previews_html(&pages);
        assert!(!html.contains("<script>alert"), "meta must not inject HTML");
        assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
        assert!(html.contains("a &amp; b"));
    }

    #[test]
    fn previews_html_is_capped_at_the_page_limit() {
        let pages: Vec<PageData> = (0..PREVIEW_PAGE_LIMIT + 5)
            .map(|i| preview_page(&format!("https://example.com/p{i}"), Some("t"), Some("d")))
            .collect();
        let html = render_share_previews_html(&pages);
        let last = format!("/p{}", PREVIEW_PAGE_LIMIT - 1);
        let first_beyond = format!("/p{PREVIEW_PAGE_LIMIT}");
        assert!(html.contains(&last), "the last capped page renders");
        assert!(!html.contains(&first_beyond), "pages beyond the cap do not");
    }

    #[test]
    fn previews_markdown_lists_card_text_and_frames_are_html_only() {
        let pages = vec![preview_page(
            "https://example.com/post",
            Some("Stored title"),
            Some("Stored description"),
        )];
        let md = render_share_previews_markdown(&pages);
        assert!(md.contains("## Social share previews"));
        assert!(md.contains("Stored title"));
        assert!(md.contains("Stored description"));
        assert!(
            render_share_previews_markdown(&[]).contains("No pages stored"),
            "an empty crawl still renders a bounded section"
        );
    }

    #[test]
    fn default_sections_preserve_pre_g9_output() {
        assert_eq!(
            parse_sections(&[]).unwrap(),
            vec![Section::Summary, Section::Backlinks]
        );
    }

    #[test]
    fn sections_parse_comma_lists_and_repeats_without_duplication() {
        let values = vec![
            "previews,summary".to_string(),
            "summary".to_string(),
            "backlinks".to_string(),
        ];
        assert_eq!(
            parse_sections(&values).unwrap(),
            vec![Section::Previews, Section::Summary, Section::Backlinks,]
        );
    }

    #[test]
    fn all_selects_every_section_including_previews() {
        assert_eq!(
            parse_sections(&["all".to_string()]).unwrap(),
            vec![Section::Summary, Section::Backlinks, Section::Previews]
        );
    }

    #[test]
    fn unknown_sections_are_rejected_not_ignored() {
        assert!(parse_sections(&["nope".to_string()]).is_err());
    }

    #[test]
    fn html_report_renders_selected_sections_deterministically() {
        let pages = vec![preview_page("https://example.com/", Some("T"), Some("D"))];
        let selection = [Section::Summary, Section::Previews];
        let html = build_html_report(
            "crawl-1",
            &stats(),
            &empty_backlink_data(),
            &selection,
            &pages,
        );
        assert!(html.contains("<h2>Summary</h2>"));
        assert!(html.contains("Social share previews"));
        assert!(!html.contains("<h2>Backlinks</h2>"));
        assert_eq!(
            html,
            build_html_report(
                "crawl-1",
                &stats(),
                &empty_backlink_data(),
                &selection,
                &pages,
            ),
            "the HTML report must be byte-identical across calls"
        );
    }

    #[test]
    fn html_report_omits_previews_by_default() {
        let pages = vec![preview_page("https://example.com/", Some("T"), Some("D"))];
        let html = build_html_report(
            "crawl-1",
            &stats(),
            &empty_backlink_data(),
            &parse_sections(&[]).unwrap(),
            &pages,
        );
        assert!(!html.contains("Social share previews"));
    }

    #[test]
    fn markdown_report_renders_previews_only_when_selected() {
        let pages = vec![preview_page(
            "https://example.com/post",
            Some("T"),
            Some("D"),
        )];
        let default_md = build_markdown_report(
            "crawl-1",
            &stats(),
            &empty_backlink_data(),
            &parse_sections(&[]).unwrap(),
            &[],
        );
        assert!(!default_md.contains("Social share previews"));

        let previews_md = build_markdown_report(
            "crawl-1",
            &stats(),
            &empty_backlink_data(),
            &[Section::Previews],
            &pages,
        );
        assert!(previews_md.contains("## Social share previews"));

        let summary_only = build_markdown_report(
            "crawl-1",
            &stats(),
            &empty_backlink_data(),
            &[Section::Summary],
            &[],
        );
        assert!(!summary_only.contains("## Backlinks"));
    }

    // ---- GAP G10: PDF export -------------------------------------------

    /// Write a file into `dir` with (or without) an execute bit, standing in
    /// for a discovered binary.
    #[cfg(unix)]
    fn write_fake_binary(dir: &Path, name: &str, executable: bool) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let path = dir.join(name);
        std::fs::write(&path, b"#!/bin/sh\n").unwrap();
        let mode = u32::from(executable) * 0o755;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).unwrap();
        path
    }

    #[cfg(unix)]
    #[test]
    fn executable_lookup_walks_the_path_in_order() {
        let first = tempfile::tempdir().unwrap();
        let found = write_fake_binary(first.path(), "chromium", true);
        let second = tempfile::tempdir().unwrap();
        let _ = write_fake_binary(second.path(), "chromium", true);
        let search = format!("{}:{}", first.path().display(), second.path().display());
        assert_eq!(find_executable_in_path(&search, "chromium"), Some(found));
        assert_eq!(find_executable_in_path(&search, "missing"), None);
    }

    #[cfg(unix)]
    #[test]
    fn non_executable_files_are_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let _ = write_fake_binary(dir.path(), "weasyprint", false);
        let search = dir.path().to_str().unwrap();
        assert_eq!(find_executable_in_path(search, "weasyprint"), None);
    }

    #[cfg(unix)]
    #[test]
    fn weasyprint_beats_chromium_when_both_are_installed() {
        let dir = tempfile::tempdir().unwrap();
        let weasyprint = write_fake_binary(dir.path(), "weasyprint", true);
        let _ = write_fake_binary(dir.path(), "chromium", true);
        let search = dir.path().to_str().unwrap();
        assert_eq!(
            resolve_pdf_engine_from_path(search),
            Some((PdfEngine::WeasyPrint, weasyprint))
        );
    }

    #[cfg(unix)]
    #[test]
    fn chromium_browser_is_a_chromium_candidate() {
        let dir = tempfile::tempdir().unwrap();
        let browser = write_fake_binary(dir.path(), "chromium-browser", true);
        let search = dir.path().to_str().unwrap();
        assert_eq!(
            resolve_pdf_engine_from_path(search),
            Some((PdfEngine::Chromium, browser))
        );
    }

    #[test]
    fn empty_path_resolves_no_engine() {
        assert_eq!(resolve_pdf_engine_from_path(""), None);
    }

    #[test]
    fn weasyprint_print_command_takes_html_then_pdf() {
        let command = pdf_print_command(
            PdfEngine::WeasyPrint,
            Path::new("/opt/wp/bin/weasyprint"),
            Path::new("/tmp/report.html"),
            Path::new("/out/report.pdf"),
        );
        let args: Vec<String> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            args,
            vec![
                "/tmp/report.html".to_string(),
                "/out/report.pdf".to_string()
            ]
        );
    }

    #[test]
    fn chromium_print_command_uses_headless_print_to_pdf() {
        let command = pdf_print_command(
            PdfEngine::Chromium,
            Path::new("/usr/bin/chromium"),
            Path::new("/tmp/report.html"),
            Path::new("/out/report.pdf"),
        );
        let args: Vec<String> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            args,
            vec![
                "--headless".to_string(),
                "--print-to-pdf=/out/report.pdf".to_string(),
                "file:///tmp/report.html".to_string(),
            ]
        );
    }

    #[test]
    fn fast_subprocess_completes_under_the_timeout() {
        let mut command = std::process::Command::new("sh");
        command.arg("-c").arg("exit 0");
        let status = run_with_timeout(command, Duration::from_secs(5)).unwrap();
        assert!(status.success());
    }

    /// A subprocess that outlives its budget must be killed and reported, not
    /// waited on forever. Uses a short budget against a 30 s sleeper, so the
    /// test itself stays fast and needs no real chromium.
    #[test]
    fn hung_subprocess_is_killed_at_the_timeout() {
        let mut command = std::process::Command::new("sh");
        command.arg("-c").arg("sleep 30");
        let started = Instant::now();
        let outcome = run_with_timeout(command, Duration::from_millis(150));
        assert!(
            outcome.is_err(),
            "a hung subprocess must not report success"
        );
        let message = outcome
            .err()
            .map(|error| error.to_string())
            .unwrap_or_default();
        assert!(message.contains("timed out"), "message: {message}");
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "the kill must happen at the budget, not after a full default timeout"
        );
    }
}
