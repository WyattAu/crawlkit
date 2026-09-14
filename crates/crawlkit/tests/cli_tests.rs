#![allow(clippy::unwrap_used)]

use assert_cmd::Command;
use predicates::prelude::*;

fn crawlkit_cmd() -> Command {
    Command::cargo_bin("crawlkit").unwrap()
}

#[test]
fn version_flag_exits_zero_and_shows_version() {
    crawlkit_cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("crawlkit"));
}

#[test]
fn help_flag_exits_zero_and_shows_subcommands() {
    crawlkit_cmd().arg("--help").assert().success().stdout(
        predicate::str::contains("crawl")
            .and(predicate::str::contains("compare"))
            .and(predicate::str::contains("report")),
    );
}

#[test]
fn crawl_help_exits_zero_and_shows_flags() {
    crawlkit_cmd()
        .args(["crawl", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--max-pages").and(predicate::str::contains("--timeout")));
}

#[test]
fn compare_help_exits_zero() {
    crawlkit_cmd()
        .args(["compare", "--help"])
        .assert()
        .success();
}

#[test]
fn report_help_exits_zero() {
    crawlkit_cmd().args(["report", "--help"]).assert().success();
}

#[test]
fn crawl_no_args_exits_nonzero() {
    crawlkit_cmd().arg("crawl").assert().failure();
}

#[test]
fn crawl_invalid_url_exits_nonzero() {
    crawlkit_cmd()
        .args(["crawl", "not-a-url"])
        .assert()
        .failure();
}

#[test]
#[ignore]
fn crawl_unreachable_host_exits_or_zero_with_no_pages() {
    let assert = crawlkit_cmd()
        .args([
            "crawl",
            "https://invalid.example.test",
            "--max-pages",
            "1",
            "--timeout",
            "2",
        ])
        .timeout(std::time::Duration::from_secs(10))
        .assert();

    let outcome = assert.get_output();
    assert!(
        !outcome.status.success() || outcome.stdout.is_empty(),
        "crawl of unreachable host should either fail or produce no pages"
    );
}

// ---------------------------------------------------------------------------
// export-warehouse (ADR-016 bindings): end-to-end through the real binary.
// Feature-gated: the command only exists under the `warehouse` feature, so
// these tests compile out of default-feature runs.
// ---------------------------------------------------------------------------

#[cfg(feature = "warehouse")]
mod export_warehouse {
    use super::*;
    use chrono::TimeZone;
    use crawlkit_engine::storage::{Issue, IssueCategory, PageData, Severity, Storage};
    use std::path::PathBuf;

    fn seed_db(dir: &std::path::Path) -> PathBuf {
        let db = dir.join("crawlkit.db");
        let storage = Storage::new(&db).unwrap();
        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();
        let page = PageData {
            id: "p1".into(),
            url: "https://example.com/".parse().unwrap(),
            final_url: "https://example.com/".parse().unwrap(),
            status_code: 200,
            title: Some("Home".into()),
            description: Some("d".into()),
            canonical_url: None,
            word_count: Some(500),
            load_time_ms: Some(200),
            body_size: Some(1024),
            fetched_at: chrono::Utc.with_ymd_and_hms(2026, 9, 14, 12, 0, 0).unwrap(),
            links: vec![],
            tenant_id: None,
            etag: None,
            last_modified: None,
            cwv_lcp: Some(2500.5),
            cwv_cls: Some(0.05),
            cwv_inp: None,
            has_structured_data: Some(true),
            schema_types: Some("Article".into()),
            viewport_ok: Some(true),
            has_csp: Some(false),
            has_hsts: Some(true),
            images_total: Some(3),
            images_missing_alt: Some(1),
            h1_count: Some(1),
            heading_count: Some(5),
            extractions: None,
        };
        storage.insert_page(&crawl_id, &page).unwrap();
        storage
            .insert_issue(&Issue {
                id: "i1".into(),
                page_id: "p1".into(),
                category: IssueCategory::Seo,
                severity: Severity::Warning,
                code: "SEO001".into(),
                title: "t".into(),
                description: "d".into(),
                element: Some("h1".into()),
                recommendation: "r".into(),
                tenant_id: None,
            })
            .unwrap();
        storage.finish_crawl(&crawl_id, 1, 1).unwrap();
        drop(storage);
        db
    }

    #[test]
    fn export_parquet_writes_three_table_files() {
        let tmp = tempfile::tempdir().unwrap();
        let db = seed_db(tmp.path());
        let out = tempfile::tempdir().unwrap();

        crawlkit_cmd()
            .args([
                "export",
                "--db",
                db.to_str().unwrap(),
                "--format",
                "parquet",
                "--output",
                out.path().to_str().unwrap(),
            ])
            .assert()
            .success()
            .stdout(predicate::str::contains("crawl_runs.parquet"));

        for f in ["crawl_runs.parquet", "pages.parquet", "findings.parquet"] {
            assert!(
                out.path()
                    .join(f)
                    .metadata()
                    .map(|m| m.len() > 0)
                    .unwrap_or(false),
                "{f} missing or empty"
            );
        }
    }

    #[test]
    fn export_jsonl_writes_tables_plus_schema_manifest() {
        let tmp = tempfile::tempdir().unwrap();
        let db = seed_db(tmp.path());
        let out = tempfile::tempdir().unwrap();

        crawlkit_cmd()
            .args([
                "export",
                "--db",
                db.to_str().unwrap(),
                "--format",
                "jsonl",
                "--output",
                out.path().to_str().unwrap(),
            ])
            .assert()
            .success()
            .stdout(predicate::str::contains("_schema"));

        let pages = std::fs::read_to_string(out.path().join("pages.jsonl")).unwrap();
        assert!(pages.contains("\"page_id\":\"p1\""));
        assert!(pages.contains("\"crawl_id\":"));
        assert!(out.path().join("_schema").metadata().unwrap().len() > 0);
    }

    #[test]
    fn export_is_idempotent_byte_identical() {
        let tmp = tempfile::tempdir().unwrap();
        let db = seed_db(tmp.path());
        let out1 = tempfile::tempdir().unwrap();
        let out2 = tempfile::tempdir().unwrap();
        for out in [&out1, &out2] {
            crawlkit_cmd()
                .args([
                    "export",
                    "--db",
                    db.to_str().unwrap(),
                    "--format",
                    "parquet",
                    "--output",
                    out.path().to_str().unwrap(),
                ])
                .assert()
                .success();
        }
        assert_eq!(
            std::fs::read(out1.path().join("pages.parquet")).unwrap(),
            std::fs::read(out2.path().join("pages.parquet")).unwrap(),
            "re-export must be byte-identical (ADR-016 §2.2 rule 4)"
        );
    }

    #[test]
    fn export_unknown_crawl_id_fails_cleanly() {
        let tmp = tempfile::tempdir().unwrap();
        let db = seed_db(tmp.path());
        crawlkit_cmd()
            .args([
                "export",
                "--db",
                db.to_str().unwrap(),
                "--crawl-id",
                "no-such-crawl",
            ])
            .assert()
            .failure()
            .stderr(predicate::str::contains("not found"));
    }

    #[test]
    fn export_empty_db_lists_helpfully() {
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("empty.db");
        Storage::new(&db).unwrap();
        crawlkit_cmd()
            .args(["export", "--db", db.to_str().unwrap()])
            .assert()
            .failure()
            .stderr(predicate::str::contains("no crawls found"));
    }
}
