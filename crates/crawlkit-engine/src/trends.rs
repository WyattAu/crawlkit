use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::storage::CrawlStats;

/// Errors specific to trend analysis operations.
#[derive(Debug, Error)]
pub enum TrendError {
    /// Not enough snapshots for trend analysis (need at least 2).
    #[error("need at least 2 snapshots for trend analysis, got {0}")]
    InsufficientData(usize),

    /// Snapshot timestamps are not in chronological order.
    #[error("snapshots must be in chronological order")]
    UnorderedSnapshots,

    /// Arithmetic overflow in trend calculations.
    #[error("arithmetic overflow in trend calculation")]
    Overflow,
}

/// A single snapshot in a crawl time series.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlSnapshot {
    /// Unique crawl identifier.
    pub crawl_id: String,
    /// When the crawl started.
    pub timestamp: DateTime<Utc>,
    /// Aggregate crawl statistics.
    pub stats: CrawlStats,
}

/// The overall trend direction determined by linear regression of health scores.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TrendDirection {
    /// Health score trend line has a positive slope (>5% improvement).
    Improving,
    /// Health score trend line slope is within ±5% of zero.
    Stable,
    /// Health score trend line has a negative slope (>5% regression).
    Regressing,
}

/// A single point in a time-series trend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendPoint {
    /// The crawl timestamp as an RFC 3339 string.
    pub timestamp: String,
    /// The metric value at this point.
    pub value: f64,
}

/// Result of trend analysis across multiple crawl snapshots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendAnalysis {
    /// The input snapshots in chronological order.
    pub snapshots: Vec<CrawlSnapshot>,
    /// Pages crawled over time.
    pub pages_trend: Vec<TrendPoint>,
    /// Issues found over time.
    pub issues_trend: Vec<TrendPoint>,
    /// Average health score over time (0–100).
    pub score_trend: Vec<TrendPoint>,
    /// Overall trend direction based on health score regression.
    pub direction: TrendDirection,
    /// Slope of the health score trend line (issues-per-page ratio change per snapshot).
    pub slope: f64,
    /// Summary statistics.
    pub summary: TrendSummary,
}

/// Summary statistics for the trend analysis period.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendSummary {
    /// Number of snapshots analyzed.
    pub snapshot_count: usize,
    /// Time span of the analysis period (days).
    pub span_days: i64,
    /// Average pages per crawl.
    pub avg_pages: f64,
    /// Average issues per crawl.
    pub avg_issues: f64,
    /// Average health score across all snapshots.
    pub avg_health_score: f64,
    /// Min health score observed.
    pub min_health_score: f64,
    /// Max health score observed.
    pub max_health_score: f64,
    /// Percentage change in pages from first to last snapshot.
    pub pages_change_pct: f64,
    /// Percentage change in issues from first to last snapshot.
    pub issues_change_pct: f64,
}

/// Compute a health score from crawl stats.
///
/// The score is 0–100 where higher is better. It penalizes high issue counts
/// relative to the number of pages crawled.
pub fn compute_health_score(stats: &CrawlStats) -> f64 {
    if stats.total_pages == 0 {
        return 0.0;
    }
    let issues_per_page = stats.total_issues as f64 / stats.total_pages as f64;
    // Map issues_per_page to a 0–100 score:
    //   0 issues/page → 100
    //   1.0 issues/page → 50
    //   2.0+ issues/page → 0
    (1.0 - issues_per_page).clamp(0.0, 1.0) * 100.0
}

/// Analyze trends across multiple crawl snapshots.
///
/// Snapshots must be in chronological order (oldest first).
///
/// # Errors
///
/// Returns [`TrendError::InsufficientData`] if fewer than 2 snapshots are provided.
pub fn analyze_trends(snapshots: Vec<CrawlSnapshot>) -> Result<TrendAnalysis, TrendError> {
    if snapshots.len() < 2 {
        return Err(TrendError::InsufficientData(snapshots.len()));
    }

    // Verify chronological order
    for window in snapshots.windows(2) {
        if window[0].timestamp > window[1].timestamp {
            return Err(TrendError::UnorderedSnapshots);
        }
    }

    let pages_trend: Vec<TrendPoint> = snapshots
        .iter()
        .map(|s| TrendPoint {
            timestamp: s.timestamp.to_rfc3339(),
            value: s.stats.total_pages as f64,
        })
        .collect();

    let issues_trend: Vec<TrendPoint> = snapshots
        .iter()
        .map(|s| TrendPoint {
            timestamp: s.timestamp.to_rfc3339(),
            value: s.stats.total_issues as f64,
        })
        .collect();

    let score_trend: Vec<TrendPoint> = snapshots
        .iter()
        .map(|s| TrendPoint {
            timestamp: s.timestamp.to_rfc3339(),
            value: compute_health_score(&s.stats),
        })
        .collect();

    // Linear regression on health scores
    let scores: Vec<f64> = score_trend.iter().map(|p| p.value).collect();
    let n = scores.len() as f64;
    let sum_x: f64 = (0..scores.len()).map(|i| i as f64).sum();
    let sum_y: f64 = scores.iter().sum();
    let sum_xy: f64 = scores.iter().enumerate().map(|(i, &y)| i as f64 * y).sum();
    let sum_x2: f64 = (0..scores.len()).map(|i| (i as f64) * (i as f64)).sum();

    let denom = n * sum_x2 - sum_x * sum_x;
    let slope = if denom.abs() > f64::EPSILON {
        (n * sum_xy - sum_x * sum_y) / denom
    } else {
        0.0
    };

    // Normalize slope relative to average score
    let avg_score = if n > 0.0 { sum_y / n } else { 0.0 };
    let normalized_slope = if avg_score > f64::EPSILON {
        slope / avg_score
    } else {
        0.0
    };

    let direction = if normalized_slope > 0.05 {
        TrendDirection::Improving
    } else if normalized_slope < -0.05 {
        TrendDirection::Regressing
    } else {
        TrendDirection::Stable
    };

    let first = &snapshots[0];
    let last = &snapshots.last().unwrap_or(first);
    let span_days = (last.timestamp - first.timestamp).num_days();

    let avg_pages: f64 = pages_trend.iter().map(|p| p.value).sum::<f64>() / n;
    let avg_issues: f64 = issues_trend.iter().map(|p| p.value).sum::<f64>() / n;

    let min_score = scores.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_score = scores.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    let pages_change_pct = if first.stats.total_pages > 0 {
        ((last.stats.total_pages as f64 - first.stats.total_pages as f64)
            / first.stats.total_pages as f64)
            * 100.0
    } else {
        0.0
    };

    let issues_change_pct = if first.stats.total_issues > 0 {
        ((last.stats.total_issues as f64 - first.stats.total_issues as f64)
            / first.stats.total_issues as f64)
            * 100.0
    } else {
        0.0
    };

    let snapshot_count = snapshots.len();
    Ok(TrendAnalysis {
        snapshots,
        pages_trend,
        issues_trend,
        score_trend,
        direction,
        slope,
        summary: TrendSummary {
            snapshot_count,
            span_days,
            avg_pages,
            avg_issues,
            avg_health_score: avg_score,
            min_health_score: min_score,
            max_health_score: max_score,
            pages_change_pct,
            issues_change_pct,
        },
    })
}

/// Serialize a [`TrendAnalysis`] to JSON.
pub fn trend_to_json(analysis: &TrendAnalysis, pretty: bool) -> Result<String, serde_json::Error> {
    if pretty {
        serde_json::to_string_pretty(analysis)
    } else {
        serde_json::to_string(analysis)
    }
}

/// Serialize a [`TrendAnalysis`] to Markdown.
pub fn trend_to_markdown(analysis: &TrendAnalysis) -> String {
    let mut md = String::new();

    md.push_str("# Crawl Trend Analysis\n\n");

    md.push_str(&format!(
        "- **Snapshots:** {}\n\
         - **Time span:** {} days\n\
         - **Direction:** {:?}\n\
         - **Slope:** {:.4}\n",
        analysis.summary.snapshot_count,
        analysis.summary.span_days,
        analysis.direction,
        analysis.slope,
    ));

    md.push_str(&format!(
        "- **Avg pages:** {:.0}\n\
         - **Avg issues:** {:.0}\n\
         - **Avg health score:** {:.1}/100\n\
         - **Health score range:** {:.1} – {:.1}\n",
        analysis.summary.avg_pages,
        analysis.summary.avg_issues,
        analysis.summary.avg_health_score,
        analysis.summary.min_health_score,
        analysis.summary.max_health_score,
    ));

    md.push_str(&format!(
        "- **Pages change:** {:+.1}%\n\
         - **Issues change:** {:+.1}%\n\n",
        analysis.summary.pages_change_pct, analysis.summary.issues_change_pct,
    ));

    md.push_str("## Snapshot Timeline\n\n");
    md.push_str("| Crawl ID | Timestamp | Pages | Issues | Health Score |\n");
    md.push_str("|----------|-----------|-------|--------|-------------|\n");
    for snapshot in &analysis.snapshots {
        let score = compute_health_score(&snapshot.stats);
        md.push_str(&format!(
            "| {} | {} | {} | {} | {:.1} |\n",
            snapshot.crawl_id,
            snapshot.timestamp.format("%Y-%m-%d %H:%M"),
            snapshot.stats.total_pages,
            snapshot.stats.total_issues,
            score,
        ));
    }
    md.push('\n');

    md.push_str("## Pages Trend\n\n");
    for point in &analysis.pages_trend {
        md.push_str(&format!(
            "- `{}`: {:.0} pages\n",
            point.timestamp, point.value
        ));
    }
    md.push('\n');

    md.push_str("## Issues Trend\n\n");
    for point in &analysis.issues_trend {
        md.push_str(&format!(
            "- `{}`: {:.0} issues\n",
            point.timestamp, point.value,
        ));
    }
    md.push('\n');

    md
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::CrawlStats;
    use chrono::TimeZone;

    fn make_stats(pages: usize, issues: usize) -> CrawlStats {
        CrawlStats {
            total_pages: pages,
            total_issues: issues,
            issues_by_severity: std::collections::HashMap::new(),
            issues_by_category: std::collections::HashMap::new(),
            avg_response_time_ms: Some(200.0),
            total_body_size: Some(pages * 1024),
        }
    }

    fn make_snapshot(
        crawl_id: &str,
        days_offset: i64,
        pages: usize,
        issues: usize,
    ) -> CrawlSnapshot {
        CrawlSnapshot {
            crawl_id: crawl_id.to_string(),
            timestamp: Utc
                .with_ymd_and_hms(2026, 1, (1 + days_offset) as u32, 0, 0, 0)
                .unwrap(),
            stats: make_stats(pages, issues),
        }
    }

    #[test]
    fn test_compute_health_score_zero_pages() {
        let stats = make_stats(0, 0);
        assert_eq!(compute_health_score(&stats), 0.0);
    }

    #[test]
    fn test_compute_health_score_perfect() {
        let stats = make_stats(100, 0);
        assert_eq!(compute_health_score(&stats), 100.0);
    }

    #[test]
    fn test_compute_health_score_half() {
        let stats = make_stats(100, 100);
        let score = compute_health_score(&stats);
        assert!((score - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_analyze_trends_insufficient_data() {
        let snapshots = vec![make_snapshot("c1", 0, 100, 10)];
        let result = analyze_trends(snapshots);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TrendError::InsufficientData(1)
        ));
    }

    #[test]
    fn test_analyze_trends_stable() {
        let snapshots = vec![
            make_snapshot("c1", 0, 100, 10),
            make_snapshot("c2", 7, 102, 10),
            make_snapshot("c3", 14, 98, 11),
        ];
        let analysis = analyze_trends(snapshots).unwrap();
        assert_eq!(analysis.direction, TrendDirection::Stable);
        assert_eq!(analysis.snapshots.len(), 3);
        assert_eq!(analysis.pages_trend.len(), 3);
    }

    #[test]
    fn test_analyze_trends_improving() {
        let snapshots = vec![
            make_snapshot("c1", 0, 100, 50),
            make_snapshot("c2", 7, 120, 30),
            make_snapshot("c3", 14, 140, 10),
            make_snapshot("c4", 21, 160, 5),
        ];
        let analysis = analyze_trends(snapshots).unwrap();
        assert_eq!(analysis.direction, TrendDirection::Improving);
    }

    #[test]
    fn test_analyze_trends_regressing() {
        let snapshots = vec![
            make_snapshot("c1", 0, 100, 5),
            make_snapshot("c2", 7, 100, 20),
            make_snapshot("c3", 14, 100, 40),
            make_snapshot("c4", 21, 100, 60),
        ];
        let analysis = analyze_trends(snapshots).unwrap();
        assert_eq!(analysis.direction, TrendDirection::Regressing);
    }

    #[test]
    fn test_trend_to_json() {
        let snapshots = vec![
            make_snapshot("c1", 0, 100, 50),
            make_snapshot("c2", 7, 120, 30),
            make_snapshot("c3", 14, 140, 10),
            make_snapshot("c4", 21, 160, 5),
        ];
        let analysis = analyze_trends(snapshots).unwrap();
        let json = trend_to_json(&analysis, true).unwrap();
        assert!(json.contains("snapshot_count"));
        assert!(json.contains("Improving"));
    }

    #[test]
    fn test_trend_to_markdown() {
        let snapshots = vec![
            make_snapshot("c1", 0, 100, 10),
            make_snapshot("c2", 7, 105, 8),
        ];
        let analysis = analyze_trends(snapshots).unwrap();
        let md = trend_to_markdown(&analysis);
        assert!(md.contains("# Crawl Trend Analysis"));
        assert!(md.contains("Snapshot Timeline"));
        assert!(md.contains("c1"));
        assert!(md.contains("c2"));
    }

    #[test]
    fn test_summary_statistics() {
        let snapshots = vec![
            make_snapshot("c1", 0, 100, 20),
            make_snapshot("c2", 30, 200, 10),
        ];
        let analysis = analyze_trends(snapshots).unwrap();
        assert_eq!(analysis.summary.snapshot_count, 2);
        assert_eq!(analysis.summary.span_days, 30);
        assert!((analysis.summary.avg_pages - 150.0).abs() < 0.01);
        assert!((analysis.summary.avg_issues - 15.0).abs() < 0.01);
    }
}
