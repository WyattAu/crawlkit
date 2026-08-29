use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors specific to GSC API operations.
#[derive(Debug, Error)]
pub enum GscError {
    /// Required environment variables are not set.
    #[error("GSC environment variables not set: {0}")]
    EnvMissing(String),

    /// HTTP request to GSC API failed.
    #[error("GSC API request failed: {0}")]
    RequestFailed(String),

    /// GSC API returned an error response.
    #[error("GSC API error (HTTP {status}): {body}")]
    ApiError { status: u16, body: String },

    /// Response parsing failed.
    #[error("failed to parse GSC response: {0}")]
    ParseError(String),
}

/// Client for the Google Search Console API.
///
/// Reads credentials from `GSC_ACCESS_TOKEN` and `GSC_SITE_URL`
/// environment variables, or accepts them directly.
pub struct GscClient {
    access_token: String,
    http_client: reqwest::Client,
    site_url: String,
}

impl GscClient {
    /// Create a new GSC client with explicit credentials.
    pub fn new(access_token: String, site_url: String) -> Self {
        Self {
            access_token,
            http_client: reqwest::Client::new(),
            site_url,
        }
    }

    /// Create a GSC client from environment variables.
    ///
    /// Reads `GSC_ACCESS_TOKEN` and `GSC_SITE_URL`. Returns `None`
    /// if either variable is missing.
    pub fn from_env() -> Option<Self> {
        let access_token = std::env::var("GSC_ACCESS_TOKEN").ok()?;
        let site_url = std::env::var("GSC_SITE_URL").ok()?;
        Some(Self::new(access_token, site_url))
    }

    /// The site URL this client is configured for.
    pub fn site_url(&self) -> &str {
        &self.site_url
    }

    /// Fetch search analytics data for a date range.
    ///
    /// `start_date` and `end_date` must be in `YYYY-MM-DD` format.
    /// GSC date ranges are limited to 3 months.
    ///
    /// # Errors
    ///
    /// Returns [`GscError`] if the API call or parsing fails.
    pub async fn get_search_analytics(
        &self,
        start_date: &str,
        end_date: &str,
    ) -> Result<GscAnalytics, GscError> {
        self.get_search_analytics_with_dimensions(start_date, end_date, &["query", "page"])
            .await
    }

    /// Fetch search analytics with specific dimensions.
    ///
    /// Valid dimensions: `"query"`, `"page"`, `"country"`, `"device"`.
    pub async fn get_search_analytics_with_dimensions(
        &self,
        start_date: &str,
        end_date: &str,
        dimensions: &[&str],
    ) -> Result<GscAnalytics, GscError> {
        let encoded_site = urlencoding::encode(&self.site_url);
        let url = format!(
            "https://searchconsole.googleapis.com/webmasters/v3/sites/{encoded_site}/searchAnalytics/query"
        );

        let body = serde_json::json!({
            "startDate": start_date,
            "endDate": end_date,
            "dimensions": dimensions,
            "rowLimit": 25000,
            "startRow": 0,
        });

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&self.access_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| GscError::RequestFailed(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(GscError::ApiError {
                status: status.as_u16(),
                body: text,
            });
        }

        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| GscError::ParseError(e.to_string()))?;

        let rows = data["rows"]
            .as_array()
            .cloned()
            .unwrap_or_default();

        let mut queries = Vec::new();
        let mut pages = Vec::new();
        let mut total_clicks: u64 = 0;
        let mut total_impressions: u64 = 0;
        let mut total_ctr: f64 = 0.0;
        let mut total_position: f64 = 0.0;
        let mut row_count: u64 = 0;

        for row in &rows {
            let keys = row["keys"]
                .as_array()
                .cloned()
                .unwrap_or_default();

            let key_str = keys
                .first()
                .and_then(|k| k.as_str())
                .unwrap_or("")
                .to_string();

            let clicks = row["clicks"].as_u64().unwrap_or(0);
            let impressions = row["impressions"].as_u64().unwrap_or(0);
            let ctr = row["ctr"].as_f64().unwrap_or(0.0);
            let position = row["position"].as_f64().unwrap_or(0.0);

            total_clicks += clicks;
            total_impressions += impressions;
            total_ctr += ctr;
            total_position += position;
            row_count += 1;

            let entry = GscRow {
                key: key_str.clone(),
                clicks,
                impressions,
                ctr,
                position,
            };

            if dimensions.contains(&"query") {
                queries.push(entry.clone());
            }
            if dimensions.contains(&"page") {
                pages.push(entry);
            }
        }

        let avg_ctr = if row_count > 0 {
            total_ctr / row_count as f64
        } else {
            0.0
        };
        let avg_position = if row_count > 0 {
            total_position / row_count as f64
        } else {
            0.0
        };

        Ok(GscAnalytics {
            queries,
            pages,
            total_clicks,
            total_impressions,
            average_ctr: avg_ctr,
            average_position: avg_position,
        })
    }

    /// Fetch top queries for the site.
    pub async fn top_queries(
        &self,
        start_date: &str,
        end_date: &str,
        limit: usize,
    ) -> Result<Vec<GscRow>, GscError> {
        let analytics = self
            .get_search_analytics_with_dimensions(start_date, end_date, &["query"])
            .await?;
        let mut rows = analytics.queries;
        rows.sort_by_key(|row| std::cmp::Reverse(row.clicks));
        rows.truncate(limit);
        Ok(rows)
    }

    /// Fetch top pages for the site.
    pub async fn top_pages(
        &self,
        start_date: &str,
        end_date: &str,
        limit: usize,
    ) -> Result<Vec<GscRow>, GscError> {
        let analytics = self
            .get_search_analytics_with_dimensions(start_date, end_date, &["page"])
            .await?;
        let mut rows = analytics.pages;
        rows.sort_by_key(|row| std::cmp::Reverse(row.clicks));
        rows.truncate(limit);
        Ok(rows)
    }

    /// Perform a URL inspection for a single URL.
    ///
    /// Uses the Search Analytics API to get performance data for a specific page.
    pub async fn get_url_inspection(
        &self,
        url: &str,
        start_date: &str,
        end_date: &str,
    ) -> Result<UrlInspection, GscError> {
        let encoded_site = urlencoding::encode(&self.site_url);
        let inspection_url = format!(
            "https://searchconsole.googleapis.com/webmasters/v3/sites/{encoded_site}/searchAnalytics/query"
        );

        let body = serde_json::json!({
            "startDate": start_date,
            "endDate": end_date,
            "dimensions": ["page"],
            "dimensionFilterGroups": [{
                "filters": [{
                    "dimension": "page",
                    "expression": url
                }]
            }],
            "rowLimit": 1,
        });

        let response = self
            .http_client
            .post(&inspection_url)
            .bearer_auth(&self.access_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| GscError::RequestFailed(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(GscError::ApiError {
                status: status.as_u16(),
                body: text,
            });
        }

        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| GscError::ParseError(e.to_string()))?;

        let row = data["rows"]
            .as_array()
            .and_then(|arr| arr.first());

        match row {
            Some(row) => {
                let clicks = row["clicks"].as_u64().unwrap_or(0);
                let impressions = row["impressions"].as_u64().unwrap_or(0);
                let ctr = row["ctr"].as_f64().unwrap_or(0.0);
                let position = row["position"].as_f64().unwrap_or(0.0);
                Ok(UrlInspection {
                    url: url.to_string(),
                    clicks,
                    impressions,
                    ctr,
                    position,
                    indexed: true,
                })
            }
            None => Ok(UrlInspection {
                url: url.to_string(),
                clicks: 0,
                impressions: 0,
                ctr: 0.0,
                position: 0.0,
                indexed: false,
            }),
        }
    }

    /// Get a list of verified sites in the GSC account.
    pub async fn list_sites(&self) -> Result<Vec<String>, GscError> {
        let url = "https://searchconsole.googleapis.com/webmasters/v3/sites";

        let response = self
            .http_client
            .get(url)
            .bearer_auth(&self.access_token)
            .send()
            .await
            .map_err(|e| GscError::RequestFailed(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(GscError::ApiError {
                status: status.as_u16(),
                body: text,
            });
        }

        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| GscError::ParseError(e.to_string()))?;

        let sites = data["siteEntry"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|entry| entry["siteUrl"].as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        Ok(sites)
    }
}

/// A single row from the GSC Search Analytics API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GscRow {
    /// The query string or page URL.
    pub key: String,
    /// Number of clicks.
    pub clicks: u64,
    /// Number of impressions.
    pub impressions: u64,
    /// Click-through rate (0.0–1.0).
    pub ctr: f64,
    /// Average position in search results.
    pub position: f64,
}

/// Aggregated search analytics data from GSC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GscAnalytics {
    /// Query-level data.
    pub queries: Vec<GscRow>,
    /// Page-level data.
    pub pages: Vec<GscRow>,
    /// Total clicks across all rows.
    pub total_clicks: u64,
    /// Total impressions across all rows.
    pub total_impressions: u64,
    /// Average CTR across all rows.
    pub average_ctr: f64,
    /// Average position across all rows.
    pub average_position: f64,
}

/// Result of a URL inspection via GSC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrlInspection {
    /// The URL that was inspected.
    pub url: String,
    /// Number of clicks in the date range.
    pub clicks: u64,
    /// Number of impressions in the date range.
    pub impressions: u64,
    /// Click-through rate.
    pub ctr: f64,
    /// Average position.
    pub position: f64,
    /// Whether the page appears to be indexed (has impressions/clicks).
    pub indexed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gsc_client_from_env_missing() {
        // Without env vars set, from_env returns None
        std::env::remove_var("GSC_ACCESS_TOKEN");
        std::env::remove_var("GSC_SITE_URL");
        assert!(GscClient::from_env().is_none());
    }

    #[test]
    fn test_gsc_client_from_env_token_only() {
        std::env::set_var("GSC_ACCESS_TOKEN", "test_token");
        std::env::remove_var("GSC_SITE_URL");
        assert!(GscClient::from_env().is_none());
        std::env::remove_var("GSC_ACCESS_TOKEN");
    }

    #[test]
    fn test_gsc_client_new() {
        let client = GscClient::new("token".to_string(), "https://example.com/".to_string());
        assert_eq!(client.site_url(), "https://example.com/");
    }

    #[test]
    fn test_gsc_row_serialization() {
        let row = GscRow {
            key: "test query".to_string(),
            clicks: 100,
            impressions: 5000,
            ctr: 0.02,
            position: 3.5,
        };
        let json = serde_json::to_string(&row).unwrap();
        assert!(json.contains("test query"));
        assert!(json.contains("100"));
    }

    #[test]
    fn test_gsc_analytics_serialization() {
        let analytics = GscAnalytics {
            queries: vec![GscRow {
                key: "rust".to_string(),
                clicks: 200,
                impressions: 10000,
                ctr: 0.02,
                position: 2.0,
            }],
            pages: vec![GscRow {
                key: "https://example.com/rust".to_string(),
                clicks: 200,
                impressions: 10000,
                ctr: 0.02,
                position: 2.0,
            }],
            total_clicks: 200,
            total_impressions: 10000,
            average_ctr: 0.02,
            average_position: 2.0,
        };
        let json = serde_json::to_string_pretty(&analytics).unwrap();
        assert!(json.contains("total_clicks"));
        assert!(json.contains("average_ctr"));
    }

    #[test]
    fn test_url_inspection_serialization() {
        let inspection = UrlInspection {
            url: "https://example.com/page".to_string(),
            clicks: 50,
            impressions: 2000,
            ctr: 0.025,
            position: 4.2,
            indexed: true,
        };
        let json = serde_json::to_string(&inspection).unwrap();
        assert!(json.contains("indexed"));
    }
}
