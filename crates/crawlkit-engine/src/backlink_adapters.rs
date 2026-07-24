use serde::{Deserialize, Serialize};

/// External backlink data source adapter trait.
///
/// All backlink providers (Ahrefs, Majestic, GSC) implement this trait.
#[async_trait::async_trait]
pub trait BacklinkAdapter: Send + Sync {
    /// Get adapter name.
    fn name(&self) -> &str;

    /// Fetch backlinks for a domain.
    ///
    /// # Errors
    /// Returns error if API call fails.
    async fn fetch_backlinks(
        &self,
        domain: &str,
        limit: usize,
    ) -> Result<Vec<ExternalBacklink>, AdapterError>;

    /// Get domain rating/authority score.
    ///
    /// # Errors
    /// Returns error if API call fails.
    async fn get_domain_rating(&self, domain: &str) -> Result<f64, AdapterError>;

    /// Check if adapter is configured and available.
    fn is_available(&self) -> bool;
}

/// A backlink from an external source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalBacklink {
    /// Source URL of the backlink.
    pub source_url: String,
    /// Target URL being linked to.
    pub target_url: String,
    /// Anchor text.
    pub anchor_text: String,
    /// Domain rating of source (0-100).
    pub domain_rating: f64,
    /// Whether the link is followed.
    pub is_followed: bool,
    /// First seen date.
    pub first_seen: Option<String>,
    /// Last seen date.
    pub last_seen: Option<String>,
}

/// Adapter errors.
#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    #[error("API key not configured")]
    ApiKeyMissing,

    #[error("API request failed: {0}")]
    RequestFailed(String),

    #[error("Rate limit exceeded, retry after {0}s")]
    RateLimited(u64),

    #[error("Domain not found: {0}")]
    DomainNotFound(String),

    #[error("Adapter not available: {0}")]
    NotAvailable(String),
}

// ---------------------------------------------------------------------------
// Ahrefs Adapter
// ---------------------------------------------------------------------------

/// Ahrefs backlink adapter.
pub struct AhrefsAdapter {
    api_key: Option<String>,
    #[allow(dead_code)]
    base_url: String,
}

impl AhrefsAdapter {
    /// Create new Ahrefs adapter.
    #[must_use]
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            api_key,
            base_url: "https://api.ahrefs.com".to_string(),
        }
    }

    /// Create from environment variable.
    #[must_use]
    pub fn from_env() -> Self {
        let api_key = std::env::var("AHREFS_API_KEY").ok();
        Self::new(api_key)
    }
}

#[async_trait::async_trait]
impl BacklinkAdapter for AhrefsAdapter {
    fn name(&self) -> &str {
        "ahrefs"
    }

    async fn fetch_backlinks(
        &self,
        domain: &str,
        limit: usize,
    ) -> Result<Vec<ExternalBacklink>, AdapterError> {
        if !self.is_available() {
            return Err(AdapterError::ApiKeyMissing);
        }

        let api_key = self.api_key.as_deref().ok_or(AdapterError::ApiKeyMissing)?;
        let url = format!(
            "https://api.ahrefs.com/v3/backlinks?target={}&mode=live&output=json&token={}&limit={}",
            urlencoding::encode(domain),
            api_key,
            limit
        );

        let response = reqwest::get(&url)
            .await
            .map_err(|e| AdapterError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(AdapterError::RequestFailed(format!(
                "HTTP {}",
                response.status()
            )));
        }

        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AdapterError::RequestFailed(e.to_string()))?;

        let backlinks = data["refdomains"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|item| ExternalBacklink {
                        source_url: item["source_url"].as_str().unwrap_or("").to_string(),
                        target_url: item["target_url"].as_str().unwrap_or("").to_string(),
                        anchor_text: item["anchor"].as_str().unwrap_or("").to_string(),
                        domain_rating: item["domain_rating"].as_f64().unwrap_or(0.0),
                        is_followed: item["dofollow"].as_bool().unwrap_or(false),
                        first_seen: item["first_seen"].as_str().map(|s| s.to_string()),
                        last_seen: item["last_seen"].as_str().map(|s| s.to_string()),
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(backlinks)
    }

    async fn get_domain_rating(&self, domain: &str) -> Result<f64, AdapterError> {
        if !self.is_available() {
            return Err(AdapterError::ApiKeyMissing);
        }

        let api_key = self.api_key.as_deref().ok_or(AdapterError::ApiKeyMissing)?;
        let url = format!(
            "https://api.ahrefs.com/v3/domain-rating?target={}&output=json&token={}",
            urlencoding::encode(domain),
            api_key
        );

        let response = reqwest::get(&url)
            .await
            .map_err(|e| AdapterError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(AdapterError::RequestFailed(format!(
                "HTTP {}",
                response.status()
            )));
        }

        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AdapterError::RequestFailed(e.to_string()))?;

        Ok(data["domain_rating"].as_f64().unwrap_or(0.0))
    }

    fn is_available(&self) -> bool {
        self.api_key.is_some()
    }
}

// ---------------------------------------------------------------------------
// Majestic Adapter
// ---------------------------------------------------------------------------

/// Majestic backlink adapter.
pub struct MajesticAdapter {
    api_key: Option<String>,
}

impl MajesticAdapter {
    /// Create new Majestic adapter.
    #[must_use]
    pub fn new(api_key: Option<String>) -> Self {
        Self { api_key }
    }

    /// Create from environment variable.
    #[must_use]
    pub fn from_env() -> Self {
        let api_key = std::env::var("MAJESTIC_API_KEY").ok();
        Self::new(api_key)
    }
}

#[async_trait::async_trait]
impl BacklinkAdapter for MajesticAdapter {
    fn name(&self) -> &str {
        "majestic"
    }

    async fn fetch_backlinks(
        &self,
        domain: &str,
        limit: usize,
    ) -> Result<Vec<ExternalBacklink>, AdapterError> {
        if !self.is_available() {
            return Err(AdapterError::ApiKeyMissing);
        }

        let api_key = self.api_key.as_deref().ok_or(AdapterError::ApiKeyMissing)?;
        let client = reqwest::Client::new();

        let params = [
            ("cmd", "BacklinkData"),
            ("item", domain),
            ("count", &limit.to_string()),
            ("output", "json"),
            ("key", api_key),
        ];

        let response = client
            .post("https://api.majestic.com/api/command")
            .form(&params)
            .send()
            .await
            .map_err(|e| AdapterError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(AdapterError::RequestFailed(format!(
                "HTTP {}",
                response.status()
            )));
        }

        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AdapterError::RequestFailed(e.to_string()))?;

        let backlinks = data["Data"]["Backlinks"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|item| ExternalBacklink {
                        source_url: item["SourceURL"].as_str().unwrap_or("").to_string(),
                        target_url: item["TargetURL"].as_str().unwrap_or("").to_string(),
                        anchor_text: item["AnchorText"].as_str().unwrap_or("").to_string(),
                        domain_rating: item["TrustFlow"].as_f64().unwrap_or(0.0),
                        is_followed: item["Flag"].as_str().unwrap_or("") != "nofollow",
                        first_seen: item["FirstSeen"].as_str().map(|s| s.to_string()),
                        last_seen: item["LastSeen"].as_str().map(|s| s.to_string()),
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(backlinks)
    }

    async fn get_domain_rating(&self, domain: &str) -> Result<f64, AdapterError> {
        if !self.is_available() {
            return Err(AdapterError::ApiKeyMissing);
        }

        let api_key = self.api_key.as_deref().ok_or(AdapterError::ApiKeyMissing)?;
        let client = reqwest::Client::new();

        let params = [
            ("cmd", "GetIndexItemInfo"),
            ("item", domain),
            ("items", "0"),
            ("output", "json"),
            ("key", api_key),
        ];

        let response = client
            .post("https://api.majestic.com/api/command")
            .form(&params)
            .send()
            .await
            .map_err(|e| AdapterError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(AdapterError::RequestFailed(format!(
                "HTTP {}",
                response.status()
            )));
        }

        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AdapterError::RequestFailed(e.to_string()))?;

        Ok(data["Data"]["Results"][0]["TrustFlow"]
            .as_f64()
            .unwrap_or(0.0))
    }

    fn is_available(&self) -> bool {
        self.api_key.is_some()
    }
}

// ---------------------------------------------------------------------------
// Google Search Console Adapter
// ---------------------------------------------------------------------------

/// Google Search Console adapter.
pub struct GscAdapter {
    access_token: Option<String>,
}

impl GscAdapter {
    /// Create new GSC adapter.
    #[must_use]
    pub fn new(access_token: Option<String>) -> Self {
        Self { access_token }
    }

    /// Create from environment variable.
    #[must_use]
    pub fn from_env() -> Self {
        let access_token = std::env::var("GSC_ACCESS_TOKEN").ok();
        Self::new(access_token)
    }
}

#[async_trait::async_trait]
impl BacklinkAdapter for GscAdapter {
    fn name(&self) -> &str {
        "google_search_console"
    }

    async fn fetch_backlinks(
        &self,
        domain: &str,
        limit: usize,
    ) -> Result<Vec<ExternalBacklink>, AdapterError> {
        if !self.is_available() {
            return Err(AdapterError::ApiKeyMissing);
        }

        let access_token = self
            .access_token
            .as_deref()
            .ok_or(AdapterError::ApiKeyMissing)?;
        let client = reqwest::Client::new();

        // GSC API for external links
        let url = format!(
            "https://searchconsole.googleapis.com/webmasters/v3/sites/{}%2FexternalLinks?rowLimit={}",
            urlencoding::encode(&format!("https://{}/", domain)),
            limit
        );

        let response = client
            .get(&url)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| AdapterError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(AdapterError::RequestFailed(format!(
                "HTTP {}",
                response.status()
            )));
        }

        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AdapterError::RequestFailed(e.to_string()))?;

        let backlinks = data["linkEntries"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|item| ExternalBacklink {
                        source_url: item["targetPage"].as_str().unwrap_or("").to_string(),
                        target_url: item["pageUrl"].as_str().unwrap_or("").to_string(),
                        anchor_text: String::new(), // GSC doesn't provide anchor text
                        domain_rating: 0.0,         // GSC doesn't provide DR
                        is_followed: true,          // GSC links are typically followed
                        first_seen: None,
                        last_seen: None,
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(backlinks)
    }

    async fn get_domain_rating(&self, _domain: &str) -> Result<f64, AdapterError> {
        // GSC doesn't provide domain rating
        Err(AdapterError::NotAvailable(
            "GSC does not provide domain rating".to_string(),
        ))
    }

    fn is_available(&self) -> bool {
        self.access_token.is_some()
    }
}

/// GSC search analytics result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GscSearchResult {
    /// Query or page URL.
    pub key: String,
    /// Number of clicks.
    pub clicks: u64,
    /// Number of impressions.
    pub impressions: u64,
    /// Click-through rate (0.0-1.0).
    pub ctr: f64,
    /// Average position in search results.
    pub position: f64,
}

/// GSC index coverage entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GscCoverageEntry {
    /// The URL or page.
    pub url: String,
    /// Index status: "indexed", "excluded", or "error".
    pub status: String,
    /// Number of pages in this coverage category.
    pub coverage_count: u64,
}

impl GscAdapter {
    /// Fetch search analytics from Google Search Console.
    ///
    /// Returns top queries or pages with clicks, impressions, CTR, and position.
    pub async fn search_analytics(
        &self,
        domain: &str,
        row_limit: usize,
        dimension: &str, // "query" or "page"
    ) -> Result<Vec<GscSearchResult>, AdapterError> {
        if !self.is_available() {
            return Err(AdapterError::ApiKeyMissing);
        }

        let access_token = self
            .access_token
            .as_deref()
            .ok_or(AdapterError::ApiKeyMissing)?;
        let client = reqwest::Client::new();

        let site_url = format!("https://{domain}/");
        let url = format!(
            "https://searchconsole.googleapis.com/webmasters/v3/sites/{}/searchAnalytics/query",
            urlencoding::encode(&site_url)
        );

        let body = serde_json::json!({
            "startDate": "2026-06-01",
            "endDate": "2026-06-30",
            "dimensions": [dimension],
            "rowLimit": row_limit,
        });

        let response = client
            .post(&url)
            .bearer_auth(access_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| AdapterError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(AdapterError::RequestFailed(format!(
                "HTTP {}",
                response.status()
            )));
        }

        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AdapterError::RequestFailed(e.to_string()))?;

        let results = data["rows"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|item| {
                        let key = item["keys"]
                            .as_array()
                            .and_then(|k| k.first())
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        GscSearchResult {
                            key,
                            clicks: item["clicks"].as_u64().unwrap_or(0),
                            impressions: item["impressions"].as_u64().unwrap_or(0),
                            ctr: item["ctr"].as_f64().unwrap_or(0.0),
                            position: item["position"].as_f64().unwrap_or(0.0),
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(results)
    }

    /// Fetch top queries from Google Search Console.
    pub async fn top_queries(
        &self,
        domain: &str,
        limit: usize,
    ) -> Result<Vec<GscSearchResult>, AdapterError> {
        self.search_analytics(domain, limit, "query").await
    }

    /// Fetch top pages from Google Search Console.
    pub async fn top_pages(
        &self,
        domain: &str,
        limit: usize,
    ) -> Result<Vec<GscSearchResult>, AdapterError> {
        self.search_analytics(domain, limit, "page").await
    }

    /// Fetch index coverage data from Google Search Console.
    ///
    /// Uses the Search Analytics API with `dimension: ["page"]` to retrieve
    /// page-level index status information. Pages with clicks/impressions
    /// are classified as "indexed"; the endpoint also surfacescrawl errors
    /// and excluded URLs when available.
    pub async fn index_coverage(
        &self,
        domain: &str,
    ) -> Result<Vec<GscCoverageEntry>, AdapterError> {
        if !self.is_available() {
            return Err(AdapterError::ApiKeyMissing);
        }

        let access_token = self
            .access_token
            .as_deref()
            .ok_or(AdapterError::ApiKeyMissing)?;
        let client = reqwest::Client::new();

        let site_url = format!("https://{domain}/");
        let url = format!(
            "https://searchconsole.googleapis.com/webmasters/v3/sites/{}/searchAnalytics/query",
            urlencoding::encode(&site_url)
        );

        // Use page dimension with a generous row limit to capture coverage data
        let body = serde_json::json!({
            "startDate": "2026-06-01",
            "endDate": "2026-06-30",
            "dimensions": ["page"],
            "rowLimit": 25000,
        });

        let response = client
            .post(&url)
            .bearer_auth(access_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| AdapterError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(AdapterError::RequestFailed(format!(
                "HTTP {}",
                response.status()
            )));
        }

        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AdapterError::RequestFailed(e.to_string()))?;

        let entries = data["rows"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|item| {
                        let url = item["keys"]
                            .as_array()
                            .and_then(|k| k.first())
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();

                        let clicks = item["clicks"].as_u64().unwrap_or(0);
                        let impressions = item["impressions"].as_u64().unwrap_or(0);

                        // Classify index status based on Search Analytics data:
                        // - pages with impressions are indexed
                        // - pages with zero impressions but present in results are likely indexed but not visible
                        // - pages not returned at all are excluded or errored
                        let status = if clicks > 0 || impressions > 0 {
                            "indexed".to_string()
                        } else {
                            "error".to_string()
                        };

                        GscCoverageEntry {
                            url,
                            status,
                            coverage_count: clicks + impressions,
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(entries)
    }
}

// ---------------------------------------------------------------------------
// Adapter Registry
// ---------------------------------------------------------------------------

/// Registry of available backlink adapters.
pub struct BacklinkAdapterRegistry {
    adapters: Vec<Box<dyn BacklinkAdapter>>,
}

impl BacklinkAdapterRegistry {
    /// Create registry with default adapters.
    #[must_use]
    pub fn with_defaults() -> Self {
        Self {
            adapters: vec![
                Box::new(AhrefsAdapter::from_env()),
                Box::new(MajesticAdapter::from_env()),
                Box::new(GscAdapter::from_env()),
            ],
        }
    }

    /// Get all available adapters.
    #[must_use]
    pub fn available(&self) -> Vec<&dyn BacklinkAdapter> {
        self.adapters
            .iter()
            .filter(|a| a.is_available())
            .map(|a| a.as_ref())
            .collect()
    }

    /// Get adapter by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&dyn BacklinkAdapter> {
        self.adapters
            .iter()
            .find(|a| a.name() == name)
            .map(|a| a.as_ref())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ahrefs_adapter_not_available() {
        let adapter = AhrefsAdapter::new(None);
        assert!(!adapter.is_available());
    }

    #[test]
    fn test_ahrefs_adapter_available() {
        let adapter = AhrefsAdapter::new(Some("test_key".to_string()));
        assert!(adapter.is_available());
    }

    #[test]
    fn test_backlink_registry() {
        let registry = BacklinkAdapterRegistry::with_defaults();
        assert!(registry.get("ahrefs").is_some());
        assert!(registry.get("majestic").is_some());
        assert!(registry.get("google_search_console").is_some());
        assert!(registry.get("nonexistent").is_none());
    }
}
