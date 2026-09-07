//! End-to-end crawl throughput benchmark.
//!
//! Spins up a local HTTP/1.1 **keep-alive** server (thread per connection,
//! any number of connections in flight) and measures pages/sec at varying
//! site sizes with the engine's fetch concurrency pinned to 4. Peak RSS is
//! read from `/proc/self/status` (`VmHWM`).
//!
//! The server handles connections concurrently and reuses them across
//! requests, so measured throughput is bounded by the engine pipeline
//! (fetch → parse → analyze → store), not by connection setup or a serial
//! accept loop. Server-side counters (`server-reqs`, `max-concurrent`) are
//! printed alongside each result as evidence of real fetch overlap: if
//! `max-concurrent` never exceeds 1, the engine did not overlap fetches and
//! the number is NOT a concurrent-throughput measurement.
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
// Minimal concurrent HTTP server (keep-alive, thread per connection)
// ---------------------------------------------------------------------------

#[derive(Default)]
struct ServerConfig {
    support_etag: bool,
    deny_crawl: bool,
}

struct TestServer {
    url_root: String,
    requests_served: Arc<AtomicUsize>,
    max_concurrent: Arc<AtomicUsize>,
}

impl TestServer {
    /// Binds an ephemeral port and serves `page_count` fixture pages.
    ///
    /// The accept loop runs on its own thread; every accepted connection is
    /// handed to a fresh thread that serves requests in a keep-alive loop.
    /// Connections therefore never serialize behind each other and the
    /// server imposes no per-request connection-setup cost.
    fn start(page_count: usize, cfg: ServerConfig) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let requests_served = Arc::new(AtomicUsize::new(0));
        let max_concurrent = Arc::new(AtomicUsize::new(0));
        let in_flight = Arc::new(AtomicUsize::new(0));

        let t_requests = Arc::clone(&requests_served);
        let t_max = Arc::clone(&max_concurrent);
        let t_flight = Arc::clone(&in_flight);
        let cfg = Arc::new(cfg);
        std::thread::Builder::new()
            .name("bench-accept".into())
            .spawn(move || {
                for stream in listener.incoming().flatten() {
                    let requests = Arc::clone(&t_requests);
                    let max = Arc::clone(&t_max);
                    let flight = Arc::clone(&t_flight);
                    let cfg = Arc::clone(&cfg);
                    let _ =
                        std::thread::Builder::new()
                            .name("bench-conn".into())
                            .spawn(move || {
                                serve_connection(
                                    stream, page_count, &cfg, &requests, &max, &flight,
                                );
                            });
                }
            })
            .unwrap();

        Self {
            url_root: format!("http://127.0.0.1:{port}"),
            requests_served,
            max_concurrent,
        }
    }

    fn index_url(&self) -> String {
        format!("{}/", self.url_root)
    }
}

/// Serves one connection until the client closes it (HTTP/1.1 keep-alive).
fn serve_connection(
    mut stream: TcpStream,
    page_count: usize,
    cfg: &ServerConfig,
    requests_served: &AtomicUsize,
    max_concurrent: &AtomicUsize,
    in_flight: &AtomicUsize,
) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));
    let _ = stream.set_nodelay(true);
    let mut buf: Vec<u8> = Vec::with_capacity(8192);
    let mut chunk = [0u8; 4096];

    loop {
        // Read until the full request header block (…\r\n\r\n) has arrived.
        buf.clear();
        loop {
            match stream.read(&mut chunk) {
                Ok(0) | Err(_) => return, // client closed / errored
                Ok(n) => {
                    buf.extend_from_slice(&chunk[..n]);
                    if buf.windows(4).any(|w| w == b"\r\n\r\n") {
                        break;
                    }
                }
            }
        }

        let current = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
        max_concurrent.fetch_max(current, Ordering::SeqCst);

        let request = String::from_utf8_lossy(&buf);
        let (path, if_none_match) = parse_request(&request);
        let (status, headers, body) = route(
            &path,
            page_count,
            cfg.support_etag,
            cfg.deny_crawl,
            if_none_match,
        );

        // No `Connection: close` — the client's pool can reuse this socket.
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\n{headers}\r\n{body}",
            body.len()
        );
        let wrote_ok = stream.write_all(response.as_bytes()).is_ok();
        in_flight.fetch_sub(1, Ordering::SeqCst);
        requests_served.fetch_add(1, Ordering::SeqCst);
        if !wrote_ok {
            return;
        }
    }
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
// Peak RSS measurement (Linux: VmHWM from /proc/self/status)
// ---------------------------------------------------------------------------

#[cfg(target_os = "linux")]
fn get_peak_rss_bytes() -> u64 {
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmHWM:") {
            let kb: u64 = rest
                .trim()
                .trim_end_matches("kB")
                .trim()
                .parse()
                .unwrap_or(0);
            return kb * 1024;
        }
    }
    0
}

#[cfg(not(target_os = "linux"))]
fn get_peak_rss_bytes() -> u64 {
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

/// Runs one crawl against a fresh fixture server and prints the result row.
fn bench_once(rt: &tokio::runtime::Runtime, n: usize, concurrency: usize) {
    let server = TestServer::start(n, ServerConfig::default());
    let engine = CrawlEngine::new_shared(engine_config(n + 1, concurrency), shared_storage());

    let start = Instant::now();
    let output = rt.block_on(engine.run(&server.index_url()));
    let elapsed = start.elapsed();

    match output {
        Ok(o) => {
            let pps = o.pages_crawled as f64 / elapsed.as_secs_f64();
            let served = server.requests_served.load(Ordering::SeqCst);
            let overlap = server.max_concurrent.load(Ordering::SeqCst);
            println!(
                "n={n:<4} conc={concurrency}  {:>4} pages in {:>6.2}s  = {:>7.1} pages/sec   \
                 [server: {served} reqs, max {overlap} concurrent]",
                o.pages_crawled,
                elapsed.as_secs_f64(),
                pps
            );
            if o.pages_crawled != n + 1 {
                eprintln!(
                    "  WARNING: expected {} crawled pages, got {} (skipped/dup/failed) — number may be inflated",
                    n + 1,
                    o.pages_crawled
                );
            }
            if overlap < 2 {
                eprintln!(
                    "  WARNING: server never observed more than {overlap} concurrent request — fetches did not overlap"
                );
            }
        }
        Err(e) => eprintln!("n={n}: FAILED: {e}"),
    }
}

fn main() {
    println!("crawlkit throughput benchmark (engine-limited)");
    println!("==============================================");
    println!(
        "server: local HTTP/1.1 keep-alive, thread per connection (concurrent, no per-request connection setup)"
    );

    // Warm-up: one unmeasured crawl to amortize allocator/analyzer-registry
    // and OS cold-start costs. Results below are steady-state numbers.
    {
        let server = TestServer::start(25, ServerConfig::default());
        let rt = tokio::runtime::Runtime::new().unwrap();
        let engine = CrawlEngine::new_shared(engine_config(26, 4), shared_storage());
        let _ = rt.block_on(engine.run(&server.index_url()));
    }

    let rss_before = get_peak_rss_bytes();
    let rt = tokio::runtime::Runtime::new().unwrap();

    println!("\n--- Pages/sec at concurrency=4 ---");
    for &n in &[50usize, 100, 500] {
        bench_once(&rt, n, 4);
    }

    println!("\n--- Peak RSS ---");
    let rss_after = get_peak_rss_bytes();
    println!(
        "Peak RSS: {:.1} MB (delta since start: {:.1} MB)",
        rss_after as f64 / 1_048_576.0,
        (rss_after.saturating_sub(rss_before)) as f64 / 1_048_576.0
    );
}
