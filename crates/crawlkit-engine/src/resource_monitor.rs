use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// Resource limits for a crawl session.
///
/// Defines maximum thresholds for memory, CPU, disk, file descriptors,
/// duration, and page count. When any limit is exceeded, the crawl
/// should be terminated gracefully.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum memory usage in bytes.
    pub max_memory_bytes: Option<u64>,
    /// Maximum CPU time in seconds.
    pub max_cpu_seconds: Option<u64>,
    /// Maximum disk usage in bytes.
    pub max_disk_bytes: Option<u64>,
    /// Maximum number of open file descriptors.
    pub max_open_files: Option<u32>,
    /// Maximum crawl duration.
    pub max_duration: Option<Duration>,
    /// Maximum number of pages.
    pub max_pages: Option<usize>,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_bytes: Some(512 * 1024 * 1024), // 500 MB
            max_cpu_seconds: Some(3600),               // 1 hour
            max_disk_bytes: Some(1024 * 1024 * 1024),  // 1 GB
            max_open_files: Some(1024),
            max_duration: Some(Duration::from_secs(3600)),
            max_pages: Some(10000),
        }
    }
}

/// Current resource usage.
///
/// Snapshot of resource consumption at a point in time. Compared against
/// [`ResourceLimits`] to determine if the crawl should be stopped.
#[derive(Debug, Clone, Default)]
pub struct ResourceUsage {
    /// Current memory usage in bytes.
    pub memory_bytes: u64,
    /// CPU time used in seconds.
    pub cpu_seconds: u64,
    /// Disk usage in bytes.
    pub disk_bytes: u64,
    /// Number of open file descriptors.
    pub open_files: u32,
    /// Elapsed time since crawl start.
    pub elapsed: Duration,
    /// Number of pages processed.
    pub pages_processed: usize,
}

/// Resource monitor for tracking and enforcing limits.
///
/// Thread-safe monitor that tracks resource usage and compares against
/// configured limits. Use [`is_over_limit`](ResourceMonitor::is_over_limit)
/// to check if the crawl should be terminated.
///
/// # Examples
///
/// ```rust
/// use crawlkit_engine::{ResourceMonitor, ResourceLimits};
///
/// let limits = ResourceLimits { max_pages: Some(10), ..Default::default() };
/// let monitor = ResourceMonitor::new(limits);
/// monitor.record_page();
/// assert!(!monitor.is_over_limit());
/// ```
pub struct ResourceMonitor {
    limits: ResourceLimits,
    usage: Arc<RwLock<ResourceUsage>>,
    start_time: Instant,
}

impl ResourceMonitor {
    /// Create a new resource monitor.
    #[must_use]
    pub fn new(limits: ResourceLimits) -> Self {
        Self {
            limits,
            usage: Arc::new(RwLock::new(ResourceUsage::default())),
            start_time: Instant::now(),
        }
    }

    /// Create with default limits.
    #[must_use]
    pub fn with_default_limits() -> Self {
        Self::new(ResourceLimits::default())
    }

    /// Update current usage.
    pub fn update(&self, usage: ResourceUsage) {
        *self.usage.write() = usage;
    }

    /// Record a page processed.
    pub fn record_page(&self) {
        self.usage.write().pages_processed += 1;
    }

    /// Get current usage.
    #[must_use]
    pub fn current_usage(&self) -> ResourceUsage {
        let mut usage = self.usage.read().clone();
        usage.elapsed = self.start_time.elapsed();
        usage
    }

    /// Check if any limit is exceeded.
    #[must_use]
    pub fn is_over_limit(&self) -> bool {
        let usage = self.current_usage();

        if let Some(max_mem) = self.limits.max_memory_bytes {
            if usage.memory_bytes > max_mem {
                return true;
            }
        }

        if let Some(max_cpu) = self.limits.max_cpu_seconds {
            if usage.cpu_seconds > max_cpu {
                return true;
            }
        }

        if let Some(max_disk) = self.limits.max_disk_bytes {
            if usage.disk_bytes > max_disk {
                return true;
            }
        }

        if let Some(max_files) = self.limits.max_open_files {
            if usage.open_files > max_files {
                return true;
            }
        }

        if let Some(max_duration) = self.limits.max_duration {
            if usage.elapsed > max_duration {
                return true;
            }
        }

        if let Some(max_pages) = self.limits.max_pages {
            if usage.pages_processed > max_pages {
                return true;
            }
        }

        false
    }

    /// Get which limits are exceeded.
    #[must_use]
    pub fn exceeded_limits(&self) -> Vec<String> {
        let usage = self.current_usage();
        let mut exceeded = Vec::new();

        if let Some(max_mem) = self.limits.max_memory_bytes {
            if usage.memory_bytes > max_mem {
                exceeded.push(format!(
                    "Memory: {} / {} bytes",
                    usage.memory_bytes, max_mem
                ));
            }
        }

        if let Some(max_cpu) = self.limits.max_cpu_seconds {
            if usage.cpu_seconds > max_cpu {
                exceeded.push(format!("CPU: {} / {} seconds", usage.cpu_seconds, max_cpu));
            }
        }

        if let Some(max_pages) = self.limits.max_pages {
            if usage.pages_processed > max_pages {
                exceeded.push(format!("Pages: {} / {}", usage.pages_processed, max_pages));
            }
        }

        if let Some(max_duration) = self.limits.max_duration {
            if usage.elapsed > max_duration {
                exceeded.push(format!(
                    "Duration: {:?} / {:?}",
                    usage.elapsed, max_duration
                ));
            }
        }

        exceeded
    }
}

impl Default for ResourceMonitor {
    fn default() -> Self {
        Self::with_default_limits()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_monitor_default_limits() {
        let monitor = ResourceMonitor::with_default_limits();
        assert!(!monitor.is_over_limit());
    }

    #[test]
    fn test_resource_monitor_page_limit() {
        let limits = ResourceLimits {
            max_pages: Some(2),
            ..Default::default()
        };
        let monitor = ResourceMonitor::new(limits);

        monitor.record_page();
        assert!(!monitor.is_over_limit());

        monitor.record_page();
        assert!(!monitor.is_over_limit());

        monitor.record_page();
        assert!(monitor.is_over_limit());
    }

    #[test]
    fn test_resource_monitor_exceeded_limits() {
        let limits = ResourceLimits {
            max_pages: Some(1),
            ..Default::default()
        };
        let monitor = ResourceMonitor::new(limits);

        monitor.record_page();
        monitor.record_page();

        let exceeded = monitor.exceeded_limits();
        assert!(!exceeded.is_empty());
        assert!(exceeded[0].contains("Pages"));
    }
}
