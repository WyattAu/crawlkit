use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use reqwest::header::USER_AGENT;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use tokio::time::sleep;
use url::Url;

use crate::{CrawlConfig, CrawlError, FetchResult, RedirectHop};

/// Retry policy for failed requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts.
    pub max_retries: usize,
    /// Initial backoff duration.
    #[serde(with = "crate::duration_ms")]
    pub initial_backoff: Duration,
    /// Maximum backoff duration.
    #[serde(with = "crate::duration_ms")]
    pub max_backoff: Duration,
    /// Multiplier applied to backoff on each attempt.
    pub backoff_multiplier: f64,
    /// HTTP status codes that trigger a retry.
    pub retryable_statuses: Vec<u16>,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff: Duration::from_secs(1),
            max_backoff: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            retryable_statuses: vec![429, 500, 502, 503, 504],
        }
    }
}

impl RetryPolicy {
    /// Returns the backoff duration for a given attempt number (0-indexed).
    pub fn backoff_duration(&self, attempt: usize) -> Duration {
        let base = self.initial_backoff.as_secs_f64();
        let backoff = base * self.backoff_multiplier.powi(attempt as i32);
        let capped = backoff.min(self.max_backoff.as_secs_f64());
        Duration::from_secs_f64(capped)
    }

    /// Returns `true` if the given status code should trigger a retry.
    pub fn is_retryable(&self, status: u16) -> bool {
        self.retryable_statuses.contains(&status)
    }
}

/// User-agent rotator that cycles through a list of user-agent strings.
#[derive(Debug)]
pub struct UserAgentRotator {
    agents: Vec<String>,
    index: AtomicUsize,
}

impl UserAgentRotator {
    /// Creates a new rotator with the given user-agent strings.
    ///
    /// # Panics
    ///
    /// Panics if `agents` is empty.
    pub fn new(agents: Vec<String>) -> Self {
        assert!(
            !agents.is_empty(),
            "UserAgentRotator requires at least one user-agent"
        );
        Self {
            agents,
            index: AtomicUsize::new(0),
        }
    }

    /// Returns the next user-agent string in rotation.
    pub fn next(&self) -> &str {
        let idx = self.index.fetch_add(1, Ordering::Relaxed);
        &self.agents[idx % self.agents.len()]
    }

    /// Returns the number of user-agents in the rotation.
    pub fn len(&self) -> usize {
        self.agents.len()
    }

    /// Returns `true` if the rotation contains no user-agents.
    pub fn is_empty(&self) -> bool {
        self.agents.is_empty()
    }
}

impl Default for UserAgentRotator {
    fn default() -> Self {
        Self::new(vec![format!("crawlkit/{}", env!("CARGO_PKG_VERSION"))])
    }
}

/// Configuration for the HTTP client.
#[derive(Debug, Clone)]
pub struct HttpClientConfig {
    /// Request timeout.
    pub timeout: Duration,
    /// Maximum number of redirects to follow.
    pub max_redirects: usize,
    /// Retry policy.
    pub retry_policy: RetryPolicy,
    /// User-agent rotator.
    pub user_agent: Arc<UserAgentRotator>,
    /// Maximum response body size in bytes (0 = unlimited).
    pub max_body_size: usize,
}

impl From<&CrawlConfig> for HttpClientConfig {
    fn from(config: &CrawlConfig) -> Self {
        Self {
            timeout: config.request_timeout,
            max_redirects: config.max_redirects,
            retry_policy: RetryPolicy::default(),
            user_agent: Arc::new(UserAgentRotator::new(vec![config.user_agent.clone()])),
            max_body_size: 10 * 1024 * 1024, // 10MB default
        }
    }
}

/// An HTTP client with retry, redirect tracking, and user-agent rotation.
pub struct HttpClient {
    client: Client,
    config: HttpClientConfig,
}

impl HttpClient {
    /// Creates a new `HttpClient` from the given configuration.
    ///
    /// Builds a `reqwest::Client` with TLS, HTTP/2, and redirect policy.
    pub fn new(config: HttpClientConfig) -> Result<Self, CrawlError> {
        let client = Client::builder()
            .timeout(config.timeout)
            .redirect(reqwest::redirect::Policy::limited(config.max_redirects))
            .user_agent(config.user_agent.next())
            .https_only(true)
            .http2_prior_knowledge()
            .build()?;

        Ok(Self { client, config })
    }

    /// Creates a new `HttpClient` from a `CrawlConfig`.
    pub fn from_crawl_config(config: &CrawlConfig) -> Result<Self, CrawlError> {
        Self::new(HttpClientConfig::from(config))
    }

    /// Fetches a URL with retry logic and redirect tracking.
    ///
    /// Returns a `FetchResult` with the final URL, status, headers, and body.
    /// Follows redirects manually up to `max_redirects` to record each hop.
    pub async fn fetch(&self, url: &Url) -> Result<FetchResult, CrawlError> {
        self.fetch_with_redirects(url, self.config.max_redirects)
            .await
    }

    /// Fetches a URL, following up to `max_hops` redirects manually.
    ///
    /// Each redirect hop is recorded. If the hop limit is exceeded,
    /// `CrawlError::TooManyRedirects` is returned.
    pub async fn fetch_with_redirects(
        &self,
        url: &Url,
        max_hops: usize,
    ) -> Result<FetchResult, CrawlError> {
        let mut current_url = url.clone();
        let mut hops: Vec<RedirectHop> = Vec::new();

        for _ in 0..=max_hops {
            match self.fetch_once(&current_url).await {
                Ok((final_url, status, headers, body, elapsed)) => {
                    if status.is_redirection() {
                        let next_url = headers
                            .iter()
                            .find(|(k, _)| k.eq_ignore_ascii_case("location"))
                            .map(|(_, v)| v.clone());

                        match next_url {
                            Some(loc) => {
                                let resolved = current_url.join(&loc)?;
                                hops.push(RedirectHop {
                                    from: current_url.clone(),
                                    to: resolved.clone(),
                                    status_code: status.as_u16(),
                                });
                                current_url = resolved;
                                continue;
                            }
                            None => {
                                // No Location header — return the redirect response as-is
                                let body_size = body.len();
                                return Ok(FetchResult {
                                    final_url,
                                    status_code: status.as_u16(),
                                    headers,
                                    body,
                                    response_time: elapsed,
                                    body_size,
                                    fetched_at: chrono::Utc::now(),
                                });
                            }
                        }
                    }

                    let body_size = body.len();
                    return Ok(FetchResult {
                        final_url,
                        status_code: status.as_u16(),
                        headers,
                        body,
                        response_time: elapsed,
                        body_size,
                        fetched_at: chrono::Utc::now(),
                    });
                }
                Err(CrawlError::RequestFailed(e)) => {
                    if e.is_timeout() || e.is_connect() {
                        return Err(CrawlError::RequestFailed(e));
                    }
                    return Err(CrawlError::RequestFailed(e));
                }
                Err(e) => return Err(e),
            }
        }

        Err(CrawlError::TooManyRedirects(max_hops))
    }

    /// Performs a single HTTP request with retry logic.
    ///
    /// Returns the final URL, status, headers, body text, and elapsed time.
    async fn fetch_once(
        &self,
        url: &Url,
    ) -> Result<(Url, StatusCode, Vec<(String, String)>, String, Duration), CrawlError> {
        let mut last_error: Option<CrawlError> = None;
        let max_retries = self.config.retry_policy.max_retries;

        for attempt in 0..=max_retries {
            let start = Instant::now();
            let user_agent = self.config.user_agent.next();

            let result = self
                .client
                .get(url.as_str())
                .header(USER_AGENT, user_agent)
                .send()
                .await;

            match result {
                Ok(response) => {
                    let status = response.status();
                    let elapsed = start.elapsed();
                    let headers: Vec<(String, String)> = response
                        .headers()
                        .iter()
                        .map(|(k, v)| {
                            (
                                k.as_str().to_string(),
                                String::from_utf8_lossy(v.as_bytes()).to_string(),
                            )
                        })
                        .collect();

                    if self.config.retry_policy.is_retryable(status.as_u16())
                        && attempt < max_retries
                    {
                        let backoff = self.config.retry_policy.backoff_duration(attempt);

                        // Respect Retry-After header for 429
                        if status == StatusCode::TOO_MANY_REQUESTS {
                            if let Some(retry_after) = headers
                                .iter()
                                .find(|(k, _)| k.eq_ignore_ascii_case("retry-after"))
                                .and_then(|(_, v)| v.parse::<u64>().ok())
                            {
                                let wait = Duration::from_secs(retry_after).max(backoff);
                                tracing::warn!(
                                    url = %url,
                                    status = status.as_u16(),
                                    retry_after = retry_after,
                                    "429 Too Many Requests, waiting before retry"
                                );
                                sleep(wait).await;
                                continue;
                            }
                        }

                        tracing::warn!(
                            url = %url,
                            status = status.as_u16(),
                            attempt = attempt + 1,
                            backoff_ms = backoff.as_millis(),
                            "Retrying after retryable status"
                        );
                        sleep(backoff).await;
                        continue;
                    }

                    // Extract final_url before consuming the response body
                    let final_url = response.url().clone();

                    let body = if self.config.max_body_size > 0 {
                        let bytes = response.bytes().await.map_err(CrawlError::RequestFailed)?;
                        let limited = &bytes[..bytes.len().min(self.config.max_body_size)];
                        String::from_utf8_lossy(limited).to_string()
                    } else {
                        response.text().await.map_err(CrawlError::RequestFailed)?
                    };

                    return Ok((final_url, status, headers, body, elapsed));
                }
                Err(e) => {
                    if (e.is_timeout() || e.is_connect()) && attempt < max_retries {
                        let backoff = self.config.retry_policy.backoff_duration(attempt);
                        tracing::warn!(
                            url = %url,
                            error = %e,
                            attempt = attempt + 1,
                            backoff_ms = backoff.as_millis(),
                            "Retrying after network error"
                        );
                        sleep(backoff).await;
                        last_error = Some(CrawlError::RequestFailed(e));
                        continue;
                    }
                    return Err(CrawlError::RequestFailed(e));
                }
            }
        }

        Err(last_error.unwrap_or(CrawlError::MaxRetriesExceeded(max_retries)))
    }

    /// Returns a reference to the inner `reqwest::Client`.
    pub fn inner(&self) -> &Client {
        &self.client
    }

    /// Returns a reference to the client configuration.
    pub fn config(&self) -> &HttpClientConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_policy_default() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_retries, 3);
        assert_eq!(policy.initial_backoff, Duration::from_secs(1));
        assert_eq!(policy.max_backoff, Duration::from_secs(30));
        assert!((policy.backoff_multiplier - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_retry_policy_backoff_duration() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.backoff_duration(0), Duration::from_secs(1));
        assert_eq!(policy.backoff_duration(1), Duration::from_secs(2));
        assert_eq!(policy.backoff_duration(2), Duration::from_secs(4));
        assert_eq!(policy.backoff_duration(3), Duration::from_secs(8));
        // Capped at max_backoff
        assert_eq!(policy.backoff_duration(10), Duration::from_secs(30));
    }

    #[test]
    fn test_retry_policy_is_retryable() {
        let policy = RetryPolicy::default();
        assert!(policy.is_retryable(429));
        assert!(policy.is_retryable(500));
        assert!(policy.is_retryable(502));
        assert!(policy.is_retryable(503));
        assert!(policy.is_retryable(504));
        assert!(!policy.is_retryable(200));
        assert!(!policy.is_retryable(404));
    }

    #[test]
    fn test_user_agent_rotator() {
        let rotator = UserAgentRotator::new(vec![
            "agent-1".to_string(),
            "agent-2".to_string(),
            "agent-3".to_string(),
        ]);
        assert_eq!(rotator.len(), 3);
        assert!(!rotator.is_empty());
        assert_eq!(rotator.next(), "agent-1");
        assert_eq!(rotator.next(), "agent-2");
        assert_eq!(rotator.next(), "agent-3");
        assert_eq!(rotator.next(), "agent-1"); // wraps around
    }

    #[test]
    fn test_user_agent_rotator_default() {
        let rotator = UserAgentRotator::default();
        assert_eq!(rotator.len(), 1);
        let agent = rotator.next().to_string();
        assert!(agent.starts_with("crawlkit/"));
    }

    #[test]
    fn test_http_client_config_from_crawl_config() {
        let crawl_config = CrawlConfig::default();
        let http_config = HttpClientConfig::from(&crawl_config);
        assert_eq!(http_config.timeout, Duration::from_secs(30));
        assert_eq!(http_config.max_redirects, 20);
        assert_eq!(http_config.max_body_size, 10 * 1024 * 1024);
    }

    #[tokio::test]
    async fn test_http_client_creation() {
        let config = HttpClientConfig::from(&CrawlConfig::default());
        let client = HttpClient::new(config);
        assert!(client.is_ok());
    }
}
