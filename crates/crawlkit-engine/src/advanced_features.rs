use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::storage::Severity;

/// Alert definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    /// Alert ID.
    pub id: String,
    /// Alert name.
    pub name: String,
    /// Alert description.
    pub description: String,
    /// Alert severity.
    pub severity: Severity,
    /// Metric to monitor.
    pub metric: String,
    /// Threshold value.
    pub threshold: f64,
    /// Comparison operator.
    pub operator: AlertOperator,
    /// Whether alert is enabled.
    pub enabled: bool,
}

/// Alert comparison operator.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AlertOperator {
    /// Value must be greater than threshold.
    GreaterThan,
    /// Value must be less than threshold.
    LessThan,
    /// Value must equal threshold.
    Equals,
    /// Value must not equal threshold.
    NotEquals,
}

/// Alert state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertState {
    /// Alert ID.
    pub alert_id: String,
    /// Whether alert is currently triggered.
    pub triggered: bool,
    /// Last triggered timestamp.
    pub last_triggered: Option<String>,
    /// Trigger count.
    pub trigger_count: u64,
}

/// Alert manager for monitoring and notifications.
pub struct AlertManager {
    alerts: Arc<RwLock<Vec<Alert>>>,
    states: Arc<RwLock<HashMap<String, AlertState>>>,
}

impl AlertManager {
    /// Create new alert manager.
    #[must_use]
    pub fn new() -> Self {
        Self {
            alerts: Arc::new(RwLock::new(Vec::new())),
            states: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add an alert.
    pub fn add_alert(&self, alert: Alert) {
        let mut alerts = self.alerts.write();
        alerts.push(alert);
    }

    /// Check alerts against current metrics.
    pub fn check_alerts(&self, metrics: &HashMap<String, f64>) -> Vec<Alert> {
        let alerts = self.alerts.read();
        let mut states = self.states.write();
        let mut triggered = Vec::new();

        for alert in alerts.iter() {
            if !alert.enabled {
                continue;
            }

            if let Some(value) = metrics.get(&alert.metric) {
                let is_triggered = match alert.operator {
                    AlertOperator::GreaterThan => *value > alert.threshold,
                    AlertOperator::LessThan => *value < alert.threshold,
                    AlertOperator::Equals => (*value - alert.threshold).abs() < f64::EPSILON,
                    AlertOperator::NotEquals => (*value - alert.threshold).abs() > f64::EPSILON,
                };

                let state = states
                    .entry(alert.id.clone())
                    .or_insert_with(|| AlertState {
                        alert_id: alert.id.clone(),
                        triggered: false,
                        last_triggered: None,
                        trigger_count: 0,
                    });

                if is_triggered && !state.triggered {
                    state.triggered = true;
                    state.last_triggered = Some(chrono::Utc::now().to_rfc3339());
                    state.trigger_count += 1;
                    triggered.push(alert.clone());
                } else if !is_triggered {
                    state.triggered = false;
                }
            }
        }

        triggered
    }

    /// Get all alerts.
    #[must_use]
    pub fn alerts(&self) -> Vec<Alert> {
        self.alerts.read().clone()
    }

    /// Get alert states.
    #[must_use]
    pub fn states(&self) -> HashMap<String, AlertState> {
        self.states.read().clone()
    }
}

impl Default for AlertManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Scheduled crawl configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledCrawl {
    /// Crawl ID.
    pub id: String,
    /// Target URL.
    pub url: String,
    /// Cron expression (simplified: "daily", "weekly", "monthly").
    pub schedule: String,
    /// Crawl configuration.
    pub config: serde_json::Value,
    /// Whether schedule is enabled.
    pub enabled: bool,
    /// Last run timestamp.
    pub last_run: Option<String>,
    /// Next run timestamp.
    pub next_run: Option<String>,
}

/// Scheduler for automated crawls.
pub struct CrawlScheduler {
    schedules: Arc<RwLock<Vec<ScheduledCrawl>>>,
}

impl CrawlScheduler {
    /// Create new scheduler.
    #[must_use]
    pub fn new() -> Self {
        Self {
            schedules: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Add a scheduled crawl.
    pub fn add_schedule(&self, schedule: ScheduledCrawl) {
        let mut schedules = self.schedules.write();
        schedules.push(schedule);
    }

    /// Get all scheduled crawls.
    #[must_use]
    pub fn schedules(&self) -> Vec<ScheduledCrawl> {
        self.schedules.read().clone()
    }

    /// Get due crawls (simplified: always return enabled schedules).
    #[must_use]
    pub fn get_due_crawls(&self) -> Vec<ScheduledCrawl> {
        self.schedules
            .read()
            .iter()
            .filter(|s| s.enabled)
            .cloned()
            .collect()
    }
}

impl Default for CrawlScheduler {
    fn default() -> Self {
        Self::new()
    }
}

/// Historical trend data point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendDataPoint {
    /// Timestamp.
    pub timestamp: String,
    /// Metric name.
    pub metric: String,
    /// Metric value.
    pub value: f64,
    /// Crawl ID.
    pub crawl_id: String,
}

/// Historical trends tracker.
pub struct TrendTracker {
    data: Arc<RwLock<Vec<TrendDataPoint>>>,
}

impl TrendTracker {
    /// Create new trend tracker.
    #[must_use]
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Record a data point.
    pub fn record(&self, point: TrendDataPoint) {
        let mut data = self.data.write();
        data.push(point);
    }

    /// Get trend data for a metric.
    #[must_use]
    pub fn get_trend(&self, metric: &str) -> Vec<TrendDataPoint> {
        self.data
            .read()
            .iter()
            .filter(|p| p.metric == metric)
            .cloned()
            .collect()
    }

    /// Get all data points.
    #[must_use]
    pub fn all_data(&self) -> Vec<TrendDataPoint> {
        self.data.read().clone()
    }

    /// Calculate average for a metric.
    #[must_use]
    pub fn average(&self, metric: &str) -> Option<f64> {
        let values: Vec<f64> = self
            .data
            .read()
            .iter()
            .filter(|p| p.metric == metric)
            .map(|p| p.value)
            .collect();

        if values.is_empty() {
            None
        } else {
            Some(values.iter().sum::<f64>() / values.len() as f64)
        }
    }
}

impl Default for TrendTracker {
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
    fn test_alert_manager() {
        let manager = AlertManager::new();

        let alert = Alert {
            id: "test".to_string(),
            name: "Test Alert".to_string(),
            description: "Test alert".to_string(),
            severity: Severity::Warning,
            metric: "pages_crawled".to_string(),
            threshold: 100.0,
            operator: AlertOperator::GreaterThan,
            enabled: true,
        };

        manager.add_alert(alert);
        assert_eq!(manager.alerts().len(), 1);
    }

    #[test]
    fn test_alert_triggering() {
        let manager = AlertManager::new();

        let alert = Alert {
            id: "test".to_string(),
            name: "Test Alert".to_string(),
            description: "Test alert".to_string(),
            severity: Severity::Warning,
            metric: "pages_crawled".to_string(),
            threshold: 100.0,
            operator: AlertOperator::GreaterThan,
            enabled: true,
        };

        manager.add_alert(alert);

        let mut metrics = HashMap::new();
        metrics.insert("pages_crawled".to_string(), 150.0);

        let triggered = manager.check_alerts(&metrics);
        assert_eq!(triggered.len(), 1);
        assert_eq!(triggered[0].id, "test");
    }

    #[test]
    fn test_scheduler() {
        let scheduler = CrawlScheduler::new();

        let schedule = ScheduledCrawl {
            id: "test".to_string(),
            url: "https://example.com".to_string(),
            schedule: "daily".to_string(),
            config: serde_json::json!({}),
            enabled: true,
            last_run: None,
            next_run: None,
        };

        scheduler.add_schedule(schedule);
        assert_eq!(scheduler.schedules().len(), 1);
        assert_eq!(scheduler.get_due_crawls().len(), 1);
    }

    #[test]
    fn test_trend_tracker() {
        let tracker = TrendTracker::new();

        tracker.record(TrendDataPoint {
            timestamp: "2026-01-01".to_string(),
            metric: "pages_crawled".to_string(),
            value: 100.0,
            crawl_id: "crawl1".to_string(),
        });

        tracker.record(TrendDataPoint {
            timestamp: "2026-01-02".to_string(),
            metric: "pages_crawled".to_string(),
            value: 150.0,
            crawl_id: "crawl2".to_string(),
        });

        assert_eq!(tracker.get_trend("pages_crawled").len(), 2);
        assert!((tracker.average("pages_crawled").unwrap() - 125.0).abs() < 0.01);
    }
}
