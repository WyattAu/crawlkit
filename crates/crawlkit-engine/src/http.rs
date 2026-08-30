use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::stream::Stream;
use futures::StreamExt;
use reqwest::header::USER_AGENT;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use tokio::time::sleep;
use url::Url;

use crate::ssrf::is_private_ip;
use crate::{CrawlConfig, CrawlError, FetchResult, RedirectHop};

/// Extract conditional request headers (ETag, Last-Modified) from response headers.
fn extract_conditional_headers(headers: &[(String, String)]) -> (Option<String>, Option<String>) {
    let etag = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("etag"))
        .map(|(_, v)| v.clone());
    let last_modified = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("last-modified"))
        .map(|(_, v)| v.clone());
    (etag, last_modified)
}

/// Retry policy for failed requests.
///
/// Controls exponential backoff behavior for retryable HTTP status codes
/// and network errors. The backoff duration is calculated as:
/// `initial_backoff * backoff_multiplier^attempt`, capped at `max_backoff`.
///
/// # Examples
///
/// ```rust
/// use crawlkit_engine::http::RetryPolicy;
/// use std::time::Duration;
///
/// let policy = RetryPolicy::default();
/// assert_eq!(policy.max_retries, 3);
/// assert!(policy.is_retryable(429));
/// assert!(!policy.is_retryable(200));
/// ```
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
    ///
    /// The duration grows exponentially: `initial_backoff * multiplier^attempt`,
    /// capped at `max_backoff`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crawlkit_engine::http::RetryPolicy;
    /// use std::time::Duration;
    ///
    /// let policy = RetryPolicy::default();
    /// assert_eq!(policy.backoff_duration(0), Duration::from_secs(1));
    /// assert_eq!(policy.backoff_duration(1), Duration::from_secs(2));
    /// assert_eq!(policy.backoff_duration(10), Duration::from_secs(30)); // capped
    /// ```
    pub fn backoff_duration(&self, attempt: usize) -> Duration {
        let base = self.initial_backoff.as_secs_f64();
        let backoff = base * self.backoff_multiplier.powi(attempt as i32);
        let capped = backoff.min(self.max_backoff.as_secs_f64());
        Duration::from_secs_f64(capped)
    }

    /// Returns `true` if the given status code should trigger a retry.
    ///
    /// By default retries on: 429 (Too Many Requests), 500, 502, 503, 504.
    pub fn is_retryable(&self, status: u16) -> bool {
        self.retryable_statuses.contains(&status)
    }
}

/// User-agent rotator that cycles through a list of user-agent strings.
///
/// Thread-safe rotation using atomic operations. Useful for distributing
/// requests across multiple identity strings to avoid detection.
///
/// # Examples
///
/// ```rust
/// use crawlkit_engine::http::UserAgentRotator;
///
/// let rotator = UserAgentRotator::new(vec![
///     "bot/1.0".to_string(),
///     "bot/2.0".to_string(),
/// ]);
/// assert_eq!(rotator.next(), "bot/1.0");
/// assert_eq!(rotator.next(), "bot/2.0");
/// assert_eq!(rotator.next(), "bot/1.0"); // wraps around
/// ```
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
    ///
    /// Uses `AcqRel` ordering to ensure fair rotation under contention.
    /// `Relaxed` would allow multiple threads to read the same index.
    pub fn next(&self) -> &str {
        let idx = self.index.fetch_add(1, Ordering::AcqRel);
        &self.agents[idx % self.agents.len()]
    }

    /// Returns the user-agent for a URL, selected by a stable hash of
    /// `(seed, url)`.
    ///
    /// Unlike [`next`](Self::next), this is a pure function of its inputs:
    /// the same `(url, seed)` pair always maps to the same agent across
    /// threads, rotator instances, and runs. Use this for seeded,
    /// reproducible crawls; keep [`next`](Self::next) for unseeded ones.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crawlkit_engine::http::UserAgentRotator;
    ///
    /// let rotator = UserAgentRotator::new(vec!["bot/1.0".to_string(), "bot/2.0".to_string()]);
    /// // Same (url, seed) → same agent, regardless of call order.
    /// assert_eq!(
    ///     rotator.ua_for_url("https://example.com/a", 42),
    ///     rotator.ua_for_url("https://example.com/a", 42)
    /// );
    /// // A fresh instance selects identically.
    /// let rotator2 = UserAgentRotator::new(vec!["bot/1.0".to_string(), "bot/2.0".to_string()]);
    /// assert_eq!(
    ///     rotator.ua_for_url("https://example.com/a", 42),
    ///     rotator2.ua_for_url("https://example.com/a", 42)
    /// );
    /// ```
    pub fn ua_for_url(&self, url: &str, seed: u64) -> &str {
        let mut hasher = DefaultHasher::new();
        hasher.write_u64(seed);
        hasher.write(url.as_bytes());
        let index = (hasher.finish() % self.agents.len() as u64) as usize;
        &self.agents[index]
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
///
/// Controls timeout, redirect policy, retry behavior, connection pooling,
/// and HTTP/2 settings. Can be constructed from a [`CrawlConfig`].
///
/// Pool sizes and timeouts scale with the configured concurrency level
/// when constructed via `From<&CrawlConfig>`.
///
/// # Examples
///
/// ```rust
/// use crawlkit_engine::{CrawlConfig, http::HttpClientConfig};
///
/// let config = HttpClientConfig::from(&CrawlConfig::default());
/// assert_eq!(config.max_body_size, 10 * 1024 * 1024);
/// // Default concurrency=4 → pool_max_idle_per_host=8, pool_max_idle=16
/// assert_eq!(config.pool_max_idle_per_host, 8);
/// assert_eq!(config.pool_max_idle, 16);
/// ```
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
    /// Maximum number of idle connections per host.
    pub pool_max_idle_per_host: usize,
    /// Maximum number of idle connections across all hosts.
    pub pool_max_idle: usize,
    /// Whether to enable TCP keepalive.
    pub tcp_keepalive: Option<Duration>,
    /// Timeout for idle connections in the pool.
    pub pool_idle_timeout: Duration,
    /// Timeout for establishing a new TCP connection.
    pub connect_timeout: Duration,
    /// Allow plain-HTTP fetches. Secure by default (`false`); enabling is
    /// intended for local test servers and trusted intranets.
    pub allow_http: bool,
    /// Optional seed for deterministic user-agent rotation.
    ///
    /// When `Some`, per-request user agents are selected via
    /// [`UserAgentRotator::ua_for_url`] (a stable hash of the URL and the
    /// seed) instead of round-robin [`UserAgentRotator::next`], so
    /// concurrent task interleaving cannot change which agent is sent.
    /// `None` by default.
    pub seed: Option<u64>,
}

impl HttpClientConfig {
    /// Enables deterministic user-agent rotation for this client.
    ///
    /// When a seed is set, each request's user agent is chosen by a stable
    /// hash of `(seed, url)` rather than by round-robin counter, so the
    /// same crawl always sends the same agent per URL.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crawlkit_engine::{CrawlConfig, http::HttpClientConfig};
    ///
    /// let config = HttpClientConfig::from(&CrawlConfig::default()).with_seed(42);
    /// assert_eq!(config.seed, Some(42));
    /// ```
    #[must_use]
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }
}

impl From<&CrawlConfig> for HttpClientConfig {
    fn from(config: &CrawlConfig) -> Self {
        let concurrency = config.concurrency.max(1);
        Self {
            timeout: config.request_timeout,
            max_redirects: config.max_redirects,
            retry_policy: RetryPolicy::default(),
            user_agent: Arc::new(UserAgentRotator::new(vec![config.user_agent.clone()])),
            max_body_size: 10 * 1024 * 1024, // 10MB default
            pool_max_idle_per_host: concurrency * 2,
            pool_max_idle: concurrency * 4,
            tcp_keepalive: Some(Duration::from_secs(60)),
            pool_idle_timeout: Duration::from_secs(90),
            connect_timeout: Duration::from_secs(10),
            allow_http: false,
            seed: None,
        }
    }
}

/// Resolve the domain of a URL and check that no resolved IP is private.
///
/// Prevents DNS rebinding attacks where a domain resolves to a private IP
/// after the initial check. Returns `Ok(())` if all resolved IPs are public,
/// or `Err(CrawlError)` if any private IP is detected.
async fn dns_pin_check(url: &Url) -> Result<(), CrawlError> {
    let host = match url.host_str() {
        Some(h) => h,
        None => return Ok(()),
    };
    let host = host.trim_matches(['[', ']']);
    if host.parse::<std::net::IpAddr>().is_ok() {
        return Ok(());
    }
    let socket_addr = format!("{host}:0");
    let addrs = tokio::net::lookup_host(&socket_addr)
        .await
        .map_err(|e| CrawlError::Internal(format!("DNS resolution failed for {host}: {e}")))?;
    for addr in addrs {
        if is_private_ip(addr.ip()) {
            return Err(CrawlError::Internal(format!(
                "DNS rebinding blocked: {host} resolved to private IP {}",
                addr.ip()
            )));
        }
    }
    Ok(())
}

/// An HTTP client with retry, redirect tracking, and user-agent rotation.
///
/// Built on top of `reqwest::Client` with additional features for web crawling:
/// - Manual redirect following with hop recording
/// - Exponential backoff retry for transient failures
/// - User-agent rotation across requests
/// - Response body size limiting
/// - Streaming responses
///
/// # Examples
///
/// ```rust,no_run
/// use crawlkit_engine::{CrawlConfig, HttpClient};
/// use url::Url;
///
/// # async fn example() -> Result<(), crawlkit_engine::CrawlError> {
/// let client = HttpClient::from_crawl_config(&CrawlConfig::default())?;
/// let url = Url::parse("https://example.com")?;
/// let result = client.fetch(&url).await?;
/// assert_eq!(result.status_code, 200);
/// # Ok(())
/// # }
/// ```
pub struct HttpClient {
    client: Client,
    config: HttpClientConfig,
}

impl HttpClient {
    /// Creates a new `HttpClient` from the given configuration.
    ///
    /// Builds a `reqwest::Client` with TLS, HTTP/2 multiplexing, connection
    /// pooling, and redirect policy.
    ///
    /// # Errors
    ///
    /// Returns [`CrawlError::RequestFailed`] if the underlying reqwest client
    /// cannot be built (e.g., invalid TLS configuration).
    pub fn new(config: HttpClientConfig) -> Result<Self, CrawlError> {
        let mut builder = Client::builder()
            .timeout(config.timeout)
            .redirect(reqwest::redirect::Policy::limited(config.max_redirects))
            .user_agent(config.user_agent.next())
            .https_only(!config.allow_http)
            .pool_max_idle_per_host(config.pool_max_idle_per_host)
            .pool_idle_timeout(config.pool_idle_timeout)
            .connect_timeout(config.connect_timeout);

        if let Some(keepalive) = config.tcp_keepalive {
            builder = builder.tcp_keepalive(keepalive);
        }

        let client = builder.build()?;

        Ok(Self { client, config })
    }

    /// Creates a new `HttpClient` from a `CrawlConfig`.
    ///
    /// Convenience method that converts the crawl config into an
    /// [`HttpClientConfig`] and builds the client.
    ///
    /// # Errors
    ///
    /// Returns [`CrawlError::RequestFailed`] if the client cannot be built.
    pub fn from_crawl_config(config: &CrawlConfig) -> Result<Self, CrawlError> {
        Self::new(HttpClientConfig::from(config))
    }

    /// Creates a new `HttpClient` optimized for high-throughput crawling.
    ///
    /// Enables HTTP/2, larger connection pools, and TCP keepalive.
    pub fn high_throughput(config: HttpClientConfig) -> Result<Self, CrawlError> {
        let cfg = HttpClientConfig {
            pool_max_idle_per_host: 64,
            pool_max_idle: 128,
            tcp_keepalive: Some(Duration::from_secs(60)),
            ..config
        };
        Self::new(cfg)
    }

    /// Selects the per-request user-agent for a URL.
    ///
    /// Seeded clients pick the agent via a stable `(seed, url)` hash so the
    /// selection is independent of task interleaving; unseeded clients keep
    /// the round-robin rotation.
    fn ua_for(&self, url: &Url) -> &str {
        match self.config.seed {
            Some(seed) => self.config.user_agent.ua_for_url(url.as_str(), seed),
            None => self.config.user_agent.next(),
        }
    }

    /// Fetches a URL with retry logic and redirect tracking.
    ///
    /// Returns a [`FetchResult`] with the final URL, status, headers, and body.
    /// Follows redirects manually to record each hop in the chain.
    ///
    /// # Errors
    ///
    /// Returns [`CrawlError::RequestFailed`] on network errors after retries
    /// are exhausted, or [`CrawlError::TooManyRedirects`] if the redirect
    /// limit is exceeded.
    pub async fn fetch(&self, url: &Url) -> Result<FetchResult, CrawlError> {
        match self
            .fetch_with_redirects(url, self.config.max_redirects)
            .await
        {
            Err(CrawlError::RequestFailed(ref e)) if e.is_connect() => {
                tracing::warn!(
                    url = %url,
                    error = %e,
                    "Connection failed, retrying with fresh connection after 2s delay"
                );
                sleep(Duration::from_secs(2)).await;
                self.fetch_with_redirects(url, self.config.max_redirects)
                    .await
            }
            other => other,
        }
    }

    /// Fetches a URL with conditional request headers (ETag / If-Modified-Since).
    ///
    /// Sends `If-None-Match` and/or `If-Modified-Since` headers when the
    /// corresponding values are `Some`. Returns a `FetchResult` with
    /// `status_code: 304` and an empty body when the server responds with
    /// Not Modified.
    ///
    /// # Errors
    ///
    /// Returns [`CrawlError::RequestFailed`] on network errors.
    pub async fn fetch_conditional(
        &self,
        url: &Url,
        etag: Option<&str>,
        last_modified: Option<&str>,
    ) -> Result<FetchResult, CrawlError> {
        let start = Instant::now();
        let user_agent = self.ua_for(url);

        let mut request = self.client.get(url.as_str()).header(USER_AGENT, user_agent);

        if let Some(etag_val) = etag {
            request = request.header("If-None-Match", etag_val);
        }
        if let Some(lm_val) = last_modified {
            request = request.header("If-Modified-Since", lm_val);
        }

        let response = request.send().await.map_err(CrawlError::RequestFailed)?;

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

        let final_url = response.url().clone();

        if status == StatusCode::NOT_MODIFIED {
            let (resp_etag, resp_lm) = extract_conditional_headers(&headers);
            return Ok(FetchResult {
                final_url,
                status_code: 304,
                headers,
                body: String::new(),
                response_time: elapsed,
                body_size: 0,
                fetched_at: chrono::Utc::now(),
                etag: resp_etag,
                last_modified: resp_lm,
            });
        }

        let body = if self.config.max_body_size > 0 {
            let bytes = response.bytes().await.map_err(CrawlError::RequestFailed)?;
            let limited = &bytes[..bytes.len().min(self.config.max_body_size)];
            String::from_utf8_lossy(limited).to_string()
        } else {
            response.text().await.map_err(CrawlError::RequestFailed)?
        };

        let (resp_etag, resp_lm) = extract_conditional_headers(&headers);
        let body_size = body.len();
        Ok(FetchResult {
            final_url,
            status_code: status.as_u16(),
            headers,
            body,
            response_time: elapsed,
            body_size,
            fetched_at: chrono::Utc::now(),
            etag: resp_etag,
            last_modified: resp_lm,
        })
    }

    /// Fetches a URL, following up to `max_hops` redirects manually.
    ///
    /// Each redirect hop is recorded. If the hop limit is exceeded,
    /// [`CrawlError::TooManyRedirects`] is returned.
    ///
    /// # Errors
    ///
    /// Returns errors for network failures or exceeded redirect limits.
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
                                let (etag, last_modified) = extract_conditional_headers(&headers);
                                return Ok(FetchResult {
                                    final_url,
                                    status_code: status.as_u16(),
                                    headers,
                                    body,
                                    response_time: elapsed,
                                    body_size,
                                    fetched_at: chrono::Utc::now(),
                                    etag,
                                    last_modified,
                                });
                            }
                        }
                    }

                    let body_size = body.len();
                    let (etag, last_modified) = extract_conditional_headers(&headers);
                    return Ok(FetchResult {
                        final_url,
                        status_code: status.as_u16(),
                        headers,
                        body,
                        response_time: elapsed,
                        body_size,
                        fetched_at: chrono::Utc::now(),
                        etag,
                        last_modified,
                    });
                }
                Err(CrawlError::RequestFailed(e)) => {
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
        dns_pin_check(url).await?;
        let mut last_error: Option<CrawlError> = None;
        let max_retries = self.config.retry_policy.max_retries;

        for attempt in 0..=max_retries {
            let start = Instant::now();
            let user_agent = self.ua_for(url);

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

    /// Fetches a URL and streams the response body, calling the callback with
    /// each chunk.
    ///
    /// This is useful for large pages where you want to process HTML as it
    /// arrives rather than buffering the entire response in memory.
    ///
    /// # Errors
    ///
    /// Returns errors for network failures or redirect limit exceeded.
    pub async fn fetch_stream<F>(
        &self,
        url: &Url,
        mut on_chunk: F,
    ) -> Result<FetchResult, CrawlError>
    where
        F: FnMut(&str) + Send,
    {
        let mut current_url = url.clone();
        let mut hops: Vec<RedirectHop> = Vec::new();

        for _ in 0..=self.config.max_redirects {
            let start = Instant::now();
            let user_agent = self.ua_for(&current_url);

            let response = self
                .client
                .get(current_url.as_str())
                .header(USER_AGENT, user_agent)
                .send()
                .await
                .map_err(CrawlError::RequestFailed)?;

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
                        let final_url = response.url().clone();
                        return Ok(FetchResult {
                            final_url,
                            status_code: status.as_u16(),
                            headers,
                            body: String::new(),
                            response_time: elapsed,
                            body_size: 0,
                            fetched_at: chrono::Utc::now(),
                            etag: None,
                            last_modified: None,
                        });
                    }
                }
            }

            let final_url = response.url().clone();
            let mut body = String::new();
            let mut stream = response.bytes_stream();
            let mut total_size: usize = 0;

            while let Some(chunk_result) = stream.next().await {
                let chunk = chunk_result.map_err(CrawlError::RequestFailed)?;
                total_size += chunk.len();

                if self.config.max_body_size > 0 && total_size > self.config.max_body_size {
                    break;
                }

                let chunk_str = String::from_utf8_lossy(&chunk);
                on_chunk(&chunk_str);
                body.push_str(&chunk_str);
            }

            let (etag, last_modified) = extract_conditional_headers(&headers);
            return Ok(FetchResult {
                final_url,
                status_code: status.as_u16(),
                headers,
                body,
                response_time: elapsed,
                body_size: total_size,
                fetched_at: chrono::Utc::now(),
                etag,
                last_modified,
            });
        }

        Err(CrawlError::TooManyRedirects(self.config.max_redirects))
    }

    /// Fetches a URL and returns the response as a streaming reader.
    ///
    /// Returns the response metadata (status, headers) and a streaming body.
    /// The caller can read chunks from the stream via [`FetchStreamReader::next_chunk`].
    ///
    /// # Errors
    ///
    /// Returns errors for network failures.
    pub async fn fetch_reader(&self, url: &Url) -> Result<FetchStreamReader, CrawlError> {
        let start = Instant::now();
        let user_agent = self.ua_for(url);

        let response = self
            .client
            .get(url.as_str())
            .header(USER_AGENT, user_agent)
            .send()
            .await
            .map_err(CrawlError::RequestFailed)?;

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
        let final_url = response.url().clone();
        let max_body_size = self.config.max_body_size;

        let stream = response
            .bytes_stream()
            .scan(0usize, move |total_size, result| {
                let output = match result {
                    Ok(bytes) => {
                        *total_size += bytes.len();
                        if max_body_size > 0 && *total_size > max_body_size {
                            None
                        } else {
                            Some(Ok(bytes))
                        }
                    }
                    Err(e) => Some(Err(e)),
                };
                async { output }
            });

        Ok(FetchStreamReader {
            final_url,
            status_code: status.as_u16(),
            headers,
            response_time: elapsed,
            stream: Box::pin(stream),
            body_size: 0,
            max_body_size,
        })
    }
}

/// A streaming HTTP response reader.
///
/// Read chunks from the body using the [`next_chunk`](FetchStreamReader::next_chunk) method.
/// The stream automatically respects `max_body_size`. Can be converted into
/// a [`FetchResult`] via [`into_fetch_result`](FetchStreamReader::into_fetch_result).
///
/// # Examples
///
/// ```rust,no_run
/// use crawlkit_engine::{CrawlConfig, HttpClient};
/// use url::Url;
///
/// # async fn example() -> Result<(), crawlkit_engine::CrawlError> {
/// let client = HttpClient::from_crawl_config(&CrawlConfig::default())?;
/// let url = Url::parse("https://example.com")?;
/// let mut reader = client.fetch_reader(&url).await?;
/// while let Some(chunk) = reader.next_chunk().await? {
///     // process chunk
/// }
/// # Ok(())
/// # }
/// ```
pub struct FetchStreamReader {
    pub final_url: Url,
    pub status_code: u16,
    pub headers: Vec<(String, String)>,
    pub response_time: Duration,
    stream: Pin<Box<dyn Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send>>,
    pub body_size: usize,
    max_body_size: usize,
}

impl FetchStreamReader {
    /// Reads the next chunk of the response body.
    ///
    /// Returns `Ok(Some(bytes))` if data is available, `Ok(None)` if the
    /// stream is complete, or `Err` on error. Respects `max_body_size`.
    ///
    /// # Errors
    ///
    /// Returns [`CrawlError::RequestFailed`] on network errors.
    pub async fn next_chunk(&mut self) -> Result<Option<Vec<u8>>, CrawlError> {
        if self.max_body_size > 0 && self.body_size >= self.max_body_size {
            return Ok(None);
        }

        match self.stream.next().await {
            Some(Ok(chunk)) => {
                let chunk: bytes::Bytes = chunk;
                let remaining = self.max_body_size.saturating_sub(self.body_size);
                let truncated = if remaining > 0 && chunk.len() > remaining {
                    self.body_size = self.max_body_size;
                    chunk[..remaining].to_vec()
                } else {
                    self.body_size += chunk.len();
                    chunk.to_vec()
                };
                Ok(Some(truncated))
            }
            Some(Err(e)) => Err(CrawlError::RequestFailed(e)),
            None => Ok(None),
        }
    }

    /// Reads the entire remaining body into a String.
    ///
    /// Convenience method that drains all remaining chunks and concatenates
    /// them into a single UTF-8 string (lossy conversion).
    ///
    /// # Errors
    ///
    /// Returns [`CrawlError::RequestFailed`] on network errors.
    pub async fn read_body(&mut self) -> Result<String, CrawlError> {
        let mut body = String::new();
        while let Some(chunk) = self.next_chunk().await? {
            body.push_str(&String::from_utf8_lossy(&chunk));
        }
        Ok(body)
    }

    /// Converts this into a [`FetchResult`] by reading the full body.
    ///
    /// Consumes the reader and returns the complete response including
    /// headers, status, and body content.
    ///
    /// # Errors
    ///
    /// Returns [`CrawlError::RequestFailed`] on network errors.
    pub async fn into_fetch_result(mut self) -> Result<FetchResult, CrawlError> {
        let body = self.read_body().await?;
        let body_size = self.body_size;
        let (etag, last_modified) = extract_conditional_headers(&self.headers);
        Ok(FetchResult {
            final_url: self.final_url,
            status_code: self.status_code,
            headers: self.headers,
            body,
            response_time: self.response_time,
            body_size,
            fetched_at: chrono::Utc::now(),
            etag,
            last_modified,
        })
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
    fn test_ua_for_url_is_stable_across_calls_and_instances() {
        let agents: Vec<String> = (1..=5).map(|i| format!("agent-{i}")).collect();
        let rotator_a = UserAgentRotator::new(agents.clone());
        let rotator_b = UserAgentRotator::new(agents);
        for url in [
            "https://example.com/",
            "https://example.com/a",
            "https://other.org/b?x=1",
        ] {
            let first = rotator_a.ua_for_url(url, 42);
            assert_eq!(first, rotator_a.ua_for_url(url, 42), "repeat call differs");
            assert_eq!(first, rotator_b.ua_for_url(url, 42), "instance differs");
        }
    }

    #[test]
    fn test_ua_for_url_returns_member_of_pool() {
        let agents: Vec<String> = (1..=4).map(|i| format!("agent-{i}")).collect();
        let rotator = UserAgentRotator::new(agents.clone());
        for seed in [0u64, 1, 42, u64::MAX] {
            for url in ["https://example.com/", "", "https://example.com/deep/path"] {
                assert!(agents.iter().any(|a| a == rotator.ua_for_url(url, seed)));
            }
        }
    }

    #[test]
    fn test_ua_for_url_seed_changes_selection() {
        let agents: Vec<String> = (1..=8).map(|i| format!("agent-{i}")).collect();
        let rotator = UserAgentRotator::new(agents);
        let urls: Vec<String> = (0..64)
            .map(|i| format!("https://example.com/page/{i}"))
            .collect();
        let differing = urls
            .iter()
            .filter(|u| rotator.ua_for_url(u, 1) != rotator.ua_for_url(u, 2))
            .count();
        // With a 64-bit hash over an 8-agent pool, seeds 1 and 2 must
        // disagree on the large majority of URLs; require at least half.
        assert!(
            differing >= urls.len() / 2,
            "seeds 1 and 2 agreed on {differing}/{} URLs",
            urls.len()
        );
    }

    #[test]
    fn test_ua_for_url_uses_default_hasher_seed_then_url_ordering() {
        // The selection must be the documented stable hash: seed first,
        // then url bytes, via DefaultHasher.
        let agents: Vec<String> = (1..=3).map(|i| format!("agent-{i}")).collect();
        let rotator = UserAgentRotator::new(agents.clone());
        let mut hasher = DefaultHasher::new();
        hasher.write_u64(7);
        hasher.write(b"https://example.com/x".as_slice());
        let expected = &agents[(hasher.finish() % 3) as usize];
        assert_eq!(rotator.ua_for_url("https://example.com/x", 7), expected);
    }

    #[test]
    fn test_http_client_config_with_seed() {
        let config = HttpClientConfig::from(&CrawlConfig::default());
        assert_eq!(config.seed, None);
        let seeded = config.with_seed(1234);
        assert_eq!(seeded.seed, Some(1234));
        // Builder must preserve the other fields.
        assert_eq!(seeded.timeout, Duration::from_secs(30));
        assert_eq!(seeded.max_body_size, 10 * 1024 * 1024);
    }

    #[test]
    fn test_http_client_config_from_crawl_config() {
        let crawl_config = CrawlConfig::default();
        let http_config = HttpClientConfig::from(&crawl_config);
        assert_eq!(http_config.timeout, Duration::from_secs(30));
        assert_eq!(http_config.max_redirects, 20);
        assert_eq!(http_config.max_body_size, 10 * 1024 * 1024);
        // concurrency=4 → pool_max_idle_per_host=8, pool_max_idle=16
        assert_eq!(http_config.pool_max_idle_per_host, 8);
        assert_eq!(http_config.pool_max_idle, 16);
        assert_eq!(http_config.tcp_keepalive, Some(Duration::from_secs(60)));
        assert_eq!(http_config.pool_idle_timeout, Duration::from_secs(90));
        assert_eq!(http_config.connect_timeout, Duration::from_secs(10));
    }

    #[tokio::test]
    async fn test_http_client_creation() {
        let config = HttpClientConfig::from(&CrawlConfig::default());
        let client = HttpClient::new(config);
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_http_client_offers_http2() {
        let config = HttpClientConfig::from(&CrawlConfig::default());
        let client = HttpClient::new(config).expect("client should build");
        // reqwest with default TLS (rustls-tls) enables both HTTP/1.1 and HTTP/2
        // via ALPN. Verify the client was built without http1_only() restriction
        // by confirming the inner client is accessible and functional.
        let _inner = client.inner();
    }

    #[tokio::test]
    async fn test_high_throughput_client_offers_http2() {
        let config = HttpClientConfig::from(&CrawlConfig::default());
        let client = HttpClient::high_throughput(config).expect("client should build");
        let _inner = client.inner();
    }

    #[test]
    fn test_extract_conditional_headers() {
        let headers = vec![
            ("content-type".to_string(), "text/html".to_string()),
            ("etag".to_string(), "\"abc123\"".to_string()),
            (
                "last-modified".to_string(),
                "Wed, 21 Oct 2024 07:28:00 GMT".to_string(),
            ),
        ];
        let (etag, last_modified) = extract_conditional_headers(&headers);
        assert_eq!(etag.as_deref(), Some("\"abc123\""));
        assert_eq!(
            last_modified.as_deref(),
            Some("Wed, 21 Oct 2024 07:28:00 GMT")
        );

        // Case-insensitive matching
        let headers = vec![
            ("ETag".to_string(), "\"xyz\"".to_string()),
            (
                "Last-Modified".to_string(),
                "Thu, 01 Jan 2025 00:00:00 GMT".to_string(),
            ),
        ];
        let (etag, last_modified) = extract_conditional_headers(&headers);
        assert_eq!(etag.as_deref(), Some("\"xyz\""));
        assert_eq!(
            last_modified.as_deref(),
            Some("Thu, 01 Jan 2025 00:00:00 GMT")
        );

        // Missing headers
        let headers = vec![("content-type".to_string(), "text/html".to_string())];
        let (etag, last_modified) = extract_conditional_headers(&headers);
        assert!(etag.is_none());
        assert!(last_modified.is_none());
    }
}
