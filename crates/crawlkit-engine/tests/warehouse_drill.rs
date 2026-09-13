//! Warehouse migration drill (ADR-016 §2.4: "export → load → query → verify"
//! with artifacts committed) — the CI-provable core of the stability gate.
//!
//! The drill treats the writer and reader as separate programs, mirroring a
//! customer's migration: the export side writes with this binary, the load
//! side "loads" by parsing the files with independent readers (arrow record
//! batches for the Parquet binding, raw line parsing for the JSONL binding —
//! NOT the writer's row structs), and the verify side compares counts and
//! spot values.
//!
//! The drill record (`warehouse-drill-out/drill_record.json`) is uploaded as
//! a CI artifact per ADR-016 §2.4; the remaining §2.4 item (a load against a
//! live warehouse) is recorded explicitly as not-proven-here.

// The drill exercises the `warehouse` bindings; compiled out otherwise.
#![cfg(feature = "warehouse")]
#![allow(clippy::unwrap_used)]

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use arrow_array::{Array, StringArray};
use chrono::{TimeZone, Utc};
use crawlkit_engine::export::warehouse::{
    arrow_schema_from_manifest, export_crawl_jsonl, export_crawl_parquet, pages_manifest,
    read_parquet_batches,
};
use crawlkit_engine::storage::{Issue, IssueCategory, IssueFilter, PageData, Severity, Storage};
use url::Url;

fn drill_out_dir() -> PathBuf {
    std::env::var("WAREHOUSE_DRILL_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("warehouse-drill-out"))
}

/// Deterministic fixture: 25 pages, 35 findings (25 SEO + 5 security +
/// 5 custom-plugin), one tenant stamped everywhere.
fn fixture_crawl(storage: &Storage) -> String {
    let cid = storage.start_crawl("https://drill.example/", None).unwrap();
    let page_count = 25usize;
    for i in 0..page_count {
        let id = format!("p{i:03}");
        storage.insert_page(&cid, &test_page(&id)).unwrap();
        storage
            .insert_issue(&test_issue(&format!("i{i:03}"), &id, false))
            .unwrap();
        if i % 5 == 0 {
            let mut sec = test_issue(&format!("s{i:03}"), &id, false);
            sec.severity = Severity::Error;
            sec.code = "SEC001".into();
            storage.insert_issue(&sec).unwrap();
            let mut custom = test_issue(&format!("c{i:03}"), &id, true);
            custom.code = "PLUG001".into();
            storage.insert_issue(&custom).unwrap();
        }
    }
    storage.finish_crawl(&cid, page_count, page_count).unwrap();
    cid
}

fn test_page(id: &str) -> PageData {
    PageData {
        id: id.to_string(),
        url: Url::parse(&format!("https://drill.example/{id}")).unwrap(),
        final_url: Url::parse(&format!("https://drill.example/{id}")).unwrap(),
        status_code: 200,
        title: Some(format!("Drill page {id}")),
        description: Some(format!("Fixture page for the migration drill ({id})")),
        canonical_url: None,
        word_count: Some(400),
        load_time_ms: Some(120),
        body_size: Some(2048),
        fetched_at: Utc.with_ymd_and_hms(2026, 9, 13, 12, 0, 0).unwrap(),
        links: vec![],
        tenant_id: Some("drill-tenant".to_string()),
        etag: None,
        last_modified: None,
        cwv_lcp: Some(2100.0),
        cwv_cls: Some(0.04),
        cwv_inp: None,
        has_structured_data: Some(true),
        schema_types: Some("Article".to_string()),
        viewport_ok: Some(true),
        has_csp: Some(false),
        has_hsts: Some(true),
        images_total: Some(2),
        images_missing_alt: Some(0),
        h1_count: Some(1),
        heading_count: Some(3),
        extractions: None,
    }
}

fn test_issue(id: &str, page_id: &str, custom: bool) -> Issue {
    Issue {
        id: id.to_string(),
        page_id: page_id.to_string(),
        category: if custom {
            IssueCategory::Custom("drill-plugin".to_string())
        } else {
            IssueCategory::Seo
        },
        severity: Severity::Warning,
        code: "SEO001".into(),
        title: format!("Issue {id}"),
        description: "drill fixture issue".into(),
        element: Some("h1".into()),
        recommendation: "fix it".into(),
        tenant_id: Some("drill-tenant".to_string()),
    }
}

/// Counts + spot-check results from the verify phase, per table.
#[derive(Debug, serde::Serialize)]
struct TableVerify {
    table: &'static str,
    binding: &'static str,
    rows: usize,
    spot_checks_passed: usize,
    spot_checks_total: usize,
}

#[derive(Debug, serde::Serialize)]
struct DrillRecord {
    schema: &'static str,
    drill_at_unix: u64,
    crawlkit_version: &'static str,
    crawl_id: String,
    fixture: FixtureCounts,
    parquet: Vec<TableVerify>,
    jsonl: Vec<TableVerify>,
    schema_matches_manifest: bool,
    custom_category_string: Option<bool>,
    idempotent_reexport: Option<bool>,
    live_warehouse_load: &'static str,
    all_checks_passed: bool,
}

#[derive(Debug, serde::Serialize)]
struct FixtureCounts {
    pages: usize,
    findings: usize,
}

#[test]
fn warehouse_migration_drill() {
    let storage = Storage::new_in_memory().unwrap();
    let cid = fixture_crawl(&storage);

    let pages_expected = storage.get_pages(&cid, usize::MAX).unwrap().len();
    let findings_expected = storage
        .get_issues(&cid, &IssueFilter::default())
        .unwrap()
        .len();
    assert_eq!(pages_expected, 25);
    assert_eq!(findings_expected, 35);

    // ------------------------------------------------------------------
    // Phase 1: EXPORT — both bindings, same crawl, tenant stamped.
    // ------------------------------------------------------------------
    let pq = export_crawl_parquet(&storage, &cid, Some("drill-tenant"), None).unwrap();
    let js = export_crawl_jsonl(&storage, &cid, Some("drill-tenant"), None).unwrap();

    // ------------------------------------------------------------------
    // Phase 2: LOAD — independent readers, not the writer's row structs.
    // ------------------------------------------------------------------
    let pages_batches = read_parquet_batches(&pq.pages).unwrap();
    let findings_batches = read_parquet_batches(&pq.findings).unwrap();
    let runs_batches = read_parquet_batches(&pq.crawl_runs).unwrap();

    // Reader-side schema check: the loaded Parquet schema must equal the
    // schema rendered from the manifest (the contract, not the code).
    let expected_pages_schema = arrow_schema_from_manifest(pages_manifest().unwrap()).unwrap();
    let schema_matches_manifest = pages_batches[0].schema().as_ref() == &expected_pages_schema;

    // JSONL: raw line parsing — each line must be a JSON object; field-set
    // conformance to the manifest is separately gated in the lib tests.
    let parse_jsonl = |buf: &[u8]| -> Vec<serde_json::Map<String, serde_json::Value>> {
        String::from_utf8(buf.to_vec())
            .unwrap()
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| {
                let v: serde_json::Value = serde_json::from_str(l).unwrap();
                v.as_object().unwrap().clone()
            })
            .collect()
    };
    let jsonl_pages = parse_jsonl(&js.pages);
    let jsonl_findings = parse_jsonl(&js.findings);
    let jsonl_runs = parse_jsonl(&js.crawl_runs);

    // ------------------------------------------------------------------
    // Phase 3: QUERY — aggregate over the loaded shapes.
    // ------------------------------------------------------------------
    let pq_pages_rows: usize = pages_batches.iter().map(|b| b.num_rows()).sum();
    let pq_findings_rows: usize = findings_batches.iter().map(|b| b.num_rows()).sum();
    let pq_runs_rows: usize = runs_batches.iter().map(|b| b.num_rows()).sum();

    // A real query: total findings by severity from the loaded Parquet
    // `findings` table (severity is manifest column index 4).
    let severity_col = findings_batches[0]
        .column(4)
        .as_any()
        .downcast_ref::<StringArray>()
        .unwrap();
    let mut severity_counts: BTreeMap<String, i64> = Default::default();
    for i in 0..severity_col.len() {
        *severity_counts
            .entry(severity_col.value(i).to_string())
            .or_default() += 1;
    }
    let severity_query_sums_to_total =
        severity_counts.values().sum::<i64>() == findings_expected as i64;
    let severity_error_count = severity_counts.get("error").copied().unwrap_or(0);

    // JSONL aggregation: custom-category findings survive as strings.
    let jsonl_custom = jsonl_findings
        .iter()
        .filter(|o| o["category"].as_str().unwrap_or("").starts_with("custom:"))
        .count();

    // ------------------------------------------------------------------
    // Phase 4: VERIFY — counts and spot values.
    // ------------------------------------------------------------------
    let mut pq_passed = 0usize;
    let mut pq_total = 0usize;
    let mut js_passed = 0usize;
    let mut js_total = 0usize;
    let check = |passed: &mut usize, total: &mut usize, ok: bool| {
        *total += 1;
        if ok {
            *passed += 1;
        }
    };

    check(
        &mut pq_passed,
        &mut pq_total,
        pq_pages_rows == pages_expected,
    );
    check(
        &mut pq_passed,
        &mut pq_total,
        pq_findings_rows == findings_expected,
    );
    check(&mut pq_passed, &mut pq_total, pq_runs_rows == 1);
    check(&mut pq_passed, &mut pq_total, severity_query_sums_to_total);
    check(&mut pq_passed, &mut pq_total, severity_error_count == 5);
    let pq_runs_crawl_id = runs_batches[0]
        .column(0)
        .as_any()
        .downcast_ref::<StringArray>()
        .unwrap();
    check(
        &mut pq_passed,
        &mut pq_total,
        pq_runs_crawl_id.value(0) == cid,
    );
    check(&mut pq_passed, &mut pq_total, schema_matches_manifest);

    check(
        &mut js_passed,
        &mut js_total,
        jsonl_pages.len() == pages_expected,
    );
    check(
        &mut js_passed,
        &mut js_total,
        jsonl_findings.len() == findings_expected,
    );
    check(&mut js_passed, &mut js_total, jsonl_runs.len() == 1);
    check(&mut js_passed, &mut js_total, jsonl_custom == 5);
    check(
        &mut js_passed,
        &mut js_total,
        jsonl_runs[0]["crawl_id"].as_str() == Some(cid.as_str()),
    );
    check(
        &mut js_passed,
        &mut js_total,
        jsonl_runs[0]["tenant_id"].as_str() == Some("drill-tenant"),
    );

    // Idempotency: re-export overwrites byte-identically (both bindings).
    let pq_again = export_crawl_parquet(&storage, &cid, Some("drill-tenant"), None).unwrap();
    let js_again = export_crawl_jsonl(&storage, &cid, Some("drill-tenant"), None).unwrap();
    let idempotent_reexport = Some(
        pq_again.pages == pq.pages
            && pq_again.findings == pq.findings
            && js_again.pages == js.pages
            && js_again.findings == js.findings,
    );

    // The custom plugin category must be a plain string in the data.
    let custom_category_string = Some(
        jsonl_findings
            .iter()
            .any(|o| o["category"].as_str() == Some("custom:drill-plugin")),
    );

    let parquet_verifies = vec![
        TableVerify {
            table: "crawl_runs",
            binding: "parquet",
            rows: pq_runs_rows,
            spot_checks_passed: pq_passed,
            spot_checks_total: pq_total,
        },
        TableVerify {
            table: "pages",
            binding: "parquet",
            rows: pq_pages_rows,
            spot_checks_passed: pq_passed,
            spot_checks_total: pq_total,
        },
        TableVerify {
            table: "findings",
            binding: "parquet",
            rows: pq_findings_rows,
            spot_checks_passed: pq_passed,
            spot_checks_total: pq_total,
        },
    ];
    let jsonl_verifies = vec![
        TableVerify {
            table: "crawl_runs",
            binding: "jsonl",
            rows: jsonl_runs.len(),
            spot_checks_passed: js_passed,
            spot_checks_total: js_total,
        },
        TableVerify {
            table: "pages",
            binding: "jsonl",
            rows: jsonl_pages.len(),
            spot_checks_passed: js_passed,
            spot_checks_total: js_total,
        },
        TableVerify {
            table: "findings",
            binding: "jsonl",
            rows: jsonl_findings.len(),
            spot_checks_passed: js_passed,
            spot_checks_total: js_total,
        },
    ];

    let all_checks_passed = parquet_verifies
        .iter()
        .chain(jsonl_verifies.iter())
        .all(|t| t.spot_checks_passed == t.spot_checks_total)
        && schema_matches_manifest
        && custom_category_string == Some(true)
        && idempotent_reexport == Some(true);

    let record = DrillRecord {
        schema: "crawlkit.warehouse.drill_record/v1",
        drill_at_unix: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        crawlkit_version: env!("CARGO_PKG_VERSION"),
        crawl_id: cid,
        fixture: FixtureCounts {
            pages: pages_expected,
            findings: findings_expected,
        },
        parquet: parquet_verifies,
        jsonl: jsonl_verifies,
        schema_matches_manifest,
        custom_category_string,
        idempotent_reexport,
        live_warehouse_load: "not proven here — ADR-016 §2.4 requires one documented \
              load against a live BigQuery or Snowflake; this drill proves \
              the export/load/query/verify cycle with independent readers",
        all_checks_passed,
    };

    let dir = drill_out_dir();
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("drill_record.json"),
        serde_json::to_string_pretty(&record).unwrap(),
    )
    .unwrap();

    assert!(
        all_checks_passed,
        "migration drill failed verification: {record:?}"
    );
}
