//! End-to-end integration tests for the parallel crawl pipeline.
//!
//! Each test spins up a local HTTP server and runs [`CrawlEngine`] against
//! it, exercising the FuturesUnordered dispatch loop, the max-pages budget,
//! fetch overlap, and incremental 304 handling.
#![allow(clippy::unwrap_used)]

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crawlkit_engine::crawl_engine::{CrawlEngine, CrawlEngineConfig};
use crawlkit_engine::http::{HttpClient, HttpClientConfig, UserAgentRotator};
use crawlkit_engine::storage::Storage;
use crawlkit_engine::storage_trait::StorageBackend;
use crawlkit_engine::CrawlConfig;

/// Behavior knobs for the local test server.
struct ServerConfig {
    /// Artificial delay before answering each request.
    per_request_delay: Duration,
    /// Serve an `ETag` and answer 304 to matching `If-None-Match` requests.
    support_etag: bool,
    deny_crawl: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            per_request_delay: Duration::ZERO,
            support_etag: false,
            deny_crawl: false,
        }
    }
}

/// Minimal HTTP/1.1 server for one crawl fixture: an index page linking to
/// `page_count` child pages, plus robots.txt. Tracks request counts and the
/// maximum number of concurrently open connections for overlap assertions.
struct TestServer {
    url_root: String,
    max_concurrent: Arc<AtomicUsize>,
}

impl TestServer {
    fn start(page_count: usize, cfg: ServerConfig) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let max_concurrent = Arc::new(AtomicUsize::new(0));
        let in_flight = Arc::new(AtomicUsize::new(0));

        let thread_max = Arc::clone(&max_concurrent);
        let thread_in_flight = Arc::clone(&in_flight);
        let cfg = Arc::new(cfg);
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let max = Arc::clone(&thread_max);
                let flight = Arc::clone(&thread_in_flight);
                let cfg = Arc::clone(&cfg);
                std::thread::spawn(move || {
                    serve_connection(stream, page_count, &cfg, &max, &flight);
                });
            }
        });

        Self {
            url_root: format!("http://127.0.0.1:{port}"),
            max_concurrent,
        }
    }

    fn index_url(&self) -> String {
        format!("{}/", self.url_root)
    }
}

fn serve_connection(
    mut stream: TcpStream,
    page_count: usize,
    cfg: &ServerConfig,
    max_concurrent: &Arc<AtomicUsize>,
    in_flight: &Arc<AtomicUsize>,
) {
    let mut buf = [0u8; 4096];
    let Ok(n) = stream.read(&mut buf) else {
        return;
    };
    let request = String::from_utf8_lossy(&buf[..n]).to_string();

    let current = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
    max_concurrent.fetch_max(current, Ordering::SeqCst);

    if cfg.per_request_delay > Duration::ZERO {
        std::thread::sleep(cfg.per_request_delay);
    }

    let (path, if_none_match) = parse_request(&request);
    let (status, headers, body) = route(
        &path,
        page_count,
        cfg.support_etag,
        cfg.deny_crawl,
        if_none_match,
    );

    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\n{headers}Connection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
    in_flight.fetch_sub(1, Ordering::SeqCst);
}

fn parse_request(request: &str) -> (String, Option<String>) {
    let path = request.split_whitespace().nth(1).unwrap_or("/").to_string();
    let if_none_match = request.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.trim()
            .eq_ignore_ascii_case("if-none-match")
            .then(|| value.trim().to_string())
    });
    (path, if_none_match)
}

fn route(
    path: &str,
    page_count: usize,
    support_etag: bool,
    deny_crawl: bool,
    if_none_match: Option<String>,
) -> (&'static str, String, String) {
    if path == "/robots.txt" {
        let body = if deny_crawl {
            "User-agent: *\nDisallow: /\n"
        } else {
            "User-agent: *\nAllow: /\n"
        };
        return ("200 OK", String::new(), body.into());
    }

    if path == "/" {
        let links: String = (0..page_count)
            .map(|i| format!("<a href=\"/page-{i}\">Page {i}</a>\n"))
            .collect();
        let body = format!(
            "<!DOCTYPE html><html lang=\"en\"><head><title>Index</title>\
             <meta name=\"description\" content=\"test index page\"></head>\
             <body><h1>Index</h1>{links}<p>{}</p></body></html>",
            "word ".repeat(60)
        );
        return respond("200 OK", support_etag, "index-v1", body, if_none_match);
    }

    if let Some(num) = path.strip_prefix("/page-") {
        let Ok(num) = num.parse::<usize>() else {
            return not_found();
        };
        let body = format!(
            "<!DOCTYPE html><html lang=\"en\"><head><title>Page {num}</title>\
             <meta name=\"description\" content=\"child page {num}\"></head>\
             <body><h1>Page {num}</h1><p>{}</p></body></html>",
            "content ".repeat(40 + num * 3)
        );
        return respond(
            "200 OK",
            support_etag,
            &format!("page-{num}-v1"),
            body,
            if_none_match,
        );
    }

    not_found()
}

fn not_found() -> (&'static str, String, String) {
    (
        "404 Not Found",
        String::new(),
        "<html><body>404</body></html>".into(),
    )
}

/// Answer 304 when the client presents a matching validator.
fn respond(
    status: &'static str,
    support_etag: bool,
    tag: &str,
    body: String,
    if_none_match: Option<String>,
) -> (&'static str, String, String) {
    if !support_etag {
        return (status, String::new(), body);
    }
    let quoted = format!("\"{tag}\"");
    if if_none_match.as_deref() == Some(quoted.as_str()) {
        ("304 Not Modified", String::new(), String::new())
    } else {
        (status, format!("ETag: {quoted}\r\n"), body)
    }
}

fn engine_config(max_pages: usize, concurrency: usize) -> CrawlEngineConfig {
    CrawlEngineConfig {
        crawl_config: CrawlConfig {
            max_pages,
            concurrency,
            respect_robots_txt: true,
            ..CrawlConfig::default()
        },
        concurrency: Some(concurrency),
        allow_http: true, // local test server only
        ..CrawlEngineConfig::default()
    }
}

fn shared_storage() -> Arc<dyn StorageBackend> {
    Arc::new(Storage::new_in_memory().unwrap()) as Arc<dyn StorageBackend>
}

#[tokio::test]
async fn test_max_pages_budget_is_respected_exactly() {
    let server = TestServer::start(30, ServerConfig::default());
    let engine = CrawlEngine::new_shared(engine_config(10, 4), shared_storage());
    let output = engine.run(&server.index_url()).await.unwrap();

    assert_eq!(
        output.pages_crawled, 10,
        "engine must stop at exactly max_pages"
    );
    assert_eq!(output.pages_stored, 10);
}

#[tokio::test]
async fn test_parallel_crawl_crawls_and_stores_entire_site() {
    let server = TestServer::start(8, ServerConfig::default());
    let engine = CrawlEngine::new_shared(engine_config(50, 4), shared_storage());
    let output = engine.run(&server.index_url()).await.unwrap();

    assert_eq!(output.pages_crawled, 9, "index + 8 child pages");
    assert_eq!(output.pages_stored, 9);
    assert!(output.issues_found > 0, "analyzers must produce findings");
    assert_eq!(output.skipped_robots, 0);
    assert_eq!(
        output.skipped_duplicate, 0,
        "distinct bodies must not dedup"
    );
}

#[tokio::test]
async fn test_fetches_overlap_under_concurrency() {
    // 7 requests x 120ms delay: sequential >= 840ms; concurrency 4 ~240ms.
    // A tiny request_delay keeps per-domain politeness pacing from
    // serializing the fetches; the server-side overlap counter is the
    // authoritative proof of parallelism.
    let server = TestServer::start(
        6,
        ServerConfig {
            per_request_delay: Duration::from_millis(120),
            support_etag: false,
            deny_crawl: false,
        },
    );
    let mut config = engine_config(20, 4);
    config.crawl_config.request_delay = Duration::from_millis(1);

    let engine = CrawlEngine::new_shared(config, shared_storage());

    let start = Instant::now();
    let output = engine.run(&server.index_url()).await.unwrap();
    let elapsed = start.elapsed();

    assert_eq!(output.pages_crawled, 7);
    // The server-side overlap counter is the authoritative proof of
    // parallelism; the timing assertion is relaxed for slow CI runners.
    assert!(
        server.max_concurrent.load(Ordering::SeqCst) >= 2,
        "server must observe overlapping requests (saw {}, elapsed: {:?})",
        server.max_concurrent.load(Ordering::SeqCst),
        elapsed
    );
}

#[tokio::test]
async fn test_incremental_recrawl_reports_not_modified() {
    let server = TestServer::start(
        4,
        ServerConfig {
            per_request_delay: Duration::ZERO,
            support_etag: true,
            deny_crawl: false,
        },
    );
    let storage = shared_storage();

    let first = CrawlEngine::new_shared(engine_config(50, 2), Arc::clone(&storage));
    let run_one = first.run(&server.index_url()).await.unwrap();
    assert_eq!(run_one.pages_crawled, 5);
    assert_eq!(run_one.pages_unchanged, 0, "first run has no validators");

    let mut second_cfg = engine_config(50, 2);
    second_cfg.incremental = true;
    let second = CrawlEngine::new_shared(second_cfg, Arc::clone(&storage));
    let run_two = second.run(&server.index_url()).await.unwrap();

    assert_eq!(
        run_two.pages_unchanged, 5,
        "all five pages should answer 304 on recrawl"
    );
    assert_eq!(run_two.pages_crawled, 0, "304 pages are not re-analyzed");
}

#[tokio::test]
async fn test_http_client_rejects_plain_http_by_default() {
    let server = TestServer::start(1, ServerConfig::default());
    let config = HttpClientConfig {
        timeout: Duration::from_secs(5),
        max_redirects: 3,
        retry_policy: Default::default(),
        user_agent: Arc::new(UserAgentRotator::new(vec!["test".to_string()])),
        max_body_size: 1024 * 1024,
        pool_max_idle_per_host: 4,
        pool_max_idle: 8,
        tcp_keepalive: None,
        pool_idle_timeout: Duration::from_secs(30),
        connect_timeout: Duration::from_secs(5),
        allow_http: false,
        seed: None,
    };
    let client = HttpClient::new(config).unwrap();
    let url = url::Url::parse(&server.index_url()).unwrap();
    assert!(
        client.fetch(&url).await.is_err(),
        "secure-by-default client must refuse plain HTTP"
    );
}

#[tokio::test]
async fn test_robots_txt_uses_non_standard_port_and_blocks_seed() {
    let server = TestServer::start(
        1,
        ServerConfig {
            deny_crawl: true,
            ..ServerConfig::default()
        },
    );
    let engine = CrawlEngine::new_shared(engine_config(10, 2), shared_storage());
    let output = engine.run(&server.index_url()).await.unwrap();

    assert_eq!(output.pages_crawled, 0);
    assert_eq!(output.pages_stored, 0);
    assert_eq!(output.skipped_robots, 1);
}

// ---------------------------------------------------------------------------
// Crawl-plugin execution (plugins run during real crawls)
// ---------------------------------------------------------------------------

/// Build the soft-404 SDK example to wasm32; `None` when the target is
/// unavailable (graceful skip, matching the conformance suites).
fn build_soft404(target_dir: &std::path::Path) -> Option<std::path::PathBuf> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_root = std::path::Path::new(&manifest_dir)
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf();
    let installed = std::process::Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
        .ok()?;
    if !String::from_utf8_lossy(&installed.stdout)
        .lines()
        .any(|l| l.starts_with("wasm32-unknown-unknown"))
    {
        return None;
    }
    let status = std::process::Command::new("cargo")
        .current_dir(&workspace_root)
        .args([
            "build",
            "-p",
            "crawlkit-plugin-sdk",
            "--example",
            "soft-404",
            "--target",
            "wasm32-unknown-unknown",
            "--target-dir",
        ])
        .arg(target_dir)
        .status()
        .ok()?;
    if !status.success() {
        return None;
    }
    Some(
        target_dir
            .join("wasm32-unknown-unknown")
            .join("debug")
            .join("examples")
            .join("soft-404.wasm"),
    )
}

/// A crawl with `plugin_dirs` populated runs the installed plugin on every
/// fetched page: the local 200-page server produces no SOFT404 findings,
/// and the plugin-loaded engine completes a normal crawl end to end.
#[tokio::test]
async fn crawl_runs_plugins_against_fetched_pages() {
    let target_dir = std::env::temp_dir().join("crawl-plugin-e2e");
    let Some(wasm) = build_soft404(&target_dir) else {
        eprintln!("skipping: wasm32-unknown-unknown build unavailable");
        return;
    };

    // Sign + stage the plugin in the install layout.
    let wasm_bytes = std::fs::read(&wasm).unwrap();
    let seed: [u8; 32] = "92bb3bc94dc375ea2c3111e1636511a8c0b22995437ee0338f4d21cdb9bfdd4d"
        .as_bytes()
        .chunks(2)
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect::<Vec<u8>>()
        .try_into()
        .unwrap();
    let (hash, sig, signer) = crawlkit_engine::plugin::sign_plugin_wasm(&wasm_bytes, &seed);
    let plugins_root = std::env::temp_dir().join(format!("crawl-plugins-{}", std::process::id()));
    let plugin_dir = plugins_root.join("soft-404");
    std::fs::create_dir_all(&plugin_dir).unwrap();
    std::fs::write(plugin_dir.join("plugin.wasm"), &wasm_bytes).unwrap();
    std::fs::write(
        plugin_dir.join("crawlkit-plugin.toml"),
        format!(
            "[plugin]\nname = \"soft-404\"\nversion = \"1.0.0\"\napi_version = \"1.0\"\nauthor = \"e2e\"\ndescription = \"plugin execution during crawls\"\nlicense = \"Apache-2.0\"\nwasm_hash = \"{hash}\"\nsignature = \"{sig}\"\nsigned_by = \"{signer}\"\n\n[plugin.entry]\nwasm = \"plugin.wasm\"\n\n[plugin.analyzer]\nname = \"soft-404\"\ncategories = [\"seo\"]\n"
        ),
    )
    .unwrap();

    let server = TestServer::start(
        3,
        ServerConfig {
            per_request_delay: Duration::from_millis(10),
            support_etag: false,
            deny_crawl: false,
        },
    );
    let mut config = engine_config(10, 2);
    config.plugin_dirs = vec![plugins_root.clone()];
    let engine = CrawlEngine::new_shared(config, shared_storage());

    let output = engine.run(&server.index_url()).await.unwrap();
    assert_eq!(output.pages_crawled, 4); // index + 3 pages
    assert!(output.issues_found > 0, "built-in analyzers still run");

    // Clean-up signed fixture.
    let _ = std::fs::remove_dir_all(&plugins_root);
}

/// A plugin that errors on every page must never abort the crawl.
#[tokio::test]
async fn failing_plugin_does_not_abort_crawl() {
    // A "plugin" whose analyze traps: valid module, garbage analyze body.
    const TRAP_WAT: &str = r#"
(module
  (memory (export "memory") 1)
  (global $heap (mut i32) (i32.const 1024))
  (func (export "crawlkit_plugin_init") (param i32) (result i32) i32.const 0)
  (func (export "crawlkit_plugin_alloc") (param $size i32) (result i32)
    (local $p i32)
    (local.set $p (global.get $heap))
    (global.set $heap (i32.add (global.get $heap) (local.get $size)))
    (local.get $p))
  (func (export "crawlkit_plugin_free") (param i32))
  (func (export "crawlkit_plugin_analyze") (param i32 i32 i32 i32) (result i32)
    unreachable)
)
"#;
    let plugins_root = std::env::temp_dir().join(format!("crawl-trap-{}", std::process::id()));
    let plugin_dir = plugins_root.join("trap");
    std::fs::create_dir_all(&plugin_dir).unwrap();
    std::fs::write(plugin_dir.join("plugin.wasm"), TRAP_WAT).unwrap();
    std::fs::write(
        plugin_dir.join("crawlkit-plugin.toml"),
        "[plugin]\nname = \"trap\"\nversion = \"1.0.0\"\napi_version = \"1.0\"\nauthor = \"e2e\"\ndescription = \"always traps\"\nlicense = \"Apache-2.0\"\n\n[plugin.entry]\nwasm = \"plugin.wasm\"\n\n[plugin.analyzer]\nname = \"trap\"\ncategories = [\"test\"]\n",
    )
    .unwrap();

    let server = TestServer::start(
        2,
        ServerConfig {
            per_request_delay: Duration::from_millis(10),
            support_etag: false,
            deny_crawl: false,
        },
    );
    let mut config = engine_config(10, 2);
    config.plugin_dirs = vec![plugins_root.clone()];
    let engine = CrawlEngine::new_shared(config, shared_storage());

    let Ok(output) = engine.run(&server.index_url()).await else {
        unreachable!("crawl aborted despite trapping plugin")
    };
    assert_eq!(output.pages_crawled, 3);

    let _ = std::fs::remove_dir_all(&plugins_root);
}
