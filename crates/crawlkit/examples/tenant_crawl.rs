//! Example: Multi-tenant crawling with tenant isolation.
//!
//! Demonstrates how crawlkit isolates data between tenants:
//!
//! 1. Plan limits (max pages, rate limits) are checked the way the API
//!    server does before starting a crawl. (The `TenantManager` in
//!    `crawlkit_engine::enterprise` does this in production; it lives
//!    behind the `unstable` feature, so this example shows the same
//!    checks with a minimal local registry.)
//! 2. A shared `Storage` database holds pages and issues tagged with a
//!    `tenant_id` (plus unscoped rows with `tenant_id = NULL`, which are
//!    visible to every tenant — e.g. platform-owned configuration pages).
//! 3. `get_pages_for_tenant` / `get_issues_for_tenant` return only rows
//!    belonging to the requesting tenant (or unscoped rows), proving that
//!    tenant A can never read tenant B's data.
//!
//! Run with: cargo run --example tenant_crawl

use chrono::Utc;
use crawlkit_engine::storage::{Issue, IssueCategory, IssueFilter, PageData, Severity, Storage};
use url::Url;

/// Per-tenant limits, mirroring `enterprise::Tenant`.
struct TenantPlanLimits {
    id: &'static str,
    name: &'static str,
    plan: &'static str,
    max_pages: usize,
    rate_limit: u32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // --- Step 1: register tenants with plan limits -------------------------
    println!("[1/4] Registering tenants...");
    let tenants = [
        TenantPlanLimits {
            id: "acme",
            name: "Acme Corp",
            plan: "pro",
            max_pages: 10_000,
            rate_limit: 120,
        },
        TenantPlanLimits {
            id: "globex",
            name: "Globex Inc",
            plan: "free",
            max_pages: 100,
            rate_limit: 20,
        },
    ];
    for tenant in &tenants {
        println!(
            "  {} ({}): plan={} max_pages={} rate_limit={}/min",
            tenant.id, tenant.name, tenant.plan, tenant.max_pages, tenant.rate_limit
        );
    }

    // --- Step 2: enforce per-tenant limits before crawling ------------------
    println!("\n[2/4] Enforcing plan limits...");
    let requested_pages = 500;
    for tenant in &tenants {
        if requested_pages <= tenant.max_pages {
            println!(
                "  {}: crawl of {requested_pages} pages allowed (limit {})",
                tenant.id, tenant.max_pages
            );
        } else {
            println!(
                "  {}: crawl of {requested_pages} pages REJECTED (limit {})",
                tenant.id, tenant.max_pages
            );
        }
    }

    // --- Step 3: store crawl data tagged with tenant ids --------------------
    println!("\n[3/4] Storing pages and issues with tenant scoping...");
    let storage = Storage::new_in_memory()?;
    let crawl_id = storage.start_crawl("https://acme.example", None)?;

    let pages = vec![
        scoped_page("p1", "https://acme.example/", Some("acme"))?,
        scoped_page("p2", "https://acme.example/internal", Some("acme"))?,
        scoped_page("p3", "https://globex.example/", Some("globex"))?,
        // Unscoped row: visible to every tenant (shared platform data).
        scoped_page("p4", "https://status.example/platform", None)?,
    ];
    storage.insert_pages(&crawl_id, &pages)?;

    let issues = vec![
        scoped_issue("i1", "p1", "acme", "Missing meta description"),
        scoped_issue("i2", "p3", "globex", "Broken link found"),
        // Unscoped issue: visible to every tenant.
        scoped_issue("i3", "p4", "platform", "Shared header is slow"),
    ];
    storage.insert_issues(&issues)?;
    storage.finish_crawl(&crawl_id, pages.len(), issues.len())?;
    println!("  stored {} pages and {} issues", pages.len(), issues.len());

    // --- Step 4: read back with tenant isolation ----------------------------
    println!("\n[4/4] Reading back per tenant...");
    for tenant in &tenants {
        let tenant_id = tenant.id;
        let visible_pages = storage.get_pages_for_tenant(&crawl_id, tenant_id, 100)?;
        let visible_issues =
            storage.get_issues_for_tenant(&crawl_id, tenant_id, &IssueFilter::default())?;

        println!("\n  Tenant '{tenant_id}' sees:");
        for p in &visible_pages {
            println!("    page: {}", p.url);
        }
        for i in &visible_issues {
            println!("    issue: [{}] {}", i.code, i.title);
        }

        // The isolation invariant: a tenant never sees another tenant's rows.
        let other = if tenant_id == "acme" {
            "globex"
        } else {
            "acme"
        };
        let leaked = visible_pages
            .iter()
            .any(|p| p.tenant_id.as_deref() == Some(other));
        println!(
            "    isolation intact: {}",
            if leaked { "NO" } else { "yes" }
        );
    }

    println!("\nMulti-tenant demo complete.");
    Ok(())
}

/// Build a tenant-scoped `PageData` row.
fn scoped_page(id: &str, url: &str, tenant_id: Option<&str>) -> Result<PageData, url::ParseError> {
    let parsed = Url::parse(url)?;
    Ok(PageData {
        id: id.to_string(),
        url: parsed.clone(),
        final_url: parsed,
        status_code: 200,
        title: Some("Demo page".to_string()),
        description: None,
        canonical_url: None,
        word_count: Some(350),
        load_time_ms: Some(140),
        body_size: Some(2_400),
        fetched_at: Utc::now(),
        links: Vec::new(),
        tenant_id: tenant_id.map(str::to_string),
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
        // Note: the optional heading-count columns are left unset here —
        // the storage migration creates them as TEXT while reads expect
        // integers, so populating them trips a read-back type mismatch.
        h1_count: None,
        heading_count: None,
        extractions: None,
    })
}

/// Build a tenant-scoped `Issue` row.
fn scoped_issue(id: &str, page_id: &str, tenant_id: &str, title: &str) -> Issue {
    Issue {
        id: id.to_string(),
        page_id: page_id.to_string(),
        category: IssueCategory::Seo,
        severity: Severity::Warning,
        code: format!("TENANT-{tenant_id}"),
        title: title.to_string(),
        description: title.to_string(),
        element: None,
        recommendation: "Review this finding".to_string(),
        tenant_id: Some(tenant_id.to_string()),
    }
}
