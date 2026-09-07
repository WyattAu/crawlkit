//! Rank snapshot history and trend analysis.
//!
//! Positions are stored per keyword per check (see
//! [`crate::storage::Storage::record_rank_position`]). This module
//! analyzes a chronological series of snapshots into a
//! [`RankTrend`] describing direction, best/average position, and
//! net change — mirroring the crawl trend analysis in
//! [`crate::trends`].

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::rank::RankError;

/// Synthetic position assigned to checks where the keyword was not
/// found in the top results, so it can participate in regressions.
pub const NOT_RANKED_POSITION: u16 = 101;

/// A single point in a keyword's position history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankSnapshot {
    /// The stored keyword this snapshot belongs to.
    pub keyword_id: String,
    /// When the check was performed.
    pub checked_at: DateTime<Utc>,
    /// 1-based SERP position, or `None` when not in the top results.
    pub position: Option<u16>,
    /// Data source that produced the snapshot (e.g. `duckduckgo`).
    pub source: String,
}

/// Direction of a rank trend over the analysis window.
///
/// Lower positions are better, so *improving* means the regression
/// line slopes downward (toward position 1).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RankTrendDirection {
    /// Normalized slope < -5%: position moving toward the top.
    Improving,
    /// Normalized slope within ±5% of zero.
    Stable,
    /// Normalized slope > 5%: position moving away from the top.
    Declining,
}

/// Result of trend analysis over a keyword's position history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankTrend {
    /// Number of snapshots analyzed.
    pub points: usize,
    /// Position at the start of the window.
    pub first_position: Option<u16>,
    /// Position at the end of the window.
    pub last_position: Option<u16>,
    /// Best (lowest) position observed.
    pub best_position: Option<u16>,
    /// Mean position, ignoring not-ranked checks.
    pub avg_position: Option<f64>,
    /// Net change from first to last (negative = improved).
    pub change: i32,
    /// Overall direction from linear regression.
    pub direction: RankTrendDirection,
}

/// Analyze a chronological series of rank snapshots.
///
/// Snapshots must be ordered oldest first. Not-ranked checks
/// (`position: None`) count as [`NOT_RANKED_POSITION`] for the
/// regression and net change, but are excluded from the average and
/// best-position computations.
///
/// # Errors
///
/// Returns [`RankError::Trend`] when fewer than two snapshots are
/// supplied.
pub fn analyze_rank_trend(history: &[RankSnapshot]) -> Result<RankTrend, RankError> {
    if history.len() < 2 {
        return Err(RankError::Trend(format!(
            "need at least 2 snapshots for trend analysis, got {}",
            history.len()
        )));
    }

    for window in history.windows(2) {
        if window[0].checked_at > window[1].checked_at {
            return Err(RankError::Trend(
                "snapshots must be in chronological order".to_string(),
            ));
        }
    }

    let first_position = history.first().and_then(|s| s.position);
    let last_position = history.last().and_then(|s| s.position);

    let ranked: Vec<u16> = history.iter().filter_map(|s| s.position).collect();
    let best_position = ranked.iter().copied().min();
    let avg_position = if ranked.is_empty() {
        None
    } else {
        let sum: f64 = ranked.iter().map(|p| f64::from(*p)).sum();
        Some(sum / ranked.len() as f64)
    };

    let change = match (first_position, last_position) {
        (Some(first), Some(last)) => i32::from(last) - i32::from(first),
        (None, Some(last)) => i32::from(last) - i32::from(NOT_RANKED_POSITION),
        (Some(first), None) => i32::from(NOT_RANKED_POSITION) - i32::from(first),
        (None, None) => 0,
    };

    // Linear regression over the effective positions.
    let effective: Vec<f64> = history
        .iter()
        .map(|s| f64::from(s.position.unwrap_or(NOT_RANKED_POSITION)))
        .collect();
    let n = effective.len() as f64;
    let sum_x: f64 = (0..effective.len()).map(|i| i as f64).sum();
    let sum_y: f64 = effective.iter().sum();
    let sum_xy: f64 = effective
        .iter()
        .enumerate()
        .map(|(i, &y)| i as f64 * y)
        .sum();
    let sum_x2: f64 = (0..effective.len()).map(|i| (i as f64) * (i as f64)).sum();

    let denom = n * sum_x2 - sum_x * sum_x;
    let slope = if denom.abs() > f64::EPSILON {
        (n * sum_xy - sum_x * sum_y) / denom
    } else {
        0.0
    };

    let avg = if n > 0.0 { sum_y / n } else { 0.0 };
    let normalized_slope = if avg > f64::EPSILON { slope / avg } else { 0.0 };

    // Positions shrink as rankings improve, so a negative slope means
    // the keyword is moving toward the top.
    let direction = if normalized_slope < -0.05 {
        RankTrendDirection::Improving
    } else if normalized_slope > 0.05 {
        RankTrendDirection::Declining
    } else {
        RankTrendDirection::Stable
    };

    Ok(RankTrend {
        points: history.len(),
        first_position,
        last_position,
        best_position,
        avg_position,
        change,
        direction,
    })
}

/// Serialize a [`RankTrend`] to JSON.
///
/// # Errors
///
/// Returns the serialization error, if any.
pub fn trend_to_json(trend: &RankTrend, pretty: bool) -> Result<String, serde_json::Error> {
    if pretty {
        serde_json::to_string_pretty(trend)
    } else {
        serde_json::to_string(trend)
    }
}

/// Render a [`RankTrend`] as a small Markdown report.
#[must_use]
pub fn trend_to_markdown(trend: &RankTrend) -> String {
    let fmt_pos = |p: Option<u16>| {
        p.map(|v| v.to_string())
            .unwrap_or_else(|| "not ranked".to_string())
    };
    let mut md = String::new();
    md.push_str("# Rank Trend\n\n");
    md.push_str(&format!(
        "- **Points:** {}\n\
         - **Direction:** {:?}\n\
         - **First position:** {}\n\
         - **Last position:** {}\n\
         - **Best position:** {}\n\
         - **Average position:** {}\n\
         - **Change:** {:+}\n",
        trend.points,
        trend.direction,
        fmt_pos(trend.first_position),
        fmt_pos(trend.last_position),
        fmt_pos(trend.best_position),
        trend
            .avg_position
            .map(|v| format!("{v:.1}"))
            .unwrap_or_else(|| "n/a".to_string()),
        trend.change,
    ));
    md
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn snapshot(minutes: i64, position: Option<u16>) -> RankSnapshot {
        RankSnapshot {
            keyword_id: "kw1".to_string(),
            checked_at: Utc.timestamp_opt(0, 0).single().unwrap_or_else(Utc::now)
                + chrono::Duration::minutes(minutes),
            position,
            source: "test".to_string(),
        }
    }

    #[test]
    fn test_insufficient_data() {
        let err = analyze_rank_trend(&[snapshot(0, Some(5))]).unwrap_err();
        assert!(err.to_string().contains("at least 2"));
    }

    #[test]
    fn test_improving_trend() {
        let history = vec![
            snapshot(0, Some(20)),
            snapshot(60, Some(15)),
            snapshot(120, Some(10)),
            snapshot(180, Some(5)),
        ];
        let trend = analyze_rank_trend(&history).unwrap();
        assert_eq!(trend.direction, RankTrendDirection::Improving);
        assert_eq!(trend.best_position, Some(5));
        assert_eq!(trend.change, -15);
        assert_eq!(trend.points, 4);
    }

    #[test]
    fn test_declining_trend() {
        let history = vec![
            snapshot(0, Some(3)),
            snapshot(60, Some(10)),
            snapshot(120, Some(20)),
            snapshot(180, Some(40)),
        ];
        let trend = analyze_rank_trend(&history).unwrap();
        assert_eq!(trend.direction, RankTrendDirection::Declining);
        assert_eq!(trend.change, 37);
    }

    #[test]
    fn test_stable_trend() {
        let history = vec![
            snapshot(0, Some(5)),
            snapshot(60, Some(6)),
            snapshot(120, Some(5)),
            snapshot(180, Some(6)),
        ];
        let trend = analyze_rank_trend(&history).unwrap();
        assert_eq!(trend.direction, RankTrendDirection::Stable);
        assert_eq!(trend.change, 1);
    }

    #[test]
    fn test_not_ranked_excluded_from_average() {
        let history = vec![
            snapshot(0, Some(2)),
            snapshot(60, None),
            snapshot(120, Some(4)),
        ];
        let trend = analyze_rank_trend(&history).unwrap();
        assert_eq!(trend.avg_position, Some(3.0));
        assert_eq!(trend.best_position, Some(2));
        // Both endpoints are ranked, so the NOT_RANKED sentinel is not used.
        assert_eq!(trend.change, 2);
    }

    #[test]
    fn test_all_not_ranked() {
        let history = vec![snapshot(0, None), snapshot(60, None)];
        let trend = analyze_rank_trend(&history).unwrap();
        assert_eq!(trend.direction, RankTrendDirection::Stable);
        assert_eq!(trend.avg_position, None);
        assert_eq!(trend.change, 0);
    }

    #[test]
    fn test_out_of_order_rejected() {
        let history = vec![snapshot(120, Some(5)), snapshot(60, Some(6))];
        let err = analyze_rank_trend(&history).unwrap_err();
        assert!(err.to_string().contains("chronological"));
    }

    #[test]
    fn test_trend_to_json_and_markdown() {
        let history = vec![snapshot(0, Some(9)), snapshot(60, Some(4))];
        let trend = analyze_rank_trend(&history).unwrap();
        let json = trend_to_json(&trend, true).unwrap();
        assert!(json.contains("Improving"));
        let md = trend_to_markdown(&trend);
        assert!(md.contains("# Rank Trend"));
        assert!(md.contains("Best position:** 4"));
    }
}
