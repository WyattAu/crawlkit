//! Hosted free-scan scanner (ADR-012) — scan engine.
//!
//! Runs a bounded, read-only BFS over the submitted URL and its same-host
//! links, then analyzes each fetched page with the engine's default analyzer
//! registry. Plugins are never loaded: the registry is analyzer-only, keeping
//! the scanner out of the ADR-003 plugin threat model.
//!
//! Network access is confined behind the [`Fetch`] trait. The production
//! implementation ([`PinnedFetcher`]) enforces the ADR-012 trust boundary
//! (validation + DNS pinning per hop). Tests substitute a fixture fetcher;
//! the boundary code is exercised by its own unit tests, never bypassed via
//! configuration flags — there are none.

use std::collections::{HashSet, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crawlkit_engine::analyzers::AnalyzerRegistry;
use crawlkit_engine::{AnalysisContext, RedirectHop};
use url::Url;

use crate::bounds::{BudgetError, BudgetStore, MAX_DEPTH, MAX_PAGES, MAX_WALL_CLOCK};
use crate::guard;

/// User agent sent for every scanner request.
pub const SCANNER_USER_AGENT: &str = concat!(
    "crawlkit-scanner/",
    env!("CARGO_PKG_VERSION"),
    " (free-scan; +https://crawlkit.dev/scanner)"
);

/// One fetched page (or failed fetch attempt).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PageRecord {
    pub url: String,
    /// HTTP status; `None` when the fetch itself failed.
    pub status: Option<u16>,
    pub response_time_ms: Option<u64>,
    pub error: Option<String>,
}

/// Aggregated finding summary for a scan result (ADR-012 §4: summary only,
/// token-addressable, no account linkage).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FindingSummary {
    pub code: String,
    pub severity: String,
    pub title: String,
    pub occurrences: usize,
    /// Up to three sample URLs exhibiting the issue.
    pub sample_urls: Vec<String>,
}

/// Why a scan stopped before its page ceiling, if it did.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TruncationReason {
    /// Page ceiling (25) reached.
    PageCeiling,
    /// Wall-clock budget exhausted.
    WallClock,
    /// The target's global politeness budget was exhausted.
    TargetBudget,
    /// Depth limit reached with frontier remaining.
    DepthCeiling,
}

/// Final report for one scan.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScanReport {
    pub submitted_url: String,
    pub pages_scanned: usize,
    pub pages: Vec<PageRecord>,
    pub findings: Vec<FindingSummary>,
    /// Set when the scan stopped before exhausting its own frontier.
    pub truncated_reason: Option<TruncationReason>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub finished_at: chrono::DateTime<chrono::Utc>,
}

/// Terminal state of a scan attempt.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanOutcome {
    Complete(Box<ScanReport>),
    /// robots.txt disallows the submitted path for `*` (or it 404s nothing —
    /// only explicit disallow blocks; a missing robots.txt never blocks).
    RobotsBlocked {
        submitted_url: String,
    },
    /// The target failed validation before any request was issued.
    Rejected {
        reason: String,
    },
    /// Scan could not run (e.g., budget exhausted before the first fetch).
    BudgetExhausted {
        retry_after_secs: u64,
    },
}

/// A fetched response handed to the scan engine.
#[derive(Debug, Clone)]
pub struct FetchOutcome {
    pub status: u16,
    /// Lowercased header name -> first value.
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub elapsed: Duration,
    /// Redirect hops already followed by the fetcher to produce this
    /// response (each hop individually validated by the fetcher).
    pub redirect_chain: Vec<RedirectHop>,
    /// Fetch-layer failure (request error, policy rejection, body-cap hit).
    /// `status` is 0 when no response was received at all.
    pub error: Option<String>,
}

/// Network boundary. `fetch` must issue at most one HTTP request cycle for
/// `url` (following redirects itself, validating each hop) and return the
/// final response. Implementations must enforce the guard rules; the engine
/// layer re-checks nothing on the hot path by design.
pub trait Fetch: Send + Sync {
    fn fetch<'a>(&'a self, url: &'a Url)
        -> Pin<Box<dyn Future<Output = FetchOutcome> + Send + 'a>>;
}

/// Dependencies for one scan run. The fetcher is trait-object typed so the
/// same path serves inline requests and queue-consuming workers.
pub struct ScanDeps {
    pub fetcher: Arc<dyn Fetch>,
    pub budget: Arc<dyn BudgetStore>,
    pub registry: Arc<AnalyzerRegistry>,
}

/// Run one bounded scan for `submitted`.
///
/// Ordering of trust decisions (ADR-012): static validation → budget slot →
/// robots gate → BFS with per-page budget slots → analysis. Every fetch
/// consumes exactly one politeness slot, robots included, so concurrent
/// submitters for the same host share a hard ceiling.
pub async fn run_scan(submitted: &str, deps: ScanDeps) -> ScanOutcome {
    let started_at = chrono::Utc::now();
    let start = Instant::now();

    let url = match guard::validate_target(submitted) {
        Ok(u) => u,
        Err(e) => {
            return ScanOutcome::Rejected {
                reason: e.to_string(),
            }
        }
    };
    let submitted_url = url.clone();

    let Some(host) = scan_host(&url) else {
        return ScanOutcome::Rejected {
            reason: "URL has no host".to_string(),
        };
    };

    // Politeness slot for the robots fetch.
    if let Err(e) = deps.budget.consume(&host, now_ms()).await {
        return budget_outcome(e);
    }

    // Robots gate: fetch and evaluate before any page request.
    let robots_url = robots_txt_url(&url);
    let robots_fetch = deps.fetcher.fetch(&robots_url).await;
    let robots_text = if robots_fetch.status == 200 {
        Some(String::from_utf8_lossy(&robots_fetch.body).into_owned())
    } else {
        None
    };
    if let Some(text) = &robots_text {
        if robots_disallows(text, url.path()) {
            return ScanOutcome::RobotsBlocked {
                submitted_url: submitted_url.to_string(),
            };
        }
    }

    // BFS over same-host links.
    let mut queue: VecDeque<(Url, usize)> = VecDeque::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut pages: Vec<PageRecord> = Vec::new();
    let mut all_findings: Vec<crawlkit_types::Finding> = Vec::new();
    let mut truncation: Option<TruncationReason> = None;

    queue.push_back((submitted_url.clone(), 0));
    seen.insert(normalize(&submitted_url));

    while let Some((next, depth)) = queue.pop_front() {
        if pages.len() >= MAX_PAGES {
            truncation = Some(TruncationReason::PageCeiling);
            break;
        }
        if start.elapsed() >= MAX_WALL_CLOCK {
            truncation = Some(TruncationReason::WallClock);
            break;
        }
        // Budget slot per page fetch (shared with concurrent submitters).
        if let Err(e) = deps.budget.consume(&host, now_ms()).await {
            truncation = Some(TruncationReason::TargetBudget);
            let _ = e;
            break;
        }

        let fetch = deps.fetcher.fetch(&next).await;
        let mut record = PageRecord {
            url: next.to_string(),
            status: Some(fetch.status),
            response_time_ms: Some(u64::try_from(fetch.elapsed.as_millis()).unwrap_or(u64::MAX)),
            error: fetch.error.clone(),
        };
        if (200..400).contains(&fetch.status) && is_html(&fetch.headers) {
            let page_url = next.clone();
            let body_text = String::from_utf8_lossy(&fetch.body).into_owned();
            let parsed = crawlkit_engine::parser::HtmlParser::parse(&body_text, &page_url);
            let ctx = AnalysisContext {
                page: &parsed,
                body: Some(&body_text),
                status_code: Some(fetch.status),
                headers: &fetch.headers,
                response_time: Some(fetch.elapsed),
                redirect_chain: &fetch.redirect_chain,
                robots_txt: robots_text.as_deref(),
                body_size: Some(fetch.body.len()),
                compressed_size: Some(fetch.body.len()),
                server: header_value(&fetch.headers, "server"),
                content_type: header_value(&fetch.headers, "content-type"),
                rendered: None,
            };
            all_findings.extend(deps.registry.analyze(&ctx));

            // Enqueue same-host links within depth budget. Links discovered
            // at the depth ceiling are counted so the report can say the
            // depth limit left frontier unexpanded.
            let mut blocked_by_depth = 0usize;
            for link in &parsed.links {
                if link.is_external {
                    continue;
                }
                let Ok(link_url) = Url::parse(&link.href) else {
                    continue;
                };
                if scan_host(&link_url).as_deref() != Some(host.as_str()) {
                    continue;
                }
                if depth < MAX_DEPTH {
                    if seen.insert(normalize(&link_url)) {
                        queue.push_back((link_url, depth + 1));
                    }
                } else {
                    blocked_by_depth += 1;
                }
            }
            if blocked_by_depth > 0 && truncation.is_none() {
                truncation = Some(TruncationReason::DepthCeiling);
            }
        } else if fetch.status >= 400 || fetch.status < 200 {
            // Fetch-layer errors (status 0 / `error` set) take precedence;
            // otherwise record the HTTP status as the page error.
            if record.error.is_none() {
                record.error = Some(format!("http status {}", fetch.status));
            }
        }
        pages.push(record);
    }

    let report = ScanReport {
        submitted_url: submitted_url.to_string(),
        pages_scanned: pages.len(),
        pages,
        findings: summarize(&all_findings),
        truncated_reason: truncation,
        started_at,
        finished_at: chrono::Utc::now(),
    };
    ScanOutcome::Complete(Box::new(report))
}

fn budget_outcome(e: BudgetError) -> ScanOutcome {
    let BudgetError::Exhausted { retry_after_secs } = e;
    ScanOutcome::BudgetExhausted { retry_after_secs }
}

/// Lowercased, port-stripped host for budget keys and same-host checks.
fn scan_host(url: &Url) -> Option<String> {
    url.host_str()
        .map(|h| h.trim_end_matches('.').to_ascii_lowercase())
}

fn normalize(url: &Url) -> String {
    let mut s = url.as_str().trim_end_matches('/').to_string();
    if s.is_empty() {
        s = url.as_str().to_string();
    }
    s.to_ascii_lowercase()
}

fn robots_txt_url(base: &Url) -> Url {
    let mut origin = base.origin().ascii_serialization();
    if origin.ends_with('/') {
        origin.pop();
    }
    Url::parse(&format!("{origin}/robots.txt")).unwrap_or_else(|_| base.clone())
}

/// Minimal robots evaluation: `User-agent: *` blocks whose `Disallow:`
/// prefixes match `path`. `Allow:` directives are ignored — blocking is the
/// safe direction for a teaser surface. A missing or unparseable robots.txt
/// never blocks.
fn robots_disallows(robots: &str, path: &str) -> bool {
    let mut in_star = false;
    let mut disallows: Vec<String> = Vec::new();
    for line in robots.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim().to_ascii_lowercase();
        let value = value.trim();
        match key.as_str() {
            "user-agent" => in_star = value == "*",
            "disallow" if in_star && !value.is_empty() => disallows.push(value.to_string()),
            _ => {}
        }
    }
    disallows.iter().any(|d| path.starts_with(d.as_str()))
}

fn is_html(headers: &[(String, String)]) -> bool {
    header_value(headers, "content-type")
        .map(|v| v.to_ascii_lowercase().contains("text/html"))
        .unwrap_or(false)
}

fn header_value<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.as_str())
}

/// Group raw findings into summaries keyed by code, sorted by severity then
/// occurrences (worst first).
fn summarize(findings: &[crawlkit_types::Finding]) -> Vec<FindingSummary> {
    use std::collections::BTreeMap;
    let mut order: Vec<String> = Vec::new();
    let mut grouped: BTreeMap<String, FindingSummary> = BTreeMap::new();
    for f in findings {
        let entry = grouped.entry(f.code.clone()).or_insert_with(|| {
            order.push(f.code.clone());
            FindingSummary {
                code: f.code.clone(),
                severity: f.severity.as_str().to_string(),
                title: f.title.clone(),
                occurrences: 0,
                sample_urls: Vec::new(),
            }
        });
        entry.occurrences += 1;
        if entry.sample_urls.len() < 3 && !entry.sample_urls.contains(&f.url) {
            entry.sample_urls.push(f.url.clone());
        }
    }
    let mut out: Vec<FindingSummary> = grouped.into_values().collect();
    out.sort_by(|a, b| {
        severity_rank(&a.severity)
            .cmp(&severity_rank(&b.severity))
            .then(b.occurrences.cmp(&a.occurrences))
            .then(a.code.cmp(&b.code))
    });
    out
}

fn severity_rank(sev: &str) -> u8 {
    match sev {
        "critical" => 0,
        "error" => 1,
        "warning" => 2,
        _ => 3,
    }
}

fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bounds::InMemoryBudgetStore;
    use crawlkit_engine::CrawlConfig;

    /// Fixture fetcher: serves canned pages by path from a map. Performs no
    /// network I/O; the guard boundary is tested separately in guard.rs.
    struct FixtureFetcher {
        pages: std::collections::HashMap<String, (u16, String)>,
    }

    impl Fetch for FixtureFetcher {
        fn fetch<'a>(
            &'a self,
            url: &'a Url,
        ) -> Pin<Box<dyn Future<Output = FetchOutcome> + Send + 'a>> {
            Box::pin(async move {
                let key = format!("{}{}", url.host_str().unwrap_or(""), url.path());
                match self.pages.get(&key) {
                    Some((status, body)) => FetchOutcome {
                        status: *status,
                        headers: vec![("content-type".into(), "text/html".into())],
                        body: body.as_bytes().to_vec(),
                        elapsed: Duration::from_millis(5),
                        redirect_chain: Vec::new(),
                        error: None,
                    },
                    None => FetchOutcome {
                        status: 404,
                        headers: vec![],
                        body: Vec::new(),
                        elapsed: Duration::from_millis(1),
                        redirect_chain: Vec::new(),
                        error: None,
                    },
                }
            })
        }
    }

    fn fixture_deps<I, S, B>(pages: I) -> ScanDeps
    where
        S: Into<String>,
        B: Into<String>,
        I: IntoIterator<Item = (S, u16, B)>,
    {
        let map = pages
            .into_iter()
            .map(|(path, status, body)| (path.into(), (status, body.into())))
            .collect();
        ScanDeps {
            fetcher: Arc::new(FixtureFetcher { pages: map }),
            budget: Arc::new(InMemoryBudgetStore::new()),
            registry: Arc::new(AnalyzerRegistry::new(&CrawlConfig::default())),
        }
    }

    fn page(links: &[String]) -> String {
        let hrefs: String = links
            .iter()
            .map(|l| format!("<a href=\"{l}\">link</a>"))
            .collect();
        format!(
            "<!DOCTYPE html><html><head><title>t</title></head><body>{hrefs}<p>content words here</p></body></html>"
        )
    }

    #[tokio::test]
    async fn scans_single_page_within_bounds() {
        let deps = fixture_deps(vec![("example.com/", 200, page(&[]))]);
        let outcome = run_scan("https://example.com/", deps).await;
        let ScanOutcome::Complete(report) = outcome else {
            panic!("expected complete, got {outcome:?}");
        };
        assert_eq!(report.pages_scanned, 1);
        assert!(report.truncated_reason.is_none());
        assert!(!report.pages.is_empty());
    }

    #[tokio::test]
    async fn follows_same_host_links_up_to_depth() {
        let deps = fixture_deps(vec![
            (
                "example.com/",
                200,
                page(&["/a".to_string(), "/b".to_string()]),
            ),
            ("example.com/a", 200, page(&[])),
            ("example.com/b", 200, page(&[])),
        ]);
        let outcome = run_scan("https://example.com/", deps).await;
        let ScanOutcome::Complete(report) = outcome else {
            panic!("expected complete, got {outcome:?}");
        };
        assert_eq!(report.pages_scanned, 3);
    }

    #[tokio::test]
    async fn truncates_at_page_ceiling() {
        // 30 linked pages exceed MAX_PAGES = 25; the scan must stop at 25
        // and report PageCeiling.
        let mut pages: Vec<(String, u16, String)> = Vec::new();
        let root_links: Vec<String> = (1..=30).map(|i| format!("/p{i}")).collect();
        pages.push(("example.com/".into(), 200, page(&root_links)));
        for i in 1..=30 {
            pages.push((format!("/p{i}"), 200, "<html><body>x</body></html>".into()));
        }
        let deps = fixture_deps(pages);
        let outcome = run_scan("https://example.com/", deps).await;
        let ScanOutcome::Complete(report) = outcome else {
            panic!("expected complete, got {outcome:?}");
        };
        assert_eq!(report.pages_scanned, MAX_PAGES);
        assert_eq!(report.truncated_reason, Some(TruncationReason::PageCeiling));
    }

    #[tokio::test]
    async fn truncates_when_target_budget_exhausted() {
        // Budget that allows exactly the robots slot, then exhausts: the
        // first page fetch is refused and the scan reports TargetBudget.
        struct OnceThenFull(std::sync::atomic::AtomicUsize);
        #[async_trait::async_trait]
        impl BudgetStore for OnceThenFull {
            async fn consume(&self, _host: &str, _now_ms: u64) -> Result<(), BudgetError> {
                let n = self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                if n == 0 {
                    Ok(())
                } else {
                    Err(BudgetError::Exhausted {
                        retry_after_secs: 60,
                    })
                }
            }
            async fn sweep(&self, _now_ms: u64) -> usize {
                0
            }
        }
        let deps = ScanDeps {
            fetcher: Arc::new(FixtureFetcher {
                pages: std::collections::HashMap::new(),
            }),
            budget: Arc::new(OnceThenFull(std::sync::atomic::AtomicUsize::new(0))),
            registry: Arc::new(AnalyzerRegistry::new(&CrawlConfig::default())),
        };
        let outcome = run_scan("https://example.com/", deps).await;
        let ScanOutcome::Complete(report) = outcome else {
            panic!("expected complete, got {outcome:?}");
        };
        assert_eq!(report.pages_scanned, 0);
        assert_eq!(
            report.truncated_reason,
            Some(TruncationReason::TargetBudget)
        );
    }

    #[tokio::test]
    async fn robots_disallow_blocks_submission() {
        struct RobotsOnly;
        impl Fetch for RobotsOnly {
            fn fetch<'a>(
                &'a self,
                url: &'a Url,
            ) -> Pin<Box<dyn Future<Output = FetchOutcome> + Send + 'a>> {
                Box::pin(async move {
                    if url.path() == "/robots.txt" {
                        FetchOutcome {
                            status: 200,
                            headers: vec![("content-type".into(), "text/plain".into())],
                            body: b"User-agent: *\nDisallow: /private/".to_vec(),
                            elapsed: Duration::from_millis(1),
                            redirect_chain: Vec::new(),
                            error: None,
                        }
                    } else {
                        FetchOutcome {
                            status: 599,
                            headers: vec![],
                            body: Vec::new(),
                            elapsed: Duration::from_millis(1),
                            redirect_chain: Vec::new(),
                            error: None,
                        }
                    }
                })
            }
        }
        let deps = ScanDeps {
            fetcher: Arc::new(RobotsOnly),
            budget: Arc::new(InMemoryBudgetStore::new()),
            registry: Arc::new(AnalyzerRegistry::new(&CrawlConfig::default())),
        };
        let outcome = run_scan("https://example.com/private/thing", deps).await;
        assert!(matches!(outcome, ScanOutcome::RobotsBlocked { .. }));

        // Same robots, allowed path: proceeds (pages 599 → no scan of pages
        // but outcome Complete with error records).
        let deps2 = ScanDeps {
            fetcher: Arc::new(RobotsOnly),
            budget: Arc::new(InMemoryBudgetStore::new()),
            registry: Arc::new(AnalyzerRegistry::new(&CrawlConfig::default())),
        };
        let outcome2 = run_scan("https://example.com/public", deps2).await;
        assert!(matches!(outcome2, ScanOutcome::Complete(_)));
    }

    #[tokio::test]
    async fn rejects_invalid_target_before_any_fetch() {
        // Validation rejects the target before any fetch can be issued; the
        // fetcher's identity is irrelevant here, so a unit fixture suffices.
        struct Noop;
        impl Fetch for Noop {
            fn fetch<'a>(
                &'a self,
                _url: &'a Url,
            ) -> Pin<Box<dyn Future<Output = FetchOutcome> + Send + 'a>> {
                Box::pin(async move {
                    FetchOutcome {
                        status: 200,
                        headers: vec![],
                        body: Vec::new(),
                        elapsed: Duration::ZERO,
                        redirect_chain: Vec::new(),
                        error: None,
                    }
                })
            }
        }
        let deps = ScanDeps {
            fetcher: Arc::new(Noop),
            budget: Arc::new(InMemoryBudgetStore::new()),
            registry: Arc::new(AnalyzerRegistry::new(&CrawlConfig::default())),
        };
        let outcome = run_scan("http://169.254.169.254/latest/meta-data/", deps).await;
        assert!(matches!(outcome, ScanOutcome::Rejected { .. }));
    }

    #[test]
    fn robots_parser_matches_prefix_disallows_only() {
        let robots = "User-agent: BadBot\nDisallow: /\n\nUser-agent: *\nDisallow: /private\nAllow: /public\n";
        assert!(robots_disallows(robots, "/private/x"));
        assert!(!robots_disallows(robots, "/public/x"));
        assert!(!robots_disallows("", "/anything"));
    }

    #[test]
    fn summarize_groups_by_code_and_sorts_by_severity() {
        use crawlkit_types::{Finding, IssueCategory, Severity};
        let mk = |code: &str, sev: Severity, url: &str| Finding {
            severity: sev,
            category: IssueCategory::Seo,
            code: code.to_string(),
            title: format!("title {code}"),
            description: "d".into(),
            url: url.to_string(),
            recommendation: "r".into(),
        };
        let findings = vec![
            mk("A001", Severity::Warning, "https://e/1"),
            mk("A001", Severity::Warning, "https://e/2"),
            mk("B001", Severity::Error, "https://e/3"),
        ];
        let out = summarize(&findings);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].code, "B001", "errors rank before warnings");
        assert_eq!(out[1].code, "A001");
        assert_eq!(out[1].occurrences, 2);
        assert_eq!(out[1].sample_urls.len(), 2);
    }
}
