use super::counters::{bump, bump_by, CrawlCounters};
use super::dedup::ContentHashes;
use super::fetch::{FetchOutcome, FetchedPage, Freshness};
use super::CrawlEngineConfig;
use crate::analyzers::AnalyzerRegistry;
use crate::encryption::EncryptionManager;
use crate::queue::{Priority, QueueEntry};
use crate::queue_trait::Queue;
use crate::storage::{Issue, PageData};
use crate::storage_trait::StorageBackend;
use crate::{Metrics, RedirectHop, ResourceMonitor};
use chrono::Utc;
use dashmap::DashSet;
use std::sync::Arc;
use std::time::Duration;
use url::Url;

use super::OnPageCrawled;

/// Shared state for one crawl execution: counters, dedup sets, and the
/// collaborators needed to analyze and persist each fetched page.
///
/// Each stage of the per-page pipeline is a focused method
/// (dedup → render → analyze → store → link extraction), keeping every
/// unit small and independently testable.
pub(crate) struct CrawlRun<'a> {
    pub(crate) counters: CrawlCounters,
    pub(crate) visited: DashSet<String>,
    pub(crate) content_hashes: ContentHashes,
    pub(crate) analyzer_registry: &'a AnalyzerRegistry,
    pub(crate) cfg: &'a CrawlEngineConfig,
    pub(crate) storage: Arc<dyn StorageBackend>,
    pub(crate) crawl_id: String,
    pub(crate) seed_domain: String,
    pub(crate) on_page: Option<OnPageCrawled>,
    pub(crate) metrics: Metrics,
    pub(crate) resource_monitor: ResourceMonitor,
    pub(crate) queue: Arc<dyn Queue>,
    pub(crate) plugins: Vec<crate::plugin_runtime::CrawlPlugin>,
}

impl CrawlRun<'_> {
    /// Route a completed fetch through dedup, analysis, storage, and discovery.
    pub(crate) async fn process(&self, fetched: &FetchedPage) {
        match &fetched.outcome {
            FetchOutcome::NotModified { page_id } => {
                self.record_not_modified(page_id).await;
            }
            FetchOutcome::Failed(err) => self.record_failure(&fetched.entry.url, err),
            FetchOutcome::Fetched { result, freshness } => {
                self.process_fetched(fetched, result, *freshness).await;
            }
        }
    }

    /// Handle a 304: count it and refresh the stored page's access timestamp.
    async fn record_not_modified(&self, page_id: &Option<String>) {
        bump(&self.counters.pages_unchanged);
        self.metrics.record_page_unchanged();
        let Some(id) = page_id.clone() else {
            return;
        };
        let storage = Arc::clone(&self.storage);
        let result =
            tokio::task::spawn_blocking(move || storage.update_page_fetched_at(&id, Utc::now()))
                .await;
        match result {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                tracing::warn!(error = %e, page_id = ?page_id, "Failed to refresh fetched_at")
            }
            Err(e) => {
                tracing::warn!(error = %e, page_id = ?page_id, "Blocking task join failed")
            }
        }
    }

    /// Handle a failed fetch.
    fn record_failure(&self, url: &Url, err: &crate::CrawlError) {
        tracing::warn!("Failed to fetch {}: {}", url, err);
        self.metrics.record_page_failure();
    }

    /// Process a successfully fetched page.
    async fn process_fetched(
        &self,
        fetched: &FetchedPage,
        result: &crate::FetchResult,
        freshness: Freshness,
    ) {
        if !self.content_hashes.insert(&result.body) {
            tracing::debug!("Skipping duplicate content: {}", fetched.entry.url);
            bump(&self.counters.skipped_duplicate);
            self.metrics.record_page_skipped_duplicate();
            return;
        }

        bump(&self.counters.pages_crawled);
        match freshness {
            Freshness::New => bump(&self.counters.pages_new),
            Freshness::Modified => bump(&self.counters.pages_modified),
            Freshness::Unconditional => {}
        }

        let mut body_text = result.body.clone();
        let mut parsed = {
            let _parse_span = tracing::info_span!(
                "parse",
                url = %fetched.entry.url,
            );
            let _parse_enter = _parse_span.enter();
            crate::HtmlParser::parse(&body_text, &fetched.entry.url)
        };

        let mut rendered_page: Option<crate::playwright::RenderedPage> = None;
        self.render_js_if_needed(
            &fetched.entry.url,
            &mut body_text,
            &mut parsed,
            &mut rendered_page,
        )
        .await;

        let (findings, analysis_time) = {
            let _analyze_span = tracing::info_span!(
                "analyze",
                url = %fetched.entry.url,
                analyzer_count = self.analyzer_registry.len(),
            );
            let _analyze_enter = _analyze_span.enter();
            self.analyze(&parsed, &body_text, result, fetched, rendered_page.as_ref())
        };
        bump_by(&self.counters.issues_found, findings.len());

        let page_id = uuid::Uuid::new_v4().to_string();
        let page_data =
            self.build_page_data(&page_id, &fetched.entry.url, result, &parsed, &body_text);
        self.store(&page_data, &findings).await;

        self.metrics.record_page_success(
            result.body.len() as u64,
            fetched.fetch_time.as_micros() as u64,
            analysis_time.as_micros() as u64,
            0, // storage_time not tracked per page
            findings.len() as u64,
        );
        self.resource_monitor.record_page();

        if let Some(cb) = &self.on_page {
            cb(fetched.entry.url.as_ref(), &page_id, findings.len());
        }

        self.extract_links(&parsed, fetched.entry.depth);
    }

    /// Re-render with JavaScript when the decision engine requires it.
    async fn render_js_if_needed(
        &self,
        url: &Url,
        body_text: &mut String,
        parsed: &mut crate::ParsedPage,
        rendered_page: &mut Option<crate::playwright::RenderedPage>,
    ) {
        if !self.cfg.enable_js_rendering {
            return;
        }
        let decision =
            crate::JsRenderDecisionEngine::new().should_render_js(url.as_ref(), Some(body_text));
        let crate::JsRenderDecision::Render { reason } = decision else {
            return;
        };
        tracing::info!("JS render decision for {}: {}", url, reason);

        let Some(renderer) = self.cfg.js_renderer.as_ref() else {
            return;
        };
        if !renderer.is_available() {
            tracing::warn!("JS renderer not available, using static HTML: {}", url);
            return;
        }
        match tokio::time::timeout(Duration::from_secs(30), renderer.render_rich(url.as_str()))
            .await
        {
            Ok(Ok(page)) => {
                *body_text = page.html.clone();
                *parsed = crate::HtmlParser::parse(body_text, url);
                *rendered_page = Some(page);
            }
            Ok(Err(e)) => tracing::warn!("JS render failed for {}: {}", url, e),
            Err(_) => tracing::warn!("JS render timed out for {}", url),
        }
    }

    /// Run all analyzers against the parsed page.
    #[must_use]
    fn analyze(
        &self,
        parsed: &crate::ParsedPage,
        body_text: &str,
        result: &crate::FetchResult,
        fetched: &FetchedPage,
        rendered: Option<&crate::playwright::RenderedPage>,
    ) -> (Vec<crate::Finding>, Duration) {
        let headers_vec: Vec<(String, String)> = result.headers.clone();
        let empty_chain: Vec<RedirectHop> = Vec::new();
        let robots_ref = if fetched.robots_raw.is_empty() {
            None
        } else {
            Some(fetched.robots_raw.as_str())
        };
        let ctx = crate::analyzers::AnalysisContext {
            page: parsed,
            body: Some(body_text),
            status_code: Some(result.status_code),
            headers: &headers_vec,
            response_time: Some(fetched.fetch_time),
            redirect_chain: &empty_chain,
            robots_txt: robots_ref,
            body_size: Some(result.body_size),
            compressed_size: result
                .headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("content-length"))
                .and_then(|(_, v)| v.parse().ok()),
            server: result
                .headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("server"))
                .map(|(_, v)| v.as_str()),
            content_type: result
                .headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
                .map(|(_, v)| v.as_str()),
            rendered,
        };
        let analysis_start = std::time::Instant::now();
        let mut findings = self.analyzer_registry.analyze(&ctx);

        // Crawl plugins run after the built-ins, with the B4 structured
        // context; failures degrade to no findings (never abort a crawl).
        if !self.plugins.is_empty() {
            let url_str = fetched.entry.url.to_string();
            let context_json = crate::plugin_runtime::build_context_json(
                &url_str,
                Some(result.status_code),
                &headers_vec,
                fetched.fetch_time.as_millis().try_into().ok(),
                Some(parsed),
            );
            for plugin in &self.plugins {
                findings.extend(plugin.analyze(body_text, &url_str, Some(&context_json)));
            }
        }

        (findings, analysis_start.elapsed())
    }

    /// Assemble the storage record for a crawled page.
    #[must_use]
    fn build_page_data(
        &self,
        page_id: &str,
        url: &Url,
        result: &crate::FetchResult,
        parsed: &crate::ParsedPage,
        body_text: &str,
    ) -> PageData {
        // Run custom extraction rules if enabled.
        let extractions_json = if self.cfg.crawl_config.extraction.enabled
            && !self.cfg.crawl_config.extraction.rules.is_empty()
        {
            let extraction_results =
                crate::extraction::extract_page(body_text, &self.cfg.crawl_config.extraction.rules);
            let pairs: Vec<(String, Vec<String>)> = extraction_results
                .into_iter()
                .map(|r| (r.rule_name, r.values))
                .collect();
            serde_json::to_string(&pairs).ok()
        } else {
            None
        };

        let mut page_data = PageData {
            id: page_id.to_string(),
            url: url.clone(),
            final_url: result.final_url.clone(),
            status_code: result.status_code,
            title: parsed.meta.title.clone(),
            description: parsed.meta.description.clone(),
            canonical_url: parsed.meta.canonical.clone(),
            word_count: Some(parsed.word_count),
            load_time_ms: Some(result.response_time.as_millis() as u64),
            body_size: Some(result.body.len()),
            fetched_at: Utc::now(),
            links: parsed
                .links
                .iter()
                .filter_map(|l| Url::parse(&l.href).ok())
                .collect(),
            tenant_id: self.cfg.tenant_id.clone(),
            etag: result.etag.clone(),
            last_modified: result.last_modified.clone(),
            cwv_lcp: None,
            cwv_cls: None,
            cwv_inp: None,
            has_structured_data: Some(!parsed.structured_data.is_empty()),
            schema_types: Some(
                parsed
                    .structured_data
                    .iter()
                    .filter_map(|sd| sd.r#type.as_deref())
                    .collect::<Vec<_>>()
                    .join(","),
            ),
            viewport_ok: Some(
                parsed
                    .meta
                    .viewport
                    .as_deref()
                    .is_some_and(|v| v.contains("device-width")),
            ),
            has_csp: Some(
                result
                    .headers
                    .iter()
                    .any(|(k, _)| k.eq_ignore_ascii_case("content-security-policy")),
            ),
            has_hsts: Some(
                result
                    .headers
                    .iter()
                    .any(|(k, _)| k.eq_ignore_ascii_case("strict-transport-security")),
            ),
            images_total: Some(parsed.images.len()),
            images_missing_alt: Some(parsed.images.iter().filter(|i| !i.has_alt).count()),
            h1_count: Some(parsed.headings.iter().filter(|h| h.level == 1).count()),
            heading_count: Some(parsed.headings.len()),
            extractions: extractions_json,
        };
        if let Some(encryption) = self.cfg.encryption.as_ref() {
            if encryption.is_enabled() {
                page_data.title = encrypt_field(encryption, page_data.title.take());
                page_data.description = encrypt_field(encryption, page_data.description.take());
            }
        }
        page_data
    }

    /// Persist the page and its findings (batched in one transaction) on the
    /// blocking pool so SQLite writes never stall the async runtime.
    async fn store(&self, page_data: &PageData, findings: &[crate::Finding]) {
        let issues: Vec<Issue> = findings
            .iter()
            .map(|finding| Issue {
                id: uuid::Uuid::new_v4().to_string(),
                page_id: page_data.id.clone(),
                category: finding.category.clone(),
                severity: finding.severity,
                code: finding.code.clone(),
                title: finding.title.clone(),
                description: finding.description.clone(),
                element: None,
                recommendation: finding.recommendation.clone(),
                tenant_id: self.cfg.tenant_id.clone(),
            })
            .collect();

        let storage = Arc::clone(&self.storage);
        let crawl_id = self.crawl_id.clone();
        let page = page_data.clone();
        let result = tokio::task::spawn_blocking(move || {
            let page_stored = storage
                .insert_page(&crawl_id, &page)
                .map(|()| page.url.clone());
            let issues_stored = storage.insert_issues_batch(&issues);
            (page_stored, issues_stored)
        })
        .await;

        match result {
            Ok((Ok(_), Ok(()))) => bump(&self.counters.pages_stored),
            Ok((page_res, issue_res)) => {
                if let Err(e) = page_res {
                    tracing::warn!("Failed to store page {}: {}", page_data.url, e);
                }
                if let Err(e) = issue_res {
                    tracing::warn!("Failed to store issues: {}", e);
                }
            }
            Err(e) => tracing::warn!(error = %e, "Blocking store task join failed"),
        }
    }

    /// Apply scope/pattern filters and enqueue newly discovered links.
    fn extract_links(&self, parsed: &crate::ParsedPage, depth: usize) {
        let max_depth = self.cfg.crawl_config.max_depth;
        for link in &parsed.links {
            let link_url = match Url::parse(&link.href) {
                Ok(u) => u,
                Err(_) => continue,
            };
            if self.visited.contains(&link_url.to_string()) {
                continue;
            }
            let is_internal = link_url.host_str() == Some(self.seed_domain.as_str());

            if !is_internal && !self.cfg.allow_external {
                bump(&self.counters.skipped_external);
                continue;
            }
            if !self.cfg.include_patterns.is_empty()
                && !self
                    .cfg
                    .include_patterns
                    .iter()
                    .any(|p| link.href.contains(p.as_str()))
            {
                continue;
            }
            if self
                .cfg
                .exclude_patterns
                .iter()
                .any(|p| link.href.contains(p.as_str()))
            {
                continue;
            }
            if let Some(max) = max_depth {
                if depth + 1 > max {
                    continue;
                }
            }

            let priority = if is_internal {
                Priority::NORMAL
            } else {
                Priority::LOW
            };
            let entry = QueueEntry {
                url: link_url.clone(),
                canonical_url: link_url,
                depth: depth + 1,
                priority,
                discovered_at: Utc::now(),
                referrer: None,
            };
            self.queue.push(entry).unwrap();
        }
    }
}

/// Encrypt one field value for storage, hex-encoded and `enc:`-prefixed.
/// Falls back to the plaintext value if encryption fails.
pub(crate) fn encrypt_field(encryption: &EncryptionManager, value: Option<String>) -> Option<String> {
    value.map(|v| {
        encryption
            .encrypt(v.as_bytes())
            .map(|enc| {
                format!(
                    "enc:{}",
                    enc.iter().map(|b| format!("{b:02x}")).collect::<String>()
                )
            })
            .unwrap_or(v)
    })
}
