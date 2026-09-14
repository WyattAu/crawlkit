//! Heap attribution for the 10k-class RSS gap (docs/CAPACITY_EVIDENCE_PLAN.md
//! §8, 2026-09-13 finding: 2115 MB peak RSS vs the < 500 MB target).
//!
//! Attribution method — three instruments:
//!
//! 1. **Phase snapshots.** `dhat::HeapStats` sampled around each phase
//!    (setup / crawl / storage readback) under a single dhat profiler.
//!    Deltas of the running `total_bytes` counter are order-independent.
//! 2. **Ablation.** The same crawl run twice in one process —
//!    `AnalyzerProfile::Core` first (clean globals: its `max_bytes` is its
//!    own peak), then `Full` (the test asserts its peak exceeds the core
//!    peak, so the global peak is attributable to the full run).
//! 3. **Timeline sampling.** A background thread polls `curr_bytes` every
//!    10 ms so the *moment* of the allocation peak can be attributed to the
//!    crawl loop vs the post-loop report/drain phase.
//!
//! Honest scope: dhat instruments the Rust allocator only (not mmapped
//! SQLite pages or kernel page cache), and its counters include allocator
//! slack. File-backed storage is used (matching the committed 10k reference
//! class) — note the engine's own 512 MB default resource monitor aborts
//! mid-crawl on the in-memory-storage variant, which is itself evidence:
//! the full profile crossed 512 MB of Rust-heap + in-memory-DB by page
//! ~4500 while the core profile stayed under it.
//!
//! Run via: `cargo test -p crawlkit-engine --features profiling \
//!           --test heap_attribution -- --ignored --nocapture`

#![cfg(feature = "profiling")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

/// dhat instruments allocations only through its own global allocator —
/// installing the `Profiler` alone records nothing (the same wiring the
/// `crawl --profiling` binary needs; see the TODO on that dormant path).
#[cfg(feature = "profiling")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crawlkit_engine::analyzers::AnalyzerProfile;
use crawlkit_engine::crawl_engine::{CrawlEngine, CrawlEngineConfig};
use crawlkit_engine::storage::Storage;
use crawlkit_engine::storage_trait::StorageBackend;
use crawlkit_engine::CrawlConfig;

const PAGES: usize = 10_000;
const CONCURRENCY: usize = 8;
const READBACK_LIMIT: usize = 2000;

/// Deterministic loopback corpus: identical shape to the capacity smoke's
/// (plan §4 — index + children, ~0.2–8 KB bodies, issue shapes at fixed
/// ratios) so allocation numbers here are directly comparable to the
/// committed 10k RSS records.
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
                 <meta name=\"description\" content=\"capacity index\"></head>\
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

fn serve_connection(mut stream: std::net::TcpStream, page_count: usize) {
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

/// One dhat counter reading.
#[derive(Debug, Clone, Copy)]
struct Phase {
    total_bytes: u64,
    max_bytes: usize,
    curr_bytes: usize,
}

impl Phase {
    fn take() -> Self {
        let s = dhat::HeapStats::get();
        Self {
            total_bytes: s.total_bytes,
            max_bytes: s.max_bytes,
            curr_bytes: s.curr_bytes,
        }
    }
}

fn mb(bytes: u64) -> f64 {
    bytes as f64 / 1024.0 / 1024.0
}

#[derive(Debug)]
struct RunStats {
    setup_total: u64,
    crawl_total: u64,
    readback_total: u64,
    /// Pages actually crawled — the budget check reads this from the record,
    /// not from an early assert, so a tripped cap still leaves evidence.
    pages_crawled: usize,
    /// Allocation peak (`max_bytes`) at readback time — the process-global
    /// peak (see module doc for per-run attribution).
    peak_bytes: usize,
    /// Bytes still allocated after readback (retained by storage/engine).
    retained_bytes: usize,
    /// Peak of `curr_bytes` observed *during* the crawl loop (timeline).
    crawl_curr_peak: usize,
    /// Peak of `curr_bytes` observed after the crawl loop returned.
    post_crawl_curr_peak: usize,
    crawl_secs: f64,
}

fn run_crawl(profile: AnalyzerProfile) -> RunStats {
    // The engine's 512 MB default monitor would abort the full-profile run
    // mid-crawl (observed: stopped at 4507/10001 pages when Rust-heap +
    // analyzer intermediates crossed the cap). The measurement must be
    // allowed to reach its natural peak — set once, process-wide, before
    // any engine construction (semver-safe additive API; the public config
    // struct cannot gain a field without a major version).
    let _ = crawlkit_engine::set_default_limits(crawlkit_engine::ResourceLimits {
        max_memory_bytes: None,
        ..crawlkit_engine::ResourceLimits::default()
    });
    let server = TestServer::start(PAGES);
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("capacity.db");
    let storage: Arc<dyn StorageBackend> = Arc::new(Storage::new(&db_path).unwrap());
    let engine = CrawlEngine::new_shared(
        CrawlEngineConfig {
            crawl_config: CrawlConfig {
                max_pages: PAGES + 100,
                concurrency: CONCURRENCY,
                respect_robots_txt: true,
                // Loopback corpus we own — the 500 ms default politeness
                // delay would serialize the workload (same pinning as the
                // capacity smoke, plan §4).
                request_delay: Duration::ZERO,
                ..CrawlConfig::default()
            },
            analyzer_profile: profile,
            concurrency: Some(CONCURRENCY),
            allow_http: true, // local loopback corpus only
            ..CrawlEngineConfig::default()
        },
        Arc::clone(&storage),
    );

    let after_setup = Phase::take();

    // Timeline sampler: curr_bytes every 10 ms across the whole run.
    let stop = Arc::new(AtomicBool::new(false));
    let samples: Arc<Mutex<Vec<(u128, usize)>>> = Arc::new(Mutex::new(Vec::new()));
    let sampler = {
        let stop = Arc::clone(&stop);
        let samples = Arc::clone(&samples);
        let t0 = Instant::now();
        std::thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                let ms = t0.elapsed().as_millis();
                let curr = dhat::HeapStats::get().curr_bytes;
                samples.lock().unwrap().push((ms, curr));
                std::thread::sleep(Duration::from_millis(10));
            }
        })
    };

    let started = Instant::now();
    let output = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap()
        .block_on(async { engine.run(&server.index_url()).await.unwrap() });
    let crawl_secs = started.elapsed().as_secs_f64();
    let after_crawl = Phase::take();

    // No early assert: a run that misses the page budget (e.g. tripped by a
    // resource cap) must still produce its record — the record IS the
    // evidence. The budget check happens after the record is written.

    // Storage readback through the real query path, using the actual
    // crawl id the engine generated (the last row inserted).
    let crawls = storage.list_crawls().unwrap();
    let crawl_id = crawls.last().map(|(id, _)| id).expect("crawl row exists");
    let pages = storage.get_pages(&crawl_id, READBACK_LIMIT).unwrap();
    let issues = storage
        .get_issues(&crawl_id, &crawlkit_engine::storage::IssueFilter::default())
        .unwrap();
    let after_readback = Phase::take();

    stop.store(true, Ordering::Relaxed);
    // The sampler can observe stats during unwind of the main thread; its
    // panic (if any) must not mask the attribution result.
    let _ = sampler.join();

    let (crawl_curr_peak, post_crawl_curr_peak) = {
        let s = samples.lock().unwrap();
        // Samples are (ms, curr); the crawl ended at crawl_secs — split there.
        let boundary_ms = (crawl_secs * 1000.0) as u128;
        let during = s
            .iter()
            .filter(|(ms, _)| ms <= &boundary_ms)
            .map(|(_, c)| *c);
        let after = s
            .iter()
            .filter(|(ms, _)| ms > &boundary_ms)
            .map(|(_, c)| *c);
        (
            during.max().unwrap_or(0),
            after.max().unwrap_or(after_crawl.curr_bytes),
        )
    };
    println!(
        "[{profile:?}] readback: {} pages, {} issues",
        pages.len(),
        issues.len()
    );

    RunStats {
        setup_total: after_setup.total_bytes,
        crawl_total: after_crawl.total_bytes,
        readback_total: after_readback.total_bytes,
        pages_crawled: output.pages_crawled,
        peak_bytes: after_readback.max_bytes,
        retained_bytes: after_readback.curr_bytes,
        crawl_curr_peak,
        post_crawl_curr_peak,
        crawl_secs,
    }
}

#[test]
#[ignore = "heap attribution; run under --features profiling with --ignored"]
fn heap_attribution_10k_phase_and_ablation() {
    let _profiler = dhat::Profiler::builder().testing().build();

    // -- run 1: core profile, clean globals --------------------------------
    let core = run_crawl(AnalyzerProfile::Core);
    println!("\n=== core profile (run 1) ===");
    println!(
        "crawl alloc Δ:   {:>8.0} MB  ({:.1} s)",
        mb(core.crawl_total - core.setup_total),
        core.crawl_secs
    );
    println!(
        "readback alloc Δ:{:>8.0} MB",
        mb(core.readback_total - core.crawl_total)
    );
    println!("allocation peak: {:>8.0} MB", mb(core.peak_bytes as u64));
    println!(
        "  during crawl:  {:>8.0} MB",
        mb(core.crawl_curr_peak as u64)
    );
    println!(
        "  after crawl:   {:>8.0} MB",
        mb(core.post_crawl_curr_peak as u64)
    );
    println!(
        "retained:        {:>8.0} MB",
        mb(core.retained_bytes as u64)
    );

    // -- run 2: full profile, same workload ---------------------------------
    let full = run_crawl(AnalyzerProfile::Full);
    println!("\n=== full profile (run 2) ===");
    println!(
        "crawl alloc Δ:   {:>8.0} MB  ({:.1} s)",
        mb(full.crawl_total - full.setup_total),
        full.crawl_secs
    );
    println!(
        "readback alloc Δ:{:>8.0} MB",
        mb(full.readback_total - full.crawl_total)
    );
    println!("allocation peak: {:>8.0} MB", mb(full.peak_bytes as u64));
    println!(
        "  during crawl:  {:>8.0} MB",
        mb(full.crawl_curr_peak as u64)
    );
    println!(
        "  after crawl:   {:>8.0} MB",
        mb(full.post_crawl_curr_peak as u64)
    );
    println!(
        "retained:        {:>8.0} MB",
        mb(full.retained_bytes as u64)
    );

    // -- attribution ---------------------------------------------------------
    let core_crawl = core.crawl_total - core.setup_total;
    let full_crawl = full.crawl_total - full.setup_total;
    let core_readback = core.readback_total - core.crawl_total;
    let full_readback = full.readback_total - full.crawl_total;
    let analyzer_share = (full_crawl.saturating_sub(core_crawl)) as f64 / full_crawl as f64 * 100.0;

    println!("\n=== attribution ===");
    println!(
        "crawl allocation:   full={:>7.0} MB  core={:>7.0} MB",
        mb(full_crawl),
        mb(core_crawl)
    );
    println!(
        "readback alloc:     full={:>7.0} MB  core={:>7.0} MB",
        mb(full_readback),
        mb(core_readback)
    );
    println!(
        "allocation peak:    full={:>7.0} MB  core={:>7.0} MB",
        mb(full.peak_bytes as u64),
        mb(core.peak_bytes as u64)
    );
    println!(
        "  during-crawl peak: full={:>7.0} MB  core={:>7.0} MB",
        mb(full.crawl_curr_peak as u64),
        mb(core.crawl_curr_peak as u64)
    );
    println!(
        "  post-crawl peak:   full={:>7.0} MB  core={:>7.0} MB",
        mb(full.post_crawl_curr_peak as u64),
        mb(core.post_crawl_curr_peak as u64)
    );
    println!(
        "retained:           full={:>7.0} MB  core={:>7.0} MB",
        mb(full.retained_bytes as u64),
        mb(core.retained_bytes as u64)
    );
    println!("analyzer share of crawl allocation: {analyzer_share:.1}%");

    let record = serde_json::json!({
        "schema": "crawlkit.capacity.heap_attribution/v1",
        "workload": format!("crawl-{PAGES}"),
        "concurrency": CONCURRENCY,
        "profile": "release",
        "storage": "sqlite-file",
        "note": "dhat instruments the Rust allocator only. Peak attribution: core ran first (clean globals); full peak asserted > core peak. curr_bytes timeline splits during-crawl vs post-crawl.",
        "core": {
            "pages_crawled": core.pages_crawled,
            "crawl_alloc_mb": (core_crawl / 1024 / 1024) as u64,
            "readback_alloc_mb": (core_readback / 1024 / 1024) as u64,
            "peak_mb": (core.peak_bytes / 1024 / 1024) as u64,
            "crawl_curr_peak_mb": (core.crawl_curr_peak / 1024 / 1024) as u64,
            "post_crawl_curr_peak_mb": (core.post_crawl_curr_peak / 1024 / 1024) as u64,
            "retained_mb": (core.retained_bytes / 1024 / 1024) as u64,
            "crawl_secs": core.crawl_secs,
        },
        "full": {
            "pages_crawled": full.pages_crawled,
            "crawl_alloc_mb": (full_crawl / 1024 / 1024) as u64,
            "readback_alloc_mb": (full_readback / 1024 / 1024) as u64,
            "peak_mb": (full.peak_bytes / 1024 / 1024) as u64,
            "crawl_curr_peak_mb": (full.crawl_curr_peak / 1024 / 1024) as u64,
            "post_crawl_curr_peak_mb": (full.post_crawl_curr_peak / 1024 / 1024) as u64,
            "retained_mb": (full.retained_bytes / 1024 / 1024) as u64,
            "crawl_secs": full.crawl_secs,
        },
        "analyzer_share_pct": analyzer_share,
    });
    let dir = std::path::PathBuf::from(
        std::env::var("CAPACITY_RECORD_DIR").unwrap_or_else(|_| "target/capacity".into()),
    );
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("heap_attribution.json");
    std::fs::write(&path, serde_json::to_string_pretty(&record).unwrap()).unwrap();
    println!("\nattribution record: {}", path.display());

    // Sanity assertions (evidence integrity, not gates) — after the record
    // is on disk:
    assert_eq!(
        core.pages_crawled,
        PAGES + 1,
        "core run missed the page budget — workload broken"
    );
    assert_eq!(
        full.pages_crawled,
        PAGES + 1,
        "full run missed the page budget (check the engine resource monitor)"
    );
    assert!(full_crawl > 0, "no crawl allocation recorded");
    assert!(
        full_crawl >= core_crawl,
        "full profile allocated less than core on the same workload — attribution method broken"
    );
    assert!(
        full.peak_bytes > core.peak_bytes,
        "full-profile peak did not exceed the core peak — the global max_bytes is not attributable to the full run"
    );
}
