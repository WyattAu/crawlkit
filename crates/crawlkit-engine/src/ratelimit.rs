//! Per-domain and global request throttling for the crawl engine.
//!
//! The limiter is a thin scheduler over the estate's [`ratelimit`] crate:
//! each bucket is a `ratelimit::GcraCell` (integer-microsecond GCRA state)
//! parameterized by `ratelimit::GcraParams`. The GCRA core is Lean-proven in
//! `ratelimit/proofs` (namespace `Ratelimit.Gcra`, THM-RATE-001..007): a
//! request conforms iff `now ≥ TAT − τ`, an admission advances
//! `TAT ← max(TAT, now) + T`, and the burst capacity is `b = 1 + ⌊τ/T⌋`
//! with `(b−1)·T ≤ τ < b·T`.
//!
//! Each domain gets its own cell stored in an LRU cache that evicts
//! least-recently-used domains when the capacity is reached. A global cell
//! throttles all requests across all domains. A request proceeds only when
//! both the domain and global cells conform. Supports crawl-delay from
//! robots.txt and optional concurrency limiting.
//!
//! # Examples
//!
//! ```rust
//! use crawlkit_engine::ratelimit::RateLimiter;
//!
//! let limiter = RateLimiter::new(2.0, 10.0);
//! assert!((limiter.per_domain_rps() - 2.0).abs() < f64::EPSILON);
//! ```
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::{Duration, Instant};

use lru::LruCache;
use parking_lot::Mutex;
use ratelimit::{GcraCell, GcraParams};
use tokio::sync::Semaphore;

use crate::CrawlConfig;

/// Monotonic microseconds since process start (the GCRA clock domain).
///
/// Anchored once so clock adjustments never move the TAT timeline; shared
/// by the per-domain and global cells so their states are comparable.
fn monotonic_us() -> u64 {
    use std::sync::OnceLock;
    static ANCHOR: OnceLock<Instant> = OnceLock::new();
    ANCHOR.get_or_init(Instant::now).elapsed().as_micros() as u64
}

/// GCRA rate limiter for per-domain and global request throttling.
///
/// Each domain gets its own GCRA cell stored in an LRU cache that
/// evicts least-recently-used domains when the capacity is reached.
/// A global cell throttles all requests across all domains. A request
/// proceeds only when both the domain and global cells conform. Supports
/// crawl-delay from robots.txt and optional concurrency limiting.
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
    /// Per-domain GCRA cells (LRU eviction when full).
    domain_buckets: Mutex<LruCache<String, GcraBucket>>,
    /// Global GCRA cell.
    global_bucket: Mutex<GcraBucket>,
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
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crawlkit_engine::ratelimit::RateLimiter;
    ///
    /// let limiter = RateLimiter::new(2.0, 10.0);
    /// assert!((limiter.per_domain_rps() - 2.0).abs() < f64::EPSILON);
    /// assert!((limiter.global_rps() - 10.0).abs() < f64::EPSILON);
    /// ```
    pub fn new(per_domain_rps: f64, global_rps: f64) -> Self {
        Self::with_max_domains(per_domain_rps, global_rps, 10_000)
    }

    /// Creates a rate limiter with a custom maximum number of tracked domains.
    ///
    /// When the cache is full and a new domain is accessed, the
    /// least-recently-used domain cell is evicted.
    pub fn with_max_domains(per_domain_rps: f64, global_rps: f64, max_domains: usize) -> Self {
        let per_domain_burst = (per_domain_rps * 2.0).ceil() as usize;
        let global_burst = (global_rps * 2.0).ceil() as usize;

        Self {
            domain_buckets: Mutex::new(LruCache::new(
                NonZeroUsize::new(max_domains).unwrap_or(NonZeroUsize::MIN),
            )),
            global_bucket: Mutex::new(GcraBucket::new(global_rps, global_burst)),
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
    /// Blocks until both the domain and global GCRA cells conform.
    /// Returns `Err` if the wait exceeds the timeout.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # #[tokio::main] async fn main() {
    /// use crawlkit_engine::ratelimit::RateLimiter;
    ///
    /// let limiter = RateLimiter::new(100.0, 1000.0);
    /// limiter.acquire("example.com").await.unwrap();
    /// # }
    /// ```
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

        // Wait for the domain cell to conform
        loop {
            {
                let mut buckets = self.domain_buckets.lock();
                let rps = self.per_domain_rps;
                let burst = self.per_domain_burst;
                let bucket =
                    buckets.get_or_insert_mut(domain.to_string(), || GcraBucket::new(rps, burst));

                if bucket.try_consume() {
                    break;
                }
            }

            if Instant::now() >= deadline {
                return Err(RateLimitError::Timeout);
            }

            // Sleep until the domain cell can next conform
            let wait = {
                let buckets = self.domain_buckets.lock();
                buckets
                    .peek(domain)
                    .map(GcraBucket::time_until_next_token)
                    .unwrap_or(Duration::from_millis(10))
            };

            let remaining = deadline.saturating_duration_since(Instant::now());
            let sleep_time = wait.min(remaining);
            tokio::time::sleep(sleep_time).await;
        }

        // Wait for the global cell to conform
        loop {
            // Acquire lock in a block to ensure it is dropped before any await point.
            // `parking_lot::Mutex` guards are `!Send`; holding them across an `.await`
            // would poison the future and violate tokio's Send requirement.
            let sleep_time = {
                let mut global = self.global_bucket.lock();

                if global.try_consume() {
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
    /// This overrides the default RPS for that domain by replacing its
    /// GCRA cell with a fresh full-burst budget.
    pub fn set_crawl_delay(&self, domain: &str, delay: Duration) {
        let rps = if delay.is_zero() {
            10.0
        } else {
            1.0 / delay.as_secs_f64()
        };
        let burst = (rps * 2.0).ceil() as usize;
        let mut buckets = self.domain_buckets.lock();
        buckets.put(domain.to_string(), GcraBucket::new(rps, burst));
    }

    /// Returns the current burst credits for a domain cell.
    /// Creates the cell if it doesn't exist.
    pub fn domain_tokens(&self, domain: &str) -> f64 {
        let mut buckets = self.domain_buckets.lock();
        let rps = self.per_domain_rps;
        let burst = self.per_domain_burst;
        let bucket = buckets.get_or_insert_mut(domain.to_string(), || GcraBucket::new(rps, burst));
        bucket.tokens()
    }

    /// Returns the current global burst credits.
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
    /// The wait timed out before the cells could conform.
    #[error("rate limit acquire timed out")]
    Timeout,
    /// The rate limiter has been closed.
    #[error("rate limiter closed")]
    Closed,
}

/// One GCRA bucket: a `ratelimit::GcraCell` plus its parameters.
///
/// Admission, wait-time, and credit accounting are all delegated to the
/// Lean-proven `ratelimit::gcra` core — this type only owns the mapping
/// from `(rps, burst)` config to GCRA parameters.
#[derive(Debug)]
struct GcraBucket {
    /// GCRA parameters: `T = 1/rps` µs, `τ = (burst−1)·T` (the integer
    /// configuration of the proofs). Burst capacity is derived from `τ`
    /// (`b = 1 + ⌊τ/T⌋`), so no separate field is stored.
    params: GcraParams,
    /// Integer-µs GCRA state (theoretical arrival time).
    cell: GcraCell,
}

impl GcraBucket {
    /// Creates a fresh bucket with a full burst budget.
    ///
    /// `burst` is clamped to at least 1 so a degenerate `rps` cannot mint a
    /// zero-capacity bucket.
    fn new(rps: f64, burst: usize) -> Self {
        let burst = (burst as u64).max(1);
        Self {
            params: GcraParams::from_rate(rps, burst),
            cell: GcraCell::new(monotonic_us()),
        }
    }

    /// Runs one GCRA step at `now_us`; `true` iff the request conforms.
    fn check_at(&mut self, now_us: u64) -> bool {
        self.cell.check(now_us, self.params).allowed
    }

    /// Tries to admit one request now.
    fn try_consume(&mut self) -> bool {
        self.check_at(monotonic_us())
    }

    /// Time until the cell can next conform.
    fn time_until_next_token(&self) -> Duration {
        Duration::from_micros(self.cell.retry_after_us(monotonic_us(), self.params))
    }

    /// Same-instant burst credits visible now, in `[0, burst]` — the
    /// GCRA analogue of the token count (`ratelimit::GcraCell::credits`).
    fn tokens(&self) -> f64 {
        self.cell.credits(monotonic_us(), self.params) as f64
    }
}

// Tests exercise delegation invariants directly; unwrap/expect,
// slicing, and panicking asserts are acceptable here — violations
// surface as test failures, not production panics.
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gcra_bucket_basic() {
        let mut bucket = GcraBucket::new(10.0, 10);
        assert!(bucket.try_consume());
        assert_eq!(bucket.tokens(), 9.0);
    }

    #[test]
    fn test_gcra_bucket_empty() {
        let mut bucket = GcraBucket::new(1.0, 1);
        assert!(bucket.try_consume());
        assert!(!bucket.try_consume()); // burst exhausted
    }

    #[test]
    fn test_gcra_bucket_recovers_after_elapsed() {
        let mut bucket = GcraBucket::new(100.0, 10);
        // Drain the burst (T = 10 ms, τ = 9·T: ten same-instant admissions).
        for _ in 0..10 {
            assert!(bucket.try_consume());
        }
        assert!(!bucket.try_consume());

        // Simulate 1.1 s passing by checking at an explicit future instant.
        let now = monotonic_us();
        assert!(bucket.check_at(now + 1_100_000));
    }

    #[test]
    fn test_gcra_bucket_burst() {
        let mut bucket = GcraBucket::new(5.0, 10);
        // Should allow a burst of 10
        for _ in 0..10 {
            assert!(bucket.try_consume());
        }
        assert!(!bucket.try_consume());
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
        // Cell is created lazily with burst = ceil(5.0 * 2.0) = 10
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
    fn test_bucket_time_until_next_token() {
        let bucket = GcraBucket::new(10.0, 5);
        assert_eq!(bucket.time_until_next_token(), Duration::ZERO);

        let mut bucket = GcraBucket::new(10.0, 1);
        bucket.try_consume();
        let wait = bucket.time_until_next_token();
        // Should be ~100ms (T = 1 / 10 rps, τ = 0)
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
                                                      // Consume the initial burst cell
        let _ = limiter
            .acquire_with_timeout("example.com", Duration::from_secs(1))
            .await;
        // Now the cell cannot conform for ~1000 s, should time out
        let result = limiter
            .acquire_with_timeout("example.com", Duration::from_millis(50))
            .await;
        assert!(matches!(result, Err(RateLimitError::Timeout)));
    }

    #[test]
    fn test_lru_eviction() {
        let limiter = RateLimiter::with_max_domains(10.0, 100.0, 3);

        // Fill the cache with 3 domains
        limiter.domain_tokens("a.com");
        limiter.domain_tokens("b.com");
        limiter.domain_tokens("c.com");

        // All 3 should be present
        {
            let buckets = limiter.domain_buckets.lock();
            assert_eq!(buckets.len(), 3);
        }

        // Access a.com to make it most recently used
        limiter.domain_tokens("a.com");

        // Insert d.com → evicts b.com (least recently used)
        limiter.domain_tokens("d.com");

        {
            let buckets = limiter.domain_buckets.lock();
            assert_eq!(buckets.len(), 3);
            assert!(buckets.peek("a.com").is_some());
            assert!(buckets.peek("b.com").is_none());
            assert!(buckets.peek("c.com").is_some());
            assert!(buckets.peek("d.com").is_some());
        }
    }

    #[test]
    fn test_max_domains_configurable() {
        let limiter = RateLimiter::with_max_domains(10.0, 100.0, 1);
        limiter.domain_tokens("first.com");
        limiter.domain_tokens("second.com");

        let buckets = limiter.domain_buckets.lock();
        assert_eq!(buckets.len(), 1);
        assert!(buckets.peek("second.com").is_some());
        assert!(buckets.peek("first.com").is_none());
    }

    #[test]
    fn test_lru_eviction_at_capacity() {
        let capacity = 5;
        let limiter = RateLimiter::with_max_domains(10.0, 100.0, capacity);

        // Fill to capacity
        for i in 0..capacity {
            limiter.domain_tokens(&format!("domain{i}.com"));
        }

        {
            let buckets = limiter.domain_buckets.lock();
            assert_eq!(buckets.len(), capacity);
        }

        // Access domain0 to make it most recently used
        limiter.domain_tokens("domain0.com");

        // Insert beyond capacity — should evict domain1 (LRU)
        limiter.domain_tokens("overflow.com");

        {
            let buckets = limiter.domain_buckets.lock();
            assert_eq!(buckets.len(), capacity);
            assert!(buckets.peek("domain0.com").is_some());
            assert!(buckets.peek("domain1.com").is_none());
            assert!(buckets.peek("overflow.com").is_some());
        }
    }

    #[test]
    fn test_acquire_updates_lru_order() {
        let limiter = RateLimiter::with_max_domains(1000.0, 10000.0, 3);

        limiter.domain_tokens("a.com");
        limiter.domain_tokens("b.com");
        limiter.domain_tokens("c.com");

        // Acquire on a.com promotes it in LRU order
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(limiter.acquire("a.com")).unwrap();

        // Insert d.com — evicts b.com (LRU)
        limiter.domain_tokens("d.com");

        let buckets = limiter.domain_buckets.lock();
        assert!(buckets.peek("a.com").is_some());
        assert!(buckets.peek("b.com").is_none());
        assert!(buckets.peek("c.com").is_some());
        assert!(buckets.peek("d.com").is_some());
    }
}
