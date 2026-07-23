use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Observability metrics for crawl operations.
///
/// Thread-safe metrics collection using atomic operations.
/// Designed for zero-allocation in hot paths.
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
        }
    }

    /// Record a successful page crawl.
    pub fn record_page_success(&self, bytes: u64, fetch_us: u64, analysis_us: u64, storage_us: u64, findings: u64) {
        self.pages_crawled.fetch_add(1, Ordering::Relaxed);
        self.bytes_fetched.fetch_add(bytes, Ordering::Relaxed);
        self.fetch_time_us.fetch_add(fetch_us, Ordering::Relaxed);
        self.analysis_time_us.fetch_add(analysis_us, Ordering::Relaxed);
        self.storage_time_us.fetch_add(storage_us, Ordering::Relaxed);
        self.findings_generated.fetch_add(findings, Ordering::Relaxed);
    }

    /// Record a failed page crawl.
    pub fn record_page_failure(&self) {
        self.pages_failed.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment active connections.
    pub fn inc_connections(&self) {
        self.active_connections.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrement active connections.
    pub fn dec_connections(&self) {
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
    }

    /// Get pages per second.
    #[must_use]
    pub fn pages_per_second(&self, elapsed: Duration) -> f64 {
        let pages = self.pages_crawled.load(Ordering::Relaxed) as f64;
        pages / elapsed.as_secs_f64()
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
        bytes / elapsed.as_secs_f64()
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
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot of metrics at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub pages_crawled: u64,
    pub pages_failed: u64,
    pub findings_generated: u64,
    pub bytes_fetched: u64,
    pub fetch_time_us: u64,
    pub analysis_time_us: u64,
    pub storage_time_us: u64,
    pub active_connections: u64,
}

/// Shared metrics for concurrent access.
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
