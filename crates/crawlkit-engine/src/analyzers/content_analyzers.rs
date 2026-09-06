#![allow(
    clippy::unwrap_used,
    clippy::manual_range_contains,
    clippy::redundant_closure,
    clippy::collapsible_if,
    clippy::unnecessary_map_or,
    clippy::default_constructed_unit_structs,
    clippy::needless_return,
    clippy::needless_range_loop,
    clippy::useless_format,
    clippy::if_same_then_else,
    clippy::derivable_impls,
    clippy::manual_pattern_char_comparison,
    clippy::manual_contains
)]
use std::collections::{HashMap, HashSet};

use regex::Regex;

use crate::parser::ExtractedLink;
use crate::types::{IssueCategory, Severity};

use super::{
    count_sentences, count_syllables, flesch_kincaid_grade, flesch_reading_ease, is_utility_page,
    AnalysisContext, Analyzer, Finding, STOP_WORDS,
};

/// Required properties per Schema.org type (subset).
const REQUIRED_PROPERTIES: &[(&str, &[&str])] = &[
    ("Article", &["headline", "author"]),
    ("NewsArticle", &["headline", "author"]),
    ("BlogPosting", &["headline", "author"]),
    ("ScholarlyArticle", &["headline", "author"]),
    ("Product", &["name"]),
    ("Organization", &["name"]),
    ("LocalBusiness", &["name", "address"]),
    ("Store", &["name", "address"]),
    ("WebPage", &["name"]),
    ("BreadcrumbList", &["itemListElement"]),
    ("FAQPage", &["mainEntity"]),
    ("HowTo", &["name"]),
    ("HowToStep", &["text"]),
    ("Event", &["name", "startDate"]),
    ("Recipe", &["name"]),
    ("VideoObject", &["name", "embedUrl"]),
    ("SoftwareApplication", &["name"]),
    ("Book", &["name"]),
    ("MusicAlbum", &["name"]),
    ("Movie", &["name"]),
    ("Quiz", &["name"]),
    ("Question", &["text"]),
    ("Answer", &["text"]),
    ("Drug", &["name"]),
    ("DefinedTerm", &["name"]),
    ("DefinedTermSet", &["name"]),
    ("Dataset", &["name"]),
];

/// Recognized Schema.org types for validation.
const RECOGNIZED_TYPES: &[&str] = &[
    // Core content types
    "Article",
    "NewsArticle",
    "BlogPosting",
    "ScholarlyArticle",
    // Product & commerce
    "Product",
    "Offer",
    "Brand",
    "AggregateRating",
    "Review",
    // Organization & people
    "Organization",
    "LocalBusiness",
    "Store",
    "Person",
    "Place",
    // Web
    "WebSite",
    "WebPage",
    "WebPageElement",
    // Navigation
    "BreadcrumbList",
    "ItemList",
    "ListItem",
    // Interactive content
    "FAQPage",
    "HowTo",
    "HowToStep",
    "HowToDirection",
    "HowToSupply",
    "HowToTool",
    "Quiz",
    "Question",
    "Answer",
    // Media
    "Event",
    "Recipe",
    "VideoObject",
    "ImageObject",
    "AudioObject",
    // Software
    "SoftwareApplication",
    "SoftwareSourceCode",
    // Books & media
    "Book",
    "MusicAlbum",
    "MusicRecording",
    "Movie",
    "TVSeries",
    // Knowledge & data
    "Dataset",
    "DataDownload",
    "DefinedTerm",
    "DefinedTermSet",
    // Medical
    "Drug",
    "MedicalWebPage",
    "MedicalSubstance",
    "MedicalAudience",
    "MedicalCondition",
    "MedicalProcedure",
    // Creative works
    "CreativeWork",
    "Course",
    "LearningResource",
    // Research & Data
    "ResearchProject",
    "CollectionPage",
];

pub struct StructuredDataValidator;

impl StructuredDataValidator {
    pub fn new() -> Self {
        Self
    }

    fn required_properties(schema_type: &str) -> &'static [&'static str] {
        REQUIRED_PROPERTIES
            .iter()
            .find(|(t, _)| *t == schema_type)
            .map(|(_, props)| *props)
            .unwrap_or(&[])
    }
}

impl Default for StructuredDataValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for StructuredDataValidator {
    fn name(&self) -> &str {
        "structured-data"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        if ctx.page.structured_data.is_empty() {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Schema,
                code: "SD001".to_string(),
                title: "No structured data found".to_string(),
                description: "No JSON-LD structured data was found on this page.".to_string(),
                url: url.clone(),
                recommendation: "Add relevant Schema.org JSON-LD markup to enhance search \
                                 results."
                    .to_string(),
            });
            return findings;
        }

        for sd in &ctx.page.structured_data {
            // 2.5 — Validate @context
            match &sd.context {
                None => {
                    findings.push(Finding {
                        severity: Severity::Error,
                        category: IssueCategory::Schema,
                        code: "SD002".to_string(),
                        title: "Missing @context".to_string(),
                        description: "JSON-LD block is missing the @context property.".to_string(),
                        url: url.clone(),
                        recommendation: "Add \"@context\": \"https://schema.org\" to all JSON-LD \
                                         blocks."
                            .to_string(),
                    });
                }
                Some(ctx_val) => {
                    if ctx_val != "https://schema.org" && ctx_val != "schema.org" {
                        findings.push(Finding {
                            severity: Severity::Warning,
                            category: IssueCategory::Schema,
                            code: "SD003".to_string(),
                            title: "Non-standard @context".to_string(),
                            description: format!(
                                "JSON-LD @context is \"{ctx_val}\" instead of \
                                 \"https://schema.org\"."
                            ),
                            url: url.clone(),
                            recommendation: "Use \"https://schema.org\" as the @context."
                                .to_string(),
                        });
                    }
                }
            }

            // Validate @type
            match &sd.r#type {
                None => {
                    findings.push(Finding {
                        severity: Severity::Error,
                        category: IssueCategory::Schema,
                        code: "SD004".to_string(),
                        title: "Missing @type".to_string(),
                        description: "JSON-LD block is missing the @type property.".to_string(),
                        url: url.clone(),
                        recommendation: "Add an appropriate @type to describe the content."
                            .to_string(),
                    });
                }
                Some(type_val) => {
                    if !RECOGNIZED_TYPES.contains(&type_val.as_str()) {
                        findings.push(Finding {
                            severity: Severity::Warning,
                            category: IssueCategory::Schema,
                            code: "SD005".to_string(),
                            title: "Unknown @type".to_string(),
                            description: format!(
                                "JSON-LD @type \"{type_val}\" is not a recognized Schema.org \
                                 type."
                            ),
                            url: url.clone(),
                            recommendation: "Verify this is a valid Schema.org type or use a \
                                             recognized type."
                                .to_string(),
                        });
                    }

                    // 2.6 — Check required properties
                    let required = Self::required_properties(type_val);
                    if !required.is_empty() {
                        let mut missing = Vec::new();
                        for prop in required {
                            if sd.data.get(*prop).is_none() {
                                missing.push(*prop);
                            }
                        }
                        if !missing.is_empty() {
                            findings.push(Finding {
                                severity: Severity::Error,
                                category: IssueCategory::Schema,
                                code: "SD006".to_string(),
                                title: "Missing required properties".to_string(),
                                description: format!(
                                    "Schema type \"{type_val}\" is missing required properties: \
                                     {}.",
                                    missing.join(", ")
                                ),
                                url: url.clone(),
                                recommendation: format!(
                                    "Add the missing properties to the \"{type_val}\" schema."
                                ),
                            });
                        }
                    }
                }
            }
        }

        findings
    }
}

pub struct ContentQualityAnalyzer;

impl ContentQualityAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ContentQualityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for ContentQualityAnalyzer {
    fn name(&self) -> &str {
        "content-quality"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        // 2.7 — Flesch-Kincaid readability
        let word_count = ctx.page.word_count;
        if word_count > 0 {
            let text = &ctx
                .page
                .headings
                .iter()
                .map(|h| h.text.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            let sentences = count_sentences(text);
            let syllables: usize = text.split_whitespace().map(count_syllables).sum();
            let score = flesch_reading_ease(word_count, sentences.max(1), syllables);

            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Content,
                code: "CQ001".to_string(),
                title: "Flesch-Kincaid readability score".to_string(),
                description: format!(
                    "Readability score: {score:.1}/100 (words: {word_count}, sentences: \
                     {sentences}, syllables: {syllables})."
                ),
                url: url.clone(),
                recommendation: if score < 30.0 {
                    "Content is very difficult to read. Consider simplifying language and \
                     shortening sentences."
                        .to_string()
                } else if score < 50.0 {
                    "Content is fairly difficult to read. Aim for a score of 60+ for general \
                     audiences."
                        .to_string()
                } else if score < 70.0 {
                    "Content has moderate readability. Suitable for most audiences.".to_string()
                } else {
                    "Content is easy to read. Good for general audiences.".to_string()
                },
            });
        }

        // Keyword density (top 10 terms from headings as proxy)
        let headings_text: String = ctx
            .page
            .headings
            .iter()
            .map(|h| h.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        if !headings_text.trim().is_empty() {
            let mut freq: HashMap<String, usize> = HashMap::new();
            for word in headings_text.split_whitespace() {
                let lower = word
                    .to_lowercase()
                    .chars()
                    .filter(|c| c.is_alphanumeric())
                    .collect::<String>();
                if lower.len() > 2 && !STOP_WORDS.contains(&lower.as_str()) {
                    *freq.entry(lower).or_default() += 1;
                }
            }

            let mut terms: Vec<(String, usize)> = freq.into_iter().collect();
            terms.sort_by_key(|b| std::cmp::Reverse(b.1));
            terms.truncate(10);

            if !terms.is_empty() {
                let display: String = terms
                    .iter()
                    .map(|(word, count)| format!("\"{}\" ({})", word, count))
                    .collect::<Vec<_>>()
                    .join(", ");
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Content,
                    code: "CQ002".to_string(),
                    title: "Top keywords".to_string(),
                    description: format!("Top 10 keyword occurrences in headings: {display}."),
                    url: url.clone(),
                    recommendation: "Ensure target keywords appear in headings and body content \
                                     naturally."
                        .to_string(),
                });
            }
        }

        // Content-to-markup ratio (word count as proxy for content volume)
        if word_count == 0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Content,
                code: "CQ003".to_string(),
                title: "No content detected".to_string(),
                description: "The page has zero word count, which may indicate missing or \
                             hidden content."
                    .to_string(),
                url: url.clone(),
                recommendation: "Ensure the page has meaningful visible text content.".to_string(),
            });
        } else if word_count < 300 {
            // Skip thin content warning for utility pages — search engines don't penalize these
            if !is_utility_page(url) {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Content,
                    code: "CQ004".to_string(),
                    title: "Thin content".to_string(),
                    description: format!(
                        "Page has only {word_count} words. Pages with fewer than 300 words may be \
                         considered thin content."
                    ),
                    url: url.clone(),
                    recommendation: "Expand the content to at least 300 words for better search \
                                     visibility."
                        .to_string(),
                });
            }
        } else if word_count > 3000 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Content,
                code: "CQ005".to_string(),
                title: "Long-form content".to_string(),
                description: format!(
                    "Page has {word_count} words. Consider whether all content is necessary or \
                     if it could be split into multiple pages."
                ),
                url: url.clone(),
                recommendation: "Long-form content is good for SEO but ensure it remains \
                                 scannable with proper headings."
                    .to_string(),
            });
        }

        findings
    }
}

// ---------------------------------------------------------------------------

const PERSON_INDICATORS: &[&str] = &[
    "mr.",
    "mrs.",
    "ms.",
    "dr.",
    "prof.",
    "sir",
    "lord",
    "president",
    "ceo",
    "cto",
    "founder",
    "author",
    "by",
    "written by",
    "edited by",
    "interview with",
];

const ORG_INDICATORS: &[&str] = &[
    "inc.",
    "llc",
    "ltd.",
    "corp.",
    "corporation",
    "company",
    "organization",
    "university",
    "institute",
    "foundation",
    "association",
    "group",
    "partners",
];

const LOCATION_INDICATORS: &[&str] = &[
    "city",
    "state",
    "country",
    "province",
    "district",
    "region",
    "street",
    "avenue",
    "boulevard",
    "road",
    "lane",
    "square",
    "park",
    "mountain",
    "river",
    "lake",
    "island",
    "bay",
    "coast",
    "valley",
];

pub struct EntityAnalyzer;

impl EntityAnalyzer {
    pub fn new() -> Self {
        Self
    }

    pub(crate) fn detect_people(text: &str) -> Vec<String> {
        let mut found = Vec::new();
        let lower = text.to_lowercase();
        for indicator in PERSON_INDICATORS {
            if lower.contains(indicator) {
                let words: Vec<&str> = text.split_whitespace().collect();
                let indicator_word = indicator.trim_end_matches('.');
                for (i, word) in words.iter().enumerate() {
                    let clean: String = word.chars().filter(|c| c.is_alphanumeric()).collect();
                    if clean.to_lowercase() == indicator_word && i + 1 < words.len() {
                        let mut name_parts = Vec::new();
                        for w in words.iter().skip(i + 1) {
                            let w_clean: String = w
                                .chars()
                                .filter(|c| c.is_alphanumeric() || *c == '-')
                                .collect();
                            let w_lower = w_clean.to_lowercase();
                            if w_clean
                                .chars()
                                .next()
                                .map(|c| c.is_uppercase())
                                .unwrap_or(false)
                                || w_lower == "de"
                                || w_lower == "van"
                                || w_lower == "von"
                                || w_lower == "la"
                                || w_lower == "le"
                            {
                                name_parts.push(w_clean);
                            } else {
                                break;
                            }
                        }
                        let name = name_parts.join(" ");
                        if !name.is_empty() {
                            found.push(name);
                        }
                    }
                }
            }
        }
        found.sort();
        found.dedup();
        found
    }

    pub(crate) fn detect_organizations(text: &str) -> Vec<String> {
        let mut found = Vec::new();
        let lower = text.to_lowercase();
        for indicator in ORG_INDICATORS {
            if lower.contains(indicator) {
                for sentence in text.split(['.', '!', '?']) {
                    let words: Vec<&str> = sentence.split_whitespace().collect();
                    for (i, word) in words.iter().enumerate() {
                        if word.to_lowercase().contains(indicator) {
                            let start = i.saturating_sub(2);
                            let org: String = words[start..=i.min(words.len() - 1)]
                                .iter()
                                .map(|w| w.to_string())
                                .collect::<Vec<_>>()
                                .join(" ");
                            if org.len() > 3 {
                                found.push(org);
                            }
                        }
                    }
                }
            }
        }
        found.sort();
        found.dedup();
        found
    }

    pub(crate) fn detect_locations(text: &str) -> Vec<String> {
        let mut found = Vec::new();
        let lower = text.to_lowercase();
        for indicator in LOCATION_INDICATORS {
            if lower.contains(indicator) {
                for sentence in text.split(['.', '!', '?']) {
                    let words: Vec<&str> = sentence.split_whitespace().collect();
                    for (i, word) in words.iter().enumerate() {
                        if word.to_lowercase().contains(indicator) {
                            let start = i.saturating_sub(2);
                            let loc: String = words[start..=i.min(words.len() - 1)]
                                .iter()
                                .map(|w| w.to_string())
                                .collect::<Vec<_>>()
                                .join(" ");
                            if loc.len() > 3 {
                                found.push(loc);
                            }
                        }
                    }
                }
            }
        }
        found.sort();
        found.dedup();
        found
    }

    pub(crate) fn detect_topics(
        headings: &[crate::parser::Heading],
        word_count: usize,
    ) -> Vec<String> {
        if word_count == 0 {
            return Vec::new();
        }
        let mut freq: HashMap<String, usize> = HashMap::new();
        for heading in headings {
            for word in heading.text.split_whitespace() {
                let lower = word
                    .to_lowercase()
                    .chars()
                    .filter(|c| c.is_alphanumeric())
                    .collect::<String>();
                if lower.len() > 3 && !STOP_WORDS.contains(&lower.as_str()) {
                    *freq.entry(lower).or_default() += 1;
                }
            }
        }
        let mut terms: Vec<(String, usize)> = freq.into_iter().collect();
        terms.sort_by_key(|b| std::cmp::Reverse(b.1));
        terms.into_iter().take(5).map(|(w, _)| w).collect()
    }

    pub(crate) fn analyze_sentiment(text: &str) -> (f64, &'static str) {
        let positive_words = [
            "good",
            "great",
            "excellent",
            "amazing",
            "wonderful",
            "best",
            "love",
            "happy",
            "fantastic",
            "superb",
            "outstanding",
            "perfect",
            "beautiful",
            "brilliant",
            "awesome",
            "nice",
            "pleasant",
            "delightful",
            "impressive",
            "remarkable",
            "magnificent",
            "splendid",
            "fabulous",
            "terrific",
        ];
        let negative_words = [
            "bad",
            "terrible",
            "horrible",
            "awful",
            "worst",
            "hate",
            "sad",
            "ugly",
            "poor",
            "disappointing",
            "boring",
            "annoying",
            "frustrating",
            "difficult",
            "broken",
            "failed",
            "error",
            "wrong",
            "problem",
            "issue",
            "bug",
            "fail",
            "crash",
            "dead",
        ];
        let words: Vec<String> = text
            .split_whitespace()
            .map(|w| {
                w.to_lowercase()
                    .chars()
                    .filter(|c| c.is_alphanumeric())
                    .collect()
            })
            .collect();
        if words.is_empty() {
            return (0.0, "neutral");
        }
        let pos_count = words
            .iter()
            .filter(|w| positive_words.contains(&w.as_str()))
            .count();
        let neg_count = words
            .iter()
            .filter(|w| negative_words.contains(&w.as_str()))
            .count();
        let total = words.len() as f64;
        let score = ((pos_count as f64 - neg_count as f64) / total * 100.0).round() / 100.0;
        let label = if score > 0.05 {
            "positive"
        } else if score < -0.05 {
            "negative"
        } else {
            "neutral"
        };
        (score, label)
    }
}

impl Default for EntityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for EntityAnalyzer {
    fn name(&self) -> &str {
        "entity-analyzer"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let headings_text: String = ctx
            .page
            .headings
            .iter()
            .map(|h| h.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        let text = &headings_text;

        let people = Self::detect_people(text);
        let organizations = Self::detect_organizations(text);
        let locations = Self::detect_locations(text);
        let topics = Self::detect_topics(&ctx.page.headings, ctx.page.word_count);
        let (sentiment_score, sentiment_label) = Self::analyze_sentiment(text);

        if !people.is_empty() {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Content,
                code: "ENTITY001".to_string(),
                title: "People entities detected".to_string(),
                description: format!(
                    "Found {} people entity(ies): {}.",
                    people.len(),
                    people.join(", ")
                ),
                url: url.clone(),
                recommendation: "People entities can boost E-E-A-T signals. Link to author \
                                 profiles when applicable."
                    .to_string(),
            });
        }

        if !organizations.is_empty() {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Content,
                code: "ENTITY002".to_string(),
                title: "Organization entities detected".to_string(),
                description: format!(
                    "Found {} organization entity(ies): {}.",
                    organizations.len(),
                    organizations.join(", ")
                ),
                url: url.clone(),
                recommendation: "Organization entities help establish topical authority. \
                                 Consider adding Organization schema markup."
                    .to_string(),
            });
        }

        if !locations.is_empty() {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Content,
                code: "ENTITY003".to_string(),
                title: "Location entities detected".to_string(),
                description: format!(
                    "Found {} location entity(ies): {}.",
                    locations.len(),
                    locations.join(", ")
                ),
                url: url.clone(),
                recommendation: "Location entities are important for local SEO. Ensure \
                                 NAP consistency across the site."
                    .to_string(),
            });
        }

        if !topics.is_empty() {
            let topic_display = topics.join(", ");
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Content,
                code: "ENTITY004".to_string(),
                title: "Detected topics and themes".to_string(),
                description: format!("Primary topics: {topic_display}."),
                url: url.clone(),
                recommendation: "Ensure these topics align with the target keywords and \
                                 page intent."
                    .to_string(),
            });
        }

        findings.push(Finding {
            severity: Severity::Info,
            category: IssueCategory::Content,
            code: "ENTITY005".to_string(),
            title: "Content sentiment analysis".to_string(),
            description: format!("Sentiment score: {sentiment_score} ({sentiment_label})."),
            url: url.clone(),
            recommendation: if sentiment_label == "negative" {
                "Content has a negative sentiment tone. Consider revising for a more \
                 neutral or positive tone."
                    .to_string()
            } else if sentiment_label == "positive" {
                "Positive sentiment detected. This can improve user engagement.".to_string()
            } else {
                "Neutral sentiment detected.".to_string()
            },
        });

        findings.push(Finding {
            severity: Severity::Info,
            category: IssueCategory::Content,
            code: "ENTITY006".to_string(),
            title: "Entity counts per page".to_string(),
            description: format!(
                "People: {}, Organizations: {}, Locations: {}, Topics: {}.",
                people.len(),
                organizations.len(),
                locations.len(),
                topics.len()
            ),
            url: url.clone(),
            recommendation: String::new(),
        });

        findings
    }
}

// ---------------------------------------------------------------------------
// 20. Enhanced Readability Analyzer
// ---------------------------------------------------------------------------

pub struct EnhancedReadabilityAnalyzer;

impl EnhancedReadabilityAnalyzer {
    pub fn new() -> Self {
        Self
    }

    fn count_letters(text: &str) -> usize {
        text.chars().filter(|c| c.is_alphabetic()).count()
    }

    fn coleman_liau_index(letters: usize, words: usize, sentences: usize) -> f64 {
        if words == 0 {
            return 0.0;
        }
        let l = letters as f64 / words as f64 * 100.0;
        let s = sentences as f64 / words as f64 * 100.0;
        0.0588 * l - 0.296 * s - 15.8
    }

    fn automated_readability_index(characters: usize, words: usize, sentences: usize) -> f64 {
        if words == 0 || sentences == 0 {
            return 0.0;
        }
        4.71 * (characters as f64 / words as f64) + 0.5 * (words as f64 / sentences as f64) - 21.43
    }

    fn gunning_fog_index(words: usize, sentences: usize, complex_words: usize) -> f64 {
        if words == 0 || sentences == 0 {
            return 0.0;
        }
        0.4 * (words as f64 / sentences as f64 + 100.0 * complex_words as f64 / words as f64)
    }

    pub(crate) fn reading_ease_label(score: f64) -> &'static str {
        if score >= 90.0 {
            "very easy"
        } else if score >= 80.0 {
            "easy"
        } else if score >= 70.0 {
            "fairly easy"
        } else if score >= 60.0 {
            "standard"
        } else if score >= 50.0 {
            "fairly difficult"
        } else if score >= 30.0 {
            "difficult"
        } else {
            "very difficult"
        }
    }

    pub(crate) fn grade_label(grade: f64) -> &'static str {
        if grade < 1.0 {
            "kindergarten"
        } else if grade < 6.0 {
            "elementary school"
        } else if grade < 9.0 {
            "middle school"
        } else if grade < 13.0 {
            "high school"
        } else if grade < 16.0 {
            "college"
        } else {
            "postgraduate"
        }
    }
}

impl Default for EnhancedReadabilityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for EnhancedReadabilityAnalyzer {
    fn name(&self) -> &str {
        "enhanced-readability"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        if ctx.page.word_count == 0 {
            return findings;
        }

        let text = ctx
            .page
            .headings
            .iter()
            .map(|h| h.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        if text.trim().is_empty() {
            return findings;
        }

        let words: Vec<&str> = text.split_whitespace().collect();
        let word_count = words.len();
        let sentence_count = count_sentences(&text);
        let letter_count = Self::count_letters(&text);
        let syllable_count: usize = words.iter().map(|w| count_syllables(w)).sum();
        let complex_words = Self::complex_words_count(&words);

        let fk_grade = flesch_kincaid_grade(word_count, sentence_count, syllable_count);
        let cl_index = Self::coleman_liau_index(letter_count, word_count, sentence_count);
        let ari = Self::automated_readability_index(letter_count, word_count, sentence_count);
        let fog = Self::gunning_fog_index(word_count, sentence_count, complex_words);
        let fre = flesch_reading_ease(word_count, sentence_count, syllable_count);

        findings.push(Finding {
            severity: Severity::Info,
            category: IssueCategory::Content,
            code: "READ001".to_string(),
            title: "Flesch-Kincaid Grade Level".to_string(),
            description: format!(
                "Grade level: {fk_grade:.1} ({})",
                Self::grade_label(fk_grade)
            ),
            url: url.clone(),
            recommendation: if fk_grade > 12.0 {
                "Content is at a college reading level. Consider simplifying for broader \
                 audiences."
                    .to_string()
            } else if fk_grade > 8.0 {
                "Content is at a high school reading level. Suitable for most web audiences."
                    .to_string()
            } else {
                "Content is easy to read for most audiences.".to_string()
            },
        });

        findings.push(Finding {
            severity: Severity::Info,
            category: IssueCategory::Content,
            code: "READ002".to_string(),
            title: "Coleman-Liau Index".to_string(),
            description: format!("Index: {cl_index:.1}"),
            url: url.clone(),
            recommendation: if cl_index > 12.0 {
                "High Coleman-Liau index. Consider reducing sentence complexity.".to_string()
            } else {
                "Readability is within acceptable range.".to_string()
            },
        });

        findings.push(Finding {
            severity: Severity::Info,
            category: IssueCategory::Content,
            code: "READ003".to_string(),
            title: "Automated Readability Index".to_string(),
            description: format!("ARI: {ari:.1}"),
            url: url.clone(),
            recommendation: if ari > 12.0 {
                "High ARI score. Content may be difficult for general audiences.".to_string()
            } else {
                "Readability is within acceptable range.".to_string()
            },
        });

        findings.push(Finding {
            severity: Severity::Info,
            category: IssueCategory::Content,
            code: "READ004".to_string(),
            title: "Gunning Fog Index".to_string(),
            description: format!("Fog index: {fog:.1}"),
            url: url.clone(),
            recommendation: if fog > 17.0 {
                "Very high Fog index. Content is extremely complex. Simplify vocabulary \
                 and shorten sentences."
                    .to_string()
            } else if fog > 12.0 {
                "High Fog index. Consider simplifying for a broader audience.".to_string()
            } else {
                "Fog index is within acceptable range.".to_string()
            },
        });

        findings.push(Finding {
            severity: Severity::Info,
            category: IssueCategory::Content,
            code: "READ005".to_string(),
            title: "Flesch Reading Ease score".to_string(),
            description: format!("Score: {fre:.1}/100 ({})", Self::reading_ease_label(fre)),
            url: url.clone(),
            recommendation: if fre < 30.0 {
                "Very difficult to read. Aim for a score of 60+ for general audiences.".to_string()
            } else if fre < 50.0 {
                "Fairly difficult. Consider simplifying language.".to_string()
            } else if fre < 70.0 {
                "Standard readability. Suitable for most web content.".to_string()
            } else {
                "Easy to read. Good for broad audiences.".to_string()
            },
        });

        findings
    }
}

impl EnhancedReadabilityAnalyzer {
    fn complex_words_count(words: &[&str]) -> usize {
        words.iter().filter(|w| count_syllables(w) >= 3).count()
    }
}

// ---------------------------------------------------------------------------
// RDFa Validator
// ---------------------------------------------------------------------------

/// Deprecated RDFa vocabulary prefixes.
const DEPRECATED_RDFA_VOCABS: &[&str] = &[
    "http://data-vocabulary.org/",
    "http://vocab.composites.com/",
];

pub struct RdfaValidator;

impl RdfaValidator {
    pub fn new() -> Self {
        #[allow(clippy::unwrap_used)]
        Self
    }

    fn has_rdfa_attributes(body: &str) -> bool {
        let lower = body.to_lowercase();
        if lower.contains("vocab=")
            || lower.contains("typeof=")
            || lower.contains("about=")
            || lower.contains("resource=")
        {
            return true;
        }
        // `property=` alone is not RDFa evidence: Open Graph and Twitter Card
        // meta tags use namespaced properties (og:*, twitter:*). Only
        // non-namespaced property values suggest RDFa usage.
        Self::has_non_namespaced_property(body)
    }

    /// True when some `property="..."` value is not OG/Twitter-namespaced.
    fn has_non_namespaced_property(body: &str) -> bool {
        let re = Regex::new(r#"(?i)property\s*=\s*["']([^"']*)["']"#).unwrap();
        let has = re.captures_iter(body).any(|c| {
            let v = c.get(1).map(|m| m.as_str()).unwrap_or("");
            let namespaced = v.starts_with("og:")
                || v.starts_with("twitter:")
                || v.starts_with("fb:")
                || v.starts_with("article:")
                || v.starts_with("music:")
                || v.starts_with("video:")
                || v.starts_with("profile:")
                || v.is_empty();
            !namespaced
        });
        has
    }

    fn extract_vocabs(body: &str) -> Vec<String> {
        let mut vocabs = Vec::new();
        if let Ok(re) = Regex::new(r#"(?i)vocab\s*=\s*["']([^"']+)["']"#) {
            if let Some(caps) = re.captures_iter(body).next() {
                vocabs.push(caps[1].to_string());
            }
        }
        vocabs
    }

    fn extract_typeofs(body: &str) -> Vec<String> {
        let mut types = Vec::new();
        if let Ok(re) = Regex::new(r#"(?i)typeof\s*=\s*["']([^"']+)["']"#) {
            for caps in re.captures_iter(body) {
                types.push(caps[1].to_string());
            }
        }
        types
    }
}

impl Default for RdfaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for RdfaValidator {
    fn name(&self) -> &str {
        "rdfa-validator"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let body = match ctx.body {
            Some(b) => b,
            None => return findings,
        };

        if !Self::has_rdfa_attributes(body) {
            return findings;
        }

        let vocabs = Self::extract_vocabs(body);
        let typeofs = Self::extract_typeofs(body);

        if vocabs.is_empty() {
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Schema,
                code: "RDFA001".to_string(),
                title: "RDFa attributes present but missing vocab".to_string(),
                description: "RDFa attributes (typeof, property, about) were found but no vocab \
                              attribute is declared. Without vocab, the meaning of RDFa types \
                              and properties is ambiguous."
                    .to_string(),
                url: url.clone(),
                recommendation: "Add a vocab attribute to define the RDFa vocabulary, e.g., \
                                 vocab=\"https://schema.org/\"."
                    .to_string(),
            });
        }

        if typeofs.is_empty() {
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Schema,
                code: "RDFA002".to_string(),
                title: "RDFa attributes present but missing typeof".to_string(),
                description: "RDFa properties were found but no typeof attribute declares the \
                              entity type. Search engines cannot determine what the RDFa data \
                              describes without typeof."
                    .to_string(),
                url: url.clone(),
                recommendation: "Add a typeof attribute to declare the type of the RDFa entity, \
                                 e.g., typeof=\"Person\"."
                    .to_string(),
            });
        }

        for vocab in &vocabs {
            let lower = vocab.to_lowercase();
            for deprecated in DEPRECATED_RDFA_VOCABS {
                if lower.starts_with(deprecated) {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Schema,
                        code: "RDFA003".to_string(),
                        title: "RDFa uses deprecated vocabulary".to_string(),
                        description: format!(
                            "RDFa vocab \"{vocab}\" uses a deprecated vocabulary prefix. \
                             Deprecated vocabularies are no longer recognized by search engines."
                        ),
                        url: url.clone(),
                        recommendation: "Replace deprecated vocabularies with modern alternatives \
                                         such as Schema.org."
                            .to_string(),
                    });
                    break;
                }
            }
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// Microdata Validator
// ---------------------------------------------------------------------------

const RECOGNIZED_MICRODATA_TYPES: &[&str] = &[
    "http://schema.org/Article",
    "http://schema.org/NewsArticle",
    "http://schema.org/BlogPosting",
    "http://schema.org/Product",
    "http://schema.org/Offer",
    "http://schema.org/Brand",
    "http://schema.org/AggregateRating",
    "http://schema.org/Review",
    "http://schema.org/Organization",
    "http://schema.org/LocalBusiness",
    "http://schema.org/Store",
    "http://schema.org/Person",
    "http://schema.org/Place",
    "http://schema.org/WebSite",
    "http://schema.org/WebPage",
    "http://schema.org/BreadcrumbList",
    "http://schema.org/ItemList",
    "http://schema.org/ListItem",
    "http://schema.org/FAQPage",
    "http://schema.org/HowTo",
    "http://schema.org/Event",
    "http://schema.org/Recipe",
    "http://schema.org/VideoObject",
    "http://schema.org/ImageObject",
    "http://schema.org/AudioObject",
    "http://schema.org/SoftwareApplication",
    "http://schema.org/Book",
    "http://schema.org/MusicAlbum",
    "http://schema.org/Movie",
    "http://schema.org/Dataset",
    "http://schema.org/Course",
    "http://schema.org/CreativeWork",
];

const REQUIRED_MICRODATA_PROPERTIES: &[(&str, &[&str])] = &[
    ("http://schema.org/Article", &["headline", "author"]),
    ("http://schema.org/Product", &["name"]),
    ("http://schema.org/Organization", &["name"]),
    ("http://schema.org/Event", &["name", "startDate"]),
    ("http://schema.org/Recipe", &["name"]),
];

pub struct MicrodataValidator;

impl MicrodataValidator {
    pub fn new() -> Self {
        #[allow(clippy::unwrap_used)]
        Self
    }

    fn has_microdata(body: &str) -> bool {
        body.to_lowercase().contains("itemscope")
    }

    fn has_itemprop(body: &str) -> bool {
        body.to_lowercase().contains("itemprop")
    }

    fn extract_itemtype(body: &str) -> Vec<String> {
        let mut types = Vec::new();
        if let Ok(re) = Regex::new(r#"(?i)itemtype\s*=\s*["']([^"']+)["']"#) {
            for caps in re.captures_iter(body) {
                types.push(caps[1].to_string());
            }
        }
        types
    }

    fn is_known_type(itemtype: &str) -> bool {
        RECOGNIZED_MICRODATA_TYPES
            .iter()
            .any(|t| *t == itemtype || itemtype.contains("schema.org"))
    }

    fn required_properties(itemtype: &str) -> &'static [&'static str] {
        REQUIRED_MICRODATA_PROPERTIES
            .iter()
            .find(|(t, _)| *t == itemtype)
            .map(|(_, props)| *props)
            .unwrap_or(&[])
    }
}

impl Default for MicrodataValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for MicrodataValidator {
    fn name(&self) -> &str {
        "microdata-validator"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let body = match ctx.body {
            Some(b) => b,
            None => return findings,
        };

        if !Self::has_microdata(body) {
            return findings;
        }

        if !Self::has_itemprop(body) {
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Schema,
                code: "MD001".to_string(),
                title: "Microdata itemscope without itemprop".to_string(),
                description: "An itemscope attribute was found but no itemprop attributes exist. \
                              Without itemprop, microdata provides no meaningful structured data."
                    .to_string(),
                url: url.clone(),
                recommendation: "Add itemprop attributes to describe the properties of the \
                                 microdata item."
                    .to_string(),
            });
        }

        let itemtypes = Self::extract_itemtype(body);
        for itemtype in &itemtypes {
            if !Self::is_known_type(itemtype) {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "MD003".to_string(),
                    title: "Microdata @type not in Schema.org vocabulary".to_string(),
                    description: format!(
                        "Microdata itemtype \"{itemtype}\" is not a recognized Schema.org type."
                    ),
                    url: url.clone(),
                    recommendation: "Use a valid Schema.org type, e.g., \
                                     itemtype=\"http://schema.org/Product\"."
                        .to_string(),
                });
            }
        }

        for itemtype in &itemtypes {
            let required = Self::required_properties(itemtype);
            if !required.is_empty() && Self::has_itemprop(body) {
                let mut missing = Vec::new();
                let lower_body = body.to_lowercase();
                for prop in required {
                    if !lower_body.contains(&format!("itemprop=\"{}\"", prop.to_lowercase())) {
                        missing.push(*prop);
                    }
                }
                if !missing.is_empty() {
                    findings.push(Finding {
                        severity: Severity::Error,
                        category: IssueCategory::Schema,
                        code: "MD002".to_string(),
                        title: "Microdata missing required properties".to_string(),
                        description: format!(
                            "Microdata type \"{itemtype}\" is missing required properties: {}.",
                            missing.join(", ")
                        ),
                        url: url.clone(),
                        recommendation: format!(
                            "Add the missing itemprop attributes to the \"{itemtype}\" microdata."
                        ),
                    });
                }
            }
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// Entity Linking Analyzer
// ---------------------------------------------------------------------------

pub struct EntityLinkingAnalyzer;

impl EntityLinkingAnalyzer {
    pub fn new() -> Self {
        Self
    }

    fn get_entity_names(ctx: &AnalysisContext) -> Vec<String> {
        let mut names = Vec::new();
        for sd in &ctx.page.structured_data {
            if let Some(name) = sd.data.get("name").and_then(|v| v.as_str()) {
                if !name.is_empty() {
                    names.push(name.to_string());
                }
            }
        }
        names
    }

    fn has_wikipedia_link(entity_name: &str, links: &[ExtractedLink]) -> bool {
        let lower_name = entity_name.to_lowercase();
        links.iter().any(|l| {
            let href_lower = l.href.to_lowercase();
            href_lower.contains("wikipedia.org")
                && (href_lower.contains(&lower_name.replace(' ', "_"))
                    || l.text.to_lowercase().contains(&lower_name))
        })
    }

    fn has_outbound_link(entity_name: &str, links: &[ExtractedLink]) -> bool {
        let lower_name = entity_name.to_lowercase();
        links
            .iter()
            .any(|l| l.is_external && l.text.to_lowercase().contains(&lower_name))
    }

    fn same_type_groups(ctx: &AnalysisContext) -> HashMap<String, Vec<String>> {
        let mut groups: HashMap<String, Vec<String>> = HashMap::new();
        for sd in &ctx.page.structured_data {
            if let Some(type_name) = &sd.r#type {
                if let Some(name) = sd.data.get("name").and_then(|v| v.as_str()) {
                    if !name.is_empty() {
                        groups
                            .entry(type_name.clone())
                            .or_default()
                            .push(name.to_string());
                    }
                }
            }
        }
        groups
    }

    fn cross_links_exist(entity_name: &str, other_name: &str, links: &[ExtractedLink]) -> bool {
        let lower_entity = entity_name.to_lowercase();
        let lower_other = other_name.to_lowercase();
        links.iter().any(|l| {
            let href_lower = l.href.to_lowercase();
            let text_lower = l.text.to_lowercase();
            let has_entity_to_other = text_lower.contains(&lower_entity)
                && href_lower.contains(&lower_other.replace(' ', "_"));
            let has_other_to_entity = text_lower.contains(&lower_other)
                && href_lower.contains(&lower_entity.replace(' ', "_"));
            has_entity_to_other || has_other_to_entity
        })
    }
}

impl Default for EntityLinkingAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for EntityLinkingAnalyzer {
    fn name(&self) -> &str {
        "entity-linking"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let entity_names = Self::get_entity_names(ctx);
        if entity_names.is_empty() {
            return findings;
        }

        for name in &entity_names {
            if !Self::has_wikipedia_link(name, &ctx.page.links)
                && !Self::has_outbound_link(name, &ctx.page.links)
            {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "ELINK001".to_string(),
                    title: "Entity lacks outbound link".to_string(),
                    description: format!(
                        "Entity \"{name}\" was detected in structured data but has no outbound \
                         link to a Wikipedia or external page."
                    ),
                    url: url.clone(),
                    recommendation: format!(
                        "Add a link from \"{name}\" to its Wikipedia or authoritative external \
                         page to strengthen entity signals."
                    ),
                });
            }
        }

        let groups = Self::same_type_groups(ctx);
        for (schema_type, names) in &groups {
            if names.len() < 2 {
                continue;
            }
            for i in 0..names.len() {
                let mut all_cross_linked = true;
                for j in 0..names.len() {
                    if i == j {
                        continue;
                    }
                    if !Self::cross_links_exist(&names[i], &names[j], &ctx.page.links) {
                        all_cross_linked = false;
                        break;
                    }
                }
                if !all_cross_linked {
                    findings.push(Finding {
                        severity: Severity::Info,
                        category: IssueCategory::Schema,
                        code: "ELINK002".to_string(),
                        title: "Multiple same-type entities lack cross-links".to_string(),
                        description: format!(
                            "Found {} entities of type \"{}\" but they are not cross-linked. \
                             Entity \"{}\" lacks links to other entities of the same type.",
                            names.len(),
                            schema_type,
                            names[i]
                        ),
                        url: url.clone(),
                        recommendation: "Cross-link related entities of the same type using \
                                         schema references or outbound links to strengthen \
                                         topical authority."
                            .to_string(),
                    });
                    break;
                }
            }
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// Shipping Schema Validator
// ---------------------------------------------------------------------------

// =========================================================================
// BreadcrumbsValidator
// =========================================================================

// Validates BreadcrumbList structured data completeness and consistency.
///
/// Checks for incomplete BreadcrumbList schema, breadcrumb URLs that
/// don't match the page hierarchy, and missing breadcrumbs on deep pages.
// =========================================================================
// DuplicateContentDetector
// =========================================================================
/// Detects potential duplicate content patterns within a single page.
///
/// Checks for title/description similarity, boilerplate patterns,
/// and low-entropy content that may indicate duplication.
pub struct DuplicateContentDetector;

impl Default for DuplicateContentDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl DuplicateContentDetector {
    pub fn new() -> Self {
        Self
    }

    /// Compute simple cosine similarity between two word vectors.
    fn cosine_similarity(a: &[String], b: &[String]) -> f64 {
        if a.is_empty() || b.is_empty() {
            return 0.0;
        }
        let mut intersection = 0;
        for word in a {
            if b.contains(word) {
                intersection += 1;
            }
        }
        let len_a = a.len() as f64;
        let len_b = b.len() as f64;
        if len_a == 0.0 || len_b == 0.0 {
            return 0.0;
        }
        intersection as f64 / (len_a * len_b).sqrt()
    }

    fn tokenize(text: &str) -> Vec<String> {
        text.to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.len() > 2)
            .map(|w| w.to_string())
            .collect()
    }
}

impl Analyzer for DuplicateContentDetector {
    fn name(&self) -> &str {
        "duplicate-content"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        // DUP001: Title and description have >90% word overlap
        if let (Some(title), Some(desc)) = (&ctx.page.meta.title, &ctx.page.meta.description) {
            let title_words = Self::tokenize(title);
            let desc_words = Self::tokenize(desc);
            let similarity = Self::cosine_similarity(&title_words, &desc_words);
            if similarity > 0.9 && !title_words.is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Content,
                    code: "DUP001".to_string(),
                    title: "Title and description are nearly identical".to_string(),
                    description: format!(
                        "The title and meta description share {:.0}% word overlap. This may \
                         indicate duplicate or auto-generated content.",
                        similarity * 100.0
                    ),
                    url: url.clone(),
                    recommendation: "Write unique, complementary title and description. The \
                                     description should expand on the title, not repeat it."
                        .to_string(),
                });
            }
        }

        // DUP002: Description starts with title text (boilerplate pattern)
        if let (Some(title), Some(desc)) = (&ctx.page.meta.title, &ctx.page.meta.description) {
            let title_lower = title.to_lowercase();
            let desc_lower = desc.to_lowercase();
            if desc_lower.starts_with(&title_lower) && title.len() > 10 {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Content,
                    code: "DUP002".to_string(),
                    title: "Description starts with title text".to_string(),
                    description: "The meta description begins with the same text as the page \
                                 title, suggesting auto-generated or duplicated content."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Write a unique meta description that provides additional \
                                     context beyond the title."
                        .to_string(),
                });
            }
        }

        // DUP003: Low-entropy content (unique token ratio < 30%)
        if let Some(body) = ctx.body {
            let words: Vec<&str> = body.split_whitespace().collect();
            if words.len() > 100 {
                let unique: std::collections::HashSet<&str> = words.iter().copied().collect();
                let ratio = unique.len() as f64 / words.len() as f64;
                if ratio < 0.3 {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Content,
                        code: "DUP003".to_string(),
                        title: "Low content diversity detected".to_string(),
                        description: format!(
                            "Only {:.0}% of words are unique, suggesting repetitive or \
                             boilerplate content.",
                            ratio * 100.0
                        ),
                        url: url.clone(),
                        recommendation: "Add more unique, substantive content to the page."
                            .to_string(),
                    });
                }
            }
        }

        findings
    }
}

// =========================================================================
// EventSchemaValidator
// =========================================================================

// Validates Event structured data for completeness.
// =========================================================================
// ReviewSchemaValidator
// =========================================================================

// Validates Review and AggregateRating structured data.
// =========================================================================
// VideoSchemaValidator
// =========================================================================

// Validates VideoObject structured data.
// =========================================================================
// TableOfContentsAnalyzer
// =========================================================================

/// Detects long-form pages missing a table of contents.
///
/// Pages with >2000 words and >5 headings should have a ToC for
/// better navigation and AI extraction.
pub struct TableOfContentsAnalyzer;

impl Default for TableOfContentsAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl TableOfContentsAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for TableOfContentsAnalyzer {
    fn name(&self) -> &str {
        "table-of-contents"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        if ctx.page.word_count > 2000 && ctx.page.headings.len() > 5 {
            // Check for anchor links (links starting with #)
            let has_toc = ctx.page.links.iter().any(|l| l.href.starts_with('#'));
            // Also check for nav/ol elements with anchor refs in HTML
            let has_toc_html = ctx
                .body
                .is_some_and(|body| body.contains("<nav") && body.contains("href=\"#"))
                || ctx
                    .body
                    .is_some_and(|body| body.contains("<ol") && body.contains("href=\"#"));

            if !has_toc && !has_toc_html {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Content,
                    code: "TOC001".to_string(),
                    title: "Long-form page missing table of contents".to_string(),
                    description: format!(
                        "This page has {} words and {} headings but no table of contents \
                         (anchor links or nav/ol with internal anchors).",
                        ctx.page.word_count,
                        ctx.page.headings.len()
                    ),
                    url: url.clone(),
                    recommendation: "Add a table of contents with anchor links to each major \
                                     section. This improves navigation and helps AI crawlers \
                                     extract structured content."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// LocalBusinessSchemaValidator
// =========================================================================

// Validates LocalBusiness and subtype schemas for NAP consistency.
// =========================================================================
// LocalBusinessNapAnalyzer
// =========================================================================

// Validates LocalBusiness NAP (Name, Address, Phone) consistency.
// =========================================================================
// FaqSchemaValidator
// =========================================================================

// Validates FAQPage structured data for completeness.
// =========================================================================
// HowToSchemaValidator
// =========================================================================

// Validates HowTo structured data for completeness.
// =========================================================================
// SpeakableSchemaValidator
// =========================================================================

// Validates Speakable structured data for completeness.
// =========================================================================
// DatasetSchemaValidator
// =========================================================================

// Validates Dataset structured data for completeness.
// =========================================================================
// SpecialAnnouncementSchemaValidator
// =========================================================================

// Validates SpecialAnnouncement structured data for completeness.
// =========================================================================
// SoftwareApplicationValidator
// =========================================================================

// Validates SoftwareApplication structured data for completeness.
// =========================================================================
// ArticleSchemaValidator
// =========================================================================

// Validates Article (and subtype) structured data for completeness.
// =========================================================================
// OrganizationSchemaValidator
// =========================================================================

// Validates Organization structured data for completeness.
// =========================================================================
// PersonSchemaValidator
// =========================================================================

// Validates Person structured data for completeness.
// =========================================================================
// JobPostingSchemaValidator
// =========================================================================

// Validates JobPosting structured data for completeness.
// =========================================================================
// CourseSchemaValidator
// =========================================================================

// Validates Course structured data for completeness.
// =========================================================================
// RecipeSchemaValidator
// =========================================================================

// Validates Recipe structured data for completeness.
// =========================================================================
// ContentFreshnessScorer
// =========================================================================

/// Scores content freshness by checking date metadata consistency.
pub struct ContentFreshnessScorer;

impl Default for ContentFreshnessScorer {
    fn default() -> Self {
        Self::new()
    }
}

impl ContentFreshnessScorer {
    pub fn new() -> Self {
        Self
    }

    /// Check if a URL path looks like time-sensitive content.
    fn is_time_sensitive_url(url: &str) -> bool {
        let lower = url.to_lowercase();
        [
            "/blog/",
            "/news/",
            "/article/",
            "/post/",
            "/press/",
            "/release/",
            "/update/",
            "/announcement/",
        ]
        .iter()
        .any(|p| lower.contains(p))
    }

    /// Check if any date-like metadata exists on the page.
    fn has_date_metadata(ctx: &AnalysisContext) -> bool {
        // Check structured data for datePublished or dateModified
        for sd in &ctx.page.structured_data {
            if sd.data.get("datePublished").is_some()
                || sd.data.get("dateModified").is_some()
                || sd.data.get("dateCreated").is_some()
            {
                return true;
            }
        }
        false
    }

    /// Extract the datePublished from the first Article-like schema.
    fn extract_schema_date(ctx: &AnalysisContext) -> Option<String> {
        for sd in &ctx.page.structured_data {
            let schema_type = sd.data.get("@type").and_then(|t| t.as_str()).unwrap_or("");
            let article_types = [
                "Article",
                "NewsArticle",
                "BlogPosting",
                "ScholarlyArticle",
                "TechArticle",
            ];
            if article_types.contains(&schema_type) {
                if let Some(date) = sd.data.get("datePublished").and_then(|d| d.as_str()) {
                    return Some(date.to_string());
                }
            }
        }
        None
    }
}

impl Analyzer for ContentFreshnessScorer {
    fn name(&self) -> &str {
        "content-freshness"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        // FRESH001: No date metadata on time-sensitive content
        if Self::is_time_sensitive_url(url) && !Self::has_date_metadata(ctx) {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Content,
                code: "FRESH001".to_string(),
                title: "No date metadata on time-sensitive content".to_string(),
                description:
                    "This page appears to be time-sensitive content (blog, news, article) \
                              but has no date metadata (datePublished, dateModified) in structured \
                              data."
                        .to_string(),
                url: url.clone(),
                recommendation: "Add datePublished and dateModified to the Article schema to \
                                 signal content freshness to search engines."
                    .to_string(),
            });
        }

        // FRESH002: Date in visible text differs from schema datePublished
        if let Some(schema_date) = Self::extract_schema_date(ctx) {
            if let Some(body) = ctx.body {
                // Simple heuristic: look for the year in the schema date
                if let Some(year) = schema_date.get(0..4) {
                    // If the body mentions a different year prominently near date-related words
                    let lower = body.to_lowercase();
                    let date_indicators = [
                        "published",
                        "posted",
                        "written",
                        "updated",
                        "on january",
                        "on february",
                        "on march",
                        "on april",
                        "on may",
                        "on june",
                        "on july",
                        "on august",
                        "on september",
                        "on october",
                        "on november",
                        "on december",
                    ];
                    for indicator in &date_indicators {
                        if let Some(pos) = lower.find(indicator) {
                            // Look for a 4-digit year within 100 chars after the indicator
                            let window = &body[pos..(pos + 100).min(body.len())];
                            for candidate_year_str in window.split(|c: char| !c.is_ascii_digit()) {
                                if candidate_year_str.len() == 4 {
                                    if let Ok(candidate_year) = candidate_year_str.parse::<i32>() {
                                        if let Ok(schema_year) = year.parse::<i32>() {
                                            if candidate_year != schema_year
                                                && candidate_year > 2000
                                                && candidate_year < 2100
                                            {
                                                findings.push(Finding {
                                                    severity: Severity::Warning,
                                                    category: IssueCategory::Content,
                                                    code: "FRESH002".to_string(),
                                                    title: "Date mismatch between visible text \
                                                             and schema"
                                                        .to_string(),
                                                    description: format!(
                                                        "Schema datePublished year ({}) differs \
                                                         from year ({}) found in visible text \
                                                         near date indicators.",
                                                        schema_year, candidate_year
                                                    ),
                                                    url: url.clone(),
                                                    recommendation: "Ensure the datePublished in \
                                                                     structured data matches the \
                                                                     visible publication date on \
                                                                     the page."
                                                        .to_string(),
                                                });
                                                return findings;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        findings
    }
}

// =========================================================================
// BreadcrumbListDepthAnalyzer
// =========================================================================

// Validates BreadcrumbList depth consistency with URL depth.
pub struct BreadcrumbListDepthAnalyzer;

impl Default for BreadcrumbListDepthAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl BreadcrumbListDepthAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for BreadcrumbListDepthAnalyzer {
    fn name(&self) -> &str {
        "breadcrumb-depth"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        // Compute URL depth (number of non-empty path segments)
        let url_depth = if let Ok(parsed) = url::Url::parse(url) {
            parsed
                .path_segments()
                .map(|s| s.filter(|seg| !seg.is_empty()).count())
                .unwrap_or(0)
        } else {
            0
        };

        // Find BreadcrumbList depth
        for sd in &ctx.page.structured_data {
            let schema_type = sd.data.get("@type").and_then(|t| t.as_str()).unwrap_or("");
            if schema_type != "BreadcrumbList" {
                continue;
            }

            let breadcrumb_depth = sd
                .data
                .get("itemListElement")
                .and_then(|i| i.as_array())
                .map_or(0, |arr| arr.len());

            // BDEPTH001: Breadcrumb depth inconsistent with URL depth
            if url_depth > 2
                && breadcrumb_depth > 0
                && (breadcrumb_depth as isize - url_depth as isize).abs() > 1
            {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Content,
                    code: "BDEPTH001".to_string(),
                    title: "Breadcrumb depth inconsistent with URL depth".to_string(),
                    description: format!(
                        "URL has {url_depth} path segments but BreadcrumbList has \
                         {breadcrumb_depth} items. The breadcrumb trail should reflect the \
                         page hierarchy."
                    ),
                    url: url.clone(),
                    recommendation: "Ensure the BreadcrumbList depth matches the URL path depth. \
                                     Each segment should correspond to one breadcrumb item."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// WebPageSchemaValidator
// =========================================================================

// Validates WebPage structured data for completeness.
// =========================================================================
// ServiceSchemaValidator
// =========================================================================

// Validates Service structured data for completeness.
// =========================================================================
// ItemListSchemaValidator
// =========================================================================

// Validates ItemList structured data for completeness.
// =========================================================================
// OfferSchemaValidator
// =========================================================================

// Validates Offer structured data for completeness.
// =========================================================================
// AggregateOfferSchemaValidator
// =========================================================================

// Validates AggregateOffer structured data for completeness.
// =========================================================================
// BrandSchemaValidator
// =========================================================================

// Validates Brand structured data for completeness.
// =========================================================================
// OccupationSchemaValidator
// =========================================================================

// Validates Occupation structured data for completeness.
// =========================================================================
// QuestSchemaValidator
// =========================================================================

// Validates Quest structured data for games and education.
// =========================================================================
// ActionSchemaValidator
// =========================================================================

// Validates Action structured data for completeness.
// =========================================================================
// PlaybookSchemaValidator
// =========================================================================

// Validates Playbook structured data for completeness.
// =========================================================================
// LocalBusinessHoursValidator
// =========================================================================

// =========================================================================
// ProductReviewValidator
// =========================================================================

// =========================================================================
// EventLocationValidator
// =========================================================================

// =========================================================================
// OrganizationLogoValidator
// =========================================================================

// =========================================================================
// PersonJobTitleValidator
// =========================================================================

// =========================================================================
// RecipeNutritionValidator
// =========================================================================

// =========================================================================
// CourseProviderValidator
// =========================================================================

// =========================================================================
// JobPostingSalaryValidator
// =========================================================================

// =========================================================================
// LocalBusinessHoursValidator tests
// =========================================================================

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::meta::MetaTags;
    use crate::parser::StructuredData;

    fn make_page(url: &str) -> crate::parser::ParsedPage {
        crate::parser::ParsedPage {
            url: url.to_string(),
            meta: MetaTags::default(),
            headings: Vec::new(),
            links: Vec::new(),
            images: Vec::new(),
            forms: Vec::new(),
            scripts: Vec::new(),
            styles: Vec::new(),
            structured_data: Vec::new(),
            word_count: 0,
            sentence_count: 0,
            landmarks: Vec::new(),
            has_skip_link: false,
            has_main_landmark: false,
            has_nav_landmark: false,
            has_positive_tabindex: false,
            tabindex_negative_count: 0,
            aria_role_count: 0,
            aria_label_count: 0,
            has_lang_attribute: false,
            html_lang: None,
            has_aria_hidden: false,
            tables_with_headers: 0,
            tables_total: 0,
            tables_with_captions: 0,
            og_image_width: None,
            og_image_height: None,
        }
    }

    fn make_ctx<'a>(
        page: &'a crate::parser::ParsedPage,
        status: Option<u16>,
    ) -> AnalysisContext<'a> {
        AnalysisContext {
            page,
            body: None,
            status_code: status,
            headers: &[],
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        }
    }

    fn make_ctx_with_body<'a>(
        page: &'a crate::parser::ParsedPage,
        status: Option<u16>,
        body: &'a str,
    ) -> AnalysisContext<'a> {
        AnalysisContext {
            page,
            body: Some(body),
            status_code: status,
            headers: &[],
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        }
    }

    #[test]
    fn test_rdfa_no_rdfa_attributes() {
        let page = make_page("https://example.com");
        let ctx = make_ctx_with_body(&page, Some(200), "<html><body>Hello</body></html>");
        assert!(RdfaValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_rdfa_no_body() {
        let page = make_page("https://example.com");
        assert!(RdfaValidator::new()
            .analyze(&make_ctx(&page, Some(200)))
            .is_empty());
    }

    #[test]
    fn test_rdfa_missing_vocab() {
        let page = make_page("https://example.com");
        let body = r#"<div typeof="Person"><span property="name">John</span></div>"#;
        let ctx = make_ctx_with_body(&page, Some(200), body);
        assert!(RdfaValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "RDFA001"));
    }

    #[test]
    fn test_rdfa_missing_typeof() {
        let page = make_page("https://example.com");
        let body = r#"<div vocab="https://schema.org/"><span property="name">John</span></div>"#;
        let ctx = make_ctx_with_body(&page, Some(200), body);
        assert!(RdfaValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "RDFA002"));
    }

    #[test]
    fn test_rdfa_deprecated_vocab() {
        let page = make_page("https://example.com");
        let body = r#"<div vocab="http://data-vocabulary.org/Review" typeof="Review"></div>"#;
        let ctx = make_ctx_with_body(&page, Some(200), body);
        assert!(RdfaValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "RDFA003"));
    }

    #[test]
    fn test_rdfa_valid_vocab_and_typeof() {
        let page = make_page("https://example.com");
        let body = r#"<div vocab="https://schema.org/" typeof="Person"><span property="name">John</span></div>"#;
        let ctx = make_ctx_with_body(&page, Some(200), body);
        let f = RdfaValidator::new().analyze(&ctx);
        assert!(!f.iter().any(|f| f.code == "RDFA001"));
        assert!(!f.iter().any(|f| f.code == "RDFA002"));
        assert!(!f.iter().any(|f| f.code == "RDFA003"));
    }

    #[test]
    fn test_rdfa_missing_both_vocab_and_typeof() {
        let page = make_page("https://example.com");
        let body = r#"<div property="name">John</div>"#;
        let ctx = make_ctx_with_body(&page, Some(200), body);
        let f = RdfaValidator::new().analyze(&ctx);
        assert!(f.iter().any(|f| f.code == "RDFA001"));
        assert!(f.iter().any(|f| f.code == "RDFA002"));
    }

    #[test]
    fn test_rdfa_only_about_attribute() {
        let page = make_page("https://example.com");
        let body =
            r#"<div about="http://example.com/page"><span property="name">Page</span></div>"#;
        let ctx = make_ctx_with_body(&page, Some(200), body);
        assert!(RdfaValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "RDFA002"));
    }

    #[test]
    fn test_rdfa_case_insensitive() {
        let page = make_page("https://example.com");
        let body = r#"<div Vocab="https://schema.org/" TypeOf="Person"></div>"#;
        let ctx = make_ctx_with_body(&page, Some(200), body);
        let f = RdfaValidator::new().analyze(&ctx);
        assert!(!f.iter().any(|f| f.code == "RDFA001"));
        assert!(!f.iter().any(|f| f.code == "RDFA002"));
    }

    #[test]
    fn test_rdfa_non_deprecated_vocab() {
        let page = make_page("https://example.com");
        let body = r#"<div vocab="http://creativecommons.org/ns#" typeof="License"></div>"#;
        let ctx = make_ctx_with_body(&page, Some(200), body);
        assert!(!RdfaValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "RDFA003"));
    }

    #[test]
    fn test_rdfa_all_issues() {
        let page = make_page("https://example.com");
        let body = r#"<div property="name">John</div>"#;
        let ctx = make_ctx_with_body(&page, Some(200), body);
        assert!(RdfaValidator::new().analyze(&ctx).len() >= 2);
    }

    #[test]
    fn test_rdfa_only_property_attribute() {
        let page = make_page("https://example.com");
        let body = r#"<span property="name">John</span>"#;
        let ctx = make_ctx_with_body(&page, Some(200), body);
        let f = RdfaValidator::new().analyze(&ctx);
        assert!(f.iter().any(|f| f.code == "RDFA001"));
        assert!(f.iter().any(|f| f.code == "RDFA002"));
    }

    #[test]
    fn test_microdata_no_microdata() {
        let page = make_page("https://example.com");
        let ctx = make_ctx_with_body(&page, Some(200), "<html><body>Hello</body></html>");
        assert!(MicrodataValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_microdata_no_body() {
        let page = make_page("https://example.com");
        assert!(MicrodataValidator::new()
            .analyze(&make_ctx(&page, Some(200)))
            .is_empty());
    }

    #[test]
    fn test_microdata_itemscope_without_itemprop() {
        let page = make_page("https://example.com");
        let body = r#"<div itemscope itemtype="http://schema.org/Product"></div>"#;
        let ctx = make_ctx_with_body(&page, Some(200), body);
        assert!(MicrodataValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "MD001"));
    }

    #[test]
    fn test_microdata_unknown_type() {
        let page = make_page("https://example.com");
        let body = r#"<div itemscope itemtype="http://example.com/Custom"><span itemprop="name">X</span></div>"#;
        let ctx = make_ctx_with_body(&page, Some(200), body);
        assert!(MicrodataValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "MD003"));
    }

    #[test]
    fn test_microdata_known_type_no_missing() {
        let page = make_page("https://example.com");
        let body = r#"<div itemscope itemtype="http://schema.org/Product"><span itemprop="name">Widget</span></div>"#;
        let ctx = make_ctx_with_body(&page, Some(200), body);
        let f = MicrodataValidator::new().analyze(&ctx);
        assert!(!f.iter().any(|f| f.code == "MD003"));
        assert!(!f.iter().any(|f| f.code == "MD002"));
    }

    #[test]
    fn test_microdata_missing_required_properties() {
        let page = make_page("https://example.com");
        let body = r#"<div itemscope itemtype="http://schema.org/Article"><span itemprop="headline">Title</span></div>"#;
        let ctx = make_ctx_with_body(&page, Some(200), body);
        assert!(MicrodataValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "MD002"));
    }

    #[test]
    fn test_microdata_valid_article() {
        let page = make_page("https://example.com");
        let body = r#"<div itemscope itemtype="http://schema.org/Article"><span itemprop="headline">Title</span><span itemprop="author">Author</span></div>"#;
        let ctx = make_ctx_with_body(&page, Some(200), body);
        assert!(!MicrodataValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "MD002"));
    }

    #[test]
    fn test_microdata_itemprop_only_no_itemscope() {
        let page = make_page("https://example.com");
        let body = r#"<span itemprop="name">John</span>"#;
        let ctx = make_ctx_with_body(&page, Some(200), body);
        assert!(MicrodataValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_microdata_product_missing_name() {
        let page = make_page("https://example.com");
        let body = r#"<div itemscope itemtype="http://schema.org/Product"><span itemprop="description">A widget</span></div>"#;
        let ctx = make_ctx_with_body(&page, Some(200), body);
        assert!(MicrodataValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "MD002"));
    }

    #[test]
    fn test_microdata_case_insensitive() {
        let page = make_page("https://example.com");
        let body = r#"<div ITEMSCOPE itemtype="http://schema.org/Product"><span itemprop="name">X</span></div>"#;
        let ctx = make_ctx_with_body(&page, Some(200), body);
        // Case-insensitive: ITEMSCOPE should be recognized, no findings expected
        let findings = MicrodataValidator::new().analyze(&ctx);
        assert!(findings.is_empty() || findings.iter().all(|f| f.code != "MD001"));
    }

    #[test]
    fn test_microdata_schema_org_url_known() {
        let page = make_page("https://example.com");
        let body = r#"<div itemscope itemtype="http://schema.org/Event"><span itemprop="name">Concert</span></div>"#;
        let ctx = make_ctx_with_body(&page, Some(200), body);
        assert!(!MicrodataValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "MD003"));
    }

    #[test]
    fn test_microdata_multiple_types() {
        let page = make_page("https://example.com");
        let body = r#"<div itemscope itemtype="http://schema.org/Product"><span itemprop="name">Widget</span></div>"#;
        let ctx = make_ctx_with_body(&page, Some(200), body);
        assert!(!MicrodataValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "MD003"));
    }

    #[test]
    fn test_entity_linking_no_structured_data() {
        let page = make_page("https://example.com");
        assert!(EntityLinkingAnalyzer::new()
            .analyze(&make_ctx(&page, Some(200)))
            .is_empty());
    }

    #[test]
    fn test_entity_linking_no_outbound_link() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Person".to_string()),
            data: serde_json::json!({"@type": "Person", "name": "John Doe"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(EntityLinkingAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "ELINK001"));
    }

    #[test]
    fn test_entity_linking_with_wikipedia_link() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Person".to_string()),
            data: serde_json::json!({"@type": "Person", "name": "Albert Einstein"}),
        }];
        page.links = vec![ExtractedLink {
            href: "https://en.wikipedia.org/wiki/Albert_Einstein".to_string(),
            text: "Albert Einstein".to_string(),
            rel: vec![],
            is_external: true,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(!EntityLinkingAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "ELINK001"));
    }

    #[test]
    fn test_entity_linking_with_external_link() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Person".to_string()),
            data: serde_json::json!({"@type": "Person", "name": "John Doe"}),
        }];
        page.links = vec![ExtractedLink {
            href: "https://example.org/john-doe".to_string(),
            text: "John Doe".to_string(),
            rel: vec![],
            is_external: true,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(!EntityLinkingAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "ELINK001"));
    }

    #[test]
    fn test_entity_linking_same_type_no_cross_link() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Product".to_string()),
                data: serde_json::json!({"@type": "Product", "name": "Widget A"}),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Product".to_string()),
                data: serde_json::json!({"@type": "Product", "name": "Widget B"}),
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        assert!(EntityLinkingAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "ELINK002"));
    }

    #[test]
    fn test_entity_linking_single_entity_no_cross_link_issue() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product", "name": "Widget A"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(!EntityLinkingAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "ELINK002"));
    }

    #[test]
    fn test_entity_linking_empty_name_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Person".to_string()),
            data: serde_json::json!({"@type": "Person", "name": ""}),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(EntityLinkingAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_entity_linking_internal_link_not_wikipedia() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Person".to_string()),
            data: serde_json::json!({"@type": "Person", "name": "John Doe"}),
        }];
        page.links = vec![ExtractedLink {
            href: "https://example.com/about".to_string(),
            text: "About us".to_string(),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(EntityLinkingAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "ELINK001"));
    }

    #[test]
    fn test_entity_linking_wikipedia_in_href_no_text() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Person".to_string()),
            data: serde_json::json!({"@type": "Person", "name": "Albert Einstein"}),
        }];
        page.links = vec![ExtractedLink {
            href: "https://en.wikipedia.org/wiki/Albert_Einstein".to_string(),
            text: "Read more".to_string(),
            rel: vec![],
            is_external: true,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(!EntityLinkingAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "ELINK001"));
    }

    #[test]
    fn test_entity_linking_different_type_groups() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Person".to_string()),
                data: serde_json::json!({"@type": "Person", "name": "John Doe"}),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Product".to_string()),
                data: serde_json::json!({"@type": "Product", "name": "Widget"}),
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        assert!(!EntityLinkingAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "ELINK002"));
    }

    #[test]
    fn test_entity_linking_no_name_in_schema() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Person".to_string()),
            data: serde_json::json!({"@type": "Person", "description": "A person"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(EntityLinkingAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_duplicate_title_description_high_overlap() {
        let mut page = make_page("https://example.com");
        // Title and description share 6 out of 7 unique words (>90% overlap)
        page.meta.title = Some("Premium Quality Widgets Available Here Purchase".to_string());
        page.meta.description =
            Some("Premium Quality Widgets Available Here Purchase Today".to_string());
        let ctx = make_ctx(&page, Some(200));
        assert!(DuplicateContentDetector::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "DUP001"));
    }

    #[test]
    fn test_duplicate_title_description_different() {
        let mut page = make_page("https://example.com");
        page.meta.title = Some("Best Widgets for Sale".to_string());
        page.meta.description =
            Some("Premium quality widgets with free shipping and 30-day returns".to_string());
        let ctx = make_ctx(&page, Some(200));
        assert!(!DuplicateContentDetector::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "DUP001"));
    }

    #[test]
    fn test_duplicate_description_starts_with_title() {
        let mut page = make_page("https://example.com");
        page.meta.title = Some("Widget Product Page".to_string());
        page.meta.description =
            Some("Widget Product Page - Learn more about our amazing widgets".to_string());
        let ctx = make_ctx(&page, Some(200));
        assert!(DuplicateContentDetector::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "DUP002"));
    }

    #[test]
    fn test_duplicate_low_content_diversity() {
        let mut page = make_page("https://example.com");
        // Create repetitive content: same word repeated many times
        let words: Vec<&str> = vec!["widget"; 200];
        page.word_count = 200;
        let body_text = words.join(" ");
        let ctx = make_ctx_with_body(&page, Some(200), &body_text);
        let findings = DuplicateContentDetector::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "DUP003"));
    }

    #[test]
    fn test_duplicate_diverse_content_no_dup003() {
        let mut page = make_page("https://example.com");
        let body_text = "The quick brown fox jumps over the lazy dog. A journey of a thousand miles begins with a single step. Knowledge is power but enthusiasm pulls the switch. Actions speak louder than words. The pen is mightier than the sword.";
        page.word_count = body_text.split_whitespace().count();
        let ctx = make_ctx_with_body(&page, Some(200), body_text);
        assert!(!DuplicateContentDetector::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "DUP003"));
    }

    #[test]
    fn test_toc_missing_on_long_page() {
        let mut page = make_page("https://example.com/guide");
        page.word_count = 3000;
        page.headings = vec![
            crate::parser::Heading {
                level: 1,
                text: "Intro".to_string(),
                length: "Intro".len(),
            },
            crate::parser::Heading {
                level: 2,
                text: "Section 1".to_string(),
                length: "Section 1".len(),
            },
            crate::parser::Heading {
                level: 2,
                text: "Section 2".to_string(),
                length: "Section 2".len(),
            },
            crate::parser::Heading {
                level: 2,
                text: "Section 3".to_string(),
                length: "Section 3".len(),
            },
            crate::parser::Heading {
                level: 2,
                text: "Section 4".to_string(),
                length: "Section 4".len(),
            },
            crate::parser::Heading {
                level: 2,
                text: "Section 5".to_string(),
                length: "Section 5".len(),
            },
            crate::parser::Heading {
                level: 2,
                text: "Section 6".to_string(),
                length: "Section 6".len(),
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        assert!(TableOfContentsAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "TOC001"));
    }

    #[test]
    fn test_toc_not_flagged_on_short_page() {
        let mut page = make_page("https://example.com/about");
        page.word_count = 500;
        page.headings = vec![crate::parser::Heading {
            level: 1,
            text: "About".to_string(),
            length: "About".len(),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(!TableOfContentsAnalyzer::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "TOC001"));
    }

    // ---- ContentTopicCoverageAnalyzer tests ----

    #[test]
    fn test_topcov_no_headings() {
        let page = make_page("https://example.com");
        let ctx = make_ctx_with_body(&page, Some(200), "Some body text here.");
        assert!(ContentTopicCoverageAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_topcov_no_body() {
        let mut page = make_page("https://example.com");
        page.headings = vec![crate::parser::Heading {
            level: 1,
            text: "Rust Programming".to_string(),
            length: 16,
        }];
        page.word_count = 100;
        let ctx = make_ctx(&page, Some(200));
        assert!(ContentTopicCoverageAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_topcov_empty_body() {
        let mut page = make_page("https://example.com");
        page.headings = vec![crate::parser::Heading {
            level: 1,
            text: "Rust Programming".to_string(),
            length: 16,
        }];
        page.word_count = 0;
        let ctx = make_ctx_with_body(&page, Some(200), "");
        assert!(ContentTopicCoverageAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_topcov_zero_word_count() {
        let mut page = make_page("https://example.com");
        page.headings = vec![crate::parser::Heading {
            level: 1,
            text: "Rust Programming".to_string(),
            length: 16,
        }];
        page.word_count = 0;
        let ctx = make_ctx_with_body(&page, Some(200), "Some content");
        assert!(ContentTopicCoverageAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_topcov_good_coverage() {
        let mut page = make_page("https://example.com");
        page.headings = vec![crate::parser::Heading {
            level: 1,
            text: "Rust Programming Guide".to_string(),
            length: 22,
        }];
        page.word_count = 100;
        let body = "Rust is a systems programming language. Programming in Rust is safe and fast. This guide covers all the basics of Rust programming.";
        let ctx = make_ctx_with_body(&page, Some(200), body);
        let findings = ContentTopicCoverageAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "TOPCOV001"));
    }

    #[test]
    fn test_topcov_poor_coverage() {
        let mut page = make_page("https://example.com");
        page.headings = vec![crate::parser::Heading {
            level: 1,
            text: "Rust Programming Guide".to_string(),
            length: 22,
        }];
        page.word_count = 100;
        let body = "The quick brown fox jumps over the lazy dog. A journey of a thousand miles begins with a single step. Knowledge is power.";
        let ctx = make_ctx_with_body(&page, Some(200), body);
        let findings = ContentTopicCoverageAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "TOPCOV001"));
    }

    #[test]
    fn test_topcov_strictly_below_threshold() {
        let mut page = make_page("https://example.com");
        page.headings = vec![crate::parser::Heading {
            level: 1,
            text: "Advanced Rust Programming Guide".to_string(),
            length: 30,
        }];
        page.word_count = 100;
        let body = "This guide covers everything you need to know about getting started with technology. The guide is comprehensive and detailed with many examples.";
        let ctx = make_ctx_with_body(&page, Some(200), body);
        let findings = ContentTopicCoverageAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "TOPCOV001"));
    }

    #[test]
    fn test_topcov_stop_words_excluded() {
        let mut page = make_page("https://example.com");
        page.headings = vec![crate::parser::Heading {
            level: 1,
            text: "The Best Rust Guide".to_string(),
            length: 19,
        }];
        page.word_count = 100;
        let body = "Rust is great. This guide will help you learn. The guide is comprehensive.";
        let ctx = make_ctx_with_body(&page, Some(200), body);
        let findings = ContentTopicCoverageAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "TOPCOV001"));
    }

    #[test]
    fn test_topcov_case_insensitive() {
        let mut page = make_page("https://example.com");
        page.headings = vec![crate::parser::Heading {
            level: 1,
            text: "RUST Programming".to_string(),
            length: 16,
        }];
        page.word_count = 100;
        let body = "rust is a language. Programming in rust is fun.";
        let ctx = make_ctx_with_body(&page, Some(200), body);
        let findings = ContentTopicCoverageAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "TOPCOV001"));
    }

    #[test]
    fn test_topcov_empty_heading_text() {
        let mut page = make_page("https://example.com");
        page.headings = vec![crate::parser::Heading {
            level: 1,
            text: "".to_string(),
            length: 0,
        }];
        page.word_count = 100;
        let ctx = make_ctx_with_body(&page, Some(200), "Some content here.");
        let findings = ContentTopicCoverageAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_topcov_multiple_headings() {
        let mut page = make_page("https://example.com");
        page.headings = vec![
            crate::parser::Heading {
                level: 1,
                text: "Rust Programming".to_string(),
                length: 16,
            },
            crate::parser::Heading {
                level: 2,
                text: "Getting Started".to_string(),
                length: 15,
            },
        ];
        page.word_count = 100;
        let body = "Rust is a systems programming language. Programming in Rust is safe and fast.";
        let ctx = make_ctx_with_body(&page, Some(200), body);
        let findings = ContentTopicCoverageAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "TOPCOV001"));
    }

    #[test]
    fn test_topcov_severity_warning() {
        let mut page = make_page("https://example.com");
        page.headings = vec![crate::parser::Heading {
            level: 1,
            text: "Advanced Rust Programming Guide".to_string(),
            length: 30,
        }];
        page.word_count = 100;
        let body =
            "This guide covers everything you need to know about getting started with technology.";
        let ctx = make_ctx_with_body(&page, Some(200), body);
        let findings = ContentTopicCoverageAnalyzer::new().analyze(&ctx);
        if let Some(f) = findings.iter().find(|f| f.code == "TOPCOV001") {
            assert_eq!(f.severity, Severity::Warning);
            assert_eq!(f.category, IssueCategory::Content);
        }
    }

    #[test]
    fn test_topcov_name() {
        assert_eq!(
            ContentTopicCoverageAnalyzer::new().name(),
            "content-topic-coverage"
        );
    }

    #[test]
    fn test_topcov_no_body_text_no_findings() {
        let mut page = make_page("https://example.com");
        page.headings = vec![crate::parser::Heading {
            level: 1,
            text: "Rust".to_string(),
            length: 4,
        }];
        page.word_count = 100;
        let ctx = make_ctx(&page, Some(200));
        assert!(ContentTopicCoverageAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_topcov_short_heading_keywords() {
        let mut page = make_page("https://example.com");
        page.headings = vec![crate::parser::Heading {
            level: 1,
            text: "Is It OK".to_string(),
            length: 8,
        }];
        page.word_count = 100;
        let ctx = make_ctx_with_body(&page, Some(200), "Some content here.");
        let findings = ContentTopicCoverageAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }
}

// =========================================================================
// OpenGraphVideoUrlValidator
// =========================================================================

pub struct OpenGraphVideoUrlValidator;

impl Default for OpenGraphVideoUrlValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl OpenGraphVideoUrlValidator {
    pub fn new() -> Self {
        Self
    }

    fn is_valid_video_url(url: &str) -> bool {
        if url.is_empty() {
            return false;
        }
        url::Url::parse(url).is_ok() && (url.starts_with("http://") || url.starts_with("https://"))
    }
}

impl Analyzer for OpenGraphVideoUrlValidator {
    fn name(&self) -> &str {
        "og-video-url-validator"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let video_url = match ctx.page.meta.og.get("video") {
            Some(v) => v,
            None => return findings,
        };

        if video_url.is_empty() {
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Social,
                code: "OGVIDURL001".to_string(),
                title: "Empty og:video URL".to_string(),
                description: "The og:video meta tag has an empty value.".to_string(),
                url: url.clone(),
                recommendation: "Provide a valid video URL in the og:video meta tag.".to_string(),
            });
            return findings;
        }

        if !Self::is_valid_video_url(video_url) {
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Social,
                code: "OGVIDURL001".to_string(),
                title: "Invalid og:video URL format".to_string(),
                description: format!(
                    "The og:video URL \"{video_url}\" is not a valid HTTP/HTTPS URL."
                ),
                url: url.clone(),
                recommendation: "Ensure og:video points to a valid HTTP or HTTPS URL.".to_string(),
            });
        }

        findings
    }
}

// =========================================================================
// TwitterPlayerStreamValidator
// =========================================================================

pub struct TwitterPlayerStreamValidator;

impl Default for TwitterPlayerStreamValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl TwitterPlayerStreamValidator {
    pub fn new() -> Self {
        Self
    }

    fn is_valid_stream_url(url: &str) -> bool {
        if url.is_empty() {
            return false;
        }
        url::Url::parse(url).is_ok() && (url.starts_with("http://") || url.starts_with("https://"))
    }
}

impl Analyzer for TwitterPlayerStreamValidator {
    fn name(&self) -> &str {
        "twitter-player-stream-validator"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let stream_url = match &ctx.page.meta.twitter.player_stream {
            Some(s) => s.as_str(),
            None => return findings,
        };

        if stream_url.is_empty() {
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Social,
                code: "TWSTREAM001".to_string(),
                title: "Empty twitter:player:stream URL".to_string(),
                description: "The twitter:player:stream meta tag has an empty value.".to_string(),
                url: url.clone(),
                recommendation: "Provide a valid video stream URL in twitter:player:stream."
                    .to_string(),
            });
            return findings;
        }

        if !Self::is_valid_stream_url(stream_url) {
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Social,
                code: "TWSTREAM001".to_string(),
                title: "Invalid twitter:player:stream URL format".to_string(),
                description: format!(
                    "The twitter:player:stream URL \"{stream_url}\" is not a valid HTTP/HTTPS URL."
                ),
                url: url.clone(),
                recommendation: "Ensure twitter:player:stream points to a valid HTTP or HTTPS URL."
                    .to_string(),
            });
        }

        findings
    }
}

// =========================================================================
// SchemaNestingDepthValidator
// =========================================================================

pub struct SchemaNestingDepthValidator;

impl Default for SchemaNestingDepthValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl SchemaNestingDepthValidator {
    pub fn new() -> Self {
        Self
    }

    fn max_nesting_depth(value: &serde_json::Value, depth: usize) -> usize {
        match value {
            serde_json::Value::Object(map) => {
                let child_max = map
                    .values()
                    .map(|v| Self::max_nesting_depth(v, depth + 1))
                    .max()
                    .unwrap_or(depth);
                child_max.max(depth)
            }
            serde_json::Value::Array(arr) => arr
                .iter()
                .map(|v| Self::max_nesting_depth(v, depth))
                .max()
                .unwrap_or(depth),
            _ => depth,
        }
    }
}

impl Analyzer for SchemaNestingDepthValidator {
    fn name(&self) -> &str {
        "schema-nesting-depth"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            let depth = Self::max_nesting_depth(&sd.data, 0);
            if depth > 3 {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "SCNEST001".to_string(),
                    title: "Schema nesting depth exceeds 3 levels".to_string(),
                    description: format!(
                        "JSON-LD block for type \"{}\" has a nesting depth of {} levels. \
                         Deeply nested schemas may not be fully parsed by search engines.",
                        sd.r#type.as_deref().unwrap_or("unknown"),
                        depth
                    ),
                    url: url.clone(),
                    recommendation: "Flatten nested schema structures to 3 levels or fewer for \
                                     better search engine parsing."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// SchemaIdReferenceValidator
// =========================================================================

pub struct SchemaIdReferenceValidator;

impl Default for SchemaIdReferenceValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl SchemaIdReferenceValidator {
    pub fn new() -> Self {
        Self
    }

    fn collect_ids(schemas: &[crate::parser::StructuredData]) -> HashSet<String> {
        let mut ids = HashSet::new();
        for sd in schemas {
            if let Some(id) = sd.data.get("@id").and_then(|v| v.as_str()) {
                if !id.is_empty() {
                    ids.insert(id.to_string());
                }
            }
        }
        ids
    }

    fn collect_refs(value: &serde_json::Value, refs: &mut Vec<String>) {
        match value {
            serde_json::Value::Object(map) => {
                for (key, val) in map {
                    // Collect @id references from any property (including nested objects)
                    if (key == "@id" || key.ends_with("/@id") || key == "id") && val.is_string() {
                        if let Some(s) = val.as_str() {
                            if s.starts_with('#') {
                                refs.push(s.to_string());
                            }
                        }
                    } else if let serde_json::Value::String(s) = val {
                        if s.contains('#') && !s.starts_with("http") {
                            refs.push(s.clone());
                        }
                    }
                    Self::collect_refs(val, refs);
                }
            }
            serde_json::Value::Array(arr) => {
                for item in arr {
                    Self::collect_refs(item, refs);
                }
            }
            _ => {}
        }
    }
}

impl Analyzer for SchemaIdReferenceValidator {
    fn name(&self) -> &str {
        "schema-id-reference"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let ids = Self::collect_ids(&ctx.page.structured_data);

        for sd in &ctx.page.structured_data {
            // Collect all @id fragment references from this schema's properties
            let mut refs = Vec::new();
            Self::collect_refs(&sd.data, &mut refs);
            for ref_id in &refs {
                if ref_id.starts_with('#') && !ids.contains(ref_id) {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Schema,
                        code: "SCREF001".to_string(),
                        title: "Schema @id reference has no matching target".to_string(),
                        description: format!(
                            "Schema block references @id=\"{ref_id}\" but no other schema \
                             block defines this @id."
                        ),
                        url: url.clone(),
                        recommendation: "Ensure all @id references have a corresponding \
                                         @id target defined in another schema block."
                            .to_string(),
                    });
                }
            }
        }

        findings
    }
}

// =========================================================================
// BreadcrumbActivePageValidator
// =========================================================================

pub struct BreadcrumbActivePageValidator;

impl Default for BreadcrumbActivePageValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl BreadcrumbActivePageValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for BreadcrumbActivePageValidator {
    fn name(&self) -> &str {
        "breadcrumb-active-page"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            let schema_type = sd.data.get("@type").and_then(|t| t.as_str()).unwrap_or("");
            if schema_type != "BreadcrumbList" {
                continue;
            }

            let items = match sd.data.get("itemListElement").and_then(|i| i.as_array()) {
                Some(arr) if !arr.is_empty() => arr,
                _ => continue,
            };

            if let Some(last_item) = items.last() {
                let item_url = last_item
                    .get("item")
                    .and_then(|i| i.get("@id").or(i.get("url")))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                if !item_url.is_empty() && item_url != url {
                    // Normalize for comparison
                    let normalize = |u: &str| -> String {
                        u.trim_end_matches('/')
                            .to_lowercase()
                            .replace("https://", "http://")
                    };
                    if normalize(item_url) != normalize(url) {
                        findings.push(Finding {
                            severity: Severity::Warning,
                            category: IssueCategory::Content,
                            code: "BREADACT001".to_string(),
                            title: "Last breadcrumb does not match current URL".to_string(),
                            description: format!(
                                "The last breadcrumb item points to \"{item_url}\" but the \
                                 current page URL is \"{url}\"."
                            ),
                            url: url.clone(),
                            recommendation: "The last breadcrumb item should represent the \
                                             current page. Update the breadcrumb list to match \
                                             the actual page URL."
                                .to_string(),
                        });
                    }
                }
            }
        }

        findings
    }
}

// =========================================================================
// ContentLanguageValidator
// =========================================================================

pub struct ContentLanguageValidator;

impl Default for ContentLanguageValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ContentLanguageValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for ContentLanguageValidator {
    fn name(&self) -> &str {
        "content-language"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let html_lang = match &ctx.page.html_lang {
            Some(l) if !l.is_empty() => l,
            _ => return findings,
        };

        // Check meta language vs html lang
        if let Some(meta_lang) = &ctx.page.meta.language {
            if !meta_lang.is_empty() && meta_lang != html_lang {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Content,
                    code: "CLANG001".to_string(),
                    title: "Content language mismatch with html lang".to_string(),
                    description: format!(
                        "The meta language \"{meta_lang}\" differs from the html lang \
                         attribute \"{html_lang}\". Language declarations should be consistent."
                    ),
                    url: url.clone(),
                    recommendation: "Ensure the meta language and html lang attribute declare \
                                     the same language."
                        .to_string(),
                });
            }
        }

        // Check hreflang consistency
        for tag in &ctx.page.meta.hreflang {
            if tag.lang.to_lowercase() != "x-default" {
                let tag_lang_base = tag.lang.split('-').next().unwrap_or(&tag.lang);
                let html_lang_base = html_lang.split('-').next().unwrap_or(html_lang);
                if tag_lang_base == html_lang_base {
                    // Found a matching hreflang — language is consistent
                    return findings;
                }
            }
        }

        if !ctx.page.meta.hreflang.is_empty() {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Content,
                code: "CLANG001".to_string(),
                title: "No hreflang matches html lang".to_string(),
                description: format!(
                    "The html lang is \"{html_lang}\" but no hreflang tag matches this language \
                     code."
                ),
                url: url.clone(),
                recommendation: "Add an hreflang tag for the primary language or verify the \
                                 html lang attribute is correct."
                    .to_string(),
            });
        }

        findings
    }
}

// =========================================================================
// ContentTopicCoverageAnalyzer
// =========================================================================

/// Checks if body content covers the same topics as headings (Lumar NLP-lite angle).
pub struct ContentTopicCoverageAnalyzer;

impl Default for ContentTopicCoverageAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl ContentTopicCoverageAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Tokenize text into lowercase words > 2 chars, excluding stop words.
    fn tokenize(text: &str) -> HashSet<String> {
        text.to_lowercase()
            .split_whitespace()
            .map(|w| {
                w.to_lowercase()
                    .chars()
                    .filter(|c| c.is_alphanumeric())
                    .collect::<String>()
            })
            .filter(|w| w.len() > 2 && !STOP_WORDS.contains(&w.as_str()))
            .collect()
    }
}

impl Analyzer for ContentTopicCoverageAnalyzer {
    fn name(&self) -> &str {
        "content-topic-coverage"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        if ctx.page.headings.is_empty() || ctx.page.word_count == 0 {
            return findings;
        }

        let heading_text: String = ctx
            .page
            .headings
            .iter()
            .map(|h| h.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        let heading_keywords = Self::tokenize(&heading_text);

        if heading_keywords.is_empty() {
            return findings;
        }

        let body_text = ctx.body.unwrap_or("");
        if body_text.is_empty() {
            return findings;
        }

        let body_words = Self::tokenize(body_text);
        if body_words.is_empty() {
            return findings;
        }

        let covered = heading_keywords
            .iter()
            .filter(|kw| body_words.contains(*kw))
            .count();
        let total = heading_keywords.len();
        let ratio = covered as f64 / total as f64;

        if ratio < 0.30 {
            let missing: Vec<&String> = heading_keywords
                .iter()
                .filter(|kw| !body_words.contains(*kw))
                .collect();
            let missing_display: Vec<&str> = missing.iter().take(5).map(|s| s.as_str()).collect();
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Content,
                code: "TOPCOV001".to_string(),
                title: "Body content lacks heading topic coverage".to_string(),
                description: format!(
                    "Only {:.0}% of heading keywords ({}/{}) appear in the body text.                      Headings introduce topics that the body content should elaborate on.                      Missing keywords: {}",
                    ratio * 100.0,
                    covered,
                    total,
                    missing_display.join(", ")
                ),
                url: url.clone(),
                recommendation: "Ensure body content elaborates on the topics introduced by                                  headings. Each heading keyword should be discussed in the                                  corresponding section."
                    .to_string(),
            });
        }

        findings
    }
}

// =========================================================================
// JSON-LD Validator
// =========================================================================

pub struct JsonLdValidator;

impl JsonLdValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for JsonLdValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for JsonLdValidator {
    fn name(&self) -> &str {
        "jsonld-validator"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        if ctx.page.structured_data.is_empty() {
            return findings;
        }

        for sd in &ctx.page.structured_data {
            // Check for empty JSON-LD (context and type both missing suggests empty/invalid)
            let is_empty = sd.context.is_none()
                && sd.r#type.is_none()
                && (sd.data.is_object() && sd.data.as_object().map_or(false, |m| m.is_empty()));

            if is_empty {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "JSONLD001".to_string(),
                    title: "Empty JSON-LD script tag".to_string(),
                    description: "A JSON-LD script tag is present but contains no data. Empty \
                                  JSON-LD blocks waste bytes and may confuse parsers."
                        .to_string(),
                    url: url.to_string(),
                    recommendation: "Either populate the JSON-LD block with valid structured \
                                     data or remove the empty script tag."
                        .into(),
                });
                continue;
            }

            // Check @context is schema.org
            match &sd.context {
                None => {
                    findings.push(Finding {
                        severity: Severity::Error,
                        category: IssueCategory::Schema,
                        code: "JSONLD002".to_string(),
                        title: "JSON-LD @context is not schema.org".to_string(),
                        description: "JSON-LD block is missing @context or it is not set to \
                                      schema.org. Search engines require @context: schema.org \
                                      for structured data."
                            .to_string(),
                        url: url.to_string(),
                        recommendation: "Set @context to \"https://schema.org\" in the JSON-LD \
                                         block."
                            .into(),
                    });
                }
                Some(ctx_val) => {
                    if ctx_val != "https://schema.org" && ctx_val != "schema.org" {
                        findings.push(Finding {
                            severity: Severity::Warning,
                            category: IssueCategory::Schema,
                            code: "JSONLD002".to_string(),
                            title: "JSON-LD @context is not schema.org".to_string(),
                            description: format!(
                                "JSON-LD @context is \"{ctx_val}\" instead of \
                                 \"https://schema.org\"."
                            ),
                            url: url.to_string(),
                            recommendation: "Set @context to \"https://schema.org\" in the \
                                             JSON-LD block."
                                .into(),
                        });
                    }
                }
            }

            // Check @type is present
            if sd.r#type.is_none() {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Schema,
                    code: "JSONLD003".to_string(),
                    title: "JSON-LD @type is missing".to_string(),
                    description: "JSON-LD block is missing the @type property. The @type \
                                  property is required for search engines to understand the \
                                  structured data."
                        .to_string(),
                    url: url.to_string(),
                    recommendation: "Add an @type property (e.g., \"Article\", \"Product\", \
                                     \"Organization\") to the JSON-LD block."
                        .into(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// Meta Description Length Analyzer
// =========================================================================

pub struct MetaDescriptionLengthAnalyzer;

impl MetaDescriptionLengthAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MetaDescriptionLengthAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for MetaDescriptionLengthAnalyzer {
    fn name(&self) -> &str {
        "meta-description-length"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let description = match &ctx.page.meta.description {
            Some(d) if !d.trim().is_empty() => d.trim(),
            _ => return findings,
        };

        let len = description.chars().count();

        if len < 70 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "METADESC001".to_string(),
                title: "Meta description too short".to_string(),
                description: format!(
                    "Meta description is {len} characters, which is below the recommended \
                     minimum of 70 characters. Short descriptions may be truncated or ignored \
                     by search engines."
                ),
                url: url.to_string(),
                recommendation: "Write a meta description of at least 70 characters that \
                                 accurately summarizes the page content."
                    .into(),
            });
        }

        if len > 160 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "METADESC002".to_string(),
                title: "Meta description too long".to_string(),
                description: format!(
                    "Meta description is {len} characters, which exceeds the recommended \
                     maximum of 160 characters. Search engines will truncate descriptions \
                     longer than this."
                ),
                url: url.to_string(),
                recommendation: "Keep the meta description under 160 characters to ensure \
                                 it displays fully in search results."
                    .into(),
            });
        }

        // Check if description is same as title
        if let Some(title) = &ctx.page.meta.title {
            if description == title.trim() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "METADESC003".to_string(),
                    title: "Meta description identical to title".to_string(),
                    description: "The meta description is exactly the same as the page title. \
                                  Title and description should provide complementary information \
                                  to maximize click-through rates from search results."
                        .to_string(),
                    url: url.to_string(),
                    recommendation: "Write a unique meta description that complements the title \
                                     rather than duplicating it."
                        .into(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// Title Length Analyzer
// =========================================================================

pub struct TitleLengthAnalyzer;

impl TitleLengthAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TitleLengthAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for TitleLengthAnalyzer {
    fn name(&self) -> &str {
        "title-length"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let title = match &ctx.page.meta.title {
            Some(t) if !t.trim().is_empty() => t.trim(),
            _ => return findings,
        };

        let len = title.chars().count();

        if len < 30 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "TITLE001".to_string(),
                title: "Title too short".to_string(),
                description: format!(
                    "Title is {len} characters, which is below the recommended minimum of 30 \
                     characters. Short titles may not provide enough context for search engines \
                     or users."
                ),
                url: url.to_string(),
                recommendation: "Write a title of at least 30 characters that includes your \
                                 primary keyword."
                    .into(),
            });
        }

        if len > 60 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "TITLE002".to_string(),
                title: "Title too long".to_string(),
                description: format!(
                    "Title is {len} characters, which exceeds the recommended maximum of 60 \
                     characters. Search engines typically truncate titles longer than this in \
                     search results."
                ),
                url: url.to_string(),
                recommendation: "Keep the title under 60 characters to ensure it displays \
                                 fully in search results."
                    .into(),
            });
        }

        // Check for pipe separators suggesting CMS auto-generation
        if title.contains('|') || title.contains(" – ") || title.contains(" - ") {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Seo,
                code: "TITLE003".to_string(),
                title: "Title contains separator characters".to_string(),
                description: format!(
                    "Title \"{title}\" contains pipe (|) or dash separators, which often \
                     indicates CMS auto-generation. Search engines may truncate these at the \
                     separator."
                ),
                url: url.to_string(),
                recommendation: "Consider removing separator-based title patterns (e.g., \
                                 \"Page | Site Name\") and writing unique, descriptive titles."
                    .into(),
            });
        }

        findings
    }
}

// =========================================================================
// Content Thin Analyzer
// =========================================================================

pub struct ContentThinAnalyzer;

impl ContentThinAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ContentThinAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for ContentThinAnalyzer {
    fn name(&self) -> &str {
        "content-thin"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        // THIN002: any page with <100 words
        if ctx.page.word_count < 100 {
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Content,
                code: "THIN002".to_string(),
                title: "Extremely thin content".to_string(),
                description: format!(
                    "Page has only {} word(s). Pages with fewer than 100 words are unlikely to \
                     rank for any meaningful search queries and may be penalized by search \
                     engines as thin content.",
                    ctx.page.word_count
                ),
                url: url.to_string(),
                recommendation: "Add substantial, unique content. Aim for at least 300 words \
                                 for informational pages."
                    .into(),
            });
            return findings;
        }

        // THIN001: non-utility pages with <300 words
        if ctx.page.word_count < 300 && !is_utility_page(url) {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Content,
                code: "THIN001".to_string(),
                title: "Thin content on non-utility page".to_string(),
                description: format!(
                    "Page has {} word(s), which is below the recommended minimum of 300 words \
                     for non-utility pages. Thin content may not provide enough value to rank \
                     well in search results.",
                    ctx.page.word_count
                ),
                url: url.to_string(),
                recommendation: "Expand the content to at least 300 words with useful, \
                                 original information that satisfies user intent."
                    .into(),
            });
        }

        findings
    }
}

// =========================================================================
// MetaDescriptionUniquenessAnalyzer
// =========================================================================

pub struct MetaDescriptionUniquenessAnalyzer;

impl Default for MetaDescriptionUniquenessAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl MetaDescriptionUniquenessAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for MetaDescriptionUniquenessAnalyzer {
    fn name(&self) -> &str {
        "meta-description-uniqueness"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let description = match &ctx.page.meta.description {
            Some(d) if !d.trim().is_empty() => d.trim(),
            _ => return findings,
        };

        if description.len() < 10 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Seo,
                code: "METADESC-UNI001".to_string(),
                title: "Meta description is very short".to_string(),
                description: format!(
                    "Meta description is only {} characters. Very short descriptions may be \
                     too generic to differentiate this page from others in search results.",
                    description.len()
                ),
                url: url.clone(),
                recommendation: "Write a unique, descriptive meta description of 120-160 \
                                 characters for each page."
                    .to_string(),
            });
        }

        let lower = description.to_lowercase();
        let generic_patterns = [
            "welcome to",
            "click here",
            "learn more",
            "read more",
            "this page",
            "this website",
            "coming soon",
            "under construction",
        ];
        for pattern in &generic_patterns {
            if lower.contains(pattern) {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "METADESC-UNI002".to_string(),
                    title: "Meta description contains generic text".to_string(),
                    description: format!(
                        "Meta description contains the phrase \"{pattern}\", which is generic \
                         boilerplate text."
                    ),
                    url: url.clone(),
                    recommendation: "Write a unique description that accurately summarizes the \
                                     page content and includes relevant keywords."
                        .to_string(),
                });
                break;
            }
        }

        findings
    }
}

// =========================================================================
// ContentFreshnessDateAnalyzer
// =========================================================================

pub struct ContentFreshnessDateAnalyzer;

impl Default for ContentFreshnessDateAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl ContentFreshnessDateAnalyzer {
    pub fn new() -> Self {
        Self
    }

    fn is_time_sensitive_url(url: &str) -> bool {
        let lower = url.to_lowercase();
        [
            "/blog/",
            "/news/",
            "/article/",
            "/post/",
            "/press/",
            "/release/",
            "/update/",
            "/announcement/",
            "/changelog/",
        ]
        .iter()
        .any(|p| lower.contains(p))
    }

    fn has_date_metadata(ctx: &AnalysisContext) -> bool {
        for sd in &ctx.page.structured_data {
            if sd.data.get("datePublished").is_some()
                || sd.data.get("dateModified").is_some()
                || sd.data.get("dateCreated").is_some()
            {
                return true;
            }
        }
        false
    }
}

impl Analyzer for ContentFreshnessDateAnalyzer {
    fn name(&self) -> &str {
        "content-freshness-date"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        if Self::is_time_sensitive_url(url) && !Self::has_date_metadata(ctx) {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Content,
                code: "FRESH-DATE001".to_string(),
                title: "No date metadata on time-sensitive content".to_string(),
                description: "This page appears to be time-sensitive content (blog, news, \
                              article) but has no date metadata in structured data."
                    .to_string(),
                url: url.clone(),
                recommendation: "Add datePublished and dateModified to the Article schema."
                    .to_string(),
            });
        }

        findings
    }
}

// =========================================================================
// StructuredDataNestingValidator
// =========================================================================

pub struct StructuredDataNestingValidator;

impl Default for StructuredDataNestingValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl StructuredDataNestingValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for StructuredDataNestingValidator {
    fn name(&self) -> &str {
        "structured-data-nesting"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Product") {
                continue;
            }
            let data = &sd.data;

            match data.get("offers") {
                None => {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Schema,
                        code: "SDNEST001".to_string(),
                        title: "Product schema missing nested Offer".to_string(),
                        description: "A Product structured data block is missing the \"offers\" \
                                      property containing an Offer object."
                            .to_string(),
                        url: url.clone(),
                        recommendation: "Add \"offers\" with an Offer object.".to_string(),
                    });
                }
                Some(offers) => {
                    let offer_list: Vec<&serde_json::Value> = match offers {
                        serde_json::Value::Array(arr) => arr.iter().collect(),
                        other => vec![other],
                    };
                    for offer in offer_list {
                        if offer.get("price").is_none() && offer.get("lowPrice").is_none() {
                            findings.push(Finding {
                                severity: Severity::Warning,
                                category: IssueCategory::Schema,
                                code: "SDNEST002".to_string(),
                                title: "Product Offer missing price".to_string(),
                                description:
                                    "A Product Offer object is missing the \"price\" property."
                                        .to_string(),
                                url: url.clone(),
                                recommendation: "Add \"price\" with the product price.".to_string(),
                            });
                        }
                        if offer.get("availability").is_none() {
                            findings.push(Finding {
                                severity: Severity::Info,
                                category: IssueCategory::Schema,
                                code: "SDNEST003".to_string(),
                                title: "Product Offer missing availability".to_string(),
                                description: "A Product Offer object is missing the \"availability\" property."
                                    .to_string(),
                                url: url.clone(),
                                recommendation: "Add \"availability\" with a Schema.org availability value."
                                    .to_string(),
                            });
                        }
                    }
                }
            }
        }

        findings
    }
}

// =========================================================================
// LocalBusinessNapAnalyzerUtil
// =========================================================================

pub struct LocalBusinessNapAnalyzerUtil;

impl Default for LocalBusinessNapAnalyzerUtil {
    fn default() -> Self {
        Self::new()
    }
}

impl LocalBusinessNapAnalyzerUtil {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for LocalBusinessNapAnalyzerUtil {
    fn name(&self) -> &str {
        "local-business-nap-util"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            let schema_type = sd.r#type.as_deref().unwrap_or("");
            let is_local = matches!(
                schema_type,
                "LocalBusiness" | "Store" | "Restaurant" | "MedicalBusiness"
            );
            if !is_local {
                continue;
            }
            let data = &sd.data;

            if data.get("telephone").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "NAP-UTIL001".to_string(),
                    title: "LocalBusiness missing telephone".to_string(),
                    description: "A LocalBusiness structured data block is missing the \
                                  \"telephone\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"telephone\" with the business phone number.".to_string(),
                });
            }

            if data.get("address").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "NAP-UTIL002".to_string(),
                    title: "LocalBusiness missing address".to_string(),
                    description: "A LocalBusiness structured data block is missing the \
                                  \"address\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"address\" with a PostalAddress object.".to_string(),
                });
            }

            if data.get("name").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "NAP-UTIL003".to_string(),
                    title: "LocalBusiness missing name".to_string(),
                    description:
                        "A LocalBusiness structured data block is missing the \"name\" property."
                            .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with the full business name.".to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// EventLocationValidatorV2
// =========================================================================

pub struct EventLocationValidatorV2;

impl Default for EventLocationValidatorV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl EventLocationValidatorV2 {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for EventLocationValidatorV2 {
    fn name(&self) -> &str {
        "event-location-v2"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Event") {
                continue;
            }
            let data = &sd.data;

            match data.get("location") {
                None => {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Schema,
                        code: "ELOC-V2001".to_string(),
                        title: "Event missing location".to_string(),
                        description:
                            "An Event structured data block is missing the \"location\" property."
                                .to_string(),
                        url: url.clone(),
                        recommendation:
                            "Add \"location\" with a Place, VirtualLocation, or PostalAddress."
                                .to_string(),
                    });
                }
                Some(location) => {
                    if location.get("name").is_none()
                        && location.get("url").is_none()
                        && location.get("address").is_none()
                    {
                        findings.push(Finding {
                            severity: Severity::Warning,
                            category: IssueCategory::Schema,
                            code: "ELOC-V2002".to_string(),
                            title: "Event location missing name".to_string(),
                            description: "The Event location object does not contain a \"name\", \
                                          \"url\", or \"address\" sub-property."
                                .to_string(),
                            url: url.clone(),
                            recommendation: "Add \"name\" to the location object.".to_string(),
                        });
                    }
                }
            }
        }

        findings
    }
}

// =========================================================================
// OrganizationLogoValidatorV2
// =========================================================================

pub struct OrganizationLogoValidatorV2;

impl Default for OrganizationLogoValidatorV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl OrganizationLogoValidatorV2 {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for OrganizationLogoValidatorV2 {
    fn name(&self) -> &str {
        "organization-logo-v2"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Organization") {
                continue;
            }
            let data = &sd.data;

            match data.get("logo") {
                None => {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Schema,
                        code: "OLOGO-V2001".to_string(),
                        title: "Organization missing logo".to_string(),
                        description: "An Organization structured data block is missing the \
                                      \"logo\" property."
                            .to_string(),
                        url: url.clone(),
                        recommendation: "Add \"logo\" with an ImageObject or URL.".to_string(),
                    });
                }
                Some(logo) => {
                    if let Some(logo_str) = logo.as_str() {
                        if logo_str.is_empty() {
                            findings.push(Finding {
                                severity: Severity::Warning,
                                category: IssueCategory::Schema,
                                code: "OLOGO-V2001".to_string(),
                                title: "Organization empty logo".to_string(),
                                description: "The Organization logo property is empty.".to_string(),
                                url: url.clone(),
                                recommendation: "Provide a valid URL to the organization logo."
                                    .to_string(),
                            });
                        }
                    }
                }
            }
        }

        findings
    }
}

// =========================================================================
// PersonJobTitleValidatorV2
// =========================================================================

pub struct PersonJobTitleValidatorV2;

impl Default for PersonJobTitleValidatorV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl PersonJobTitleValidatorV2 {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for PersonJobTitleValidatorV2 {
    fn name(&self) -> &str {
        "person-job-title-v2"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Person") {
                continue;
            }
            let data = &sd.data;

            if data.get("jobTitle").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "PJOB-V2001".to_string(),
                    title: "Person missing jobTitle".to_string(),
                    description:
                        "A Person structured data block is missing the \"jobTitle\" property."
                            .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"jobTitle\" with the person's current job title."
                        .to_string(),
                });
            }

            if data.get("worksFor").is_none() && data.get("affiliation").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "PJOB-V2002".to_string(),
                    title: "Person missing worksFor/affiliation".to_string(),
                    description: "A Person structured data block is missing \"worksFor\" or \
                                  \"affiliation\" properties."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"worksFor\" with an Organization object.".to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// RecipeNutritionValidatorV2
// =========================================================================

pub struct RecipeNutritionValidatorV2;

impl Default for RecipeNutritionValidatorV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl RecipeNutritionValidatorV2 {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for RecipeNutritionValidatorV2 {
    fn name(&self) -> &str {
        "recipe-nutrition-v2"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Recipe") {
                continue;
            }
            let data = &sd.data;

            if data.get("nutrition").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "RNUT-V2001".to_string(),
                    title: "Recipe missing nutrition information".to_string(),
                    description:
                        "A Recipe structured data block is missing the \"nutrition\" property."
                            .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"nutrition\" with a NutritionInformation object."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// CourseProviderValidatorV2
// =========================================================================

pub struct CourseProviderValidatorV2;

impl Default for CourseProviderValidatorV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl CourseProviderValidatorV2 {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for CourseProviderValidatorV2 {
    fn name(&self) -> &str {
        "course-provider-v2"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Course") {
                continue;
            }
            let data = &sd.data;

            if data.get("provider").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "CPROV-V2001".to_string(),
                    title: "Course missing provider".to_string(),
                    description:
                        "A Course structured data block is missing the \"provider\" property."
                            .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"provider\" with an Organization or Person object."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// ArticleQualityAnalyzer
// =========================================================================

pub struct ArticleQualityAnalyzer;

impl Default for ArticleQualityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl ArticleQualityAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for ArticleQualityAnalyzer {
    fn name(&self) -> &str {
        "article-quality"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        for sd in &ctx.page.structured_data {
            let type_str = sd.r#type.as_deref().unwrap_or("");
            if !matches!(
                type_str,
                "Article" | "NewsArticle" | "BlogPosting" | "ScholarlyArticle"
            ) {
                continue;
            }
            let data = &sd.data;
            if data
                .get("headline")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .is_empty()
            {
                findings.push(Finding { severity: Severity::Error, category: IssueCategory::Schema, code: "ARTQUAL001".to_string(), title: "Article schema missing headline".to_string(), description: "An Article structured data block is missing the required \"headline\" property.".to_string(), url: url.clone(), recommendation: "Add \"headline\" with a concise article title.".to_string() });
            }
            if data.get("author").is_none() {
                findings.push(Finding { severity: Severity::Error, category: IssueCategory::Schema, code: "ARTQUAL002".to_string(), title: "Article schema missing author".to_string(), description: "An Article structured data block is missing the required \"author\" property.".to_string(), url: url.clone(), recommendation: "Add \"author\" with a Person or Organization object.".to_string() });
            }
            if data.get("datePublished").is_none() {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Schema, code: "ARTQUAL003".to_string(), title: "Article schema missing datePublished".to_string(), description: "An Article structured data block is missing the \"datePublished\" property.".to_string(), url: url.clone(), recommendation: "Add \"datePublished\" with an ISO 8601 date.".to_string() });
            }
            if data.get("image").is_none() && data.get("thumbnailUrl").is_none() {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Schema, code: "ARTQUAL004".to_string(), title: "Article schema missing image".to_string(), description: "An Article structured data block has neither \"image\" nor \"thumbnailUrl\".".to_string(), url: url.clone(), recommendation: "Add \"image\" to enable rich snippet images.".to_string() });
            }
        }
        findings
    }
}

// =========================================================================
// ContentDepthAnalyzer
// =========================================================================

pub struct ContentDepthAnalyzer;

impl Default for ContentDepthAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl ContentDepthAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for ContentDepthAnalyzer {
    fn name(&self) -> &str {
        "content-depth"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let word_count = ctx.page.word_count;
        let heading_count = ctx.page.headings.len();

        if word_count == 0 {
            return findings;
        }

        if heading_count == 0 && word_count > 100 {
            findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Content, code: "CDEPTH001".to_string(), title: "Content missing headings".to_string(), description: format!("Page has {word_count} words but no headings. Headings improve content scannability and SEO."), url: url.clone(), recommendation: "Add H1-H6 headings to structure the content.".to_string() });
        }

        let h1_count = ctx.page.headings.iter().filter(|h| h.level == 1).count();
        if h1_count == 0 && word_count > 50 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Content,
                code: "CDEPTH002".to_string(),
                title: "Missing H1 heading".to_string(),
                description: "Page has no H1 heading. The H1 should describe the main topic."
                    .to_string(),
                url: url.clone(),
                recommendation: "Add exactly one H1 heading with the main topic.".to_string(),
            });
        }
        if h1_count > 1 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Content,
                code: "CDEPTH003".to_string(),
                title: "Multiple H1 headings".to_string(),
                description: format!(
                    "Page has {h1_count} H1 headings. Best practice is to have exactly one H1."
                ),
                url: url.clone(),
                recommendation: "Use exactly one H1 heading per page.".to_string(),
            });
        }

        if word_count > 50 && heading_count > 0 {
            let ratio = word_count as f64 / heading_count as f64;
            if ratio > 300.0 {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Content, code: "CDEPTH004".to_string(), title: "Low heading density".to_string(), description: format!("Average {ratio:.0} words per heading. Consider adding more subheadings for better scannability."), url: url.clone(), recommendation: "Add H2/H3 subheadings every 200-300 words.".to_string() });
            }
        }

        findings
    }
}

// =========================================================================
// HeadingCoverageAnalyzer
// =========================================================================

pub struct HeadingCoverageAnalyzer;

impl Default for HeadingCoverageAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl HeadingCoverageAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for HeadingCoverageAnalyzer {
    fn name(&self) -> &str {
        "heading-coverage"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.headings.is_empty() {
            return findings;
        }

        let levels: Vec<u8> = ctx.page.headings.iter().map(|h| h.level).collect();
        let max_level = levels.iter().copied().max().unwrap_or(0);
        if max_level > 0 {
            let has_h1 = levels.contains(&1);
            if !has_h1 {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Content,
                    code: "HCOV001".to_string(),
                    title: "Heading hierarchy missing H1".to_string(),
                    description: "Headings exist but no H1 is present.".to_string(),
                    url: url.clone(),
                    recommendation: "Add an H1 heading as the first heading on the page."
                        .to_string(),
                });
            }
        }

        let mut prev_level: u8 = 0;
        for h in &ctx.page.headings {
            if prev_level > 0 && h.level > prev_level + 1 {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Content,
                    code: "HCOV002".to_string(),
                    title: "Heading level skipped".to_string(),
                    description: format!(
                        "Heading level jumped from H{prev_level} to H{}.",
                        h.level
                    ),
                    url: url.clone(),
                    recommendation: format!(
                        "Use H{} after H{} for proper hierarchy.",
                        prev_level + 1,
                        prev_level
                    ),
                });
            }
            prev_level = h.level;
        }

        findings
    }
}

// =========================================================================
// KeywordProminenceAnalyzer
// =========================================================================

pub struct KeywordProminenceAnalyzer;

impl Default for KeywordProminenceAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl KeywordProminenceAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for KeywordProminenceAnalyzer {
    fn name(&self) -> &str {
        "keyword-prominence"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        if ctx.page.meta.title.is_none() && ctx.page.meta.description.is_none() {
            return findings;
        }

        let title_words: Vec<String> = ctx
            .page
            .meta
            .title
            .as_deref()
            .unwrap_or("")
            .split_whitespace()
            .map(|w| {
                w.to_lowercase()
                    .chars()
                    .filter(|c| c.is_alphanumeric())
                    .collect()
            })
            .filter(|w: &String| w.len() > 2 && !STOP_WORDS.contains(&w.as_str()))
            .collect();

        let desc_words: Vec<String> = ctx
            .page
            .meta
            .description
            .as_deref()
            .unwrap_or("")
            .split_whitespace()
            .map(|w| {
                w.to_lowercase()
                    .chars()
                    .filter(|c| c.is_alphanumeric())
                    .collect()
            })
            .filter(|w: &String| w.len() > 2 && !STOP_WORDS.contains(&w.as_str()))
            .collect();

        if !title_words.is_empty() && !desc_words.is_empty() {
            let overlap: usize = title_words
                .iter()
                .filter(|w| desc_words.contains(w))
                .count();
            let overlap_ratio = overlap as f64 / title_words.len() as f64;
            if overlap_ratio < 0.3 && title_words.len() >= 3 {
                findings.push(Finding { severity: Severity::Info, category: IssueCategory::Seo, code: "KWPRO001".to_string(), title: "Low keyword overlap between title and description".to_string(), description: format!("Only {overlap}/{} title keywords appear in the meta description.", title_words.len()), url: url.clone(), recommendation: "Include important keywords in both title and meta description.".to_string() });
            }
        }

        let h1_text: String = ctx
            .page
            .headings
            .iter()
            .filter(|h| h.level == 1)
            .map(|h| h.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        if !h1_text.is_empty() && !title_words.is_empty() {
            let h1_lower = h1_text.to_lowercase();
            let title_in_h1: usize = title_words
                .iter()
                .filter(|w| h1_lower.contains(w.as_str()))
                .count();
            let ratio = title_in_h1 as f64 / title_words.len() as f64;
            if ratio < 0.2 && title_words.len() >= 2 {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Seo,
                    code: "KWPRO002".to_string(),
                    title: "Title keywords missing from H1".to_string(),
                    description: "Most title keywords are not reflected in the H1 heading."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Ensure the H1 and title share core keywords for consistency."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// ContentFreshnessSignalAnalyzer
// =========================================================================

pub struct ContentFreshnessSignalAnalyzer;

impl Default for ContentFreshnessSignalAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl ContentFreshnessSignalAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for ContentFreshnessSignalAnalyzer {
    fn name(&self) -> &str {
        "content-freshness-signal"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let has_date_published = ctx
            .page
            .structured_data
            .iter()
            .any(|sd| sd.data.get("datePublished").is_some());
        let has_date_modified = ctx
            .page
            .structured_data
            .iter()
            .any(|sd| sd.data.get("dateModified").is_some());

        if !has_date_published && !has_date_modified {
            let lower_url = url.to_lowercase();
            if lower_url.contains("/blog/")
                || lower_url.contains("/news/")
                || lower_url.contains("/article/")
                || lower_url.contains("/post/")
            {
                findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Content, code: "FRESHSIG001".to_string(), title: "Blog/news content missing date signals".to_string(), description: "Blog or news URL pattern detected but no datePublished or dateModified in structured data.".to_string(), url: url.clone(), recommendation: "Add datePublished and dateModified to Article schema for freshness signals.".to_string() });
            }
        }

        if has_date_published && !has_date_modified {
            findings.push(Finding { severity: Severity::Info, category: IssueCategory::Content, code: "FRESHSIG002".to_string(), title: "Missing dateModified in schema".to_string(), description: "datePublished is present but dateModified is missing. Adding dateModified helps search engines understand content freshness.".to_string(), url: url.clone(), recommendation: "Add \"dateModified\" to the Article schema when content is updated.".to_string() });
        }

        findings
    }
}

// =========================================================================
// MetaRobotsValidationAnalyzer
// =========================================================================

pub struct MetaRobotsValidationAnalyzer;

impl Default for MetaRobotsValidationAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl MetaRobotsValidationAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for MetaRobotsValidationAnalyzer {
    fn name(&self) -> &str {
        "meta-robots-validation"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let robots = match &ctx.page.meta.robots {
            Some(r) => r,
            None => return findings,
        };

        let directives: Vec<String> = robots.split(',').map(|s| s.trim().to_lowercase()).collect();

        if directives.contains(&"noindex".to_string()) && directives.contains(&"index".to_string())
        {
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Seo,
                code: "MRVAL001".to_string(),
                title: "Conflicting noindex and index directives".to_string(),
                description:
                    "Meta robots contains both noindex and index. Behavior is browser-dependent."
                        .to_string(),
                url: url.clone(),
                recommendation: "Use either noindex or index, not both.".to_string(),
            });
        }

        if directives.contains(&"nofollow".to_string())
            && directives.contains(&"follow".to_string())
        {
            findings.push(Finding {
                severity: Severity::Error,
                category: IssueCategory::Seo,
                code: "MRVAL002".to_string(),
                title: "Conflicting nofollow and follow directives".to_string(),
                description:
                    "Meta robots contains both nofollow and follow. Behavior is browser-dependent."
                        .to_string(),
                url: url.clone(),
                recommendation: "Use either nofollow or follow, not both.".to_string(),
            });
        }

        if directives.contains(&"noarchive".to_string())
            || directives.contains(&"nosnippet".to_string())
        {
            findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Seo, code: "MRVAL003".to_string(), title: "Search snippet restriction detected".to_string(), description: format!("Meta robots contains directives that limit search snippet display: {robots}."), url: url.clone(), recommendation: "Remove noarchive/nosnippet unless intentionally hiding content from search results.".to_string() });
        }

        findings
    }
}

// =========================================================================
// CanonicalConsistencyAnalyzer
// =========================================================================

pub struct CanonicalConsistencyAnalyzer;

impl Default for CanonicalConsistencyAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl CanonicalConsistencyAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for CanonicalConsistencyAnalyzer {
    fn name(&self) -> &str {
        "canonical-consistency"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if let Some(canonical) = &ctx.page.meta.canonical {
            let canonical_str = canonical.as_str();
            if canonical_str.is_empty() {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Seo,
                    code: "CANCON001".to_string(),
                    title: "Empty canonical URL".to_string(),
                    description: "A canonical tag is present but has an empty href value."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Remove the canonical tag or provide a valid URL.".to_string(),
                });
            } else {
                if let Ok(page_url) = url::Url::parse(url) {
                    if canonical.scheme() != page_url.scheme() {
                        findings.push(Finding {
                            severity: Severity::Warning,
                            category: IssueCategory::Seo,
                            code: "CANCON002".to_string(),
                            title: "Canonical URL scheme mismatch".to_string(),
                            description: format!(
                                "Canonical uses {} but page uses {}.",
                                canonical.scheme(),
                                page_url.scheme()
                            ),
                            url: url.clone(),
                            recommendation: "Canonical URL should use the same scheme as the page."
                                .to_string(),
                        });
                    }
                    if canonical.host_str() != page_url.host_str() {
                        findings.push(Finding {
                            severity: Severity::Info,
                            category: IssueCategory::Seo,
                            code: "CANCON003".to_string(),
                            title: "Canonical URL points to different domain".to_string(),
                            description: format!(
                                "Canonical host {} differs from page host {}.",
                                canonical.host_str().unwrap_or(""),
                                page_url.host_str().unwrap_or("")
                            ),
                            url: url.clone(),
                            recommendation: "Verify cross-domain canonical is intentional."
                                .to_string(),
                        });
                    }
                }
            }
        }
        findings
    }
}

// =========================================================================
// HreflangNetworkValidator
// =========================================================================

pub struct HreflangNetworkValidator;

impl Default for HreflangNetworkValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl HreflangNetworkValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for HreflangNetworkValidator {
    fn name(&self) -> &str {
        "hreflang-network"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        let hreflang_tags = &ctx.page.meta.hreflang;

        if hreflang_tags.len() < 2 {
            return findings;
        }

        let langs: Vec<&str> = hreflang_tags.iter().map(|t| t.lang.as_str()).collect();
        let mut seen = std::collections::HashSet::new();
        for lang in &langs {
            if !seen.insert(*lang) {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "HREFNET001".to_string(),
                    title: "Duplicate hreflang language".to_string(),
                    description: format!("Hreflang language \"{lang}\" appears multiple times."),
                    url: url.clone(),
                    recommendation: "Each hreflang language should appear exactly once per page."
                        .to_string(),
                });
            }
        }

        let has_x_default = langs.contains(&"x-default");
        if !has_x_default && hreflang_tags.len() > 2 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "HREFNET002".to_string(),
                title: "Missing x-default in hreflang network".to_string(),
                description: "Multiple hreflang tags exist but no x-default fallback is defined."
                    .to_string(),
                url: url.clone(),
                recommendation:
                    "Add an x-default hreflang tag for users whose language doesn't match."
                        .to_string(),
            });
        }

        findings
    }
}

// =========================================================================
// ContentReadabilityScorer
// =========================================================================

pub struct ContentReadabilityScorer;

impl Default for ContentReadabilityScorer {
    fn default() -> Self {
        Self::new()
    }
}

impl ContentReadabilityScorer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for ContentReadabilityScorer {
    fn name(&self) -> &str {
        "content-readability-scorer"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;
        if ctx.page.word_count == 0 {
            return findings;
        }

        let text: String = ctx
            .page
            .headings
            .iter()
            .map(|h| h.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        if text.trim().is_empty() {
            return findings;
        }

        let words: Vec<&str> = text.split_whitespace().collect();
        let word_count = words.len();
        let sentence_count = count_sentences(&text);
        let syllable_count: usize = words.iter().map(|w| count_syllables(w)).sum();
        let fre = flesch_reading_ease(word_count, sentence_count.max(1), syllable_count);

        if fre < 30.0 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Content,
                code: "CREAD001".to_string(),
                title: "Very difficult readability".to_string(),
                description: format!(
                    "Flesch Reading Ease score is {fre:.1}/100. Content is very difficult to read."
                ),
                url: url.clone(),
                recommendation: "Simplify language, shorten sentences, and use common words."
                    .to_string(),
            });
        } else if fre < 50.0 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Content,
                code: "CREAD002".to_string(),
                title: "Fairly difficult readability".to_string(),
                description: format!("Flesch Reading Ease score is {fre:.1}/100."),
                url: url.clone(),
                recommendation: "Consider simplifying for a broader audience.".to_string(),
            });
        }

        let avg_words_per_sentence = word_count as f64 / sentence_count.max(1) as f64;
        if avg_words_per_sentence > 25.0 {
            findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Content, code: "CREAD003".to_string(), title: "Long average sentence length".to_string(), description: format!("Average sentence length is {avg_words_per_sentence:.1} words. Aim for under 20."), url: url.clone(), recommendation: "Break long sentences into shorter ones.".to_string() });
        }

        findings
    }
}

// =========================================================================
// PageImportanceAnalyzer
// =========================================================================

pub struct PageImportanceAnalyzer;

impl Default for PageImportanceAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl PageImportanceAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for PageImportanceAnalyzer {
    fn name(&self) -> &str {
        "page-importance"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let internal_links: usize = ctx.page.links.iter().filter(|l| !l.is_external).count();
        let external_links: usize = ctx.page.links.iter().filter(|l| l.is_external).count();

        if ctx.page.word_count > 0 && internal_links == 0 && external_links == 0 {
            findings.push(Finding { severity: Severity::Info, category: IssueCategory::Links, code: "PIMP001".to_string(), title: "Page has no links".to_string(), description: "The page contains no internal or external links. Pages without links may be orphaned or poorly connected.".to_string(), url: url.clone(), recommendation: "Add relevant internal and external links to improve navigation and topical authority.".to_string() });
        }

        if internal_links == 0 && ctx.page.word_count > 200 {
            findings.push(Finding { severity: Severity::Warning, category: IssueCategory::Links, code: "PIMP002".to_string(), title: "Content page missing internal links".to_string(), description: "A content-rich page has no internal links to other pages on the site.".to_string(), url: url.clone(), recommendation: "Add internal links to related content to improve site structure and user navigation.".to_string() });
        }

        if external_links > 20 {
            findings.push(Finding { severity: Severity::Info, category: IssueCategory::Links, code: "PIMP003".to_string(), title: "High number of external links".to_string(), description: format!("Page has {external_links} external links. Too many outbound links can dilute link equity."), url: url.clone(), recommendation: "Review external links and consider nofollowing non-essential ones.".to_string() });
        }

        findings
    }
}

#[cfg(test)]
mod meta_desc_length_tests {
    use super::*;
    use crate::meta::MetaTags;
    use crate::parser::{ParsedPage, StructuredData};

    fn make_page(url: &str) -> ParsedPage {
        ParsedPage {
            url: url.to_string(),
            meta: MetaTags::default(),
            headings: Vec::new(),
            links: Vec::new(),
            images: Vec::new(),
            forms: Vec::new(),
            scripts: Vec::new(),
            styles: Vec::new(),
            structured_data: Vec::new(),
            word_count: 0,
            sentence_count: 0,
            landmarks: Vec::new(),
            has_skip_link: false,
            has_main_landmark: false,
            has_nav_landmark: false,
            has_positive_tabindex: false,
            tabindex_negative_count: 0,
            aria_role_count: 0,
            aria_label_count: 0,
            has_lang_attribute: false,
            html_lang: None,
            has_aria_hidden: false,
            tables_with_headers: 0,
            tables_total: 0,
            tables_with_captions: 0,
            og_image_width: None,
            og_image_height: None,
        }
    }

    fn make_ctx<'a>(page: &'a ParsedPage, status: Option<u16>) -> AnalysisContext<'a> {
        AnalysisContext {
            page,
            body: None,
            status_code: status,
            headers: &[],
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        }
    }

    // ---- MetaDescriptionLengthAnalyzer ----

    #[test]
    fn test_meta_desc_no_description() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = MetaDescriptionLengthAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_meta_desc_empty_description() {
        let mut page = make_page("https://example.com");
        page.meta.description = Some("".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = MetaDescriptionLengthAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_meta_desc_too_short() {
        let mut page = make_page("https://example.com");
        page.meta.description = Some("Short desc".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = MetaDescriptionLengthAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "METADESC001"));
    }

    #[test]
    fn test_meta_desc_just_right() {
        let mut page = make_page("https://example.com");
        page.meta.description = Some("A".repeat(120));
        let ctx = make_ctx(&page, Some(200));
        let findings = MetaDescriptionLengthAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "METADESC001"));
        assert!(!findings.iter().any(|f| f.code == "METADESC002"));
    }

    #[test]
    fn test_meta_desc_too_long() {
        let mut page = make_page("https://example.com");
        page.meta.description = Some("A".repeat(200));
        let ctx = make_ctx(&page, Some(200));
        let findings = MetaDescriptionLengthAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "METADESC002"));
    }

    #[test]
    fn test_meta_desc_exact_boundary_70() {
        let mut page = make_page("https://example.com");
        page.meta.description = Some("A".repeat(70));
        let ctx = make_ctx(&page, Some(200));
        let findings = MetaDescriptionLengthAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "METADESC001"));
    }

    #[test]
    fn test_meta_desc_exact_boundary_160() {
        let mut page = make_page("https://example.com");
        page.meta.description = Some("A".repeat(160));
        let ctx = make_ctx(&page, Some(200));
        let findings = MetaDescriptionLengthAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "METADESC002"));
    }

    #[test]
    fn test_meta_desc_same_as_title() {
        let mut page = make_page("https://example.com");
        page.meta.title = Some("My Page Title".to_string());
        page.meta.description = Some("My Page Title".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = MetaDescriptionLengthAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "METADESC003"));
    }

    #[test]
    fn test_meta_desc_different_from_title() {
        let mut page = make_page("https://example.com");
        page.meta.title = Some("My Page Title".to_string());
        page.meta.description =
            Some("A completely different description for the page content".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = MetaDescriptionLengthAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "METADESC003"));
    }

    #[test]
    fn test_meta_desc_no_title_no_duplicate_check() {
        let mut page = make_page("https://example.com");
        page.meta.description = Some("A".repeat(120));
        let ctx = make_ctx(&page, Some(200));
        let findings = MetaDescriptionLengthAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "METADESC003"));
    }

    #[test]
    fn test_meta_desc_short_and_same_as_title() {
        let mut page = make_page("https://example.com");
        page.meta.title = Some("Hi".to_string());
        page.meta.description = Some("Hi".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = MetaDescriptionLengthAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "METADESC001"));
        assert!(findings.iter().any(|f| f.code == "METADESC003"));
    }

    // ---- TitleLengthAnalyzer ----

    #[test]
    fn test_title_no_title() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = TitleLengthAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_title_empty() {
        let mut page = make_page("https://example.com");
        page.meta.title = Some("".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = TitleLengthAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_title_too_short() {
        let mut page = make_page("https://example.com");
        page.meta.title = Some("Hi".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = TitleLengthAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "TITLE001"));
    }

    #[test]
    fn test_title_just_right() {
        let mut page = make_page("https://example.com");
        page.meta.title = Some("A Perfect Length Title for SEO".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = TitleLengthAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "TITLE001"));
        assert!(!findings.iter().any(|f| f.code == "TITLE002"));
    }

    #[test]
    fn test_title_too_long() {
        let mut page = make_page("https://example.com");
        page.meta.title = Some("A".repeat(80));
        let ctx = make_ctx(&page, Some(200));
        let findings = TitleLengthAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "TITLE002"));
    }

    #[test]
    fn test_title_boundary_30() {
        let mut page = make_page("https://example.com");
        page.meta.title = Some("A".repeat(30));
        let ctx = make_ctx(&page, Some(200));
        let findings = TitleLengthAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "TITLE001"));
    }

    #[test]
    fn test_title_boundary_60() {
        let mut page = make_page("https://example.com");
        page.meta.title = Some("A".repeat(60));
        let ctx = make_ctx(&page, Some(200));
        let findings = TitleLengthAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "TITLE002"));
    }

    #[test]
    fn test_title_pipe_separator() {
        let mut page = make_page("https://example.com");
        page.meta.title = Some("Page Title | Site Name".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = TitleLengthAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "TITLE003"));
    }

    #[test]
    fn test_title_dash_separator() {
        let mut page = make_page("https://example.com");
        page.meta.title = Some("Page Title - Site Name".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = TitleLengthAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "TITLE003"));
    }

    #[test]
    fn test_title_en_dash_separator() {
        let mut page = make_page("https://example.com");
        page.meta.title = Some("Page Title \u{2013} Site Name".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = TitleLengthAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "TITLE003"));
    }

    #[test]
    fn test_title_no_separator() {
        let mut page = make_page("https://example.com");
        page.meta.title = Some("A Perfect Title for My Website".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = TitleLengthAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "TITLE003"));
    }

    // ---- ContentThinAnalyzer ----

    #[test]
    fn test_thin_empty_page() {
        let page = make_page("https://example.com/page");
        let ctx = make_ctx(&page, Some(200));
        let findings = ContentThinAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "THIN002"));
    }

    #[test]
    fn test_thin_50_words() {
        let mut page = make_page("https://example.com/page");
        page.word_count = 50;
        let ctx = make_ctx(&page, Some(200));
        let findings = ContentThinAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "THIN002"));
    }

    #[test]
    fn test_thin_99_words_boundary() {
        let mut page = make_page("https://example.com/page");
        page.word_count = 99;
        let ctx = make_ctx(&page, Some(200));
        let findings = ContentThinAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "THIN002"));
    }

    #[test]
    fn test_thin_100_words_non_utility() {
        let mut page = make_page("https://example.com/blog/post");
        page.word_count = 100;
        let ctx = make_ctx(&page, Some(200));
        let findings = ContentThinAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "THIN001"));
        assert!(!findings.iter().any(|f| f.code == "THIN002"));
    }

    #[test]
    fn test_thin_299_words_non_utility() {
        let mut page = make_page("https://example.com/blog/post");
        page.word_count = 299;
        let ctx = make_ctx(&page, Some(200));
        let findings = ContentThinAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "THIN001"));
    }

    #[test]
    fn test_thin_300_words_non_utility() {
        let mut page = make_page("https://example.com/blog/post");
        page.word_count = 300;
        let ctx = make_ctx(&page, Some(200));
        let findings = ContentThinAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "THIN001"));
        assert!(!findings.iter().any(|f| f.code == "THIN002"));
    }

    #[test]
    fn test_thin_utility_page_100_words() {
        let mut page = make_page("https://example.com/login");
        page.word_count = 100;
        let ctx = make_ctx(&page, Some(200));
        let findings = ContentThinAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "THIN001"));
        assert!(!findings.iter().any(|f| f.code == "THIN002"));
    }

    #[test]
    fn test_thin_utility_page_under_100_words() {
        let mut page = make_page("https://example.com/login");
        page.word_count = 50;
        let ctx = make_ctx(&page, Some(200));
        let findings = ContentThinAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "THIN002"));
        assert!(!findings.iter().any(|f| f.code == "THIN001"));
    }

    #[test]
    fn test_thin_search_page() {
        let mut page = make_page("https://example.com/search?q=test");
        page.word_count = 50;
        let ctx = make_ctx(&page, Some(200));
        let findings = ContentThinAnalyzer::new().analyze(&ctx);
        // Search is a utility page, should not fire THIN001
        assert!(!findings.iter().any(|f| f.code == "THIN001"));
    }

    #[test]
    fn test_thin_cart_page() {
        let mut page = make_page("https://example.com/cart");
        page.word_count = 200;
        let ctx = make_ctx(&page, Some(200));
        let findings = ContentThinAnalyzer::new().analyze(&ctx);
        // Cart is a utility page
        assert!(!findings.iter().any(|f| f.code == "THIN001"));
    }

    // ---- JsonLdValidator ----

    #[test]
    fn test_jsonld_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        let findings = JsonLdValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_jsonld_empty_jsonld() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: None,
            r#type: None,
            data: serde_json::json!({}),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = JsonLdValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "JSONLD001"));
    }

    #[test]
    fn test_jsonld_missing_context() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: None,
            r#type: Some("Article".to_string()),
            data: serde_json::json!({"@type": "Article"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = JsonLdValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "JSONLD002"));
    }

    #[test]
    fn test_jsonld_wrong_context() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://example.com/schema".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({"@context": "https://example.com/schema"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = JsonLdValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "JSONLD002"));
    }

    #[test]
    fn test_jsonld_valid_schema_org() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Article",
                "headline": "Test"
            }),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = JsonLdValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_jsonld_schema_org_without_https() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("schema.org".to_string()),
            r#type: Some("WebSite".to_string()),
            data: serde_json::json!({"@context": "schema.org"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = JsonLdValidator::new().analyze(&ctx);
        // "schema.org" without https is accepted as valid
        assert!(!findings.iter().any(|f| f.code == "JSONLD002"));
    }

    #[test]
    fn test_jsonld_missing_type() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: None,
            data: serde_json::json!({"@context": "https://schema.org"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = JsonLdValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "JSONLD003"));
    }

    #[test]
    fn test_jsonld_valid_with_type() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product", "name": "Widget"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = JsonLdValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_jsonld_multiple_blocks_one_empty() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Article".to_string()),
                data: serde_json::json!({"@type": "Article", "headline": "Test"}),
            },
            StructuredData {
                context: None,
                r#type: None,
                data: serde_json::json!({}),
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = JsonLdValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "JSONLD001"));
    }

    #[test]
    fn test_jsonld_multiple_blocks_all_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Article".to_string()),
                data: serde_json::json!({"@type": "Article", "headline": "Test"}),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Organization".to_string()),
                data: serde_json::json!({"@type": "Organization", "name": "Org"}),
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = JsonLdValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_jsonld_empty_object_not_flagged_as_empty() {
        let mut page = make_page("https://example.com");
        // An object with properties is NOT empty
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("WebSite".to_string()),
            data: serde_json::json!({"@context": "https://schema.org", "@type": "WebSite", "name": "Test"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = JsonLdValidator::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "JSONLD001"));
    }

    #[test]
    fn test_jsonld_array_not_flagged_as_empty() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!([{"@type": "Article"}]),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = JsonLdValidator::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "JSONLD001"));
    }

    #[test]
    fn test_jsonld_wrong_context_and_missing_type() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://example.com/schema".to_string()),
            r#type: None,
            data: serde_json::json!({"@context": "https://example.com/schema"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = JsonLdValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "JSONLD002"));
        assert!(findings.iter().any(|f| f.code == "JSONLD003"));
    }

    #[test]
    fn test_jsonld_only_type_missing_context_and_type() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: None,
            r#type: None,
            data: serde_json::json!({"headline": "Test"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = JsonLdValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "JSONLD002"));
        assert!(findings.iter().any(|f| f.code == "JSONLD003"));
    }

    #[test]
    fn test_jsonld_three_blocks_mixed_validity() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Article".to_string()),
                data: serde_json::json!({"@type": "Article", "headline": "OK"}),
            },
            StructuredData {
                context: None,
                r#type: None,
                data: serde_json::json!({}),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: None,
                data: serde_json::json!({"@context": "https://schema.org"}),
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = JsonLdValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "JSONLD001"));
        assert!(findings.iter().any(|f| f.code == "JSONLD003"));
    }
}

// =========================================================================
// Additional MetaDescriptionLengthAnalyzer tests
// =========================================================================

#[cfg(test)]
mod meta_desc_extra_tests {
    use super::*;
    use crate::meta::MetaTags;
    use crate::parser::ParsedPage;

    fn make_page(url: &str) -> ParsedPage {
        ParsedPage {
            url: url.to_string(),
            meta: MetaTags::default(),
            headings: Vec::new(),
            links: Vec::new(),
            images: Vec::new(),
            forms: Vec::new(),
            scripts: Vec::new(),
            styles: Vec::new(),
            structured_data: Vec::new(),
            word_count: 0,
            sentence_count: 0,
            landmarks: Vec::new(),
            has_skip_link: false,
            has_main_landmark: false,
            has_nav_landmark: false,
            has_positive_tabindex: false,
            tabindex_negative_count: 0,
            aria_role_count: 0,
            aria_label_count: 0,
            has_lang_attribute: false,
            html_lang: None,
            has_aria_hidden: false,
            tables_with_headers: 0,
            tables_total: 0,
            tables_with_captions: 0,
            og_image_width: None,
            og_image_height: None,
        }
    }

    fn make_ctx<'a>(page: &'a ParsedPage, status: Option<u16>) -> AnalysisContext<'a> {
        AnalysisContext {
            page,
            body: None,
            status_code: status,
            headers: &[],
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        }
    }

    #[test]
    fn test_meta_desc_69_chars_too_short() {
        let mut page = make_page("https://example.com");
        page.meta.description = Some("A".repeat(69));
        let ctx = make_ctx(&page, Some(200));
        let findings = MetaDescriptionLengthAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "METADESC001"));
    }

    #[test]
    fn test_meta_desc_161_chars_too_long() {
        let mut page = make_page("https://example.com");
        page.meta.description = Some("A".repeat(161));
        let ctx = make_ctx(&page, Some(200));
        let findings = MetaDescriptionLengthAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "METADESC002"));
    }

    #[test]
    fn test_meta_desc_both_short_and_long_not_possible() {
        // A string cannot be both <70 and >160
        let mut page = make_page("https://example.com");
        page.meta.description = Some("A".repeat(120));
        let ctx = make_ctx(&page, Some(200));
        let findings = MetaDescriptionLengthAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "METADESC001"));
        assert!(!findings.iter().any(|f| f.code == "METADESC002"));
    }

    #[test]
    fn test_meta_desc_whitespace_only_treated_as_empty() {
        let mut page = make_page("https://example.com");
        page.meta.description = Some("   ".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = MetaDescriptionLengthAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_meta_desc_unicode_chars_counted() {
        let mut page = make_page("https://example.com");
        // Each emoji is 1 char, 65 emojis = 65 chars (too short)
        page.meta.description = Some("\u{1F600}".repeat(65));
        let ctx = make_ctx(&page, Some(200));
        let findings = MetaDescriptionLengthAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "METADESC001"));
    }
}

// =========================================================================
// Additional TitleLengthAnalyzer tests
// =========================================================================

#[cfg(test)]
mod title_extra_tests {
    use super::*;
    use crate::meta::MetaTags;
    use crate::parser::ParsedPage;

    fn make_page(url: &str) -> ParsedPage {
        ParsedPage {
            url: url.to_string(),
            meta: MetaTags::default(),
            headings: Vec::new(),
            links: Vec::new(),
            images: Vec::new(),
            forms: Vec::new(),
            scripts: Vec::new(),
            styles: Vec::new(),
            structured_data: Vec::new(),
            word_count: 0,
            sentence_count: 0,
            landmarks: Vec::new(),
            has_skip_link: false,
            has_main_landmark: false,
            has_nav_landmark: false,
            has_positive_tabindex: false,
            tabindex_negative_count: 0,
            aria_role_count: 0,
            aria_label_count: 0,
            has_lang_attribute: false,
            html_lang: None,
            has_aria_hidden: false,
            tables_with_headers: 0,
            tables_total: 0,
            tables_with_captions: 0,
            og_image_width: None,
            og_image_height: None,
        }
    }

    fn make_ctx<'a>(page: &'a ParsedPage, status: Option<u16>) -> AnalysisContext<'a> {
        AnalysisContext {
            page,
            body: None,
            status_code: status,
            headers: &[],
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        }
    }

    #[test]
    fn test_title_29_chars_too_short() {
        let mut page = make_page("https://example.com");
        page.meta.title = Some("A".repeat(29));
        let ctx = make_ctx(&page, Some(200));
        let findings = TitleLengthAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "TITLE001"));
    }

    #[test]
    fn test_title_61_chars_too_long() {
        let mut page = make_page("https://example.com");
        page.meta.title = Some("A".repeat(61));
        let ctx = make_ctx(&page, Some(200));
        let findings = TitleLengthAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "TITLE002"));
    }

    #[test]
    fn test_title_multiple_separators() {
        let mut page = make_page("https://example.com");
        page.meta.title = Some("Blog | My Site - Post Title".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = TitleLengthAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "TITLE003"));
    }

    #[test]
    fn test_title_whitespace_only_treated_as_empty() {
        let mut page = make_page("https://example.com");
        page.meta.title = Some("   ".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = TitleLengthAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_title_short_and_with_separator() {
        let mut page = make_page("https://example.com");
        page.meta.title = Some("Hi | Site".to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = TitleLengthAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "TITLE001"));
        assert!(findings.iter().any(|f| f.code == "TITLE003"));
    }
}

// =========================================================================
// Additional ContentThinAnalyzer tests
// =========================================================================

#[cfg(test)]
mod thin_extra_tests {
    use super::*;
    use crate::meta::MetaTags;
    use crate::parser::ParsedPage;

    fn make_page(url: &str) -> ParsedPage {
        ParsedPage {
            url: url.to_string(),
            meta: MetaTags::default(),
            headings: Vec::new(),
            links: Vec::new(),
            images: Vec::new(),
            forms: Vec::new(),
            scripts: Vec::new(),
            styles: Vec::new(),
            structured_data: Vec::new(),
            word_count: 0,
            sentence_count: 0,
            landmarks: Vec::new(),
            has_skip_link: false,
            has_main_landmark: false,
            has_nav_landmark: false,
            has_positive_tabindex: false,
            tabindex_negative_count: 0,
            aria_role_count: 0,
            aria_label_count: 0,
            has_lang_attribute: false,
            html_lang: None,
            has_aria_hidden: false,
            tables_with_headers: 0,
            tables_total: 0,
            tables_with_captions: 0,
            og_image_width: None,
            og_image_height: None,
        }
    }

    fn make_ctx<'a>(page: &'a ParsedPage, status: Option<u16>) -> AnalysisContext<'a> {
        AnalysisContext {
            page,
            body: None,
            status_code: status,
            headers: &[],
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        }
    }

    #[test]
    fn test_thin_100_words_exactly_non_utility() {
        let mut page = make_page("https://example.com/blog/post");
        page.word_count = 100;
        let ctx = make_ctx(&page, Some(200));
        let findings = ContentThinAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "THIN001"));
        assert!(!findings.iter().any(|f| f.code == "THIN002"));
    }

    #[test]
    fn test_thin_500_words_no_issue() {
        let mut page = make_page("https://example.com/blog/post");
        page.word_count = 500;
        let ctx = make_ctx(&page, Some(200));
        let findings = ContentThinAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "THIN001"));
        assert!(!findings.iter().any(|f| f.code == "THIN002"));
    }

    #[test]
    fn test_thin_admin_page() {
        let mut page = make_page("https://example.com/admin/dashboard");
        page.word_count = 200;
        let ctx = make_ctx(&page, Some(200));
        let findings = ContentThinAnalyzer::new().analyze(&ctx);
        // Admin is a utility page
        assert!(!findings.iter().any(|f| f.code == "THIN001"));
    }

    #[test]
    fn test_thin_contact_page() {
        let mut page = make_page("https://example.com/contact");
        page.word_count = 150;
        let ctx = make_ctx(&page, Some(200));
        let findings = ContentThinAnalyzer::new().analyze(&ctx);
        // Contact is a utility page
        assert!(!findings.iter().any(|f| f.code == "THIN001"));
    }

    #[test]
    fn test_thin_regular_page_200_words() {
        let mut page = make_page("https://example.com/products/widget");
        page.word_count = 200;
        let ctx = make_ctx(&page, Some(200));
        let findings = ContentThinAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "THIN001"));
    }
}

// =========================================================================
// ApartmentSchemaValidator
// =========================================================================

// Validates Apartment/Residence structured data for completeness.
// =========================================================================
// CarSchemaValidator
// =========================================================================

// Validates Car structured data for completeness.
// =========================================================================
// MusicAlbumSchemaValidator
// =========================================================================

// Validates MusicAlbum structured data for completeness.
// =========================================================================
// TVSeriesSchemaValidator
// =========================================================================

// Validates TVSeries structured data for completeness.
// =========================================================================
// MovieSchemaValidator
// =========================================================================

// Validates Movie structured data for completeness.
// =========================================================================
// GovernmentServiceSchemaValidator
// =========================================================================

// Validates GovernmentService structured data for completeness.
// =========================================================================
// HealthPlanSchemaValidator
// =========================================================================

// Validates HealthPlan structured data for completeness.
// =========================================================================
// InvoiceSchemaValidator
// =========================================================================

// Validates Invoice structured data for completeness.
// =========================================================================
// PermitSchemaValidator
// =========================================================================

// Validates Permit structured data for completeness.
// =========================================================================
// PlanSchemaValidator
// =========================================================================

// Validates Plan structured data for completeness.
// =========================================================================
// ProductModelSchemaValidator
// =========================================================================

// Validates ProductModel structured data for completeness.
// =========================================================================
// ResearchProjectSchemaValidator
// =========================================================================

// Validates ResearchProject structured data for completeness.
// =========================================================================
// ScheduleSchemaValidator
// =========================================================================

// Validates Schedule structured data for completeness.
// =========================================================================
// TripSchemaValidator
// =========================================================================

// Validates Trip structured data for completeness.
// =========================================================================
// WorkersUnionSchemaValidator
// =========================================================================

// Validates WorkersUnion structured data for completeness.
// =========================================================================
// WebAPISchemaValidator
// =========================================================================

// Validates WebAPI structured data for completeness.
// =========================================================================
// WearableSchemaValidator
// =========================================================================

// Validates Wearable structured data for completeness.
// =========================================================================
// WebPageElementSchemaValidator
// =========================================================================

// Validates WebPageElement structured data for completeness.
// =========================================================================
// WebSiteSchemaValidator
// =========================================================================

// Validates WebSite structured data for completeness.
// =========================================================================
// WorkerSchemaValidator
// =========================================================================

// Validates Worker structured data for completeness.
// =========================================================================
// Tests for new schema validators
// =========================================================================

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod new_validator_tests {
    use super::*;
    use crate::meta::MetaTags;
    use crate::parser::ParsedPage;
    use crate::parser::StructuredData;

    fn make_page(url: &str) -> ParsedPage {
        ParsedPage {
            url: url.to_string(),
            meta: MetaTags::default(),
            headings: Vec::new(),
            links: Vec::new(),
            images: Vec::new(),
            forms: Vec::new(),
            scripts: Vec::new(),
            styles: Vec::new(),
            structured_data: Vec::new(),
            word_count: 0,
            sentence_count: 0,
            landmarks: Vec::new(),
            has_skip_link: false,
            has_main_landmark: false,
            has_nav_landmark: false,
            has_positive_tabindex: false,
            tabindex_negative_count: 0,
            aria_role_count: 0,
            aria_label_count: 0,
            has_lang_attribute: false,
            html_lang: None,
            has_aria_hidden: false,
            tables_with_headers: 0,
            tables_total: 0,
            tables_with_captions: 0,
            og_image_width: None,
            og_image_height: None,
        }
    }

    fn make_ctx<'a>(page: &'a ParsedPage) -> AnalysisContext<'a> {
        AnalysisContext {
            page,
            body: None,
            status_code: Some(200),
            headers: &[],
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        }
    }

    // ---- ArticleSchemaValidator ----

    // ---- OrganizationSchemaValidator ----

    // ---- PersonSchemaValidator ----

    // ---- JobPostingSchemaValidator ----

    // ---- CourseSchemaValidator ----

    // ---- RecipeSchemaValidator ----

    // ---- ContentFreshnessScorer ----

    #[test]
    fn test_fresh001_blog_no_date() {
        let page = make_page("https://example.com/blog/my-post");
        let ctx = make_ctx(&page);
        let findings = ContentFreshnessScorer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "FRESH001"));
    }

    #[test]
    fn test_fresh001_blog_with_date() {
        let mut page = make_page("https://example.com/blog/my-post");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Article",
                "datePublished": "2024-01-01"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ContentFreshnessScorer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "FRESH001"));
    }

    #[test]
    fn test_fresh001_non_time_sensitive() {
        let page = make_page("https://example.com/about");
        let ctx = make_ctx(&page);
        let findings = ContentFreshnessScorer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "FRESH001"));
    }

    #[test]
    fn test_fresh002_year_mismatch() {
        let mut page = make_page("https://example.com/article");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Article",
                "datePublished": "2023-01-01"
            }),
        }];
        let body = "This article was published on January 15, 2024 and covers the topic.";
        let ctx = AnalysisContext {
            page: &page,
            body: Some(body),
            status_code: Some(200),
            headers: &[],
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let findings = ContentFreshnessScorer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "FRESH002"));
    }

    #[test]
    fn test_fresh002_year_match() {
        let mut page = make_page("https://example.com/article");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Article",
                "datePublished": "2024-01-01"
            }),
        }];
        let body = "This article was published on January 15, 2024 and covers the topic.";
        let ctx = AnalysisContext {
            page: &page,
            body: Some(body),
            status_code: Some(200),
            headers: &[],
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let findings = ContentFreshnessScorer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "FRESH002"));
    }

    #[test]
    fn test_fresh002_no_body() {
        let mut page = make_page("https://example.com/article");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Article",
                "datePublished": "2023-01-01"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ContentFreshnessScorer::new().analyze(&ctx);
        // No body = can't detect year mismatch
        assert!(!findings.iter().any(|f| f.code == "FRESH002"));
    }

    #[test]
    fn test_fresh001_news_url() {
        let page = make_page("https://example.com/news/breaking");
        let ctx = make_ctx(&page);
        let findings = ContentFreshnessScorer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "FRESH001"));
    }

    #[test]
    fn test_fresh001_article_url() {
        let page = make_page("https://example.com/article/something");
        let ctx = make_ctx(&page);
        let findings = ContentFreshnessScorer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "FRESH001"));
    }

    #[test]
    fn test_fresh001_post_url() {
        let page = make_page("https://example.com/post/my-post");
        let ctx = make_ctx(&page);
        let findings = ContentFreshnessScorer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "FRESH001"));
    }

    #[test]
    fn test_fresh001_update_url() {
        let page = make_page("https://example.com/update/latest");
        let ctx = make_ctx(&page);
        let findings = ContentFreshnessScorer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "FRESH001"));
    }

    // ---- BreadcrumbListDepthAnalyzer ----

    #[test]
    fn test_bdepth001_consistent_depth() {
        let mut page = make_page("https://example.com/a/b/c");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("BreadcrumbList".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "BreadcrumbList",
                "itemListElement": [
                    {"@type": "ListItem", "position": 1, "name": "Home"},
                    {"@type": "ListItem", "position": 2, "name": "A"},
                    {"@type": "ListItem", "position": 3, "name": "B"},
                    {"@type": "ListItem", "position": 4, "name": "C"}
                ]
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = BreadcrumbListDepthAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "BDEPTH001"));
    }

    #[test]
    fn test_bdepth001_inconsistent_depth() {
        let mut page = make_page("https://example.com/a/b/c");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("BreadcrumbList".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "BreadcrumbList",
                "itemListElement": [
                    {"@type": "ListItem", "position": 1, "name": "Home"}
                ]
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = BreadcrumbListDepthAnalyzer::new().analyze(&ctx);
        // url_depth=3, breadcrumb_depth=1, diff=2 > 1, so finding
        assert!(findings.iter().any(|f| f.code == "BDEPTH001"));
    }

    #[test]
    fn test_bdepth001_shallow_url_no_issue() {
        let mut page = make_page("https://example.com/a");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("BreadcrumbList".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "BreadcrumbList",
                "itemListElement": [
                    {"@type": "ListItem", "position": 1, "name": "Home"}
                ]
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = BreadcrumbListDepthAnalyzer::new().analyze(&ctx);
        // url_depth=1 which is <= 2, so no check
        assert!(!findings.iter().any(|f| f.code == "BDEPTH001"));
    }

    #[test]
    fn test_bdepth001_no_breadcrumb() {
        let page = make_page("https://example.com/a/b/c");
        let ctx = make_ctx(&page);
        let findings = BreadcrumbListDepthAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_bdepth001_deep_url_many_items() {
        let mut page = make_page("https://example.com/a/b/c/d/e");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("BreadcrumbList".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "BreadcrumbList",
                "itemListElement": [
                    {"@type": "ListItem", "position": 1, "name": "Home"},
                    {"@type": "ListItem", "position": 2, "name": "A"},
                    {"@type": "ListItem", "position": 3, "name": "B"},
                    {"@type": "ListItem", "position": 4, "name": "C"},
                    {"@type": "ListItem", "position": 5, "name": "D"},
                    {"@type": "ListItem", "position": 6, "name": "E"},
                    {"@type": "ListItem", "position": 7, "name": "F"}
                ]
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = BreadcrumbListDepthAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "BDEPTH001"));
    }

    #[test]
    fn test_bdepth001_url_depth_3_breadcrumb_2() {
        let mut page = make_page("https://example.com/a/b/c");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("BreadcrumbList".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "BreadcrumbList",
                "itemListElement": [
                    {"@type": "ListItem", "position": 1, "name": "Home"},
                    {"@type": "ListItem", "position": 2, "name": "A"}
                ]
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = BreadcrumbListDepthAnalyzer::new().analyze(&ctx);
        // url_depth=3, breadcrumb_depth=2, diff=1 which is <= 1, so no issue
        assert!(!findings.iter().any(|f| f.code == "BDEPTH001"));
    }

    // =========================================================================
    // WebPageSchemaValidator tests

    // =========================================================================
    // ServiceSchemaValidator tests

    // =========================================================================
    // ItemListSchemaValidator tests

    // =========================================================================
    // OfferSchemaValidator tests

    // =========================================================================
    // AggregateOfferSchemaValidator tests

    // =========================================================================
    // BrandSchemaValidator tests

    // =========================================================================
    // OccupationSchemaValidator tests

    // =========================================================================
    // QuestSchemaValidator tests

    // =========================================================================
    // ActionSchemaValidator tests

    // =========================================================================
    // PlaybookSchemaValidator tests

    // =========================================================================
    // ApartmentSchemaValidator tests

    // =========================================================================
    // CarSchemaValidator tests

    // =========================================================================
    // MusicAlbumSchemaValidator tests

    // =========================================================================
    // TVSeriesSchemaValidator tests

    // =========================================================================
    // MovieSchemaValidator tests

    // =========================================================================
    // GovernmentServiceSchemaValidator tests

    // =========================================================================
    // HealthPlanSchemaValidator tests

    // =========================================================================
    // InvoiceSchemaValidator tests

    // =========================================================================
    // PermitSchemaValidator tests

    // =========================================================================
    // PlanSchemaValidator tests

    // =========================================================================
    // ProductModelSchemaValidator tests

    // =========================================================================
    // ResearchProjectSchemaValidator tests

    // =========================================================================
    // ScheduleSchemaValidator tests

    // =========================================================================
    // TripSchemaValidator tests

    // =========================================================================
    // WorkersUnionSchemaValidator tests

    // =========================================================================
    // WebAPISchemaValidator tests

    // =========================================================================
    // WearableSchemaValidator tests

    // =========================================================================
    // WebPageElementSchemaValidator tests

    // =========================================================================
    // WebSiteSchemaValidator tests

    // =========================================================================
    // WorkerSchemaValidator tests

    // =========================================================================
    // LocalBusinessHoursValidator tests

    // =========================================================================
    // ProductReviewValidator tests

    // =========================================================================
    // EventLocationValidator tests

    // =========================================================================
    // OrganizationLogoValidator tests

    // =========================================================================
    // PersonJobTitleValidator tests

    // =========================================================================
    // RecipeNutritionValidator tests

    // =========================================================================
    // CourseProviderValidator tests

    // =========================================================================
    // JobPostingSalaryValidator tests
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod new_content_analyzer_tests {
    use super::*;
    use crate::meta::MetaTags;
    use crate::parser::{ParsedPage, StructuredData};

    fn make_page(url: &str) -> ParsedPage {
        ParsedPage {
            url: url.to_string(),
            meta: MetaTags::default(),
            headings: Vec::new(),
            links: Vec::new(),
            images: Vec::new(),
            forms: Vec::new(),
            scripts: Vec::new(),
            styles: Vec::new(),
            structured_data: Vec::new(),
            word_count: 0,
            sentence_count: 0,
            landmarks: Vec::new(),
            has_skip_link: false,
            has_main_landmark: false,
            has_nav_landmark: false,
            has_positive_tabindex: false,
            tabindex_negative_count: 0,
            aria_role_count: 0,
            aria_label_count: 0,
            has_lang_attribute: false,
            html_lang: None,
            has_aria_hidden: false,
            tables_with_headers: 0,
            tables_total: 0,
            tables_with_captions: 0,
            og_image_width: None,
            og_image_height: None,
        }
    }

    fn make_ctx<'a>(page: &'a ParsedPage) -> AnalysisContext<'a> {
        AnalysisContext {
            page,
            body: None,
            status_code: Some(200),
            headers: &[],
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        }
    }

    // OpenGraphVideoUrlValidator tests

    #[test]
    fn test_og_video_no_video() {
        let page = make_page("https://example.com");
        assert!(OpenGraphVideoUrlValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_og_video_valid_url() {
        let mut page = make_page("https://example.com");
        page.meta.og.insert(
            "video".to_string(),
            "https://example.com/video.mp4".to_string(),
        );
        assert!(OpenGraphVideoUrlValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_og_video_invalid_url() {
        let mut page = make_page("https://example.com");
        page.meta
            .og
            .insert("video".to_string(), "not-a-url".to_string());
        assert!(OpenGraphVideoUrlValidator::new()
            .analyze(&make_ctx(&page))
            .iter()
            .any(|f| f.code == "OGVIDURL001"));
    }

    #[test]
    fn test_og_video_empty() {
        let mut page = make_page("https://example.com");
        page.meta.og.insert("video".to_string(), "".to_string());
        assert!(OpenGraphVideoUrlValidator::new()
            .analyze(&make_ctx(&page))
            .iter()
            .any(|f| f.code == "OGVIDURL001"));
    }

    #[test]
    fn test_og_video_ftp_url() {
        let mut page = make_page("https://example.com");
        page.meta.og.insert(
            "video".to_string(),
            "ftp://example.com/video.mp4".to_string(),
        );
        assert!(OpenGraphVideoUrlValidator::new()
            .analyze(&make_ctx(&page))
            .iter()
            .any(|f| f.code == "OGVIDURL001"));
    }

    #[test]
    fn test_og_video_name() {
        assert_eq!(
            OpenGraphVideoUrlValidator::new().name(),
            "og-video-url-validator"
        );
    }

    #[test]
    fn test_og_video_default() {
        let _ = OpenGraphVideoUrlValidator::default();
    }

    // TwitterPlayerStreamValidator tests

    #[test]
    fn test_twitter_stream_no_stream() {
        let page = make_page("https://example.com");
        assert!(TwitterPlayerStreamValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_twitter_stream_valid_url() {
        let mut page = make_page("https://example.com");
        page.meta.twitter.player_stream = Some("https://example.com/stream.mp4".to_string());
        assert!(TwitterPlayerStreamValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_twitter_stream_invalid_url() {
        let mut page = make_page("https://example.com");
        page.meta.twitter.player_stream = Some("not-a-url".to_string());
        assert!(TwitterPlayerStreamValidator::new()
            .analyze(&make_ctx(&page))
            .iter()
            .any(|f| f.code == "TWSTREAM001"));
    }

    #[test]
    fn test_twitter_stream_empty() {
        let mut page = make_page("https://example.com");
        page.meta.twitter.player_stream = Some("".to_string());
        assert!(TwitterPlayerStreamValidator::new()
            .analyze(&make_ctx(&page))
            .iter()
            .any(|f| f.code == "TWSTREAM001"));
    }

    #[test]
    fn test_twitter_stream_ftp() {
        let mut page = make_page("https://example.com");
        page.meta.twitter.player_stream = Some("ftp://example.com/stream.mp4".to_string());
        assert!(TwitterPlayerStreamValidator::new()
            .analyze(&make_ctx(&page))
            .iter()
            .any(|f| f.code == "TWSTREAM001"));
    }

    #[test]
    fn test_twitter_stream_name() {
        assert_eq!(
            TwitterPlayerStreamValidator::new().name(),
            "twitter-player-stream-validator"
        );
    }

    #[test]
    fn test_twitter_stream_default() {
        let _ = TwitterPlayerStreamValidator::default();
    }

    // SchemaNestingDepthValidator tests

    #[test]
    fn test_nesting_no_schemas() {
        let page = make_page("https://example.com");
        assert!(SchemaNestingDepthValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_nesting_shallow() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({"@type": "Article", "name": "Test"}),
        }];
        assert!(SchemaNestingDepthValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_nesting_deep() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({"@type": "Article", "a": {"b": {"c": {"d": "deep"}}}}),
        }];
        assert!(SchemaNestingDepthValidator::new()
            .analyze(&make_ctx(&page))
            .iter()
            .any(|f| f.code == "SCNEST001"));
    }

    #[test]
    fn test_nesting_exactly_3() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({"@type": "Article", "a": {"b": {"c": "ok"}}}),
        }];
        assert!(SchemaNestingDepthValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_nesting_name() {
        assert_eq!(
            SchemaNestingDepthValidator::new().name(),
            "schema-nesting-depth"
        );
    }

    #[test]
    fn test_nesting_default() {
        let _ = SchemaNestingDepthValidator::default();
    }

    // SchemaIdReferenceValidator tests

    #[test]
    fn test_id_ref_no_schemas() {
        let page = make_page("https://example.com");
        assert!(SchemaIdReferenceValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_id_ref_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({"@type": "Article", "@id": "#article1"}),
        }];
        assert!(SchemaIdReferenceValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_id_ref_invalid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({"@type": "Article", "author": {"@id": "#missing"}}),
        }];
        assert!(SchemaIdReferenceValidator::new()
            .analyze(&make_ctx(&page))
            .iter()
            .any(|f| f.code == "SCREF001"));
    }

    #[test]
    fn test_id_ref_with_target() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Organization".to_string()),
                data: serde_json::json!({"@type": "Organization", "@id": "#org"}),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Article".to_string()),
                data: serde_json::json!({"@type": "Article", "author": {"@id": "#org"}}),
            },
        ];
        assert!(SchemaIdReferenceValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_id_ref_name() {
        assert_eq!(
            SchemaIdReferenceValidator::new().name(),
            "schema-id-reference"
        );
    }

    #[test]
    fn test_id_ref_default() {
        let _ = SchemaIdReferenceValidator::default();
    }

    // BreadcrumbActivePageValidator tests

    #[test]
    fn test_breadcrumb_active_no_breadcrumb() {
        let page = make_page("https://example.com/page");
        assert!(BreadcrumbActivePageValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_breadcrumb_active_match() {
        let mut page = make_page("https://example.com/page");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("BreadcrumbList".to_string()),
            data: serde_json::json!({"@type": "BreadcrumbList", "itemListElement": [
                {"@type": "ListItem", "position": 1, "item": {"@id": "https://example.com"}},
                {"@type": "ListItem", "position": 2, "item": {"@id": "https://example.com/page"}}
            ]}),
        }];
        assert!(BreadcrumbActivePageValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_breadcrumb_active_mismatch() {
        let mut page = make_page("https://example.com/page");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("BreadcrumbList".to_string()),
            data: serde_json::json!({"@type": "BreadcrumbList", "itemListElement": [
                {"@type": "ListItem", "position": 1, "item": {"@id": "https://example.com"}},
                {"@type": "ListItem", "position": 2, "item": {"@id": "https://example.com/other"}}
            ]}),
        }];
        assert!(BreadcrumbActivePageValidator::new()
            .analyze(&make_ctx(&page))
            .iter()
            .any(|f| f.code == "BREADACT001"));
    }

    #[test]
    fn test_breadcrumb_active_empty_list() {
        let mut page = make_page("https://example.com/page");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("BreadcrumbList".to_string()),
            data: serde_json::json!({"@type": "BreadcrumbList", "itemListElement": []}),
        }];
        assert!(BreadcrumbActivePageValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_breadcrumb_active_name() {
        assert_eq!(
            BreadcrumbActivePageValidator::new().name(),
            "breadcrumb-active-page"
        );
    }

    #[test]
    fn test_breadcrumb_active_default() {
        let _ = BreadcrumbActivePageValidator::default();
    }

    // ContentLanguageValidator tests

    #[test]
    fn test_content_lang_no_html_lang() {
        let page = make_page("https://example.com");
        assert!(ContentLanguageValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_content_lang_match() {
        let mut page = make_page("https://example.com");
        page.html_lang = Some("en".to_string());
        page.meta.language = Some("en".to_string());
        assert!(ContentLanguageValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_content_lang_mismatch() {
        let mut page = make_page("https://example.com");
        page.html_lang = Some("en".to_string());
        page.meta.language = Some("fr".to_string());
        assert!(ContentLanguageValidator::new()
            .analyze(&make_ctx(&page))
            .iter()
            .any(|f| f.code == "CLANG001"));
    }

    #[test]
    fn test_content_lang_no_meta_lang() {
        let mut page = make_page("https://example.com");
        page.html_lang = Some("en".to_string());
        assert!(ContentLanguageValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_content_lang_hreflang_match() {
        let mut page = make_page("https://example.com");
        page.html_lang = Some("en".to_string());
        page.meta.hreflang = vec![crate::meta::HreflangTag {
            lang: "en".to_string(),
            url: url::Url::parse("https://example.com/en").unwrap(),
        }];
        assert!(ContentLanguageValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_content_lang_hreflang_no_match() {
        let mut page = make_page("https://example.com");
        page.html_lang = Some("en".to_string());
        page.meta.hreflang = vec![crate::meta::HreflangTag {
            lang: "fr".to_string(),
            url: url::Url::parse("https://example.com/fr").unwrap(),
        }];
        assert!(ContentLanguageValidator::new()
            .analyze(&make_ctx(&page))
            .iter()
            .any(|f| f.code == "CLANG001"));
    }

    #[test]
    fn test_content_lang_name() {
        assert_eq!(ContentLanguageValidator::new().name(), "content-language");
    }

    #[test]
    fn test_content_lang_default() {
        let _ = ContentLanguageValidator::default();
    }

    // MetaDescriptionUniquenessAnalyzer tests

    #[test]
    fn test_meta_desc_uniq_no_description() {
        let page = make_page("https://example.com");
        assert!(MetaDescriptionUniquenessAnalyzer::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_meta_desc_uniq_very_short() {
        let mut page = make_page("https://example.com");
        page.meta.description = Some("Hi".to_string());
        assert!(MetaDescriptionUniquenessAnalyzer::new()
            .analyze(&make_ctx(&page))
            .iter()
            .any(|f| f.code == "METADESC-UNI001"));
    }

    #[test]
    fn test_meta_desc_uniq_generic_text() {
        let mut page = make_page("https://example.com");
        page.meta.description = Some(
            "Welcome to our website, click here to learn more about our services.".to_string(),
        );
        assert!(MetaDescriptionUniquenessAnalyzer::new()
            .analyze(&make_ctx(&page))
            .iter()
            .any(|f| f.code == "METADESC-UNI002"));
    }

    #[test]
    fn test_meta_desc_uniq_good_description() {
        let mut page = make_page("https://example.com");
        page.meta.description = Some("A comprehensive guide to Rust programming language covering ownership, borrowing, and lifetime concepts.".to_string());
        assert!(MetaDescriptionUniquenessAnalyzer::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    // ContentFreshnessDateAnalyzer tests

    #[test]
    fn test_fresh_date_no_date_on_blog() {
        let page = make_page("https://example.com/blog/my-post");
        assert!(ContentFreshnessDateAnalyzer::new()
            .analyze(&make_ctx(&page))
            .iter()
            .any(|f| f.code == "FRESH-DATE001"));
    }

    #[test]
    fn test_fresh_date_with_date_on_blog() {
        let mut page = make_page("https://example.com/blog/my-post");
        page.structured_data = vec![crate::parser::StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({"@type": "Article", "datePublished": "2024-01-01"}),
        }];
        assert!(ContentFreshnessDateAnalyzer::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_fresh_date_non_blog_ignored() {
        let page = make_page("https://example.com/about");
        assert!(ContentFreshnessDateAnalyzer::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    // StructuredDataNestingValidator tests

    #[test]
    fn test_sd_nest_product_missing_offers() {
        let mut page = make_page("https://example.com/product");
        page.structured_data = vec![crate::parser::StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product", "name": "Widget"}),
        }];
        assert!(StructuredDataNestingValidator::new()
            .analyze(&make_ctx(&page))
            .iter()
            .any(|f| f.code == "SDNEST001"));
    }

    #[test]
    fn test_sd_nest_product_offer_missing_price() {
        let mut page = make_page("https://example.com/product");
        page.structured_data = vec![crate::parser::StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product", "name": "Widget", "offers": {"@type": "Offer"}}),
        }];
        assert!(StructuredDataNestingValidator::new()
            .analyze(&make_ctx(&page))
            .iter()
            .any(|f| f.code == "SDNEST002"));
    }

    #[test]
    fn test_sd_nest_product_valid() {
        let mut page = make_page("https://example.com/product");
        page.structured_data = vec![crate::parser::StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product", "name": "Widget", "offers": {"@type": "Offer", "price": "9.99", "availability": "https://schema.org/InStock"}}),
        }];
        assert!(StructuredDataNestingValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_sd_nest_non_product_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![crate::parser::StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({"@type": "Article", "headline": "Test"}),
        }];
        assert!(StructuredDataNestingValidator::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    // LocalBusinessNapAnalyzerUtil tests

    #[test]
    fn test_nap_util_missing_phone() {
        let mut page = make_page("https://example.com/business");
        page.structured_data = vec![crate::parser::StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("LocalBusiness".to_string()),
            data: serde_json::json!({"@type": "LocalBusiness", "name": "Acme Store"}),
        }];
        assert!(LocalBusinessNapAnalyzerUtil::new()
            .analyze(&make_ctx(&page))
            .iter()
            .any(|f| f.code == "NAP-UTIL001"));
    }

    #[test]
    fn test_nap_util_valid() {
        let mut page = make_page("https://example.com/business");
        page.structured_data = vec![crate::parser::StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("LocalBusiness".to_string()),
            data: serde_json::json!({"@type": "LocalBusiness", "name": "Acme Store", "telephone": "+1-555-555-5555", "address": {"@type": "PostalAddress", "streetAddress": "123 Main St"}}),
        }];
        assert!(LocalBusinessNapAnalyzerUtil::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    #[test]
    fn test_nap_util_non_local_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![crate::parser::StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product", "name": "Widget"}),
        }];
        assert!(LocalBusinessNapAnalyzerUtil::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    // EventLocationValidatorV2 tests

    #[test]
    fn test_event_loc_v2_missing_location() {
        let mut page = make_page("https://example.com/event");
        page.structured_data = vec![crate::parser::StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Event".to_string()),
            data: serde_json::json!({"@type": "Event", "name": "Conference", "startDate": "2024-06-15"}),
        }];
        assert!(EventLocationValidatorV2::new()
            .analyze(&make_ctx(&page))
            .iter()
            .any(|f| f.code == "ELOC-V2001"));
    }

    #[test]
    fn test_event_loc_v2_valid() {
        let mut page = make_page("https://example.com/event");
        page.structured_data = vec![crate::parser::StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Event".to_string()),
            data: serde_json::json!({"@type": "Event", "name": "Conference", "startDate": "2024-06-15", "location": {"@type": "Place", "name": "Convention Center"}}),
        }];
        assert!(EventLocationValidatorV2::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    // OrganizationLogoValidatorV2 tests

    #[test]
    fn test_org_logo_v2_missing() {
        let mut page = make_page("https://example.com/org");
        page.structured_data = vec![crate::parser::StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Organization".to_string()),
            data: serde_json::json!({"@type": "Organization", "name": "Acme Corp"}),
        }];
        assert!(OrganizationLogoValidatorV2::new()
            .analyze(&make_ctx(&page))
            .iter()
            .any(|f| f.code == "OLOGO-V2001"));
    }

    #[test]
    fn test_org_logo_v2_valid() {
        let mut page = make_page("https://example.com/org");
        page.structured_data = vec![crate::parser::StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Organization".to_string()),
            data: serde_json::json!({"@type": "Organization", "name": "Acme Corp", "logo": "https://example.com/logo.png"}),
        }];
        assert!(OrganizationLogoValidatorV2::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    // PersonJobTitleValidatorV2 tests

    #[test]
    fn test_person_job_v2_missing() {
        let mut page = make_page("https://example.com/person");
        page.structured_data = vec![crate::parser::StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Person".to_string()),
            data: serde_json::json!({"@type": "Person", "name": "John Doe"}),
        }];
        let findings = PersonJobTitleValidatorV2::new().analyze(&make_ctx(&page));
        assert!(findings.iter().any(|f| f.code == "PJOB-V2001"));
    }

    #[test]
    fn test_person_job_v2_valid() {
        let mut page = make_page("https://example.com/person");
        page.structured_data = vec![crate::parser::StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Person".to_string()),
            data: serde_json::json!({"@type": "Person", "name": "John Doe", "jobTitle": "Engineer", "worksFor": {"@type": "Organization", "name": "Acme"}}),
        }];
        assert!(PersonJobTitleValidatorV2::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    // RecipeNutritionValidatorV2 tests

    #[test]
    fn test_recipe_nutrition_v2_missing() {
        let mut page = make_page("https://example.com/recipe");
        page.structured_data = vec![crate::parser::StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Recipe".to_string()),
            data: serde_json::json!({"@type": "Recipe", "name": "Pasta"}),
        }];
        assert!(RecipeNutritionValidatorV2::new()
            .analyze(&make_ctx(&page))
            .iter()
            .any(|f| f.code == "RNUT-V2001"));
    }

    #[test]
    fn test_recipe_nutrition_v2_valid() {
        let mut page = make_page("https://example.com/recipe");
        page.structured_data = vec![crate::parser::StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Recipe".to_string()),
            data: serde_json::json!({"@type": "Recipe", "name": "Pasta", "nutrition": {"@type": "NutritionInformation", "calories": "400 calories"}}),
        }];
        assert!(RecipeNutritionValidatorV2::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }

    // CourseProviderValidatorV2 tests

    #[test]
    fn test_course_prov_v2_missing() {
        let mut page = make_page("https://example.com/course");
        page.structured_data = vec![crate::parser::StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Course".to_string()),
            data: serde_json::json!({"@type": "Course", "name": "Rust Basics"}),
        }];
        assert!(CourseProviderValidatorV2::new()
            .analyze(&make_ctx(&page))
            .iter()
            .any(|f| f.code == "CPROV-V2001"));
    }

    #[test]
    fn test_course_prov_v2_valid() {
        let mut page = make_page("https://example.com/course");
        page.structured_data = vec![crate::parser::StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Course".to_string()),
            data: serde_json::json!({"@type": "Course", "name": "Rust Basics", "provider": {"@type": "Organization", "name": "Udemy"}}),
        }];
        assert!(CourseProviderValidatorV2::new()
            .analyze(&make_ctx(&page))
            .is_empty());
    }
}

// =========================================================================
// JsonLdContextValidator
// =========================================================================

pub struct JsonLdContextValidator;

impl Default for JsonLdContextValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl JsonLdContextValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for JsonLdContextValidator {
    fn name(&self) -> &str {
        "jsonld-context-validator"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if let Some(ref ctx_val) = sd.context {
                if ctx_val != "https://schema.org" && ctx_val != "schema.org" {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Schema,
                        code: "JLDCTX001".to_string(),
                        title: "JSON-LD @context not https://schema.org".to_string(),
                        description: format!(
                            "JSON-LD @context is \"{ctx_val}\" instead of \
                             \"https://schema.org\"."
                        ),
                        url: url.clone(),
                        recommendation: "Use \"https://schema.org\" as the @context in all \
                                         JSON-LD blocks."
                            .to_string(),
                    });
                }
            }
        }

        findings
    }
}

// =========================================================================
// JsonLdTypeValidator
// =========================================================================

pub struct JsonLdTypeValidator;

impl Default for JsonLdTypeValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl JsonLdTypeValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for JsonLdTypeValidator {
    fn name(&self) -> &str {
        "jsonld-type-validator"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.is_none() {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Schema,
                    code: "JLDTYPE001".to_string(),
                    title: "JSON-LD @type missing".to_string(),
                    description: "A JSON-LD block is missing the @type property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add an @type property to describe the structured data content."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// MetaRobotsValidator
// =========================================================================

pub struct MetaRobotsValidator;

impl Default for MetaRobotsValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl MetaRobotsValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for MetaRobotsValidator {
    fn name(&self) -> &str {
        "meta-robots-validator"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let robots = match &ctx.page.meta.robots {
            Some(r) if !r.is_empty() => r.to_lowercase(),
            _ => return findings,
        };

        let directives: Vec<&str> = robots.split(|c| c == ',' || c == ' ').collect();
        let has_noindex = directives.iter().any(|d| *d == "noindex");
        let has_index = directives.iter().any(|d| *d == "index");
        let has_nofollow = directives.iter().any(|d| *d == "nofollow");
        let has_follow = directives.iter().any(|d| *d == "follow");

        if has_noindex && has_index {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "META-ROB001".to_string(),
                title: "Conflicting meta robots directives: index and noindex".to_string(),
                description: "The meta robots tag contains both \"index\" and \"noindex\" \
                              directives, which are contradictory."
                    .to_string(),
                url: url.clone(),
                recommendation: "Use either \"index\" or \"noindex\", not both.".to_string(),
            });
        }

        if has_nofollow && has_follow {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Seo,
                code: "META-ROB002".to_string(),
                title: "Conflicting meta robots directives: follow and nofollow".to_string(),
                description: "The meta robots tag contains both \"follow\" and \"nofollow\" \
                              directives, which are contradictory."
                    .to_string(),
                url: url.clone(),
                recommendation: "Use either \"follow\" or \"nofollow\", not both.".to_string(),
            });
        }

        findings
    }
}

// =========================================================================
// CanonicalChainValidator
// =========================================================================

pub struct CanonicalChainValidator;

impl Default for CanonicalChainValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl CanonicalChainValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for CanonicalChainValidator {
    fn name(&self) -> &str {
        "canonical-chain-validator"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let canonical = match &ctx.page.meta.canonical {
            Some(c) => c.as_str(),
            _ => return findings,
        };

        if canonical.is_empty() {
            return findings;
        }

        if !ctx.redirect_chain.is_empty() {
            let final_url = ctx
                .redirect_chain
                .last()
                .map_or(url.as_str(), |h| h.to.as_str());
            if canonical != final_url && canonical != url.as_str() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "CAN-CHAIN001".to_string(),
                    title: "Canonical URL points to redirect target".to_string(),
                    description: format!(
                        "The canonical URL \"{canonical}\" differs from both the original \
                         URL and the final redirect destination \"{final_url}\"."
                    ),
                    url: url.clone(),
                    recommendation: "Set the canonical URL to the final destination URL after \
                                     all redirects."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// InternalLinkDepthAnalyzerV2
// =========================================================================

pub struct InternalLinkDepthAnalyzerV2;

impl Default for InternalLinkDepthAnalyzerV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl InternalLinkDepthAnalyzerV2 {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for InternalLinkDepthAnalyzerV2 {
    fn name(&self) -> &str {
        "internal-link-depth-v2"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let url_depth = if let Ok(parsed) = url::Url::parse(url) {
            parsed
                .path_segments()
                .map(|s| s.filter(|seg| !seg.is_empty()).count())
                .unwrap_or(0)
        } else {
            return findings;
        };

        if url_depth > 4 {
            findings.push(Finding {
                severity: Severity::Info,
                category: IssueCategory::Links,
                code: "IL-DEPTH001".to_string(),
                title: "Deep internal page".to_string(),
                description: format!(
                    "This page is at depth {url_depth} in the site hierarchy. Deep pages \
                     may receive less link equity."
                ),
                url: url.clone(),
                recommendation: "Ensure deep pages are reachable within 3 clicks from the \
                                 homepage or link to them from higher-level pages."
                    .to_string(),
            });
        }

        findings
    }
}

// =========================================================================
// ExternalLinkQualityAnalyzer
// =========================================================================

pub struct ExternalLinkQualityAnalyzer;

impl Default for ExternalLinkQualityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl ExternalLinkQualityAnalyzer {
    pub fn new() -> Self {
        Self
    }

    fn is_known_bad_domain(host: &str) -> bool {
        const BAD_DOMAINS: &[&str] = &[
            "linkfarm.example",
            "spammy-site.example",
            "lowquality.example",
        ];
        BAD_DOMAINS.iter().any(|d| host.contains(d))
    }
}

impl Analyzer for ExternalLinkQualityAnalyzer {
    fn name(&self) -> &str {
        "external-link-quality"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let bad_links: Vec<&crate::parser::ExtractedLink> = ctx
            .page
            .links
            .iter()
            .filter(|l| l.is_external)
            .filter(|l| {
                url::Url::parse(&l.href)
                    .ok()
                    .and_then(|u| u.host_str().map(|h| Self::is_known_bad_domain(h)))
                    .unwrap_or(false)
            })
            .collect();

        if !bad_links.is_empty() {
            let count = bad_links.len();
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Links,
                code: "EXT-QUAL001".to_string(),
                title: "External links to known low-quality domains".to_string(),
                description: format!(
                    "This page has {count} external link(s) to known low-quality domains."
                ),
                url: url.clone(),
                recommendation: "Remove or nofollow links to known spammy or low-quality domains."
                    .to_string(),
            });
        }

        findings
    }
}

// =========================================================================
// ContentStructureAnalyzer
// =========================================================================

pub struct ContentStructureAnalyzer;

impl Default for ContentStructureAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl ContentStructureAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for ContentStructureAnalyzer {
    fn name(&self) -> &str {
        "content-structure"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        if ctx.page.word_count > 1500 && ctx.page.headings.len() <= 1 {
            findings.push(Finding {
                severity: Severity::Warning,
                category: IssueCategory::Content,
                code: "CONT-STR001".to_string(),
                title: "Long-form content without subheadings".to_string(),
                description: format!(
                    "This page has {} words but only {} heading(s). Long-form content \
                     should use subheadings for better readability and SEO.",
                    ctx.page.word_count,
                    ctx.page.headings.len()
                ),
                url: url.clone(),
                recommendation: "Add H2/H3 subheadings to break up long-form content into \
                                 scannable sections."
                    .to_string(),
            });
        }

        findings
    }
}

// =========================================================================
// KeywordDensityAnalyzer
// =========================================================================

pub struct KeywordDensityAnalyzer;

impl Default for KeywordDensityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl KeywordDensityAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for KeywordDensityAnalyzer {
    fn name(&self) -> &str {
        "keyword-density"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let title = match &ctx.page.meta.title {
            Some(t) if !t.trim().is_empty() => t.trim().to_lowercase(),
            _ => return findings,
        };

        let title_words: Vec<String> = title
            .split_whitespace()
            .map(|w| {
                w.to_lowercase()
                    .chars()
                    .filter(|c| c.is_alphanumeric())
                    .collect::<String>()
            })
            .filter(|w| w.len() > 2 && !STOP_WORDS.contains(&w.as_str()))
            .collect();

        if title_words.is_empty() {
            return findings;
        }

        if let Some(body) = ctx.body {
            let body_lower = body.to_lowercase();
            let body_words: Vec<&str> = body_lower.split_whitespace().collect();
            let total_words = body_words.len();

            if total_words < 100 {
                return findings;
            }

            for keyword in &title_words {
                let count = body_words
                    .iter()
                    .filter(|w| w.contains(keyword.as_str()))
                    .count();
                let density = count as f64 / total_words as f64 * 100.0;

                if density > 3.0 {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Content,
                        code: "KW-DENS001".to_string(),
                        title: "Potential keyword stuffing detected".to_string(),
                        description: format!(
                            "Keyword \"{keyword}\" appears {count} times ({density:.1}%) in \
                             body text. High keyword density may be flagged as keyword stuffing."
                        ),
                        url: url.clone(),
                        recommendation: "Reduce keyword repetition and use natural language \
                                         variations and synonyms."
                            .to_string(),
                    });
                    break;
                }
            }
        }

        findings
    }
}

// =========================================================================
// PageSpeedScoreAnalyzer
// =========================================================================

pub struct PageSpeedScoreAnalyzer;

impl Default for PageSpeedScoreAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl PageSpeedScoreAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for PageSpeedScoreAnalyzer {
    fn name(&self) -> &str {
        "pagespeed-score"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let mut score: u32 = 100;

        if let Some(ttfb) = ctx.response_time {
            let ttfb_ms = ttfb.as_millis() as u32;
            if ttfb_ms > 2000 {
                score = score.saturating_sub(30);
            } else if ttfb_ms > 1000 {
                score = score.saturating_sub(15);
            }
        }

        if let Some(size) = ctx.body_size {
            let size_kb = size / 1024;
            if size_kb > 5000 {
                score = score.saturating_sub(25);
            } else if size_kb > 2000 {
                score = score.saturating_sub(10);
            }
        }

        if let Some(compressed) = ctx.compressed_size {
            if let Some(uncompressed) = ctx.body_size {
                if uncompressed > 0 {
                    let ratio = compressed as f64 / uncompressed as f64;
                    if ratio > 0.9 {
                        score = score.saturating_sub(15);
                    }
                }
            }
        }

        findings.push(Finding {
            severity: Severity::Info,
            category: IssueCategory::Performance,
            code: "PERF-EST001".to_string(),
            title: "Estimated performance score".to_string(),
            description: format!("Estimated page performance score: {score}/100."),
            url: url.clone(),
            recommendation: if score < 50 {
                "Performance is poor. Optimize response time, compress resources, and \
                 reduce page size."
                    .to_string()
            } else if score < 80 {
                "Performance is moderate. Consider optimizing TTFB and compressing \
                 responses."
                    .to_string()
            } else {
                "Performance is good.".to_string()
            },
        });

        findings
    }
}

// =========================================================================
// MobileFriendlinessScoreAnalyzer
// =========================================================================

pub struct MobileFriendlinessScoreAnalyzer;

impl Default for MobileFriendlinessScoreAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl MobileFriendlinessScoreAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for MobileFriendlinessScoreAnalyzer {
    fn name(&self) -> &str {
        "mobile-friendliness-score"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let mut score: u32 = 100;
        let mut issues = Vec::new();

        if ctx.page.meta.viewport.is_none() {
            score = score.saturating_sub(40);
            issues.push("missing viewport meta tag");
        }

        if let Some(body) = ctx.body {
            let lower = body.to_lowercase();
            if lower.contains("width=600")
                || lower.contains("width=480")
                || lower.contains("width=320")
            {
                score = score.saturating_sub(20);
                issues.push("fixed viewport width");
            }

            if lower.contains("user-scalable=no") || lower.contains("user-scalable=0") {
                score = score.saturating_sub(15);
                issues.push("pinch-to-zoom disabled");
            }

            if !ctx.page.has_lang_attribute && !ctx.page.html_lang.is_some() {
                score = score.saturating_sub(5);
            }
        }

        let issue_text = if issues.is_empty() {
            "No issues detected.".to_string()
        } else {
            format!("Issues: {}.", issues.join(", "))
        };

        findings.push(Finding {
            severity: Severity::Info,
            category: IssueCategory::Mobile,
            code: "MOB-SCORE001".to_string(),
            title: "Mobile friendliness score".to_string(),
            description: format!("Estimated mobile friendliness score: {score}/100. {issue_text}"),
            url: url.clone(),
            recommendation: if score < 60 {
                "Page has significant mobile-friendliness issues. Add a viewport meta tag \
                 and ensure responsive design."
                    .to_string()
            } else if score < 85 {
                "Page has minor mobile-friendliness issues. Review viewport settings.".to_string()
            } else {
                "Page appears mobile-friendly.".to_string()
            },
        });

        findings
    }
}

// ---------------------------------------------------------------------------
// Content Analyzer: Article Author Validator
// ---------------------------------------------------------------------------

pub struct ArticleAuthorValidator;

impl Default for ArticleAuthorValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ArticleAuthorValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for ArticleAuthorValidator {
    fn name(&self) -> &str {
        "article-author"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            let schema_type = sd.data.get("@type").and_then(|t| t.as_str()).unwrap_or("");
            if !matches!(
                schema_type,
                "Article" | "NewsArticle" | "BlogPosting" | "ScholarlyArticle"
            ) {
                continue;
            }
            let data = &sd.data;
            if data.get("author").is_none() {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Content,
                    code: "ART-AUTH001".to_string(),
                    title: "Article missing author".to_string(),
                    description: format!(
                        "A {schema_type} structured data block is missing the \"author\" property."
                    ),
                    url: url.clone(),
                    recommendation: "Add \"author\" with a Person or Organization object."
                        .to_string(),
                });
            }
        }
        findings
    }
}

// ---------------------------------------------------------------------------
// Content Analyzer: Article Date Published Validator
// ---------------------------------------------------------------------------

pub struct ArticleDatePublishedValidator;

impl Default for ArticleDatePublishedValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ArticleDatePublishedValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for ArticleDatePublishedValidator {
    fn name(&self) -> &str {
        "article-date-published"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            let schema_type = sd.data.get("@type").and_then(|t| t.as_str()).unwrap_or("");
            if !matches!(
                schema_type,
                "Article" | "NewsArticle" | "BlogPosting" | "ScholarlyArticle"
            ) {
                continue;
            }
            let data = &sd.data;
            if data.get("datePublished").is_none() {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Content,
                    code: "ART-DT001".to_string(),
                    title: "Article missing datePublished".to_string(),
                    description: format!(
                        "A {schema_type} structured data block is missing the \
                         \"datePublished\" property."
                    ),
                    url: url.clone(),
                    recommendation: "Add \"datePublished\" with an ISO 8601 date value."
                        .to_string(),
                });
            }
        }
        findings
    }
}

// ---------------------------------------------------------------------------
// Content Analyzer: Article Headline Validator
// ---------------------------------------------------------------------------

pub struct ArticleHeadlineValidator;

impl Default for ArticleHeadlineValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ArticleHeadlineValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for ArticleHeadlineValidator {
    fn name(&self) -> &str {
        "article-headline"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            let schema_type = sd.data.get("@type").and_then(|t| t.as_str()).unwrap_or("");
            if !matches!(
                schema_type,
                "Article" | "NewsArticle" | "BlogPosting" | "ScholarlyArticle"
            ) {
                continue;
            }
            let data = &sd.data;
            if data.get("headline").is_none() {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Content,
                    code: "ART-HL001".to_string(),
                    title: "Article missing headline".to_string(),
                    description: format!(
                        "A {schema_type} structured data block is missing the \"headline\" property."
                    ),
                    url: url.clone(),
                    recommendation: "Add \"headline\" with the article title.".to_string(),
                });
            }
        }
        findings
    }
}

// ---------------------------------------------------------------------------
// Content Analyzer: Organization Name Validator
// ---------------------------------------------------------------------------

pub struct OrganizationNameValidator;

impl Default for OrganizationNameValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl OrganizationNameValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for OrganizationNameValidator {
    fn name(&self) -> &str {
        "organization-name"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Organization") {
                continue;
            }
            let data = &sd.data;
            if data
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .is_empty()
            {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Content,
                    code: "ORG-NAME001".to_string(),
                    title: "Organization missing name".to_string(),
                    description: "An Organization structured data block is missing the \"name\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with the organization name.".to_string(),
                });
            }
        }
        findings
    }
}

// ---------------------------------------------------------------------------
// Content Analyzer: Person Name Validator
// ---------------------------------------------------------------------------

pub struct PersonNameValidator;

impl Default for PersonNameValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl PersonNameValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for PersonNameValidator {
    fn name(&self) -> &str {
        "person-name"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Person") {
                continue;
            }
            let data = &sd.data;
            if data
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .is_empty()
            {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Content,
                    code: "PERS-NAME001".to_string(),
                    title: "Person missing name".to_string(),
                    description: "A Person structured data block is missing the \"name\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with the person's name.".to_string(),
                });
            }
        }
        findings
    }
}

// ---------------------------------------------------------------------------
// Content Analyzer: JobPosting Title Validator
// ---------------------------------------------------------------------------

pub struct JobPostingTitleValidator;

impl Default for JobPostingTitleValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl JobPostingTitleValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for JobPostingTitleValidator {
    fn name(&self) -> &str {
        "job-posting-title"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("JobPosting") {
                continue;
            }
            let data = &sd.data;
            if data.get("title").is_none() {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Content,
                    code: "JOB-TITLE001".to_string(),
                    title: "JobPosting missing title".to_string(),
                    description: "A JobPosting structured data block is missing the \"title\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"title\" with the job position title.".to_string(),
                });
            }
        }
        findings
    }
}

// ---------------------------------------------------------------------------
// Content Analyzer: Course Name Validator
// ---------------------------------------------------------------------------

pub struct CourseNameValidator;

impl Default for CourseNameValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl CourseNameValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for CourseNameValidator {
    fn name(&self) -> &str {
        "course-name"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Course") {
                continue;
            }
            let data = &sd.data;
            if data
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .is_empty()
            {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Content,
                    code: "COURSE-NAME001".to_string(),
                    title: "Course missing name".to_string(),
                    description: "A Course structured data block is missing the \"name\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with the course title.".to_string(),
                });
            }
        }
        findings
    }
}

// ---------------------------------------------------------------------------
// Content Analyzer: Recipe Name Validator
// ---------------------------------------------------------------------------

pub struct RecipeNameValidator;

impl Default for RecipeNameValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl RecipeNameValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for RecipeNameValidator {
    fn name(&self) -> &str {
        "recipe-name"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Recipe") {
                continue;
            }
            let data = &sd.data;
            if data
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .is_empty()
            {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Content,
                    code: "RECIPE-NAME001".to_string(),
                    title: "Recipe missing name".to_string(),
                    description: "A Recipe structured data block is missing the \"name\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with the recipe title.".to_string(),
                });
            }
        }
        findings
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod content_analyzer_v2_tests {
    use super::*;
    use crate::meta::MetaTags;
    use crate::parser::{ExtractedLink, Heading, ParsedPage, StructuredData};
    use crate::RedirectHop;

    fn make_page(url: &str) -> ParsedPage {
        ParsedPage {
            url: url.to_string(),
            meta: MetaTags::default(),
            headings: Vec::new(),
            links: Vec::new(),
            images: Vec::new(),
            forms: Vec::new(),
            scripts: Vec::new(),
            styles: Vec::new(),
            structured_data: Vec::new(),
            word_count: 0,
            sentence_count: 0,
            landmarks: Vec::new(),
            has_skip_link: false,
            has_main_landmark: false,
            has_nav_landmark: false,
            has_positive_tabindex: false,
            tabindex_negative_count: 0,
            aria_role_count: 0,
            aria_label_count: 0,
            has_lang_attribute: false,
            html_lang: None,
            has_aria_hidden: false,
            tables_with_headers: 0,
            tables_total: 0,
            tables_with_captions: 0,
            og_image_width: None,
            og_image_height: None,
        }
    }

    fn make_ctx<'a>(page: &'a ParsedPage, status: Option<u16>) -> AnalysisContext<'a> {
        AnalysisContext {
            page,
            body: None,
            status_code: status,
            headers: &[],
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        }
    }

    fn make_ctx_full<'a>(
        page: &'a ParsedPage,
        body: Option<&'a str>,
        redirect_chain: &'a [RedirectHop],
    ) -> AnalysisContext<'a> {
        AnalysisContext {
            page,
            body,
            status_code: Some(200),
            headers: &[],
            response_time: None,
            redirect_chain,
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        }
    }

    // ---- JsonLdContextValidator (10 tests) ----

    #[test]
    fn test_jldctx_wrong_context() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://example.com/schema".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({"@type": "Article"}),
        }];
        let ctx = make_ctx(&page, None);
        let findings = JsonLdContextValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "JLDCTX001"));
    }

    #[test]
    fn test_jldctx_valid_https() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({"@type": "Article"}),
        }];
        let ctx = make_ctx(&page, None);
        assert!(JsonLdContextValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_jldctx_valid_bare_schema_org() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product"}),
        }];
        let ctx = make_ctx(&page, None);
        assert!(JsonLdContextValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_jldctx_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, None);
        assert!(JsonLdContextValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_jldctx_none_context() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: None,
            r#type: Some("Article".to_string()),
            data: serde_json::json!({"@type": "Article"}),
        }];
        let ctx = make_ctx(&page, None);
        assert!(JsonLdContextValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_jldctx_multiple_blocks_one_wrong() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Article".to_string()),
                data: serde_json::json!({"@type": "Article"}),
            },
            StructuredData {
                context: Some("https://example.com/schema".to_string()),
                r#type: Some("Product".to_string()),
                data: serde_json::json!({"@type": "Product"}),
            },
        ];
        let ctx = make_ctx(&page, None);
        let findings = JsonLdContextValidator::new().analyze(&ctx);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "JLDCTX001");
    }

    #[test]
    fn test_jldctx_severity_warning() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://bad.example.com".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({"@type": "Article"}),
        }];
        let ctx = make_ctx(&page, None);
        let findings = JsonLdContextValidator::new().analyze(&ctx);
        assert_eq!(findings[0].severity, Severity::Warning);
        assert_eq!(findings[0].category, IssueCategory::Schema);
    }

    #[test]
    fn test_jldctx_all_blocks_wrong() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://a.example.com".to_string()),
                r#type: Some("Article".to_string()),
                data: serde_json::json!({"@type": "Article"}),
            },
            StructuredData {
                context: Some("https://b.example.com".to_string()),
                r#type: Some("Product".to_string()),
                data: serde_json::json!({"@type": "Product"}),
            },
        ];
        let ctx = make_ctx(&page, None);
        assert_eq!(JsonLdContextValidator::new().analyze(&ctx).len(), 2);
    }

    #[test]
    fn test_jldctx_url_is_page_url() {
        let mut page = make_page("https://example.com/article");
        page.structured_data = vec![StructuredData {
            context: Some("https://custom-vocab.example.com".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({"@type": "Article"}),
        }];
        let ctx = make_ctx(&page, None);
        let findings = JsonLdContextValidator::new().analyze(&ctx);
        assert_eq!(findings[0].url, "https://example.com/article");
    }

    // ---- JsonLdTypeValidator (10 tests) ----

    #[test]
    fn test_jldtype_missing() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: None,
            data: serde_json::json!({"@context": "https://schema.org"}),
        }];
        let ctx = make_ctx(&page, None);
        let findings = JsonLdTypeValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "JLDTYPE001"));
    }

    #[test]
    fn test_jldtype_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({"@type": "Article"}),
        }];
        let ctx = make_ctx(&page, None);
        assert!(JsonLdTypeValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_jldtype_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, None);
        assert!(JsonLdTypeValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_jldtype_multiple_one_missing() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Article".to_string()),
                data: serde_json::json!({"@type": "Article"}),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: None,
                data: serde_json::json!({"@context": "https://schema.org"}),
            },
        ];
        let ctx = make_ctx(&page, None);
        let findings = JsonLdTypeValidator::new().analyze(&ctx);
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn test_jldtype_severity_error() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: None,
            data: serde_json::json!({}),
        }];
        let ctx = make_ctx(&page, None);
        let findings = JsonLdTypeValidator::new().analyze(&ctx);
        assert_eq!(findings[0].severity, Severity::Error);
        assert_eq!(findings[0].category, IssueCategory::Schema);
    }

    #[test]
    fn test_jldtype_all_missing() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: None,
                r#type: None,
                data: serde_json::json!({}),
            },
            StructuredData {
                context: None,
                r#type: None,
                data: serde_json::json!({}),
            },
        ];
        let ctx = make_ctx(&page, None);
        assert_eq!(JsonLdTypeValidator::new().analyze(&ctx).len(), 2);
    }

    #[test]
    fn test_jldtype_product_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product", "name": "Widget"}),
        }];
        let ctx = make_ctx(&page, None);
        assert!(JsonLdTypeValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_jldtype_empty_string_type() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("".to_string()),
            data: serde_json::json!({"@type": ""}),
        }];
        let ctx = make_ctx(&page, None);
        assert!(JsonLdTypeValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_jldtype_url_in_finding() {
        let mut page = make_page("https://example.com/page");
        page.structured_data = vec![StructuredData {
            context: None,
            r#type: None,
            data: serde_json::json!({}),
        }];
        let ctx = make_ctx(&page, None);
        let findings = JsonLdTypeValidator::new().analyze(&ctx);
        assert_eq!(findings[0].url, "https://example.com/page");
    }

    // ---- MetaRobotsValidator (10 tests) ----

    #[test]
    fn test_metarob_conflicting_index_noindex() {
        let mut page = make_page("https://example.com");
        page.meta.robots = Some("index, noindex".to_string());
        let ctx = make_ctx(&page, None);
        let findings = MetaRobotsValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "META-ROB001"));
    }

    #[test]
    fn test_metarob_conflicting_follow_nofollow() {
        let mut page = make_page("https://example.com");
        page.meta.robots = Some("follow, nofollow".to_string());
        let ctx = make_ctx(&page, None);
        let findings = MetaRobotsValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "META-ROB002"));
    }

    #[test]
    fn test_metarob_valid_noindex() {
        let mut page = make_page("https://example.com");
        page.meta.robots = Some("noindex".to_string());
        let ctx = make_ctx(&page, None);
        assert!(MetaRobotsValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_metarob_valid_index() {
        let mut page = make_page("https://example.com");
        page.meta.robots = Some("index, follow".to_string());
        let ctx = make_ctx(&page, None);
        assert!(MetaRobotsValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_metarob_no_robots() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, None);
        assert!(MetaRobotsValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_metarob_empty_robots() {
        let mut page = make_page("https://example.com");
        page.meta.robots = Some("".to_string());
        let ctx = make_ctx(&page, None);
        assert!(MetaRobotsValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_metarob_both_conflicts() {
        let mut page = make_page("https://example.com");
        page.meta.robots = Some("index, noindex, follow, nofollow".to_string());
        let ctx = make_ctx(&page, None);
        let findings = MetaRobotsValidator::new().analyze(&ctx);
        assert_eq!(findings.len(), 2);
    }

    #[test]
    fn test_metarob_severity_warning() {
        let mut page = make_page("https://example.com");
        page.meta.robots = Some("index, noindex".to_string());
        let ctx = make_ctx(&page, None);
        let findings = MetaRobotsValidator::new().analyze(&ctx);
        assert_eq!(findings[0].severity, Severity::Warning);
    }

    #[test]
    fn test_metarob_case_insensitive() {
        let mut page = make_page("https://example.com");
        page.meta.robots = Some("INDEX, NOINDEX".to_string());
        let ctx = make_ctx(&page, None);
        let findings = MetaRobotsValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "META-ROB001"));
    }

    #[test]
    fn test_metarob_only_nofollow() {
        let mut page = make_page("https://example.com");
        page.meta.robots = Some("nofollow".to_string());
        let ctx = make_ctx(&page, None);
        assert!(MetaRobotsValidator::new().analyze(&ctx).is_empty());
    }

    // ---- CanonicalChainValidator (10 tests) ----

    #[test]
    fn test_canonical_chain_differs_from_final() {
        let mut page = make_page("https://example.com/old");
        page.meta.canonical = Some("https://example.com/canonical".parse().unwrap());
        let chain = vec![RedirectHop {
            from: "https://example.com/old".parse().unwrap(),
            to: "https://example.com/new".parse().unwrap(),
            status_code: 301,
        }];
        let ctx = make_ctx_full(&page, None, &chain);
        let findings = CanonicalChainValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "CAN-CHAIN001"));
    }

    #[test]
    fn test_canonical_chain_matches_final() {
        let mut page = make_page("https://example.com/old");
        page.meta.canonical = Some("https://example.com/new".parse().unwrap());
        let chain = vec![RedirectHop {
            from: "https://example.com/old".parse().unwrap(),
            to: "https://example.com/new".parse().unwrap(),
            status_code: 301,
        }];
        let ctx = make_ctx_full(&page, None, &chain);
        assert!(CanonicalChainValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_canonical_chain_no_redirect() {
        let mut page = make_page("https://example.com");
        page.meta.canonical = Some("https://example.com".parse().unwrap());
        let ctx = make_ctx(&page, None);
        assert!(CanonicalChainValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_canonical_chain_no_canonical() {
        let page = make_page("https://example.com");
        let chain = vec![RedirectHop {
            from: "https://example.com/old".parse().unwrap(),
            to: "https://example.com/new".parse().unwrap(),
            status_code: 301,
        }];
        let ctx = make_ctx_full(&page, None, &chain);
        assert!(CanonicalChainValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_canonical_chain_matches_original_url() {
        let mut page = make_page("https://example.com/page");
        page.meta.canonical = Some("https://example.com/page".parse().unwrap());
        let chain = vec![RedirectHop {
            from: "https://example.com/old".parse().unwrap(),
            to: "https://example.com/different".parse().unwrap(),
            status_code: 302,
        }];
        let ctx = make_ctx_full(&page, None, &chain);
        assert!(CanonicalChainValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_canonical_chain_multi_hop() {
        let mut page = make_page("https://example.com/a");
        page.meta.canonical = Some("https://example.com/canonical".parse().unwrap());
        let chain = vec![
            RedirectHop {
                from: "https://example.com/a".parse().unwrap(),
                to: "https://example.com/b".parse().unwrap(),
                status_code: 301,
            },
            RedirectHop {
                from: "https://example.com/b".parse().unwrap(),
                to: "https://example.com/c".parse().unwrap(),
                status_code: 301,
            },
        ];
        let ctx = make_ctx_full(&page, None, &chain);
        let findings = CanonicalChainValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "CAN-CHAIN001"));
    }

    #[test]
    fn test_canonical_chain_severity_warning() {
        let mut page = make_page("https://example.com/old");
        page.meta.canonical = Some("https://example.com/canonical".parse().unwrap());
        let chain = vec![RedirectHop {
            from: "https://example.com/old".parse().unwrap(),
            to: "https://example.com/new".parse().unwrap(),
            status_code: 301,
        }];
        let ctx = make_ctx_full(&page, None, &chain);
        let findings = CanonicalChainValidator::new().analyze(&ctx);
        assert_eq!(findings[0].severity, Severity::Warning);
    }

    #[test]
    fn test_canonical_chain_category_seo() {
        let mut page = make_page("https://example.com/old");
        page.meta.canonical = Some("https://example.com/canonical".parse().unwrap());
        let chain = vec![RedirectHop {
            from: "https://example.com/old".parse().unwrap(),
            to: "https://example.com/new".parse().unwrap(),
            status_code: 301,
        }];
        let ctx = make_ctx_full(&page, None, &chain);
        let findings = CanonicalChainValidator::new().analyze(&ctx);
        assert_eq!(findings[0].category, IssueCategory::Seo);
    }

    #[test]
    fn test_canonical_chain_one_finding() {
        let mut page = make_page("https://example.com/old");
        page.meta.canonical = Some("https://example.com/canonical".parse().unwrap());
        let chain = vec![RedirectHop {
            from: "https://example.com/old".parse().unwrap(),
            to: "https://example.com/new".parse().unwrap(),
            status_code: 301,
        }];
        let ctx = make_ctx_full(&page, None, &chain);
        assert_eq!(CanonicalChainValidator::new().analyze(&ctx).len(), 1);
    }

    // ---- InternalLinkDepthAnalyzerV2 (10 tests) ----

    #[test]
    fn test_il_depth_deep_page() {
        let page = make_page("https://example.com/a/b/c/d/e");
        let ctx = make_ctx(&page, None);
        let findings = InternalLinkDepthAnalyzerV2::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "IL-DEPTH001"));
    }

    #[test]
    fn test_il_depth_shallow_page() {
        let page = make_page("https://example.com/about");
        let ctx = make_ctx(&page, None);
        assert!(InternalLinkDepthAnalyzerV2::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_il_depth_root() {
        let page = make_page("https://example.com/");
        let ctx = make_ctx(&page, None);
        assert!(InternalLinkDepthAnalyzerV2::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_il_depth_exactly_4() {
        let page = make_page("https://example.com/a/b/c/d");
        let ctx = make_ctx(&page, None);
        assert!(InternalLinkDepthAnalyzerV2::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_il_depth_5_segments() {
        let page = make_page("https://example.com/a/b/c/d/e");
        let ctx = make_ctx(&page, None);
        let findings = InternalLinkDepthAnalyzerV2::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "IL-DEPTH001"));
    }

    #[test]
    fn test_il_depth_severity_info() {
        let page = make_page("https://example.com/a/b/c/d/e/f");
        let ctx = make_ctx(&page, None);
        let findings = InternalLinkDepthAnalyzerV2::new().analyze(&ctx);
        assert_eq!(findings[0].severity, Severity::Info);
        assert_eq!(findings[0].category, IssueCategory::Links);
    }

    #[test]
    fn test_il_depth_one_finding() {
        let page = make_page("https://example.com/a/b/c/d/e/f/g");
        let ctx = make_ctx(&page, None);
        assert_eq!(InternalLinkDepthAnalyzerV2::new().analyze(&ctx).len(), 1);
    }

    #[test]
    fn test_il_depth_no_segments() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, None);
        assert!(InternalLinkDepthAnalyzerV2::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_il_depth_very_deep() {
        let page = make_page("https://example.com/a/b/c/d/e/f/g/h/i/j");
        let ctx = make_ctx(&page, None);
        let findings = InternalLinkDepthAnalyzerV2::new().analyze(&ctx);
        assert_eq!(findings[0].code, "IL-DEPTH001");
    }

    #[test]
    fn test_il_depth_invalid_url() {
        let page = make_page("not-a-valid-url");
        let ctx = make_ctx(&page, None);
        assert!(InternalLinkDepthAnalyzerV2::new().analyze(&ctx).is_empty());
    }

    // ---- ExternalLinkQualityAnalyzer (10 tests) ----

    #[test]
    fn test_ext_qual_bad_domain() {
        let mut page = make_page("https://example.com");
        page.links = vec![ExtractedLink {
            href: "https://linkfarm.example.com/page".to_string(),
            text: "Link".to_string(),
            rel: vec![],
            is_external: true,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, None);
        let findings = ExternalLinkQualityAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "EXT-QUAL001"));
    }

    #[test]
    fn test_ext_qual_good_domain() {
        let mut page = make_page("https://example.com");
        page.links = vec![ExtractedLink {
            href: "https://wikipedia.org/wiki/Test".to_string(),
            text: "Wikipedia".to_string(),
            rel: vec![],
            is_external: true,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, None);
        assert!(ExternalLinkQualityAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_ext_qual_internal_link_ignored() {
        let mut page = make_page("https://example.com");
        page.links = vec![ExtractedLink {
            href: "https://linkfarm.example.com/page".to_string(),
            text: "Link".to_string(),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, None);
        assert!(ExternalLinkQualityAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_ext_qual_no_links() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, None);
        assert!(ExternalLinkQualityAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_ext_qual_multiple_bad_links() {
        let mut page = make_page("https://example.com");
        page.links = vec![
            ExtractedLink {
                href: "https://linkfarm.example.com/a".to_string(),
                text: "A".to_string(),
                rel: vec![],
                is_external: true,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "https://spammy-site.example.com/b".to_string(),
                text: "B".to_string(),
                rel: vec![],
                is_external: true,
                aria_label: None,
                img_alt: None,
            },
        ];
        let ctx = make_ctx(&page, None);
        let findings = ExternalLinkQualityAnalyzer::new().analyze(&ctx);
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn test_ext_qual_severity_warning() {
        let mut page = make_page("https://example.com");
        page.links = vec![ExtractedLink {
            href: "https://linkfarm.example.com/page".to_string(),
            text: "Link".to_string(),
            rel: vec![],
            is_external: true,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, None);
        let findings = ExternalLinkQualityAnalyzer::new().analyze(&ctx);
        assert_eq!(findings[0].severity, Severity::Warning);
    }

    #[test]
    fn test_ext_qual_category_links() {
        let mut page = make_page("https://example.com");
        page.links = vec![ExtractedLink {
            href: "https://lowquality.example.com/page".to_string(),
            text: "Link".to_string(),
            rel: vec![],
            is_external: true,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, None);
        let findings = ExternalLinkQualityAnalyzer::new().analyze(&ctx);
        assert_eq!(findings[0].category, IssueCategory::Links);
    }

    #[test]
    fn test_ext_qual_count_in_description() {
        let mut page = make_page("https://example.com");
        page.links = vec![ExtractedLink {
            href: "https://linkfarm.example.com/a".to_string(),
            text: "A".to_string(),
            rel: vec![],
            is_external: true,
            aria_label: None,
            img_alt: None,
        }];
        let ctx = make_ctx(&page, None);
        let findings = ExternalLinkQualityAnalyzer::new().analyze(&ctx);
        assert!(findings[0].description.contains("1"));
    }

    #[test]
    fn test_ext_qual_mixed_links() {
        let mut page = make_page("https://example.com");
        page.links = vec![
            ExtractedLink {
                href: "https://linkfarm.example.com/page".to_string(),
                text: "Bad".to_string(),
                rel: vec![],
                is_external: true,
                aria_label: None,
                img_alt: None,
            },
            ExtractedLink {
                href: "https://wikipedia.org/wiki/Test".to_string(),
                text: "Good".to_string(),
                rel: vec![],
                is_external: true,
                aria_label: None,
                img_alt: None,
            },
        ];
        let ctx = make_ctx(&page, None);
        let findings = ExternalLinkQualityAnalyzer::new().analyze(&ctx);
        assert_eq!(findings.len(), 1);
    }

    // ---- ContentStructureAnalyzer (10 tests) ----

    #[test]
    fn test_cont_str_long_content_no_headings() {
        let mut page = make_page("https://example.com/article");
        page.word_count = 2000;
        page.headings = vec![Heading {
            level: 1,
            text: "Title".to_string(),
            length: 5,
        }];
        let ctx = make_ctx(&page, None);
        let findings = ContentStructureAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "CONT-STR001"));
    }

    #[test]
    fn test_cont_str_short_content_ok() {
        let mut page = make_page("https://example.com/article");
        page.word_count = 500;
        page.headings = vec![Heading {
            level: 1,
            text: "Title".to_string(),
            length: 5,
        }];
        let ctx = make_ctx(&page, None);
        assert!(ContentStructureAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_cont_str_long_content_with_headings() {
        let mut page = make_page("https://example.com/article");
        page.word_count = 2000;
        page.headings = vec![
            Heading {
                level: 1,
                text: "Title".to_string(),
                length: 5,
            },
            Heading {
                level: 2,
                text: "Section".to_string(),
                length: 7,
            },
        ];
        let ctx = make_ctx(&page, None);
        assert!(ContentStructureAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_cont_str_zero_words() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, None);
        assert!(ContentStructureAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_cont_str_exactly_1500_words() {
        let mut page = make_page("https://example.com/article");
        page.word_count = 1500;
        page.headings = vec![Heading {
            level: 1,
            text: "Title".to_string(),
            length: 5,
        }];
        let ctx = make_ctx(&page, None);
        assert!(ContentStructureAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_cont_str_1501_words() {
        let mut page = make_page("https://example.com/article");
        page.word_count = 1501;
        page.headings = vec![Heading {
            level: 1,
            text: "Title".to_string(),
            length: 5,
        }];
        let ctx = make_ctx(&page, None);
        let findings = ContentStructureAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "CONT-STR001"));
    }

    #[test]
    fn test_cont_str_severity_warning() {
        let mut page = make_page("https://example.com/article");
        page.word_count = 3000;
        page.headings = vec![Heading {
            level: 1,
            text: "Title".to_string(),
            length: 5,
        }];
        let ctx = make_ctx(&page, None);
        let findings = ContentStructureAnalyzer::new().analyze(&ctx);
        assert_eq!(findings[0].severity, Severity::Warning);
        assert_eq!(findings[0].category, IssueCategory::Content);
    }

    #[test]
    fn test_cont_str_one_finding() {
        let mut page = make_page("https://example.com/article");
        page.word_count = 5000;
        page.headings = vec![Heading {
            level: 1,
            text: "Title".to_string(),
            length: 5,
        }];
        let ctx = make_ctx(&page, None);
        assert_eq!(ContentStructureAnalyzer::new().analyze(&ctx).len(), 1);
    }

    #[test]
    fn test_cont_str_two_headings_ok() {
        let mut page = make_page("https://example.com/article");
        page.word_count = 2000;
        page.headings = vec![
            Heading {
                level: 1,
                text: "Title".to_string(),
                length: 5,
            },
            Heading {
                level: 2,
                text: "Sub".to_string(),
                length: 3,
            },
        ];
        let ctx = make_ctx(&page, None);
        assert!(ContentStructureAnalyzer::new().analyze(&ctx).is_empty());
    }

    // ---- PageSpeedScoreAnalyzer (10 tests) ----

    #[test]
    fn test_perf_est_no_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, None);
        let findings = PageSpeedScoreAnalyzer::new().analyze(&ctx);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "PERF-EST001");
    }

    #[test]
    fn test_perf_est_good_score() {
        let page = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &page,
            body: None,
            status_code: Some(200),
            headers: &[],
            response_time: Some(std::time::Duration::from_millis(100)),
            redirect_chain: &[],
            robots_txt: None,
            body_size: Some(100_000),
            compressed_size: Some(30_000),
            server: None,
            content_type: None,
            rendered: None,
        };
        let findings = PageSpeedScoreAnalyzer::new().analyze(&ctx);
        assert_eq!(findings[0].code, "PERF-EST001");
        assert!(findings[0].description.contains("100/100"));
    }

    #[test]
    fn test_perf_est_slow_ttfb() {
        let page = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &page,
            body: None,
            status_code: Some(200),
            headers: &[],
            response_time: Some(std::time::Duration::from_millis(3000)),
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let findings = PageSpeedScoreAnalyzer::new().analyze(&ctx);
        assert!(findings[0].description.contains("70/100"));
    }

    #[test]
    fn test_perf_est_large_body() {
        let page = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &page,
            body: None,
            status_code: Some(200),
            headers: &[],
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: Some(6_000_000),
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let findings = PageSpeedScoreAnalyzer::new().analyze(&ctx);
        assert!(findings[0].description.contains("75/100"));
    }

    #[test]
    fn test_perf_est_poor_compression() {
        let page = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &page,
            body: None,
            status_code: Some(200),
            headers: &[],
            response_time: None,
            redirect_chain: &[],
            robots_txt: None,
            body_size: Some(1_000_000),
            compressed_size: Some(950_000),
            server: None,
            content_type: None,
            rendered: None,
        };
        let findings = PageSpeedScoreAnalyzer::new().analyze(&ctx);
        assert!(findings[0].description.contains("85/100"));
    }

    #[test]
    fn test_perf_est_category_performance() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, None);
        let findings = PageSpeedScoreAnalyzer::new().analyze(&ctx);
        assert_eq!(findings[0].category, IssueCategory::Performance);
    }

    #[test]
    fn test_perf_est_severity_info() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, None);
        let findings = PageSpeedScoreAnalyzer::new().analyze(&ctx);
        assert_eq!(findings[0].severity, Severity::Info);
    }

    #[test]
    fn test_perf_est_poor_recommendation() {
        let page = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &page,
            body: None,
            status_code: Some(200),
            headers: &[],
            response_time: Some(std::time::Duration::from_millis(5000)),
            redirect_chain: &[],
            robots_txt: None,
            body_size: Some(10_000_000),
            compressed_size: Some(9_500_000),
            server: None,
            content_type: None,
            rendered: None,
        };
        let findings = PageSpeedScoreAnalyzer::new().analyze(&ctx);
        assert!(findings[0].recommendation.contains("poor"));
    }

    #[test]
    fn test_perf_est_moderate_ttfb() {
        let page = make_page("https://example.com");
        let ctx = AnalysisContext {
            page: &page,
            body: None,
            status_code: Some(200),
            headers: &[],
            response_time: Some(std::time::Duration::from_millis(1500)),
            redirect_chain: &[],
            robots_txt: None,
            body_size: None,
            compressed_size: None,
            server: None,
            content_type: None,
            rendered: None,
        };
        let findings = PageSpeedScoreAnalyzer::new().analyze(&ctx);
        assert!(findings[0].description.contains("85/100"));
    }

    #[test]
    fn test_perf_est_one_finding() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, None);
        assert_eq!(PageSpeedScoreAnalyzer::new().analyze(&ctx).len(), 1);
    }

    // ---- MobileFriendlinessScoreAnalyzer (10 tests) ----

    #[test]
    fn test_mob_score_no_viewport() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, None);
        let findings = MobileFriendlinessScoreAnalyzer::new().analyze(&ctx);
        assert_eq!(findings[0].code, "MOB-SCORE001");
        assert!(findings[0].description.contains("60/100"));
    }

    #[test]
    fn test_mob_score_with_viewport() {
        let mut page = make_page("https://example.com");
        page.meta.viewport = Some("width=device-width, initial-scale=1".to_string());
        let ctx = make_ctx(&page, None);
        let findings = MobileFriendlinessScoreAnalyzer::new().analyze(&ctx);
        assert!(findings[0].description.contains("100/100"));
    }

    #[test]
    fn test_mob_score_fixed_width() {
        let mut page = make_page("https://example.com");
        page.has_lang_attribute = true;
        let body =
            r#"<html><head><meta name="viewport" content="width=600"></head><body></body></html>"#;
        let ctx = make_ctx_full(&page, Some(body), &[]);
        let findings = MobileFriendlinessScoreAnalyzer::new().analyze(&ctx);
        // 100 - 40 (no viewport in meta) - 20 (fixed width in body) = 40
        assert!(findings[0].description.contains("40/100"));
    }

    #[test]
    fn test_mob_score_pinch_zoom_disabled() {
        let mut page = make_page("https://example.com");
        page.has_lang_attribute = true;
        let body = r#"<html><head><meta name="viewport" content="width=device-width, user-scalable=no"></head><body></body></html>"#;
        let ctx = make_ctx_full(&page, Some(body), &[]);
        let findings = MobileFriendlinessScoreAnalyzer::new().analyze(&ctx);
        // 100 - 40 (no viewport in meta) - 15 (user-scalable=no) = 45
        assert!(findings[0].description.contains("45/100"));
    }

    #[test]
    fn test_mob_score_category_mobile() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, None);
        let findings = MobileFriendlinessScoreAnalyzer::new().analyze(&ctx);
        assert_eq!(findings[0].category, IssueCategory::Mobile);
    }

    #[test]
    fn test_mob_score_severity_info() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, None);
        let findings = MobileFriendlinessScoreAnalyzer::new().analyze(&ctx);
        assert_eq!(findings[0].severity, Severity::Info);
    }

    #[test]
    fn test_mob_score_poor_recommendation() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, None);
        let findings = MobileFriendlinessScoreAnalyzer::new().analyze(&ctx);
        // Score is 60 (100 - 40 for missing viewport), which is < 85 but >= 60
        assert!(findings[0].recommendation.contains("minor"));
    }

    #[test]
    fn test_mob_score_good_recommendation() {
        let mut page = make_page("https://example.com");
        page.meta.viewport = Some("width=device-width".to_string());
        let ctx = make_ctx(&page, None);
        let findings = MobileFriendlinessScoreAnalyzer::new().analyze(&ctx);
        assert!(findings[0].recommendation.contains("mobile-friendly"));
    }

    #[test]
    fn test_mob_score_all_issues() {
        let mut page = make_page("https://example.com");
        page.has_lang_attribute = true;
        let body = r#"<html><head><meta name="viewport" content="width=320, user-scalable=0"></head><body></body></html>"#;
        let ctx = make_ctx_full(&page, Some(body), &[]);
        let findings = MobileFriendlinessScoreAnalyzer::new().analyze(&ctx);
        // 100 - 40 (no viewport in meta) - 20 (fixed width in body) - 15 (user-scalable) = 25
        assert!(findings[0].description.contains("25/100"));
    }

    #[test]
    fn test_mob_score_one_finding() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, None);
        assert_eq!(
            MobileFriendlinessScoreAnalyzer::new().analyze(&ctx).len(),
            1
        );
    }

    // --- ArticleAuthorValidator tests ---
    #[test]
    fn test_content_article_author_missing() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({"@type": "Article", "headline": "Test"}),
        }];
        let ctx = make_ctx(&page, None);
        assert!(ArticleAuthorValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "ART-AUTH001"));
    }

    #[test]
    fn test_content_article_author_present() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({"@type": "Article", "author": "Joe"}),
        }];
        let ctx = make_ctx(&page, None);
        assert!(ArticleAuthorValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_content_article_author_news() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("NewsArticle".to_string()),
            data: serde_json::json!({"@type": "NewsArticle"}),
        }];
        let ctx = make_ctx(&page, None);
        assert!(ArticleAuthorValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "ART-AUTH001"));
    }

    #[test]
    fn test_content_article_author_no_sd() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, None);
        assert!(ArticleAuthorValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_content_article_author_non_article() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product"}),
        }];
        let ctx = make_ctx(&page, None);
        assert!(ArticleAuthorValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_content_article_author_default() {
        let _ = ArticleAuthorValidator::default();
    }

    #[test]
    fn test_content_article_author_name() {
        assert_eq!(ArticleAuthorValidator::new().name(), "article-author");
    }

    #[test]
    fn test_content_article_author_category() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({"@type": "Article"}),
        }];
        let ctx = make_ctx(&page, None);
        for f in ArticleAuthorValidator::new().analyze(&ctx) {
            assert_eq!(f.category, IssueCategory::Content);
        }
    }

    #[test]
    fn test_content_article_author_severity() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({"@type": "Article"}),
        }];
        let ctx = make_ctx(&page, None);
        assert_eq!(
            ArticleAuthorValidator::new().analyze(&ctx)[0].severity,
            Severity::Error
        );
    }

    #[test]
    fn test_content_article_author_multiple() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Article".to_string()),
                data: serde_json::json!({"@type": "Article", "author": "A"}),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Article".to_string()),
                data: serde_json::json!({"@type": "Article"}),
            },
        ];
        let ctx = make_ctx(&page, None);
        assert!(!ArticleAuthorValidator::new().analyze(&ctx).is_empty());
    }

    // --- ArticleDatePublishedValidator tests ---
    #[test]
    fn test_content_article_date_missing() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({"@type": "Article"}),
        }];
        let ctx = make_ctx(&page, None);
        assert!(ArticleDatePublishedValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "ART-DT001"));
    }

    #[test]
    fn test_content_article_date_present() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({"@type": "Article", "datePublished": "2024-01-01"}),
        }];
        let ctx = make_ctx(&page, None);
        assert!(ArticleDatePublishedValidator::new()
            .analyze(&ctx)
            .is_empty());
    }

    #[test]
    fn test_content_article_date_no_sd() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, None);
        assert!(ArticleDatePublishedValidator::new()
            .analyze(&ctx)
            .is_empty());
    }

    #[test]
    fn test_content_article_date_default() {
        let _ = ArticleDatePublishedValidator::default();
    }

    #[test]
    fn test_content_article_date_name() {
        assert_eq!(
            ArticleDatePublishedValidator::new().name(),
            "article-date-published"
        );
    }

    #[test]
    fn test_content_article_date_category() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({"@type": "Article"}),
        }];
        let ctx = make_ctx(&page, None);
        for f in ArticleDatePublishedValidator::new().analyze(&ctx) {
            assert_eq!(f.category, IssueCategory::Content);
        }
    }

    #[test]
    fn test_content_article_date_severity() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({"@type": "Article"}),
        }];
        let ctx = make_ctx(&page, None);
        assert_eq!(
            ArticleDatePublishedValidator::new().analyze(&ctx)[0].severity,
            Severity::Error
        );
    }

    #[test]
    fn test_content_article_date_blog() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("BlogPosting".to_string()),
            data: serde_json::json!({"@type": "BlogPosting"}),
        }];
        let ctx = make_ctx(&page, None);
        assert!(ArticleDatePublishedValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "ART-DT001"));
    }

    #[test]
    fn test_content_article_date_non_article() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product"}),
        }];
        let ctx = make_ctx(&page, None);
        assert!(ArticleDatePublishedValidator::new()
            .analyze(&ctx)
            .is_empty());
    }

    // --- ArticleHeadlineValidator tests ---
    #[test]
    fn test_content_article_headline_missing() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({"@type": "Article"}),
        }];
        let ctx = make_ctx(&page, None);
        assert!(ArticleHeadlineValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "ART-HL001"));
    }

    #[test]
    fn test_content_article_headline_present() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({"@type": "Article", "headline": "Test"}),
        }];
        let ctx = make_ctx(&page, None);
        assert!(ArticleHeadlineValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_content_article_headline_no_sd() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, None);
        assert!(ArticleHeadlineValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_content_article_headline_default() {
        let _ = ArticleHeadlineValidator::default();
    }

    #[test]
    fn test_content_article_headline_name() {
        assert_eq!(ArticleHeadlineValidator::new().name(), "article-headline");
    }

    #[test]
    fn test_content_article_headline_category() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({"@type": "Article"}),
        }];
        let ctx = make_ctx(&page, None);
        for f in ArticleHeadlineValidator::new().analyze(&ctx) {
            assert_eq!(f.category, IssueCategory::Content);
        }
    }

    #[test]
    fn test_content_article_headline_severity() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({"@type": "Article"}),
        }];
        let ctx = make_ctx(&page, None);
        assert_eq!(
            ArticleHeadlineValidator::new().analyze(&ctx)[0].severity,
            Severity::Error
        );
    }

    #[test]
    fn test_content_article_headline_non_article() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product"}),
        }];
        let ctx = make_ctx(&page, None);
        assert!(ArticleHeadlineValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_content_article_headline_multiple() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Article".to_string()),
                data: serde_json::json!({"@type": "Article", "headline": "OK"}),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Article".to_string()),
                data: serde_json::json!({"@type": "Article"}),
            },
        ];
        let ctx = make_ctx(&page, None);
        assert!(!ArticleHeadlineValidator::new().analyze(&ctx).is_empty());
    }

    // --- OrganizationNameValidator tests ---
    #[test]
    fn test_content_org_name_missing() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Organization".to_string()),
            data: serde_json::json!({"@type": "Organization"}),
        }];
        let ctx = make_ctx(&page, None);
        assert!(OrganizationNameValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "ORG-NAME001"));
    }

    #[test]
    fn test_content_org_name_present() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Organization".to_string()),
            data: serde_json::json!({"@type": "Organization", "name": "Acme"}),
        }];
        let ctx = make_ctx(&page, None);
        assert!(OrganizationNameValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_content_org_name_no_sd() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, None);
        assert!(OrganizationNameValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_content_org_name_default() {
        let _ = OrganizationNameValidator::default();
    }

    #[test]
    fn test_content_org_name_name() {
        assert_eq!(OrganizationNameValidator::new().name(), "organization-name");
    }

    #[test]
    fn test_content_org_name_empty() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Organization".to_string()),
            data: serde_json::json!({"@type": "Organization", "name": ""}),
        }];
        let ctx = make_ctx(&page, None);
        assert!(OrganizationNameValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "ORG-NAME001"));
    }

    #[test]
    fn test_content_org_name_category() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Organization".to_string()),
            data: serde_json::json!({"@type": "Organization"}),
        }];
        let ctx = make_ctx(&page, None);
        for f in OrganizationNameValidator::new().analyze(&ctx) {
            assert_eq!(f.category, IssueCategory::Content);
        }
    }

    #[test]
    fn test_content_org_name_severity() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Organization".to_string()),
            data: serde_json::json!({"@type": "Organization"}),
        }];
        let ctx = make_ctx(&page, None);
        assert_eq!(
            OrganizationNameValidator::new().analyze(&ctx)[0].severity,
            Severity::Warning
        );
    }

    #[test]
    fn test_content_org_name_non_org() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product"}),
        }];
        let ctx = make_ctx(&page, None);
        assert!(OrganizationNameValidator::new().analyze(&ctx).is_empty());
    }

    // --- PersonNameValidator tests ---
    #[test]
    fn test_content_person_name_missing() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Person".to_string()),
            data: serde_json::json!({"@type": "Person"}),
        }];
        let ctx = make_ctx(&page, None);
        assert!(PersonNameValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "PERS-NAME001"));
    }

    #[test]
    fn test_content_person_name_present() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Person".to_string()),
            data: serde_json::json!({"@type": "Person", "name": "Jane"}),
        }];
        let ctx = make_ctx(&page, None);
        assert!(PersonNameValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_content_person_name_no_sd() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, None);
        assert!(PersonNameValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_content_person_name_default() {
        let _ = PersonNameValidator::default();
    }

    #[test]
    fn test_content_person_name_name() {
        assert_eq!(PersonNameValidator::new().name(), "person-name");
    }

    #[test]
    fn test_content_person_name_empty() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Person".to_string()),
            data: serde_json::json!({"@type": "Person", "name": ""}),
        }];
        let ctx = make_ctx(&page, None);
        assert!(PersonNameValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "PERS-NAME001"));
    }

    #[test]
    fn test_content_person_name_category() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Person".to_string()),
            data: serde_json::json!({"@type": "Person"}),
        }];
        let ctx = make_ctx(&page, None);
        for f in PersonNameValidator::new().analyze(&ctx) {
            assert_eq!(f.category, IssueCategory::Content);
        }
    }

    #[test]
    fn test_content_person_name_severity() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Person".to_string()),
            data: serde_json::json!({"@type": "Person"}),
        }];
        let ctx = make_ctx(&page, None);
        assert_eq!(
            PersonNameValidator::new().analyze(&ctx)[0].severity,
            Severity::Warning
        );
    }

    #[test]
    fn test_content_person_name_non_person() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product"}),
        }];
        let ctx = make_ctx(&page, None);
        assert!(PersonNameValidator::new().analyze(&ctx).is_empty());
    }

    // --- JobPostingTitleValidator tests ---
    #[test]
    fn test_content_job_title_missing() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("JobPosting".to_string()),
            data: serde_json::json!({"@type": "JobPosting"}),
        }];
        let ctx = make_ctx(&page, None);
        assert!(JobPostingTitleValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "JOB-TITLE001"));
    }

    #[test]
    fn test_content_job_title_present() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("JobPosting".to_string()),
            data: serde_json::json!({"@type": "JobPosting", "title": "Engineer"}),
        }];
        let ctx = make_ctx(&page, None);
        assert!(JobPostingTitleValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_content_job_title_no_sd() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, None);
        assert!(JobPostingTitleValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_content_job_title_default() {
        let _ = JobPostingTitleValidator::default();
    }

    #[test]
    fn test_content_job_title_name() {
        assert_eq!(JobPostingTitleValidator::new().name(), "job-posting-title");
    }

    #[test]
    fn test_content_job_title_category() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("JobPosting".to_string()),
            data: serde_json::json!({"@type": "JobPosting"}),
        }];
        let ctx = make_ctx(&page, None);
        for f in JobPostingTitleValidator::new().analyze(&ctx) {
            assert_eq!(f.category, IssueCategory::Content);
        }
    }

    #[test]
    fn test_content_job_title_severity() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("JobPosting".to_string()),
            data: serde_json::json!({"@type": "JobPosting"}),
        }];
        let ctx = make_ctx(&page, None);
        assert_eq!(
            JobPostingTitleValidator::new().analyze(&ctx)[0].severity,
            Severity::Error
        );
    }

    #[test]
    fn test_content_job_title_non_job() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product"}),
        }];
        let ctx = make_ctx(&page, None);
        assert!(JobPostingTitleValidator::new().analyze(&ctx).is_empty());
    }

    // --- CourseNameValidator tests ---
    #[test]
    fn test_content_course_name_missing() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Course".to_string()),
            data: serde_json::json!({"@type": "Course"}),
        }];
        let ctx = make_ctx(&page, None);
        assert!(CourseNameValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "COURSE-NAME001"));
    }

    #[test]
    fn test_content_course_name_present() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Course".to_string()),
            data: serde_json::json!({"@type": "Course", "name": "Rust 101"}),
        }];
        let ctx = make_ctx(&page, None);
        assert!(CourseNameValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_content_course_name_no_sd() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, None);
        assert!(CourseNameValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_content_course_name_default() {
        let _ = CourseNameValidator::default();
    }

    #[test]
    fn test_content_course_name_name() {
        assert_eq!(CourseNameValidator::new().name(), "course-name");
    }

    #[test]
    fn test_content_course_name_empty() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Course".to_string()),
            data: serde_json::json!({"@type": "Course", "name": ""}),
        }];
        let ctx = make_ctx(&page, None);
        assert!(CourseNameValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "COURSE-NAME001"));
    }

    #[test]
    fn test_content_course_name_category() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Course".to_string()),
            data: serde_json::json!({"@type": "Course"}),
        }];
        let ctx = make_ctx(&page, None);
        for f in CourseNameValidator::new().analyze(&ctx) {
            assert_eq!(f.category, IssueCategory::Content);
        }
    }

    #[test]
    fn test_content_course_name_severity() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Course".to_string()),
            data: serde_json::json!({"@type": "Course"}),
        }];
        let ctx = make_ctx(&page, None);
        assert_eq!(
            CourseNameValidator::new().analyze(&ctx)[0].severity,
            Severity::Error
        );
    }

    #[test]
    fn test_content_course_name_non_course() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product"}),
        }];
        let ctx = make_ctx(&page, None);
        assert!(CourseNameValidator::new().analyze(&ctx).is_empty());
    }

    // --- RecipeNameValidator tests ---
    #[test]
    fn test_content_recipe_name_missing() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Recipe".to_string()),
            data: serde_json::json!({"@type": "Recipe"}),
        }];
        let ctx = make_ctx(&page, None);
        assert!(RecipeNameValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "RECIPE-NAME001"));
    }

    #[test]
    fn test_content_recipe_name_present() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Recipe".to_string()),
            data: serde_json::json!({"@type": "Recipe", "name": "Cake"}),
        }];
        let ctx = make_ctx(&page, None);
        assert!(RecipeNameValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_content_recipe_name_no_sd() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, None);
        assert!(RecipeNameValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_content_recipe_name_default() {
        let _ = RecipeNameValidator::default();
    }

    #[test]
    fn test_content_recipe_name_name() {
        assert_eq!(RecipeNameValidator::new().name(), "recipe-name");
    }

    #[test]
    fn test_content_recipe_name_empty() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Recipe".to_string()),
            data: serde_json::json!({"@type": "Recipe", "name": ""}),
        }];
        let ctx = make_ctx(&page, None);
        assert!(RecipeNameValidator::new()
            .analyze(&ctx)
            .iter()
            .any(|f| f.code == "RECIPE-NAME001"));
    }

    #[test]
    fn test_content_recipe_name_category() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Recipe".to_string()),
            data: serde_json::json!({"@type": "Recipe"}),
        }];
        let ctx = make_ctx(&page, None);
        for f in RecipeNameValidator::new().analyze(&ctx) {
            assert_eq!(f.category, IssueCategory::Content);
        }
    }

    #[test]
    fn test_content_recipe_name_severity() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Recipe".to_string()),
            data: serde_json::json!({"@type": "Recipe"}),
        }];
        let ctx = make_ctx(&page, None);
        assert_eq!(
            RecipeNameValidator::new().analyze(&ctx)[0].severity,
            Severity::Error
        );
    }

    #[test]
    fn test_content_recipe_name_non_recipe() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product"}),
        }];
        let ctx = make_ctx(&page, None);
        assert!(RecipeNameValidator::new().analyze(&ctx).is_empty());
    }
}
