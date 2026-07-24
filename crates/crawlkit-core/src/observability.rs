use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Observability metrics for crawl operations.
///
/// Thread-safe metrics collection using atomic operations.
/// Designed for zero-allocation in hot paths. All counters use
/// `Relaxed` ordering for maximum throughput.
///
/// # Examples
///
/// ```rust
/// use crawlkit_core::Metrics;
/// use std::time::Duration;
///
/// let metrics = Metrics::new();
/// metrics.record_page_success(1024, 100, 50, 10, 3);
/// assert_eq!(metrics.pages_crawled.load(std::sync::atomic::Ordering::Relaxed), 1);
/// ```
pub struct Metrics {
    /// Total pages crawled.
    pub pages_crawled: AtomicU64,
    /// Total pages failed.
    pub pages_failed: AtomicU64,
    /// Total findings generated.
    pub findings_generated: AtomicU64,
    /// Total bytes fetched.
    pub bytes_fetched: AtomicU64,
    /// Total fetch time (microseconds).
    pub fetch_time_us: AtomicU64,
    /// Total analysis time (microseconds).
    pub analysis_time_us: AtomicU64,
    /// Total storage write time (microseconds).
    pub storage_time_us: AtomicU64,
    /// Active connections.
    pub active_connections: AtomicU64,
    /// Total circuit breaker trips (transitions to Open state).
    pub circuit_breaker_trips: AtomicU64,
    /// Total resource limit hits.
    pub resource_limit_hits: AtomicU64,
    /// Pages skipped due to circuit breaker being open.
    pub pages_skipped_circuit_breaker: AtomicU64,
}

impl Metrics {
    /// Create new metrics instance.
    #[must_use]
    pub fn new() -> Self {
        Self {
            pages_crawled: AtomicU64::new(0),
            pages_failed: AtomicU64::new(0),
            findings_generated: AtomicU64::new(0),
            bytes_fetched: AtomicU64::new(0),
            fetch_time_us: AtomicU64::new(0),
            analysis_time_us: AtomicU64::new(0),
            storage_time_us: AtomicU64::new(0),
            active_connections: AtomicU64::new(0),
            circuit_breaker_trips: AtomicU64::new(0),
            resource_limit_hits: AtomicU64::new(0),
            pages_skipped_circuit_breaker: AtomicU64::new(0),
        }
    }

    /// Record a successful page crawl.
    pub fn record_page_success(
        &self,
        bytes: u64,
        fetch_us: u64,
        analysis_us: u64,
        storage_us: u64,
        findings: u64,
    ) {
        self.pages_crawled.fetch_add(1, Ordering::Relaxed);
        self.bytes_fetched.fetch_add(bytes, Ordering::Relaxed);
        self.fetch_time_us.fetch_add(fetch_us, Ordering::Relaxed);
        self.analysis_time_us
            .fetch_add(analysis_us, Ordering::Relaxed);
        self.storage_time_us
            .fetch_add(storage_us, Ordering::Relaxed);
        self.findings_generated
            .fetch_add(findings, Ordering::Relaxed);
    }

    /// Record a failed page crawl.
    pub fn record_page_failure(&self) {
        self.pages_failed.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a circuit breaker trip (transition to Open state).
    pub fn record_circuit_breaker_trip(&self) {
        self.circuit_breaker_trips.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a resource limit being hit.
    pub fn record_resource_limit_hit(&self) {
        self.resource_limit_hits.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a page skipped because the circuit breaker was open.
    pub fn record_page_skipped_circuit_breaker(&self) {
        self.pages_skipped_circuit_breaker
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Increment active connections.
    pub fn inc_connections(&self) {
        self.active_connections.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrement active connections.
    /// Uses `fetch_update` to prevent underflow below zero.
    pub fn dec_connections(&self) {
        // `fetch_update` returns `Err` only if the closure returns `None` on every
        // attempt, which in our case means the count was already zero. This is
        // the desired behavior — we simply ignore the "already at zero" case.
        let _ =
            self.active_connections
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                    if current > 0 {
                        Some(current - 1)
                    } else {
                        // Already at zero; do not underflow
                        None
                    }
                });
    }

    /// Get pages per second.
    #[must_use]
    pub fn pages_per_second(&self, elapsed: Duration) -> f64 {
        let pages = self.pages_crawled.load(Ordering::Relaxed) as f64;
        let secs = elapsed.as_secs_f64();
        if secs <= 0.0 {
            return 0.0;
        }
        pages / secs
    }

    /// Get average fetch time in milliseconds.
    #[must_use]
    pub fn avg_fetch_time_ms(&self) -> f64 {
        let pages = self.pages_crawled.load(Ordering::Relaxed);
        if pages == 0 {
            return 0.0;
        }
        let total_us = self.fetch_time_us.load(Ordering::Relaxed);
        (total_us as f64 / pages as f64) / 1000.0
    }

    /// Get average analysis time in milliseconds.
    #[must_use]
    pub fn avg_analysis_time_ms(&self) -> f64 {
        let pages = self.pages_crawled.load(Ordering::Relaxed);
        if pages == 0 {
            return 0.0;
        }
        let total_us = self.analysis_time_us.load(Ordering::Relaxed);
        (total_us as f64 / pages as f64) / 1000.0
    }

    /// Get total throughput (bytes per second).
    #[must_use]
    pub fn throughput_bps(&self, elapsed: Duration) -> f64 {
        let bytes = self.bytes_fetched.load(Ordering::Relaxed) as f64;
        let secs = elapsed.as_secs_f64();
        if secs <= 0.0 {
            return 0.0;
        }
        bytes / secs
    }

    /// Get snapshot of all metrics.
    #[must_use]
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            pages_crawled: self.pages_crawled.load(Ordering::Relaxed),
            pages_failed: self.pages_failed.load(Ordering::Relaxed),
            findings_generated: self.findings_generated.load(Ordering::Relaxed),
            bytes_fetched: self.bytes_fetched.load(Ordering::Relaxed),
            fetch_time_us: self.fetch_time_us.load(Ordering::Relaxed),
            analysis_time_us: self.analysis_time_us.load(Ordering::Relaxed),
            storage_time_us: self.storage_time_us.load(Ordering::Relaxed),
            active_connections: self.active_connections.load(Ordering::Relaxed),
            circuit_breaker_trips: self.circuit_breaker_trips.load(Ordering::Relaxed),
            resource_limit_hits: self.resource_limit_hits.load(Ordering::Relaxed),
            pages_skipped_circuit_breaker: self
                .pages_skipped_circuit_breaker
                .load(Ordering::Relaxed),
        }
    }

    /// Reset all metrics.
    pub fn reset(&self) {
        self.pages_crawled.store(0, Ordering::Relaxed);
        self.pages_failed.store(0, Ordering::Relaxed);
        self.findings_generated.store(0, Ordering::Relaxed);
        self.bytes_fetched.store(0, Ordering::Relaxed);
        self.fetch_time_us.store(0, Ordering::Relaxed);
        self.analysis_time_us.store(0, Ordering::Relaxed);
        self.storage_time_us.store(0, Ordering::Relaxed);
        self.active_connections.store(0, Ordering::Relaxed);
        self.circuit_breaker_trips.store(0, Ordering::Relaxed);
        self.resource_limit_hits.store(0, Ordering::Relaxed);
        self.pages_skipped_circuit_breaker
            .store(0, Ordering::Relaxed);
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot of metrics at a point in time.
///
/// Created by [`Metrics::snapshot`] for reporting and API responses.
/// All fields are plain `u64` values for easy serialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    /// Total pages successfully crawled.
    pub pages_crawled: u64,
    /// Total pages that failed to fetch.
    pub pages_failed: u64,
    /// Total findings generated by analyzers.
    pub findings_generated: u64,
    /// Total bytes fetched.
    pub bytes_fetched: u64,
    /// Total fetch time in microseconds.
    pub fetch_time_us: u64,
    /// Total analysis time in microseconds.
    pub analysis_time_us: u64,
    /// Total storage write time in microseconds.
    pub storage_time_us: u64,
    /// Current number of active HTTP connections.
    pub active_connections: u64,
    /// Total circuit breaker trips.
    pub circuit_breaker_trips: u64,
    /// Total resource limit hits.
    pub resource_limit_hits: u64,
    /// Pages skipped because circuit breaker was open.
    pub pages_skipped_circuit_breaker: u64,
}

/// Shared metrics for concurrent access.
///
/// Wraps [`Metrics`] in an `Arc` for sharing across tasks.
/// Clone is cheap (reference count increment).
#[derive(Clone)]
pub struct SharedMetrics {
    inner: Arc<Metrics>,
}

impl SharedMetrics {
    /// Create shared metrics.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Metrics::new()),
        }
    }

    /// Get inner metrics reference.
    #[must_use]
    pub fn inner(&self) -> &Metrics {
        &self.inner
    }
}

impl Default for SharedMetrics {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_record() {
        let metrics = Metrics::new();
        metrics.record_page_success(1024, 100, 50, 10, 3);
        metrics.record_page_failure();

        assert_eq!(metrics.pages_crawled.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.pages_failed.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.bytes_fetched.load(Ordering::Relaxed), 1024);
        assert_eq!(metrics.findings_generated.load(Ordering::Relaxed), 3);
    }

    #[test]
    fn test_metrics_snapshot() {
        let metrics = Metrics::new();
        metrics.record_page_success(1024, 100, 50, 10, 3);

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.pages_crawled, 1);
        assert_eq!(snapshot.bytes_fetched, 1024);
    }

    #[test]
    fn test_metrics_reset() {
        let metrics = Metrics::new();
        metrics.record_page_success(1024, 100, 50, 10, 3);
        metrics.reset();

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.pages_crawled, 0);
    }

    #[test]
    fn test_metrics_avg_times() {
        let metrics = Metrics::new();
        metrics.record_page_success(1024, 1000, 500, 100, 3); // 1ms fetch, 0.5ms analysis
        metrics.record_page_success(1024, 2000, 1000, 200, 5); // 2ms fetch, 1ms analysis

        assert!((metrics.avg_fetch_time_ms() - 1.5).abs() < 0.01);
        assert!((metrics.avg_analysis_time_ms() - 0.75).abs() < 0.01);
    }
}
