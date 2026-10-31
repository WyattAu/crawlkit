//! `crawlkit lighthouse` — Lighthouse JSON report ingestion (GAP G5).
//!
//! Reads a Lighthouse report file produced by
//! `lighthouse --output=json --output-path lh-report.json`, runs it through
//! the engine's [`LighthouseAnalyzer`], and either stores the resulting
//! findings in a crawl database (`--crawl`) or prints them. Running
//! Lighthouse itself is deliberately out of scope: this command never shells
//! out and makes no network requests, which keeps ingestion testable from
//! fixture files.

use anyhow::{Context, Result};
use std::path::PathBuf;

use crawlkit_engine::analyzers::lighthouse::LighthouseAnalyzer;
use crawlkit_engine::analyzers::Analyzer;

/// Arguments for `crawlkit lighthouse`.
pub(crate) struct LighthouseParams {
    /// Path to the Lighthouse JSON report file.
    pub report: PathBuf,
    /// URL the report was generated for (findings are attributed to it).
    pub url: String,
    /// Crawl database to store findings into. `None` prints instead.
    pub crawl: Option<PathBuf>,
}

/// Ingest one Lighthouse report: parse, classify, print or store.
pub fn run(params: LighthouseParams) -> Result<()> {
    let report_json = std::fs::read_to_string(&params.report)
        .with_context(|| format!("reading Lighthouse report {}", params.report.display()))?;

    // The analyzer reads the raw report through the ordinary analysis
    // context, so ingestion exercises exactly the code a crawl would run
    // when a stored body is a Lighthouse report.
    let url =
        url::Url::parse(&params.url).with_context(|| format!("invalid URL: {}", params.url))?;
    let parsed = crawlkit_engine::HtmlParser::parse("", &url);
    let ctx = crawlkit_engine::analyzers::AnalysisContext {
        body: Some(report_json.as_str()),
        ..crawlkit_engine::analyzers::AnalysisContext::new(&parsed)
    };
    let findings = LighthouseAnalyzer::new().analyze(&ctx);

    if findings.is_empty() {
        anyhow::bail!(
            "no findings produced: {} does not look like a Lighthouse JSON report \
             (expected `lighthouseVersion` and `audits`)",
            params.report.display()
        );
    }

    match params.crawl.as_deref() {
        Some(db) => store_findings(db, &url, &report_json, &findings),
        None => {
            print_findings(&findings);
            Ok(())
        }
    }
}

fn print_findings(findings: &[crawlkit_engine::Finding]) {
    let header = format!("| {:<8} | {:<30} | {} |", "Severity", "Code", "Title");
    println!("{header}");
    println!("|----------|--------------------------------|-------|");
    for f in findings {
        println!(
            "| {:<8} | {:<30} | {} |",
            f.severity.as_str(),
            f.code,
            f.title
        );
    }
    println!(
        "\n{} findings. Pass --crawl <DB> to store them.",
        findings.len()
    );
}

/// Store the findings under a fresh crawl, with one page row carrying the
/// CWV measurements so the `cwv_*` columns stay populated for history/trend.
fn store_findings(
    db: &std::path::Path,
    url: &url::Url,
    report_json: &str,
    findings: &[crawlkit_engine::Finding],
) -> Result<()> {
    let storage = crawlkit_engine::storage::Storage::new(db)
        .with_context(|| format!("opening crawl database {}", db.display()))?;
    let crawl_id = storage
        .start_crawl(url.as_str(), Some("lighthouse ingestion"))
        .context("starting crawl record")?;

    // Lab measurements into the page row so the same columns a crawl fills
    // (history, trend, score history) are populated here too. A report too
    // malformed to re-parse still stores its error finding; the columns then
    // stay NULL, which is the honest value.
    let report = crawlkit_engine::analyzers::lighthouse::parse_lighthouse_report(report_json).ok();
    let (lcp, cls, tbt) = report
        .as_ref()
        .map(|r| (r.lcp_ms, r.cls, r.tbt_ms))
        .unwrap_or((None, None, None));

    let mut page = crawlkit_engine::storage::PageData {
        id: uuid::Uuid::new_v4().to_string(),
        url: url.clone(),
        ..crawlkit_engine::storage::PageData::default()
    };
    page.final_url = page.url.clone();
    page.cwv_lcp = lcp;
    page.cwv_cls = cls;
    page.cwv_inp = tbt;
    storage
        .insert_page(&crawl_id, &page)
        .context("inserting page row")?;

    let issues: Vec<crawlkit_engine::storage::Issue> = findings
        .iter()
        .map(|f| crawlkit_engine::storage::Issue {
            id: uuid::Uuid::new_v4().to_string(),
            page_id: page.id.clone(),
            category: f.category.clone(),
            severity: f.severity,
            code: f.code.clone(),
            title: f.title.clone(),
            description: f.description.clone(),
            element: None,
            recommendation: f.recommendation.clone(),
            tenant_id: None,
        })
        .collect();
    let count = issues.len();
    storage
        .insert_issues(&issues)
        .context("inserting lighthouse findings")?;
    storage
        .finish_crawl(&crawl_id, 1, count)
        .context("closing crawl record")?;

    println!(
        "Stored {count} lighthouse findings for {url} in {} (crawl {crawl_id}).",
        db.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD_REPORT: &str = r#"{
        "lighthouseVersion": "11.0.0",
        "audits": {
            "largest-contentful-paint": {"numericValue": 1200.0},
            "cumulative-layout-shift": {"numericValue": 0.02},
            "total-blocking-time": {"numericValue": 90.0},
            "server-response-time": {"numericValue": 250.0}
        },
        "categories": {
            "performance": {"score": 0.96},
            "accessibility": {"score": 0.95},
            "best-practices": {"score": 1.0},
            "seo": {"score": 0.92}
        }
    }"#;

    #[test]
    fn good_report_ingests_to_info_findings() {
        let findings = LighthouseAnalyzer::new().analyze_report(GOOD_REPORT, "https://x.test/");
        assert!(!findings.is_empty());
        assert!(
            findings
                .iter()
                .all(|f| f.severity == crawlkit_engine::Severity::Info),
            "an all-good report must carry no warnings/errors: {:?}",
            findings
                .iter()
                .map(|f| (&f.code, f.severity))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn unrelated_file_stays_silent_so_run_can_bail_with_guidance() {
        // run() turns an empty finding set into an error naming the file;
        // the analyzer itself stays silent by design.
        let findings =
            LighthouseAnalyzer::new().analyze_report("<html><body>hi</body></html>", "u");
        assert!(findings.is_empty());
    }

    #[test]
    fn stores_findings_into_sqlite_db() {
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("crawl.db");
        let report = tmp.path().join("lh.json");
        std::fs::write(&report, GOOD_REPORT).unwrap();

        run(LighthouseParams {
            report,
            url: "https://store.test/".to_string(),
            crawl: Some(db.clone()),
        })
        .unwrap();

        let storage = crawlkit_engine::storage::Storage::new(&db).unwrap();
        let crawl_id = storage.get_latest_crawl_id().unwrap().unwrap();
        let pages = storage.get_pages(&crawl_id, 10).unwrap();
        assert_eq!(pages.len(), 1);
        let issues = storage
            .get_issues(&crawl_id, &crawlkit_engine::storage::IssueFilter::default())
            .unwrap();
        assert!(
            issues.iter().any(|i| i.code == "cwv:lcp"),
            "CWV findings stored: {:?}",
            issues.iter().map(|i| &i.code).collect::<Vec<_>>()
        );
    }

    #[test]
    fn malformed_report_still_stores_the_error_finding() {
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("crawl.db");
        let report = tmp.path().join("lh.json");
        std::fs::write(&report, "{\"lighthouseVersion\": \"11\", \"audits\": {").unwrap();

        run(LighthouseParams {
            report,
            url: "https://store.test/".to_string(),
            crawl: Some(db.clone()),
        })
        .unwrap();

        let storage = crawlkit_engine::storage::Storage::new(&db).unwrap();
        let crawl_id = storage.get_latest_crawl_id().unwrap().unwrap();
        let issues = storage
            .get_issues(&crawl_id, &crawlkit_engine::storage::IssueFilter::default())
            .unwrap();
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].code, "lighthouse:invalid-report");
    }
}
