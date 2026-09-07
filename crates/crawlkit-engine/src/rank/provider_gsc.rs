//! Google Search Console SERP provider.
//!
//! Wraps the existing [`GscClient`](crate::gsc::GscClient) to implement
//! [`SerpProvider`]. Positions come from GSC Search Analytics: they are
//! *average* Google positions over the lookback window with daily
//! granularity — real positions, but not a real-time SERP snapshot.
//! `device` and `locale` parameters are accepted but not applied
//! (GSC data is reported for the property as a whole).

use crate::gsc::GscClient;
use crate::rank::provider::SerpProvider;
use crate::rank::{OrganicResult, RankError};

/// Default lookback window in days for GSC position lookups.
pub const GSC_LOOKBACK_DAYS: i64 = 7;

/// [`SerpProvider`] backed by the Google Search Console API.
pub struct GscSerpProvider {
    client: GscClient,
    lookback_days: i64,
}

impl GscSerpProvider {
    /// Wrap an existing GSC client.
    #[must_use]
    pub fn new(client: GscClient) -> Self {
        Self {
            client,
            lookback_days: GSC_LOOKBACK_DAYS,
        }
    }

    /// Build from `GSC_ACCESS_TOKEN` / `GSC_SITE_URL` environment
    /// variables. Returns `None` when either is missing.
    #[must_use]
    pub fn from_env() -> Option<Self> {
        Some(Self::new(GscClient::from_env()?))
    }

    /// Override the lookback window (clamped to 1–90 days; short
    /// windows reflect current rankings best).
    #[must_use]
    pub fn with_lookback_days(mut self, days: i64) -> Self {
        self.lookback_days = days.clamp(1, 90);
        self
    }

    /// The underlying GSC client.
    #[must_use]
    pub fn client(&self) -> &GscClient {
        &self.client
    }

    /// The configured lookback window in days.
    #[must_use]
    pub fn lookback_days(&self) -> i64 {
        self.lookback_days
    }

    /// Look up the best-ranking page for `keyword` in the lookback
    /// window.
    async fn best_row_for_keyword(
        &self,
        keyword: &str,
    ) -> Result<Option<crate::gsc::GscQueryPageRow>, RankError> {
        let end = chrono::Utc::now().date_naive() - chrono::Duration::days(1);
        let start = end - chrono::Duration::days(self.lookback_days - 1);

        let rows = self
            .client
            .get_query_page_rows(
                &start.format("%Y-%m-%d").to_string(),
                &end.format("%Y-%m-%d").to_string(),
            )
            .await
            .map_err(RankError::from)?;

        Ok(rows
            .into_iter()
            .filter(|row| row.query.eq_ignore_ascii_case(keyword))
            .min_by(|a, b| {
                a.position
                    .partial_cmp(&b.position)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }))
    }
}

#[async_trait::async_trait]
impl SerpProvider for GscSerpProvider {
    async fn search(
        &self,
        keyword: &str,
        _device: &str,
        _locale: &str,
    ) -> Result<Vec<OrganicResult>, RankError> {
        if keyword.trim().is_empty() {
            return Ok(Vec::new());
        }

        match self.best_row_for_keyword(keyword).await? {
            Some(row) => {
                // GSC reports fractional average positions; round to
                // the nearest integer SERP slot.
                let position = row.position.round().clamp(1.0, f64::from(u16::MAX)) as u16;
                Ok(vec![OrganicResult {
                    position,
                    url: row.page,
                    title: row.query,
                }])
            }
            None => Ok(Vec::new()),
        }
    }

    fn name(&self) -> &str {
        "gsc"
    }

    fn rate_limit_domain(&self) -> &str {
        "searchconsole.googleapis.com"
    }
}

impl From<crate::gsc::GscError> for RankError {
    fn from(err: crate::gsc::GscError) -> Self {
        match err {
            crate::gsc::GscError::EnvMissing(details) => RankError::ProviderConfig(details),
            crate::gsc::GscError::RequestFailed(details) => RankError::RequestFailed(details),
            crate::gsc::GscError::ApiError { status, body } => {
                RankError::Internal(format!("GSC API error (HTTP {status}): {body}"))
            }
            crate::gsc::GscError::ParseError(details) => RankError::ParseError(details),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_metadata() {
        let provider = GscSerpProvider::new(GscClient::new(
            "token".to_string(),
            "https://example.com/".to_string(),
        ));
        assert_eq!(provider.name(), "gsc");
        assert_eq!(provider.rate_limit_domain(), "searchconsole.googleapis.com");
    }

    #[test]
    fn test_lookback_days_default_and_clamp() {
        let make = || {
            GscSerpProvider::new(GscClient::new(
                "token".to_string(),
                "https://example.com/".to_string(),
            ))
        };
        assert_eq!(make().lookback_days(), GSC_LOOKBACK_DAYS);
        assert_eq!(make().with_lookback_days(0).lookback_days(), 1);
        assert_eq!(make().with_lookback_days(500).lookback_days(), 90);
        assert_eq!(make().with_lookback_days(30).lookback_days(), 30);
    }

    #[test]
    fn test_gsc_error_conversion() {
        let rank_err: RankError = crate::gsc::GscError::EnvMissing("token".to_string()).into();
        assert!(matches!(rank_err, RankError::ProviderConfig(_)));
    }
}
