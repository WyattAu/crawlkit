use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use parking_lot::Mutex;
use tokio::sync::Semaphore;

use crate::CrawlConfig;

/// Token-bucket rate limiter for per-domain and global request throttling.
///
/// Each domain gets its own token bucket. A global bucket throttles
/// all requests across all domains. A request proceeds only when both
/// the domain and global buckets have tokens. Supports crawl-delay
/// from robots.txt and optional concurrency limiting.
///
/// # Examples
///
/// ```rust
/// use crawlkit_engine::ratelimit::RateLimiter;
///
/// let limiter = RateLimiter::new(2.0, 10.0);
/// assert!((limiter.per_domain_rps() - 2.0).abs() < f64::EPSILON);
/// ```
pub struct RateLimiter {
    /// Per-domain token buckets.
    domain_buckets: DashMap<String, TokenBucket>,
    /// Global token bucket.
    global_bucket: Mutex<TokenBucket>,
    /// Default per-domain RPS (requests per second).
    per_domain_rps: f64,
    /// Global RPS limit.
    global_rps: f64,
    /// Maximum burst size per domain.
    per_domain_burst: usize,
    /// Maximum burst size globally.
    _global_burst: usize,
    /// Optional semaphore for concurrency limiting.
    concurrency_limit: Option<Arc<Semaphore>>,
}

impl RateLimiter {
    /// Creates a new rate limiter with the given RPS settings.
    pub fn new(per_domain_rps: f64, global_rps: f64) -> Self {
        let per_domain_burst = (per_domain_rps * 2.0).ceil() as usize;
        let global_burst = (global_rps * 2.0).ceil() as usize;

        Self {
            domain_buckets: DashMap::new(),
            global_bucket: Mutex::new(TokenBucket::new(global_rps, global_burst)),
            per_domain_rps,
            global_rps,
            per_domain_burst,
            _global_burst: global_burst,
            concurrency_limit: None,
        }
    }

    /// Creates a rate limiter from a `CrawlConfig`.
    pub fn from_crawl_config(config: &CrawlConfig) -> Self {
        let per_domain_rps = if config.request_delay.is_zero() {
            10.0
        } else {
            1.0 / config.request_delay.as_secs_f64()
        };
        // Global RPS = per-domain * concurrency as a rough default
        let global_rps = per_domain_rps * config.concurrency as f64;
        Self::new(per_domain_rps, global_rps)
    }

    /// Creates a rate limiter with a concurrency limit.
    pub fn with_concurrency(self, max_concurrent: usize) -> Self {
        Self {
            concurrency_limit: Some(Arc::new(Semaphore::new(max_concurrent))),
            ..self
        }
    }

    /// Acquires permission to make a request to the given domain.
    ///
    /// Blocks until tokens are available in both the domain and global
    /// buckets. Returns `Err` if the wait exceeds the timeout.
    pub async fn acquire(&self, domain: &str) -> Result<(), RateLimitError> {
        self.acquire_with_timeout(domain, Duration::from_secs(60))
            .await
    }

    /// Acquires permission with a timeout.
    pub async fn acquire_with_timeout(
        &self,
        domain: &str,
        timeout: Duration,
    ) -> Result<(), RateLimitError> {
        let deadline = Instant::now() + timeout;

        // Acquire concurrency permit if configured
        let _concurrency_permit = if let Some(ref sem) = self.concurrency_limit {
            let permit = Arc::clone(sem)
                .acquire_owned()
                .await
                .map_err(|_| RateLimitError::Closed)?;
            Some(permit)
        } else {
            None
        };

        // Wait for domain bucket token
        loop {
            {
                let mut bucket = self
                    .domain_buckets
                    .entry(domain.to_string())
                    .or_insert_with(|| {
                        TokenBucket::new(self.per_domain_rps, self.per_domain_burst)
                    });
                bucket.refill();

                if bucket.try_consume(1) {
                    break;
                }
            }

            if Instant::now() >= deadline {
                return Err(RateLimitError::Timeout);
            }

            // Sleep for the time until the next token is available
            let wait = self
                .domain_buckets
                .get(domain)
                .map(|b| b.time_until_next_token())
                .unwrap_or(Duration::from_millis(10));

            let remaining = deadline.saturating_duration_since(Instant::now());
            let sleep_time = wait.min(remaining);
            tokio::time::sleep(sleep_time).await;
        }

        // Wait for global bucket token
        loop {
            // Acquire lock in a block to ensure it is dropped before any await point.
            // `parking_lot::Mutex` guards are `!Send`; holding them across an `.await`
            // would poison the future and violate tokio's Send requirement.
            let sleep_time = {
                let mut global = self.global_bucket.lock();
                global.refill();

                if global.try_consume(1) {
                    break;
                }

                if Instant::now() >= deadline {
                    return Err(RateLimitError::Timeout);
                }

                let wait = global.time_until_next_token();
                let remaining = deadline.saturating_duration_since(Instant::now());
                wait.min(remaining)
                // Guard dropped here, before the await.
            };

            tokio::time::sleep(sleep_time).await;
        }

        Ok(())
    }

    /// Sets the crawl-delay for a specific domain (from robots.txt).
    ///
    /// This overrides the default RPS for that domain.
    pub fn set_crawl_delay(&self, domain: &str, delay: Duration) {
        let rps = if delay.is_zero() {
            10.0
        } else {
            1.0 / delay.as_secs_f64()
        };
        let burst = (rps * 2.0).ceil() as usize;
        self.domain_buckets
            .insert(domain.to_string(), TokenBucket::new(rps, burst));
    }

    /// Returns the current token count for a domain bucket.
    /// Creates the bucket if it doesn't exist.
    pub fn domain_tokens(&self, domain: &str) -> f64 {
        self.domain_buckets
            .entry(domain.to_string())
            .or_insert_with(|| TokenBucket::new(self.per_domain_rps, self.per_domain_burst))
            .tokens()
    }

    /// Returns the current global token count.
    pub fn global_tokens(&self) -> f64 {
        self.global_bucket.lock().tokens()
    }

    /// Returns the per-domain RPS setting.
    pub fn per_domain_rps(&self) -> f64 {
        self.per_domain_rps
    }

    /// Returns the global RPS setting.
    pub fn global_rps(&self) -> f64 {
        self.global_rps
    }
}

/// Errors that can occur during rate limiting.
#[derive(Debug, Clone, thiserror::Error)]
pub enum RateLimitError {
    /// The wait timed out before a token became available.
    #[error("rate limit acquire timed out")]
    Timeout,
    /// The rate limiter has been closed.
    #[error("rate limiter closed")]
    Closed,
}

/// Token bucket implementation for rate limiting.
#[derive(Debug)]
struct TokenBucket {
    /// Current tokens available.
    tokens: f64,
    /// Maximum tokens (burst size).
    max_tokens: f64,
    /// Refill rate in tokens per second.
    refill_rate: f64,
    /// Last time tokens were refilled.
    last_refill: Instant,
}

impl TokenBucket {
    /// Creates a new token bucket.
    fn new(rps: f64, burst: usize) -> Self {
        Self {
            tokens: burst as f64,
            max_tokens: burst as f64,
            refill_rate: rps,
            last_refill: Instant::now(),
        }
    }

    /// Refills tokens based on elapsed time since last refill.
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.max_tokens);
        self.last_refill = now;
    }

    /// Tries to consume `n` tokens. Returns `true` if successful.
    fn try_consume(&mut self, n: usize) -> bool {
        if self.tokens >= n as f64 {
            self.tokens -= n as f64;
            true
        } else {
            false
        }
    }

    /// Returns the time until the next token is available.
    fn time_until_next_token(&self) -> Duration {
        if self.tokens >= 1.0 {
            Duration::ZERO
        } else {
            let deficit = 1.0 - self.tokens;
            Duration::from_secs_f64(deficit / self.refill_rate)
        }
    }

    /// Returns the current number of tokens.
    fn tokens(&self) -> f64 {
        self.tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_bucket_basic() {
        let mut bucket = TokenBucket::new(10.0, 10);
        assert!(bucket.try_consume(1));
        assert_eq!(bucket.tokens(), 9.0);
    }

    #[test]
    fn test_token_bucket_empty() {
        let mut bucket = TokenBucket::new(1.0, 1);
        assert!(bucket.try_consume(1));
        assert!(!bucket.try_consume(1)); // empty
    }

    #[test]
    fn test_token_bucket_refill() {
        let mut bucket = TokenBucket::new(100.0, 10);
        // Drain
        for _ in 0..10 {
            bucket.try_consume(1);
        }
        assert!(!bucket.try_consume(1));

        // Simulate time passing by manipulating last_refill
        bucket.last_refill = Instant::now() - Duration::from_secs(1);
        bucket.refill();
        assert!(bucket.try_consume(1));
    }

    #[test]
    fn test_token_bucket_burst() {
        let mut bucket = TokenBucket::new(5.0, 10);
        // Should allow burst of 10
        for _ in 0..10 {
            assert!(bucket.try_consume(1));
        }
        assert!(!bucket.try_consume(1));
    }

    #[test]
    fn test_rate_limiter_creation() {
        let limiter = RateLimiter::new(2.0, 10.0);
        assert!((limiter.per_domain_rps() - 2.0).abs() < f64::EPSILON);
        assert!((limiter.global_rps() - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_rate_limiter_from_crawl_config() {
        let config = CrawlConfig::default();
        let limiter = RateLimiter::from_crawl_config(&config);
        // request_delay is 500ms → 2 RPS per domain
        assert!((limiter.per_domain_rps() - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_rate_limiter_domain_tokens() {
        let limiter = RateLimiter::new(5.0, 20.0);
        let tokens = limiter.domain_tokens("example.com");
        // Bucket is created lazily with burst = ceil(5.0 * 2.0) = 10
        assert!(
            (9.0..=11.0).contains(&tokens),
            "expected ~10.0, got {tokens}"
        );
    }

    #[test]
    fn test_rate_limiter_global_tokens() {
        let limiter = RateLimiter::new(5.0, 20.0);
        let tokens = limiter.global_tokens();
        // Should start at burst level (40)
        assert!((tokens - 40.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_set_crawl_delay() {
        let limiter = RateLimiter::new(5.0, 20.0);
        limiter.set_crawl_delay("example.com", Duration::from_secs(2));
        // 0.5 RPS, burst = 1
        let tokens = limiter.domain_tokens("example.com");
        assert!((tokens - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_token_bucket_time_until_next_token() {
        let bucket = TokenBucket::new(10.0, 5);
        assert_eq!(bucket.time_until_next_token(), Duration::ZERO);

        let mut bucket = TokenBucket::new(10.0, 1);
        bucket.try_consume(1);
        let wait = bucket.time_until_next_token();
        // Should be ~100ms (1 token / 10 tokens per second)
        assert!(wait > Duration::from_millis(90));
        assert!(wait <= Duration::from_millis(110));
    }

    #[tokio::test]
    async fn test_rate_limiter_acquire() {
        let limiter = RateLimiter::new(100.0, 1000.0); // high RPS for fast test
        let result = limiter
            .acquire_with_timeout("example.com", Duration::from_secs(1))
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_rate_limiter_acquire_timeout() {
        let limiter = RateLimiter::new(0.001, 0.001); // very low RPS
                                                      // Consume the initial burst token
        let _ = limiter
            .acquire_with_timeout("example.com", Duration::from_secs(1))
            .await;
        // Now the bucket is empty, should time out
        let result = limiter
            .acquire_with_timeout("example.com", Duration::from_millis(50))
            .await;
        assert!(matches!(result, Err(RateLimitError::Timeout)));
    }
}
