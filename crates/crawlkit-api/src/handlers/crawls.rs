use std::sync::Arc;

use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use uuid::Uuid;

use crate::auth;
use crate::types::*;
use crawlkit_engine::CrawlConfig;

pub async fn start_crawl(
    State(state): State<AppState>,
    Extension(claims): Extension<auth::Claims>,
    Json(req): Json<CreateCrawlRequest>,
) -> Result<(StatusCode, Json<CrawlResponse>), ApiError> {
    validate_url(&req.start_url)?;
    validate_max_pages(req.max_pages)?;
    validate_concurrency(req.concurrency)?;
    validate_delay(req.request_delay_ms)?;

    let start_url = url::Url::parse(&req.start_url)
        .map_err(|e| ApiError::BadRequest(format!("Invalid URL: {e}")))?;

    let crawl_id = Uuid::new_v4().to_string();
    let tenant_id = extract_tenant(&claims).to_string();

    let config_json = serde_json::to_string(&serde_json::json!({
        "start_url": start_url,
        "max_pages": req.max_pages,
        "request_delay_ms": req.request_delay_ms,
        "concurrency": req.concurrency,
    }))
    .unwrap_or_default();

    state
        .storage
        .start_crawl(start_url.as_ref(), Some(&config_json))
        .map_err(|e| ApiError::Internal(format!("Failed to start crawl: {e}")))?;

    let result = CrawlResult {
        crawl_id: crawl_id.clone(),
        tenant_id,
        start_url: start_url.to_string(),
        status: "running".to_string(),
        pages_crawled: 0,
        issues_found: 0,
        created_at: Utc::now(),
        completed_at: None,
    };

    state.crawl_results.insert(crawl_id.clone(), result);

    state.metrics.crawls_total.inc();
    state.metrics.active_crawls.inc();

    // Spawn crawl task in background
    let crawl_id_clone = crawl_id.clone();
    let config = CrawlConfig {
        start_url,
        max_pages: req.max_pages,
        request_delay: std::time::Duration::from_millis(req.request_delay_ms),
        concurrency: req.concurrency,
        ..Default::default()
    };

    tokio::spawn(async move {
        run_crawl_task(state, crawl_id_clone, config).await;
    });

    Ok((
        StatusCode::ACCEPTED,
        Json(CrawlResponse {
            crawl_id,
            status: "running".to_string(),
            message: "Crawl started successfully".to_string(),
        }),
    ))
}

pub async fn get_crawl_status(
    State(state): State<AppState>,
    Extension(claims): Extension<auth::Claims>,
    Path(crawl_id): Path<String>,
) -> Result<Json<CrawlResult>, ApiError> {
    let entry = state
        .crawl_results
        .get(&crawl_id)
        .ok_or_else(|| ApiError::NotFound(format!("Crawl {crawl_id} not found")))?;

    if entry.tenant_id != extract_tenant(&claims) && !is_admin(&claims) {
        return Err(ApiError::NotFound(format!("Crawl {crawl_id} not found")));
    }

    Ok(Json(entry.value().clone()))
}

pub async fn get_crawl_stats(
    State(state): State<AppState>,
    Extension(claims): Extension<auth::Claims>,
    Path(crawl_id): Path<String>,
) -> Result<Json<CrawlStatsResponse>, ApiError> {
    if let Some(entry) = state.crawl_results.get(&crawl_id) {
        if entry.tenant_id != extract_tenant(&claims) && !is_admin(&claims) {
            return Err(ApiError::NotFound(format!("Crawl {crawl_id} not found")));
        }
    }

    let stats = state
        .storage
        .get_stats(&crawl_id)
        .map_err(|e| ApiError::Internal(format!("Failed to get stats: {e}")))?;

    Ok(Json(CrawlStatsResponse {
        crawl_id,
        total_pages: stats.total_pages,
        total_issues: stats.total_issues,
        issues_by_severity: stats.issues_by_severity,
        issues_by_category: stats.issues_by_category,
        avg_response_time_ms: stats.avg_response_time_ms,
    }))
}

pub async fn get_crawl_findings(
    State(state): State<AppState>,
    Extension(claims): Extension<auth::Claims>,
    Path(crawl_id): Path<String>,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    if let Some(entry) = state.crawl_results.get(&crawl_id) {
        if entry.tenant_id != extract_tenant(&claims) && !is_admin(&claims) {
            return Err(ApiError::NotFound(format!("Crawl {crawl_id} not found")));
        }
    }

    let filter = crawlkit_engine::storage::IssueFilter::default();
    let issues = state
        .storage
        .get_issues(&crawl_id, &filter)
        .map_err(|e| ApiError::Internal(format!("Failed to get findings: {e}")))?;

    let findings: Vec<serde_json::Value> = issues
        .iter()
        .map(|issue| {
            serde_json::json!({
                "id": issue.id,
                "page_id": issue.page_id,
                "category": issue.category.as_str(),
                "severity": issue.severity.as_str(),
                "code": issue.code,
                "title": issue.title,
                "description": issue.description,
                "element": issue.element,
                "recommendation": issue.recommendation,
            })
        })
        .collect();

    Ok(Json(findings))
}

pub async fn list_crawls(
    State(state): State<AppState>,
    Extension(claims): Extension<auth::Claims>,
) -> Json<Vec<CrawlResult>> {
    let tenant = extract_tenant(&claims);
    let admin = is_admin(&claims);
    let results: Vec<CrawlResult> = state
        .crawl_results
        .iter()
        .filter(|entry| admin || entry.value().tenant_id == tenant)
        .map(|entry| entry.value().clone())
        .collect();

    Json(results)
}

#[derive(serde::Serialize)]
#[allow(dead_code)]
pub struct BacklinksResponse {
    crawl_id: String,
    total_internal_links: usize,
    total_external_links: usize,
    total_referring_domains: usize,
    orphan_pages: Vec<String>,
    top_pages_by_pagerank: Vec<serde_json::Value>,
}

pub async fn get_crawl_backlinks(
    State(state): State<AppState>,
    Extension(claims): Extension<auth::Claims>,
    Path(crawl_id): Path<String>,
) -> Result<Json<BacklinksResponse>, ApiError> {
    if let Some(entry) = state.crawl_results.get(&crawl_id) {
        if entry.tenant_id != extract_tenant(&claims) && !is_admin(&claims) {
            return Err(ApiError::NotFound(format!("Crawl {crawl_id} not found")));
        }
    }

    let link_pairs = state
        .storage
        .get_links_for_crawl(&crawl_id)
        .map_err(|e| ApiError::Internal(format!("Failed to get links: {e}")))?;

    let external_links = state
        .storage
        .get_external_links(&crawl_id)
        .map_err(|e| ApiError::Internal(format!("Failed to get external links: {e}")))?;

    let mut analyzer = crawlkit_engine::BacklinkAnalyzer::new();
    analyzer.load_from_crawl_data(&link_pairs);
    for (source, target) in &external_links {
        analyzer.add_backlink(crawlkit_engine::Backlink {
            source_url: source.clone(),
            target_url: target.clone(),
            anchor_text: String::new(),
            is_followed: true,
            is_internal: false,
        });
    }

    let _pagerank = analyzer.compute_pagerank(0.85, 20);
    let summary = analyzer.summarize();

    let top_pages: Vec<serde_json::Value> = summary
        .pages
        .iter()
        .take(20)
        .map(|p| {
            serde_json::json!({
                "url": p.url,
                "pagerank": p.pagerank,
                "inbound_links": p.inbound_links,
                "outbound_links": p.outbound_links,
                "referring_domains": p.referring_domains,
            })
        })
        .collect();

    Ok(Json(BacklinksResponse {
        crawl_id,
        total_internal_links: summary.total_internal_links,
        total_external_links: summary.total_external_links,
        total_referring_domains: summary.total_referring_domains,
        orphan_pages: summary.orphan_pages,
        top_pages_by_pagerank: top_pages,
    }))
}

pub async fn run_crawl_task(state: AppState, crawl_id: String, config: crawlkit_engine::CrawlConfig) {
    use crawlkit_engine::crawl_engine::{CrawlEngine, CrawlEngineConfig};

    let engine_config = CrawlEngineConfig {
        crawl_config: config.clone(),
        ..Default::default()
    };

    let engine = CrawlEngine::new_shared(engine_config, state.storage.clone());

    let state_clone = state.clone();
    let result = engine
        .run_with_callback(
            config.start_url.as_ref(),
            Some(Arc::new(move |_url, _page_id, _findings| {
                state_clone.metrics.pages_crawled_total.inc();
            })),
        )
        .await;

    match result {
        Ok(output) => {
            let _ =
                engine
                    .storage()
                    .finish_crawl(&crawl_id, output.pages_crawled, output.issues_found);

            let tenant_id = state
                .crawl_results
                .get(&crawl_id)
                .map(|r| r.tenant_id.clone())
                .unwrap_or_default();

            if let Some(mut result) = state.crawl_results.get_mut(&crawl_id) {
                result.status = "completed".to_string();
                result.pages_crawled = output.pages_crawled;
                result.issues_found = output.issues_found;
                result.completed_at = Some(Utc::now());
            }

            state.metrics.active_crawls.dec();
            state
                .metrics
                .pages_crawled_total
                .inc_by(output.pages_crawled as u64);
            state
                .metrics
                .issues_total
                .inc_by(output.issues_found as u64);
            state
                .metrics
                .fetch_duration_seconds
                .observe(output.elapsed.as_secs_f64());
            state
                .metrics
                .analysis_duration_seconds
                .observe(output.elapsed.as_secs_f64());
            super::webhooks::fire_webhooks(
                &state,
                "crawl.completed",
                &crawl_id,
                &tenant_id,
                output.pages_crawled,
                output.issues_found,
            );
            tracing::info!(
                "Crawl {crawl_id} completed: {} pages, {} issues",
                output.pages_crawled,
                output.issues_found
            );
        }
        Err(e) => {
            tracing::error!("Crawl {crawl_id} failed: {e}");
            state.metrics.errors_total.inc();

            let tenant_id = state
                .crawl_results
                .get(&crawl_id)
                .map(|r| r.tenant_id.clone())
                .unwrap_or_default();

            if let Some(mut result) = state.crawl_results.get_mut(&crawl_id) {
                result.status = "failed".to_string();
                result.completed_at = Some(Utc::now());
            }
            state.metrics.active_crawls.dec();
            super::webhooks::fire_webhooks(&state, "crawl.failed", &crawl_id, &tenant_id, 0, 0);
        }
    }
}
