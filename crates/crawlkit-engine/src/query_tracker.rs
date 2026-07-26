use serde::{Deserialize, Serialize};

/// Query position tracking and content gap analysis.
///
/// Tracks search engine positions over time and identifies
/// content opportunities where competitors rank but we don't.
pub struct QueryTracker {
    storage: crate::storage::Storage,
}

impl QueryTracker {
    /// Create a new query tracker.
    #[must_use]
    pub fn new(storage: crate::storage::Storage) -> Self {
        Self { storage }
    }

    /// Record daily query positions from GSC/Bing data.
    pub fn record_positions(
        &self,
        gsc_data: &[crate::backlink_adapters::GscSearchResult],
        bing_data: &[crate::backlink_adapters::BingQueryData],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.storage.conn();
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

        // Record GSC positions
        for query in gsc_data {
            conn.execute(
                "INSERT OR REPLACE INTO query_positions (query, date, source, clicks, impressions, position, ctr) 
                 VALUES (?1, ?2, 'gsc', ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    query.key,
                    today,
                    query.clicks,
                    query.impressions,
                    query.position,
                    query.ctr,
                ],
            )?;
        }

        // Record Bing positions
        for query in bing_data {
            conn.execute(
                "INSERT OR REPLACE INTO query_positions (query, date, source, clicks, impressions, position, ctr) 
                 VALUES (?1, ?2, 'bing', ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    query.query,
                    today,
                    query.clicks,
                    query.impressions,
                    query.avg_position,
                    query.ctr,
                ],
            )?;
        }

        Ok(())
    }

    /// Get low-ranking queries (position 6-20, close to top 5).
    pub fn get_low_ranking_queries(
        &self,
        min_position: f64,
        max_position: f64,
    ) -> Result<Vec<QueryWithPosition>, Box<dyn std::error::Error>> {
        let conn = self.storage.conn();
        let mut stmt = conn.prepare(
            "SELECT query, AVG(position) as avg_position, SUM(clicks) as total_clicks, SUM(impressions) as total_impressions
             FROM query_positions 
             WHERE position BETWEEN ?1 AND ?2
             GROUP BY query
             ORDER BY total_clicks DESC",
        )?;

        let rows = stmt.query_map(rusqlite::params![min_position, max_position], |row| {
            let clicks: u64 = row.get(2)?;
            let impressions: u64 = row.get(3)?;
            let ctr = if impressions > 0 {
                (clicks as f64 / impressions as f64) * 100.0
            } else {
                0.0
            };
            Ok(QueryWithPosition {
                query: row.get(0)?,
                position: row.get(1)?,
                clicks,
                impressions,
                ctr,
                trend: TrendDirection::Stable,
            })
        })?;

        let queries = rows.filter_map(|r| r.ok()).collect();
        Ok(queries)
    }
}

/// A query with its position data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryWithPosition {
    pub query: String,
    pub position: f64,
    pub clicks: u64,
    pub impressions: u64,
    /// Click-through rate as a percentage (0.0 - 100.0).
    pub ctr: f64,
    pub trend: TrendDirection,
}

/// Trend direction for a query.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TrendDirection {
    Improving,
    Declining,
    Stable,
}
