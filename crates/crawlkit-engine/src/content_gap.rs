use serde::{Deserialize, Serialize};

use crate::backlink_adapters::{BingWebmasterAdapter, GscAdapter};
use crate::query_tracker::QueryWithPosition;

/// Content gap analysis engine.
///
/// Finds queries where competitors rank but we don't,
/// and identifies high-opportunity queries close to top 5.
pub struct ContentGapAnalyzer {
    #[allow(dead_code)]
    gsc_adapter: Option<GscAdapter>,
    #[allow(dead_code)]
    bing_adapter: Option<BingWebmasterAdapter>,
}

impl ContentGapAnalyzer {
    /// Create a new content gap analyzer.
    #[must_use]
    pub fn new(
        gsc_adapter: Option<GscAdapter>,
        bing_adapter: Option<BingWebmasterAdapter>,
    ) -> Self {
        Self {
            gsc_adapter,
            bing_adapter,
        }
    }

    /// Find content gaps (queries where competitors rank but we don't).
    pub fn find_gaps(
        &self,
        our_queries: &[QueryWithPosition],
        competitor_queries: &[QueryWithPosition],
    ) -> Vec<ContentGap> {
        let our_query_set: std::collections::HashSet<&str> =
            our_queries.iter().map(|q| q.query.as_str()).collect();

        competitor_queries
            .iter()
            .filter(|q| !our_query_set.contains(q.query.as_str()))
            .map(|q| ContentGap {
                query: q.query.clone(),
                search_volume: q.impressions,
                competitor_position: q.position,
                our_position: None,
                opportunity_score: calculate_opportunity_score(q),
                suggested_content_type: suggest_content_type(&q.query),
            })
            .collect()
    }

    /// Find high-opportunity queries (ranking 6-20, close to top 5).
    pub fn find_opportunities(&self, queries: &[QueryWithPosition]) -> Vec<Opportunity> {
        queries
            .iter()
            .filter(|q| q.position >= 6.0 && q.position <= 20.0)
            .map(|q| Opportunity {
                query: q.query.clone(),
                current_position: q.position,
                target_position: 5.0,
                estimated_traffic_gain: estimate_traffic_gain(q),
                content_type: suggest_content_type(&q.query),
            })
            .collect()
    }
}

/// A content gap (query where competitor ranks but we don't).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentGap {
    pub query: String,
    pub search_volume: u64,
    pub competitor_position: f64,
    pub our_position: Option<f64>,
    pub opportunity_score: f64,
    pub suggested_content_type: ContentType,
}

/// A high-opportunity query (ranking 6-20).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Opportunity {
    pub query: String,
    pub current_position: f64,
    pub target_position: f64,
    pub estimated_traffic_gain: u64,
    pub content_type: ContentType,
}

/// Content type for addressing a gap.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ContentType {
    BlogPost,
    ProductPage,
    ComparisonGuide,
    FAQ,
    Tutorial,
    LandingPage,
}

/// Calculate opportunity score for a content gap.
fn calculate_opportunity_score(query: &QueryWithPosition) -> f64 {
    // Score based on search volume and competitor position
    let volume_score = (query.impressions as f64).log10() * 10.0;
    let position_score = (20.0 - query.position).max(0.0);
    volume_score + position_score
}

/// Estimate traffic gain from moving to top 5.
fn estimate_traffic_gain(query: &QueryWithPosition) -> u64 {
    // Rough estimate: top 5 gets ~30% of impressions
    // Current position gets ~5-10% of impressions
    let estimated_top5 = (query.impressions as f64 * 0.30) as u64;
    let current_traffic = (query.impressions as f64 * 0.05) as u64;
    estimated_top5.saturating_sub(current_traffic)
}

/// Suggest content type based on query characteristics.
fn suggest_content_type(query: &str) -> ContentType {
    let query_lower = query.to_lowercase();

    if query_lower.contains("vs") || query_lower.contains("compare") {
        ContentType::ComparisonGuide
    } else if query_lower.contains("how to") || query_lower.contains("guide") {
        ContentType::Tutorial
    } else if query_lower.contains("what is") || query_lower.contains("faq") {
        ContentType::FAQ
    } else if query_lower.contains("buy") || query_lower.contains("price") {
        ContentType::ProductPage
    } else {
        ContentType::BlogPost
    }
}
