use crate::compare::{ChangeKind, CrawlDiff};

/// Severity level for a monitoring alert.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum AlertSeverity {
    /// Minor change: new page added, title tweak, small content drift.
    Info,
    /// Significant change: large content removal, status code change among
    /// success codes, multiple CWV regressions.
    Warning,
    /// Breaking change: page now 4xx/5xx, page removed, content dropped >50%.
    Critical,
}

impl std::fmt::Display for AlertSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlertSeverity::Info => write!(f, "info"),
            AlertSeverity::Warning => write!(f, "warning"),
            AlertSeverity::Critical => write!(f, "critical"),
        }
    }
}

/// Notification delivery channel for monitoring alerts.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum NotificationChannel {
    /// HTTP webhook URL.
    Webhook(String),
    /// Email address.
    Email(String),
    /// Slack webhook or channel URL.
    Slack(String),
}

/// Errors produced by [`ContinuousMonitor`] lifecycle
/// polling.
#[derive(Debug, thiserror::Error)]
pub enum MonitorError {
    /// The check interval could not be parsed.
    #[error("configuration error: {0}")]
    Config(String),
    /// A notification delivery channel failed.
    #[error("notification delivery failed: {0}")]
    Delivery(String),
    /// The monitor encountered a transient I/O error.
    #[error("monitor I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Configuration for the continuous monitoring loop.
#[derive(Debug, Clone)]
pub struct MonitorConfig {
    /// Seconds between each monitoring check cycle.
    pub check_interval_secs: u64,
    /// Minimum number of total changes (new + removed + changed + CWV
    /// regressions) required before an alert is triggered. A value of 0
    /// means any change triggers an alert.
    pub alert_threshold: usize,
    /// Notification delivery channels for triggered alerts.
    pub notification_channels: Vec<NotificationChannel>,
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            check_interval_secs: 3600,
            alert_threshold: 1,
            notification_channels: Vec::new(),
        }
    }
}

/// Mutable state held across monitoring check cycles.
#[derive(Debug, Default)]
pub struct MonitorState {
    /// Snapshot of the last `MonitoringResult` for diffing across cycles.
    pub last_result: Option<MonitoringResult>,
}

/// A long-running monitor that periodically evaluates crawl deltas and
/// fires notifications when alert thresholds are exceeded.
#[derive(Debug)]
pub struct ContinuousMonitor {
    /// Read-only configuration for the monitoring loop.
    pub config: MonitorConfig,
    /// Mutable state that persists across loop iterations.
    pub state: std::sync::Mutex<MonitorState>,
}

impl ContinuousMonitor {
    /// Create a new monitor with the given configuration.
    pub fn new(config: MonitorConfig) -> Self {
        Self {
            config,
            state: std::sync::Mutex::new(MonitorState::default()),
        }
    }

    /// Run one monitoring cycle: evaluate a crawl delta, produce alerts,
    /// and (optionally) deliver notifications.
    ///
    /// This method is intentionally synchronous so it can be driven by
    /// external async runtimes (tokio, async-std) without spawning an
    /// internal loop — the caller is expected to sleep and re-invoke.
    ///
    /// Returns the [`MonitoringResult`] so callers can forward it to
    /// webhooks or logging.
    pub fn check(&self, diff: &CrawlDiff) -> Result<MonitoringResult, MonitorError> {
        let result = analyze_crawl_delta(diff, self.config.alert_threshold);

        let mut state = self
            .state
            .lock()
            .map_err(|e| MonitorError::Config(format!("state lock poisoned: {e}")))?;
        state.last_result = Some(result.clone());

        Ok(result)
    }
}

/// A single alert item produced by monitoring delta analysis.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AlertDetail {
    /// URL the alert pertains to.
    pub url: String,
    /// Severity of this alert.
    pub severity: AlertSeverity,
    /// Human-readable description.
    pub message: String,
}

/// Enriched alert with classification metadata, suitable for webhook
/// payloads and notification channels.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Alert {
    /// Classified severity of this alert.
    pub severity: AlertSeverity,
    /// Short title summarizing the alert.
    pub title: String,
    /// Detailed human-readable description.
    pub description: String,
    /// URLs directly affected by this alert (top entries).
    pub affected_urls: Vec<String>,
    /// Timestamp when the alert was generated.
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl Alert {
    /// Build an [`Alert`] from a [`MonitoringResult`], extracting up to
    /// `max_urls` affected URLs.
    pub fn from_result(result: &MonitoringResult, max_urls: usize) -> Self {
        let title = match result.overall_severity {
            AlertSeverity::Critical => "Critical monitoring alert",
            AlertSeverity::Warning => "Monitoring warning",
            AlertSeverity::Info => "Monitoring info",
        };
        let description = format!(
            "{} new, {} removed, {} changed pages detected ({} CWV regressions)",
            result.new_pages, result.removed_pages, result.changed_pages, result.cwv_regressions,
        );
        let mut affected_urls = result.changed_urls.clone();
        affected_urls.truncate(max_urls);

        Alert {
            severity: result.overall_severity,
            title: title.to_string(),
            description,
            affected_urls,
            timestamp: chrono::Utc::now(),
        }
    }
}

/// Result of analyzing a crawl delta for monitoring purposes.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MonitoringResult {
    /// Number of new pages (added since baseline).
    pub new_pages: usize,
    /// Number of removed pages since baseline.
    pub removed_pages: usize,
    /// Number of pages with status, title, or content changes.
    pub changed_pages: usize,
    /// Number of CWV regressions detected.
    pub cwv_regressions: usize,
    /// Whether the alert threshold was exceeded.
    pub alert_triggered: bool,
    /// Overall severity of the monitoring result (worst of all alerts).
    pub overall_severity: AlertSeverity,
    /// Per-URL alert details with severity classification.
    pub alerts: Vec<AlertDetail>,
    /// All affected URLs (union of added, removed, changed).
    pub changed_urls: Vec<String>,
}

/// Classify a status code transition into an [`AlertSeverity`].
fn classify_status_change(from: u16, to: u16) -> AlertSeverity {
    let from_success = (200..300).contains(&from);
    let to_error = to >= 400;
    if from_success && to_error {
        AlertSeverity::Critical
    } else if from != to {
        AlertSeverity::Warning
    } else {
        AlertSeverity::Info
    }
}

/// Classify a content (word count) change into an [`AlertSeverity`].
fn classify_content_change(from: Option<usize>, to: Option<usize>) -> AlertSeverity {
    match (from, to) {
        (Some(f), Some(t)) => {
            let diff = (f as isize - t as isize).unsigned_abs();
            let pct = if f > 0 {
                diff as f64 / f as f64
            } else {
                return AlertSeverity::Warning;
            };
            if pct > 0.50 {
                AlertSeverity::Critical
            } else if pct > 0.20 {
                AlertSeverity::Warning
            } else {
                AlertSeverity::Info
            }
        }
        (Some(_), None) | (None, Some(_)) => AlertSeverity::Warning,
        _ => AlertSeverity::Info,
    }
}

/// Analyze a [`CrawlDiff`] and produce a [`MonitoringResult`].
///
/// An alert is triggered when the sum of new, removed, changed pages, and CWV
/// regressions exceeds `threshold`. When `threshold` is 0, any change triggers
/// an alert.
pub fn analyze_crawl_delta(diff: &CrawlDiff, threshold: usize) -> MonitoringResult {
    let new_pages = diff.added.len();
    let removed_pages = diff.removed.len();

    // Count distinct pages with any change (status, title, content, size).
    let mut changed_urls = std::collections::HashSet::new();
    for entry in &diff.status_changes {
        changed_urls.insert(&entry.url);
    }
    for entry in &diff.title_changes {
        changed_urls.insert(&entry.url);
    }
    for entry in &diff.content_changes {
        changed_urls.insert(&entry.url);
    }
    for entry in &diff.size_changes {
        changed_urls.insert(&entry.url);
    }
    let changed_pages = changed_urls.len();

    let cwv_regressions = diff.cwv_changes.iter().filter(|c| c.regression).count();

    let total = new_pages + removed_pages + changed_pages + cwv_regressions;
    let alert_triggered = if threshold == 0 {
        total > 0
    } else {
        total >= threshold
    };

    // Build per-URL alerts with severity classification.
    let mut alerts: Vec<AlertDetail> = Vec::new();

    for entry in &diff.added {
        alerts.push(AlertDetail {
            url: entry.url.clone(),
            severity: AlertSeverity::Info,
            message: "New page detected".to_string(),
        });
    }

    for entry in &diff.removed {
        alerts.push(AlertDetail {
            url: entry.url.clone(),
            severity: AlertSeverity::Critical,
            message: "Page removed".to_string(),
        });
    }

    for entry in &diff.status_changes {
        if let ChangeKind::StatusChanged { from, to } = &entry.change {
            let severity = classify_status_change(*from, *to);
            alerts.push(AlertDetail {
                url: entry.url.clone(),
                severity,
                message: format!("Status code changed: {from} -> {to}"),
            });
        }
    }

    for entry in &diff.title_changes {
        alerts.push(AlertDetail {
            url: entry.url.clone(),
            severity: AlertSeverity::Info,
            message: "Page title changed".to_string(),
        });
    }

    for entry in &diff.content_changes {
        if let ChangeKind::ContentChanged { from, to } = &entry.change {
            let severity = classify_content_change(*from, *to);
            alerts.push(AlertDetail {
                url: entry.url.clone(),
                severity,
                message: format!(
                    "Content changed: {} -> {} words",
                    from.map(|v| v.to_string()).unwrap_or_else(|| "?".into()),
                    to.map(|v| v.to_string()).unwrap_or_else(|| "?".into()),
                ),
            });
        }
    }

    for entry in &diff.size_changes {
        if let ChangeKind::SizeChanged { from, to } = &entry.change {
            alerts.push(AlertDetail {
                url: entry.url.clone(),
                severity: AlertSeverity::Warning,
                message: format!(
                    "Body size changed: {} -> {} bytes",
                    from.map(|v| v.to_string()).unwrap_or_else(|| "?".into()),
                    to.map(|v| v.to_string()).unwrap_or_else(|| "?".into()),
                ),
            });
        }
    }

    for cwv in &diff.cwv_changes {
        if cwv.regression {
            alerts.push(AlertDetail {
                url: cwv.url.clone(),
                severity: AlertSeverity::Warning,
                message: format!(
                    "CWV regression: {} {:.2} -> {:.2}",
                    cwv.metric_name,
                    cwv.old_value.unwrap_or(0.0),
                    cwv.new_value.unwrap_or(0.0),
                ),
            });
        }
    }

    let overall_severity = alerts
        .iter()
        .map(|a| a.severity)
        .max()
        .unwrap_or(AlertSeverity::Info);

    let mut all_urls: Vec<String> = changed_urls.into_iter().cloned().collect();
    all_urls.extend(diff.added.iter().map(|e| e.url.clone()));
    all_urls.extend(diff.removed.iter().map(|e| e.url.clone()));
    all_urls.sort();
    all_urls.dedup();

    MonitoringResult {
        new_pages,
        removed_pages,
        changed_pages,
        cwv_regressions,
        alert_triggered,
        overall_severity,
        alerts,
        changed_urls: all_urls,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compare::{ChangeKind, CwvChange, DiffEntry};

    fn empty_diff() -> CrawlDiff {
        CrawlDiff {
            baseline_pages: 0,
            target_pages: 0,
            added: vec![],
            removed: vec![],
            status_changes: vec![],
            title_changes: vec![],
            content_changes: vec![],
            size_changes: vec![],
            cwv_changes: vec![],
        }
    }

    #[test]
    fn test_analyze_empty_diff_no_alert() {
        let diff = empty_diff();
        let result = analyze_crawl_delta(&diff, 1);
        assert!(!result.alert_triggered);
        assert_eq!(result.new_pages, 0);
        assert_eq!(result.removed_pages, 0);
        assert_eq!(result.changed_pages, 0);
        assert_eq!(result.cwv_regressions, 0);
        assert_eq!(result.overall_severity, AlertSeverity::Info);
        assert!(result.alerts.is_empty());
        assert!(result.changed_urls.is_empty());
    }

    #[test]
    fn test_analyze_empty_diff_zero_threshold() {
        let diff = empty_diff();
        let result = analyze_crawl_delta(&diff, 0);
        assert!(!result.alert_triggered);
    }

    #[test]
    fn test_analyze_with_changes_below_threshold() {
        let mut diff = empty_diff();
        diff.added.push(DiffEntry {
            url: "https://example.com/new".into(),
            change: ChangeKind::Added,
        });
        let result = analyze_crawl_delta(&diff, 5);
        assert!(!result.alert_triggered);
        assert_eq!(result.new_pages, 1);
        assert_eq!(result.alerts.len(), 1);
        assert_eq!(result.alerts[0].severity, AlertSeverity::Info);
        assert!(result
            .changed_urls
            .contains(&"https://example.com/new".to_string()));
    }

    #[test]
    fn test_analyze_with_changes_meets_threshold() {
        let mut diff = empty_diff();
        diff.added.push(DiffEntry {
            url: "https://example.com/new1".into(),
            change: ChangeKind::Added,
        });
        diff.added.push(DiffEntry {
            url: "https://example.com/new2".into(),
            change: ChangeKind::Added,
        });
        diff.removed.push(DiffEntry {
            url: "https://example.com/old".into(),
            change: ChangeKind::Removed,
        });
        let result = analyze_crawl_delta(&diff, 3);
        assert!(result.alert_triggered);
        assert_eq!(result.new_pages, 2);
        assert_eq!(result.removed_pages, 1);
    }

    #[test]
    fn test_analyze_zero_threshold_any_change_triggers() {
        let mut diff = empty_diff();
        diff.status_changes.push(DiffEntry {
            url: "https://example.com/page".into(),
            change: ChangeKind::StatusChanged { from: 200, to: 404 },
        });
        let result = analyze_crawl_delta(&diff, 0);
        assert!(result.alert_triggered);
        assert_eq!(result.changed_pages, 1);
    }

    #[test]
    fn test_analyze_changed_pages_deduplicated() {
        let mut diff = empty_diff();
        diff.status_changes.push(DiffEntry {
            url: "https://example.com/page".into(),
            change: ChangeKind::StatusChanged { from: 200, to: 301 },
        });
        diff.title_changes.push(DiffEntry {
            url: "https://example.com/page".into(),
            change: ChangeKind::TitleChanged {
                from: Some("Old".into()),
                to: Some("New".into()),
            },
        });
        let result = analyze_crawl_delta(&diff, 1);
        assert_eq!(result.changed_pages, 1);
        assert!(result
            .changed_urls
            .contains(&"https://example.com/page".to_string()));
    }

    #[test]
    fn test_analyze_cwv_regressions() {
        let mut diff = empty_diff();
        diff.cwv_changes.push(CwvChange {
            url: "https://example.com/slow".into(),
            metric_name: "LCP".into(),
            old_value: Some(2.0),
            new_value: Some(3.0),
            regression: true,
        });
        diff.cwv_changes.push(CwvChange {
            url: "https://example.com/fast".into(),
            metric_name: "CLS".into(),
            old_value: Some(0.1),
            new_value: Some(0.05),
            regression: false,
        });
        let result = analyze_crawl_delta(&diff, 1);
        assert_eq!(result.cwv_regressions, 1);
        assert!(result.alert_triggered);
    }

    #[test]
    fn test_analyze_mixed_changes() {
        let mut diff = empty_diff();
        diff.added.push(DiffEntry {
            url: "https://example.com/new".into(),
            change: ChangeKind::Added,
        });
        diff.removed.push(DiffEntry {
            url: "https://example.com/old".into(),
            change: ChangeKind::Removed,
        });
        diff.title_changes.push(DiffEntry {
            url: "https://example.com/changed".into(),
            change: ChangeKind::TitleChanged {
                from: Some("A".into()),
                to: Some("B".into()),
            },
        });
        diff.cwv_changes.push(CwvChange {
            url: "https://example.com/slow".into(),
            metric_name: "INP".into(),
            old_value: Some(100.0),
            new_value: Some(200.0),
            regression: true,
        });
        let result = analyze_crawl_delta(&diff, 4);
        assert!(result.alert_triggered);
        assert_eq!(result.new_pages, 1);
        assert_eq!(result.removed_pages, 1);
        assert_eq!(result.changed_pages, 1);
        assert_eq!(result.cwv_regressions, 1);
        assert!(result.alerts.len() >= 4);
    }

    #[test]
    fn test_severity_classification_status_change() {
        assert_eq!(classify_status_change(200, 404), AlertSeverity::Critical);
        assert_eq!(classify_status_change(200, 500), AlertSeverity::Critical);
        assert_eq!(classify_status_change(200, 301), AlertSeverity::Warning);
        assert_eq!(classify_status_change(301, 302), AlertSeverity::Warning);
        assert_eq!(classify_status_change(200, 200), AlertSeverity::Info);
    }

    #[test]
    fn test_severity_classification_content_change() {
        assert_eq!(
            classify_content_change(Some(1000), Some(400)),
            AlertSeverity::Critical
        );
        assert_eq!(
            classify_content_change(Some(1000), Some(700)),
            AlertSeverity::Warning
        );
        assert_eq!(
            classify_content_change(Some(1000), Some(950)),
            AlertSeverity::Info
        );
        assert_eq!(
            classify_content_change(Some(1000), None),
            AlertSeverity::Warning
        );
        assert_eq!(
            classify_content_change(None, Some(500)),
            AlertSeverity::Warning
        );
    }

    #[test]
    fn test_overall_severity_is_worst() {
        let mut diff = empty_diff();
        diff.added.push(DiffEntry {
            url: "https://example.com/new".into(),
            change: ChangeKind::Added,
        });
        diff.removed.push(DiffEntry {
            url: "https://example.com/removed".into(),
            change: ChangeKind::Removed,
        });
        diff.status_changes.push(DiffEntry {
            url: "https://example.com/broken".into(),
            change: ChangeKind::StatusChanged { from: 200, to: 404 },
        });
        let result = analyze_crawl_delta(&diff, 0);
        assert_eq!(result.overall_severity, AlertSeverity::Critical);
    }

    #[test]
    fn test_removed_page_is_critical() {
        let mut diff = empty_diff();
        diff.removed.push(DiffEntry {
            url: "https://example.com/gone".into(),
            change: ChangeKind::Removed,
        });
        let result = analyze_crawl_delta(&diff, 0);
        let removed_alert = result
            .alerts
            .iter()
            .find(|a| a.url.contains("gone"))
            .unwrap();
        assert_eq!(removed_alert.severity, AlertSeverity::Critical);
        assert_eq!(result.overall_severity, AlertSeverity::Critical);
    }

    #[test]
    fn test_changed_urls_deduplication() {
        let mut diff = empty_diff();
        diff.status_changes.push(DiffEntry {
            url: "https://example.com/page".into(),
            change: ChangeKind::StatusChanged { from: 200, to: 301 },
        });
        diff.title_changes.push(DiffEntry {
            url: "https://example.com/page".into(),
            change: ChangeKind::TitleChanged {
                from: Some("Old".into()),
                to: Some("New".into()),
            },
        });
        diff.added.push(DiffEntry {
            url: "https://example.com/page".into(),
            change: ChangeKind::Added,
        });
        let result = analyze_crawl_delta(&diff, 0);
        let count = result
            .changed_urls
            .iter()
            .filter(|u| **u == "https://example.com/page")
            .count();
        assert_eq!(count, 1, "changed_urls should deduplicate");
    }

    #[test]
    fn test_content_change_critical_severity() {
        let mut diff = empty_diff();
        diff.content_changes.push(DiffEntry {
            url: "https://example.com/gutted".into(),
            change: ChangeKind::ContentChanged {
                from: Some(1000),
                to: Some(100),
            },
        });
        let result = analyze_crawl_delta(&diff, 0);
        let alert = result
            .alerts
            .iter()
            .find(|a| a.url.contains("gutted"))
            .unwrap();
        assert_eq!(alert.severity, AlertSeverity::Critical);
    }

    #[test]
    fn test_alert_severity_display() {
        assert_eq!(AlertSeverity::Info.to_string(), "info");
        assert_eq!(AlertSeverity::Warning.to_string(), "warning");
        assert_eq!(AlertSeverity::Critical.to_string(), "critical");
    }

    #[test]
    fn test_alert_severity_ord() {
        assert!(AlertSeverity::Info < AlertSeverity::Warning);
        assert!(AlertSeverity::Warning < AlertSeverity::Critical);
    }

    #[test]
    fn test_monitor_config_default() {
        let cfg = MonitorConfig::default();
        assert_eq!(cfg.check_interval_secs, 3600);
        assert_eq!(cfg.alert_threshold, 1);
        assert!(cfg.notification_channels.is_empty());
    }

    #[test]
    fn test_continuous_monitor_check_no_change() {
        let monitor = ContinuousMonitor::new(MonitorConfig::default());
        let diff = empty_diff();
        let result = monitor.check(&diff).unwrap();
        assert!(!result.alert_triggered);
        assert_eq!(result.new_pages, 0);
        let state = monitor.state.lock().unwrap();
        assert!(state.last_result.is_some());
    }

    #[test]
    fn test_continuous_monitor_check_with_changes() {
        let monitor = ContinuousMonitor::new(MonitorConfig {
            alert_threshold: 2,
            ..Default::default()
        });
        let mut diff = empty_diff();
        diff.added.push(DiffEntry {
            url: "https://example.com/a".into(),
            change: ChangeKind::Added,
        });
        diff.added.push(DiffEntry {
            url: "https://example.com/b".into(),
            change: ChangeKind::Added,
        });
        let result = monitor.check(&diff).unwrap();
        assert!(result.alert_triggered);
        assert_eq!(result.new_pages, 2);
    }

    #[test]
    fn test_alert_from_result() {
        let mut diff = empty_diff();
        diff.added.push(DiffEntry {
            url: "https://example.com/new".into(),
            change: ChangeKind::Added,
        });
        diff.removed.push(DiffEntry {
            url: "https://example.com/old".into(),
            change: ChangeKind::Removed,
        });
        let result = analyze_crawl_delta(&diff, 0);
        let alert = Alert::from_result(&result, 10);
        assert_eq!(alert.severity, AlertSeverity::Critical);
        assert!(alert.title.contains("Critical"));
        assert_eq!(alert.affected_urls.len(), 2);
    }

    #[test]
    fn test_alert_from_result_truncates_urls() {
        let mut diff = empty_diff();
        for i in 0..30 {
            diff.added.push(DiffEntry {
                url: format!("https://example.com/page-{i}"),
                change: ChangeKind::Added,
            });
        }
        let result = analyze_crawl_delta(&diff, 0);
        let alert = Alert::from_result(&result, 20);
        assert_eq!(alert.affected_urls.len(), 20);
    }

    #[test]
    fn test_notification_channel_equality() {
        let a = NotificationChannel::Webhook("https://example.com/hook".into());
        let b = NotificationChannel::Webhook("https://example.com/hook".into());
        let c = NotificationChannel::Email("admin@example.com".into());
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_monitor_error_display() {
        let err = MonitorError::Config("bad value".into());
        assert!(err.to_string().contains("configuration error"));
        let err = MonitorError::Delivery("timeout".into());
        assert!(err.to_string().contains("notification delivery failed"));
    }
}
