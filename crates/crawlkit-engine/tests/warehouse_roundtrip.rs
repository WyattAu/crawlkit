//! Round-trip contract test (6.0.0-alpha.1 exit criterion): fixture rows →
//! ADR-016 warehouse export (Parquet) → real S3 upload via the destination
//! client → signed GET → byte-identical object.
//!
//! The destination is **MinIO** (S3-compatible) started as a service by CI
//! (`.github/workflows/ci.yml`, `warehouse-roundtrip` job) or locally with
//! `docker run -p 9000:9000 minio/minio server /data`. Skipped — with an
//! explicit reason line — when `WAREHOUSE_S3_ENDPOINT` is unset, so the
//! plain `cargo test` path stays green everywhere.

// Compiled only with the `warehouse` feature (like `warehouse_drill.rs`):
// the destination clients are feature-gated, and CI's clippy/roadmap jobs
// run `--all-targets` on default features, where these imports would not
// exist. Without this gate the workspace build fails on default features.
#![cfg(feature = "warehouse")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chrono::Utc;
use crawlkit_engine::export::destinations::reqwest_transport::ReqwestTransport;
use crawlkit_engine::export::warehouse as wh;
use crawlkit_engine::export::{LoadError, S3Client, S3Config};

fn s3_test_config() -> Option<S3Config> {
    // CI and local runs use LocalStack's S3 (MinIO withdrew its freely
    // pullable community images: Docker Hub no longer hosts `minio/minio`
    // and quay.io returns 401 for anonymous pulls).
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
    let Some(cfg) = s3_test_config() else {
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
///
/// The rejection is served by a loopback server reproducing S3's canonical
/// 403 wire response (status + `AccessDenied` XML body) over real HTTP with
/// the production reqwest transport: the client performs full SigV4 signing
/// and live response classification. LocalStack's community edition accepts
/// any credentials, so the live-rejection path is pinned here
/// deterministically instead.
#[tokio::test]
async fn s3_error_classification_surfaces_from_live_transport() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let body = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
                <Error><Code>AccessDenied</Code><Message>Access Denied</Message>\
                <RequestId>RT-CONTRACT</RequestId></Error>";
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut sock, _) = listener.accept().await.unwrap();
        // Drain the signed PUT request head; the verdict is fixed.
        let mut buf = [0u8; 8192];
        let _ = sock.read(&mut buf).await;
        let resp = format!(
            "HTTP/1.1 403 Forbidden\r\nContent-Type: application/xml\r\n\
             Content-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        sock.write_all(resp.as_bytes()).await.unwrap();
        sock.shutdown().await.unwrap();
    });

    let cfg = S3Config {
        bucket: "rt-contract-bucket".into(),
        region: "us-east-1".into(),
        endpoint: format!("http://{addr}"),
        access_key: "crawlkit".into(),
        secret_key: "definitely-wrong".into(),
        session_token: None,
        path_style: true,
    };
    let transport = ReqwestTransport::new();
    let client = S3Client::new(cfg, &transport);
    let result = client
        .put_object("rt-contract/negative.parquet", b"x".to_vec())
        .await;
    server.await.unwrap();
    match result {
        Err(LoadError::Fatal { status: 403, .. }) => {}
        other => panic!("expected Fatal(403) for bad credentials, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// BigQuery destination contract (alpha.1 exit criterion): export → load →
// query → assert, verified through the destination itself. CI and local runs
// use goccy/bigquery-emulator; the same wire contract serves real BigQuery.
// ---------------------------------------------------------------------------

fn bigquery_config() -> Option<crawlkit_engine::export::BigQueryConfig> {
    let endpoint = std::env::var("WAREHOUSE_BQ_ENDPOINT").ok()?;
    Some(crawlkit_engine::export::BigQueryConfig {
        project_id: std::env::var("WAREHOUSE_BQ_PROJECT").unwrap_or_else(|_| "crawlkit".into()),
        dataset: std::env::var("WAREHOUSE_BQ_DATASET").unwrap_or_else(|_| "crawlkit".into()),
        bearer_token: std::env::var("WAREHOUSE_BQ_TOKEN").unwrap_or_else(|_| "test-token".into()),
        endpoint: Some(endpoint),
    })
}

/// Deterministic JSONL rows for the same crawl as the Parquet fixture (same
/// construction, second physical binding).
fn jsonl_fixture(crawl_id: &str) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
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
    let runs = wh::write_crawl_runs_jsonl(&[run]).expect("crawl_runs jsonl");
    let pages = wh::write_pages_jsonl(&[page]).expect("pages jsonl");
    let findings = wh::write_findings_jsonl(&[finding]).expect("findings jsonl");
    (runs, pages, findings)
}

/// The alpha.1 BigQuery exit criterion: every contract table loads with its
/// manifest-derived fields schema, and the loaded rows are verifiable
/// **through the destination** (jobs.query) — row counts and a value
/// round-trip — not through our own writer.
#[tokio::test]
async fn bigquery_roundtrip_load_and_query() {
    let Some(cfg) = bigquery_config() else {
        eprintln!(
            "skipping: WAREHOUSE_BQ_ENDPOINT not set — start bigquery-emulator and set the \
             WAREHOUSE_BQ_* variables to run the live load contract"
        );
        return;
    };

    let crawl_id = "rt-contract-c1";
    let (runs, pages, findings) = jsonl_fixture(crawl_id);
    let transport = ReqwestTransport::new();
    let client = crawlkit_engine::export::BigQueryClient::new(cfg.clone(), &transport);

    for (table, jsonl) in [
        ("crawl_runs", runs),
        ("pages", pages),
        ("findings", findings),
    ] {
        let manifest = match table {
            "crawl_runs" => wh::crawl_runs_manifest().unwrap(),
            "pages" => wh::pages_manifest().unwrap(),
            _ => wh::findings_manifest().unwrap(),
        };
        let fields = wh::bigquery_schema_fields_json(manifest).unwrap();
        client
            .load_jsonl(table, jsonl, &fields)
            .await
            .unwrap_or_else(|e| panic!("{table} load failed: {e}"));
    }

    // Row counts through the destination.
    for (table, want) in [("crawl_runs", 1u64), ("pages", 1), ("findings", 1)] {
        let sql = format!(
            "SELECT COUNT(*) AS n FROM `{}.{}` WHERE crawl_id = '{crawl_id}'",
            cfg.dataset, table
        );
        let resp = client
            .query(&sql)
            .await
            .unwrap_or_else(|e| panic!("{table} count query failed: {e}"));
        assert_eq!(
            resp["jobComplete"],
            serde_json::json!(true),
            "{table}: query did not complete: {resp}"
        );
        let n: u64 = resp["rows"][0]["f"][0]["v"]
            .as_str()
            .expect("COUNT(*) cell")
            .parse()
            .expect("count parses");
        assert_eq!(n, want, "{table}: loaded row count");
    }

    // A value round-trips: column names and string content survive the load.
    let sql = format!(
        "SELECT code FROM `{}.findings` WHERE crawl_id = '{crawl_id}'",
        cfg.dataset
    );
    let resp = client.query(&sql).await.unwrap();
    assert_eq!(
        resp["rows"][0]["f"][0]["v"],
        serde_json::json!("TEST_FINDING"),
        "finding code must round-trip: {resp}"
    );
}

// ---------------------------------------------------------------------------
// Snowflake destination contract. There is no practical Snowflake emulator,
// so this is a **loopback fake of the SQL Statements API v2** over real HTTP
// (the production reqwest transport): it pins the wire shape (path, auth
// header, COPY INTO statement), the statement-handle extraction, and the
// connector error classification (429 retryable, 401 fatal). The remaining
// gap — data landing in a real Snowflake table — stays an operator-owned
// smoke per docs/RELEASE_6_0_0_ALPHA_PLAN.md (stage decision recorded there).
// ---------------------------------------------------------------------------

/// One-shot HTTP fake: answers every connection with `status` + `body`,
/// recording the first request line, headers, and body it receives.
struct FakeStatementsApi {
    port: u16,
    captured: std::sync::Arc<std::sync::Mutex<Option<String>>>,
}

impl FakeStatementsApi {
    fn spawn(status: u16, body: &'static str) -> Self {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind loopback");
        let port = listener.local_addr().expect("addr").port();
        let captured = std::sync::Arc::new(std::sync::Mutex::new(None));
        let captured_clone = captured.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                // Read until end of headers, then to Content-Length.
                let mut buf = Vec::new();
                let mut chunk = [0u8; 4096];
                let header_end;
                loop {
                    let n = match std::io::Read::read(&mut stream, &mut chunk) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => n,
                    };
                    buf.extend_from_slice(&chunk[..n]);
                    if let Some(pos) = find_subslice(&buf, b"\r\n\r\n") {
                        header_end = pos + 4;
                        let head = String::from_utf8_lossy(&buf[..pos]);
                        let len: usize = head
                            .lines()
                            .find_map(|l| {
                                let (k, v) = l.split_once(':')?;
                                k.eq_ignore_ascii_case("content-length")
                                    .then(|| v.trim().parse().ok())?
                            })
                            .unwrap_or(0);
                        while buf.len() < header_end + len {
                            match std::io::Read::read(&mut stream, &mut chunk) {
                                Ok(0) | Err(_) => break,
                                Ok(n) => buf.extend_from_slice(&chunk[..n]),
                            }
                        }
                        *captured_clone.lock().unwrap() =
                            Some(String::from_utf8_lossy(&buf).into_owned());
                        break;
                    }
                }
                let resp = format!(
                    "HTTP/1.1 {status} T\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = std::io::Write::write_all(&mut stream, resp.as_bytes());
            }
        });
        Self { port, captured }
    }

    fn endpoint(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn snowflake_config(endpoint: String) -> crawlkit_engine::export::SnowflakeConfig {
    crawlkit_engine::export::SnowflakeConfig {
        account: "acct".into(),
        warehouse: "WH_TEST".into(),
        database: "CRAWLK".into(),
        schema: "PUBLIC".into(),
        bearer_token: "sf-token".into(),
        endpoint: Some(endpoint),
    }
}

#[tokio::test]
async fn snowflake_copy_over_real_http_shape_and_handle() {
    let fake = FakeStatementsApi::spawn(
        200,
        r#"{"statementHandle":"HANDLE-ABC","status":"running"}"#,
    );
    let transport = ReqwestTransport::new();
    let client = crawlkit_engine::export::SnowflakeClient::new(
        snowflake_config(fake.endpoint()),
        &transport,
    );
    let handle = client
        .copy_into("findings", "exports_stage", "exports/c1/findings.jsonl")
        .await
        .expect("COPY INTO submission");
    assert_eq!(handle, "HANDLE-ABC", "statement handle must be extracted");

    let captured = fake
        .captured
        .lock()
        .unwrap()
        .clone()
        .expect("request captured");
    assert!(
        captured.starts_with("POST /api/v2/statements "),
        "{captured}"
    );
    assert!(captured
        .to_ascii_lowercase()
        .contains("authorization: bearer sf-token"));
    assert!(captured.contains("COPY INTO CRAWLK.findings"), "{captured}");
    assert!(
        captured.contains("@exports_stage/exports/c1/findings.jsonl"),
        "{captured}"
    );
    assert!(captured.contains("TYPE = JSON"), "{captured}");
    assert!(captured.contains("MATCH_BY_COLUMN_NAME"), "{captured}");
}

#[tokio::test]
async fn snowflake_error_classification_over_real_http() {
    // 429 → retryable (the load may be resubmitted as-is).
    let fake429 = FakeStatementsApi::spawn(429, r#"{"message":"rate limited"}"#);
    let transport = ReqwestTransport::new();
    let client = crawlkit_engine::export::SnowflakeClient::new(
        snowflake_config(fake429.endpoint()),
        &transport,
    );
    match client.copy_into("findings", "s", "p").await {
        Err(LoadError::Retryable { status: 429 }) => {}
        other => panic!("expected Retryable(429), got {other:?}"),
    }

    // 401 → fatal (bad credentials are configuration, not load).
    let fake401 = FakeStatementsApi::spawn(401, r#"{"message":"unauthorized"}"#);
    let client = crawlkit_engine::export::SnowflakeClient::new(
        snowflake_config(fake401.endpoint()),
        &transport,
    );
    match client.copy_into("findings", "s", "p").await {
        Err(LoadError::Fatal { status: 401, .. }) => {}
        other => panic!("expected Fatal(401), got {other:?}"),
    }
}
