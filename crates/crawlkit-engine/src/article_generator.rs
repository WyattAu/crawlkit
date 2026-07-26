use serde::{Deserialize, Serialize};

use crate::content_gap::{ContentGap, ContentType, Opportunity};

/// Article outline generator for content gap opportunities.
///
/// Generates structured outlines for articles targeting
/// high-opportunity queries.
pub struct ArticleGenerator;

impl ArticleGenerator {
    /// Generate article outlines from content gaps.
    pub fn generate_outlines(gaps: &[ContentGap]) -> Vec<ArticleOutline> {
        gaps.iter().map(generate_outline).collect()
    }

    /// Generate article outlines from opportunities.
    pub fn generate_from_opportunities(opportunities: &[Opportunity]) -> Vec<ArticleOutline> {
        opportunities
            .iter()
            .map(|opp| ArticleOutline {
                title: generate_title(&opp.query),
                meta_description: generate_meta_description(&opp.query),
                target_query: opp.query.clone(),
                search_volume: 0, // Will be populated from gap data
                sections: generate_sections(&opp.query, &opp.content_type),
                internal_links: vec![], // Will be populated from site crawl
                word_count_target: match opp.content_type {
                    ContentType::BlogPost => 2000,
                    ContentType::ComparisonGuide => 2500,
                    ContentType::Tutorial => 1800,
                    ContentType::FAQ => 1500,
                    ContentType::ProductPage => 800,
                    ContentType::LandingPage => 1200,
                },
                content_type: opp.content_type.clone(),
            })
            .collect()
    }
}

/// Generate an article outline from a content gap.
fn generate_outline(gap: &ContentGap) -> ArticleOutline {
    ArticleOutline {
        title: generate_title(&gap.query),
        meta_description: generate_meta_description(&gap.query),
        target_query: gap.query.clone(),
        search_volume: gap.search_volume,
        sections: generate_sections(&gap.query, &gap.suggested_content_type),
        internal_links: vec![], // Will be populated from site crawl
        word_count_target: match gap.suggested_content_type {
            ContentType::BlogPost => 2000,
            ContentType::ComparisonGuide => 2500,
            ContentType::Tutorial => 1800,
            ContentType::FAQ => 1500,
            ContentType::ProductPage => 800,
            ContentType::LandingPage => 1200,
        },
        content_type: gap.suggested_content_type.clone(),
    }
}

/// Generate a title for a query.
fn generate_title(query: &str) -> String {
    // Title case the query and add appropriate suffix
    let title = query
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    let upper: String = first.to_uppercase().collect();
                    upper + chars.as_str()
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    format!("{}: Complete Guide", title)
}

/// Generate a meta description for a query.
fn generate_meta_description(query: &str) -> String {
    format!(
        "Comprehensive guide to {}. Learn everything you need to know about {} from Kingston Peptides.",
        query.to_lowercase(),
        query.to_lowercase()
    )
}

/// Generate sections for an article.
fn generate_sections(query: &str, content_type: &ContentType) -> Vec<ArticleSection> {
    match content_type {
        ContentType::ComparisonGuide => vec![
            ArticleSection {
                heading: format!("What is {}?", query),
                key_points: vec![
                    "Definition and overview".to_string(),
                    "Key characteristics".to_string(),
                    "Common use cases".to_string(),
                ],
                word_count_target: 300,
            },
            ArticleSection {
                heading: "Comparison Criteria".to_string(),
                key_points: vec![
                    "Effectiveness".to_string(),
                    "Safety profile".to_string(),
                    "Cost".to_string(),
                ],
                word_count_target: 400,
            },
            ArticleSection {
                heading: "Results and Analysis".to_string(),
                key_points: vec![
                    "Research findings".to_string(),
                    "User experiences".to_string(),
                    "Expert opinions".to_string(),
                ],
                word_count_target: 500,
            },
            ArticleSection {
                heading: "Conclusion".to_string(),
                key_points: vec![
                    "Summary of findings".to_string(),
                    "Recommendations".to_string(),
                ],
                word_count_target: 300,
            },
        ],
        ContentType::Tutorial => vec![
            ArticleSection {
                heading: "Getting Started".to_string(),
                key_points: vec!["Prerequisites".to_string(), "Equipment needed".to_string()],
                word_count_target: 200,
            },
            ArticleSection {
                heading: "Step-by-Step Guide".to_string(),
                key_points: vec![
                    "Step 1: Preparation".to_string(),
                    "Step 2: Procedure".to_string(),
                    "Step 3: Storage".to_string(),
                ],
                word_count_target: 800,
            },
            ArticleSection {
                heading: "Safety Considerations".to_string(),
                key_points: vec![
                    "Handling precautions".to_string(),
                    "Storage requirements".to_string(),
                ],
                word_count_target: 300,
            },
        ],
        _ => vec![
            ArticleSection {
                heading: "Overview".to_string(),
                key_points: vec!["What is it?".to_string(), "Key benefits".to_string()],
                word_count_target: 300,
            },
            ArticleSection {
                heading: "How It Works".to_string(),
                key_points: vec![
                    "Mechanism of action".to_string(),
                    "Research evidence".to_string(),
                ],
                word_count_target: 500,
            },
            ArticleSection {
                heading: "Dosage and Administration".to_string(),
                key_points: vec![
                    "Recommended dosage".to_string(),
                    "Administration method".to_string(),
                ],
                word_count_target: 400,
            },
            ArticleSection {
                heading: "Safety and Side Effects".to_string(),
                key_points: vec![
                    "Known side effects".to_string(),
                    "Contraindications".to_string(),
                ],
                word_count_target: 300,
            },
        ],
    }
}

/// An article outline for content creation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArticleOutline {
    pub title: String,
    pub meta_description: String,
    pub target_query: String,
    pub search_volume: u64,
    pub sections: Vec<ArticleSection>,
    pub internal_links: Vec<String>,
    pub word_count_target: u32,
    pub content_type: ContentType,
}

/// A section within an article outline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArticleSection {
    pub heading: String,
    pub key_points: Vec<String>,
    pub word_count_target: u32,
}
