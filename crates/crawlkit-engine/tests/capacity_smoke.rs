//! Capacity smoke test (docs/CAPACITY_EVIDENCE_PLAN.md §6) — the CI
//! regression gate for the 6.0.0 capacity-evidence work.
//!
//! Runs a deterministic 1k-page loopback crawl with the real
//! [`CrawlEngine`], samples process resources from `/proc` while it runs,
//! and writes a machine run record to `target/capacity/run_record.json`
//! (override the directory with `CAPACITY_RECORD_DIR`).
//!
//! Gates:
//! - **Absolute caps** (always enforced): peak RSS < 500 MB, throughput
//!   ≥ 20 pages/s on the smoke workload, open fds return to within
//!   baseline + 10 after the crawl, and the page budget is respected
//!   exactly (index + 1000 children).
//! - **Relative baseline** (only when `CAPACITY_ENFORCE=1` and the baseline
//!   record named by `CAPACITY_BASELINE_PATH` exists): throughput and peak
//!   RSS must stay within 20% of the baseline. CI sets the flag only once a
//!   committed CI baseline exists; until then records accumulate without
//!   gating so a noisy hosted runner cannot fail unrelated PRs.
//!
//! Run via: `cargo test -p crawlkit-engine --test capacity_smoke -- --ignored --nocapture`

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crawlkit_engine::crawl_engine::{CrawlEngine, CrawlEngineConfig};
use crawlkit_engine::storage::Storage;
use crawlkit_engine::storage_trait::StorageBackend;
use crawlkit_engine::CrawlConfig;

// ---------------------------------------------------------------------------
// Deterministic loopback corpus (plan §4): index + `page_count` children,
// body sizes varied by a fixed formula, issue-triggering shapes sprinkled at
// fixed ratios. Seeded by construction — no randomness anywhere.
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

    let (status, body) = route(&path, page_count);
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

fn route(path: &str, page_count: usize) -> (&'static str, String) {
    if path == "/robots.txt" {
        return ("200 OK", "User-agent: *\nAllow: /\n".into());
    }
    if path == "/" {
        let links: String = (0..page_count)
            .map(|i| format!("<a href=\"/page-{i}\">Page {i}</a>\n"))
            .collect();
        return (
            "200 OK",
            format!(
                "<!DOCTYPE html><html lang=\"en\"><head><title>Capacity Index</title>\
                 <meta name=\"description\" content=\"capacity smoke index\"></head>\
                 <body><h1>Index</h1>{links}<p>{}</p></body></html>",
                "word ".repeat(60)
            ),
        );
    }
    if let Some(num) = path
        .strip_prefix("/page-")
        .and_then(|s| s.parse::<usize>().ok())
    {
        if num < page_count {
            // Deterministic size variation (~0.2–8 KB) and fixed-ratio
            // issue shapes: every 3rd page lacks a description, every 7th
            // carries an image without alt text, every 11th stacks headings.
            let filler = "content ".repeat(30 + (num * 13) % 1000);
            let description = if num % 3 == 0 {
                String::new()
            } else {
                format!("<meta name=\"description\" content=\"child page {num}\">")
            };
            let img = if num % 7 == 0 {
                "<img src=\"/x.png\">"
            } else {
                ""
            };
            let headings = if num % 11 == 0 {
                "<h2>a</h2><h2>b</h2><h2>c</h2>"
            } else {
                ""
            };
            return (
                "200 OK",
                format!(
                    "<!DOCTYPE html><html lang=\"en\"><head><title>Page {num}</title>\
                     {description}</head><body><h1>Page {num}</h1>{headings}{img}\
                     <p>{filler}</p></body></html>"
                ),
            );
        }
    }
    ("404 Not Found", "<html><body>404</body></html>".into())
}

// ---------------------------------------------------------------------------
// /proc sampler (plan §7): RSS, fd count, task count. Linux-only by design —
// the smoke class runs on CI's Linux runners and the reference environment.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
struct Sample {
    ms_since_start: u128,
    rss_kb: u64,
    fds: u64,
    tasks: u64,
}

fn read_rss_kb() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let line = status.lines().find(|l| l.starts_with("VmRSS:"))?;
    line.split_whitespace().nth(1)?.parse().ok()
}

fn read_hwm_kb() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let line = status.lines().find(|l| l.starts_with("VmHWM:"))?;
    line.split_whitespace().nth(1)?.parse().ok()
}

fn count_entries(dir: &str) -> u64 {
    std::fs::read_dir(dir)
        .map(|d| d.filter_map(Result::ok).count() as u64)
        .unwrap_or(0)
}

fn spawn_sampler(
    stop: Arc<AtomicBool>,
    samples: Arc<Mutex<Vec<Sample>>>,
) -> std::thread::JoinHandle<()> {
    let start = Instant::now();
    std::thread::spawn(move || {
        while !stop.load(Ordering::Relaxed) {
            if let Some(rss_kb) = read_rss_kb() {
                let s = Sample {
                    ms_since_start: start.elapsed().as_millis(),
                    rss_kb,
                    fds: count_entries("/proc/self/fd"),
                    tasks: count_entries("/proc/self/task"),
                };
                samples.lock().unwrap().push(s);
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    })
}

// ---------------------------------------------------------------------------
// Run record (plan §5): the machine-written artifact every published number
// must trace to.
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Serialize)]
struct RunRecord {
    schema: &'static str,
    recorded_at_unix: u64,
    crawlkit_version: &'static str,
    workload: &'static str,
    kernel: String,
    cpu_model: String,
    config: ConfigRecord,
    results: ResultsRecord,
    resources: ResourcesRecord,
    gates: GatesRecord,
    enforce: EnforceRecord,
}

#[derive(Debug, serde::Serialize)]
struct ConfigRecord {
    pages: usize,
    concurrency: usize,
    max_pages: usize,
    request_delay_ms: u64,
    storage: &'static str,
    mode: &'static str,
}

#[derive(Debug, serde::Serialize)]
struct ResultsRecord {
    pages_crawled: usize,
    issues_found: usize,
    elapsed_secs: f64,
    throughput_pages_per_sec: f64,
}

#[derive(Debug, serde::Serialize)]
struct ResourcesRecord {
    rss_baseline_kb: u64,
    rss_peak_kb: u64,
    rss_hwm_kb: u64,
    rss_end_kb: u64,
    fd_baseline: u64,
    fd_peak: u64,
    fd_end: u64,
    tasks_peak: u64,
    samples: usize,
    settle_secs: u64,
}

#[derive(Debug, serde::Serialize)]
struct GatesRecord {
    pages_exact: bool,
    peak_rss_under_cap: bool,
    throughput_above_floor: bool,
    fds_returned_to_baseline: bool,
    baseline_within_20pct: Option<bool>,
    all_absolute_pass: bool,
}

#[derive(Debug, serde::Serialize)]
struct EnforceRecord {
    requested: bool,
    baseline_path: Option<String>,
    baseline_found: bool,
}

const PEAK_RSS_CAP_KB: u64 = 500_000; // 500 MB (plan §2)
const THROUGHPUT_FLOOR: f64 = 20.0; // pages/s, smoke-class floor (plan §6)
const FD_BASELINE_SLACK: u64 = 10; // plan §2
const BASELINE_TOLERANCE: f64 = 0.20; // plan §6
const PAGES: usize = 1000;
const CONCURRENCY: usize = 8;
const SETTLE_SECS: u64 = 2;

fn kernel() -> String {
    std::fs::read_to_string("/proc/sys/kernel/osrelease")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "unknown".into())
}

fn cpu_model() -> String {
    std::fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("model name"))
                .and_then(|l| l.split(':').nth(1))
                .map(|m| m.trim().to_string())
        })
        .unwrap_or_else(|| "unknown".into())
}

fn record_dir() -> PathBuf {
    std::env::var("CAPACITY_RECORD_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("target/capacity"))
}

// ---------------------------------------------------------------------------
// The smoke crawl + gates
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "capacity smoke; run via the CI capacity-smoke job or --ignored"]
async fn capacity_smoke_1k_crawl_meets_absolute_caps() {
    let server = TestServer::start(PAGES);
    let storage: Arc<dyn StorageBackend> = Arc::new(Storage::new_in_memory().unwrap());
    let engine = CrawlEngine::new_shared(
        CrawlEngineConfig {
            crawl_config: CrawlConfig {
                max_pages: PAGES + 100,
                concurrency: CONCURRENCY,
                respect_robots_txt: true,
                // Loopback corpus we own — the 500 ms default politeness
                // delay would serialize the workload and measure the delay,
                // not the engine (plan §4: config is pinned in the record).
                request_delay: Duration::ZERO,
                ..CrawlConfig::default()
            },
            concurrency: Some(CONCURRENCY),
            allow_http: true, // local loopback corpus only
            ..CrawlEngineConfig::default()
        },
        Arc::clone(&storage),
    );

    // Baseline resource readings taken after setup, before the crawl.
    let rss_baseline_kb = read_rss_kb().unwrap_or(0);
    let fd_baseline = count_entries("/proc/self/fd");

    let stop = Arc::new(AtomicBool::new(false));
    let samples: Arc<Mutex<Vec<Sample>>> = Arc::new(Mutex::new(Vec::new()));
    let sampler = spawn_sampler(Arc::clone(&stop), Arc::clone(&samples));

    let started = Instant::now();
    let output = engine.run(&server.index_url()).await.unwrap();
    let elapsed = started.elapsed();

    stop.store(true, Ordering::Relaxed);
    sampler.join().unwrap();
    std::thread::sleep(Duration::from_secs(SETTLE_SECS));

    let rss_end_kb = read_rss_kb().unwrap_or(0);
    let rss_hwm_kb = read_hwm_kb().unwrap_or(0);
    let fd_end = count_entries("/proc/self/fd");

    let (rss_peak_kb, fd_peak, tasks_peak) = {
        let s = samples.lock().unwrap();
        (
            s.iter()
                .map(|x| x.rss_kb)
                .max()
                .unwrap_or(0)
                .max(rss_hwm_kb),
            s.iter().map(|x| x.fds).max().unwrap_or(0),
            s.iter().map(|x| x.tasks).max().unwrap_or(0),
        )
    };

    let elapsed_secs = elapsed.as_secs_f64();
    let throughput = output.pages_crawled as f64 / elapsed_secs;

    // --- absolute caps (plan §6: always enforced) ---
    let pages_exact = output.pages_crawled == PAGES + 1; // index + children
    let peak_rss_under_cap = rss_peak_kb < PEAK_RSS_CAP_KB;
    let throughput_above_floor = throughput >= THROUGHPUT_FLOOR;
    let fds_returned_to_baseline = fd_end <= fd_baseline + FD_BASELINE_SLACK;
    let all_absolute_pass =
        pages_exact && peak_rss_under_cap && throughput_above_floor && fds_returned_to_baseline;

    // --- relative baseline (only when enforce is requested and available) ---
    let enforce_requested = std::env::var("CAPACITY_ENFORCE").is_ok_and(|v| v == "1");
    let baseline_path = std::env::var("CAPACITY_BASELINE_PATH").ok();
    let baseline_found = baseline_path
        .as_deref()
        .map(std::path::Path::new)
        .map(std::fs::read_to_string)
        .transpose()
        .ok()
        .flatten()
        .and_then(|raw| serde_json::from_str::<BaselineRecord>(&raw).ok());
    let baseline_within_20pct = baseline_found.as_ref().map(|b| {
        let t_ok = throughput >= b.results.throughput_pages_per_sec * (1.0 - BASELINE_TOLERANCE);
        let r_ok =
            rss_peak_kb as f64 <= b.resources.rss_peak_kb as f64 * (1.0 + BASELINE_TOLERANCE);
        t_ok && r_ok
    });

    let record = RunRecord {
        schema: "crawlkit.capacity.run_record/v1",
        recorded_at_unix: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        crawlkit_version: env!("CARGO_PKG_VERSION"),
        workload: "smoke-1k",
        kernel: kernel(),
        cpu_model: cpu_model(),
        config: ConfigRecord {
            pages: PAGES,
            concurrency: CONCURRENCY,
            max_pages: PAGES + 100,
            request_delay_ms: 0,
            storage: "sqlite-in-memory",
            mode: "inline",
        },
        results: ResultsRecord {
            pages_crawled: output.pages_crawled,
            issues_found: output.issues_found,
            elapsed_secs,
            throughput_pages_per_sec: throughput,
        },
        resources: ResourcesRecord {
            rss_baseline_kb,
            rss_peak_kb,
            rss_hwm_kb,
            rss_end_kb,
            fd_baseline,
            fd_peak,
            fd_end,
            tasks_peak,
            samples: samples.lock().unwrap().len(),
            settle_secs: SETTLE_SECS,
        },
        gates: GatesRecord {
            pages_exact,
            peak_rss_under_cap,
            throughput_above_floor,
            fds_returned_to_baseline,
            baseline_within_20pct,
            all_absolute_pass,
        },
        enforce: EnforceRecord {
            requested: enforce_requested,
            baseline_path,
            baseline_found: baseline_found.is_some(),
        },
    };

    // Emit the record before asserting so a failure still leaves the artifact.
    let dir = record_dir();
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("run_record.json");
    std::fs::write(&path, serde_json::to_string_pretty(&record).unwrap()).unwrap();
    println!("capacity run record: {}", path.display());

    assert!(
        pages_exact,
        "page budget not respected exactly: crawled {}",
        output.pages_crawled
    );
    assert!(
        peak_rss_under_cap,
        "peak RSS {} KB exceeds cap {PEAK_RSS_CAP_KB} KB",
        rss_peak_kb
    );
    assert!(
        throughput_above_floor,
        "throughput {throughput:.1} pages/s below smoke floor {THROUGHPUT_FLOOR}"
    );
    assert!(
        fds_returned_to_baseline,
        "fds did not return to baseline: {fd_baseline} → {fd_end}"
    );
    if enforce_requested {
        if let Some(true) = record.gates.baseline_within_20pct {
            // pass
        } else {
            panic!(
                "baseline enforcement requested but the run is not within \
                 {BASELINE_TOLERANCE} of the baseline (or the baseline was missing)"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Baseline record: only the fields the relative gate compares.
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Deserialize)]
struct BaselineRecord {
    results: BaselineResults,
    resources: BaselineResources,
}

#[derive(Debug, serde::Deserialize)]
struct BaselineResults {
    throughput_pages_per_sec: f64,
}

#[derive(Debug, serde::Deserialize)]
struct BaselineResources {
    rss_peak_kb: u64,
}
