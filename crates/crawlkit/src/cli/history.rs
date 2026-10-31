//! `crawlkit history` (§G6): print the score-history trend recorded by
//! scheduled runs.
//!
//! Reads the `history` table (`run_id, ts, page, category, metric, value`)
//! through the engine's [`HistoryStore`] — its own read connection, safe
//! alongside a running API server in WAL mode. The weighted scoring engine
//! (§G2) does not exist yet, so the recorded metric is finding counts
//! (`metric = "findings"`); the output says so explicitly.

use anyhow::{bail, Context, Result};
use crawlkit_engine::history::HistoryStore;
use std::path::{Path, PathBuf};

/// Execute `crawlkit history`.
pub fn run(target: &str, db_flag: Option<&Path>) -> Result<()> {
    let (db_file, run_filter) = resolve(target, db_flag)?;

    if !db_file.exists() {
        bail!("database not found: {}", db_file.display());
    }

    let store = HistoryStore::open(&db_file)
        .with_context(|| format!("failed to open history in {}", db_file.display()))?;

    let runs = store.list_runs()?;
    let series = store.trend(run_filter.as_deref())?;

    if runs.is_empty() {
        println!("no history recorded yet in {}", db_file.display());
        println!(
            "scheduled runs append snapshots automatically; metric = finding counts until the \
             scoring engine (GAP_CLOSURE_PLAN §G2) lands"
        );
        return Ok(());
    }

    println!("History in {} ({} runs)", db_file.display(), runs.len());
    println!();
    println!("{:<40} {:<25} {:>8}", "RUN", "FIRST SNAPSHOT", "ROWS");
    for r in &runs {
        println!("{:<40} {:<25} {:>8}", r.run_id, r.first_ts, r.rows);
    }

    println!();
    if run_filter.is_some() {
        println!("Trend (metric = finding counts; positive delta = more findings):");
    } else {
        println!("Trend across runs (metric = finding counts; positive delta = more findings):");
    }
    println!();
    println!(
        "{:<50} {:<12} {:>8} {:>8} {:>8} {:>6}",
        "PAGE", "CATEGORY", "FIRST", "LATEST", "DELTA", "RUNS"
    );

    for s in &series {
        let first = s
            .first_value()
            .map(|v| format!("{v:.0}"))
            .unwrap_or_else(|| "—".into());
        let last = s
            .last_value()
            .map(|v| format!("{v:.0}"))
            .unwrap_or_else(|| "—".into());
        let delta = s
            .delta()
            .map(|v| {
                if v > 0.0 {
                    format!("+{v:.0}")
                } else {
                    format!("{v:.0}")
                }
            })
            .unwrap_or_else(|| "—".into());
        println!(
            "{:<50} {:<12} {:>8} {:>8} {:>8} {:>6}",
            truncate(&s.page, 50),
            truncate(&s.category, 12),
            first,
            last,
            delta,
            s.samples.len(),
        );
    }

    Ok(())
}

/// Resolve the positional target + `--db` flag into a database file and an
/// optional run-id filter.
fn resolve(target: &str, db_flag: Option<&Path>) -> Result<(PathBuf, Option<String>)> {
    let as_path = Path::new(target);
    if as_path.is_file() || as_path.is_dir() {
        let db_file = if as_path.is_dir() {
            as_path.join("crawlkit.db")
        } else {
            as_path.to_path_buf()
        };
        return Ok((db_file, None));
    }

    // Not a path → treat as a run id; the database must come from elsewhere.
    let db_file = match db_flag {
        Some(db) => {
            if db.is_dir() {
                db.join("crawlkit.db")
            } else {
                db.to_path_buf()
            }
        }
        None => std::env::var_os("CRAWLKIT_DB")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("crawlkit.db")),
    };
    Ok((db_file, Some(target.to_string())))
}

fn truncate(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        value.to_string()
    } else {
        let kept: String = value.chars().take(max.saturating_sub(1)).collect();
        format!("{kept}…")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_resolution_db_when_path_exists() {
        let dir = tempfile::tempdir().unwrap();
        let (db, run) = resolve(dir.path().to_str().unwrap(), None).unwrap();
        assert_eq!(db, dir.path().join("crawlkit.db"));
        assert!(run.is_none());

        let file = dir.path().join("custom.db");
        std::fs::write(&file, b"").unwrap();
        let (db, run) = resolve(file.to_str().unwrap(), None).unwrap();
        assert_eq!(db, file);
        assert!(run.is_none());
    }

    #[test]
    fn target_resolution_run_id_uses_db_flag() {
        let dir = tempfile::tempdir().unwrap();
        let (db, run) = resolve("run-abc-123", Some(&dir.path().join("state.db"))).unwrap();
        assert_eq!(db, dir.path().join("state.db"));
        assert_eq!(run.as_deref(), Some("run-abc-123"));
    }

    #[test]
    fn truncate_keeps_output_aligned() {
        assert_eq!(truncate("short", 50), "short");
        assert_eq!(truncate(&"x".repeat(60), 50).chars().count(), 50);
    }
}
