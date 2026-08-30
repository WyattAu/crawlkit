//! Storage trait contract harness.
//!
//! Defines invariants that *any* `StorageBackend` implementation must
//! satisfy. The in-memory SQLite implementation is the reference baseline.
//! When a second implementation (e.g. PostgreSQL) is added, it must call
//! `assert_storage_contract` so the two cannot silently diverge.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use chrono::Utc;
use crawlkit_engine::storage::{Issue, IssueCategory, IssueFilter, PageData, Severity};
use crawlkit_engine::storage_trait::{new_in_memory_backend, StorageBackend};
use url::Url;

fn make_page(id: &str, url: &str, status: u16) -> PageData {
    PageData {
        id: id.to_string(),
        url: Url::parse(url).unwrap(),
        final_url: Url::parse(url).unwrap(),
        status_code: status,
        title: Some(format!("Page {id}")),
        description: None,
        canonical_url: None,
        word_count: Some(500),
        load_time_ms: Some(200),
        body_size: Some(1024),
        fetched_at: Utc::now(),
        links: vec![],
        tenant_id: None,
        etag: None,
        last_modified: None,
        cwv_lcp: None,
        cwv_cls: None,
        cwv_inp: None,
        has_structured_data: None,
        schema_types: None,
        viewport_ok: None,
        has_csp: None,
        has_hsts: None,
        images_total: None,
        images_missing_alt: None,
        h1_count: None,
        heading_count: None,
        extractions: None,
    }
}

fn make_issue(id: &str, page_id: &str, cat: IssueCategory, sev: Severity) -> Issue {
    Issue {
        id: id.to_string(),
        page_id: page_id.to_string(),
        category: cat,
        severity: sev,
        code: format!("{id}001"),
        title: format!("Issue {id}"),
        description: format!("Desc {id}"),
        element: None,
        recommendation: "Fix it".to_string(),
        tenant_id: None,
    }
}

/// Assert the full storage contract against a freshly-constructed backend.
#[allow(dead_code)]
pub fn assert_storage_contract(backend: &dyn StorageBackend) {
    // --- start/finish a crawl ---
    let crawl_id = backend.start_crawl("https://example.com", None).unwrap();
    assert!(!crawl_id.is_empty());

    // --- insert + retrieve a page ---
    let page = make_page("p1", "https://example.com/", 200);
    backend.insert_page(&crawl_id, &page).unwrap();

    let got = backend.get_page(&crawl_id, "https://example.com/").unwrap();
    assert!(got.is_some(), "inserted page must be retrievable");
    let got = got.unwrap();
    assert_eq!(got.id, "p1");
    assert_eq!(got.status_code, 200);

    // missing page returns None
    assert!(backend
        .get_page(&crawl_id, "https://example.com/missing")
        .unwrap()
        .is_none());

    // --- batch insert ---
    let batch = vec![
        make_page("p2", "https://example.com/about", 200),
        make_page("p3", "https://example.com/contact", 200),
    ];
    backend.insert_pages_batch(&crawl_id, &batch).unwrap();
    let all = backend.get_pages(&crawl_id, 10).unwrap();
    assert_eq!(all.len(), 3, "all inserted pages must be retrievable");

    // --- limit is a hard cap ---
    let limited = backend.get_pages(&crawl_id, 2).unwrap();
    assert!(limited.len() <= 2);

    // --- issues ---
    let issue = make_issue("i1", "p1", IssueCategory::Seo, Severity::Error);
    backend.insert_issue(&issue).unwrap();
    let issues = backend
        .get_issues(&crawl_id, &IssueFilter::default())
        .unwrap();
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].id, "i1");

    // batch issues
    let batch_issues = vec![
        make_issue("i2", "p1", IssueCategory::Images, Severity::Warning),
        make_issue("i3", "p1", IssueCategory::Security, Severity::Critical),
    ];
    backend.insert_issues_batch(&batch_issues).unwrap();
    let all_issues = backend
        .get_issues(&crawl_id, &IssueFilter::default())
        .unwrap();
    assert_eq!(all_issues.len(), 3);

    // --- stats ---
    let stats = backend.get_stats(&crawl_id).unwrap();
    assert_eq!(stats.total_pages, 3);
    assert_eq!(stats.total_issues, 3);

    // --- conditional requests ---
    let mut cond_page = make_page("p4", "https://example.com/cached", 200);
    cond_page.etag = Some("\"etag1\"".to_string());
    cond_page.last_modified = Some("Mon, 01 Jan 2025 00:00:00 GMT".to_string());
    backend.insert_page(&crawl_id, &cond_page).unwrap();
    let cond = backend
        .get_page_conditional(&crawl_id, "https://example.com/cached")
        .unwrap();
    assert!(cond.is_some());
    let (_, etag, lm) = cond.unwrap();
    assert_eq!(etag.as_deref(), Some("\"etag1\""));
    assert!(lm.is_some());

    // --- finish_crawl ---
    backend.finish_crawl(&crawl_id, 4, 3).unwrap();
}

#[test]
fn in_memory_sqlite_satisfies_storage_contract() {
    let backend = new_in_memory_backend().unwrap();
    assert_storage_contract(backend.as_ref());
    backend.finish().unwrap();
}
