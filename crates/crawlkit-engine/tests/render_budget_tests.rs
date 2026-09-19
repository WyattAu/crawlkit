//! Render budget contract tests (6.0.0-alpha.2 exit criterion):
//!
//! "a crawl with a 1-page render budget produces an explicit degradation
//! finding, not a hang."
//!
//! A loopback SPA corpus (pages carrying the `#__next` SPA indicator) is
//! crawled with a counting test renderer. Assertions run against real
//! storage findings and the engine's atomic telemetry:
//!
//! - RENDER001: per-crawl render quota exhausted → the page is analyzed
//!   statically and carries an explicit degradation finding.
//! - RENDER002: per-page budget exhausted (timeout) → same explicit
//!   degradation, plus the budget-exhausted counter.
//! - Unbounded budget keeps the historical behavior (no render findings).
//!
//! The renderer test double honors the budget-then-timeout order: it only
//! sleeps past the deadline when actually invoked, so a quota denial is
//! observable as "renderer not called".

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crawlkit_engine::crawl_engine::{CrawlEngine, CrawlEngineConfig, JsRenderer};
use crawlkit_engine::render_budget::{RenderBudget, RenderGrant};
use crawlkit_engine::storage::Storage;
use crawlkit_engine::storage_trait::StorageBackend;
use crawlkit_engine::{CrawlConfig, IssueFilter};

// ---------------------------------------------------------------------------
// Loopback SPA corpus: index links to N child pages; children are SPA-shaped
// (the `#__next` indicator the JsRenderDecisionEngine keys on).
// ---------------------------------------------------------------------------

struct TestServer {
    url_root: String,
}

impl TestServer {
    fn start(page_count: usize) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                std::thread::spawn(move || serve_connection(stream, page_count));
            }
        });
        Self {
            url_root: format!("http://127.0.0.1:{port}"),
        }
    }

    fn index_url(&self) -> String {
        format!("{}/", self.url_root)
    }
}

fn serve_connection(mut stream: TcpStream, page_count: usize) {
    let mut buf = [0u8; 4096];
    let Ok(n) = stream.read(&mut buf) else {
        return;
    };
    let request = String::from_utf8_lossy(&buf[..n]).to_string();
    let path = request.split_whitespace().nth(1).unwrap_or("/").to_string();

    let body = route(&path, page_count);
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

fn route(path: &str, page_count: usize) -> String {
    if path == "/robots.txt" {
        return "User-agent: *\nAllow: /\n".into();
    }
    if path == "/" {
        let links: String = (0..page_count)
            .map(|i| format!("<a href=\"/page-{i}\">Page {i}</a>\n"))
            .collect();
        return format!(
            "<!DOCTYPE html><html lang=\"en\"><head><title>Budget Index</title>\
             <meta name=\"description\" content=\"render budget index\"></head>\
             <body><h1>Index</h1>{links}<p>{}</p></body></html>",
            "word ".repeat(60)
        );
    }
    if let Some(num) = path.strip_prefix("/page-") {
        // SPA indicator: `#__next` makes the decision engine want a render.
        // Each child embeds its number — byte-identical children would be
        // collapsed by the engine's content dedup, and the budget would
        // never see more than one render candidate.
        return format!(
            "<!DOCTYPE html><html lang=\"en\"><head><title>SPA {num}</title></head>\
             <body><div id=\"__next\">static shell {num}</div><p>{}</p></body></html>",
            format!("unique-{num} ").repeat(20)
        );
    }
    "<html><body>not found</body></html>".to_string()
}

// ---------------------------------------------------------------------------
// Counting test renderer.
// ---------------------------------------------------------------------------

struct CountingRenderer {
    calls: AtomicUsize,
    /// Sleep longer than any budget when set (to trigger per-page timeouts).
    hang: Duration,
}

impl CountingRenderer {
    fn new(hang: Duration) -> Arc<Self> {
        Arc::new(Self {
            calls: AtomicUsize::new(0),
            hang,
        })
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

#[async_trait::async_trait]
impl JsRenderer for CountingRenderer {
    fn is_available(&self) -> bool {
        true
    }

    async fn render(&self, url: &str) -> Result<String, String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if !self.hang.is_zero() {
            tokio::time::sleep(self.hang).await;
        }
        Ok(format!(
            "<html><head><title>Rendered</title></head><body>rendered content for {url}</body></html>"
        ))
    }
}

fn budget_crawl_config(max_pages: usize) -> CrawlEngineConfig {
    CrawlEngineConfig {
        crawl_config: CrawlConfig {
            max_pages,
            concurrency: 2,
            respect_robots_txt: true,
            // The test measures budget semantics, not politeness.
            request_delay: Duration::ZERO,
            ..CrawlConfig::default()
        },
        concurrency: Some(2),
        allow_http: true, // local loopback corpus only
        ..CrawlEngineConfig::default()
    }
}

async fn crawl_and_collect_findings(
    server: &TestServer,
    mut config: CrawlEngineConfig,
) -> Vec<crawlkit_engine::Finding> {
    let storage = Arc::new(Storage::new_in_memory().unwrap());
    config.crawl_config.max_pages = 20;
    let engine = CrawlEngine::new_shared(config, Arc::clone(&storage) as Arc<dyn StorageBackend>);
    let output = engine.run(&server.index_url()).await.unwrap();
    assert_eq!(output.pages_crawled, 4, "index + 3 SPA children");
    let issues = storage
        .get_issues(&output.crawl_id, &IssueFilter::default())
        .unwrap();
    issues
        .into_iter()
        .map(|p| crawlkit_engine::Finding {
            severity: p.severity,
            category: p.category,
            code: p.code,
            title: p.title,
            description: p.description,
            url: String::new(), // Issue carries page_id, not a URL
            recommendation: p.recommendation,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// The exit criterion: quota exhaustion degrades explicitly, no hang.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn one_page_quota_produces_render001_finding_not_a_hang() {
    let server = TestServer::start(3);
    let renderer = CountingRenderer::new(Duration::ZERO);

    let mut config = budget_crawl_config(20);
    config.enable_js_rendering = true;
    config.js_renderer = Some(renderer.clone());
    config.render_budget = Some(Arc::new(RenderBudget::new(Some(1), None)));

    let findings = crawl_and_collect_findings(&server, config).await;

    // Exactly one page rendered; the other SPA pages degraded explicitly.
    assert_eq!(
        renderer.calls(),
        1,
        "quota of 1 must allow exactly one render"
    );
    let render001: Vec<_> = findings.iter().filter(|f| f.code == "RENDER001").collect();
    assert_eq!(
        render001.len(),
        2,
        "3 children - 1 granted render = 2 quota-denied pages: {render001:?}"
    );
    for f in &render001 {
        assert!(
            f.description.contains("render quota"),
            "RENDER001 must say why: {}",
            f.description
        );
    }
}

#[tokio::test]
async fn per_page_timeout_produces_render002_and_counter() {
    let server = TestServer::start(3);
    // Hangs far longer than the budget.
    let renderer = CountingRenderer::new(Duration::from_secs(30));

    let mut config = budget_crawl_config(20);
    config.enable_js_rendering = true;
    config.js_renderer = Some(renderer.clone());
    config.render_budget = Some(Arc::new(RenderBudget::new(
        None,
        Some(Duration::from_millis(200)),
    )));

    let storage = Arc::new(Storage::new_in_memory().unwrap());
    config.crawl_config.max_pages = 20;
    let engine = CrawlEngine::new_shared(config, Arc::clone(&storage) as Arc<dyn StorageBackend>);
    let output = engine.run(&server.index_url()).await.unwrap();
    assert_eq!(output.pages_crawled, 4, "crawl completes — no hang");

    let render002: Vec<_> = storage
        .get_issues(&output.crawl_id, &IssueFilter::default())
        .unwrap()
        .into_iter()
        .filter(|i| i.code == "RENDER002")
        .collect();
    assert_eq!(
        render002.len(),
        3,
        "every rendered page times out under a 200ms budget: {render002:?}"
    );
    for i in &render002 {
        assert!(
            i.description.contains("per-page budget"),
            "RENDER002 must say why: {}",
            i.description
        );
    }
}

#[tokio::test]
async fn unbounded_budget_keeps_historical_behavior() {
    let server = TestServer::start(3);
    let renderer = CountingRenderer::new(Duration::ZERO);

    let mut config = budget_crawl_config(20);
    config.enable_js_rendering = true;
    config.js_renderer = Some(renderer.clone());
    // No budget configured at all.
    config.render_budget = None;

    let storage = Arc::new(Storage::new_in_memory().unwrap());
    config.crawl_config.max_pages = 20;
    let engine = CrawlEngine::new_shared(config, Arc::clone(&storage) as Arc<dyn StorageBackend>);
    let output = engine.run(&server.index_url()).await.unwrap();
    assert_eq!(output.pages_crawled, 4);

    assert_eq!(renderer.calls(), 3, "every SPA page renders with no budget");
    let render_findings: Vec<_> = storage
        .get_issues(&output.crawl_id, &IssueFilter::default())
        .unwrap()
        .into_iter()
        .filter(|i| i.code.starts_with("RENDER"))
        .collect();
    assert!(
        render_findings.is_empty(),
        "no render findings in the unbounded posture: {render_findings:?}"
    );
}

// ---------------------------------------------------------------------------
// Unit: the budget object itself (grant accounting under concurrency).
// ---------------------------------------------------------------------------

#[tokio::test]
async fn budget_grants_are_shared_across_tasks() {
    let budget = Arc::new(RenderBudget::new(Some(2), None));
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    for _ in 0..5 {
        let b = budget.clone();
        let tx = tx.clone();
        tokio::spawn(async move {
            tx.send(b.acquire()).unwrap();
        });
    }
    drop(tx);
    let mut allowed = 0;
    let mut denied = 0;
    while let Some(g) = rx.recv().await {
        match g {
            RenderGrant::Allowed => allowed += 1,
            RenderGrant::QuotaExhausted => denied += 1,
        }
    }
    assert_eq!(allowed, 2);
    assert_eq!(denied, 3);
}
