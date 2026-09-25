//! `crawlkit usage` — the ADR-017 §4 read-only operator mirror of the
//! per-tenant usage metering rollups.

use anyhow::{Context, Result};
use crawlkit_engine::metering::{day_floor, MeteredUnit};
use crawlkit_engine::storage::Storage;

/// Parameters for the usage read.
pub struct UsageParams {
    /// Path to the crawlkit storage database.
    pub db: std::path::PathBuf,
    /// Tenant id to report usage for.
    pub tenant: String,
    /// Inclusive start UTC date (YYYY-MM-DD).
    pub from: Option<String>,
    /// Inclusive end UTC date (YYYY-MM-DD).
    pub to: Option<String>,
}

/// Parses a `YYYY-MM-DD` date into a day-floor timestamp.
fn parse_day(s: &str, field: &str) -> Result<chrono::DateTime<chrono::Utc>> {
    let d = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .with_context(|| format!("`{field}` must be YYYY-MM-DD, got {s:?}"))?;
    d.and_hms_opt(0, 0, 0)
        .map(|dt| dt.and_utc())
        .context("`{field}` is not a valid date")
}

/// Runs the usage report.
pub fn run(params: UsageParams) -> Result<()> {
    let storage = Storage::new(&params.db).context("opening storage database")?;
    let to = match params.to.as_deref() {
        Some(s) => parse_day(s, "to")?,
        None => day_floor(chrono::Utc::now()),
    };
    let from = match params.from.as_deref() {
        Some(s) => parse_day(s, "from")?,
        None => to - chrono::Duration::days(30),
    };
    anyhow::ensure!(from <= to, "`from` must not be after `to`");

    let rows = storage
        .get_usage(&params.tenant, from, to)
        .context("reading usage rollups")?;

    // Group by day for a compact table.
    println!("Usage for tenant `{}` ({from}..{to}, UTC):", params.tenant);
    if rows.is_empty() {
        println!("  (no recorded usage in range)");
        return Ok(());
    }
    let mut current_day: Option<chrono::DateTime<chrono::Utc>> = None;
    for entry in &rows {
        if current_day != Some(entry.day_utc) {
            println!("  {}:", entry.day_utc.format("%Y-%m-%d"));
            current_day = Some(entry.day_utc);
        }
        let unit_label = match entry.unit {
            MeteredUnit::ExportBytes => format!("{} bytes", entry.total),
            other => format!("{} {}", entry.total, other),
        };
        println!("    {unit_label}");
    }
    Ok(())
}
