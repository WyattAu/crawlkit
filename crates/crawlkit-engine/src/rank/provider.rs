//! Search engine result page (SERP) provider abstraction.
//!
//! Implement [`SerpProvider`] to plug a new data source into the rank
//! tracker. Built-in implementations live in [`crate::rank::provider_ddg`]
//! (DuckDuckGo HTML scrape) and [`crate::rank::provider_gsc`] (Google
//! Search Console).

use crate::rank::RankError;

/// A single organic (non-ad) result extracted from a SERP.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OrganicResult {
    /// 1-based position in the organic results.
    pub position: u16,
    /// Destination URL of the result.
    pub url: String,
    /// Title text of the result link.
    pub title: String,
}

/// A backend that can return organic search results for a keyword.
///
/// Implementations must be `Send + Sync` so they can be shared across
/// concurrent rank checks. Rate limiting is handled by the
/// [`crate::rank::RankTracker`]; implementations should perform a
/// single search per call.
#[async_trait::async_trait]
pub trait SerpProvider: Send + Sync {
    /// Search for `keyword` and return the organic results in rank order.
    ///
    /// `device` is `"desktop"` or `"mobile"`; `locale` is a BCP 47 tag
    /// such as `"en-US"`. Providers that cannot honor either parameter
    /// should document how they degrade.
    ///
    /// # Errors
    ///
    /// Returns [`RankError`] on network, protocol, or parse failures.
    /// Returns an empty `Vec` when the keyword has no organic results.
    async fn search(
        &self,
        keyword: &str,
        device: &str,
        locale: &str,
    ) -> Result<Vec<OrganicResult>, RankError>;

    /// Human-readable data source name (e.g. `"duckduckgo"`).
    ///
    /// Stored alongside recorded positions so history can be
    /// attributed to the backend that produced it.
    fn name(&self) -> &str;

    /// Domain used as the rate-limit bucket key for this provider.
    ///
    /// Defaults to `"serp"`. Network backends should return the host
    /// they query so per-domain politeness applies.
    fn rate_limit_domain(&self) -> &str {
        "serp"
    }
}
