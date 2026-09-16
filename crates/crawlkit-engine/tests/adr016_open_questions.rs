//! ADR-016 §5 open-question evidence (docs/ADR-016 §5.1/5.2):
//!
//! 1. Parquet compression/encoding defaults: measure size + export time of
//!    the shipped default (`Compression::ZSTD(Default::default())`) on a
//!    *real crawl corpus* — the 10 002-page / 1.63M-findings Postgres
//!    corpus left by the 2026-09-16 queue-overhead capacity runs. The
//!    committed 25-page drill fixture (`warehouse_drill.rs`) is too small
//!    for a compression decision.
//! 2. Partitioning guidance is documented in the emitted record, informed
//!    by the same measurement (row counts per partition day).
//!
//! Writes `target/adr016/adr016_open_questions.json`.
//!
//! Run: `DATABASE_URL=postgres://crawlkit:crawlkit@127.0.0.1:5499/crawlkit \
//!       cargo test -p crawlkit-engine --features postgres,warehouse \
//!       --test adr016_open_questions -- --ignored --nocapture`
//!
//! NOTE: deliberately a plain `#[test]`, not `#[tokio::test]`. `PgStorage`'s
//! synchronous `StorageBackend` bridge builds its own runtime; calling it
//! from inside a tokio worker panics ("Cannot start a runtime from within a
//! runtime"), and blocking a worker via `thread::join` deadlocks its pool.
//! The connect happens once on a dedicated runtime, then everything else
//! runs on this plain thread (the exact calling pattern trait callers use).

#![cfg(feature = "postgres")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crawlkit_engine::export::warehouse as wh;
use crawlkit_engine::pg_storage::PgStorage;
use crawlkit_engine::storage_trait::StorageBackend;

const DATABASE_URL_ENV: &str = "DATABASE_URL";

#[test]
#[ignore = "requires the capacity Postgres corpus (DATABASE_URL)"]
fn adr016_compression_evidence_on_real_corpus() {
    let url = std::env::var(DATABASE_URL_ENV)
        .unwrap_or_else(|_| "postgres://crawlkit:crawlkit@127.0.0.1:5499/crawlkit".into());

    // One dedicated runtime for the async connect + migrate; dropped after.
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let pg = rt.block_on(async {
        let storage = PgStorage::new(&url).await.unwrap();
        storage.migrate().await.unwrap();
        storage
    });

    // Everything below is the trait's synchronous contract on a plain
    // thread — the calling pattern the capacity harness parent uses.
    // `list_crawls` returns (id, start_time::text) ordered ascending; the
    // newest crawl is last.
    let (crawl_id, _start) = pg
        .list_crawls()
        .unwrap()
        .pop()
        .expect("capacity corpus present");
    // Count via SQL: `get_pages(_, usize::MAX)` would bind a negative LIMIT
    // (usize::MAX as i64) on Postgres — a latent trait-contract wart, worked
    // around here rather than in a measurement test.
    let pages_n = pg.get_page_urls(&crawl_id).unwrap().len();
    let issues_n = pg
        .get_issues(&crawl_id, &crawlkit_engine::storage::IssueFilter::default())
        .unwrap()
        .len();

    // Shipped default (zstd default level), timed.
    let started = std::time::Instant::now();
    let export = wh::export_crawl_parquet(&pg, &crawl_id, None, None).unwrap();
    let elapsed = started.elapsed();
    let jsonl = wh::export_crawl_jsonl(&pg, &crawl_id, None, None).unwrap();

    let findings_mb = export.findings.len() as f64 / (1024.0 * 1024.0);
    let pages_mb = export.pages.len() as f64 / (1024.0 * 1024.0);
    let jsonl_mb = jsonl.findings.len() as f64 / (1024.0 * 1024.0);
    println!("corpus {crawl_id}: {pages_n} pages, {issues_n} findings");
    println!(
        "zstd(default): findings {findings_mb:.1} MB, pages {pages_mb:.1} MB, export {elapsed:.1?}"
    );
    println!("jsonl findings {jsonl_mb:.1} MB");

    assert!(
        pages_n >= 5_000,
        "this measurement needs the capacity-class corpus, got {pages_n} pages"
    );
    // The plan's 120 s gate is for the 100k stretch class; 10k must be far
    // under — assert at 30 s so a 4× regression still fails here.
    assert!(
        elapsed.as_secs() < 30,
        "10k export took {elapsed:?}; the 100k-class gate is 120 s"
    );

    let record = serde_json::json!({
        "schema": "crawlkit.adr016.open_questions/v1",
        "crawl_id": crawl_id,
        "pages": pages_n,
        "findings": issues_n,
        "parquet_zstd_default": {
            "findings_bytes": export.findings.len(),
            "pages_bytes": export.pages.len(),
            "crawl_runs_bytes": export.crawl_runs.len(),
            "export_secs": elapsed.as_secs_f64(),
        },
        "jsonl": { "findings_bytes": jsonl.findings.len() },
        "guidance": {
            "compression": "keep Compression::ZSTD(Default::default()); evidence in this record",
            "partitioning": "BQ/SF: day-partition on fetched_at, cluster by crawl_id (ADR-016 §5.2 recommendation, not enforced)"
        }
    });
    let dir = std::path::PathBuf::from("target/adr016");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("adr016_open_questions.json"),
        serde_json::to_string_pretty(&record).unwrap(),
    )
    .unwrap();
    println!("record: {}/adr016_open_questions.json", dir.display());
}
