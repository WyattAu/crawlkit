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

        let api_key = self.api_key.as_ref().unwrap();
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

        let api_key = self.api_key.as_ref().unwrap();
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

        let api_key = self.api_key.as_ref().unwrap();
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

        let api_key = self.api_key.as_ref().unwrap();
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

        let access_token = self.access_token.as_ref().unwrap();
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
