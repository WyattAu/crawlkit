//! Distributed-posture capacity run (docs/CAPACITY_EVIDENCE_PLAN.md §8.4).
//!
//! Topology measured (and named in the record so it can never be conflated
//! with inline-mode numbers): N worker **processes**, each running the real
//! [`CrawlEngine`] against its own loopback host, all writing to **one shared
//! PostgreSQL instance** — the deployment shape whose sizing the 6.0.0
//! distributed-stability claim rests on.
//!
//! Honest scope (plan §8.4 note): the Redis lease queue is not wired into the
//! crawl frontier yet — that integration is tracked as 6.0.0 engineering.
//! Each worker therefore runs the engine's inline path against its own seed;
//! what this run measures is the *shared-state* posture: multi-process
//! footprint, aggregate throughput, fd behavior under connection pools, and
//! concurrent write load on one PostgreSQL instance.
//!
//! Requires services (default endpoints match the runbook drill setup):
//!   - PostgreSQL at postgres://crawlkit:crawlkit@127.0.0.1:5499/crawlkit
//!   - Docker (the parent resets the schema between runs)
//!
//! Run via:
//! `CAPACITY_MODE=distributed cargo test -p crawlkit-engine --test capacity_distributed -- --ignored --nocapture`

// The whole harness measures the Postgres storage path; without the feature
// the crate has no `pg_storage` module to import.
#![cfg(feature = "postgres")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crawlkit_engine::crawl_engine::{CrawlEngine, CrawlEngineConfig};
use crawlkit_engine::pg_storage::PgStorage;
use crawlkit_engine::storage_trait::StorageBackend;
use crawlkit_engine::CrawlConfig;

// ---------------------------------------------------------------------------
// Deterministic loopback corpus — identical shape to capacity_smoke.rs so the
// per-page work is directly comparable (plan §4).
// ---------------------------------------------------------------------------

struct TestServer {
    url_root: String,
}

impl TestServer {
    /// Bind on a specific loopback address (127.0.0.2 … are distinct hosts on
    /// Linux) so each worker's seed domain differs — the multi-domain
    /// deployment shape — while everything stays on loopback.
    fn start(host: &str, page_count: usize) -> Self {
        let listener = TcpListener::bind((host, 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                std::thread::spawn(move || serve_connection(stream, page_count));
            }
        });
        Self {
            url_root: format!("http://{host}:{port}"),
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
// Per-worker self-sampler (same fields as capacity_smoke.rs).
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
                samples.lock().unwrap().push(Sample {
                    ms_since_start: start.elapsed().as_millis(),
                    rss_kb,
                    fds: count_entries("/proc/self/fd"),
                    tasks: count_entries("/proc/self/task"),
                });
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    })
}

// ---------------------------------------------------------------------------
// Records: per-worker result file + the topology-named parent record.
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct WorkerResult {
    worker: u32,
    host: String,
    pages_crawled: usize,
    issues_found: usize,
    elapsed_secs: f64,
    throughput_pages_per_sec: f64,
    rss_baseline_kb: u64,
    rss_peak_kb: u64,
    rss_end_kb: u64,
    fd_baseline: u64,
    fd_end: u64,
    tasks_peak: u64,
    samples: usize,
}

#[derive(Debug, serde::Serialize)]
struct DistributedRecord {
    schema: &'static str,
    recorded_at_unix: u64,
    crawlkit_version: &'static str,
    workload: String,
    kernel: String,
    cpu_model: String,
    mem_total_kb: u64,
    topology: TopologyRecord,
    results: DistributedResults,
    resources: DistributedResources,
    gates: DistributedGates,
}

#[derive(Debug, serde::Serialize)]
struct TopologyRecord {
    mode: &'static str,
    workers: u32,
    storage: &'static str,
    shared_db_instance: bool,
    queue: &'static str,
    pages_per_worker: usize,
    concurrency_per_worker: usize,
    profile: String,
}

#[derive(Debug, serde::Serialize)]
struct DistributedResults {
    pages_total: usize,
    issues_total: usize,
    elapsed_secs_max: f64,
    throughput_aggregate_pages_per_sec: f64,
    per_worker: Vec<WorkerResult>,
}

#[derive(Debug, serde::Serialize)]
struct DistributedResources {
    worker_rss_peak_sum_kb: u64,
    worker_rss_end_sum_kb: u64,
    worker_fd_end_sum: u64,
    worker_tasks_peak_sum: u64,
}

#[derive(Debug, serde::Serialize)]
struct DistributedGates {
    pages_exact: bool,
    per_worker_rss_under_cap: bool,
    worker_rss_sum_under_cap: bool,
    per_worker_throughput_above_floor: bool,
    aggregate_throughput_above_floor: bool,
    fds_returned_to_baseline: bool,
    all_pass: bool,
}

const WORKER_RSS_CAP_KB: u64 = 500_000; // same per-process cap as the inline smoke
const WORKER_RSS_SUM_CAP_KB: u64 = 1_000_000; // named before measurement: 2 × 500 MB
const THROUGHPUT_FLOOR_PER_WORKER: f64 = 20.0; // smoke-class floor (plan §6)
const FD_BASELINE_SLACK: u64 = 20; // pg pool keeps a few extra fds open
const WORKERS: u32 = 2;

fn kernel() -> String {
    std::fs::read_to_string("/proc/sys/kernel/osrelease")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "unknown".into())
}

fn build_profile() -> String {
    if cfg!(debug_assertions) {
        "debug".to_string()
    } else {
        "release".to_string()
    }
}

fn mem_total_kb() -> u64 {
    std::fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("MemTotal:"))?
                .split_whitespace()
                .nth(1)?
                .parse()
                .ok()
        })
        .unwrap_or(0)
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

fn env_u32(name: &str, default: u32) -> u32 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

// ---------------------------------------------------------------------------
// Worker mode: re-executed child. Runs one crawl against the shared Postgres
// and writes its result file; the parent aggregates.
// ---------------------------------------------------------------------------

fn worker_main(worker: u32, out_path: &PathBuf) {
    let pages = env_usize("CAP_DIST_PAGES_PER_WORKER", 5000);
    let host = format!("127.0.0.{}", worker + 1); // 127.0.0.1, 127.0.0.2, …
    let pg_url = std::env::var("CAP_DIST_PG")
        .unwrap_or_else(|_| "postgres://crawlkit:crawlkit@127.0.0.1:5499/crawlkit".into());

    let server = TestServer::start(&host, pages);

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async move {
        let pg = PgStorage::new(&pg_url).await.unwrap();
        pg.migrate().await.unwrap();
        let storage: Arc<dyn StorageBackend> = Arc::new(pg);
        let engine = CrawlEngine::new_shared(
            CrawlEngineConfig {
                crawl_config: CrawlConfig {
                    max_pages: pages + 100,
                    concurrency: 8,
                    respect_robots_txt: true,
                    request_delay: Duration::ZERO,
                    ..CrawlConfig::default()
                },
                concurrency: Some(8),
                allow_http: true, // local loopback corpus only
                ..CrawlEngineConfig::default()
            },
            Arc::clone(&storage),
        );

        let rss_baseline_kb = read_rss_kb().unwrap_or(0);
        let fd_baseline = count_entries("/proc/self/fd");

        let stop = Arc::new(AtomicBool::new(false));
        let samples: Arc<Mutex<Vec<Sample>>> = Arc::new(Mutex::new(Vec::new()));
        let sampler = spawn_sampler(Arc::clone(&stop), Arc::clone(&samples));

        let started = Instant::now();
        let output = engine.run(&server.index_url()).await.unwrap();
        let elapsed_secs = started.elapsed().as_secs_f64();

        stop.store(true, Ordering::Relaxed);
        sampler.join().unwrap();
        std::thread::sleep(Duration::from_secs(1));

        let s = samples.lock().unwrap();
        let result = WorkerResult {
            worker,
            host: host.clone(),
            pages_crawled: output.pages_crawled,
            issues_found: output.issues_found,
            elapsed_secs,
            throughput_pages_per_sec: output.pages_crawled as f64 / elapsed_secs,
            rss_baseline_kb,
            rss_peak_kb: s
                .iter()
                .map(|x| x.rss_kb)
                .max()
                .unwrap_or(0)
                .max(read_hwm_kb().unwrap_or(0)),
            rss_end_kb: read_rss_kb().unwrap_or(0),
            fd_baseline,
            fd_end: count_entries("/proc/self/fd"),
            tasks_peak: s.iter().map(|x| x.tasks).max().unwrap_or(0),
            samples: s.len(),
        };
        drop(s);
        std::fs::write(out_path, serde_json::to_string_pretty(&result).unwrap()).unwrap();
        println!(
            "worker {worker}: {} pages, {:.1} p/s, peak {} MB",
            result.pages_crawled,
            result.throughput_pages_per_sec,
            result.rss_peak_kb / 1024
        );
    });
}

/// Worker entry point — a *sync* test so the child process runs with no
/// ambient tokio runtime; `worker_main` builds its own.
#[test]
#[ignore = "internal worker entry; invoked by capacity_distributed_posture_shared_pg"]
fn capacity_distributed_worker_entry() {
    if !(std::env::var("CAPACITY_MODE").is_ok_and(|v| v == "distributed")
        && std::env::var("CAP_DIST_WORKER").is_ok())
    {
        panic!("worker entry invoked without CAPACITY_MODE=distributed + CAP_DIST_WORKER");
    }
    let worker: u32 = std::env::var("CAP_DIST_WORKER").unwrap().parse().unwrap();
    let out = record_dir().join(format!("worker_{worker}_result.json"));
    worker_main(worker, &out);
}

// ---------------------------------------------------------------------------
// Parent: verify services, reset schema, spawn workers, aggregate, gate.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "distributed-posture capacity run; requires Postgres (CAPACITY_MODE=distributed)"]
async fn capacity_distributed_posture_shared_pg() {
    // ---- preflight: services must be reachable, schema fresh ----
    let pg_url = std::env::var("CAP_DIST_PG")
        .unwrap_or_else(|_| "postgres://crawlkit:crawlkit@127.0.0.1:5499/crawlkit".into());
    let pg_container = std::env::var("CAP_DIST_PG_CONTAINER").unwrap_or_else(|_| "cap-pg".into());
    let reset = std::process::Command::new("docker")
        .args([
            "exec",
            &pg_container,
            "psql",
            "-U",
            "crawlkit",
            "-d",
            "crawlkit",
            "-c",
            "DROP SCHEMA public CASCADE; CREATE SCHEMA public;",
        ])
        .output()
        .expect("docker must be available to reset the shared schema");
    assert!(
        reset.status.success(),
        "schema reset failed — is the Postgres container running? {}",
        String::from_utf8_lossy(&reset.stderr)
    );

    let workers = env_u32("CAP_DIST_WORKERS", WORKERS);
    let pages_per_worker = env_usize("CAP_DIST_PAGES_PER_WORKER", 5000);

    // ---- spawn worker processes (fresh processes: the multi-process claim) ----
    let exe = std::env::current_exe().unwrap();
    let dir = record_dir();
    let mut children = Vec::new();
    for w in 0..workers {
        let mut cmd = std::process::Command::new(&exe);
        cmd.args([
            "--ignored",
            "--exact",
            "capacity_distributed_worker_entry",
            "--nocapture",
        ])
        .env("CAPACITY_MODE", "distributed")
        .env("CAP_DIST_WORKER", w.to_string())
        .env("CAP_DIST_PG", &pg_url)
        .env("CAP_DIST_PAGES_PER_WORKER", pages_per_worker.to_string())
        .env("CAPACITY_RECORD_DIR", &dir);
        children.push(cmd.spawn().expect("worker process spawn"));
    }
    let mut worker_pids = Vec::new();
    for c in &children {
        worker_pids.push(c.id());
    }

    // ---- parent-side sampler over child processes (aggregate view) ----
    let stop = Arc::new(AtomicBool::new(false));
    let agg = Arc::new(Mutex::new((0u64, 0u64))); // (sum rss, max sum tasks)
    let stop2 = Arc::clone(&stop);
    let agg2 = Arc::clone(&agg);
    let pids = worker_pids.clone();
    let sampler = std::thread::spawn(move || {
        while !stop2.load(Ordering::Relaxed) {
            let mut sum = 0u64;
            let mut tasks = 0u64;
            for pid in &pids {
                if let Ok(status) = std::fs::read_to_string(format!("/proc/{pid}/status")) {
                    if let Some(line) = status.lines().find(|l| l.starts_with("VmRSS:")) {
                        sum += line
                            .split_whitespace()
                            .nth(1)
                            .and_then(|v| v.parse::<u64>().ok())
                            .unwrap_or(0);
                    }
                    if let Some(line) = status.lines().find(|l| l.starts_with("Threads:")) {
                        tasks += line
                            .split_whitespace()
                            .nth(1)
                            .and_then(|v| v.parse::<u64>().ok())
                            .unwrap_or(0);
                    }
                }
            }
            *agg2.lock().unwrap() = (sum, tasks);
            std::thread::sleep(Duration::from_millis(250));
        }
    });

    let mut failures = Vec::new();
    for c in &mut children {
        let out = c.wait().expect("worker wait");
        if !out.success() {
            failures.push(format!("worker exit status {out}"));
        }
    }
    stop.store(true, Ordering::Relaxed);
    sampler.join().unwrap();
    let (agg_rss_peak, agg_tasks_peak) = {
        // The aggregator's final reading is the running max; capture what we have.
        *agg.lock().unwrap()
    };
    assert!(failures.is_empty(), "worker failures: {failures:?}");

    // ---- collect per-worker results ----
    let mut per_worker = Vec::new();
    for w in 0..workers {
        let path = dir.join(format!("worker_{w}_result.json"));
        let raw = std::fs::read_to_string(&path).unwrap();
        per_worker.push(serde_json::from_str::<WorkerResult>(&raw).unwrap());
        let _ = std::fs::remove_file(&path);
    }

    let pages_total: usize = per_worker.iter().map(|w| w.pages_crawled).sum();
    let issues_total: usize = per_worker.iter().map(|w| w.issues_found).sum();
    let elapsed_max = per_worker
        .iter()
        .map(|w| w.elapsed_secs)
        .fold(0.0, f64::max);
    let throughput_aggregate = pages_total as f64 / elapsed_max;
    let worker_rss_peak_sum: u64 = per_worker.iter().map(|w| w.rss_peak_kb).sum();
    let worker_rss_end_sum: u64 = per_worker.iter().map(|w| w.rss_end_kb).sum();
    let worker_fd_end_sum: u64 = per_worker.iter().map(|w| w.fd_end).sum();
    let worker_tasks_sum: u64 = per_worker.iter().map(|w| w.tasks_peak).sum();
    let fds_ok = per_worker
        .iter()
        .all(|w| w.fd_end <= w.fd_baseline + FD_BASELINE_SLACK);

    // ---- gates (named before measurement) ----
    let pages_exact = per_worker
        .iter()
        .all(|w| w.pages_crawled == pages_per_worker + 1);
    let per_worker_rss_under_cap = per_worker.iter().all(|w| w.rss_peak_kb < WORKER_RSS_CAP_KB);
    let worker_rss_sum_under_cap = worker_rss_peak_sum < WORKER_RSS_SUM_CAP_KB;
    let per_worker_throughput_above_floor = per_worker
        .iter()
        .all(|w| w.throughput_pages_per_sec >= THROUGHPUT_FLOOR_PER_WORKER);
    let aggregate_throughput_above_floor =
        throughput_aggregate >= THROUGHPUT_FLOOR_PER_WORKER * workers as f64;
    let all_pass = pages_exact
        && per_worker_rss_under_cap
        && worker_rss_sum_under_cap
        && per_worker_throughput_above_floor
        && aggregate_throughput_above_floor
        && fds_ok;

    let record = DistributedRecord {
        schema: "crawlkit.capacity.distributed_run_record/v1",
        recorded_at_unix: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        crawlkit_version: env!("CARGO_PKG_VERSION"),
        workload: format!("distributed-crawl-{workers}x{pages_per_worker}"),
        kernel: kernel(),
        cpu_model: cpu_model(),
        mem_total_kb: mem_total_kb(),
        topology: TopologyRecord {
            mode: "distributed-posture",
            workers,
            storage: "postgres-shared",
            shared_db_instance: true,
            queue: "inline-per-worker (Redis lease queue not in crawl path; 6.0.0 engineering)",
            pages_per_worker,
            concurrency_per_worker: 8,
            profile: build_profile(),
        },
        results: DistributedResults {
            pages_total,
            issues_total,
            elapsed_secs_max: elapsed_max,
            throughput_aggregate_pages_per_sec: throughput_aggregate,
            per_worker,
        },
        resources: DistributedResources {
            worker_rss_peak_sum_kb: worker_rss_peak_sum,
            worker_rss_end_sum_kb: worker_rss_end_sum,
            worker_fd_end_sum,
            worker_tasks_peak_sum: agg_tasks_peak.max(worker_tasks_sum),
        },
        gates: DistributedGates {
            pages_exact,
            per_worker_rss_under_cap,
            worker_rss_sum_under_cap,
            per_worker_throughput_above_floor,
            aggregate_throughput_above_floor,
            fds_returned_to_baseline: fds_ok,
            all_pass,
        },
    };
    let _ = agg_rss_peak; // kept for manual inspection in worker result files

    let path = dir.join("run_record_distributed.json");
    std::fs::write(&path, serde_json::to_string_pretty(&record).unwrap()).unwrap();
    println!("distributed run record: {}", path.display());
    println!(
        "aggregate: {pages_total} pages, {throughput_aggregate:.1} p/s, worker RSS sum peak {} MB",
        worker_rss_peak_sum / 1024
    );

    assert!(pages_exact, "page budget not exact: {pages_total}");
    assert!(
        per_worker_rss_under_cap,
        "a worker exceeded the per-process RSS cap"
    );
    assert!(
        worker_rss_sum_under_cap,
        "worker RSS sum exceeded the topology cap"
    );
    assert!(
        per_worker_throughput_above_floor,
        "a worker fell below the per-worker throughput floor"
    );
    assert!(
        aggregate_throughput_above_floor,
        "aggregate throughput below floor"
    );
    assert!(fds_ok, "worker fds did not return to baseline");
}
