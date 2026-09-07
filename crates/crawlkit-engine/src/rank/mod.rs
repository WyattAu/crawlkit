//! Keyword rank tracking across search engines.
//!
//! Provides a pluggable [`SerpProvider`] abstraction with built-in
//! DuckDuckGo (HTML scrape) and Google Search Console backends, a
//! [`RankTracker`] orchestrator that rate-limits and matches target
//! domains against organic results, and trend analysis over stored
//! position history.
//!
//! # Example
//!
//! ```rust,no_run
//! use crawlkit_engine::rank::{DuckDuckGoProvider, RankTracker};
//!
//! # async fn example() -> Result<(), crawlkit_engine::rank::RankError> {
//! let provider = DuckDuckGoProvider::new_default();
//! let tracker = RankTracker::new();
//!
//! let pos = tracker
//!     .check_keyword("rust seo tools", Some("https://example.com"), &provider)
//!     .await?;
//! println!("position: {:?}", pos.position);
//! # Ok(())
//! # }
//! ```

use crate::http::HttpClient;
use crate::ratelimit::RateLimiter;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::Url;

pub mod history;
pub mod parser;
pub mod provider;
pub mod provider_ddg;
pub mod provider_gsc;

pub use history::{
    analyze_rank_trend, RankSnapshot, RankTrend, RankTrendDirection, NOT_RANKED_POSITION,
};
pub use parser::{decode_ddg_redirect, parse_ddg_results};
pub use provider::{OrganicResult, SerpProvider};
pub use provider_ddg::DuckDuckGoProvider;
pub use provider_gsc::GscSerpProvider;

/// Errors that can occur during rank tracking operations.
#[derive(Debug, Error)]
pub enum RankError {
    /// The SERP fetch failed.
    #[error("SERP request failed: {0}")]
    RequestFailed(String),

    /// The SERP response could not be parsed.
    #[error("failed to parse SERP response: {0}")]
    ParseError(String),

    /// The search engine returned a non-success status.
    #[error("search engine returned HTTP {status}")]
    HttpError {
        /// The HTTP status code returned.
        status: u16,
    },

    /// Rate limiting prevented the request.
    #[error("rate limited: {0}")]
    RateLimited(String),

    /// The provider configuration is invalid or incomplete.
    #[error("provider configuration error: {0}")]
    ProviderConfig(String),

    /// Trend analysis input was invalid.
    #[error("trend analysis error: {0}")]
    Trend(String),

    /// An unexpected internal failure.
    #[error("internal error: {0}")]
    Internal(String),
}

/// A rank-tracking project: one domain tracked on one engine/device/locale.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankProject {
    /// Unique project identifier.
    pub id: String,
    /// The domain being tracked (e.g. `example.com`).
    pub domain: String,
    /// Search engine backend: `duckduckgo` or `gsc`.
    pub search_engine: String,
    /// Device profile: `desktop` or `mobile`.
    pub device: String,
    /// Locale tag (e.g. `en-US`).
    pub locale: String,
}

/// A keyword tracked within a [`RankProject`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankKeyword {
    /// Unique keyword identifier.
    pub id: String,
    /// The project this keyword belongs to.
    pub project_id: String,
    /// The keyword phrase.
    pub keyword: String,
    /// Optional specific URL to track; defaults to the project domain.
    pub target_url: Option<String>,
    /// Whether this keyword is checked during runs.
    pub enabled: bool,
}

/// A single observed position for a keyword at a point in time.
///
/// `keyword_id` is the identifier of the stored keyword record. When a
/// [`RankPosition`] is produced by [`RankTracker::check_keyword`] the
/// keyword text is placed in `keyword_id` as a display fallback; callers
/// should overwrite it with the real storage ID before persisting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankPosition {
    /// The keyword this observation belongs to (see type docs).
    pub keyword_id: String,
    /// When the check was performed.
    pub checked_at: DateTime<Utc>,
    /// 1-based SERP position, or `None` when not in the top results.
    pub position: Option<u16>,
    /// URL of the matching result.
    pub url: Option<String>,
    /// Title of the matching result.
    pub title: Option<String>,
    /// Data source identifier (e.g. `duckduckgo`, `gsc`).
    pub source: String,
}

/// Orchestrates rank checks: rate limiting plus target matching.
///
/// The actual SERP retrieval is delegated to a [`SerpProvider`], so the
/// tracker works with any backend (DuckDuckGo, GSC, or a custom
/// integration). The tracker owns the [`HttpClient`] and
/// [`RateLimiter`] context and the device/locale used for searches.
pub struct RankTracker {
    http_client: HttpClient,
    rate_limiter: RateLimiter,
    device: String,
    locale: String,
}

impl RankTracker {
    /// Create a rank tracker with default HTTP and rate-limit settings
    /// (`desktop` device, `en-US` locale, 1 req/s per domain).
    ///
    /// # Panics
    ///
    /// Never in practice: the default crawl config always builds a
    /// valid HTTP client.
    #[must_use]
    pub fn new() -> Self {
        let http_client = HttpClient::from_crawl_config(&Default::default())
            .unwrap_or_else(|_| unreachable!("default crawl config produces a valid client"));
        Self {
            http_client,
            rate_limiter: RateLimiter::new(1.0, 5.0),
            device: "desktop".to_string(),
            locale: "en-US".to_string(),
        }
    }

    /// Create a rank tracker with explicit HTTP client and rate limiter.
    #[must_use]
    pub fn with_components(http_client: HttpClient, rate_limiter: RateLimiter) -> Self {
        Self {
            http_client,
            rate_limiter,
            device: "desktop".to_string(),
            locale: "en-US".to_string(),
        }
    }

    /// Set the device profile used for searches (`desktop` or `mobile`).
    #[must_use]
    pub fn with_device(mut self, device: impl Into<String>) -> Self {
        self.device = device.into();
        self
    }

    /// Set the locale used for searches (e.g. `en-US`).
    #[must_use]
    pub fn with_locale(mut self, locale: impl Into<String>) -> Self {
        self.locale = locale.into();
        self
    }

    /// The configured device profile.
    #[must_use]
    pub fn device(&self) -> &str {
        &self.device
    }

    /// The configured locale.
    #[must_use]
    pub fn locale(&self) -> &str {
        &self.locale
    }

    /// The shared HTTP client (also usable to build providers that
    /// reuse its connection pool).
    #[must_use]
    pub fn http_client(&self) -> &HttpClient {
        &self.http_client
    }

    /// Check the position of `keyword`, matching results against `target_url`.
    ///
    /// When `target_url` is `None`, the top organic result is reported
    /// (useful for pure SERP tracking). When provided, the first result
    /// whose host matches the target host wins; host comparison ignores
    /// the `www.` prefix and scheme, with a substring fallback for
    /// partial targets.
    ///
    /// The returned [`RankPosition`] carries the keyword text in
    /// `keyword_id`; overwrite it with the storage ID before persisting.
    ///
    /// # Errors
    ///
    /// Returns [`RankError`] if the request fails or is rate limited.
    pub async fn check_keyword(
        &self,
        keyword: &str,
        target_url: Option<&str>,
        provider: &dyn SerpProvider,
    ) -> Result<RankPosition, RankError> {
        self.rate_limiter
            .acquire(provider.rate_limit_domain())
            .await
            .map_err(|e| RankError::RateLimited(e.to_string()))?;

        let results = provider.search(keyword, &self.device, &self.locale).await?;
        let checked_at = Utc::now();

        let matched = match target_url {
            Some(target) => results
                .iter()
                .find(|r| url_matches_target(&r.url, target))
                .or_else(|| {
                    let t = target.to_ascii_lowercase();
                    results
                        .iter()
                        .find(|r| r.url.to_ascii_lowercase().contains(&t))
                }),
            None => results.first(),
        };

        Ok(match matched {
            Some(r) => RankPosition {
                keyword_id: keyword.to_string(),
                checked_at,
                position: Some(r.position),
                url: Some(r.url.clone()),
                title: Some(r.title.clone()),
                source: provider.name().to_string(),
            },
            None => RankPosition {
                keyword_id: keyword.to_string(),
                checked_at,
                position: None,
                url: None,
                title: None,
                source: provider.name().to_string(),
            },
        })
    }
}

impl Default for RankTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract a lowercase host from a URL or bare domain string.
#[must_use]
pub fn host_of(candidate: &str) -> Option<String> {
    let trimmed = candidate.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(scheme_pos) = trimmed.find("://") {
        if let Ok(url) = Url::parse(trimmed) {
            return url.host_str().map(|h| h.to_ascii_lowercase());
        }
        let _ = scheme_pos;
    }
    // Bare domain like "example.com" or "www.example.com/path"
    let bare = trimmed.split(['/', '?', '#']).next()?;
    if bare.is_empty() || bare.contains(char::is_whitespace) {
        return None;
    }
    Some(bare.to_ascii_lowercase())
}

/// Returns `true` when `result_url` points at the same host as `target`.
///
/// Comparison is case-insensitive and ignores the `www.` prefix.
#[must_use]
pub fn url_matches_target(result_url: &str, target: &str) -> bool {
    let (Some(result_host), Some(target_host)) = (host_of(result_url), host_of(target)) else {
        return false;
    };
    let normalize = |h: &str| {
        h.strip_prefix("www.")
            .unwrap_or(h)
            .trim_end_matches('.')
            .to_string()
    };
    normalize(&result_host) == normalize(&target_host)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_host_of_full_url() {
        assert_eq!(
            host_of("https://example.com/some/path"),
            Some("example.com".to_string())
        );
    }

    #[test]
    fn test_host_of_bare_domain() {
        assert_eq!(host_of("example.com"), Some("example.com".to_string()));
        assert_eq!(
            host_of("www.example.com/path"),
            Some("www.example.com".to_string())
        );
    }

    #[test]
    fn test_host_of_empty() {
        assert_eq!(host_of(""), None);
        assert_eq!(host_of("   "), None);
    }

    #[test]
    fn test_url_matches_target_www() {
        assert!(url_matches_target(
            "https://www.example.com/page",
            "example.com"
        ));
    }

    #[test]
    fn test_url_matches_target_exact() {
        assert!(url_matches_target(
            "https://example.com/page",
            "https://example.com/other"
        ));
    }

    #[test]
    fn test_url_matches_target_different() {
        assert!(!url_matches_target("https://other.com/page", "example.com"));
    }

    #[test]
    fn test_url_matches_target_subdomain_is_different() {
        assert!(!url_matches_target(
            "https://sub.example.com/",
            "example.com"
        ));
    }

    #[test]
    fn test_url_matches_target_empty_target() {
        assert!(!url_matches_target("https://example.com/", ""));
    }

    // ---- Full rank check cycle (mock provider + storage + trend) ----

    struct MockProvider {
        results: Vec<OrganicResult>,
    }

    #[async_trait::async_trait]
    impl SerpProvider for MockProvider {
        async fn search(
            &self,
            _keyword: &str,
            _device: &str,
            _locale: &str,
        ) -> Result<Vec<OrganicResult>, RankError> {
            Ok(self.results.clone())
        }

        fn name(&self) -> &str {
            "mock"
        }

        fn rate_limit_domain(&self) -> &str {
            "mock.invalid"
        }
    }

    fn mock_results() -> Vec<OrganicResult> {
        vec![
            OrganicResult {
                position: 1,
                url: "https://competitor.com/".to_string(),
                title: "Competitor".to_string(),
            },
            OrganicResult {
                position: 2,
                url: "https://www.example.com/guide".to_string(),
                title: "Example Guide".to_string(),
            },
            OrganicResult {
                position: 3,
                url: "https://other.net/".to_string(),
                title: "Other".to_string(),
            },
        ]
    }

    #[tokio::test]
    async fn test_full_rank_check_cycle() {
        use crate::storage::Storage;

        let storage = Storage::new_in_memory().unwrap();
        let project_id = storage
            .add_rank_project("example.com", "duckduckgo", "desktop", "en-US")
            .unwrap();
        let keyword_id = storage
            .add_rank_keyword(&project_id, "seo guide", None)
            .unwrap();

        let tracker = RankTracker::new();
        let provider = MockProvider {
            results: mock_results(),
        };

        // Check 1: target ranks #2 (www. prefix must not defeat the match).
        let mut pos = tracker
            .check_keyword("seo guide", Some("https://example.com"), &provider)
            .await
            .unwrap();
        assert_eq!(pos.position, Some(2));
        assert_eq!(pos.url.as_deref(), Some("https://www.example.com/guide"));
        assert_eq!(pos.source, "mock");
        pos.keyword_id = keyword_id.clone();
        storage.record_rank_position(&pos).unwrap();

        // Check 2: keyword absent from the SERP → not ranked.
        let mut missing = tracker
            .check_keyword("seo guide", Some("https://nowhere.dev"), &provider)
            .await
            .unwrap();
        assert_eq!(missing.position, None);
        assert_eq!(missing.url, None);
        missing.keyword_id = keyword_id.clone();
        storage.record_rank_position(&missing).unwrap();

        // Check 3: no target → top organic result is reported.
        let top = tracker
            .check_keyword("seo guide", None, &provider)
            .await
            .unwrap();
        assert_eq!(top.position, Some(1));
        assert_eq!(top.url.as_deref(), Some("https://competitor.com/"));

        // History round-trips through storage.
        let history = storage.get_rank_history(&keyword_id, 30).unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].position, Some(2));

        let snapshots: Vec<history::RankSnapshot> = history
            .iter()
            .map(|p| history::RankSnapshot {
                keyword_id: p.keyword_id.clone(),
                checked_at: p.checked_at,
                position: p.position,
                source: p.source.clone(),
            })
            .collect();
        let trend = history::analyze_rank_trend(&snapshots).unwrap();
        assert_eq!(trend.points, 2);
        assert_eq!(trend.change, i32::from(NOT_RANKED_POSITION) - 2);
        assert_eq!(trend.best_position, Some(2));
    }
}
