//! Round-trip contract test (6.0.0-alpha.1 exit criterion): fixture rows →
//! ADR-016 warehouse export (Parquet) → real S3 upload via the destination
//! client → signed GET → byte-identical object.
//!
//! The destination is **MinIO** (S3-compatible) started as a service by CI
//! (`.github/workflows/ci.yml`, `warehouse-roundtrip` job) or locally with
//! `docker run -p 9000:9000 minio/minio server /data`. Skipped — with an
//! explicit reason line — when `WAREHOUSE_S3_ENDPOINT` is unset, so the
//! plain `cargo test` path stays green everywhere.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chrono::Utc;
use crawlkit_engine::export::destinations::reqwest_transport::ReqwestTransport;
use crawlkit_engine::export::warehouse as wh;
use crawlkit_engine::export::{LoadError, S3Client, S3Config};

fn minio_config() -> Option<S3Config> {
    let endpoint = std::env::var("WAREHOUSE_S3_ENDPOINT").ok()?;
    Some(S3Config {
        bucket: std::env::var("WAREHOUSE_S3_BUCKET").unwrap_or_else(|_| "crawlkit-exports".into()),
        region: std::env::var("WAREHOUSE_S3_REGION").unwrap_or_else(|_| "us-east-1".into()),
        endpoint,
        access_key: std::env::var("WAREHOUSE_S3_ACCESS_KEY")
            .unwrap_or_else(|_| "minioadmin".into()),
        secret_key: std::env::var("WAREHOUSE_S3_SECRET_KEY")
            .unwrap_or_else(|_| "minioadmin".into()),
        session_token: None,
        path_style: true,
    })
}

fn fixture_export(crawl_id: &str) -> wh::ParquetCrawlExport {
    // Deterministic rows exercising every required column of all three
    // contract tables (same construction the warehouse drill uses).
    let now = Utc::now();
    let run = wh::CrawlRunRow {
        crawl_id: crawl_id.to_string(),
        target_url: "http://fixture.test/".to_string(),
        start_time: Some(now),
        end_time: Some(now),
        pages_crawled: 1,
        total_issues: 1,
        tenant_id: None,
        sequence: None,
    };
    let page = wh::PageRow {
        crawl_id: crawl_id.to_string(),
        page_id: "p1".to_string(),
        url: "http://fixture.test/".to_string(),
        final_url: "http://fixture.test/".to_string(),
        status_code: 200,
        title: Some("fixture".into()),
        description: None,
        canonical_url: None,
        word_count: Some(12),
        load_time_ms: Some(5),
        body_size: Some(300),
        fetched_at: now,
        tenant_id: None,
        etag: None,
        last_modified: None,
        cwv_lcp_ms: None,
        cwv_cls: None,
        cwv_inp_ms: None,
        has_structured_data: Some(false),
        schema_types: None,
        viewport_ok: Some(true),
        has_csp: Some(false),
        has_hsts: Some(false),
        images_total: Some(0),
        images_missing_alt: Some(0),
        h1_count: Some(1),
        heading_count: Some(2),
        extractions_json: None,
    };
    let finding = wh::FindingRow {
        crawl_id: crawl_id.to_string(),
        page_id: "p1".to_string(),
        issue_id: "i1".to_string(),
        category: "seo".to_string(),
        severity: "low".to_string(),
        code: "TEST_FINDING".to_string(),
        title: "fixture finding".into(),
        description: "deterministic row".into(),
        element: None,
        recommendation: "none".into(),
        tenant_id: None,
    };

    let mut crawl_runs = Vec::new();
    wh::write_crawl_runs_parquet(&[run], &mut crawl_runs).expect("crawl_runs parquet");
    let mut pages = Vec::new();
    wh::write_pages_parquet(&[page], &mut pages).expect("pages parquet");
    let mut findings = Vec::new();
    wh::write_findings_parquet(&[finding], &mut findings).expect("findings parquet");
    wh::ParquetCrawlExport {
        crawl_runs,
        pages,
        findings,
    }
}

/// The full round-trip: export → PUT (all three tables) → signed GET →
/// byte-identical → idempotent re-upload. This is the alpha.1 exit criterion
/// for the S3 destination.
#[tokio::test]
async fn s3_roundtrip_parquet_is_byte_identical() {
    let Some(cfg) = minio_config() else {
        eprintln!(
            "skipping: WAREHOUSE_S3_ENDPOINT not set — start MinIO and set the \
             WAREHOUSE_S3_* variables to run the live round-trip contract"
        );
        return;
    };

    let crawl_id = "rt-contract-c1";
    let export = fixture_export(crawl_id);

    let transport = ReqwestTransport::new();
    let client = S3Client::new(cfg, &transport);
    let layout = wh::s3_layout(crawl_id);
    let [runs_key, pages_key, findings_key] = [
        layout.files[0].name.clone(),
        layout.files[1].name.clone(),
        layout.files[2].name.clone(),
    ];

    client
        .put_object(&runs_key, export.crawl_runs.clone())
        .await
        .unwrap_or_else(|e| panic!("crawl_runs upload failed: {e}"));
    client
        .put_object(&pages_key, export.pages.clone())
        .await
        .unwrap_or_else(|e| panic!("pages upload failed: {e}"));
    client
        .put_object(&findings_key, export.findings.clone())
        .await
        .unwrap_or_else(|e| panic!("findings upload failed: {e}"));

    // Signed GET → byte-identical for every table.
    assert_eq!(
        client.get_object(&runs_key).await.unwrap(),
        export.crawl_runs,
        "crawl_runs round-trip must be byte-identical"
    );
    assert_eq!(
        client.get_object(&pages_key).await.unwrap(),
        export.pages,
        "pages round-trip must be byte-identical"
    );
    assert_eq!(
        client.get_object(&findings_key).await.unwrap(),
        export.findings,
        "findings round-trip must be byte-identical"
    );

    // Idempotent overwrite: same bytes → same object (no versioning drift on
    // the contract path; re-running a pipeline never duplicates data).
    client
        .put_object(&findings_key, export.findings.clone())
        .await
        .unwrap();
    assert_eq!(
        client.get_object(&findings_key).await.unwrap(),
        export.findings
    );
}

/// Bad credentials must classify as **Fatal**, not retryable — the same
/// posture as every other connector (retrying 403s is the bug that turns a
/// misconfiguration into an outage).
#[tokio::test]
async fn s3_error_classification_surfaces_from_live_transport() {
    let Some(mut cfg) = minio_config() else {
        eprintln!("skipping: WAREHOUSE_S3_ENDPOINT not set");
        return;
    };
    cfg.secret_key = "definitely-wrong".into();
    let transport = ReqwestTransport::new();
    let client = S3Client::new(cfg, &transport);
    match client
        .put_object("rt-contract/negative.parquet", b"x".to_vec())
        .await
    {
        Err(LoadError::Fatal { status: 403, .. }) => {}
        other => panic!("expected Fatal(403) for bad credentials, got {other:?}"),
    }
}
