//! Multi-tenant end-to-end isolation tests.
//!
//! Verifies that tenant A cannot see, access, or affect tenant B's data
//! through the `StorageBackend` API: page listings, issue listings, crawl
//! metadata, and tenant-scoped purges. Also pins the documented behaviour
//! that NULL-tenant rows are shared/global and visible to every tenant.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use chrono::Utc;
use crawlkit_engine::storage::{Issue, IssueCategory, IssueFilter, PageData, Severity};
use crawlkit_engine::storage_trait::{new_in_memory_backend, StorageBackend};
use url::Url;

const TENANT_A: &str = "tenant-a";
const TENANT_B: &str = "tenant-b";

fn make_page_for_tenant(id: &str, url: &str, tenant: Option<&str>) -> PageData {
    PageData {
        id: id.to_string(),
        url: Url::parse(url).unwrap(),
        final_url: Url::parse(url).unwrap(),
        status_code: 200,
        title: Some(format!("Page {id}")),
        description: None,
        canonical_url: None,
        word_count: Some(500),
        load_time_ms: Some(200),
        body_size: Some(1024),
        fetched_at: Utc::now(),
        links: vec![],
        tenant_id: tenant.map(std::string::ToString::to_string),
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

fn make_issue_for_tenant(id: &str, page_id: &str, tenant: Option<&str>) -> Issue {
    Issue {
        id: id.to_string(),
        page_id: page_id.to_string(),
        category: IssueCategory::Seo,
        severity: Severity::Error,
        code: format!("{id}001"),
        title: format!("Issue {id}"),
        description: format!("Description for issue {id}"),
        element: None,
        recommendation: "Fix this".to_string(),
        tenant_id: tenant.map(std::string::ToString::to_string),
    }
}

/// Seed one crawl per tenant, each with a page (and optionally an issue).
fn seed_tenant_crawls(
    storage: &dyn StorageBackend,
    with_issues: bool,
) -> (String, String) {
    let crawl_a = storage.start_crawl("https://a.example.com", Some(TENANT_A)).unwrap();
    let crawl_b = storage.start_crawl("https://b.example.com", Some(TENANT_B)).unwrap();

    storage
        .insert_page(&crawl_a, &make_page_for_tenant("pa1", "https://a.example.com/", Some(TENANT_A)))
        .unwrap();
    storage
        .insert_page(&crawl_b, &make_page_for_tenant("pb1", "https://b.example.com/", Some(TENANT_B)))
        .unwrap();

    if with_issues {
        storage
            .insert_issue(&make_issue_for_tenant("ia1", "pa1", Some(TENANT_A)))
            .unwrap();
        storage
            .insert_issue(&make_issue_for_tenant("ib1", "pb1", Some(TENANT_B)))
            .unwrap();
    }

    (crawl_a, crawl_b)
}

#[test]
fn tenant_a_cannot_see_tenant_b_pages() {
    let storage = new_in_memory_backend().unwrap();
    let (crawl_a, crawl_b) = seed_tenant_crawls(storage.as_ref(), false);

    // Tenant A sees only its own page, with its own tenant stamp.
    let pages_a = storage.get_pages_for_tenant(&crawl_a, TENANT_A, 100).unwrap();
    assert_eq!(pages_a.len(), 1);
    assert_eq!(pages_a[0].id, "pa1");
    assert_eq!(pages_a[0].tenant_id, Some(TENANT_A.to_string()));

    // Tenant B sees only its own page, with its own tenant stamp.
    let pages_b = storage.get_pages_for_tenant(&crawl_b, TENANT_B, 100).unwrap();
    assert_eq!(pages_b.len(), 1);
    assert_eq!(pages_b[0].id, "pb1");
    assert_eq!(pages_b[0].tenant_id, Some(TENANT_B.to_string()));

    // Tenant B querying tenant A's crawl sees nothing...
    let b_views_a_crawl = storage.get_pages_for_tenant(&crawl_a, TENANT_B, 100).unwrap();
    assert!(b_views_a_crawl.is_empty(), "tenant B must not see tenant A's pages");

    // ...and tenant A querying tenant B's crawl sees nothing.
    let a_views_b_crawl = storage.get_pages_for_tenant(&crawl_b, TENANT_A, 100).unwrap();
    assert!(a_views_b_crawl.is_empty(), "tenant A must not see tenant B's pages");
}

#[test]
fn tenant_a_cannot_see_tenant_b_issues() {
    let storage = new_in_memory_backend().unwrap();
    let (crawl_a, crawl_b) = seed_tenant_crawls(storage.as_ref(), true);
    let default_filter = IssueFilter::default();

    let issues_a = storage
        .get_issues_for_tenant(&crawl_a, TENANT_A, &default_filter)
        .unwrap();
    assert_eq!(issues_a.len(), 1);
    assert_eq!(issues_a[0].id, "ia1");
    assert_eq!(issues_a[0].tenant_id, Some(TENANT_A.to_string()));

    let issues_b = storage
        .get_issues_for_tenant(&crawl_b, TENANT_B, &default_filter)
        .unwrap();
    assert_eq!(issues_b.len(), 1);
    assert_eq!(issues_b[0].id, "ib1");
    assert_eq!(issues_b[0].tenant_id, Some(TENANT_B.to_string()));

    // Cross-tenant issue reads are empty in both directions.
    let b_views_a_issues = storage
        .get_issues_for_tenant(&crawl_a, TENANT_B, &default_filter)
        .unwrap();
    assert!(b_views_a_issues.is_empty(), "tenant B must not see tenant A's issues");

    let a_views_b_issues = storage
        .get_issues_for_tenant(&crawl_b, TENANT_A, &default_filter)
        .unwrap();
    assert!(a_views_b_issues.is_empty(), "tenant A must not see tenant B's issues");
}

#[test]
fn crawl_meta_reports_only_its_own_crawls_data() {
    let storage = new_in_memory_backend().unwrap();
    let (crawl_a, crawl_b) = seed_tenant_crawls(storage.as_ref(), false);

    // Finish with distinct, tenant-specific counters.
    storage.finish_crawl(&crawl_a, 7, 3).unwrap();
    storage.finish_crawl(&crawl_b, 2, 11).unwrap();

    let meta_a = storage.get_crawl_meta(&crawl_a).unwrap();
    assert_eq!(meta_a.id, crawl_a);
    assert!(meta_a.target_url.contains("a.example.com"));
    assert_eq!(meta_a.pages_crawled, 7, "crawl A meta must not include crawl B's pages");
    assert_eq!(meta_a.total_issues, 3, "crawl A meta must not include crawl B's issues");

    let meta_b = storage.get_crawl_meta(&crawl_b).unwrap();
    assert_eq!(meta_b.id, crawl_b);
    assert!(meta_b.target_url.contains("b.example.com"));
    assert_eq!(meta_b.pages_crawled, 2, "crawl B meta must not include crawl A's pages");
    assert_eq!(meta_b.total_issues, 11, "crawl B meta must not include crawl A's issues");
}

#[test]
fn purge_for_tenant_a_spares_tenant_b_data() {
    let storage = new_in_memory_backend().unwrap();
    let (crawl_a, crawl_b) = seed_tenant_crawls(storage.as_ref(), true);

    // max_age_days = 0 purges every crawl containing tenant A's pages.
    let purged = storage.purge_old_crawls_for_tenant(0, TENANT_A).unwrap();
    assert_eq!(purged, 1, "exactly tenant A's crawl should be purged");

    // Tenant A's data is gone (crawl row, pages, issues).
    assert!(storage.get_crawl_meta(&crawl_a).is_err(), "tenant A's crawl row must be deleted");
    assert!(storage
        .get_pages_for_tenant(&crawl_a, TENANT_A, 100)
        .unwrap()
        .is_empty());
    assert!(storage
        .get_issues_for_tenant(&crawl_a, TENANT_A, &IssueFilter::default())
        .unwrap()
        .is_empty());

    // Tenant B's data survives untouched.
    let meta_b = storage.get_crawl_meta(&crawl_b).unwrap();
    assert_eq!(meta_b.id, crawl_b);
    let pages_b = storage.get_pages_for_tenant(&crawl_b, TENANT_B, 100).unwrap();
    assert_eq!(pages_b.len(), 1);
    assert_eq!(pages_b[0].id, "pb1");
    let issues_b = storage
        .get_issues_for_tenant(&crawl_b, TENANT_B, &IssueFilter::default())
        .unwrap();
    assert_eq!(issues_b.len(), 1);
    assert_eq!(issues_b[0].id, "ib1");
}

#[test]
fn cross_tenant_read_is_blocked() {
    let storage = new_in_memory_backend().unwrap();
    let (crawl_a, crawl_b) = seed_tenant_crawls(storage.as_ref(), true);

    // Tenant A's data is really there: a direct get_page by URL finds it.
    let page = storage.get_page(&crawl_a, "https://a.example.com/").unwrap();
    assert!(page.is_some());
    assert_eq!(page.unwrap().id, "pa1");

    // ...but tenant B cannot list it through tenant-scoped queries.
    let b_views_a_pages = storage.get_pages_for_tenant(&crawl_a, TENANT_B, 100).unwrap();
    assert!(b_views_a_pages.is_empty());
    let b_views_a_issues = storage
        .get_issues_for_tenant(&crawl_a, TENANT_B, &IssueFilter::default())
        .unwrap();
    assert!(b_views_a_issues.is_empty());

    // Tenant A's URL does not exist inside tenant B's crawl.
    let leaked = storage.get_page(&crawl_b, "https://a.example.com/").unwrap();
    assert!(leaked.is_none(), "tenant A's page must not appear in tenant B's crawl");
}

#[test]
fn null_tenant_pages_are_visible_to_all_tenants() {
    let storage = new_in_memory_backend().unwrap();
    let crawl = storage.start_crawl("https://shared.example.com", None).unwrap();

    storage
        .insert_page(&crawl, &make_page_for_tenant("p-null", "https://shared.example.com/x", None))
        .unwrap();
    storage
        .insert_page(&crawl, &make_page_for_tenant("p-a", "https://shared.example.com/a", Some(TENANT_A)))
        .unwrap();

    // NULL-tenant pages are shared/global: every tenant sees them.
    for tenant in [TENANT_A, TENANT_B, "tenant-c"] {
        let pages = storage.get_pages_for_tenant(&crawl, tenant, 100).unwrap();
        assert!(
            pages.iter().any(|p| p.id == "p-null"),
            "NULL-tenant page must be visible to {tenant}"
        );
    }

    // ...while private pages remain scoped.
    let pages_b = storage.get_pages_for_tenant(&crawl, TENANT_B, 100).unwrap();
    assert_eq!(pages_b.len(), 1);
    assert_eq!(pages_b[0].id, "p-null");
    let pages_a = storage.get_pages_for_tenant(&crawl, TENANT_A, 100).unwrap();
    assert_eq!(pages_a.len(), 2);
}

#[test]
fn tenant_with_no_data_gets_empty_results_not_errors() {
    let storage = new_in_memory_backend().unwrap();
    let (crawl_a, _crawl_b) = seed_tenant_crawls(storage.as_ref(), true);

    // A tenant that exists but owns no data in these crawls...
    let empty_pages_a = storage.get_pages_for_tenant(&crawl_a, "tenant-c", 100).unwrap();
    assert!(empty_pages_a.is_empty(), "expected empty page list, got an error or data");

    let empty_issues_a = storage
        .get_issues_for_tenant(&crawl_a, "tenant-c", &IssueFilter::default())
        .unwrap();
    assert!(empty_issues_a.is_empty(), "expected empty issue list, got an error or data");

    // Filtering must not blow up for an unknown tenant either.
    let filtered = storage
        .get_issues_for_tenant(
            &crawl_a,
            "tenant-c",
            &IssueFilter {
                severity: Some(Severity::Critical),
                ..IssueFilter::default()
            },
        )
        .unwrap();
    assert!(filtered.is_empty());

    // And a purge for that tenant is a harmless no-op.
    let purged = storage.purge_old_crawls_for_tenant(0, "tenant-c").unwrap();
    assert_eq!(purged, 0, "purging a tenant with no data must delete nothing");

    // Tenant A's data is unaffected by all of the above.
    assert_eq!(storage.get_pages_for_tenant(&crawl_a, TENANT_A, 100).unwrap().len(), 1);
}

#[test]
fn purge_for_tenant_with_no_data_is_a_no_op() {
    let storage = new_in_memory_backend().unwrap();
    let (crawl_a, crawl_b) = seed_tenant_crawls(storage.as_ref(), true);

    let purged = storage.purge_old_crawls_for_tenant(0, "tenant-does-not-exist").unwrap();
    assert_eq!(purged, 0);

    // Both tenants' crawls, pages, and issues are untouched.
    assert!(storage.get_crawl_meta(&crawl_a).is_ok());
    assert!(storage.get_crawl_meta(&crawl_b).is_ok());
    assert_eq!(storage.get_pages_for_tenant(&crawl_a, TENANT_A, 100).unwrap().len(), 1);
    assert_eq!(storage.get_pages_for_tenant(&crawl_b, TENANT_B, 100).unwrap().len(), 1);
    assert_eq!(
        storage
            .get_issues_for_tenant(&crawl_a, TENANT_A, &IssueFilter::default())
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        storage
            .get_issues_for_tenant(&crawl_b, TENANT_B, &IssueFilter::default())
            .unwrap()
            .len(),
        1
    );
}
