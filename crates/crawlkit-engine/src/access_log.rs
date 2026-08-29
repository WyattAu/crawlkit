use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// A single access log entry recording an API operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessLogEntry {
    /// When the access occurred.
    pub timestamp: DateTime<Utc>,
    /// Authenticated user ID, if available.
    pub user_id: Option<String>,
    /// API key ID used for the request, if available.
    pub api_key_id: Option<String>,
    /// Action performed (e.g. "crawl.start", "key.create").
    pub action: String,
    /// Resource acted upon (e.g. crawl_id, key_id).
    pub resource: String,
    /// Client IP address, if available.
    pub ip_address: Option<String>,
    /// Whether the operation succeeded.
    pub success: bool,
}

/// Filters for querying access log entries.
#[derive(Debug, Clone, Default)]
pub struct AccessLogFilter {
    /// Filter by user ID.
    pub user_id: Option<String>,
    /// Filter by API key ID.
    pub api_key_id: Option<String>,
    /// Filter by action prefix (e.g. "crawl." matches all crawl actions).
    pub action_prefix: Option<String>,
    /// Filter by resource.
    pub resource: Option<String>,
    /// Only include entries after this timestamp.
    pub after: Option<DateTime<Utc>>,
    /// Only include entries before this timestamp.
    pub before: Option<DateTime<Utc>>,
    /// Only include failed operations.
    pub failed_only: bool,
}

/// Thread-safe access logger for SOC 2 compliance.
///
/// Records all API access events with user identity, timestamp, action,
/// and resource. Supports in-memory storage with configurable capacity
/// and JSON export.
///
/// # Examples
///
/// ```rust
/// use crawlkit_engine::access_log::{AccessLogger, AccessLogEntry};
///
/// let mut logger = AccessLogger::new(1000);
/// logger.log(AccessLogEntry {
///     timestamp: chrono::Utc::now(),
///     user_id: Some("user-1".into()),
///     api_key_id: None,
///     action: "crawl.start".into(),
///     resource: "crawl-abc".into(),
///     ip_address: Some("127.0.0.1".into()),
///     success: true,
/// });
/// assert_eq!(logger.len(), 1);
/// ```
pub struct AccessLogger {
    entries: Arc<RwLock<Vec<AccessLogEntry>>>,
    max_entries: usize,
}

impl AccessLogger {
    /// Create a new access logger with the given maximum capacity.
    ///
    /// When the capacity is reached, the oldest entries are evicted.
    #[must_use]
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Arc::new(RwLock::new(Vec::with_capacity(max_entries.min(4096)))),
            max_entries,
        }
    }

    /// Log an access event.
    pub fn log(&self, entry: AccessLogEntry) {
        let mut entries = self.entries.write();
        if entries.len() >= self.max_entries {
            // Evict oldest 10% to amortize.
            let drain = self.max_entries / 10;
            entries.drain(..drain);
        }
        entries.push(entry);
    }

    /// Query entries matching the given filters.
    #[must_use]
    pub fn query(&self, filters: &AccessLogFilter) -> Vec<AccessLogEntry> {
        let entries = self.entries.read();
        entries
            .iter()
            .filter(|e| {
                if let Some(ref uid) = filters.user_id {
                    if e.user_id.as_deref() != Some(uid.as_str()) {
                        return false;
                    }
                }
                if let Some(ref kid) = filters.api_key_id {
                    if e.api_key_id.as_deref() != Some(kid.as_str()) {
                        return false;
                    }
                }
                if let Some(ref prefix) = filters.action_prefix {
                    if !e.action.starts_with(prefix) {
                        return false;
                    }
                }
                if let Some(ref res) = filters.resource {
                    if e.resource != *res {
                        return false;
                    }
                }
                if let Some(after) = filters.after {
                    if e.timestamp <= after {
                        return false;
                    }
                }
                if let Some(before) = filters.before {
                    if e.timestamp >= before {
                        return false;
                    }
                }
                if filters.failed_only && e.success {
                    return false;
                }
                true
            })
            .cloned()
            .collect()
    }

    /// Export all entries as a JSON string.
    ///
    /// # Errors
    /// Returns `Err` if JSON serialization fails (should not happen for
    /// well-formed entries).
    pub fn export_json(&self) -> Result<String, serde_json::Error> {
        let entries = self.entries.read();
        serde_json::to_string_pretty(&*entries)
    }

    /// Number of entries currently stored.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.read().len()
    }

    /// Whether the log is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.read().is_empty()
    }
}

impl Default for AccessLogger {
    fn default() -> Self {
        Self::new(10_000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(action: &str, resource: &str, success: bool) -> AccessLogEntry {
        AccessLogEntry {
            timestamp: Utc::now(),
            user_id: Some("user-1".into()),
            api_key_id: None,
            action: action.to_string(),
            resource: resource.to_string(),
            ip_address: Some("127.0.0.1".into()),
            success,
        }
    }

    #[test]
    fn test_log_and_len() {
        let logger = AccessLogger::new(100);
        assert!(logger.is_empty());

        logger.log(entry("crawl.start", "crawl-1", true));
        assert_eq!(logger.len(), 1);

        logger.log(entry("crawl.complete", "crawl-1", true));
        assert_eq!(logger.len(), 2);
    }

    #[test]
    fn test_eviction_at_capacity() {
        let logger = AccessLogger::new(10);
        for i in 0..15 {
            logger.log(entry("action", &format!("res-{i}"), true));
        }
        // Should have evicted some and kept at most max_entries.
        assert!(logger.len() <= 10);
    }

    #[test]
    fn test_query_by_user_id() {
        let logger = AccessLogger::new(100);
        logger.log(AccessLogEntry {
            user_id: Some("alice".into()),
            ..entry("crawl.start", "c1", true)
        });
        logger.log(AccessLogEntry {
            user_id: Some("bob".into()),
            ..entry("crawl.start", "c2", true)
        });

        let results = logger.query(&AccessLogFilter {
            user_id: Some("alice".into()),
            ..Default::default()
        });
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].resource, "c1");
    }

    #[test]
    fn test_query_by_action_prefix() {
        let logger = AccessLogger::new(100);
        logger.log(entry("crawl.start", "c1", true));
        logger.log(entry("crawl.complete", "c2", true));
        logger.log(entry("key.create", "k1", true));

        let results = logger.query(&AccessLogFilter {
            action_prefix: Some("crawl.".into()),
            ..Default::default()
        });
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_query_failed_only() {
        let logger = AccessLogger::new(100);
        logger.log(entry("crawl.start", "c1", true));
        logger.log(entry("crawl.start", "c2", false));

        let results = logger.query(&AccessLogFilter {
            failed_only: true,
            ..Default::default()
        });
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].resource, "c2");
    }

    #[test]
    fn test_query_by_time_range() {
        let logger = AccessLogger::new(100);
        let past = Utc::now() - chrono::Duration::hours(2);
        let future = Utc::now() + chrono::Duration::hours(2);

        logger.log(AccessLogEntry {
            timestamp: past,
            ..entry("a", "r1", true)
        });
        logger.log(AccessLogEntry {
            timestamp: Utc::now(),
            ..entry("a", "r2", true)
        });
        logger.log(AccessLogEntry {
            timestamp: future,
            ..entry("a", "r3", true)
        });

        let results = logger.query(&AccessLogFilter {
            after: Some(past + chrono::Duration::minutes(1)),
            before: Some(future - chrono::Duration::minutes(1)),
            ..Default::default()
        });
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].resource, "r2");
    }

    #[test]
    fn test_export_json() {
        let logger = AccessLogger::new(100);
        logger.log(entry("test", "res", true));
        let json = logger.export_json().unwrap();
        assert!(json.contains("test"));
        assert!(json.contains("res"));
    }
}
