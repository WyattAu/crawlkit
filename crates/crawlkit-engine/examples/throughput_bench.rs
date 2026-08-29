//! End-to-end crawl throughput benchmark.
//!
//! Spins up a local TestServer at varying page counts and measures
//! pages/sec + peak RSS. Criterion is NOT used — this is a simple
//! wall-clock measurement to produce a single reproducible pages/sec
//! number for the README.
//!
//! ```sh
//! cargo run --release -p crawlkit-engine --example throughput_bench
//! ```

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crawlkit_engine::crawl_engine::{CrawlEngine, CrawlEngineConfig};
use crawlkit_engine::storage::Storage;
use crawlkit_engine::storage_trait::StorageBackend;
use crawlkit_engine::CrawlConfig;

// ---------------------------------------------------------------------------
// Minimal HTTP server (adapted from parallel_pipeline_tests)
// ---------------------------------------------------------------------------

struct ServerConfig {
    per_request_delay: Duration,
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

struct TestServer {
    url_root: String,
    _max_concurrent: Arc<AtomicUsize>,
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
            _max_concurrent: max_concurrent,
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

// ---------------------------------------------------------------------------
// RSS measurement (Linux only)
// ---------------------------------------------------------------------------

#[cfg(target_os = "linux")]
fn get_process_rss_bytes() -> u64 {
    std::fs::read_to_string("/proc/self/statm")
        .ok()
        .and_then(|s| {
            let fields: Vec<&str> = s.split_whitespace().collect();
            if fields.len() >= 2 {
                let pages: u64 = fields[1].parse().ok()?;
                Some(pages * 4096)
            } else {
                None
            }
        })
        .unwrap_or(0)
}

#[cfg(not(target_os = "linux"))]
fn get_process_rss_bytes() -> u64 {
    0
}

// ---------------------------------------------------------------------------
// Benchmark harness
// ---------------------------------------------------------------------------

fn engine_config(max_pages: usize, concurrency: usize) -> CrawlEngineConfig {
    CrawlEngineConfig {
        crawl_config: CrawlConfig {
            max_pages,
            concurrency,
            respect_robots_txt: true,
            request_delay: Duration::ZERO,
            ..CrawlConfig::default()
        },
        concurrency: Some(concurrency),
        allow_http: true,
        ..CrawlEngineConfig::default()
    }
}

fn shared_storage() -> Arc<dyn StorageBackend> {
    Arc::new(Storage::new_in_memory().unwrap())
}

fn main() {
    println!("crawlkit throughput benchmark");
    println!("============================");

    // Warm up — build the engine once to amortize one-time costs
    let _ = CrawlEngine::new(
        CrawlEngineConfig::default(),
        Storage::new_in_memory().unwrap(),
    );

    let page_counts = [10, 25, 50];
    for &n in &page_counts {
        let server = TestServer::start(n, ServerConfig::default());
        let storage = shared_storage();
        let config = engine_config(n + 1, 8); // +1 for the index page
        let engine = CrawlEngine::new_shared(config, storage);

        let start = Instant::now();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let output = rt.block_on(engine.run(&server.index_url()));
        let elapsed = start.elapsed();

        match output {
            Ok(o) => {
                let pps = o.pages_crawled as f64 / elapsed.as_secs_f64();
                println!(
                    "n={n}: {} pages in {:.2}s = {:.1} pages/sec",
                    o.pages_crawled,
                    elapsed.as_secs_f64(),
                    pps
                );
            }
            Err(e) => eprintln!("n={n}: FAILED: {e}"),
        }
    }

    // Peak RSS at 50 pages
    println!("\n--- Peak RSS (50 pages) ---");
    let server = TestServer::start(50, ServerConfig::default());
    let storage = shared_storage();
    let config = engine_config(51, 8);
    let engine = CrawlEngine::new_shared(config, storage);
    let rss_before = get_process_rss_bytes();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let _ = rt.block_on(engine.run(&server.index_url()));
    let rss_after = get_process_rss_bytes();
    println!(
        "Peak RSS: {:.1} MB (delta: {:.1} MB)",
        rss_after as f64 / 1_048_576.0,
        (rss_after.saturating_sub(rss_before)) as f64 / 1_048_576.0
    );
}
