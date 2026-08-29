use crate::compare::CrawlDiff;

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

    MonitoringResult {
        new_pages,
        removed_pages,
        changed_pages,
        cwv_regressions,
        alert_triggered,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compare::{CwvChange, DiffEntry, ChangeKind};

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
        // Same URL has both a status change and a title change
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
        assert_eq!(result.changed_pages, 1); // deduplicated
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
            regression: false, // improvement, not regression
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
        // total = 1 + 1 + 1 + 1 = 4
        let result = analyze_crawl_delta(&diff, 4);
        assert!(result.alert_triggered);
        assert_eq!(result.new_pages, 1);
        assert_eq!(result.removed_pages, 1);
        assert_eq!(result.changed_pages, 1);
        assert_eq!(result.cwv_regressions, 1);
    }
}
