//! DuckDuckGo SERP provider.
//!
//! Scrapes the server-rendered HTML endpoint
//! `https://html.duckduckgo.com/html/` which requires no JavaScript and
//! no API key. See the DuckDuckGo help pages for the meaning of the
//! `kl` (region) and `df` (date range) query parameters.

use crate::http::HttpClient;
use crate::rank::parser::parse_ddg_results;
use crate::rank::provider::SerpProvider;
use crate::rank::{OrganicResult, RankError};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, ACCEPT_LANGUAGE, USER_AGENT};
use url::Url;

/// Base URL of the DuckDuckGo HTML endpoint.
pub const DDG_HTML_ENDPOINT: &str = "https://html.duckduckgo.com/html/";

/// Maximum organic results requested per search (one page ≈ 30).
pub const DDG_MAX_RESULTS: usize = 30;

const DESKTOP_USER_AGENT: &str =
    "Mozilla/5.0 (X11; Linux x86_64; rv:132.0) Gecko/20100101 Firefox/132.0";
const MOBILE_USER_AGENT: &str =
    "Mozilla/5.0 (iPhone; CPU iPhone OS 17_6 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.6 Mobile/15E148 Safari/604.1";

/// [`SerpProvider`] backed by the DuckDuckGo HTML endpoint.
pub struct DuckDuckGoProvider {
    http_client: HttpClient,
}

impl DuckDuckGoProvider {
    /// Create a provider with an explicit HTTP client.
    #[must_use]
    pub fn new(http_client: HttpClient) -> Self {
        Self { http_client }
    }

    /// Create a provider with crawlkit's default HTTP configuration.
    ///
    /// # Panics
    ///
    /// Never in practice: the default crawl config always builds a
    /// valid HTTP client.
    #[must_use]
    pub fn new_default() -> Self {
        let http_client = HttpClient::from_crawl_config(&Default::default())
            .unwrap_or_else(|_| unreachable!("default crawl config produces a valid client"));
        Self { http_client }
    }

    /// Map a BCP 47 locale (`en-US`) to a DuckDuckGo region code
    /// (`us-en`). Falls back to `"wt-wt"` (no region) for unparseable
    /// locales.
    #[must_use]
    pub fn locale_to_region(locale: &str) -> String {
        let parts: Vec<&str> = locale.split('-').filter(|p| !p.is_empty()).collect();
        match parts.as_slice() {
            [lang, country] => format!(
                "{}-{}",
                country.to_ascii_lowercase(),
                lang.to_ascii_lowercase()
            ),
            [lang] => format!(
                "{}-{}",
                lang.to_ascii_lowercase(),
                lang.to_ascii_lowercase()
            ),
            _ => "wt-wt".to_string(),
        }
    }

    fn user_agent_for_device(device: &str) -> &'static str {
        if device.eq_ignore_ascii_case("mobile") {
            MOBILE_USER_AGENT
        } else {
            DESKTOP_USER_AGENT
        }
    }

    fn build_request_url(keyword: &str, locale: &str) -> Result<Url, RankError> {
        let region = Self::locale_to_region(locale);
        Url::parse_with_params(
            DDG_HTML_ENDPOINT,
            &[("q", keyword), ("kl", region.as_str())],
        )
        .map_err(|e| RankError::Internal(format!("failed to build DDG query URL: {e}")))
    }

    fn build_headers(device: &str, locale: &str) -> Result<HeaderMap, RankError> {
        let mut headers = HeaderMap::new();
        let ua = HeaderValue::from_str(Self::user_agent_for_device(device))
            .map_err(|e| RankError::Internal(format!("invalid user agent: {e}")))?;
        let lang = HeaderValue::from_str(locale)
            .map_err(|e| RankError::Internal(format!("invalid locale header: {e}")))?;
        headers.insert(USER_AGENT, ua);
        headers.insert(ACCEPT_LANGUAGE, lang);
        headers.insert(
            ACCEPT,
            HeaderValue::from_static("text/html,application/xhtml+xml"),
        );
        Ok(headers)
    }
}

#[async_trait::async_trait]
impl SerpProvider for DuckDuckGoProvider {
    async fn search(
        &self,
        keyword: &str,
        device: &str,
        locale: &str,
    ) -> Result<Vec<OrganicResult>, RankError> {
        if keyword.trim().is_empty() {
            return Ok(Vec::new());
        }

        let url = Self::build_request_url(keyword, locale)?;
        let headers = Self::build_headers(device, locale)?;

        let client = self.http_client.inner();
        let response = client
            .get(url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| RankError::RequestFailed(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            return Err(RankError::HttpError {
                status: status.as_u16(),
            });
        }

        let body = response
            .text()
            .await
            .map_err(|e| RankError::RequestFailed(e.to_string()))?;

        let mut results = parse_ddg_results(&body);
        results.truncate(DDG_MAX_RESULTS);
        Ok(results)
    }

    fn name(&self) -> &str {
        "duckduckgo"
    }

    fn rate_limit_domain(&self) -> &str {
        "html.duckduckgo.com"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_locale_to_region_us() {
        assert_eq!(DuckDuckGoProvider::locale_to_region("en-US"), "us-en");
    }

    #[test]
    fn test_locale_to_region_germany() {
        assert_eq!(DuckDuckGoProvider::locale_to_region("de-DE"), "de-de");
    }

    #[test]
    fn test_locale_to_region_language_only() {
        assert_eq!(DuckDuckGoProvider::locale_to_region("en"), "en-en");
    }

    #[test]
    fn test_locale_to_region_fallback() {
        assert_eq!(DuckDuckGoProvider::locale_to_region(""), "wt-wt");
    }

    #[test]
    fn test_user_agent_for_device() {
        assert_eq!(
            DuckDuckGoProvider::user_agent_for_device("desktop"),
            DESKTOP_USER_AGENT
        );
        assert_eq!(
            DuckDuckGoProvider::user_agent_for_device("mobile"),
            MOBILE_USER_AGENT
        );
        // Unknown values degrade to the desktop profile.
        assert_eq!(
            DuckDuckGoProvider::user_agent_for_device("tablet"),
            DESKTOP_USER_AGENT
        );
    }

    #[test]
    fn test_build_request_url_encodes_keyword() {
        let url = DuckDuckGoProvider::build_request_url("rust seo tools", "en-US").unwrap();
        assert_eq!(url.host_str(), Some("html.duckduckgo.com"));
        assert!(url.query().unwrap().contains("q=rust+seo+tools"));
        assert!(url.query().unwrap().contains("kl=us-en"));
    }

    #[test]
    fn test_build_headers_desktop() {
        let headers = DuckDuckGoProvider::build_headers("desktop", "en-US").unwrap();
        assert_eq!(
            headers.get(USER_AGENT).and_then(|v| v.to_str().ok()),
            Some(DESKTOP_USER_AGENT)
        );
        assert_eq!(
            headers.get(ACCEPT_LANGUAGE).and_then(|v| v.to_str().ok()),
            Some("en-US")
        );
    }

    #[test]
    fn test_provider_metadata() {
        let provider = DuckDuckGoProvider::new_default();
        assert_eq!(provider.name(), "duckduckgo");
        assert_eq!(provider.rate_limit_domain(), "html.duckduckgo.com");
    }
}
