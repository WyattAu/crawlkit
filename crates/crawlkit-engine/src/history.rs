//! Score history store (GAP_CLOSURE_PLAN §G6).
//!
//! Every scheduled run appends one snapshot row per `(page, category)`
//! finding count into the `history` table:
//!
//! ```sql
//! history(run_id TEXT, ts TEXT, page TEXT, category TEXT, metric TEXT, value REAL)
//! ```
//!
//! Snapshots carry two metrics side by side, distinguished by the `metric`
//! column (the schema needed no migration for the second one):
//!
//! - `metric = "findings"` — defect counts per `(page, category)`, appended
//!   by [`append_findings`] (what storage snapshots wrote before §G2).
//! - `metric = "score"` — weighted scores per `(page, category)` in
//!   `[0, 100]`, appended by [`append_scores`] (§G2). For scores, a
//!   *positive* delta is an improvement, the inverse of finding counts.
//!
//! The table is created additively and idempotently
//! ([`ensure_table`]) following the storage-layer migration pattern:
//! `CREATE TABLE IF NOT EXISTS` on open, no destructive steps. This module
//! owns the DDL — `storage::create_schema` calls [`ensure_table`] so a
//! fresh `crawlkit.db` always has the table, and opening a
//! [`HistoryStore`] on an older database migrates it in place.

use std::path::Path;

use rusqlite::Connection;

/// Errors produced by history operations.
#[derive(Debug, thiserror::Error)]
pub enum HistoryError {
    /// SQLite error.
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
}

/// Metric name for finding-count snapshots (defect counts per
/// `(page, category)`), recorded since the pre-§G2 era.
pub const FINDINGS_METRIC: &str = "findings";

/// Metric name for weighted-score snapshots (§G2): one row per
/// `(page, category)` with the category score in `[0, 100]`.
pub const SCORE_METRIC: &str = "score";

/// One snapshot row.
#[derive(Debug, Clone, PartialEq)]
pub struct HistoryRow {
    /// Crawl (run) identifier this snapshot belongs to.
    pub run_id: String,
    /// RFC 3339 timestamp of the snapshot.
    pub ts: String,
    /// Page the measurement refers to (URL, or page id when the URL row is
    /// unavailable).
    pub page: String,
    /// Finding category (e.g. `seo`, `a11y`).
    pub category: String,
    /// Metric name ([`FINDINGS_METRIC`] counts or [`SCORE_METRIC`] values).
    pub metric: String,
    /// Measured value.
    pub value: f64,
}

/// Create the `history` table and its indexes if they do not exist.
///
/// Idempotent: safe to call on every open, on brand-new and pre-existing
/// databases alike. This is the single source of truth for the history DDL.
///
/// # Errors
///
/// Returns [`HistoryError`] when the DDL fails.
pub fn ensure_table(conn: &Connection) -> Result<(), HistoryError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS history (
            run_id   TEXT NOT NULL,
            ts       TEXT NOT NULL,
            page     TEXT NOT NULL,
            category TEXT NOT NULL,
            metric   TEXT NOT NULL,
            value    REAL NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_history_run ON history(run_id);
        CREATE INDEX IF NOT EXISTS idx_history_series ON history(page, category, metric);",
    )?;
    Ok(())
}

/// Append one snapshot row.
///
/// # Errors
///
/// Returns [`HistoryError`] when the insert fails.
pub fn append(conn: &Connection, row: &HistoryRow) -> Result<(), HistoryError> {
    ensure_table(conn)?;
    conn.execute(
        "INSERT INTO history (run_id, ts, page, category, metric, value)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![
            row.run_id,
            row.ts,
            row.page,
            row.category,
            row.metric,
            row.value
        ],
    )?;
    Ok(())
}

/// Append finding-count snapshots for one run: one row per
/// `(page, category)` with `metric = "findings"` and `value = count`.
///
/// # Errors
///
/// Returns [`HistoryError`] when any insert fails.
pub fn append_findings(
    conn: &Connection,
    run_id: &str,
    ts: &str,
    counts: &[(String, String, i64)],
) -> Result<usize, HistoryError> {
    ensure_table(conn)?;
    let mut stmt = conn.prepare(
        "INSERT INTO history (run_id, ts, page, category, metric, value)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )?;
    let mut inserted = 0usize;
    for (page, category, count) in counts {
        inserted += stmt.execute(rusqlite::params![
            run_id,
            ts,
            page,
            category,
            FINDINGS_METRIC,
            *count as f64
        ])?;
    }
    Ok(inserted)
}

/// Append weighted-score snapshots for one run: one row per
/// `(page, category)` with `metric = "score"` and `value` in `[0, 100]`
/// (§G2). Values are clamped to `[0, 100]` so a stray out-of-range input
/// cannot distort trend charts downstream.
///
/// Scores are stored under their own [`SCORE_METRIC`] namespace, so
/// [`trend`] keeps them as separate series from [`FINDINGS_METRIC`] rows
/// for the same `(page, category)` — no migration, the schema already
/// carries `metric TEXT`.
///
/// # Errors
///
/// Returns [`HistoryError`] when any insert fails.
pub fn append_scores(
    conn: &Connection,
    run_id: &str,
    ts: &str,
    scores: &[(String, String, f64)],
) -> Result<usize, HistoryError> {
    ensure_table(conn)?;
    let mut stmt = conn.prepare(
        "INSERT INTO history (run_id, ts, page, category, metric, value)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )?;
    let mut inserted = 0usize;
    for (page, category, score) in scores {
        inserted += stmt.execute(rusqlite::params![
            run_id,
            ts,
            page,
            category,
            SCORE_METRIC,
            score.clamp(0.0, 100.0)
        ])?;
    }
    Ok(inserted)
}

/// One `(page, category, metric)` series sampled across runs, in
/// chronological order.
#[derive(Debug, Clone, PartialEq)]
pub struct TrendSeries {
    /// Page the series refers to.
    pub page: String,
    /// Finding category.
    pub category: String,
    /// Metric name.
    pub metric: String,
    /// `(run_id, ts, value)` samples ordered by `ts` then `run_id`.
    pub samples: Vec<(String, String, f64)>,
}

impl TrendSeries {
    /// First recorded value (empty series → `None`).
    #[must_use]
    pub fn first_value(&self) -> Option<f64> {
        self.samples.first().map(|(_, _, v)| *v)
    }

    /// Most recent value (empty series → `None`).
    #[must_use]
    pub fn last_value(&self) -> Option<f64> {
        self.samples.last().map(|(_, _, v)| *v)
    }

    /// `last − first`; positive means the metric increased (for finding
    /// counts, that is a regression).
    #[must_use]
    pub fn delta(&self) -> Option<f64> {
        Some(self.last_value()? - self.first_value()?)
    }
}

/// All trend series for one metric, optionally restricted to a single run
/// id (rows are still grouped across runs, so pass `None` for a real
/// trend).
///
/// Series are ordered by `(page, category, metric)` for deterministic
/// output.
///
/// # Errors
///
/// Returns [`HistoryError`] when the query fails.
pub fn trend(
    conn: &Connection,
    run_filter: Option<&str>,
) -> Result<Vec<TrendSeries>, HistoryError> {
    ensure_table(conn)?;
    let mut stmt = conn.prepare(
        "SELECT run_id, ts, page, category, metric, value FROM history
         WHERE (?1 IS NULL OR run_id = ?1)
         ORDER BY page, category, metric, ts, run_id",
    )?;
    let rows = stmt.query_map([run_filter], |row| {
        Ok(HistoryRow {
            run_id: row.get(0)?,
            ts: row.get(1)?,
            page: row.get(2)?,
            category: row.get(3)?,
            metric: row.get(4)?,
            value: row.get(5)?,
        })
    })?;

    let mut series: Vec<TrendSeries> = Vec::new();
    for row in rows {
        let row = row?;
        match series.last_mut() {
            Some(s)
                if s.page == row.page && s.category == row.category && s.metric == row.metric =>
            {
                s.samples.push((row.run_id, row.ts, row.value));
            }
            _ => series.push(TrendSeries {
                page: row.page,
                category: row.category,
                metric: row.metric,
                samples: vec![(row.run_id, row.ts, row.value)],
            }),
        }
    }
    Ok(series)
}

/// Summary of one run present in the history table.
#[derive(Debug, Clone, PartialEq)]
pub struct RunSummary {
    /// Run identifier.
    pub run_id: String,
    /// Earliest snapshot timestamp in the run.
    pub first_ts: String,
    /// Number of snapshot rows.
    pub rows: i64,
}

/// List runs that have history rows, oldest first.
///
/// # Errors
///
/// Returns [`HistoryError`] when the query fails.
pub fn list_runs(conn: &Connection) -> Result<Vec<RunSummary>, HistoryError> {
    ensure_table(conn)?;
    let mut stmt = conn.prepare(
        "SELECT run_id, MIN(ts), COUNT(*) FROM history
         GROUP BY run_id ORDER BY MIN(ts), run_id",
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok(RunSummary {
                run_id: row.get(0)?,
                first_ts: row.get(1)?,
                rows: row.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Read-only handle over the `history` table of a crawlkit database.
///
/// Opens its own connection (WAL mode allows concurrent readers alongside
/// the storage connection) so the CLI can read trends without going
/// through the storage layer.
#[derive(Debug)]
pub struct HistoryStore {
    conn: Connection,
}

impl HistoryStore {
    /// Open (or create) the history table on the database at `path`.
    ///
    /// # Errors
    ///
    /// Returns [`HistoryError`] when the database cannot be opened or the
    /// schema cannot be created.
    pub fn open(path: &Path) -> Result<Self, HistoryError> {
        let conn = Connection::open(path)?;
        ensure_table(&conn)?;
        Ok(Self { conn })
    }

    /// Append one snapshot row.
    ///
    /// # Errors
    ///
    /// Returns [`HistoryError`] when the insert fails.
    pub fn append(&self, row: &HistoryRow) -> Result<(), HistoryError> {
        append(&self.conn, row)
    }

    /// Append finding-count snapshots for one run.
    ///
    /// # Errors
    ///
    /// Returns [`HistoryError`] when any insert fails.
    pub fn append_findings(
        &self,
        run_id: &str,
        ts: &str,
        counts: &[(String, String, i64)],
    ) -> Result<usize, HistoryError> {
        append_findings(&self.conn, run_id, ts, counts)
    }

    /// Append weighted-score snapshots for one run (§G2).
    ///
    /// # Errors
    ///
    /// Returns [`HistoryError`] when any insert fails.
    pub fn append_scores(
        &self,
        run_id: &str,
        ts: &str,
        scores: &[(String, String, f64)],
    ) -> Result<usize, HistoryError> {
        append_scores(&self.conn, run_id, ts, scores)
    }

    /// All trend series, optionally restricted to one run id.
    ///
    /// # Errors
    ///
    /// Returns [`HistoryError`] when the query fails.
    pub fn trend(&self, run_filter: Option<&str>) -> Result<Vec<TrendSeries>, HistoryError> {
        trend(&self.conn, run_filter)
    }

    /// Runs with history rows, oldest first.
    ///
    /// # Errors
    ///
    /// Returns [`HistoryError`] when the query fails.
    pub fn list_runs(&self) -> Result<Vec<RunSummary>, HistoryError> {
        list_runs(&self.conn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(run: &str, ts: &str, page: &str, category: &str, value: f64) -> HistoryRow {
        HistoryRow {
            run_id: run.to_string(),
            ts: ts.to_string(),
            page: page.to_string(),
            category: category.to_string(),
            metric: FINDINGS_METRIC.to_string(),
            value,
        }
    }

    #[test]
    fn migration_creates_table_and_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.db");

        {
            let conn = Connection::open(&path).unwrap();
            ensure_table(&conn).unwrap();
            // Simulate a pre-G6 database: the table already exists with the
            // same shape — ensure_table must not fail or duplicate.
            ensure_table(&conn).unwrap();
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'history'",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(count, 1, "exactly one history table");
        }

        // Re-open through the store (fresh connection, same file): the
        // migration must be a no-op and keep existing rows.
        let store = HistoryStore::open(&path).unwrap();
        store
            .append(&row("r1", "2026-10-31T00:00:00Z", "https://x/", "seo", 3.0))
            .unwrap();
        drop(store);
        let reopened = HistoryStore::open(&path).unwrap();
        assert_eq!(reopened.list_runs().unwrap().len(), 1);
    }

    #[test]
    fn snapshot_insert_and_trend_delta() {
        let dir = tempfile::tempdir().unwrap();
        let store = HistoryStore::open(&dir.path().join("history.db")).unwrap();

        let inserted = store
            .append_findings(
                "run-1",
                "2026-10-31T00:00:00Z",
                &[
                    ("https://x/".into(), "seo".into(), 2),
                    ("https://x/".into(), "a11y".into(), 1),
                ],
            )
            .unwrap();
        assert_eq!(inserted, 2);
        store
            .append_findings(
                "run-2",
                "2026-10-31T01:00:00Z",
                &[
                    ("https://x/".into(), "seo".into(), 5),
                    ("https://x/".into(), "a11y".into(), 1),
                ],
            )
            .unwrap();

        let series = store.trend(None).unwrap();
        assert_eq!(series.len(), 2, "one series per (page, category)");
        assert_eq!(series[0].category, "a11y", "deterministic category order");
        assert_eq!(series[1].category, "seo");

        let seo = &series[1];
        assert_eq!(seo.first_value(), Some(2.0));
        assert_eq!(seo.last_value(), Some(5.0));
        assert_eq!(seo.delta(), Some(3.0));
        assert_eq!(seo.samples.len(), 2);

        // Run filter only sees that run's samples.
        let filtered = store.trend(Some("run-2")).unwrap();
        assert!(filtered.iter().all(|s| s.samples.len() == 1));
    }

    #[test]
    fn list_runs_orders_chronologically() {
        let dir = tempfile::tempdir().unwrap();
        let store = HistoryStore::open(&dir.path().join("history.db")).unwrap();
        store
            .append(&row("r2", "2026-10-31T02:00:00Z", "p", "seo", 1.0))
            .unwrap();
        store
            .append(&row("r1", "2026-10-31T01:00:00Z", "p", "seo", 1.0))
            .unwrap();

        let runs = store.list_runs().unwrap();
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].run_id, "r1");
        assert_eq!(runs[1].run_id, "r2");
        assert_eq!(runs[0].rows, 1);
    }

    #[test]
    fn empty_trend_and_delta_of_empty_series() {
        let dir = tempfile::tempdir().unwrap();
        let store = HistoryStore::open(&dir.path().join("history.db")).unwrap();
        assert!(store.trend(None).unwrap().is_empty());
        assert!(store.list_runs().unwrap().is_empty());

        let s = TrendSeries {
            page: "p".into(),
            category: "seo".into(),
            metric: FINDINGS_METRIC.into(),
            samples: vec![],
        };
        assert_eq!(s.delta(), None);
    }

    // --- §G2: score metric rows ---

    #[test]
    fn score_rows_append_under_their_own_metric_and_never_mix_with_findings() {
        let dir = tempfile::tempdir().unwrap();
        let store = HistoryStore::open(&dir.path().join("history.db")).unwrap();

        let inserted = store
            .append_scores(
                "run-1",
                "2026-11-01T00:00:00Z",
                &[
                    ("https://x/".into(), "seo".into(), 87.5),
                    ("https://x/".into(), "security".into(), 25.0),
                ],
            )
            .unwrap();
        assert_eq!(inserted, 2);

        // Same (page, category) also carries findings counts from the same
        // run: the metrics must land in separate trend series.
        store
            .append_findings(
                "run-1",
                "2026-11-01T00:00:00Z",
                &[("https://x/".into(), "seo".into(), 3)],
            )
            .unwrap();

        let series = store.trend(None).unwrap();
        assert_eq!(series.len(), 3, "two score series + one findings series");
        let seo_score = series
            .iter()
            .find(|s| s.metric == SCORE_METRIC && s.category == "seo")
            .unwrap();
        assert_eq!(seo_score.last_value(), Some(87.5));
        let seo_findings = series
            .iter()
            .find(|s| s.metric == FINDINGS_METRIC && s.category == "seo")
            .unwrap();
        assert_eq!(seo_findings.last_value(), Some(3.0));
    }

    #[test]
    fn score_trend_delta_across_runs_positive_means_improvement() {
        let dir = tempfile::tempdir().unwrap();
        let store = HistoryStore::open(&dir.path().join("history.db")).unwrap();

        store
            .append_scores(
                "run-1",
                "2026-11-01T00:00:00Z",
                &[("https://x/".into(), "seo".into(), 72.25)],
            )
            .unwrap();
        store
            .append_scores(
                "run-2",
                "2026-11-01T01:00:00Z",
                &[("https://x/".into(), "seo".into(), 91.0)],
            )
            .unwrap();

        let seo = store
            .trend(None)
            .unwrap()
            .into_iter()
            .find(|s| s.metric == SCORE_METRIC)
            .unwrap();
        assert_eq!(seo.samples.len(), 2);
        assert_eq!(seo.first_value(), Some(72.25));
        assert_eq!(seo.last_value(), Some(91.0));
        assert_eq!(
            seo.delta(),
            Some(18.75),
            "scores rise as findings are fixed"
        );
    }

    #[test]
    fn score_append_clamps_out_of_range_values_and_accepts_empty_input() {
        let dir = tempfile::tempdir().unwrap();
        let store = HistoryStore::open(&dir.path().join("history.db")).unwrap();

        assert_eq!(
            store
                .append_scores("run-1", "2026-11-01T00:00:00Z", &[])
                .unwrap(),
            0,
            "no rows for no scores"
        );
        store
            .append_scores(
                "run-1",
                "2026-11-01T00:00:00Z",
                &[
                    ("https://x/".into(), "seo".into(), 120.0),
                    ("https://x/".into(), "security".into(), -4.0),
                ],
            )
            .unwrap();

        let values: Vec<f64> = store
            .trend(None)
            .unwrap()
            .into_iter()
            .flat_map(|s| s.samples.into_iter().map(|(_, _, v)| v))
            .collect();
        assert_eq!(values, vec![0.0, 100.0], "clamped to [0, 100]");
    }
}
