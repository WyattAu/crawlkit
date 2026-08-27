#![allow(clippy::unwrap_used, clippy::manual_range_contains, clippy::redundant_closure, clippy::collapsible_if, clippy::unnecessary_map_or, clippy::default_constructed_unit_structs, clippy::needless_return)]
use std::collections::HashMap;

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
        lower.contains("vocab=")
            || lower.contains("typeof=")
            || lower.contains("property=")
            || lower.contains("about=")
            || lower.contains("resource=")
    }

    #[allow(clippy::unwrap_used)]
    fn extract_vocabs(body: &str) -> Vec<String> {
        let mut vocabs = Vec::new();
        if let Some(caps) = Regex::new(r#"(?i)vocab\s*=\s*["']([^"']+)["']"#)
            .unwrap()
            .captures_iter(body)
            .next()
        {
            vocabs.push(caps[1].to_string());
        }
        vocabs
    }

    #[allow(clippy::unwrap_used)]
    fn extract_typeofs(body: &str) -> Vec<String> {
        let mut types = Vec::new();
        for caps in Regex::new(r#"(?i)typeof\s*=\s*["']([^"']+)["']"#)
            .unwrap()
            .captures_iter(body)
        {
            types.push(caps[1].to_string());
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

    #[allow(clippy::unwrap_used)]
    fn extract_itemtype(body: &str) -> Vec<String> {
        let mut types = Vec::new();
        for caps in Regex::new(r#"(?i)itemtype\s*=\s*["']([^"']+)["']"#)
            .unwrap()
            .captures_iter(body)
        {
            types.push(caps[1].to_string());
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

pub struct ShippingSchemaValidator;

impl ShippingSchemaValidator {
    pub fn new() -> Self {
        Self
    }

    fn extract_product_schemas<'a>(ctx: &'a AnalysisContext<'a>) -> Vec<&'a serde_json::Value> {
        ctx.page
            .structured_data
            .iter()
            .filter(|sd| sd.r#type.as_deref() == Some("Product"))
            .map(|sd| &sd.data)
            .collect()
    }

    fn has_offers(data: &serde_json::Value) -> bool {
        match data.get("offers") {
            None => false,
            Some(v) => {
                if let Some(arr) = v.as_array() {
                    !arr.is_empty()
                } else if let Some(obj) = v.as_object() {
                    obj.get("@type")
                        .and_then(|t| t.as_str())
                        .map(|t| t == "Offer")
                        .unwrap_or(false)
                        || obj.get("price").is_some()
                } else {
                    false
                }
            }
        }
    }

    fn has_shipping_details(data: &serde_json::Value) -> bool {
        if data.get("hasShippingDetails").is_some() {
            return true;
        }
        if let Some(offers) = data.get("offers").and_then(|v| v.as_array()) {
            return offers
                .iter()
                .any(|o| o.get("hasShippingDetails").is_some());
        }
        if let Some(offers) = data.get("offers") {
            if let Some(obj) = offers.as_object() {
                return obj.get("hasShippingDetails").is_some();
            }
        }
        false
    }
}

impl Default for ShippingSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for ShippingSchemaValidator {
    fn name(&self) -> &str {
        "shipping-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for data in Self::extract_product_schemas(ctx) {
            if Self::has_offers(data) && !Self::has_shipping_details(data) {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "SHIP001".to_string(),
                    title: "Product has offers but no ShippingDetails".to_string(),
                    description: "A Product schema has offers but no hasShippingDetails property. \
                                  Shipping details help search engines display delivery information \
                                  in product search results."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add a \"hasShippingDetails\" property to the Product or Offer \
                                     schema with shipping cost, delivery time, and destination."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// Offer Availability Analyzer
// ---------------------------------------------------------------------------

pub struct OfferAvailabilityAnalyzer;

impl OfferAvailabilityAnalyzer {
    pub fn new() -> Self {
        Self
    }

    fn extract_product_schemas<'a>(ctx: &'a AnalysisContext<'a>) -> Vec<&'a serde_json::Value> {
        ctx.page
            .structured_data
            .iter()
            .filter(|sd| sd.r#type.as_deref() == Some("Product"))
            .map(|sd| &sd.data)
            .collect()
    }

    fn get_schema_availability(data: &serde_json::Value) -> Option<String> {
        if let Some(offers) = data.get("offers") {
            if let Some(arr) = offers.as_array() {
                if let Some(first) = arr.first() {
                    return first
                        .get("availability")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                }
            } else if let Some(obj) = offers.as_object() {
                return obj
                    .get("availability")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
            }
        }
        None
    }

    fn page_says_out_of_stock(body: &str) -> bool {
        let lower = body.to_lowercase();
        [
            "out of stock",
            "out-of-stock",
            "sold out",
            "currently unavailable",
            "not available",
            "no longer available",
            "temporarily out of stock",
        ]
        .iter()
        .any(|&ind| lower.contains(ind))
    }

    fn page_says_in_stock(body: &str) -> bool {
        let lower = body.to_lowercase();
        [
            "add to cart",
            "add to bag",
            "buy now",
            "in stock",
            "ships in",
            "delivery",
            "available",
        ]
        .iter()
        .any(|&ind| lower.contains(ind))
    }
}

impl Default for OfferAvailabilityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for OfferAvailabilityAnalyzer {
    fn name(&self) -> &str {
        "offer-availability"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        let body = match ctx.body {
            Some(b) => b,
            None => return findings,
        };

        for data in Self::extract_product_schemas(ctx) {
            if let Some(availability) = Self::get_schema_availability(data) {
                let lower_avail = availability.to_lowercase();

                if lower_avail.contains("instock") && Self::page_says_out_of_stock(body) {
                    findings.push(Finding {
                        severity: Severity::Error,
                        category: IssueCategory::Schema,
                        code: "AVAIL001".to_string(),
                        title: "Schema says InStock but page says out of stock".to_string(),
                        description: format!(
                            "Product schema availability is \"{availability}\" but the page \
                             text contains out-of-stock indicators."
                        ),
                        url: url.clone(),
                        recommendation: "Update the schema availability to match the actual \
                                         page content. Mismatched availability confuses search \
                                         engines and users."
                            .to_string(),
                    });
                }

                if lower_avail.contains("outofstock") && Self::page_says_in_stock(body) {
                    findings.push(Finding {
                        severity: Severity::Error,
                        category: IssueCategory::Schema,
                        code: "AVAIL002".to_string(),
                        title: "Schema says OutOfStock but page says in stock".to_string(),
                        description: format!(
                            "Product schema availability is \"{availability}\" but the page \
                             text contains in-stock indicators."
                        ),
                        url: url.clone(),
                        recommendation: "Update the schema availability to match the actual \
                                         page content. Mismatched availability confuses search \
                                         engines and users."
                            .to_string(),
                    });
                }
            }
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// Coupon Schema Validator
// ---------------------------------------------------------------------------

pub struct CouponSchemaValidator;

impl CouponSchemaValidator {
    pub fn new() -> Self {
        Self
    }

    fn extract_coupon_schemas<'a>(ctx: &'a AnalysisContext<'a>) -> Vec<&'a serde_json::Value> {
        ctx.page
            .structured_data
            .iter()
            .filter(|sd| sd.r#type.as_deref() == Some("Coupon"))
            .map(|sd| &sd.data)
            .collect()
    }
}

impl Default for CouponSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for CouponSchemaValidator {
    fn name(&self) -> &str {
        "coupon-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for data in Self::extract_coupon_schemas(ctx) {
            if data.get("validFrom").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "COUP001".to_string(),
                    title: "Coupon schema missing validFrom".to_string(),
                    description: "A Coupon schema was found but has no validFrom property. \
                                  The validFrom date tells search engines when the coupon \
                                  becomes active."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add a \"validFrom\" property with an ISO 8601 date to \
                                     the Coupon schema."
                        .to_string(),
                });
            }

            let has_discount_percentage = data.get("discountPercentage").is_some();
            let has_discount_amount = data.get("discount").is_some();
            if !has_discount_percentage && !has_discount_amount {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Schema,
                    code: "COUP002".to_string(),
                    title: "Coupon schema missing discount information".to_string(),
                    description: "A Coupon schema was found but has neither discountPercentage \
                                  nor discount. Search engines need at least one discount \
                                  value to display coupon information in search results."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add a \"discountPercentage\" or \"discount\" property to \
                                     the Coupon schema."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// BreadcrumbsValidator
// =========================================================================

/// Validates BreadcrumbList structured data completeness and consistency.
///
/// Checks for incomplete BreadcrumbList schema, breadcrumb URLs that
/// don't match the page hierarchy, and missing breadcrumbs on deep pages.
pub struct BreadcrumbsValidator;

impl Default for BreadcrumbsValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl BreadcrumbsValidator {
    pub fn new() -> Self {
        Self
    }

    fn find_breadcrumb_schema(data: &serde_json::Value) -> Option<&serde_json::Value> {
        let schemas = data.get("@graph");
        if let Some(graph) = schemas.and_then(|g| g.as_array()) {
            for item in graph {
                if item.get("@type").and_then(|t| t.as_str()) == Some("BreadcrumbList") {
                    return Some(item);
                }
            }
        }
        if data.get("@type").and_then(|t| t.as_str()) == Some("BreadcrumbList") {
            return Some(data);
        }
        None
    }
}

impl Analyzer for BreadcrumbsValidator {
    fn name(&self) -> &str {
        "breadcrumbs-validator"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if let Some(breadcrumb) = Self::find_breadcrumb_schema(&sd.data) {
                // BREAD001: BreadcrumbList present but empty or single item
                if let Some(items) = breadcrumb.get("itemListElement").and_then(|i| i.as_array()) {
                    if items.len() <= 1 {
                        findings.push(Finding {
                            severity: Severity::Warning,
                            category: IssueCategory::Content,
                            code: "BREAD001".to_string(),
                            title: "BreadcrumbList has too few items".to_string(),
                            description: format!(
                                "BreadcrumbList contains only {} item(s). A complete breadcrumb \
                                 trail should have at least 2 items (home + current page).",
                                items.len()
                            ),
                            url: url.clone(),
                            recommendation: "Add all intermediate pages to the BreadcrumbList \
                                             schema to help search engines understand your site hierarchy."
                                .to_string(),
                        });
                    }

                    // BREAD002: Breadcrumb URLs don't match page hierarchy
                    if let Some(last_item) = items.last() {
                        if let Some(item_url) = last_item.get("item").and_then(|i| i.as_str()) {
                            let page_path = url.trim_start_matches("https://")
                                .trim_start_matches("http://")
                                .trim_start_matches(|c: char| c.is_alphanumeric());
                            if !item_url.contains(page_path) && !page_path.is_empty() {
                                findings.push(Finding {
                                    severity: Severity::Info,
                                    category: IssueCategory::Content,
                                    code: "BREAD002".to_string(),
                                    title: "Breadcrumb URL doesn't match page URL".to_string(),
                                    description: format!(
                                        "The last breadcrumb item points to \"{}\" but the current \
                                         page URL is \"{}\".",
                                        item_url, url
                                    ),
                                    url: url.clone(),
                                    recommendation: "Ensure the last breadcrumb item's URL matches \
                                                     the current page URL."
                                        .to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }

        // BREAD003: No BreadcrumbList on deep pages (depth > 2)
        let path_segments: Vec<&str> = url.split('/').filter(|s| !s.is_empty() && !s.contains(':')).collect();
        if path_segments.len() > 2 {
            let has_breadcrumb = ctx.page.structured_data.iter().any(|sd| {
                sd.data.get("@type").and_then(|t| t.as_str()) == Some("BreadcrumbList")
            });
            if !has_breadcrumb {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Content,
                    code: "BREAD003".to_string(),
                    title: "Deep page missing BreadcrumbList schema".to_string(),
                    description: format!(
                        "This page is {} levels deep but has no BreadcrumbList structured data. \
                         Breadcrumbs help search engines understand site hierarchy.",
                        path_segments.len()
                    ),
                    url: url.clone(),
                    recommendation: "Add a BreadcrumbList schema showing the full navigation \
                                     path from home to this page."
                        .to_string(),
                });
            }
        }

        findings
    }
}

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

/// Validates Event structured data for completeness.
pub struct EventSchemaValidator;

impl Default for EventSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl EventSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for EventSchemaValidator {
    fn name(&self) -> &str {
        "event-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Event") {
                continue;
            }
            let data = &sd.data;

            // EVENT001: Missing startDate
            if data.get("startDate").is_none() {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Schema,
                    code: "EVENT001".to_string(),
                    title: "Event schema missing startDate".to_string(),
                    description: "An Event structured data block is missing the required \
                                 \"startDate\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"startDate\" with an ISO 8601 date/time value."
                        .to_string(),
                });
            }

            // EVENT002: Missing location
            if data.get("location").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "EVENT002".to_string(),
                    title: "Event schema missing location".to_string(),
                    description: "An Event structured data block is missing the \"location\" \
                                 property. This may reduce eligibility for rich results."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"location\" with a Place or VirtualLocation object."
                        .to_string(),
                });
            }

            // EVENT003: Missing organizer
            if data.get("organizer").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "EVENT003".to_string(),
                    title: "Event schema missing organizer".to_string(),
                    description: "An Event structured data block is missing the \"organizer\" \
                                 property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"organizer\" with a Person or Organization object."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// ReviewSchemaValidator
// =========================================================================

/// Validates Review and AggregateRating structured data.
pub struct ReviewSchemaValidator;

impl Default for ReviewSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ReviewSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for ReviewSchemaValidator {
    fn name(&self) -> &str {
        "review-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            let data = &sd.data;
            let schema_type = data.get("@type").and_then(|t| t.as_str());

            // Check AggregateRating
            if schema_type == Some("AggregateRating") || schema_type == Some("Product") {
                if let Some(rating) = data.get("aggregateRating") {
                    // REV001: Missing reviewCount or ratingCount
                    if rating.get("reviewCount").is_none() && rating.get("ratingCount").is_none() {
                        findings.push(Finding {
                            severity: Severity::Error,
                            category: IssueCategory::Schema,
                            code: "REV001".to_string(),
                            title: "AggregateRating missing reviewCount".to_string(),
                            description: "AggregateRating schema is missing both \"reviewCount\" \
                                         and \"ratingCount\" properties."
                                .to_string(),
                            url: url.clone(),
                            recommendation: "Add \"reviewCount\" or \"ratingCount\" to the \
                                             AggregateRating schema."
                                .to_string(),
                        });
                    }

                    // REV002: ratingValue out of range
                    if let Some(value) = rating.get("ratingValue").and_then(|v| v.as_f64()) {
                        let best = rating
                            .get("bestRating")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(5.0);
                        if value > best || value < 0.0 {
                            findings.push(Finding {
                                severity: Severity::Error,
                                category: IssueCategory::Schema,
                                code: "REV002".to_string(),
                                title: "AggregateRating ratingValue out of range".to_string(),
                                description: format!(
                                    "ratingValue ({}) is outside the valid range (0 to {}).",
                                    value, best
                                ),
                                url: url.clone(),
                                recommendation: "Ensure ratingValue is between 0 and bestRating."
                                    .to_string(),
                            });
                        }
                    }
                }
            }

            // Check Review
            if schema_type == Some("Review") {
                // REV003: Missing author
                if data.get("author").is_none() {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Schema,
                        code: "REV003".to_string(),
                        title: "Review schema missing author".to_string(),
                        description: "A Review structured data block is missing the \"author\" \
                                     property."
                            .to_string(),
                        url: url.clone(),
                        recommendation: "Add \"author\" with a Person or Organization object."
                            .to_string(),
                    });
                }
            }
        }

        findings
    }
}

// =========================================================================
// VideoSchemaValidator
// =========================================================================

/// Validates VideoObject structured data.
pub struct VideoSchemaValidator;

impl Default for VideoSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl VideoSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for VideoSchemaValidator {
    fn name(&self) -> &str {
        "video-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("VideoObject") {
                continue;
            }
            let data = &sd.data;

            // VID001: Missing embedUrl
            if data.get("embedUrl").is_none() {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Schema,
                    code: "VID001".to_string(),
                    title: "VideoObject missing embedUrl".to_string(),
                    description: "A VideoObject structured data block is missing the required \
                                 \"embedUrl\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"embedUrl\" with the URL of the embedded video player."
                        .to_string(),
                });
            }

            // VID002: Missing thumbnailUrl
            if data.get("thumbnailUrl").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "VID002".to_string(),
                    title: "VideoObject missing thumbnailUrl".to_string(),
                    description: "A VideoObject structured data block is missing the \
                                 \"thumbnailUrl\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"thumbnailUrl\" with a URL to the video thumbnail image."
                        .to_string(),
                });
            }

            // VID003: Missing duration
            if data.get("duration").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "VID003".to_string(),
                    title: "VideoObject missing duration".to_string(),
                    description: "A VideoObject structured data block is missing the \
                                 \"duration\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"duration\" with an ISO 8601 duration value (e.g., PT1H30M)."
                        .to_string(),
                });
            }
        }

        findings
    }
}

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
            let has_toc_html = ctx.body.is_some_and(|body| {
                body.contains("<nav") && body.contains("href=\"#")
            }) || ctx.body.is_some_and(|body| {
                body.contains("<ol") && body.contains("href=\"#")
            });

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

/// Validates LocalBusiness and subtype schemas for NAP consistency.
pub struct LocalBusinessSchemaValidator;

impl Default for LocalBusinessSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl LocalBusinessSchemaValidator {
    pub fn new() -> Self {
        Self
    }

    const LOCAL_BUSINESS_TYPES: &[&str] = &[
        "LocalBusiness", "Store", "Restaurant", "MedicalBusiness",
        "FinancialService", "TravelAgency", "AutoBodyShop", "AutoDealer",
        "AutoPartsStore", "AutoRental", "AutoRepair", "Bakery", "BarOrPub",
        "BeautySalon", "Brewery", "CafeOrCoffeeShop", "Cemetery",
        "ChildCare", "Dentist", "EmploymentAgency", "EntertainmentBusiness",
        "FinancialService", "FoodEstablishment", "GardenStore",
        "GovernmentOffice", "HealthAndBeautyBusiness", "HomeAndConstructionBusiness",
        "InsuranceAgency", "InternetCafe", "LegalService", "Library",
        "LodgingBusiness", "ManisBusiness", "MovieRentalStore", "MovingCompany",
        "MusicStore", "OfficeEquipmentStore", "OutletStore", "PawnShop",
        "PetStore", "Physician", "Plumber", "RealEstateAgent",
        "RecyclingCenter", "SelfStorage", "ShoeStore", "ShoppingCenter",
        "SportingGoodsStore", "TattooParlor", "TelevisionStation",
        "ToyStore", "TravelAgency", "WholesaleStore",
    ];
}

impl Analyzer for LocalBusinessSchemaValidator {
    fn name(&self) -> &str {
        "local-business-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            let schema_type = sd.data.get("@type").and_then(|t| t.as_str()).unwrap_or("");
            if !Self::LOCAL_BUSINESS_TYPES.contains(&schema_type) {
                continue;
            }

            // LBIZ001: Missing name
            if sd.data.get("name").is_none() {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Schema,
                    code: "LBIZ001".to_string(),
                    title: "LocalBusiness schema missing name".to_string(),
                    description: format!(
                        "A {} schema is missing the required \"name\" property.",
                        schema_type
                    ),
                    url: url.clone(),
                    recommendation: "Add \"name\" with the business name."
                        .to_string(),
                });
            }

            // LBIZ002: Missing address
            if sd.data.get("address").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "LBIZ002".to_string(),
                    title: "LocalBusiness schema missing address".to_string(),
                    description: format!(
                        "A {} schema is missing the \"address\" property.",
                        schema_type
                    ),
                    url: url.clone(),
                    recommendation: "Add \"address\" with a PostalAddress object."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// LocalBusinessNapAnalyzer
// =========================================================================

/// Validates LocalBusiness NAP (Name, Address, Phone) consistency.
pub struct LocalBusinessNapAnalyzer;

impl Default for LocalBusinessNapAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl LocalBusinessNapAnalyzer {
    pub fn new() -> Self {
        Self
    }

    const LOCAL_BUSINESS_TYPES: &[&str] = &[
        "LocalBusiness", "Store", "Restaurant", "MedicalBusiness",
        "FinancialService", "TravelAgency", "AutoBodyShop", "AutoDealer",
        "AutoPartsStore", "AutoRental", "AutoRepair", "Bakery", "BarOrPub",
        "BeautySalon", "Brewery", "CafeOrCoffeeShop", "Cemetery",
        "ChildCare", "Dentist", "EmploymentAgency", "EntertainmentBusiness",
        "FoodEstablishment", "GardenStore",
        "GovernmentOffice", "HealthAndBeautyBusiness", "HomeAndConstructionBusiness",
        "InsuranceAgency", "InternetCafe", "LegalService", "Library",
        "LodgingBusiness", "MovingCompany",
        "MusicStore", "OfficeEquipmentStore", "OutletStore", "PawnShop",
        "PetStore", "Physician", "Plumber", "RealEstateAgent",
        "RecyclingCenter", "SelfStorage", "ShoeStore", "ShoppingCenter",
        "SportingGoodsStore", "TattooParlor", "TelevisionStation",
        "ToyStore", "WholesaleStore",
    ];
}

impl Analyzer for LocalBusinessNapAnalyzer {
    fn name(&self) -> &str {
        "local-business-nap"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            let schema_type = sd.data.get("@type").and_then(|t| t.as_str()).unwrap_or("");
            if !Self::LOCAL_BUSINESS_TYPES.contains(&schema_type) {
                continue;
            }

            // NAP001: Missing telephone
            if sd.data.get("telephone").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "NAP001".to_string(),
                    title: "LocalBusiness schema missing telephone".to_string(),
                    description: format!(
                        "A {} schema is missing the \"telephone\" property. Phone numbers are \
                         essential for NAP consistency and local SEO."
                    ,
                        schema_type
                    ),
                    url: url.clone(),
                    recommendation: "Add \"telephone\" with the business phone number in \
                                     international format (e.g., \"+1-555-555-5555\")."
                        .to_string(),
                });
            }

            // NAP002: Missing openingHours
            if sd.data.get("openingHours").is_none()
                && sd.data.get("openingHoursSpecification").is_none()
            {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "NAP002".to_string(),
                    title: "LocalBusiness schema missing openingHours".to_string(),
                    description: format!(
                        "A {} schema is missing \"openingHours\" or \
                         \"openingHoursSpecification\". Business hours help customers know when \
                         to visit."
                    ,
                        schema_type
                    ),
                    url: url.clone(),
                    recommendation: "Add \"openingHours\" with ISO 8601 time ranges or \
                                     \"openingHoursSpecification\" with OpeningHoursSpecification \
                                     objects."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// FaqSchemaValidator
// =========================================================================

/// Validates FAQPage structured data for completeness.
pub struct FaqSchemaValidator;

impl Default for FaqSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl FaqSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for FaqSchemaValidator {
    fn name(&self) -> &str {
        "faq-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("FAQPage") {
                continue;
            }
            let data = &sd.data;

            // FAQ001: Missing mainEntity
            let main_entity = data.get("mainEntity");
            if main_entity.is_none() {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Schema,
                    code: "FAQ001".to_string(),
                    title: "FAQPage schema missing mainEntity".to_string(),
                    description: "An FAQPage structured data block is missing the required \
                                  \"mainEntity\" property. Without mainEntity, search engines \
                                  cannot extract question-answer pairs."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"mainEntity\" with an array of Question objects."
                        .to_string(),
                });
                continue;
            }

            let main_entity = main_entity.unwrap();

            // FAQ002: mainEntity has fewer than 2 questions
            let questions = main_entity.as_array();
            let question_count = questions.map_or(0, |arr| arr.len());
            if question_count < 2 {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "FAQ002".to_string(),
                    title: "FAQPage schema has fewer than 2 questions".to_string(),
                    description: format!(
                        "FAQPage mainEntity contains only {} question(s). FAQ rich results \
                         typically require at least 2 question-answer pairs.",
                        question_count
                    ),
                    url: url.clone(),
                    recommendation: "Add at least 2 Question objects to the mainEntity array."
                        .to_string(),
                });
            }

            // FAQ003: Questions missing acceptedAnswer
            if let Some(arr) = questions {
                for (i, q) in arr.iter().enumerate() {
                    if q.get("acceptedAnswer").is_none() {
                        findings.push(Finding {
                            severity: Severity::Error,
                            category: IssueCategory::Schema,
                            code: "FAQ003".to_string(),
                            title: "FAQPage question missing acceptedAnswer".to_string(),
                            description: format!(
                                "Question at position {} in FAQPage mainEntity is missing the \
                                 required \"acceptedAnswer\" property.",
                                i + 1
                            ),
                            url: url.clone(),
                            recommendation: "Add \"acceptedAnswer\" with an Answer object to each \
                                             Question in the FAQPage schema."
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
// HowToSchemaValidator
// =========================================================================

/// Validates HowTo structured data for completeness.
pub struct HowToSchemaValidator;

impl Default for HowToSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl HowToSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for HowToSchemaValidator {
    fn name(&self) -> &str {
        "howto-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("HowTo") {
                continue;
            }
            let data = &sd.data;

            // HOWTO001: Missing name
            if data.get("name").is_none() {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Schema,
                    code: "HOWTO001".to_string(),
                    title: "HowTo schema missing name".to_string(),
                    description: "A HowTo structured data block is missing the required \
                                  \"name\" property. The name describes the overall procedure."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with a descriptive title for the how-to guide."
                        .to_string(),
                });
            }

            // HOWTO002: Missing step
            let steps = data.get("step");
            if steps.is_none() {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Schema,
                    code: "HOWTO002".to_string(),
                    title: "HowTo schema missing step".to_string(),
                    description: "A HowTo structured data block is missing the required \
                                  \"step\" property. Steps define the individual actions in the \
                                  how-to procedure."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"step\" with an array of HowToStep objects."
                        .to_string(),
                });
                continue;
            }

            let steps = steps.unwrap();
            let steps_arr = steps.as_array();

            // HOWTO003: Steps missing name or text
            if let Some(arr) = steps_arr {
                for (i, step) in arr.iter().enumerate() {
                    let has_name = step.get("name").is_some();
                    let has_text = step.get("text").is_some();
                    if !has_name || !has_text {
                        findings.push(Finding {
                            severity: Severity::Warning,
                            category: IssueCategory::Schema,
                            code: "HOWTO003".to_string(),
                            title: "HowTo step missing name or text".to_string(),
                            description: format!(
                                "Step at position {} is missing {}.",
                                i + 1,
                                if !has_name && !has_text {
                                    "both \"name\" and \"text\""
                                } else if !has_name {
                                    "the \"name\" property"
                                } else {
                                    "the \"text\" property"
                                }
                            ),
                            url: url.clone(),
                            recommendation: "Add both \"name\" and \"text\" properties to each \
                                             HowToStep."
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
// SpeakableSchemaValidator
// =========================================================================

/// Validates Speakable structured data for completeness.
pub struct SpeakableSchemaValidator;

impl Default for SpeakableSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl SpeakableSchemaValidator {
    pub fn new() -> Self {
    #[allow(clippy::unwrap_used)]
        Self
    }
}

impl Analyzer for SpeakableSchemaValidator {
    fn name(&self) -> &str {
        "speakable-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            let speakable = sd.data.get("speakable");
            if speakable.is_none() {
                continue;
            }
            let speakable = speakable.unwrap();

            // Handle both object and array forms
            let speakables: Vec<&serde_json::Value> = if let Some(arr) = speakable.as_array() {
                arr.iter().collect()
            } else {
                vec![speakable]
            };

            for s in &speakables {
                let has_xpath = s.get("xpath").is_some();
                let has_css_selector = s.get("cssSelector").is_some();

                // SPEAK001: Speakable present but missing xpath
                if !has_xpath {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Schema,
                        code: "SPEAK001".to_string(),
                        title: "Speakable schema missing xpath".to_string(),
                        description: "A Speakable structured data property is present but does \
                                      not specify an \"xpath\" selector. XPath helps voice \
                                      assistants identify which content to read aloud."
                            .to_string(),
                        url: url.clone(),
                        recommendation: "Add \"xpath\" with an XPath expression pointing to the \
                                         speakable content."
                            .to_string(),
                    });
                }

                // SPEAK002: Speakable present but missing cssSelector
                if !has_css_selector {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Schema,
                        code: "SPEAK002".to_string(),
                        title: "Speakable schema missing cssSelector".to_string(),
                        description: "A Speakable structured data property is present but does \
                                      not specify a \"cssSelector\". CSS selectors provide an \
                                      alternative way to identify speakable content."
                            .to_string(),
                        url: url.clone(),
                        recommendation: "Add \"cssSelector\" with a CSS selector pointing to the \
                                         speakable content."
                            .to_string(),
                    });
                }
            }
        }

        findings
    }
}

// =========================================================================
// DatasetSchemaValidator
// =========================================================================

/// Validates Dataset structured data for completeness.
pub struct DatasetSchemaValidator;

impl Default for DatasetSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl DatasetSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for DatasetSchemaValidator {
    fn name(&self) -> &str {
        "dataset-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Dataset") {
                continue;
            }
            let data = &sd.data;

            // DATA001: Missing name
            if data.get("name").is_none() {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Schema,
                    code: "DATA001".to_string(),
                    title: "Dataset schema missing name".to_string(),
                    description: "A Dataset structured data block is missing the required \
                                  \"name\" property. The name identifies the dataset."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with a descriptive title for the dataset."
                        .to_string(),
                });
            }

            // DATA002: Missing distribution
            if data.get("distribution").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "DATA002".to_string(),
                    title: "Dataset schema missing distribution".to_string(),
                    description: "A Dataset structured data block is missing the \"distribution\" \
                                  property. Distribution specifies how to access the dataset."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"distribution\" with a DataDownload object specifying \
                                     the download URL and format."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// SpecialAnnouncementSchemaValidator
// =========================================================================

/// Validates SpecialAnnouncement structured data for completeness.
pub struct SpecialAnnouncementSchemaValidator;

impl Default for SpecialAnnouncementSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl SpecialAnnouncementSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for SpecialAnnouncementSchemaValidator {
    fn name(&self) -> &str {
        "special-announcement-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("SpecialAnnouncement") {
                continue;
            }
            let data = &sd.data;

            // SPEC001: Missing datePosted
            if data.get("datePosted").is_none() {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Schema,
                    code: "SPEC001".to_string(),
                    title: "SpecialAnnouncement missing datePosted".to_string(),
                    description: "A SpecialAnnouncement structured data block is missing the \
                                  required \"datePosted\" property. The date indicates when the \
                                  announcement was published."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"datePosted\" with an ISO 8601 date value."
                        .to_string(),
                });
            }

            // SPEC002: Missing category
            if data.get("category").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "SPEC002".to_string(),
                    title: "SpecialAnnouncement missing category".to_string(),
                    description: "A SpecialAnnouncement structured data block is missing the \
                                  \"category\" property. The category classifies the type of \
                                  announcement."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"category\" with a URL from the Schema.org vocabulary \
                                     (e.g., https://schema.org/EmergencyAlert)."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// SoftwareApplicationValidator
// =========================================================================

/// Validates SoftwareApplication structured data for completeness.
pub struct SoftwareApplicationValidator;

impl Default for SoftwareApplicationValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl SoftwareApplicationValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for SoftwareApplicationValidator {
    fn name(&self) -> &str {
        "software-application-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("SoftwareApplication") {
                continue;
            }
            let data = &sd.data;

            // SOFT001: Missing operatingSystem
            if data.get("operatingSystem").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "SOFT001".to_string(),
                    title: "SoftwareApplication missing operatingSystem".to_string(),
                    description: "A SoftwareApplication structured data block is missing the \
                                  \"operatingSystem\" property. This helps search engines display \
                                  platform compatibility."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"operatingSystem\" with the supported platforms (e.g., \
                                     \"Windows\", \"macOS\", \"iOS\", \"Android\")."
                        .to_string(),
                });
            }

            // SOFT002: Missing offers
            if data.get("offers").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "SOFT002".to_string(),
                    title: "SoftwareApplication missing offers".to_string(),
                    description: "A SoftwareApplication structured data block is missing the \
                                  \"offers\" property. Offers provide pricing and availability \
                                  information that helps search engines display cost details in \
                                  app search results."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"offers\" with an Offer object containing \"price\" and \
                                     \"priceCurrency\"."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// ArticleSchemaValidator
// =========================================================================

/// Validates Article (and subtype) structured data for completeness.
pub struct ArticleSchemaValidator;

impl Default for ArticleSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ArticleSchemaValidator {
    pub fn new() -> Self {
        Self
    }

    const ARTICLE_TYPES: &[&str] = &[
        "Article",
        "NewsArticle",
        "BlogPosting",
        "ScholarlyArticle",
        "TechArticle",
        "Report",
    ];
}

impl Analyzer for ArticleSchemaValidator {
    fn name(&self) -> &str {
        "article-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            let schema_type = sd.data.get("@type").and_then(|t| t.as_str()).unwrap_or("");
            if !Self::ARTICLE_TYPES.contains(&schema_type) {
                continue;
            }
            let data = &sd.data;

            // ART001: Missing headline
            if data.get("headline").is_none() {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Schema,
                    code: "ART001".to_string(),
                    title: "Article schema missing headline".to_string(),
                    description: format!(
                        "A {schema_type} structured data block is missing the required \
                         \"headline\" property."
                    ),
                    url: url.clone(),
                    recommendation: "Add \"headline\" with the article title.".to_string(),
                });
            }

            // ART002: Missing datePublished
            if data.get("datePublished").is_none() {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Schema,
                    code: "ART002".to_string(),
                    title: "Article schema missing datePublished".to_string(),
                    description: format!(
                        "A {schema_type} structured data block is missing the required \
                         \"datePublished\" property."
                    ),
                    url: url.clone(),
                    recommendation: "Add \"datePublished\" with an ISO 8601 date value."
                        .to_string(),
                });
            }

            // ART003: Missing author
            if data.get("author").is_none() {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Schema,
                    code: "ART003".to_string(),
                    title: "Article schema missing author".to_string(),
                    description: format!(
                        "A {schema_type} structured data block is missing the required \
                         \"author\" property."
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

// =========================================================================
// OrganizationSchemaValidator
// =========================================================================

/// Validates Organization structured data for completeness.
pub struct OrganizationSchemaValidator;

impl Default for OrganizationSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl OrganizationSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for OrganizationSchemaValidator {
    fn name(&self) -> &str {
        "organization-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            let schema_type = sd.data.get("@type").and_then(|t| t.as_str()).unwrap_or("");
            if schema_type != "Organization" {
                continue;
            }
            let data = &sd.data;

            // ORG001: Missing name
            if data.get("name").is_none() {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Schema,
                    code: "ORG001".to_string(),
                    title: "Organization schema missing name".to_string(),
                    description: "An Organization structured data block is missing the required \
                                  \"name\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with the organization's official name."
                        .to_string(),
                });
            }

            // ORG002: Missing url
            if data.get("url").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "ORG002".to_string(),
                    title: "Organization schema missing url".to_string(),
                    description: "An Organization structured data block is missing the \"url\" \
                                  property. This helps search engines verify the organization."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"url\" with the organization's official website URL."
                        .to_string(),
                });
            }

            // ORG003: Missing logo
            if data.get("logo").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "ORG003".to_string(),
                    title: "Organization schema missing logo".to_string(),
                    description: "An Organization structured data block is missing the \"logo\" \
                                  property. Logos are used in Knowledge Graph results."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"logo\" with a URL to the organization's logo image."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// PersonSchemaValidator
// =========================================================================

/// Validates Person structured data for completeness.
pub struct PersonSchemaValidator;

impl Default for PersonSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl PersonSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for PersonSchemaValidator {
    fn name(&self) -> &str {
        "person-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            let schema_type = sd.data.get("@type").and_then(|t| t.as_str()).unwrap_or("");
            if schema_type != "Person" {
                continue;
            }
            let data = &sd.data;

            // PERS001: Missing name
            if data.get("name").is_none() {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Schema,
                    code: "PERS001".to_string(),
                    title: "Person schema missing name".to_string(),
                    description: "A Person structured data block is missing the required \
                                  \"name\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with the person's full name.".to_string(),
                });
            }

            // PERS002: Missing sameAs
            if data.get("sameAs").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "PERS002".to_string(),
                    title: "Person schema missing sameAs".to_string(),
                    description: "A Person structured data block is missing the \"sameAs\" \
                                  property. sameAs links to social profiles and helps build \
                                  the Knowledge Graph."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"sameAs\" with an array of URLs to social profiles \
                                     (e.g., LinkedIn, Twitter)."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// JobPostingSchemaValidator
// =========================================================================

/// Validates JobPosting structured data for completeness.
pub struct JobPostingSchemaValidator;

impl Default for JobPostingSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl JobPostingSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for JobPostingSchemaValidator {
    fn name(&self) -> &str {
        "jobposting-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            let schema_type = sd.data.get("@type").and_then(|t| t.as_str()).unwrap_or("");
            if schema_type != "JobPosting" {
                continue;
            }
            let data = &sd.data;

            // JOB001: Missing title (job title)
            if data.get("title").is_none() {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Schema,
                    code: "JOB001".to_string(),
                    title: "JobPosting schema missing title".to_string(),
                    description: "A JobPosting structured data block is missing the required \
                                  \"title\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"title\" with the job position title.".to_string(),
                });
            }

            // JOB002: Missing datePosted
            if data.get("datePosted").is_none() {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Schema,
                    code: "JOB002".to_string(),
                    title: "JobPosting schema missing datePosted".to_string(),
                    description: "A JobPosting structured data block is missing the required \
                                  \"datePosted\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"datePosted\" with an ISO 8601 date."
                        .to_string(),
                });
            }

            // JOB003: Missing validThrough
            if data.get("validThrough").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "JOB003".to_string(),
                    title: "JobPosting schema missing validThrough".to_string(),
                    description: "A JobPosting structured data block is missing the \"validThrough\" \
                                  property. This tells search engines when the job posting expires."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"validThrough\" with an ISO 8601 date/time when the \
                                     posting expires."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// CourseSchemaValidator
// =========================================================================

/// Validates Course structured data for completeness.
pub struct CourseSchemaValidator;

impl Default for CourseSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl CourseSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for CourseSchemaValidator {
    fn name(&self) -> &str {
        "course-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            let schema_type = sd.data.get("@type").and_then(|t| t.as_str()).unwrap_or("");
            if schema_type != "Course" {
                continue;
            }
            let data = &sd.data;

            // COURSE001: Missing name
            if data.get("name").is_none() {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Schema,
                    code: "COURSE001".to_string(),
                    title: "Course schema missing name".to_string(),
                    description: "A Course structured data block is missing the required \
                                  \"name\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with the course title.".to_string(),
                });
            }

            // COURSE002: Missing provider
            if data.get("provider").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "COURSE002".to_string(),
                    title: "Course schema missing provider".to_string(),
                    description: "A Course structured data block is missing the \"provider\" \
                                  property. The provider identifies the organization offering \
                                  the course."
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
// RecipeSchemaValidator
// =========================================================================

/// Validates Recipe structured data for completeness.
pub struct RecipeSchemaValidator;

impl Default for RecipeSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl RecipeSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for RecipeSchemaValidator {
    fn name(&self) -> &str {
        "recipe-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            let schema_type = sd.data.get("@type").and_then(|t| t.as_str()).unwrap_or("");
            if schema_type != "Recipe" {
                continue;
            }
            let data = &sd.data;

            // RECIPE001: Missing name
            if data.get("name").is_none() {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Schema,
                    code: "RECIPE001".to_string(),
                    title: "Recipe schema missing name".to_string(),
                    description: "A Recipe structured data block is missing the required \
                                  \"name\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with the recipe title.".to_string(),
                });
            }

            // RECIPE002: Missing cookTime
            if data.get("cookTime").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "RECIPE002".to_string(),
                    title: "Recipe schema missing cookTime".to_string(),
                    description: "A Recipe structured data block is missing the \"cookTime\" \
                                  property. cookTime helps search engines display cooking \
                                  duration in rich results."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"cookTime\" with an ISO 8601 duration (e.g., PT30M)."
                        .to_string(),
                });
            }

            // RECIPE003: Missing recipeIngredient
            if data.get("recipeIngredient").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "RECIPE003".to_string(),
                    title: "Recipe schema missing recipeIngredient".to_string(),
                    description: "A Recipe structured data block is missing the \
                                  \"recipeIngredient\" property. Ingredients are required for \
                                  Recipe rich results."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"recipeIngredient\" with an array of ingredient strings."
                        .to_string(),
                });
            }
        }

        findings
    }
}

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
                description: "This page appears to be time-sensitive content (blog, news, article) \
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
                            for candidate_year_str in
                                window.split(|c: char| !c.is_ascii_digit())
                            {
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

/// Validates BreadcrumbList depth consistency with URL depth.
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
            if url_depth > 2 && breadcrumb_depth > 0 && (breadcrumb_depth as isize - url_depth as isize).abs() > 1 {
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

/// Validates WebPage structured data for completeness.
pub struct WebPageSchemaValidator;

impl Default for WebPageSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl WebPageSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for WebPageSchemaValidator {
    fn name(&self) -> &str {
        "webpage-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("WebPage") {
                continue;
            }
            let data = &sd.data;

            if data.get("name").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "WEBPG001".to_string(),
                    title: "WebPage schema missing name".to_string(),
                    description: "A WebPage structured data block is missing the \"name\" property. \
                                  Search engines use the name to understand the page topic."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with a descriptive page title to the WebPage \
                                     schema."
                        .to_string(),
                });
            }

            if data.get("datePublished").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "WEBPG002".to_string(),
                    title: "WebPage schema missing datePublished".to_string(),
                    description: "A WebPage structured data block is missing the \"datePublished\" \
                                  property. This helps search engines assess content freshness."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"datePublished\" with an ISO 8601 date value."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// ServiceSchemaValidator
// =========================================================================

/// Validates Service structured data for completeness.
pub struct ServiceSchemaValidator;

impl Default for ServiceSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ServiceSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for ServiceSchemaValidator {
    fn name(&self) -> &str {
        "service-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Service") {
                continue;
            }
            let data = &sd.data;

            if data.get("name").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "SVC001".to_string(),
                    title: "Service schema missing name".to_string(),
                    description: "A Service structured data block is missing the \"name\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with the service name to the Service schema."
                        .to_string(),
                });
            }

            if data.get("provider").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "SVC002".to_string(),
                    title: "Service schema missing provider".to_string(),
                    description: "A Service structured data block is missing the \"provider\" \
                                  property. The provider identifies the organization offering the \
                                  service."
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
// ItemListSchemaValidator
// =========================================================================

/// Validates ItemList structured data for completeness.
pub struct ItemListSchemaValidator;

impl Default for ItemListSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ItemListSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for ItemListSchemaValidator {
    fn name(&self) -> &str {
        "itemlist-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("ItemList") {
                continue;
            }
            let data = &sd.data;

            match data.get("itemListElement") {
                None | Some(serde_json::Value::Null) => {
                    findings.push(Finding {
                        severity: Severity::Error,
                        category: IssueCategory::Schema,
                        code: "ITEMLIST001".to_string(),
                        title: "ItemList schema missing itemListElement".to_string(),
                        description: "An ItemList structured data block is missing the required \
                                      \"itemListElement\" property."
                            .to_string(),
                        url: url.clone(),
                        recommendation: "Add \"itemListElement\" with an array of ListItem objects."
                            .to_string(),
                    });
                }
                Some(val) => {
                    match val.as_array() {
                        None => {
                            // Not an array at all (e.g. string, number)
                            findings.push(Finding {
                                severity: Severity::Error,
                                category: IssueCategory::Schema,
                                code: "ITEMLIST001".to_string(),
                                title: "ItemList schema missing itemListElement".to_string(),
                                description: "An ItemList structured data block has an \
                                              \"itemListElement\" that is not an array."
                                    .to_string(),
                                url: url.clone(),
                                recommendation: "Change itemListElement to an array of ListItem objects."
                                    .to_string(),
                            });
                        }
                        Some(arr) if arr.is_empty() => {
                            findings.push(Finding {
                                severity: Severity::Error,
                                category: IssueCategory::Schema,
                                code: "ITEMLIST002".to_string(),
                                title: "ItemList schema itemListElement is empty".to_string(),
                                description: "An ItemList structured data block has an empty \
                                              \"itemListElement\" array."
                                    .to_string(),
                                url: url.clone(),
                                recommendation: "Populate the itemListElement array with ListItem objects."
                                    .to_string(),
                            });
                        }
                        _ => {}
                    }
                }
            }
        }

        findings
    }
}

// =========================================================================
// OfferSchemaValidator
// =========================================================================

/// Validates Offer structured data for completeness.
pub struct OfferSchemaValidator;

impl Default for OfferSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl OfferSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for OfferSchemaValidator {
    fn name(&self) -> &str {
        "offer-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Offer") {
                continue;
            }
            let data = &sd.data;

            if data.get("price").is_none() {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Schema,
                    code: "OFFER001".to_string(),
                    title: "Offer schema missing price".to_string(),
                    description: "An Offer structured data block is missing the \"price\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"price\" with the item price."
                        .to_string(),
                });
            }

            if data.get("priceCurrency").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Schema,
                    code: "OFFER002".to_string(),
                    title: "Offer schema missing priceCurrency".to_string(),
                    description: "An Offer structured data block is missing the \"priceCurrency\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"priceCurrency\" with an ISO 4217 currency code (e.g., \
                                     \"USD\")."
                        .to_string(),
                });
            }

            if data.get("availability").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "OFFER003".to_string(),
                    title: "Offer schema missing availability".to_string(),
                    description: "An Offer structured data block is missing the \"availability\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"availability\" with a schema.org Availability value \
                                     (e.g., \"https://schema.org/InStock\")."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// AggregateOfferSchemaValidator
// =========================================================================

/// Validates AggregateOffer structured data for completeness.
pub struct AggregateOfferSchemaValidator;

impl Default for AggregateOfferSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl AggregateOfferSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for AggregateOfferSchemaValidator {
    fn name(&self) -> &str {
        "aggregate-offer-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("AggregateOffer") {
                continue;
            }
            let data = &sd.data;

            if data.get("lowPrice").is_none() {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Schema,
                    code: "AGGOFFER001".to_string(),
                    title: "AggregateOffer schema missing lowPrice".to_string(),
                    description: "An AggregateOffer structured data block is missing the required \
                                  \"lowPrice\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"lowPrice\" with the lowest price in the range."
                        .to_string(),
                });
            }

            if data.get("priceCurrency").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding {
                    severity: Severity::Error,
                    category: IssueCategory::Schema,
                    code: "AGGOFFER002".to_string(),
                    title: "AggregateOffer schema missing priceCurrency".to_string(),
                    description: "An AggregateOffer structured data block is missing the \
                                  \"priceCurrency\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"priceCurrency\" with an ISO 4217 currency code."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// BrandSchemaValidator
// =========================================================================

/// Validates Brand structured data for completeness.
pub struct BrandSchemaValidator;

impl Default for BrandSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl BrandSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for BrandSchemaValidator {
    fn name(&self) -> &str {
        "brand-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Brand") {
                continue;
            }
            let data = &sd.data;

            if data.get("name").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "BRAND001".to_string(),
                    title: "Brand schema missing name".to_string(),
                    description: "A Brand structured data block is missing the \"name\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with the brand name."
                        .to_string(),
                });
            }

            if data.get("url").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "BRAND002".to_string(),
                    title: "Brand schema missing url".to_string(),
                    description: "A Brand structured data block is missing the \"url\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"url\" with the brand website URL."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// OccupationSchemaValidator
// =========================================================================

/// Validates Occupation structured data for completeness.
pub struct OccupationSchemaValidator;

impl Default for OccupationSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl OccupationSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for OccupationSchemaValidator {
    fn name(&self) -> &str {
        "occupation-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Occupation") {
                continue;
            }
            let data = &sd.data;

            if data.get("name").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "OCCUP001".to_string(),
                    title: "Occupation schema missing name".to_string(),
                    description: "An Occupation structured data block is missing the \"name\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with the occupation title."
                        .to_string(),
                });
            }

            let has_category = match data.get("occupationalCategory") {
                None | Some(serde_json::Value::Null) => false,
                Some(serde_json::Value::String(s)) => !s.is_empty(),
                Some(serde_json::Value::Object(_)) => true,
                Some(serde_json::Value::Array(a)) => !a.is_empty(),
                Some(_) => true, // numbers, booleans
            };
            if !has_category {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "OCCUP002".to_string(),
                    title: "Occupation schema missing occupationalCategory".to_string(),
                    description: "An Occupation structured data block is missing the \
                                  \"occupationalCategory\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"occupationalCategory\" with a category code or text."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// QuestSchemaValidator
// =========================================================================

/// Validates Quest structured data for games and education.
pub struct QuestSchemaValidator;

impl Default for QuestSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl QuestSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for QuestSchemaValidator {
    fn name(&self) -> &str {
        "quest-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Quest") {
                continue;
            }
            let data = &sd.data;

            if data.get("name").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "QUEST001".to_string(),
                    title: "Quest schema missing name".to_string(),
                    description: "A Quest structured data block is missing the \"name\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with the quest title."
                        .to_string(),
                });
            }

            if data.get("questType").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "QUEST002".to_string(),
                    title: "Quest schema missing questType".to_string(),
                    description: "A Quest structured data block is missing the \"questType\" \
                                  property. This helps classify the quest (e.g., main quest, \
                                  side quest, tutorial)."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"questType\" to classify the quest."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// ActionSchemaValidator
// =========================================================================

/// Validates Action structured data for completeness.
pub struct ActionSchemaValidator;

impl Default for ActionSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ActionSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for ActionSchemaValidator {
    fn name(&self) -> &str {
        "action-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Action") {
                continue;
            }
            let data = &sd.data;

            if data.get("actionType").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "ACTION001".to_string(),
                    title: "Action schema missing actionType".to_string(),
                    description: "An Action structured data block is missing the \"actionType\" \
                                  property. Search engines use this to understand the action kind."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"actionType\" with a specific action type (e.g., \
                                     \"BuyAction\", \"ViewAction\")."
                        .to_string(),
                });
            }

            let has_target = match data.get("target") {
                None | Some(serde_json::Value::Null) => false,
                Some(serde_json::Value::String(s)) => !s.is_empty(),
                Some(serde_json::Value::Object(_)) => true,
                Some(serde_json::Value::Array(a)) => !a.is_empty(),
                Some(_) => true,
            };
            if !has_target {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "ACTION002".to_string(),
                    title: "Action schema missing target".to_string(),
                    description: "An Action structured data block is missing the \"target\" \
                                  property. The target defines where the action leads."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"target\" with an EntryPoint or URL string."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// PlaybookSchemaValidator
// =========================================================================

/// Validates Playbook structured data for completeness.
pub struct PlaybookSchemaValidator;

impl Default for PlaybookSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl PlaybookSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for PlaybookSchemaValidator {
    fn name(&self) -> &str {
        "playbook-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Playbook") {
                continue;
            }
            let data = &sd.data;

            if data.get("name").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "PLAYBOOK001".to_string(),
                    title: "Playbook schema missing name".to_string(),
                    description: "A Playbook structured data block is missing the \"name\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with the playbook title."
                        .to_string(),
                });
            }

            match data.get("step") {
                None => {
                    findings.push(Finding {
                        severity: Severity::Error,
                        category: IssueCategory::Schema,
                        code: "PLAYBOOK002".to_string(),
                        title: "Playbook schema missing step".to_string(),
                        description: "A Playbook structured data block is missing the required \
                                      \"step\" property."
                            .to_string(),
                        url: url.clone(),
                        recommendation: "Add \"step\" with an array of HowToStep objects."
                            .to_string(),
                    });
                }
                Some(val) if val.as_array().map_or(true, |a| a.is_empty()) => {
                    findings.push(Finding {
                        severity: Severity::Error,
                        category: IssueCategory::Schema,
                        code: "PLAYBOOK002".to_string(),
                        title: "Playbook schema step is empty".to_string(),
                        description: "A Playbook structured data block has an empty \"step\" array."
                            .to_string(),
                        url: url.clone(),
                        recommendation: "Populate the step array with HowToStep objects."
                            .to_string(),
                    });
                }
                _ => {}
            }
        }

        findings
    }
}

// =========================================================================
// LocalBusinessHoursValidator
// =========================================================================

pub struct LocalBusinessHoursValidator;

impl LocalBusinessHoursValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LocalBusinessHoursValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for LocalBusinessHoursValidator {
    fn name(&self) -> &str {
        "local-business-hours"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            let schema_type = sd.r#type.as_deref();
            let is_local = matches!(
                schema_type,
                Some("LocalBusiness")
                    | Some("Store")
                    | Some("Restaurant")
                    | Some("Hotel")
                    | Some("HealthClub")
                    | Some("AutomotiveBusiness")
                    | Some("EntertainmentBusiness")
                    | Some("FinancialService")
                    | Some("FoodEstablishment")
                    | Some("GovernmentOffice")
                    | Some("HealthAndBeautyBusiness")
                    | Some("HomeAndConstructionBusiness")
                    | Some("InternetCafe")
                    | Some("LegalService")
                    | Some("Library")
                    | Some("LodgingBusiness")
                    | Some("ProfessionalService")
                    | Some("RadioStation")
                    | Some("SelfStorage")
                    | Some("ShoppingCenter")
                    | Some("SportsActivityLocation")
                    | Some("TelevisionStation")
                    | Some("TouristInformationCenter")
                    | Some("TravelAgency")
            );
            if !is_local {
                continue;
            }

            let data = &sd.data;

            if data.get("openingHours").is_none() && data.get("openingHoursSpecification").is_none()
            {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "LBH001".to_string(),
                    title: "LocalBusiness missing openingHours".to_string(),
                    description: "A LocalBusiness structured data block is missing the \
                                  \"openingHours\" or \"openingHoursSpecification\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"openingHours\" with ISO 8601 time ranges (e.g., \
                                     \"Mo-Fr 09:00-17:00\") or \"openingHoursSpecification\" for \
                                     detailed hours."
                        .to_string(),
                });
                continue;
            }

            if let Some(hours) = data.get("openingHours") {
                if let Some(s) = hours.as_str() {
                    let valid_format = s
                        .split(',')
                        .all(|entry| {
                            let entry = entry.trim();
                            if entry.is_empty() {
                                return true;
                            }
                            let days = ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];
                            let has_day = days.iter().any(|d| entry.contains(d));
                            let has_dash_range = entry.contains('-')
                                && entry.matches('-').count() <= 2;
                            let has_time = entry.contains(':');
                            has_day || has_dash_range || has_time
                        });
                    if !valid_format {
                        findings.push(Finding {
                            severity: Severity::Warning,
                            category: IssueCategory::Schema,
                            code: "LBH002".to_string(),
                            title: "LocalBusiness openingHours in invalid format".to_string(),
                            description: format!(
                                "The openingHours value \"{s}\" does not appear to follow ISO \
                                 8601 format."
                            ),
                            url: url.clone(),
                            recommendation: "Use ISO 8601 format for openingHours, e.g., \
                                             \"Mo-Fr 09:00-17:00, Sa 10:00-14:00\"."
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
// ProductReviewValidator
// =========================================================================

pub struct ProductReviewValidator;

impl ProductReviewValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ProductReviewValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for ProductReviewValidator {
    fn name(&self) -> &str {
        "product-review"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Product") {
                continue;
            }
            let data = &sd.data;

            if let Some(reviews) = data.get("review") {
                let review_iter: Vec<&serde_json::Value> = if let Some(arr) = reviews.as_array() {
                    arr.iter().collect()
                } else {
                    vec![reviews]
                };

                for (i, review) in review_iter.iter().enumerate() {
                    if review.get("reviewRating").is_none() {
                        findings.push(Finding {
                            severity: Severity::Warning,
                            category: IssueCategory::Schema,
                            code: "PREV001".to_string(),
                            title: "Product review missing reviewRating".to_string(),
                            description: format!(
                                "Review #{i} in Product schema is missing the \"reviewRating\" \
                                 property."
                            ),
                            url: url.clone(),
                            recommendation: "Add \"reviewRating\" with a Rating object to each \
                                             review."
                                .to_string(),
                        });
                    }

                    if review.get("author").is_none() {
                        findings.push(Finding {
                            severity: Severity::Warning,
                            category: IssueCategory::Schema,
                            code: "PREV002".to_string(),
                            title: "Product review missing author".to_string(),
                            description: format!(
                                "Review #{i} in Product schema is missing the \"author\" property."
                            ),
                            url: url.clone(),
                            recommendation: "Add \"author\" with a Person or Organization object \
                                             to each review."
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
// EventLocationValidator
// =========================================================================

pub struct EventLocationValidator;

impl EventLocationValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for EventLocationValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for EventLocationValidator {
    fn name(&self) -> &str {
        "event-location"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Event") {
                continue;
            }
            let data = &sd.data;

            if data.get("location").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "ELOC001".to_string(),
                    title: "Event missing location".to_string(),
                    description: "An Event structured data block is missing the \"location\" \
                                  property. Location is important for event rich results."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"location\" with a Place, VirtualLocation, or PostalAddress object."
                        .to_string(),
                });
                continue;
            }

            if let Some(location) = data.get("location") {
                let has_name = location.get("name").is_some()
                    || location.get("url").is_some()
                    || location.get("address").is_some();
                if !has_name {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Schema,
                        code: "ELOC002".to_string(),
                        title: "Event location missing name".to_string(),
                        description: "The \"location\" property in Event schema does not contain \
                                      a \"name\", \"url\", or \"address\" sub-property."
                            .to_string(),
                        url: url.clone(),
                        recommendation: "Add \"name\" to the location object to identify the \
                                         venue or place."
                            .to_string(),
                    });
                }
            }
        }

        findings
    }
}

// =========================================================================
// OrganizationLogoValidator
// =========================================================================

pub struct OrganizationLogoValidator;

impl OrganizationLogoValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for OrganizationLogoValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for OrganizationLogoValidator {
    fn name(&self) -> &str {
        "organization-logo"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            let schema_type = sd.r#type.as_deref();
            if schema_type != Some("Organization") && schema_type != Some("LocalBusiness") {
                continue;
            }
            let data = &sd.data;

            if data.get("logo").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "OLOGO001".to_string(),
                    title: "Organization missing logo".to_string(),
                    description: "An Organization structured data block is missing the \"logo\" \
                                  property. The logo is used for knowledge panel display."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"logo\" with a URL string or ImageObject pointing to \
                                     the organization's logo."
                        .to_string(),
                });
                continue;
            }

            if let Some(logo) = data.get("logo") {
                let logo_str = logo.as_str().unwrap_or("");
                if !logo_str.is_empty()
                    && !logo_str.starts_with("http://")
                    && !logo_str.starts_with("https://")
                    && logo.get("@type").is_none()
                {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Schema,
                        code: "OLOGO002".to_string(),
                        title: "Organization logo URL invalid format".to_string(),
                        description: format!(
                            "The logo value \"{logo_str}\" is not a valid absolute URL."
                        ),
                        url: url.clone(),
                        recommendation: "Use an absolute URL (https://...) for the logo property."
                            .to_string(),
                    });
                }
            }
        }

        findings
    }
}

// =========================================================================
// PersonJobTitleValidator
// =========================================================================

pub struct PersonJobTitleValidator;

impl PersonJobTitleValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PersonJobTitleValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for PersonJobTitleValidator {
    fn name(&self) -> &str {
        "person-job-title"
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
                    code: "PJOB001".to_string(),
                    title: "Person missing jobTitle".to_string(),
                    description: "A Person structured data block is missing the \"jobTitle\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"jobTitle\" with the person's professional title."
                        .to_string(),
                });
            }

            if data.get("worksFor").is_none() && data.get("memberOf").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "PJOB002".to_string(),
                    title: "Person missing worksFor".to_string(),
                    description: "A Person structured data block is missing the \"worksFor\" or \
                                  \"memberOf\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"worksFor\" with an Organization object to indicate the \
                                     person's employer."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// RecipeNutritionValidator
// =========================================================================

pub struct RecipeNutritionValidator;

impl RecipeNutritionValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RecipeNutritionValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for RecipeNutritionValidator {
    fn name(&self) -> &str {
        "recipe-nutrition"
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
                    code: "RNUT001".to_string(),
                    title: "Recipe missing nutrition information".to_string(),
                    description: "A Recipe structured data block is missing the \"nutrition\" \
                                  property. Nutrition info improves eligibility for rich results."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"nutrition\" with a NutritionInformation object \
                                     containing at least \"calories\"."
                        .to_string(),
                });
                continue;
            }

            if let Some(nutrition) = data.get("nutrition") {
                if nutrition.get("calories").is_none() {
                    findings.push(Finding {
                        severity: Severity::Info,
                        category: IssueCategory::Schema,
                        code: "RNUT002".to_string(),
                        title: "Recipe nutrition missing calories".to_string(),
                        description: "The \"nutrition\" property in Recipe schema is missing \
                                      \"calories\"."
                            .to_string(),
                        url: url.clone(),
                        recommendation: "Add \"calories\" with a string value (e.g., \"240 cal\")."
                            .to_string(),
                    });
                }
            }
        }

        findings
    }
}

// =========================================================================
// CourseProviderValidator
// =========================================================================

pub struct CourseProviderValidator;

impl CourseProviderValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CourseProviderValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for CourseProviderValidator {
    fn name(&self) -> &str {
        "course-provider"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Course") {
                continue;
            }
            let data = &sd.data;

            if let Some(provider) = data.get("provider") {
                if provider.get("name").is_none() {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Schema,
                        code: "CPROV001".to_string(),
                        title: "Course provider missing name".to_string(),
                        description: "The \"provider\" object in Course schema is missing \
                                      \"name\"."
                            .to_string(),
                        url: url.clone(),
                        recommendation: "Add \"name\" to the provider object to identify the \
                                         course provider."
                            .to_string(),
                    });
                }

                if provider.get("url").is_none() {
                    findings.push(Finding {
                        severity: Severity::Info,
                        category: IssueCategory::Schema,
                        code: "CPROV002".to_string(),
                        title: "Course provider missing URL".to_string(),
                        description: "The \"provider\" object in Course schema is missing \"url\"."
                            .to_string(),
                        url: url.clone(),
                        recommendation: "Add \"url\" to the provider object linking to the \
                                         provider's website."
                            .to_string(),
                    });
                }
            }
        }

        findings
    }
}

// =========================================================================
// JobPostingSalaryValidator
// =========================================================================

pub struct JobPostingSalaryValidator;

impl JobPostingSalaryValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for JobPostingSalaryValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for JobPostingSalaryValidator {
    fn name(&self) -> &str {
        "job-posting-salary"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("JobPosting") {
                continue;
            }
            let data = &sd.data;

            if data.get("baseSalary").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "JSAL001".to_string(),
                    title: "JobPosting missing baseSalary".to_string(),
                    description: "A JobPosting structured data block is missing the \"baseSalary\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"baseSalary\" with a MonetaryAmount or QuantitativeValue \
                                     to show salary information in search results."
                        .to_string(),
                });
            }

            if data.get("employmentType").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "JSAL002".to_string(),
                    title: "JobPosting missing employmentType".to_string(),
                    description: "A JobPosting structured data block is missing the \
                                  \"employmentType\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"employmentType\" with one of: FULL_TIME, PART_TIME, \
                                     CONTRACTOR, TEMPORARY, INTERN, VOLUNTEER, PER_DIEM, OTHER."
                        .to_string(),
                });
            }
        }

        findings
    }
}

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

    // ===== RdfaValidator =====

    #[test]
    fn test_rdfa_no_rdfa_attributes() {
        let page = make_page("https://example.com");
        let ctx = make_ctx_with_body(&page, Some(200), "<html><body>Hello</body></html>");
        assert!(RdfaValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_rdfa_no_body() {
        let page = make_page("https://example.com");
        assert!(RdfaValidator::new().analyze(&make_ctx(&page, Some(200))).is_empty());
    }

    #[test]
    fn test_rdfa_missing_vocab() {
        let page = make_page("https://example.com");
        let body = r#"<div typeof="Person"><span property="name">John</span></div>"#;
        let ctx = make_ctx_with_body(&page, Some(200), body);
        assert!(RdfaValidator::new().analyze(&ctx).iter().any(|f| f.code == "RDFA001"));
    }

    #[test]
    fn test_rdfa_missing_typeof() {
        let page = make_page("https://example.com");
        let body = r#"<div vocab="https://schema.org/"><span property="name">John</span></div>"#;
        let ctx = make_ctx_with_body(&page, Some(200), body);
        assert!(RdfaValidator::new().analyze(&ctx).iter().any(|f| f.code == "RDFA002"));
    }

    #[test]
    fn test_rdfa_deprecated_vocab() {
        let page = make_page("https://example.com");
        let body = r#"<div vocab="http://data-vocabulary.org/Review" typeof="Review"></div>"#;
        let ctx = make_ctx_with_body(&page, Some(200), body);
        assert!(RdfaValidator::new().analyze(&ctx).iter().any(|f| f.code == "RDFA003"));
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
        let body = r#"<div about="http://example.com/page"><span property="name">Page</span></div>"#;
        let ctx = make_ctx_with_body(&page, Some(200), body);
        assert!(RdfaValidator::new().analyze(&ctx).iter().any(|f| f.code == "RDFA002"));
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
        assert!(!RdfaValidator::new().analyze(&ctx).iter().any(|f| f.code == "RDFA003"));
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

    // ===== MicrodataValidator =====

    #[test]
    fn test_microdata_no_microdata() {
        let page = make_page("https://example.com");
        let ctx = make_ctx_with_body(&page, Some(200), "<html><body>Hello</body></html>");
        assert!(MicrodataValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_microdata_no_body() {
        let page = make_page("https://example.com");
        assert!(MicrodataValidator::new().analyze(&make_ctx(&page, Some(200))).is_empty());
    }

    #[test]
    fn test_microdata_itemscope_without_itemprop() {
        let page = make_page("https://example.com");
        let body = r#"<div itemscope itemtype="http://schema.org/Product"></div>"#;
        let ctx = make_ctx_with_body(&page, Some(200), body);
        assert!(MicrodataValidator::new().analyze(&ctx).iter().any(|f| f.code == "MD001"));
    }

    #[test]
    fn test_microdata_unknown_type() {
        let page = make_page("https://example.com");
        let body = r#"<div itemscope itemtype="http://example.com/Custom"><span itemprop="name">X</span></div>"#;
        let ctx = make_ctx_with_body(&page, Some(200), body);
        assert!(MicrodataValidator::new().analyze(&ctx).iter().any(|f| f.code == "MD003"));
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
        assert!(MicrodataValidator::new().analyze(&ctx).iter().any(|f| f.code == "MD002"));
    }

    #[test]
    fn test_microdata_valid_article() {
        let page = make_page("https://example.com");
        let body = r#"<div itemscope itemtype="http://schema.org/Article"><span itemprop="headline">Title</span><span itemprop="author">Author</span></div>"#;
        let ctx = make_ctx_with_body(&page, Some(200), body);
        assert!(!MicrodataValidator::new().analyze(&ctx).iter().any(|f| f.code == "MD002"));
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
        assert!(MicrodataValidator::new().analyze(&ctx).iter().any(|f| f.code == "MD002"));
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
        assert!(!MicrodataValidator::new().analyze(&ctx).iter().any(|f| f.code == "MD003"));
    }

    #[test]
    fn test_microdata_multiple_types() {
        let page = make_page("https://example.com");
        let body = r#"<div itemscope itemtype="http://schema.org/Product"><span itemprop="name">Widget</span></div>"#;
        let ctx = make_ctx_with_body(&page, Some(200), body);
        assert!(!MicrodataValidator::new().analyze(&ctx).iter().any(|f| f.code == "MD003"));
    }

    // ===== EntityLinkingAnalyzer =====

    #[test]
    fn test_entity_linking_no_structured_data() {
        let page = make_page("https://example.com");
        assert!(EntityLinkingAnalyzer::new().analyze(&make_ctx(&page, Some(200))).is_empty());
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
        assert!(EntityLinkingAnalyzer::new().analyze(&ctx).iter().any(|f| f.code == "ELINK001"));
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
        assert!(!EntityLinkingAnalyzer::new().analyze(&ctx).iter().any(|f| f.code == "ELINK001"));
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
        assert!(!EntityLinkingAnalyzer::new().analyze(&ctx).iter().any(|f| f.code == "ELINK001"));
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
        assert!(EntityLinkingAnalyzer::new().analyze(&ctx).iter().any(|f| f.code == "ELINK002"));
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
        assert!(!EntityLinkingAnalyzer::new().analyze(&ctx).iter().any(|f| f.code == "ELINK002"));
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
        assert!(EntityLinkingAnalyzer::new().analyze(&ctx).iter().any(|f| f.code == "ELINK001"));
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
        assert!(!EntityLinkingAnalyzer::new().analyze(&ctx).iter().any(|f| f.code == "ELINK001"));
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
        assert!(!EntityLinkingAnalyzer::new().analyze(&ctx).iter().any(|f| f.code == "ELINK002"));
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

    // ===== ShippingSchemaValidator =====

    #[test]
    fn test_shipping_no_product_schema() {
        let page = make_page("https://example.com");
        assert!(ShippingSchemaValidator::new().analyze(&make_ctx(&page, Some(200))).is_empty());
    }

    #[test]
    fn test_shipping_product_with_offers_no_shipping() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product", "name": "Widget", "offers": {"@type": "Offer", "price": "9.99", "priceCurrency": "USD"}}),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(ShippingSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "SHIP001"));
    }

    #[test]
    fn test_shipping_product_with_shipping_details() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product", "name": "Widget", "offers": {"@type": "Offer", "price": "9.99", "hasShippingDetails": {"@type": "ShippingDetails"}}}),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(!ShippingSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "SHIP001"));
    }

    #[test]
    fn test_shipping_product_no_offers() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product", "name": "Widget"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(ShippingSchemaValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_shipping_product_empty_offers() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product", "name": "Widget", "offers": []}),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(ShippingSchemaValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_shipping_offers_array_with_shipping() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product", "name": "Widget", "offers": [{"@type": "Offer", "price": "9.99", "hasShippingDetails": {"@type": "ShippingDetails"}}]}),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(!ShippingSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "SHIP001"));
    }

    #[test]
    fn test_shipping_top_level_shipping() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product", "name": "Widget", "offers": {"@type": "Offer", "price": "9.99"}, "hasShippingDetails": {"@type": "ShippingDetails"}}),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(!ShippingSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "SHIP001"));
    }

    #[test]
    fn test_shipping_non_product_schema() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({"@type": "Article", "headline": "News"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(ShippingSchemaValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_shipping_product_url_reference() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product", "name": "Widget", "offers": {"@type": "Offer", "price": "9.99", "priceCurrency": "USD"}}),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = ShippingSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SHIP001"));
    }

    // ===== OfferAvailabilityAnalyzer =====

    #[test]
    fn test_availability_no_product() {
        let page = make_page("https://example.com");
        let ctx = make_ctx_with_body(&page, Some(200), "<html><body>Hello</body></html>");
        assert!(OfferAvailabilityAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_availability_in_stock_schema_out_of_stock_page() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product", "name": "Widget", "offers": {"@type": "Offer", "availability": "https://schema.org/InStock"}}),
        }];
        let ctx = make_ctx_with_body(&page, Some(200), "<html><body>This product is out of stock</body></html>");
        assert!(OfferAvailabilityAnalyzer::new().analyze(&ctx).iter().any(|f| f.code == "AVAIL001"));
    }

    #[test]
    fn test_availability_out_of_stock_schema_in_stock_page() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product", "name": "Widget", "offers": {"@type": "Offer", "availability": "https://schema.org/OutOfStock"}}),
        }];
        let ctx = make_ctx_with_body(&page, Some(200), "<html><body>Add to cart now!</body></html>");
        assert!(OfferAvailabilityAnalyzer::new().analyze(&ctx).iter().any(|f| f.code == "AVAIL002"));
    }

    #[test]
    fn test_availability_consistent_in_stock() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product", "name": "Widget", "offers": {"@type": "Offer", "availability": "https://schema.org/InStock"}}),
        }];
        let ctx = make_ctx_with_body(&page, Some(200), "<html><body>In stock, add to cart</body></html>");
        assert!(OfferAvailabilityAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_availability_consistent_out_of_stock() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product", "name": "Widget", "offers": {"@type": "Offer", "availability": "https://schema.org/OutOfStock"}}),
        }];
        let ctx = make_ctx_with_body(&page, Some(200), "<html><body>Sorry, this is out of stock</body></html>");
        assert!(OfferAvailabilityAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_availability_no_availability_in_schema() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product", "name": "Widget", "offers": {"@type": "Offer", "price": "9.99"}}),
        }];
        let ctx = make_ctx_with_body(&page, Some(200), "<html><body>This product is out of stock</body></html>");
        assert!(OfferAvailabilityAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_availability_sold_out_indicator() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product", "name": "Widget", "offers": {"@type": "Offer", "availability": "https://schema.org/InStock"}}),
        }];
        let ctx = make_ctx_with_body(&page, Some(200), "<html><body>Sold out! Check back later.</body></html>");
        assert!(OfferAvailabilityAnalyzer::new().analyze(&ctx).iter().any(|f| f.code == "AVAIL001"));
    }

    #[test]
    fn test_availability_buy_now_indicator() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product", "name": "Widget", "offers": {"@type": "Offer", "availability": "https://schema.org/OutOfStock"}}),
        }];
        let ctx = make_ctx_with_body(&page, Some(200), "<html><body>Buy now! Free shipping.</body></html>");
        assert!(OfferAvailabilityAnalyzer::new().analyze(&ctx).iter().any(|f| f.code == "AVAIL002"));
    }

    #[test]
    fn test_availability_offers_array_first_item() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product", "name": "Widget", "offers": [{"@type": "Offer", "availability": "https://schema.org/InStock"}]}),
        }];
        let ctx = make_ctx_with_body(&page, Some(200), "<html><body>This product is out of stock</body></html>");
        assert!(OfferAvailabilityAnalyzer::new().analyze(&ctx).iter().any(|f| f.code == "AVAIL001"));
    }

    #[test]
    fn test_availability_no_body() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product", "name": "Widget", "offers": {"@type": "Offer", "availability": "https://schema.org/InStock"}}),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(OfferAvailabilityAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_availability_multiple_products() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Product".to_string()),
                data: serde_json::json!({"@type": "Product", "name": "A", "offers": {"@type": "Offer", "availability": "https://schema.org/InStock"}}),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Product".to_string()),
                data: serde_json::json!({"@type": "Product", "name": "B", "offers": {"@type": "Offer", "availability": "https://schema.org/OutOfStock"}}),
            },
        ];
        let ctx = make_ctx_with_body(&page, Some(200), "<html><body>This product is out of stock but also buy now</body></html>");
        let f = OfferAvailabilityAnalyzer::new().analyze(&ctx);
        assert!(f.iter().any(|f| f.code == "AVAIL001"));
        assert!(f.iter().any(|f| f.code == "AVAIL002"));
    }

    // ===== CouponSchemaValidator =====

    #[test]
    fn test_coupon_no_coupon_schema() {
        let page = make_page("https://example.com");
        assert!(CouponSchemaValidator::new().analyze(&make_ctx(&page, Some(200))).is_empty());
    }

    #[test]
    fn test_coupon_missing_valid_from() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Coupon".to_string()),
            data: serde_json::json!({"@type": "Coupon", "name": "Summer Sale", "discountPercentage": "10%"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(CouponSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "COUP001"));
    }

    #[test]
    fn test_coupon_missing_discount() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Coupon".to_string()),
            data: serde_json::json!({"@type": "Coupon", "name": "Summer Sale", "validFrom": "2025-06-01"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(CouponSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "COUP002"));
    }

    #[test]
    fn test_coupon_valid_coupon() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Coupon".to_string()),
            data: serde_json::json!({"@type": "Coupon", "name": "Summer Sale", "validFrom": "2025-06-01", "discountPercentage": "10%"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(CouponSchemaValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_coupon_with_discount_amount() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Coupon".to_string()),
            data: serde_json::json!({"@type": "Coupon", "name": "Summer Sale", "validFrom": "2025-06-01", "discount": "$5 off"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(!CouponSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "COUP002"));
    }

    #[test]
    fn test_coupon_missing_both() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Coupon".to_string()),
            data: serde_json::json!({"@type": "Coupon", "name": "Summer Sale"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        let f = CouponSchemaValidator::new().analyze(&ctx);
        assert!(f.iter().any(|f| f.code == "COUP001"));
        assert!(f.iter().any(|f| f.code == "COUP002"));
    }

    #[test]
    fn test_coupon_non_coupon_schema() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product", "name": "Widget"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(CouponSchemaValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_coupon_empty_coupon_data() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Coupon".to_string()),
            data: serde_json::json!({"@type": "Coupon"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        let f = CouponSchemaValidator::new().analyze(&ctx);
        assert!(f.iter().any(|f| f.code == "COUP001"));
        assert!(f.iter().any(|f| f.code == "COUP002"));
    }

    #[test]
    fn test_coupon_multiple_coupons() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Coupon".to_string()),
                data: serde_json::json!({"@type": "Coupon", "name": "Sale 1", "validFrom": "2025-06-01", "discountPercentage": "10%"}),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Coupon".to_string()),
                data: serde_json::json!({"@type": "Coupon", "name": "Sale 2"}),
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let f = CouponSchemaValidator::new().analyze(&ctx);
        assert!(f.iter().any(|f| f.code == "COUP001"));
        assert!(f.iter().any(|f| f.code == "COUP002"));
    }

    #[test]
    fn test_coupon_with_both_discount_types() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Coupon".to_string()),
            data: serde_json::json!({"@type": "Coupon", "name": "Summer Sale", "validFrom": "2025-06-01", "discountPercentage": "10%", "discount": "$5 off"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(CouponSchemaValidator::new().analyze(&ctx).is_empty());
    }

    // ===== BreadcrumbsValidator =====

    #[test]
    fn test_breadcrumbs_empty_list() {
        let mut page = make_page("https://example.com/products");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("BreadcrumbList".to_string()),
            data: serde_json::json!({"@type": "BreadcrumbList", "itemListElement": []}),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(BreadcrumbsValidator::new().analyze(&ctx).iter().any(|f| f.code == "BREAD001"));
    }

    #[test]
    fn test_breadcrumbs_single_item() {
        let mut page = make_page("https://example.com/products");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("BreadcrumbList".to_string()),
            data: serde_json::json!({"@type": "BreadcrumbList", "itemListElement": [{"@type": "ListItem", "position": 1, "item": "https://example.com"}]}),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(BreadcrumbsValidator::new().analyze(&ctx).iter().any(|f| f.code == "BREAD001"));
    }

    #[test]
    fn test_breadcrumbs_valid() {
        let mut page = make_page("https://example.com/products/widget");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("BreadcrumbList".to_string()),
            data: serde_json::json!({"@type": "BreadcrumbList", "itemListElement": [
                {"@type": "ListItem", "position": 1, "item": "https://example.com"},
                {"@type": "ListItem", "position": 2, "item": "https://example.com/products"},
                {"@type": "ListItem", "position": 3, "item": "https://example.com/products/widget"}
            ]}),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = BreadcrumbsValidator::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "BREAD001"));
    }

    #[test]
    fn test_breadcrumbs_missing_on_deep_page() {
        let page = make_page("https://example.com/products/widget");
        let ctx = make_ctx(&page, Some(200));
        assert!(BreadcrumbsValidator::new().analyze(&ctx).iter().any(|f| f.code == "BREAD003"));
    }

    #[test]
    fn test_breadcrumbs_no_bread003_on_shallow_page() {
        let page = make_page("https://example.com/products");
        let ctx = make_ctx(&page, Some(200));
        assert!(!BreadcrumbsValidator::new().analyze(&ctx).iter().any(|f| f.code == "BREAD003"));
    }

    // ===== DuplicateContentDetector =====

    #[test]
    fn test_duplicate_title_description_high_overlap() {
        let mut page = make_page("https://example.com");
        // Title and description share 6 out of 7 unique words (>90% overlap)
        page.meta.title = Some("Premium Quality Widgets Available Here Purchase".to_string());
        page.meta.description = Some("Premium Quality Widgets Available Here Purchase Today".to_string());
        let ctx = make_ctx(&page, Some(200));
        assert!(DuplicateContentDetector::new().analyze(&ctx).iter().any(|f| f.code == "DUP001"));
    }

    #[test]
    fn test_duplicate_title_description_different() {
        let mut page = make_page("https://example.com");
        page.meta.title = Some("Best Widgets for Sale".to_string());
        page.meta.description = Some("Premium quality widgets with free shipping and 30-day returns".to_string());
        let ctx = make_ctx(&page, Some(200));
        assert!(!DuplicateContentDetector::new().analyze(&ctx).iter().any(|f| f.code == "DUP001"));
    }

    #[test]
    fn test_duplicate_description_starts_with_title() {
        let mut page = make_page("https://example.com");
        page.meta.title = Some("Widget Product Page".to_string());
        page.meta.description = Some("Widget Product Page - Learn more about our amazing widgets".to_string());
        let ctx = make_ctx(&page, Some(200));
        assert!(DuplicateContentDetector::new().analyze(&ctx).iter().any(|f| f.code == "DUP002"));
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
        assert!(!DuplicateContentDetector::new().analyze(&ctx).iter().any(|f| f.code == "DUP003"));
    }

    // ===== EventSchemaValidator =====

    #[test]
    fn test_event_missing_start_date() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Event".to_string()),
            data: serde_json::json!({"@type": "Event", "name": "Concert"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(EventSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "EVENT001"));
    }

    #[test]
    fn test_event_missing_location() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Event".to_string()),
            data: serde_json::json!({"@type": "Event", "name": "Concert", "startDate": "2025-06-01T19:00:00Z"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(EventSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "EVENT002"));
    }

    #[test]
    fn test_event_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Event".to_string()),
            data: serde_json::json!({"@type": "Event", "name": "Concert", "startDate": "2025-06-01T19:00:00Z", "location": {"@type": "Place", "name": "Venue"}, "organizer": {"@type": "Organization", "name": "Org"}}),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = EventSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    // ===== ReviewSchemaValidator =====

    #[test]
    fn test_review_missing_review_count() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product", "aggregateRating": {"@type": "AggregateRating", "ratingValue": 4.5}}),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(ReviewSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "REV001"));
    }

    #[test]
    fn test_review_rating_out_of_range() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product", "aggregateRating": {"@type": "AggregateRating", "ratingValue": 6.0, "bestRating": 5, "reviewCount": 100}}),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(ReviewSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "REV002"));
    }

    #[test]
    fn test_review_missing_author() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Review".to_string()),
            data: serde_json::json!({"@type": "Review", "reviewBody": "Great product!"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(ReviewSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "REV003"));
    }

    #[test]
    fn test_review_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("AggregateRating".to_string()),
            data: serde_json::json!({"@type": "AggregateRating", "ratingValue": 4.5, "bestRating": 5, "reviewCount": 100}),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = ReviewSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    // ===== VideoSchemaValidator =====

    #[test]
    fn test_video_missing_embed_url() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("VideoObject".to_string()),
            data: serde_json::json!({"@type": "VideoObject", "name": "Demo"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(VideoSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "VID001"));
    }

    #[test]
    fn test_video_missing_thumbnail() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("VideoObject".to_string()),
            data: serde_json::json!({"@type": "VideoObject", "name": "Demo", "embedUrl": "https://youtube.com/embed/123"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(VideoSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "VID002"));
    }

    #[test]
    fn test_video_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("VideoObject".to_string()),
            data: serde_json::json!({"@type": "VideoObject", "name": "Demo", "embedUrl": "https://youtube.com/embed/123", "thumbnailUrl": "https://example.com/thumb.jpg", "duration": "PT10M"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = VideoSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    // ===== TableOfContentsAnalyzer =====

    #[test]
    fn test_toc_missing_on_long_page() {
        let mut page = make_page("https://example.com/guide");
        page.word_count = 3000;
        page.headings = vec![
            crate::parser::Heading { level: 1, text: "Intro".to_string(), length: "Intro".len() },
            crate::parser::Heading { level: 2, text: "Section 1".to_string(), length: "Section 1".len() },
            crate::parser::Heading { level: 2, text: "Section 2".to_string(), length: "Section 2".len() },
            crate::parser::Heading { level: 2, text: "Section 3".to_string(), length: "Section 3".len() },
            crate::parser::Heading { level: 2, text: "Section 4".to_string(), length: "Section 4".len() },
            crate::parser::Heading { level: 2, text: "Section 5".to_string(), length: "Section 5".len() },
            crate::parser::Heading { level: 2, text: "Section 6".to_string(), length: "Section 6".len() },
        ];
        let ctx = make_ctx(&page, Some(200));
        assert!(TableOfContentsAnalyzer::new().analyze(&ctx).iter().any(|f| f.code == "TOC001"));
    }

    #[test]
    fn test_toc_not_flagged_on_short_page() {
        let mut page = make_page("https://example.com/about");
        page.word_count = 500;
        page.headings = vec![crate::parser::Heading { level: 1, text: "About".to_string(), length: "About".len() }];
        let ctx = make_ctx(&page, Some(200));
        assert!(!TableOfContentsAnalyzer::new().analyze(&ctx).iter().any(|f| f.code == "TOC001"));
    }

    // ===== LocalBusinessSchemaValidator =====

    #[test]
    fn test_local_business_missing_name() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("LocalBusiness".to_string()),
            data: serde_json::json!({"@type": "LocalBusiness", "address": {"@type": "PostalAddress"}}),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(LocalBusinessSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "LBIZ001"));
    }

    #[test]
    fn test_local_business_missing_address() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("LocalBusiness".to_string()),
            data: serde_json::json!({"@type": "LocalBusiness", "name": "My Shop"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(LocalBusinessSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "LBIZ002"));
    }

    #[test]
    fn test_local_business_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("LocalBusiness".to_string()),
            data: serde_json::json!({"@type": "LocalBusiness", "name": "My Shop", "address": {"@type": "PostalAddress", "streetAddress": "123 Main St"}}),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = LocalBusinessSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_local_business_subtypes_checked() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Restaurant".to_string()),
            data: serde_json::json!({"@type": "Restaurant"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = LocalBusinessSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "LBIZ001"));
    }

    // ===== LocalBusinessNapAnalyzer =====

    #[test]
    fn test_nap_missing_telephone() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("LocalBusiness".to_string()),
            data: serde_json::json!({"@type": "LocalBusiness", "name": "My Shop", "address": {"@type": "PostalAddress"}}),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(LocalBusinessNapAnalyzer::new().analyze(&ctx).iter().any(|f| f.code == "NAP001"));
    }

    #[test]
    fn test_nap_missing_opening_hours() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("LocalBusiness".to_string()),
            data: serde_json::json!({"@type": "LocalBusiness", "name": "My Shop", "telephone": "+1-555-555-5555"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(LocalBusinessNapAnalyzer::new().analyze(&ctx).iter().any(|f| f.code == "NAP002"));
    }

    #[test]
    fn test_nap_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("LocalBusiness".to_string()),
            data: serde_json::json!({"@type": "LocalBusiness", "name": "My Shop", "telephone": "+1-555-555-5555", "openingHours": "Mo-Fr 09:00-17:00"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(LocalBusinessNapAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_nap_missing_all() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("LocalBusiness".to_string()),
            data: serde_json::json!({"@type": "LocalBusiness"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = LocalBusinessNapAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "NAP001"));
        assert!(findings.iter().any(|f| f.code == "NAP002"));
    }

    #[test]
    fn test_nap_non_local_business_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product", "name": "Widget"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(LocalBusinessNapAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_nap_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        assert!(LocalBusinessNapAnalyzer::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_nap_restaurant_subtype() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Restaurant".to_string()),
            data: serde_json::json!({"@type": "Restaurant"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = LocalBusinessNapAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "NAP001"));
        assert!(findings.iter().any(|f| f.code == "NAP002"));
    }

    #[test]
    fn test_nap_opening_hours_specification_present() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("LocalBusiness".to_string()),
            data: serde_json::json!({"@type": "LocalBusiness", "name": "My Shop", "telephone": "+1-555-555-5555", "openingHoursSpecification": [{"@type": "OpeningHoursSpecification", "dayOfWeek": "Monday", "opens": "09:00", "closes": "17:00"}]}),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(!LocalBusinessNapAnalyzer::new().analyze(&ctx).iter().any(|f| f.code == "NAP002"));
    }

    #[test]
    fn test_nap_telephone_present_no_opening_hours() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("LocalBusiness".to_string()),
            data: serde_json::json!({"@type": "LocalBusiness", "name": "My Shop", "telephone": "+1-555-555-5555"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = LocalBusinessNapAnalyzer::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "NAP001"));
        assert!(findings.iter().any(|f| f.code == "NAP002"));
    }

    #[test]
    fn test_nap_multiple_businesses() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("LocalBusiness".to_string()),
                data: serde_json::json!({"@type": "LocalBusiness"}),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("LocalBusiness".to_string()),
                data: serde_json::json!({"@type": "LocalBusiness"}),
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = LocalBusinessNapAnalyzer::new().analyze(&ctx);
        assert_eq!(findings.iter().filter(|f| f.code == "NAP001").count(), 2);
        assert_eq!(findings.iter().filter(|f| f.code == "NAP002").count(), 2);
    }

    #[test]
    fn test_nap_store_subtype() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Store".to_string()),
            data: serde_json::json!({"@type": "Store"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = LocalBusinessNapAnalyzer::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "NAP001"));
        assert!(findings.iter().any(|f| f.code == "NAP002"));
    }

    #[test]
    fn test_nap_restaurant_with_both_present() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Restaurant".to_string()),
            data: serde_json::json!({
                "@type": "Restaurant",
                "name": "Pizza Place",
                "telephone": "+1-555-123-4567",
                "openingHours": "Mo-Su 11:00-22:00"
            }),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = LocalBusinessNapAnalyzer::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    // ===== FaqSchemaValidator =====

    #[test]
    fn test_faq_missing_main_entity() {
        let mut page = make_page("https://example.com/faq");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("FAQPage".to_string()),
            data: serde_json::json!({"@type": "FAQPage"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(FaqSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "FAQ001"));
    }

    #[test]
    fn test_faq_too_few_questions() {
        let mut page = make_page("https://example.com/faq");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("FAQPage".to_string()),
            data: serde_json::json!({
                "@type": "FAQPage",
                "mainEntity": [{"@type": "Question", "name": "Q1", "acceptedAnswer": {"@type": "Answer", "text": "A1"}}]
            }),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(FaqSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "FAQ002"));
    }

    #[test]
    fn test_faq_question_missing_accepted_answer() {
        let mut page = make_page("https://example.com/faq");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("FAQPage".to_string()),
            data: serde_json::json!({
                "@type": "FAQPage",
                "mainEntity": [
                    {"@type": "Question", "name": "Q1", "acceptedAnswer": {"@type": "Answer", "text": "A1"}},
                    {"@type": "Question", "name": "Q2"}
                ]
            }),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(FaqSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "FAQ003"));
    }

    #[test]
    fn test_faq_valid() {
        let mut page = make_page("https://example.com/faq");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("FAQPage".to_string()),
            data: serde_json::json!({
                "@type": "FAQPage",
                "mainEntity": [
                    {"@type": "Question", "name": "Q1", "acceptedAnswer": {"@type": "Answer", "text": "A1"}},
                    {"@type": "Question", "name": "Q2", "acceptedAnswer": {"@type": "Answer", "text": "A2"}}
                ]
            }),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(FaqSchemaValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_faq_non_faq_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({"@type": "Article", "headline": "News"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(FaqSchemaValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_faq_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        assert!(FaqSchemaValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_faq_main_entity_not_array() {
        let mut page = make_page("https://example.com/faq");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("FAQPage".to_string()),
            data: serde_json::json!({
                "@type": "FAQPage",
                "mainEntity": "not an array"
            }),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = FaqSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "FAQ002"));
    }

    #[test]
    fn test_faq_empty_main_entity_array() {
        let mut page = make_page("https://example.com/faq");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("FAQPage".to_string()),
            data: serde_json::json!({
                "@type": "FAQPage",
                "mainEntity": []
            }),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(FaqSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "FAQ002"));
    }

    #[test]
    fn test_faq_multiple_questions_missing_answers() {
        let mut page = make_page("https://example.com/faq");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("FAQPage".to_string()),
            data: serde_json::json!({
                "@type": "FAQPage",
                "mainEntity": [
                    {"@type": "Question", "name": "Q1"},
                    {"@type": "Question", "name": "Q2"}
                ]
            }),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = FaqSchemaValidator::new().analyze(&ctx);
        let faq003_count = findings.iter().filter(|f| f.code == "FAQ003").count();
        assert_eq!(faq003_count, 2);
    }

    // ===== HowToSchemaValidator =====

    #[test]
    fn test_howto_missing_name() {
        let mut page = make_page("https://example.com/howto");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("HowTo".to_string()),
            data: serde_json::json!({
                "@type": "HowTo",
                "step": [{"@type": "HowToStep", "name": "Step 1", "text": "Do this"}]
            }),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(HowToSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "HOWTO001"));
    }

    #[test]
    fn test_howto_missing_step() {
        let mut page = make_page("https://example.com/howto");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("HowTo".to_string()),
            data: serde_json::json!({
                "@type": "HowTo",
                "name": "How to bake a cake"
            }),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(HowToSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "HOWTO002"));
    }

    #[test]
    fn test_howto_step_missing_name_and_text() {
        let mut page = make_page("https://example.com/howto");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("HowTo".to_string()),
            data: serde_json::json!({
                "@type": "HowTo",
                "name": "How to bake",
                "step": [{"@type": "HowToStep"}]
            }),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(HowToSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "HOWTO003"));
    }

    #[test]
    fn test_howto_step_missing_name() {
        let mut page = make_page("https://example.com/howto");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("HowTo".to_string()),
            data: serde_json::json!({
                "@type": "HowTo",
                "name": "How to bake",
                "step": [{"@type": "HowToStep", "text": "Do this"}]
            }),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = HowToSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "HOWTO003"));
        assert!(!findings.iter().any(|f| f.code == "HOWTO001"));
    }

    #[test]
    fn test_howto_step_missing_text() {
        let mut page = make_page("https://example.com/howto");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("HowTo".to_string()),
            data: serde_json::json!({
                "@type": "HowTo",
                "name": "How to bake",
                "step": [{"@type": "HowToStep", "name": "Step 1"}]
            }),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = HowToSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "HOWTO003"));
        assert!(!findings.iter().any(|f| f.code == "HOWTO001"));
    }

    #[test]
    fn test_howto_valid() {
        let mut page = make_page("https://example.com/howto");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("HowTo".to_string()),
            data: serde_json::json!({
                "@type": "HowTo",
                "name": "How to bake a cake",
                "step": [
                    {"@type": "HowToStep", "name": "Prep", "text": "Preheat oven"},
                    {"@type": "HowToStep", "name": "Bake", "text": "Put in oven"}
                ]
            }),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(HowToSchemaValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_howto_non_howto_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Event".to_string()),
            data: serde_json::json!({"@type": "Event", "name": "Concert"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(HowToSchemaValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_howto_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        assert!(HowToSchemaValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_howto_multiple_steps_missing_properties() {
        let mut page = make_page("https://example.com/howto");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("HowTo".to_string()),
            data: serde_json::json!({
                "@type": "HowTo",
                "name": "How to bake",
                "step": [
                    {"@type": "HowToStep", "text": "Step 1"},
                    {"@type": "HowToStep", "name": "Step 2"}
                ]
            }),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = HowToSchemaValidator::new().analyze(&ctx);
        let howto003_count = findings.iter().filter(|f| f.code == "HOWTO003").count();
        assert_eq!(howto003_count, 2);
    }

    #[test]
    fn test_howto_missing_all_fields() {
        let mut page = make_page("https://example.com/howto");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("HowTo".to_string()),
            data: serde_json::json!({"@type": "HowTo"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = HowToSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "HOWTO001"));
        assert!(findings.iter().any(|f| f.code == "HOWTO002"));
    }

    // ===== SpeakableSchemaValidator =====

    #[test]
    fn test_speakable_missing_xpath() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("WebPage".to_string()),
            data: serde_json::json!({
                "@type": "WebPage",
                "speakable": {"cssSelector": ".intro"}
            }),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(SpeakableSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "SPEAK001"));
    }

    #[test]
    fn test_speakable_missing_css_selector() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("WebPage".to_string()),
            data: serde_json::json!({
                "@type": "WebPage",
                "speakable": {"xpath": ["/html/body/h1"]}
            }),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(SpeakableSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "SPEAK002"));
    }

    #[test]
    fn test_speakable_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("WebPage".to_string()),
            data: serde_json::json!({
                "@type": "WebPage",
                "speakable": {"xpath": ["/html/body/h1"], "cssSelector": ".intro"}
            }),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(SpeakableSchemaValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_speakable_no_speakable_property() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("WebPage".to_string()),
            data: serde_json::json!({"@type": "WebPage", "name": "Home"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(SpeakableSchemaValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_speakable_array_form() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("WebPage".to_string()),
            data: serde_json::json!({
                "@type": "WebPage",
                "speakable": [
                    {"xpath": ["/html/body/h1"]},
                    {"cssSelector": ".intro"}
                ]
            }),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = SpeakableSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SPEAK002"));
    }

    #[test]
    fn test_speakable_array_form_both_missing() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("WebPage".to_string()),
            data: serde_json::json!({
                "@type": "WebPage",
                "speakable": [{"@type": "SpeakableSpecification"}]
            }),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = SpeakableSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SPEAK001"));
        assert!(findings.iter().any(|f| f.code == "SPEAK002"));
    }

    #[test]
    fn test_speakable_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        assert!(SpeakableSchemaValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_speakable_array_form_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("WebPage".to_string()),
            data: serde_json::json!({
                "@type": "WebPage",
                "speakable": [
                    {"xpath": ["/html/body/h1"], "cssSelector": ".intro"},
                    {"xpath": ["/html/body/p"], "cssSelector": "main"}
                ]
            }),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(SpeakableSchemaValidator::new().analyze(&ctx).is_empty());
    }

    // ===== DatasetSchemaValidator =====

    #[test]
    fn test_dataset_missing_name() {
        let mut page = make_page("https://example.com/data");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Dataset".to_string()),
            data: serde_json::json!({
                "@type": "Dataset",
                "description": "A dataset about weather"
            }),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(DatasetSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "DATA001"));
    }

    #[test]
    fn test_dataset_missing_description() {
        let mut page = make_page("https://example.com/data");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Dataset".to_string()),
            data: serde_json::json!({
                "@type": "Dataset",
                "name": "Weather Data"
            }),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(DatasetSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "DATA002"));
    }

    #[test]
    fn test_dataset_missing_distribution() {
        let mut page = make_page("https://example.com/data");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Dataset".to_string()),
            data: serde_json::json!({
                "@type": "Dataset",
                "name": "Weather Data",
                "description": "Daily weather data"
            }),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(DatasetSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "DATA002"));
    }

    #[test]
    fn test_dataset_valid() {
        let mut page = make_page("https://example.com/data");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Dataset".to_string()),
            data: serde_json::json!({
                "@type": "Dataset",
                "name": "Weather Data",
                "description": "Daily weather data",
                "distribution": {"@type": "DataDownload", "contentUrl": "https://example.com/data.csv"}
            }),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(DatasetSchemaValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_dataset_non_dataset_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({"@type": "Article", "headline": "News"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(DatasetSchemaValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_dataset_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        assert!(DatasetSchemaValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_dataset_missing_all_fields() {
        let mut page = make_page("https://example.com/data");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Dataset".to_string()),
            data: serde_json::json!({"@type": "Dataset"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = DatasetSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "DATA001"));
        assert!(findings.iter().any(|f| f.code == "DATA002"));
    }

    #[test]
    fn test_dataset_multiple_datasets() {
        let mut page = make_page("https://example.com/data");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Dataset".to_string()),
                data: serde_json::json!({"@type": "Dataset"}),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Dataset".to_string()),
                data: serde_json::json!({"@type": "Dataset"}),
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = DatasetSchemaValidator::new().analyze(&ctx);
        let data001_count = findings.iter().filter(|f| f.code == "DATA001").count();
        assert_eq!(data001_count, 2);
    }

    // ===== SpecialAnnouncementSchemaValidator =====

    #[test]
    fn test_special_announcement_missing_date_posted() {
        let mut page = make_page("https://example.com/announce");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("SpecialAnnouncement".to_string()),
            data: serde_json::json!({
                "@type": "SpecialAnnouncement",
                "category": "https://schema.org/EmergencyAlert"
            }),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(SpecialAnnouncementSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "SPEC001"));
    }

    #[test]
    fn test_special_announcement_missing_category() {
        let mut page = make_page("https://example.com/announce");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("SpecialAnnouncement".to_string()),
            data: serde_json::json!({
                "@type": "SpecialAnnouncement",
                "datePosted": "2025-01-15"
            }),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(SpecialAnnouncementSchemaValidator::new().analyze(&ctx).iter().any(|f| f.code == "SPEC002"));
    }

    #[test]
    fn test_special_announcement_valid() {
        let mut page = make_page("https://example.com/announce");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("SpecialAnnouncement".to_string()),
            data: serde_json::json!({
                "@type": "SpecialAnnouncement",
                "datePosted": "2025-01-15",
                "category": "https://schema.org/EmergencyAlert"
            }),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(SpecialAnnouncementSchemaValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_special_announcement_non_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Event".to_string()),
            data: serde_json::json!({"@type": "Event", "name": "Concert"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(SpecialAnnouncementSchemaValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_special_announcement_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        assert!(SpecialAnnouncementSchemaValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_special_announcement_missing_all_fields() {
        let mut page = make_page("https://example.com/announce");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("SpecialAnnouncement".to_string()),
            data: serde_json::json!({"@type": "SpecialAnnouncement"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = SpecialAnnouncementSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SPEC001"));
        assert!(findings.iter().any(|f| f.code == "SPEC002"));
    }

    #[test]
    fn test_special_announcement_multiple_announcements() {
        let mut page = make_page("https://example.com/announce");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("SpecialAnnouncement".to_string()),
                data: serde_json::json!({"@type": "SpecialAnnouncement"}),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("SpecialAnnouncement".to_string()),
                data: serde_json::json!({"@type": "SpecialAnnouncement"}),
            },
        ];
        let ctx = make_ctx(&page, Some(200));
        let findings = SpecialAnnouncementSchemaValidator::new().analyze(&ctx);
        let spec001_count = findings.iter().filter(|f| f.code == "SPEC001").count();
        let spec002_count = findings.iter().filter(|f| f.code == "SPEC002").count();
        assert_eq!(spec001_count, 2);
        assert_eq!(spec002_count, 2);
    }

    // ===== SoftwareApplicationValidator =====

    #[test]
    fn test_software_missing_operating_system() {
        let mut page = make_page("https://example.com/app");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("SoftwareApplication".to_string()),
            data: serde_json::json!({
                "@type": "SoftwareApplication",
                "name": "My App",
                "applicationCategory": "https://schema.org/GameApplication",
                "offers": {"@type": "Offer", "price": "0", "priceCurrency": "USD"}
            }),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(SoftwareApplicationValidator::new().analyze(&ctx).iter().any(|f| f.code == "SOFT001"));
    }

    #[test]
    fn test_software_missing_offers() {
        let mut page = make_page("https://example.com/app");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("SoftwareApplication".to_string()),
            data: serde_json::json!({
                "@type": "SoftwareApplication",
                "name": "My App",
                "operatingSystem": "Windows"
            }),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(SoftwareApplicationValidator::new().analyze(&ctx).iter().any(|f| f.code == "SOFT002"));
    }

    #[test]
    fn test_software_valid() {
        let mut page = make_page("https://example.com/app");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("SoftwareApplication".to_string()),
            data: serde_json::json!({
                "@type": "SoftwareApplication",
                "name": "My App",
                "operatingSystem": "Windows",
                "offers": {"@type": "Offer", "price": "0", "priceCurrency": "USD"}
            }),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(SoftwareApplicationValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_software_non_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({"@type": "Product", "name": "Widget"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(SoftwareApplicationValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_software_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page, Some(200));
        assert!(SoftwareApplicationValidator::new().analyze(&ctx).is_empty());
    }

    #[test]
    fn test_software_offers_array_with_price() {
        let mut page = make_page("https://example.com/app");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("SoftwareApplication".to_string()),
            data: serde_json::json!({
                "@type": "SoftwareApplication",
                "name": "My App",
                "operatingSystem": "iOS",
                "applicationCategory": "https://schema.org/GameApplication",
                "offers": [{"@type": "Offer", "price": "2.99", "priceCurrency": "USD"}]
            }),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(!SoftwareApplicationValidator::new().analyze(&ctx).iter().any(|f| f.code == "SOFT003"));
    }

    #[test]
    fn test_software_offers_array_without_price() {
        let mut page = make_page("https://example.com/app");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("SoftwareApplication".to_string()),
            data: serde_json::json!({
                "@type": "SoftwareApplication",
                "name": "My App",
                "operatingSystem": "Android",
                "offers": [{"@type": "Offer", "availability": "https://schema.org/InStock"}]
            }),
        }];
        let ctx = make_ctx(&page, Some(200));
        assert!(!SoftwareApplicationValidator::new().analyze(&ctx).iter().any(|f| f.code == "SOFT002"));
    }

    #[test]
    fn test_software_missing_all_fields() {
        let mut page = make_page("https://example.com/app");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("SoftwareApplication".to_string()),
            data: serde_json::json!({"@type": "SoftwareApplication"}),
        }];
        let ctx = make_ctx(&page, Some(200));
        let findings = SoftwareApplicationValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SOFT001"));
        assert!(findings.iter().any(|f| f.code == "SOFT002"));
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
                && (sd.data.is_object()
                    && sd.data.as_object().map_or(false, |m| m.is_empty()));

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
        page.meta.description = Some("A completely different description for the page content".to_string());
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

/// Validates Apartment/Residence structured data for completeness.
pub struct ApartmentSchemaValidator;

impl Default for ApartmentSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ApartmentSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for ApartmentSchemaValidator {
    fn name(&self) -> &str {
        "apartment-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Apartment") {
                continue;
            }
            let data = &sd.data;

            if data.get("name").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "APT001".to_string(),
                    title: "Apartment schema missing name".to_string(),
                    description: "An Apartment structured data block is missing the \"name\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with the apartment or residence name."
                        .to_string(),
                });
            }

            if data.get("numberOfRooms").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "APT002".to_string(),
                    title: "Apartment schema missing numberOfRooms".to_string(),
                    description: "An Apartment structured data block is missing the \
                                  \"numberOfRooms\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"numberOfRooms\" with the number of rooms."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// CarSchemaValidator
// =========================================================================

/// Validates Car structured data for completeness.
pub struct CarSchemaValidator;

impl Default for CarSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl CarSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for CarSchemaValidator {
    fn name(&self) -> &str {
        "car-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Car") {
                continue;
            }
            let data = &sd.data;

            if data.get("name").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "CAR001".to_string(),
                    title: "Car schema missing name".to_string(),
                    description: "A Car structured data block is missing the \"name\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with the car name or description."
                        .to_string(),
                });
            }

            if data.get("model").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "CAR002".to_string(),
                    title: "Car schema missing model".to_string(),
                    description: "A Car structured data block is missing the \"model\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"model\" with the car model name."
                        .to_string(),
                });
            }

            if data.get("manufacturer").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "CAR003".to_string(),
                    title: "Car schema missing manufacturer".to_string(),
                    description: "A Car structured data block is missing the \"manufacturer\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"manufacturer\" with the car manufacturer."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// MusicAlbumSchemaValidator
// =========================================================================

/// Validates MusicAlbum structured data for completeness.
pub struct MusicAlbumSchemaValidator;

impl Default for MusicAlbumSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl MusicAlbumSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for MusicAlbumSchemaValidator {
    fn name(&self) -> &str {
        "musicalbum-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("MusicAlbum") {
                continue;
            }
            let data = &sd.data;

            if data.get("name").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "MUSALB001".to_string(),
                    title: "MusicAlbum schema missing name".to_string(),
                    description: "A MusicAlbum structured data block is missing the \"name\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with the album title."
                        .to_string(),
                });
            }

            if data.get("byArtist").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "MUSALB002".to_string(),
                    title: "MusicAlbum schema missing byArtist".to_string(),
                    description: "A MusicAlbum structured data block is missing the \"byArtist\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"byArtist\" with the artist (Person or Organization)."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// TVSeriesSchemaValidator
// =========================================================================

/// Validates TVSeries structured data for completeness.
pub struct TVSeriesSchemaValidator;

impl Default for TVSeriesSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl TVSeriesSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for TVSeriesSchemaValidator {
    fn name(&self) -> &str {
        "tvseries-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("TVSeries") {
                continue;
            }
            let data = &sd.data;

            if data.get("name").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "TV001".to_string(),
                    title: "TVSeries schema missing name".to_string(),
                    description: "A TVSeries structured data block is missing the \"name\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with the TV series title."
                        .to_string(),
                });
            }

            if data.get("numberOfEpisodes").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "TV002".to_string(),
                    title: "TVSeries schema missing numberOfEpisodes".to_string(),
                    description: "A TVSeries structured data block is missing the \
                                  \"numberOfEpisodes\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"numberOfEpisodes\" with the total episode count."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// MovieSchemaValidator
// =========================================================================

/// Validates Movie structured data for completeness.
pub struct MovieSchemaValidator;

impl Default for MovieSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl MovieSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for MovieSchemaValidator {
    fn name(&self) -> &str {
        "movie-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Movie") {
                continue;
            }
            let data = &sd.data;

            if data.get("name").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "MOVIE001".to_string(),
                    title: "Movie schema missing name".to_string(),
                    description: "A Movie structured data block is missing the \"name\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with the movie title."
                        .to_string(),
                });
            }

            if data.get("director").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "MOVIE002".to_string(),
                    title: "Movie schema missing director".to_string(),
                    description: "A Movie structured data block is missing the \"director\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"director\" with the movie director."
                        .to_string(),
                });
            }

            if data.get("dateCreated").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "MOVIE003".to_string(),
                    title: "Movie schema missing dateCreated".to_string(),
                    description: "A Movie structured data block is missing the \"dateCreated\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"dateCreated\" with the movie release date."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// GovernmentServiceSchemaValidator
// =========================================================================

/// Validates GovernmentService structured data for completeness.
pub struct GovernmentServiceSchemaValidator;

impl Default for GovernmentServiceSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl GovernmentServiceSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for GovernmentServiceSchemaValidator {
    fn name(&self) -> &str {
        "government-service-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("GovernmentService") {
                continue;
            }
            let data = &sd.data;

            if data.get("name").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "GOV001".to_string(),
                    title: "GovernmentService schema missing name".to_string(),
                    description: "A GovernmentService structured data block is missing the \
                                  \"name\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with the government service name."
                        .to_string(),
                });
            }

            if data.get("provider").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "GOV002".to_string(),
                    title: "GovernmentService schema missing provider".to_string(),
                    description: "A GovernmentService structured data block is missing the \
                                  \"provider\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"provider\" with the government agency providing the \
                                     service."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// HealthPlanSchemaValidator
// =========================================================================

/// Validates HealthPlan structured data for completeness.
pub struct HealthPlanSchemaValidator;

impl Default for HealthPlanSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl HealthPlanSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for HealthPlanSchemaValidator {
    fn name(&self) -> &str {
        "healthplan-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("HealthPlan") {
                continue;
            }
            let data = &sd.data;

            if data.get("name").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "HP001".to_string(),
                    title: "HealthPlan schema missing name".to_string(),
                    description: "A HealthPlan structured data block is missing the \"name\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with the health plan name."
                        .to_string(),
                });
            }

            if data.get("provider").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "HP002".to_string(),
                    title: "HealthPlan schema missing provider".to_string(),
                    description: "A HealthPlan structured data block is missing the \"provider\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"provider\" with the insurance provider."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// InvoiceSchemaValidator
// =========================================================================

/// Validates Invoice structured data for completeness.
pub struct InvoiceSchemaValidator;

impl Default for InvoiceSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl InvoiceSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for InvoiceSchemaValidator {
    fn name(&self) -> &str {
        "invoice-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Invoice") {
                continue;
            }
            let data = &sd.data;

            if data.get("accountId").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "INV001".to_string(),
                    title: "Invoice schema missing accountId".to_string(),
                    description: "An Invoice structured data block is missing the \"accountId\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"accountId\" with the account identifier."
                        .to_string(),
                });
            }

            if data.get("dueDate").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "INV002".to_string(),
                    title: "Invoice schema missing dueDate".to_string(),
                    description: "An Invoice structured data block is missing the \"dueDate\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"dueDate\" with the invoice due date in ISO 8601 \
                                     format."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// PermitSchemaValidator
// =========================================================================

/// Validates Permit structured data for completeness.
pub struct PermitSchemaValidator;

impl Default for PermitSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl PermitSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for PermitSchemaValidator {
    fn name(&self) -> &str {
        "permit-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Permit") {
                continue;
            }
            let data = &sd.data;

            if data.get("permitNumber").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "PERMIT001".to_string(),
                    title: "Permit schema missing permitNumber".to_string(),
                    description: "A Permit structured data block is missing the \"permitNumber\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"permitNumber\" with the permit identification number."
                        .to_string(),
                });
            }

            if data.get("issuedBy").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "PERMIT002".to_string(),
                    title: "Permit schema missing issuedBy".to_string(),
                    description: "A Permit structured data block is missing the \"issuedBy\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"issuedBy\" with the issuing authority."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// PlanSchemaValidator
// =========================================================================

/// Validates Plan structured data for completeness.
pub struct PlanSchemaValidator;

impl Default for PlanSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl PlanSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for PlanSchemaValidator {
    fn name(&self) -> &str {
        "plan-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Plan") {
                continue;
            }
            let data = &sd.data;

            if data.get("name").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "PLAN001".to_string(),
                    title: "Plan schema missing name".to_string(),
                    description: "A Plan structured data block is missing the \"name\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with the plan name."
                        .to_string(),
                });
            }

            if data.get("description").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "PLAN002".to_string(),
                    title: "Plan schema missing description".to_string(),
                    description: "A Plan structured data block is missing the \"description\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"description\" with details about the plan."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// ProductModelSchemaValidator
// =========================================================================

/// Validates ProductModel structured data for completeness.
pub struct ProductModelSchemaValidator;

impl Default for ProductModelSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ProductModelSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for ProductModelSchemaValidator {
    fn name(&self) -> &str {
        "productmodel-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("ProductModel") {
                continue;
            }
            let data = &sd.data;

            if data.get("name").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "PMODEL001".to_string(),
                    title: "ProductModel schema missing name".to_string(),
                    description: "A ProductModel structured data block is missing the \"name\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with the product model name."
                        .to_string(),
                });
            }

            if data.get("brand").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "PMODEL002".to_string(),
                    title: "ProductModel schema missing brand".to_string(),
                    description: "A ProductModel structured data block is missing the \"brand\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"brand\" with the product brand."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// ResearchProjectSchemaValidator
// =========================================================================

/// Validates ResearchProject structured data for completeness.
pub struct ResearchProjectSchemaValidator;

impl Default for ResearchProjectSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ResearchProjectSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for ResearchProjectSchemaValidator {
    fn name(&self) -> &str {
        "researchproject-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("ResearchProject") {
                continue;
            }
            let data = &sd.data;

            if data.get("name").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "RPROJ001".to_string(),
                    title: "ResearchProject schema missing name".to_string(),
                    description: "A ResearchProject structured data block is missing the \"name\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with the research project name."
                        .to_string(),
                });
            }

            if data.get("about").is_none() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "RPROJ002".to_string(),
                    title: "ResearchProject schema missing about".to_string(),
                    description: "A ResearchProject structured data block is missing the \"about\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"about\" with the topic or subject of the project."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// ScheduleSchemaValidator
// =========================================================================

/// Validates Schedule structured data for completeness.
pub struct ScheduleSchemaValidator;

impl Default for ScheduleSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ScheduleSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for ScheduleSchemaValidator {
    fn name(&self) -> &str {
        "schedule-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Schedule") {
                continue;
            }
            let data = &sd.data;

            if data.get("name").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "SCHED001".to_string(),
                    title: "Schedule schema missing name".to_string(),
                    description: "A Schedule structured data block is missing the \"name\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with the schedule name or label."
                        .to_string(),
                });
            }

            if data.get("scheduleTimezone").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "SCHED002".to_string(),
                    title: "Schedule schema missing scheduleTimezone".to_string(),
                    description: "A Schedule structured data block is missing the \
                                  \"scheduleTimezone\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"scheduleTimezone\" with the IANA timezone (e.g., \
                                     \"America/New_York\")."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// TripSchemaValidator
// =========================================================================

/// Validates Trip structured data for completeness.
pub struct TripSchemaValidator;

impl Default for TripSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl TripSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for TripSchemaValidator {
    fn name(&self) -> &str {
        "trip-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Trip") {
                continue;
            }
            let data = &sd.data;

            if data.get("name").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "TRIP001".to_string(),
                    title: "Trip schema missing name".to_string(),
                    description: "A Trip structured data block is missing the \"name\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with the trip name or title."
                        .to_string(),
                });
            }

            if data.get("itinerary").is_none() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "TRIP002".to_string(),
                    title: "Trip schema missing itinerary".to_string(),
                    description: "A Trip structured data block is missing the \"itinerary\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"itinerary\" with the trip itinerary details."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// WorkersUnionSchemaValidator
// =========================================================================

/// Validates WorkersUnion structured data for completeness.
pub struct WorkersUnionSchemaValidator;

impl Default for WorkersUnionSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkersUnionSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for WorkersUnionSchemaValidator {
    fn name(&self) -> &str {
        "workersunion-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("WorkersUnion") {
                continue;
            }
            let data = &sd.data;

            if data.get("name").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "WUNION001".to_string(),
                    title: "WorkersUnion schema missing name".to_string(),
                    description: "A WorkersUnion structured data block is missing the \"name\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with the workers union name."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// WebAPISchemaValidator
// =========================================================================

/// Validates WebAPI structured data for completeness.
pub struct WebAPISchemaValidator;

impl Default for WebAPISchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl WebAPISchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for WebAPISchemaValidator {
    fn name(&self) -> &str {
        "webapi-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("WebAPI") {
                continue;
            }
            let data = &sd.data;

            if data.get("name").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "WAPI001".to_string(),
                    title: "WebAPI schema missing name".to_string(),
                    description: "A WebAPI structured data block is missing the \"name\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with the API name."
                        .to_string(),
                });
            }

            if data.get("documentation").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding {
                    severity: Severity::Info,
                    category: IssueCategory::Schema,
                    code: "WAPI002".to_string(),
                    title: "WebAPI schema missing documentation".to_string(),
                    description: "A WebAPI structured data block is missing the \"documentation\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"documentation\" with a URL to the API documentation."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// WearableSchemaValidator
// =========================================================================

/// Validates Wearable structured data for completeness.
pub struct WearableSchemaValidator;

impl Default for WearableSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl WearableSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for WearableSchemaValidator {
    fn name(&self) -> &str {
        "wearable-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Wearable") {
                continue;
            }
            let data = &sd.data;

            if data.get("name").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "WEAR001".to_string(),
                    title: "Wearable schema missing name".to_string(),
                    description: "A Wearable structured data block is missing the \"name\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with the wearable device name."
                        .to_string(),
                });
            }

            if data.get("deviceType").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "WEAR002".to_string(),
                    title: "Wearable schema missing deviceType".to_string(),
                    description: "A Wearable structured data block is missing the \"deviceType\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"deviceType\" with the type of wearable device (e.g., \
                                     \"Smartwatch\", \"FitnessTracker\")."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// WebPageElementSchemaValidator
// =========================================================================

/// Validates WebPageElement structured data for completeness.
pub struct WebPageElementSchemaValidator;

impl Default for WebPageElementSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl WebPageElementSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for WebPageElementSchemaValidator {
    fn name(&self) -> &str {
        "webpageelement-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("WebPageElement") {
                continue;
            }
            let data = &sd.data;

            if data.get("name").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "WELEM001".to_string(),
                    title: "WebPageElement schema missing name".to_string(),
                    description: "A WebPageElement structured data block is missing the \"name\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with the element name or label."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// WebSiteSchemaValidator
// =========================================================================

/// Validates WebSite structured data for completeness.
pub struct WebSiteSchemaValidator;

impl Default for WebSiteSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl WebSiteSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for WebSiteSchemaValidator {
    fn name(&self) -> &str {
        "website-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("WebSite") {
                continue;
            }
            let data = &sd.data;

            if data.get("name").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "WSITE001".to_string(),
                    title: "WebSite schema missing name".to_string(),
                    description: "A WebSite structured data block is missing the \"name\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with the website name."
                        .to_string(),
                });
            }

            if data.get("url").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "WSITE002".to_string(),
                    title: "WebSite schema missing url".to_string(),
                    description: "A WebSite structured data block is missing the \"url\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"url\" with the website URL."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// =========================================================================
// WorkerSchemaValidator
// =========================================================================

/// Validates Worker structured data for completeness.
pub struct WorkerSchemaValidator;

impl Default for WorkerSchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkerSchemaValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for WorkerSchemaValidator {
    fn name(&self) -> &str {
        "worker-schema"
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        let url = &ctx.page.url;

        for sd in &ctx.page.structured_data {
            if sd.r#type.as_deref() != Some("Worker") {
                continue;
            }
            let data = &sd.data;

            if data.get("name").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "WORKER001".to_string(),
                    title: "Worker schema missing name".to_string(),
                    description: "A Worker structured data block is missing the \"name\" property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"name\" with the worker's name."
                        .to_string(),
                });
            }

            if data.get("jobTitle").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Schema,
                    code: "WORKER002".to_string(),
                    title: "Worker schema missing jobTitle".to_string(),
                    description: "A Worker structured data block is missing the \"jobTitle\" \
                                  property."
                        .to_string(),
                    url: url.clone(),
                    recommendation: "Add \"jobTitle\" with the worker's job title."
                        .to_string(),
                });
            }
        }

        findings
    }
}

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

    #[test]
    fn test_article_missing_headline() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Article",
                "author": "John"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ArticleSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ART001"));
    }

    #[test]
    fn test_article_missing_date_published() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Article",
                "headline": "Test",
                "author": "John"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ArticleSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ART002"));
    }

    #[test]
    fn test_article_missing_author() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Article",
                "headline": "Test",
                "datePublished": "2024-01-01"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ArticleSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ART003"));
    }

    #[test]
    fn test_article_all_present() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Article",
                "headline": "Test",
                "datePublished": "2024-01-01",
                "author": "John"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ArticleSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_article_missing_all_three() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Article"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ArticleSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ART001"));
        assert!(findings.iter().any(|f| f.code == "ART002"));
        assert!(findings.iter().any(|f| f.code == "ART003"));
    }

    #[test]
    fn test_article_news_article_type() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("NewsArticle".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "NewsArticle"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ArticleSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ART001"));
    }

    #[test]
    fn test_article_blog_posting_type() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("BlogPosting".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "BlogPosting"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ArticleSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ART001"));
        assert!(findings.iter().any(|f| f.code == "ART003"));
    }

    #[test]
    fn test_article_no_schema_no_findings() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page);
        let findings = ArticleSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_article_non_article_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Product"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ArticleSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    // ---- OrganizationSchemaValidator ----

    #[test]
    fn test_org_missing_name() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Organization".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Organization",
                "url": "https://example.com"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = OrganizationSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ORG001"));
    }

    #[test]
    fn test_org_missing_url() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Organization".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Organization",
                "name": "Acme Corp"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = OrganizationSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ORG002"));
    }

    #[test]
    fn test_org_missing_logo() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Organization".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Organization",
                "name": "Acme Corp",
                "url": "https://example.com"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = OrganizationSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ORG003"));
    }

    #[test]
    fn test_org_all_present() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Organization".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Organization",
                "name": "Acme Corp",
                "url": "https://example.com",
                "logo": "https://example.com/logo.png"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = OrganizationSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_org_missing_all() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Organization".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Organization"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = OrganizationSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ORG001"));
        assert!(findings.iter().any(|f| f.code == "ORG002"));
        assert!(findings.iter().any(|f| f.code == "ORG003"));
    }

    #[test]
    fn test_org_non_org_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Person".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Person"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = OrganizationSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_org_no_schema_no_findings() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page);
        let findings = OrganizationSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    // ---- PersonSchemaValidator ----

    #[test]
    fn test_person_missing_name() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Person".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Person"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = PersonSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PERS001"));
    }

    #[test]
    fn test_person_missing_same_as() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Person".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Person",
                "name": "John Doe"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = PersonSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PERS002"));
    }

    #[test]
    fn test_person_all_present() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Person".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Person",
                "name": "John Doe",
                "sameAs": ["https://twitter.com/johndoe"]
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = PersonSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_person_missing_both() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Person".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Person"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = PersonSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PERS001"));
        assert!(findings.iter().any(|f| f.code == "PERS002"));
    }

    #[test]
    fn test_person_non_person_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Article"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = PersonSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_person_no_schema_no_findings() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page);
        let findings = PersonSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    // ---- JobPostingSchemaValidator ----

    #[test]
    fn test_job_missing_title() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("JobPosting".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "JobPosting"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = JobPostingSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "JOB001"));
    }

    #[test]
    fn test_job_missing_date_posted() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("JobPosting".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "JobPosting",
                "title": "Engineer"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = JobPostingSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "JOB002"));
    }

    #[test]
    fn test_job_missing_valid_through() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("JobPosting".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "JobPosting",
                "title": "Engineer",
                "datePosted": "2024-01-01"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = JobPostingSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "JOB003"));
    }

    #[test]
    fn test_job_all_present() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("JobPosting".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "JobPosting",
                "title": "Engineer",
                "datePosted": "2024-01-01",
                "validThrough": "2024-12-31"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = JobPostingSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_job_missing_all() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("JobPosting".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "JobPosting"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = JobPostingSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "JOB001"));
        assert!(findings.iter().any(|f| f.code == "JOB002"));
        assert!(findings.iter().any(|f| f.code == "JOB003"));
    }

    #[test]
    fn test_job_non_job_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Product"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = JobPostingSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_job_no_schema_no_findings() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page);
        let findings = JobPostingSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    // ---- CourseSchemaValidator ----

    #[test]
    fn test_course_missing_name() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Course".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Course"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = CourseSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "COURSE001"));
    }

    #[test]
    fn test_course_missing_provider() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Course".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Course",
                "name": "Rust 101"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = CourseSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "COURSE002"));
    }

    #[test]
    fn test_course_all_present() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Course".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Course",
                "name": "Rust 101",
                "provider": {"@type": "Organization", "name": "Acme U"}
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = CourseSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_course_missing_both() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Course".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Course"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = CourseSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "COURSE001"));
        assert!(findings.iter().any(|f| f.code == "COURSE002"));
    }

    #[test]
    fn test_course_no_schema_no_findings() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page);
        let findings = CourseSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    // ---- RecipeSchemaValidator ----

    #[test]
    fn test_recipe_missing_name() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Recipe".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Recipe"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = RecipeSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "RECIPE001"));
    }

    #[test]
    fn test_recipe_missing_cook_time() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Recipe".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Recipe",
                "name": "Cake"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = RecipeSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "RECIPE002"));
    }

    #[test]
    fn test_recipe_missing_ingredients() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Recipe".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Recipe",
                "name": "Cake",
                "cookTime": "PT30M"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = RecipeSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "RECIPE003"));
    }

    #[test]
    fn test_recipe_all_present() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Recipe".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Recipe",
                "name": "Cake",
                "cookTime": "PT30M",
                "recipeIngredient": ["flour", "sugar"]
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = RecipeSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_recipe_missing_all() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Recipe".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Recipe"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = RecipeSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "RECIPE001"));
        assert!(findings.iter().any(|f| f.code == "RECIPE002"));
        assert!(findings.iter().any(|f| f.code == "RECIPE003"));
    }

    #[test]
    fn test_recipe_no_schema_no_findings() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page);
        let findings = RecipeSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

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

    #[test]
    fn test_webpage_missing_name() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("WebPage".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "WebPage"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = WebPageSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "WEBPG001"));
    }

    #[test]
    fn test_webpage_missing_date_published() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("WebPage".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "WebPage",
                "name": "My Page"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = WebPageSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "WEBPG002"));
    }

    #[test]
    fn test_webpage_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("WebPage".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "WebPage",
                "name": "My Page",
                "datePublished": "2024-01-01"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = WebPageSchemaValidator::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "WEBPG001"));
        assert!(!findings.iter().any(|f| f.code == "WEBPG002"));
    }

    #[test]
    fn test_webpage_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page);
        let findings = WebPageSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_webpage_non_webpage_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Article".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Article"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = WebPageSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_webpage_multiple_webpages() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("WebPage".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "WebPage",
                    "name": "Page 1",
                    "datePublished": "2024-01-01"
                }),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("WebPage".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "WebPage"
                }),
            },
        ];
        let ctx = make_ctx(&page);
        let findings = WebPageSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "WEBPG001"));
        assert!(findings.iter().any(|f| f.code == "WEBPG002"));
    }

    #[test]
    fn test_webpage_name_empty_string() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("WebPage".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "WebPage",
                "name": ""
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = WebPageSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "WEBPG001"));
    }

    #[test]
    fn test_webpage_name_only_no_date() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("WebPage".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "WebPage",
                "name": "About Us"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = WebPageSchemaValidator::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "WEBPG001"));
        assert!(findings.iter().any(|f| f.code == "WEBPG002"));
    }

    #[test]
    fn test_webpage_date_only_no_name() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("WebPage".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "WebPage",
                "datePublished": "2024-06-15"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = WebPageSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "WEBPG001"));
        assert!(!findings.iter().any(|f| f.code == "WEBPG002"));
    }

    #[test]
    fn test_webpage_both_missing() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("WebPage".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "WebPage"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = WebPageSchemaValidator::new().analyze(&ctx);
        assert_eq!(findings.len(), 2);
        assert!(findings.iter().any(|f| f.code == "WEBPG001"));
        assert!(findings.iter().any(|f| f.code == "WEBPG002"));
    }

    // =========================================================================
    // ServiceSchemaValidator tests
    // =========================================================================

    #[test]
    fn test_service_missing_name() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Service".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Service"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ServiceSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SVC001"));
    }

    #[test]
    fn test_service_missing_provider() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Service".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Service",
                "name": "Web Hosting"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ServiceSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SVC002"));
    }

    #[test]
    fn test_service_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Service".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Service",
                "name": "Web Hosting",
                "provider": {"@type": "Organization", "name": "Acme Corp"}
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ServiceSchemaValidator::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "SVC001"));
        assert!(!findings.iter().any(|f| f.code == "SVC002"));
    }

    #[test]
    fn test_service_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page);
        let findings = ServiceSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_service_non_service_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Product",
                "name": "Widget"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ServiceSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_service_both_missing() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Service".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Service"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ServiceSchemaValidator::new().analyze(&ctx);
        assert_eq!(findings.len(), 2);
        assert!(findings.iter().any(|f| f.code == "SVC001"));
        assert!(findings.iter().any(|f| f.code == "SVC002"));
    }

    #[test]
    fn test_service_name_empty_string() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Service".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Service",
                "name": ""
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ServiceSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SVC001"));
    }

    #[test]
    fn test_service_provider_is_string() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Service".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Service",
                "name": "Cloud Storage",
                "provider": "Acme Corp"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ServiceSchemaValidator::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "SVC001"));
        assert!(!findings.iter().any(|f| f.code == "SVC002"));
    }

    #[test]
    fn test_service_multiple_services() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Service".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "Service",
                    "name": "Valid Service",
                    "provider": {"@type": "Organization", "name": "Corp"}
                }),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Service".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "Service"
                }),
            },
        ];
        let ctx = make_ctx(&page);
        let findings = ServiceSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SVC001"));
        assert!(findings.iter().any(|f| f.code == "SVC002"));
    }

    #[test]
    fn test_service_name_only_no_provider() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Service".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Service",
                "name": "SEO Audit"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ServiceSchemaValidator::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "SVC001"));
        assert!(findings.iter().any(|f| f.code == "SVC002"));
    }

    // =========================================================================
    // ItemListSchemaValidator tests
    // =========================================================================

    #[test]
    fn test_itemlist_missing_item_list_element() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("ItemList".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "ItemList"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ItemListSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ITEMLIST001"));
    }

    #[test]
    fn test_itemlist_item_list_element_empty() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("ItemList".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "ItemList",
                "itemListElement": []
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ItemListSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ITEMLIST002"));
    }

    #[test]
    fn test_itemlist_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("ItemList".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "ItemList",
                "itemListElement": [
                    {"@type": "ListItem", "position": 1, "name": "Item 1"}
                ]
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ItemListSchemaValidator::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "ITEMLIST001"));
        assert!(!findings.iter().any(|f| f.code == "ITEMLIST002"));
    }

    #[test]
    fn test_itemlist_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page);
        let findings = ItemListSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_itemlist_non_itemlist_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("BreadcrumbList".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "BreadcrumbList"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ItemListSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_itemlist_both_issues() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("ItemList".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "ItemList"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ItemListSchemaValidator::new().analyze(&ctx);
        // Missing itemListElement entirely fires only ITEMLIST001
        assert!(findings.iter().any(|f| f.code == "ITEMLIST001"));
        assert!(!findings.iter().any(|f| f.code == "ITEMLIST002"));
    }

    #[test]
    fn test_itemlist_multiple_itemlists() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("ItemList".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "ItemList",
                    "itemListElement": [{"@type": "ListItem", "position": 1}]
                }),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("ItemList".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "ItemList"
                }),
            },
        ];
        let ctx = make_ctx(&page);
        let findings = ItemListSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ITEMLIST001"));
    }

    #[test]
    fn test_itemlist_null_item_list_element() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("ItemList".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "ItemList",
                "itemListElement": null
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ItemListSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ITEMLIST001"));
    }

    #[test]
    fn test_itemlist_item_list_element_string_instead_of_array() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("ItemList".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "ItemList",
                "itemListElement": "not-an-array"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ItemListSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ITEMLIST001"));
    }

    #[test]
    fn test_itemlist_single_item_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("ItemList".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "ItemList",
                "itemListElement": [
                    {"@type": "ListItem", "position": 1, "name": "Only Item"}
                ]
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ItemListSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    // =========================================================================
    // OfferSchemaValidator tests
    // =========================================================================

    #[test]
    fn test_offer_missing_price() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Offer".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Offer",
                "priceCurrency": "USD",
                "availability": "https://schema.org/InStock"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = OfferSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "OFFER001"));
    }

    #[test]
    fn test_offer_missing_price_currency() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Offer".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Offer",
                "price": 29.99,
                "availability": "https://schema.org/InStock"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = OfferSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "OFFER002"));
    }

    #[test]
    fn test_offer_missing_availability() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Offer".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Offer",
                "price": 29.99,
                "priceCurrency": "USD"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = OfferSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "OFFER003"));
    }

    #[test]
    fn test_offer_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Offer".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Offer",
                "price": 29.99,
                "priceCurrency": "USD",
                "availability": "https://schema.org/InStock"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = OfferSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_offer_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page);
        let findings = OfferSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_offer_non_offer_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Product",
                "name": "Widget"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = OfferSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_offer_all_missing() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Offer".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Offer"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = OfferSchemaValidator::new().analyze(&ctx);
        assert_eq!(findings.len(), 3);
        assert!(findings.iter().any(|f| f.code == "OFFER001"));
        assert!(findings.iter().any(|f| f.code == "OFFER002"));
        assert!(findings.iter().any(|f| f.code == "OFFER003"));
    }

    #[test]
    fn test_offer_price_zero() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Offer".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Offer",
                "price": 0,
                "priceCurrency": "USD",
                "availability": "https://schema.org/InStock"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = OfferSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_offer_price_string() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Offer".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Offer",
                "price": "29.99",
                "priceCurrency": "USD",
                "availability": "https://schema.org/InStock"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = OfferSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_offer_price_only_no_currency_no_availability() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Offer".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Offer",
                "price": 19.99
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = OfferSchemaValidator::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "OFFER001"));
        assert!(findings.iter().any(|f| f.code == "OFFER002"));
        assert!(findings.iter().any(|f| f.code == "OFFER003"));
    }

    // =========================================================================
    // AggregateOfferSchemaValidator tests
    // =========================================================================

    #[test]
    fn test_aggregate_offer_missing_low_price() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("AggregateOffer".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "AggregateOffer",
                "priceCurrency": "USD"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = AggregateOfferSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "AGGOFFER001"));
    }

    #[test]
    fn test_aggregate_offer_missing_price_currency() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("AggregateOffer".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "AggregateOffer",
                "lowPrice": 9.99
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = AggregateOfferSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "AGGOFFER002"));
    }

    #[test]
    fn test_aggregate_offer_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("AggregateOffer".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "AggregateOffer",
                "lowPrice": 9.99,
                "priceCurrency": "USD"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = AggregateOfferSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_aggregate_offer_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page);
        let findings = AggregateOfferSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_aggregate_offer_non_aggregate_offer_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Offer".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Offer",
                "price": 10
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = AggregateOfferSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_aggregate_offer_both_missing() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("AggregateOffer".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "AggregateOffer"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = AggregateOfferSchemaValidator::new().analyze(&ctx);
        assert_eq!(findings.len(), 2);
        assert!(findings.iter().any(|f| f.code == "AGGOFFER001"));
        assert!(findings.iter().any(|f| f.code == "AGGOFFER002"));
    }

    #[test]
    fn test_aggregate_offer_low_price_zero() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("AggregateOffer".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "AggregateOffer",
                "lowPrice": 0,
                "priceCurrency": "USD"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = AggregateOfferSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_aggregate_offer_with_high_price() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("AggregateOffer".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "AggregateOffer",
                "lowPrice": 9.99,
                "highPrice": 99.99,
                "priceCurrency": "EUR"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = AggregateOfferSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_aggregate_offer_multiple_aggregate_offers() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("AggregateOffer".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "AggregateOffer",
                    "lowPrice": 5,
                    "priceCurrency": "USD"
                }),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("AggregateOffer".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "AggregateOffer"
                }),
            },
        ];
        let ctx = make_ctx(&page);
        let findings = AggregateOfferSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "AGGOFFER001"));
        assert!(findings.iter().any(|f| f.code == "AGGOFFER002"));
    }

    #[test]
    fn test_aggregate_offer_string_low_price() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("AggregateOffer".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "AggregateOffer",
                "lowPrice": "9.99",
                "priceCurrency": "USD"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = AggregateOfferSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    // =========================================================================
    // BrandSchemaValidator tests
    // =========================================================================

    #[test]
    fn test_brand_missing_name() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Brand".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Brand"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = BrandSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "BRAND001"));
    }

    #[test]
    fn test_brand_missing_url() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Brand".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Brand",
                "name": "Acme"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = BrandSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "BRAND002"));
    }

    #[test]
    fn test_brand_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Brand".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Brand",
                "name": "Acme",
                "url": "https://acme.com"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = BrandSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_brand_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page);
        let findings = BrandSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_brand_non_brand_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Product",
                "name": "Widget"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = BrandSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_brand_both_missing() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Brand".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Brand"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = BrandSchemaValidator::new().analyze(&ctx);
        assert_eq!(findings.len(), 2);
        assert!(findings.iter().any(|f| f.code == "BRAND001"));
        assert!(findings.iter().any(|f| f.code == "BRAND002"));
    }

    #[test]
    fn test_brand_name_empty_string() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Brand".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Brand",
                "name": ""
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = BrandSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "BRAND001"));
    }

    #[test]
    fn test_brand_url_empty_string() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Brand".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Brand",
                "name": "Acme",
                "url": ""
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = BrandSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "BRAND002"));
    }

    #[test]
    fn test_brand_multiple_brands() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Brand".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "Brand",
                    "name": "GoodBrand",
                    "url": "https://good.com"
                }),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Brand".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "Brand"
                }),
            },
        ];
        let ctx = make_ctx(&page);
        let findings = BrandSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "BRAND001"));
        assert!(findings.iter().any(|f| f.code == "BRAND002"));
    }

    #[test]
    fn test_brand_name_only_no_url() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Brand".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Brand",
                "name": "SuperBrand"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = BrandSchemaValidator::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "BRAND001"));
        assert!(findings.iter().any(|f| f.code == "BRAND002"));
    }

    // =========================================================================
    // OccupationSchemaValidator tests
    // =========================================================================

    #[test]
    fn test_occupation_missing_name() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Occupation".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Occupation"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = OccupationSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "OCCUP001"));
    }

    #[test]
    fn test_occupation_missing_occupational_category() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Occupation".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Occupation",
                "name": "Software Engineer"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = OccupationSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "OCCUP002"));
    }

    #[test]
    fn test_occupation_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Occupation".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Occupation",
                "name": "Software Engineer",
                "occupationalCategory": "15-1252.00"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = OccupationSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_occupation_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page);
        let findings = OccupationSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_occupation_non_occupation_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Person".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Person",
                "name": "Jane"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = OccupationSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_occupation_both_missing() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Occupation".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Occupation"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = OccupationSchemaValidator::new().analyze(&ctx);
        assert_eq!(findings.len(), 2);
        assert!(findings.iter().any(|f| f.code == "OCCUP001"));
        assert!(findings.iter().any(|f| f.code == "OCCUP002"));
    }

    #[test]
    fn test_occupation_name_empty() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Occupation".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Occupation",
                "name": ""
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = OccupationSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "OCCUP001"));
    }

    #[test]
    fn test_occupation_category_empty() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Occupation".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Occupation",
                "name": "Doctor",
                "occupationalCategory": ""
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = OccupationSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "OCCUP002"));
    }

    #[test]
    fn test_occupation_multiple() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Occupation".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "Occupation",
                    "name": "Engineer",
                    "occupationalCategory": "17-2000"
                }),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Occupation".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "Occupation"
                }),
            },
        ];
        let ctx = make_ctx(&page);
        let findings = OccupationSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "OCCUP001"));
        assert!(findings.iter().any(|f| f.code == "OCCUP002"));
    }

    #[test]
    fn test_occupation_category_as_object() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Occupation".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Occupation",
                "name": "Nurse",
                "occupationalCategory": {"@type": "CategoryCode", "codeValue": "29-1141"}
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = OccupationSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    // =========================================================================
    // QuestSchemaValidator tests
    // =========================================================================

    #[test]
    fn test_quest_missing_name() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Quest".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Quest"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = QuestSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "QUEST001"));
    }

    #[test]
    fn test_quest_missing_quest_type() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Quest".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Quest",
                "name": "Find the Dragon"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = QuestSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "QUEST002"));
    }

    #[test]
    fn test_quest_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Quest".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Quest",
                "name": "Find the Dragon",
                "questType": "Main Quest"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = QuestSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_quest_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page);
        let findings = QuestSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_quest_non_quest_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Game".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Game"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = QuestSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_quest_both_missing() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Quest".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Quest"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = QuestSchemaValidator::new().analyze(&ctx);
        assert_eq!(findings.len(), 2);
        assert!(findings.iter().any(|f| f.code == "QUEST001"));
        assert!(findings.iter().any(|f| f.code == "QUEST002"));
    }

    #[test]
    fn test_quest_name_empty() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Quest".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Quest",
                "name": ""
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = QuestSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "QUEST001"));
    }

    #[test]
    fn test_quest_quest_type_empty() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Quest".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Quest",
                "name": "Defeat Boss",
                "questType": ""
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = QuestSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "QUEST002"));
    }

    #[test]
    fn test_quest_multiple() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Quest".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "Quest",
                    "name": "Tutorial",
                    "questType": "Tutorial"
                }),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Quest".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "Quest"
                }),
            },
        ];
        let ctx = make_ctx(&page);
        let findings = QuestSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "QUEST001"));
        assert!(findings.iter().any(|f| f.code == "QUEST002"));
    }

    #[test]
    fn test_quest_name_only_no_type() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Quest".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Quest",
                "name": "Collect Gems"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = QuestSchemaValidator::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "QUEST001"));
        assert!(findings.iter().any(|f| f.code == "QUEST002"));
    }

    // =========================================================================
    // ActionSchemaValidator tests
    // =========================================================================

    #[test]
    fn test_action_missing_action_type() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Action".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Action"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ActionSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ACTION001"));
    }

    #[test]
    fn test_action_missing_target() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Action".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Action",
                "actionType": "BuyAction"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ActionSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ACTION002"));
    }

    #[test]
    fn test_action_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Action".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Action",
                "actionType": "BuyAction",
                "target": "https://example.com/buy"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ActionSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_action_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page);
        let findings = ActionSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_action_non_action_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Product"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ActionSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_action_both_missing() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Action".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Action"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ActionSchemaValidator::new().analyze(&ctx);
        assert_eq!(findings.len(), 2);
        assert!(findings.iter().any(|f| f.code == "ACTION001"));
        assert!(findings.iter().any(|f| f.code == "ACTION002"));
    }

    #[test]
    fn test_action_action_type_empty() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Action".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Action",
                "actionType": ""
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ActionSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ACTION001"));
    }

    #[test]
    fn test_action_target_empty() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Action".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Action",
                "actionType": "ViewAction",
                "target": ""
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ActionSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ACTION002"));
    }

    #[test]
    fn test_action_target_as_entry_point() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Action".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Action",
                "actionType": "BuyAction",
                "target": {
                    "@type": "EntryPoint",
                    "urlTemplate": "https://example.com/buy"
                }
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ActionSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_action_multiple_actions() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Action".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "Action",
                    "actionType": "BuyAction",
                    "target": "https://example.com/buy"
                }),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Action".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "Action"
                }),
            },
        ];
        let ctx = make_ctx(&page);
        let findings = ActionSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ACTION001"));
        assert!(findings.iter().any(|f| f.code == "ACTION002"));
    }

    // =========================================================================
    // PlaybookSchemaValidator tests
    // =========================================================================

    #[test]
    fn test_playbook_missing_name() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Playbook".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Playbook"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = PlaybookSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PLAYBOOK001"));
    }

    #[test]
    fn test_playbook_missing_step() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Playbook".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Playbook",
                "name": "Quick Start Guide"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = PlaybookSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PLAYBOOK002"));
    }

    #[test]
    fn test_playbook_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Playbook".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Playbook",
                "name": "Quick Start Guide",
                "step": [
                    {"@type": "HowToStep", "text": "Step 1"}
                ]
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = PlaybookSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_playbook_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page);
        let findings = PlaybookSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_playbook_non_playbook_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("HowTo".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "HowTo"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = PlaybookSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_playbook_both_missing() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Playbook".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Playbook"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = PlaybookSchemaValidator::new().analyze(&ctx);
        assert_eq!(findings.len(), 2);
        assert!(findings.iter().any(|f| f.code == "PLAYBOOK001"));
        assert!(findings.iter().any(|f| f.code == "PLAYBOOK002"));
    }

    #[test]
    fn test_playbook_name_empty() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Playbook".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Playbook",
                "name": ""
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = PlaybookSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PLAYBOOK001"));
    }

    #[test]
    fn test_playbook_step_empty_array() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Playbook".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Playbook",
                "name": "Guide",
                "step": []
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = PlaybookSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PLAYBOOK002"));
    }

    #[test]
    fn test_playbook_step_null() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Playbook".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Playbook",
                "name": "Guide",
                "step": null
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = PlaybookSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PLAYBOOK002"));
    }

    #[test]
    fn test_playbook_multiple() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Playbook".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "Playbook",
                    "name": "Good Guide",
                    "step": [{"@type": "HowToStep", "text": "Do this"}]
                }),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Playbook".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "Playbook"
                }),
            },
        ];
        let ctx = make_ctx(&page);
        let findings = PlaybookSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PLAYBOOK001"));
        assert!(findings.iter().any(|f| f.code == "PLAYBOOK002"));
    }

    #[test]
    fn test_playbook_name_only_no_step() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Playbook".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Playbook",
                "name": "Deployment Playbook"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = PlaybookSchemaValidator::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "PLAYBOOK001"));
        assert!(findings.iter().any(|f| f.code == "PLAYBOOK002"));
    }

    // =========================================================================
    // ApartmentSchemaValidator tests
    // =========================================================================

    #[test]
    fn test_apartment_missing_name() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Apartment".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Apartment",
                "numberOfRooms": 3
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ApartmentSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "APT001"));
    }

    #[test]
    fn test_apartment_missing_numberofrooms() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Apartment".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Apartment",
                "name": "Sunny Flat"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ApartmentSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "APT002"));
    }

    #[test]
    fn test_apartment_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Apartment".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Apartment",
                "name": "Sunny Flat",
                "numberOfRooms": 3
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ApartmentSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_apartment_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page);
        let findings = ApartmentSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_apartment_non_apartment_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Product",
                "name": "Widget"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ApartmentSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_apartment_both_missing() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Apartment".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Apartment"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ApartmentSchemaValidator::new().analyze(&ctx);
        assert_eq!(findings.len(), 2);
        assert!(findings.iter().any(|f| f.code == "APT001"));
        assert!(findings.iter().any(|f| f.code == "APT002"));
    }

    #[test]
    fn test_apartment_name_empty() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Apartment".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Apartment",
                "name": "",
                "numberOfRooms": 2
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ApartmentSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "APT001"));
    }

    #[test]
    fn test_apartment_multiple() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Apartment".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "Apartment",
                    "name": "Good Apartment",
                    "numberOfRooms": 2
                }),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Apartment".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "Apartment"
                }),
            },
        ];
        let ctx = make_ctx(&page);
        let findings = ApartmentSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "APT001"));
        assert!(findings.iter().any(|f| f.code == "APT002"));
    }

    // =========================================================================
    // CarSchemaValidator tests
    // =========================================================================

    #[test]
    fn test_car_missing_name() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Car".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Car",
                "model": "Model 3",
                "manufacturer": "Tesla"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = CarSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "CAR001"));
    }

    #[test]
    fn test_car_missing_model() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Car".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Car",
                "name": "Electric Sedan"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = CarSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "CAR002"));
    }

    #[test]
    fn test_car_missing_manufacturer() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Car".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Car",
                "name": "Electric Sedan",
                "model": "Model 3"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = CarSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "CAR003"));
    }

    #[test]
    fn test_car_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Car".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Car",
                "name": "Electric Sedan",
                "model": "Model 3",
                "manufacturer": "Tesla"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = CarSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_car_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page);
        let findings = CarSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_car_non_car_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Product"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = CarSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_car_all_missing() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Car".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Car"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = CarSchemaValidator::new().analyze(&ctx);
        assert_eq!(findings.len(), 3);
    }

    #[test]
    fn test_car_name_empty() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Car".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Car",
                "name": "",
                "model": "Civic",
                "manufacturer": "Honda"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = CarSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "CAR001"));
    }

    // =========================================================================
    // MusicAlbumSchemaValidator tests
    // =========================================================================

    #[test]
    fn test_musicalbum_missing_name() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("MusicAlbum".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "MusicAlbum",
                "byArtist": {"@type": "Person", "name": "Artist"}
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = MusicAlbumSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "MUSALB001"));
    }

    #[test]
    fn test_musicalbum_missing_byartist() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("MusicAlbum".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "MusicAlbum",
                "name": "Thriller"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = MusicAlbumSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "MUSALB002"));
    }

    #[test]
    fn test_musicalbum_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("MusicAlbum".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "MusicAlbum",
                "name": "Thriller",
                "byArtist": {"@type": "Person", "name": "Michael Jackson"}
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = MusicAlbumSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_musicalbum_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page);
        let findings = MusicAlbumSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_musicalbum_non_musicalbum_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Movie".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Movie"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = MusicAlbumSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_musicalbum_both_missing() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("MusicAlbum".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "MusicAlbum"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = MusicAlbumSchemaValidator::new().analyze(&ctx);
        assert_eq!(findings.len(), 2);
    }

    #[test]
    fn test_musicalbum_name_empty() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("MusicAlbum".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "MusicAlbum",
                "name": ""
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = MusicAlbumSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "MUSALB001"));
    }

    #[test]
    fn test_musicalbum_multiple() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("MusicAlbum".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "MusicAlbum",
                    "name": "Good Album",
                    "byArtist": {"@type": "Person", "name": "Artist"}
                }),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("MusicAlbum".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "MusicAlbum"
                }),
            },
        ];
        let ctx = make_ctx(&page);
        let findings = MusicAlbumSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "MUSALB001"));
        assert!(findings.iter().any(|f| f.code == "MUSALB002"));
    }

    // =========================================================================
    // TVSeriesSchemaValidator tests
    // =========================================================================

    #[test]
    fn test_tvseries_missing_name() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("TVSeries".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "TVSeries",
                "numberOfEpisodes": 10
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = TVSeriesSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "TV001"));
    }

    #[test]
    fn test_tvseries_missing_numberofepisodes() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("TVSeries".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "TVSeries",
                "name": "Breaking Bad"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = TVSeriesSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "TV002"));
    }

    #[test]
    fn test_tvseries_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("TVSeries".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "TVSeries",
                "name": "Breaking Bad",
                "numberOfEpisodes": 62
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = TVSeriesSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_tvseries_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page);
        let findings = TVSeriesSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_tvseries_non_tvseries_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Movie".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Movie"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = TVSeriesSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_tvseries_both_missing() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("TVSeries".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "TVSeries"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = TVSeriesSchemaValidator::new().analyze(&ctx);
        assert_eq!(findings.len(), 2);
    }

    #[test]
    fn test_tvseries_name_empty() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("TVSeries".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "TVSeries",
                "name": ""
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = TVSeriesSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "TV001"));
    }

    #[test]
    fn test_tvseries_multiple() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("TVSeries".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "TVSeries",
                    "name": "Good Series"
                }),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("TVSeries".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "TVSeries"
                }),
            },
        ];
        let ctx = make_ctx(&page);
        let findings = TVSeriesSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "TV001"));
        assert!(findings.iter().any(|f| f.code == "TV002"));
    }

    // =========================================================================
    // MovieSchemaValidator tests
    // =========================================================================

    #[test]
    fn test_movie_missing_name() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Movie".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Movie",
                "director": "Spielberg"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = MovieSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "MOVIE001"));
    }

    #[test]
    fn test_movie_missing_director() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Movie".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Movie",
                "name": "E.T."
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = MovieSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "MOVIE002"));
    }

    #[test]
    fn test_movie_missing_datecreated() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Movie".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Movie",
                "name": "E.T.",
                "director": "Spielberg"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = MovieSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "MOVIE003"));
    }

    #[test]
    fn test_movie_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Movie".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Movie",
                "name": "E.T.",
                "director": "Spielberg",
                "dateCreated": "1982-06-11"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = MovieSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_movie_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page);
        let findings = MovieSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_movie_non_movie_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Product"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = MovieSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_movie_all_missing() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Movie".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Movie"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = MovieSchemaValidator::new().analyze(&ctx);
        assert_eq!(findings.len(), 3);
    }

    #[test]
    fn test_movie_name_empty() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Movie".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Movie",
                "name": ""
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = MovieSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "MOVIE001"));
    }

    // =========================================================================
    // GovernmentServiceSchemaValidator tests
    // =========================================================================

    #[test]
    fn test_gov_missing_name() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("GovernmentService".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "GovernmentService"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = GovernmentServiceSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "GOV001"));
    }

    #[test]
    fn test_gov_missing_provider() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("GovernmentService".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "GovernmentService",
                "name": "DMV"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = GovernmentServiceSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "GOV002"));
    }

    #[test]
    fn test_gov_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("GovernmentService".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "GovernmentService",
                "name": "DMV",
                "provider": {"@type": "GovernmentOrganization", "name": "State Gov"}
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = GovernmentServiceSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_gov_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page);
        let findings = GovernmentServiceSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_gov_non_gov_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Product"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = GovernmentServiceSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_gov_both_missing() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("GovernmentService".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "GovernmentService"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = GovernmentServiceSchemaValidator::new().analyze(&ctx);
        assert_eq!(findings.len(), 2);
    }

    #[test]
    fn test_gov_name_empty() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("GovernmentService".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "GovernmentService",
                "name": ""
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = GovernmentServiceSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "GOV001"));
    }

    #[test]
    fn test_gov_multiple() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("GovernmentService".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "GovernmentService",
                    "name": "Good Service",
                    "provider": {"@type": "GovernmentOrganization"}
                }),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("GovernmentService".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "GovernmentService"
                }),
            },
        ];
        let ctx = make_ctx(&page);
        let findings = GovernmentServiceSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "GOV001"));
        assert!(findings.iter().any(|f| f.code == "GOV002"));
    }

    // =========================================================================
    // HealthPlanSchemaValidator tests
    // =========================================================================

    #[test]
    fn test_healthplan_missing_name() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("HealthPlan".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "HealthPlan"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = HealthPlanSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "HP001"));
    }

    #[test]
    fn test_healthplan_missing_provider() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("HealthPlan".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "HealthPlan",
                "name": "Gold Plan"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = HealthPlanSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "HP002"));
    }

    #[test]
    fn test_healthplan_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("HealthPlan".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "HealthPlan",
                "name": "Gold Plan",
                "provider": {"@type": "Organization", "name": "HealthCo"}
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = HealthPlanSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_healthplan_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page);
        let findings = HealthPlanSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_healthplan_non_healthplan_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Product"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = HealthPlanSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_healthplan_both_missing() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("HealthPlan".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "HealthPlan"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = HealthPlanSchemaValidator::new().analyze(&ctx);
        assert_eq!(findings.len(), 2);
    }

    #[test]
    fn test_healthplan_name_empty() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("HealthPlan".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "HealthPlan",
                "name": ""
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = HealthPlanSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "HP001"));
    }

    #[test]
    fn test_healthplan_multiple() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("HealthPlan".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "HealthPlan",
                    "name": "Good Plan",
                    "provider": {"@type": "Organization"}
                }),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("HealthPlan".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "HealthPlan"
                }),
            },
        ];
        let ctx = make_ctx(&page);
        let findings = HealthPlanSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "HP001"));
        assert!(findings.iter().any(|f| f.code == "HP002"));
    }

    // =========================================================================
    // InvoiceSchemaValidator tests
    // =========================================================================

    #[test]
    fn test_invoice_missing_accountid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Invoice".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Invoice",
                "dueDate": "2024-01-15"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = InvoiceSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "INV001"));
    }

    #[test]
    fn test_invoice_missing_duedate() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Invoice".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Invoice",
                "accountId": "INV-001"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = InvoiceSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "INV002"));
    }

    #[test]
    fn test_invoice_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Invoice".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Invoice",
                "accountId": "INV-001",
                "dueDate": "2024-01-15"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = InvoiceSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_invoice_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page);
        let findings = InvoiceSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_invoice_non_invoice_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Product"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = InvoiceSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_invoice_both_missing() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Invoice".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Invoice"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = InvoiceSchemaValidator::new().analyze(&ctx);
        assert_eq!(findings.len(), 2);
    }

    #[test]
    fn test_invoice_accountid_empty() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Invoice".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Invoice",
                "accountId": ""
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = InvoiceSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "INV001"));
    }

    #[test]
    fn test_invoice_multiple() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Invoice".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "Invoice",
                    "accountId": "INV-001",
                    "dueDate": "2024-01-15"
                }),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Invoice".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "Invoice"
                }),
            },
        ];
        let ctx = make_ctx(&page);
        let findings = InvoiceSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "INV001"));
        assert!(findings.iter().any(|f| f.code == "INV002"));
    }

    // =========================================================================
    // PermitSchemaValidator tests
    // =========================================================================

    #[test]
    fn test_permit_missing_permitnumber() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Permit".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Permit"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = PermitSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PERMIT001"));
    }

    #[test]
    fn test_permit_missing_issuedby() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Permit".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Permit",
                "permitNumber": "P-12345"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = PermitSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PERMIT002"));
    }

    #[test]
    fn test_permit_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Permit".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Permit",
                "permitNumber": "P-12345",
                "issuedBy": {"@type": "GovernmentOrganization", "name": "City Hall"}
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = PermitSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_permit_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page);
        let findings = PermitSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_permit_non_permit_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Product"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = PermitSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_permit_both_missing() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Permit".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Permit"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = PermitSchemaValidator::new().analyze(&ctx);
        assert_eq!(findings.len(), 2);
    }

    #[test]
    fn test_permit_number_empty() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Permit".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Permit",
                "permitNumber": ""
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = PermitSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PERMIT001"));
    }

    #[test]
    fn test_permit_multiple() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Permit".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "Permit",
                    "permitNumber": "P-1",
                    "issuedBy": {"@type": "GovernmentOrganization"}
                }),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Permit".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "Permit"
                }),
            },
        ];
        let ctx = make_ctx(&page);
        let findings = PermitSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PERMIT001"));
        assert!(findings.iter().any(|f| f.code == "PERMIT002"));
    }

    // =========================================================================
    // PlanSchemaValidator tests
    // =========================================================================

    #[test]
    fn test_plan_missing_name() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Plan".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Plan",
                "description": "A monthly plan"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = PlanSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PLAN001"));
    }

    #[test]
    fn test_plan_missing_description() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Plan".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Plan",
                "name": "Premium Plan"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = PlanSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PLAN002"));
    }

    #[test]
    fn test_plan_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Plan".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Plan",
                "name": "Premium Plan",
                "description": "Unlimited access"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = PlanSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_plan_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page);
        let findings = PlanSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_plan_non_plan_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Product"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = PlanSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_plan_both_missing() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Plan".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Plan"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = PlanSchemaValidator::new().analyze(&ctx);
        assert_eq!(findings.len(), 2);
    }

    #[test]
    fn test_plan_name_empty() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Plan".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Plan",
                "name": ""
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = PlanSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PLAN001"));
    }

    #[test]
    fn test_plan_multiple() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Plan".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "Plan",
                    "name": "Good Plan",
                    "description": "Details"
                }),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Plan".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "Plan"
                }),
            },
        ];
        let ctx = make_ctx(&page);
        let findings = PlanSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PLAN001"));
        assert!(findings.iter().any(|f| f.code == "PLAN002"));
    }

    // =========================================================================
    // ProductModelSchemaValidator tests
    // =========================================================================

    #[test]
    fn test_productmodel_missing_name() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("ProductModel".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "ProductModel"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ProductModelSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PMODEL001"));
    }

    #[test]
    fn test_productmodel_missing_brand() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("ProductModel".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "ProductModel",
                "name": "XPS 15"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ProductModelSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PMODEL002"));
    }

    #[test]
    fn test_productmodel_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("ProductModel".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "ProductModel",
                "name": "XPS 15",
                "brand": "Dell"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ProductModelSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_productmodel_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page);
        let findings = ProductModelSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_productmodel_non_productmodel_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Product"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ProductModelSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_productmodel_both_missing() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("ProductModel".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "ProductModel"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ProductModelSchemaValidator::new().analyze(&ctx);
        assert_eq!(findings.len(), 2);
    }

    #[test]
    fn test_productmodel_name_empty() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("ProductModel".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "ProductModel",
                "name": ""
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ProductModelSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PMODEL001"));
    }

    #[test]
    fn test_productmodel_multiple() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("ProductModel".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "ProductModel",
                    "name": "Good Model",
                    "brand": "Acme"
                }),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("ProductModel".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "ProductModel"
                }),
            },
        ];
        let ctx = make_ctx(&page);
        let findings = ProductModelSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PMODEL001"));
        assert!(findings.iter().any(|f| f.code == "PMODEL002"));
    }

    // =========================================================================
    // ResearchProjectSchemaValidator tests
    // =========================================================================

    #[test]
    fn test_researchproject_missing_name() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("ResearchProject".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "ResearchProject"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ResearchProjectSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "RPROJ001"));
    }

    #[test]
    fn test_researchproject_missing_about() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("ResearchProject".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "ResearchProject",
                "name": "Climate Study"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ResearchProjectSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "RPROJ002"));
    }

    #[test]
    fn test_researchproject_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("ResearchProject".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "ResearchProject",
                "name": "Climate Study",
                "about": "Climate Change"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ResearchProjectSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_researchproject_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page);
        let findings = ResearchProjectSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_researchproject_non_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Product"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ResearchProjectSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_researchproject_both_missing() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("ResearchProject".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "ResearchProject"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ResearchProjectSchemaValidator::new().analyze(&ctx);
        assert_eq!(findings.len(), 2);
    }

    #[test]
    fn test_researchproject_name_empty() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("ResearchProject".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "ResearchProject",
                "name": ""
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ResearchProjectSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "RPROJ001"));
    }

    #[test]
    fn test_researchproject_multiple() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("ResearchProject".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "ResearchProject",
                    "name": "Good Project",
                    "about": "Topic"
                }),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("ResearchProject".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "ResearchProject"
                }),
            },
        ];
        let ctx = make_ctx(&page);
        let findings = ResearchProjectSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "RPROJ001"));
        assert!(findings.iter().any(|f| f.code == "RPROJ002"));
    }

    // =========================================================================
    // ScheduleSchemaValidator tests
    // =========================================================================

    #[test]
    fn test_schedule_missing_name() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Schedule".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Schedule",
                "scheduleTimezone": "America/New_York"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ScheduleSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SCHED001"));
    }

    #[test]
    fn test_schedule_missing_timezone() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Schedule".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Schedule",
                "name": "Daily Standup"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ScheduleSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SCHED002"));
    }

    #[test]
    fn test_schedule_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Schedule".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Schedule",
                "name": "Daily Standup",
                "scheduleTimezone": "America/New_York"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ScheduleSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_schedule_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page);
        let findings = ScheduleSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_schedule_non_schedule_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Event".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Event"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ScheduleSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_schedule_both_missing() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Schedule".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Schedule"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ScheduleSchemaValidator::new().analyze(&ctx);
        assert_eq!(findings.len(), 2);
    }

    #[test]
    fn test_schedule_name_empty() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Schedule".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Schedule",
                "name": ""
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ScheduleSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SCHED001"));
    }

    #[test]
    fn test_schedule_multiple() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Schedule".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "Schedule",
                    "name": "Good Schedule",
                    "scheduleTimezone": "UTC"
                }),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Schedule".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "Schedule"
                }),
            },
        ];
        let ctx = make_ctx(&page);
        let findings = ScheduleSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "SCHED001"));
        assert!(findings.iter().any(|f| f.code == "SCHED002"));
    }

    // =========================================================================
    // TripSchemaValidator tests
    // =========================================================================

    #[test]
    fn test_trip_missing_name() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Trip".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Trip"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = TripSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "TRIP001"));
    }

    #[test]
    fn test_trip_missing_itinerary() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Trip".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Trip",
                "name": "Italy Vacation"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = TripSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "TRIP002"));
    }

    #[test]
    fn test_trip_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Trip".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Trip",
                "name": "Italy Vacation",
                "itinerary": {"@type": "ItemList", "numberOfItems": 5}
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = TripSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_trip_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page);
        let findings = TripSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_trip_non_trip_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Product"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = TripSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_trip_both_missing() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Trip".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Trip"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = TripSchemaValidator::new().analyze(&ctx);
        assert_eq!(findings.len(), 2);
    }

    #[test]
    fn test_trip_name_empty() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Trip".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Trip",
                "name": ""
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = TripSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "TRIP001"));
    }

    #[test]
    fn test_trip_multiple() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Trip".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "Trip",
                    "name": "Good Trip",
                    "itinerary": {"@type": "ItemList"}
                }),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Trip".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "Trip"
                }),
            },
        ];
        let ctx = make_ctx(&page);
        let findings = TripSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "TRIP001"));
        assert!(findings.iter().any(|f| f.code == "TRIP002"));
    }

    // =========================================================================
    // WorkersUnionSchemaValidator tests
    // =========================================================================

    #[test]
    fn test_workersunion_missing_name() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("WorkersUnion".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "WorkersUnion"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = WorkersUnionSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "WUNION001"));
    }

    #[test]
    fn test_workersunion_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("WorkersUnion".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "WorkersUnion",
                "name": "Steel Workers Union"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = WorkersUnionSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_workersunion_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page);
        let findings = WorkersUnionSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_workersunion_non_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Organization".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Organization"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = WorkersUnionSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_workersunion_name_empty() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("WorkersUnion".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "WorkersUnion",
                "name": ""
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = WorkersUnionSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "WUNION001"));
    }

    #[test]
    fn test_workersunion_multiple() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("WorkersUnion".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "WorkersUnion",
                    "name": "Good Union"
                }),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("WorkersUnion".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "WorkersUnion"
                }),
            },
        ];
        let ctx = make_ctx(&page);
        let findings = WorkersUnionSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "WUNION001"));
    }

    #[test]
    fn test_workersunion_both_present() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("WorkersUnion".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "WorkersUnion",
                "name": "Good Union"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = WorkersUnionSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_workersunion_with_other_schema() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("WorkersUnion".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "WorkersUnion"
                }),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Organization".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "Organization",
                    "name": "Org"
                }),
            },
        ];
        let ctx = make_ctx(&page);
        let findings = WorkersUnionSchemaValidator::new().analyze(&ctx);
        // Only WorkersUnion finding, not Organization
        assert_eq!(findings.len(), 1);
        assert!(findings.iter().any(|f| f.code == "WUNION001"));
    }

    // =========================================================================
    // WebAPISchemaValidator tests
    // =========================================================================

    #[test]
    fn test_webapi_missing_name() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("WebAPI".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "WebAPI"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = WebAPISchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "WAPI001"));
    }

    #[test]
    fn test_webapi_missing_documentation() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("WebAPI".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "WebAPI",
                "name": "Payments API"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = WebAPISchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "WAPI002"));
    }

    #[test]
    fn test_webapi_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("WebAPI".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "WebAPI",
                "name": "Payments API",
                "documentation": "https://docs.example.com/api"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = WebAPISchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_webapi_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page);
        let findings = WebAPISchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_webapi_non_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Product"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = WebAPISchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_webapi_both_missing() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("WebAPI".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "WebAPI"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = WebAPISchemaValidator::new().analyze(&ctx);
        assert_eq!(findings.len(), 2);
    }

    #[test]
    fn test_webapi_name_empty() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("WebAPI".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "WebAPI",
                "name": ""
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = WebAPISchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "WAPI001"));
    }

    #[test]
    fn test_webapi_multiple() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("WebAPI".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "WebAPI",
                    "name": "Good API",
                    "documentation": "https://docs.example.com"
                }),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("WebAPI".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "WebAPI"
                }),
            },
        ];
        let ctx = make_ctx(&page);
        let findings = WebAPISchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "WAPI001"));
        assert!(findings.iter().any(|f| f.code == "WAPI002"));
    }

    // =========================================================================
    // WearableSchemaValidator tests
    // =========================================================================

    #[test]
    fn test_wearable_missing_name() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Wearable".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Wearable"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = WearableSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "WEAR001"));
    }

    #[test]
    fn test_wearable_missing_devicetype() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Wearable".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Wearable",
                "name": "FitBand Pro"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = WearableSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "WEAR002"));
    }

    #[test]
    fn test_wearable_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Wearable".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Wearable",
                "name": "FitBand Pro",
                "deviceType": "FitnessTracker"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = WearableSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_wearable_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page);
        let findings = WearableSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_wearable_non_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Product"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = WearableSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_wearable_both_missing() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Wearable".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Wearable"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = WearableSchemaValidator::new().analyze(&ctx);
        assert_eq!(findings.len(), 2);
    }

    #[test]
    fn test_wearable_name_empty() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Wearable".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Wearable",
                "name": ""
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = WearableSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "WEAR001"));
    }

    #[test]
    fn test_wearable_multiple() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Wearable".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "Wearable",
                    "name": "Good Device",
                    "deviceType": "Smartwatch"
                }),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Wearable".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "Wearable"
                }),
            },
        ];
        let ctx = make_ctx(&page);
        let findings = WearableSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "WEAR001"));
        assert!(findings.iter().any(|f| f.code == "WEAR002"));
    }

    // =========================================================================
    // WebPageElementSchemaValidator tests
    // =========================================================================

    #[test]
    fn test_webpageelement_missing_name() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("WebPageElement".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "WebPageElement"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = WebPageElementSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "WELEM001"));
    }

    #[test]
    fn test_webpageelement_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("WebPageElement".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "WebPageElement",
                "name": "Sidebar"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = WebPageElementSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_webpageelement_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page);
        let findings = WebPageElementSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_webpageelement_non_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("WebPage".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "WebPage"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = WebPageElementSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_webpageelement_name_empty() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("WebPageElement".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "WebPageElement",
                "name": ""
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = WebPageElementSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "WELEM001"));
    }

    #[test]
    fn test_webpageelement_multiple() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("WebPageElement".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "WebPageElement",
                    "name": "Good Element"
                }),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("WebPageElement".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "WebPageElement"
                }),
            },
        ];
        let ctx = make_ctx(&page);
        let findings = WebPageElementSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "WELEM001"));
    }

    #[test]
    fn test_webpageelement_both_present() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("WebPageElement".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "WebPageElement",
                "name": "Header"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = WebPageElementSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_webpageelement_with_other_schema() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("WebPageElement".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "WebPageElement"
                }),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("WebPage".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "WebPage",
                    "name": "Page"
                }),
            },
        ];
        let ctx = make_ctx(&page);
        let findings = WebPageElementSchemaValidator::new().analyze(&ctx);
        assert_eq!(findings.len(), 1);
        assert!(findings.iter().any(|f| f.code == "WELEM001"));
    }

    // =========================================================================
    // WebSiteSchemaValidator tests
    // =========================================================================

    #[test]
    fn test_website_missing_name() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("WebSite".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "WebSite",
                "url": "https://example.com"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = WebSiteSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "WSITE001"));
    }

    #[test]
    fn test_website_missing_url() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("WebSite".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "WebSite",
                "name": "Example Site"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = WebSiteSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "WSITE002"));
    }

    #[test]
    fn test_website_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("WebSite".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "WebSite",
                "name": "Example Site",
                "url": "https://example.com"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = WebSiteSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_website_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page);
        let findings = WebSiteSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_website_non_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("WebPage".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "WebPage"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = WebSiteSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_website_both_missing() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("WebSite".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "WebSite"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = WebSiteSchemaValidator::new().analyze(&ctx);
        assert_eq!(findings.len(), 2);
    }

    #[test]
    fn test_website_name_empty() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("WebSite".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "WebSite",
                "name": ""
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = WebSiteSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "WSITE001"));
    }

    #[test]
    fn test_website_multiple() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("WebSite".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "WebSite",
                    "name": "Good Site",
                    "url": "https://example.com"
                }),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("WebSite".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "WebSite"
                }),
            },
        ];
        let ctx = make_ctx(&page);
        let findings = WebSiteSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "WSITE001"));
        assert!(findings.iter().any(|f| f.code == "WSITE002"));
    }

    // =========================================================================
    // WorkerSchemaValidator tests
    // =========================================================================

    #[test]
    fn test_worker_missing_name() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Worker".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Worker"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = WorkerSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "WORKER001"));
    }

    #[test]
    fn test_worker_missing_jobtitle() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Worker".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Worker",
                "name": "Jane Doe"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = WorkerSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "WORKER002"));
    }

    #[test]
    fn test_worker_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Worker".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Worker",
                "name": "Jane Doe",
                "jobTitle": "Engineer"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = WorkerSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_worker_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page);
        let findings = WorkerSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_worker_non_type_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Person".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Person"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = WorkerSchemaValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_worker_both_missing() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Worker".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Worker"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = WorkerSchemaValidator::new().analyze(&ctx);
        assert_eq!(findings.len(), 2);
    }

    #[test]
    fn test_worker_name_empty() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Worker".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Worker",
                "name": ""
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = WorkerSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "WORKER001"));
    }

    #[test]
    fn test_worker_multiple() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Worker".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "Worker",
                    "name": "Good Worker",
                    "jobTitle": "Manager"
                }),
            },
            StructuredData {
                context: Some("https://schema.org".to_string()),
                r#type: Some("Worker".to_string()),
                data: serde_json::json!({
                    "@context": "https://schema.org",
                    "@type": "Worker"
                }),
            },
        ];
        let ctx = make_ctx(&page);
        let findings = WorkerSchemaValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "WORKER001"));
        assert!(findings.iter().any(|f| f.code == "WORKER002"));
    }

    // =========================================================================
    // LocalBusinessHoursValidator tests
    // =========================================================================

    #[test]
    fn test_lbh_missing_opening_hours() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("LocalBusiness".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "LocalBusiness",
                "name": "My Shop"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = LocalBusinessHoursValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "LBH001"));
    }

    #[test]
    fn test_lbh_with_opening_hours_no_findings() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("LocalBusiness".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "LocalBusiness",
                "name": "My Shop",
                "openingHours": "Mo-Fr 09:00-17:00"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = LocalBusinessHoursValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_lbh_invalid_format() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("LocalBusiness".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "LocalBusiness",
                "name": "My Shop",
                "openingHours": "open all day!!!"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = LocalBusinessHoursValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "LBH002"));
    }

    #[test]
    fn test_lbh_store_type() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Store".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Store",
                "name": "My Store"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = LocalBusinessHoursValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "LBH001"));
    }

    #[test]
    fn test_lbh_restaurant_type() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Restaurant".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Restaurant",
                "name": "My Restaurant"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = LocalBusinessHoursValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "LBH001"));
    }

    #[test]
    fn test_lbh_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page);
        let findings = LocalBusinessHoursValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_lbh_non_local_type_ignored() {
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
        let ctx = make_ctx(&page);
        let findings = LocalBusinessHoursValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_lbh_opening_hours_specification_present() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("LocalBusiness".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "LocalBusiness",
                "name": "My Shop",
                "openingHoursSpecification": {
                    "@type": "OpeningHoursSpecification",
                    "dayOfWeek": "Monday",
                    "opens": "09:00",
                    "closes": "17:00"
                }
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = LocalBusinessHoursValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    // =========================================================================
    // ProductReviewValidator tests
    // =========================================================================

    #[test]
    fn test_prev_missing_review_rating() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Product",
                "name": "Widget",
                "review": {
                    "@type": "Review",
                    "author": {"@type": "Person", "name": "John"}
                }
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ProductReviewValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PREV001"));
    }

    #[test]
    fn test_prev_missing_author() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Product",
                "name": "Widget",
                "review": {
                    "@type": "Review",
                    "reviewRating": {"@type": "Rating", "ratingValue": 5}
                }
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ProductReviewValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PREV002"));
    }

    #[test]
    fn test_prev_both_missing() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Product",
                "name": "Widget",
                "review": {
                    "@type": "Review"
                }
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ProductReviewValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PREV001"));
        assert!(findings.iter().any(|f| f.code == "PREV002"));
    }

    #[test]
    fn test_prev_valid_review() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Product",
                "name": "Widget",
                "review": {
                    "@type": "Review",
                    "author": {"@type": "Person", "name": "John"},
                    "reviewRating": {"@type": "Rating", "ratingValue": 5}
                }
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ProductReviewValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_prev_no_reviews() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Product",
                "name": "Widget"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ProductReviewValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_prev_multiple_reviews() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Product",
                "name": "Widget",
                "review": [
                    {"@type": "Review"},
                    {"@type": "Review"}
                ]
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = ProductReviewValidator::new().analyze(&ctx);
        assert_eq!(findings.len(), 4);
    }

    #[test]
    fn test_prev_non_product_ignored() {
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
        let ctx = make_ctx(&page);
        let findings = ProductReviewValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_prev_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page);
        let findings = ProductReviewValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    // =========================================================================
    // EventLocationValidator tests
    // =========================================================================

    #[test]
    fn test_eloc_missing_location() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Event".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Event",
                "name": "Conference",
                "startDate": "2024-06-15"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = EventLocationValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ELOC001"));
    }

    #[test]
    fn test_eloc_location_missing_name() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Event".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Event",
                "name": "Conference",
                "startDate": "2024-06-15",
                "location": {"@type": "Place"}
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = EventLocationValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ELOC002"));
    }

    #[test]
    fn test_eloc_valid_location() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Event".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Event",
                "name": "Conference",
                "startDate": "2024-06-15",
                "location": {"@type": "Place", "name": "Convention Center"}
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = EventLocationValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_eloc_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page);
        let findings = EventLocationValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_eloc_non_event_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Product",
                "name": "Widget"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = EventLocationValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_eloc_virtual_location_no_name() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Event".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Event",
                "name": "Webinar",
                "startDate": "2024-06-15",
                "location": {"@type": "VirtualLocation"}
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = EventLocationValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "ELOC002"));
    }

    #[test]
    fn test_eloc_location_with_url() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Event".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Event",
                "name": "Webinar",
                "startDate": "2024-06-15",
                "location": {"@type": "VirtualLocation", "url": "https://zoom.us/123"}
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = EventLocationValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_eloc_location_with_address() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Event".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Event",
                "name": "Concert",
                "startDate": "2024-06-15",
                "location": {
                    "@type": "Place",
                    "address": {"@type": "PostalAddress", "streetAddress": "123 Main St"}
                }
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = EventLocationValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    // =========================================================================
    // OrganizationLogoValidator tests
    // =========================================================================

    #[test]
    fn test_ologo_missing_logo() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Organization".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Organization",
                "name": "Acme Corp"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = OrganizationLogoValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "OLOGO001"));
    }

    #[test]
    fn test_ologo_valid_logo() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Organization".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Organization",
                "name": "Acme Corp",
                "logo": "https://example.com/logo.png"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = OrganizationLogoValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_ologo_invalid_url() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Organization".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Organization",
                "name": "Acme Corp",
                "logo": "/images/logo.png"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = OrganizationLogoValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "OLOGO002"));
    }

    #[test]
    fn test_ologo_local_business_type() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("LocalBusiness".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "LocalBusiness",
                "name": "My Shop"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = OrganizationLogoValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "OLOGO001"));
    }

    #[test]
    fn test_ologo_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page);
        let findings = OrganizationLogoValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_ologo_non_org_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Person".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Person",
                "name": "Jane"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = OrganizationLogoValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_ologo_logo_object() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Organization".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Organization",
                "name": "Acme Corp",
                "logo": {
                    "@type": "ImageObject",
                    "url": "https://example.com/logo.png"
                }
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = OrganizationLogoValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_ologo_empty_logo_string() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Organization".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Organization",
                "name": "Acme Corp",
                "logo": ""
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = OrganizationLogoValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    // =========================================================================
    // PersonJobTitleValidator tests
    // =========================================================================

    #[test]
    fn test_pjob_missing_job_title() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Person".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Person",
                "name": "Jane Doe"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = PersonJobTitleValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PJOB001"));
    }

    #[test]
    fn test_pjob_missing_works_for() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Person".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Person",
                "name": "Jane Doe",
                "jobTitle": "Engineer"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = PersonJobTitleValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "PJOB002"));
    }

    #[test]
    fn test_pjob_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Person".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Person",
                "name": "Jane Doe",
                "jobTitle": "Engineer",
                "worksFor": {"@type": "Organization", "name": "Acme Corp"}
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = PersonJobTitleValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_pjob_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page);
        let findings = PersonJobTitleValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_pjob_non_person_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Product",
                "name": "Widget"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = PersonJobTitleValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_pjob_both_missing() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Person".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Person",
                "name": "Jane Doe"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = PersonJobTitleValidator::new().analyze(&ctx);
        assert_eq!(findings.len(), 2);
    }

    #[test]
    fn test_pjob_with_member_of() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Person".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Person",
                "name": "Jane Doe",
                "jobTitle": "Engineer",
                "memberOf": {"@type": "Organization", "name": "Acme Corp"}
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = PersonJobTitleValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_pjob_with_job_title_no_works_for() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Person".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Person",
                "name": "Jane Doe",
                "jobTitle": "Engineer"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = PersonJobTitleValidator::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "PJOB001"));
        assert!(findings.iter().any(|f| f.code == "PJOB002"));
    }

    // =========================================================================
    // RecipeNutritionValidator tests
    // =========================================================================

    #[test]
    fn test_rnut_missing_nutrition() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Recipe".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Recipe",
                "name": "Chocolate Cake"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = RecipeNutritionValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "RNUT001"));
    }

    #[test]
    fn test_rnut_missing_calories() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Recipe".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Recipe",
                "name": "Chocolate Cake",
                "nutrition": {"@type": "NutritionInformation"}
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = RecipeNutritionValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "RNUT002"));
    }

    #[test]
    fn test_rnut_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Recipe".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Recipe",
                "name": "Chocolate Cake",
                "nutrition": {
                    "@type": "NutritionInformation",
                    "calories": "240 calories"
                }
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = RecipeNutritionValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_rnut_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page);
        let findings = RecipeNutritionValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_rnut_non_recipe_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Product",
                "name": "Widget"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = RecipeNutritionValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_rnut_empty_nutrition_object() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Recipe".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Recipe",
                "name": "Chocolate Cake",
                "nutrition": {}
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = RecipeNutritionValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "RNUT002"));
    }

    #[test]
    fn test_rnut_both_missing() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Recipe".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Recipe",
                "name": "Chocolate Cake"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = RecipeNutritionValidator::new().analyze(&ctx);
        assert_eq!(findings.len(), 1);
        assert!(findings.iter().any(|f| f.code == "RNUT001"));
    }

    #[test]
    fn test_rnut_nutrition_with_other_fields() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Recipe".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Recipe",
                "name": "Chocolate Cake",
                "nutrition": {
                    "@type": "NutritionInformation",
                    "fatContent": "10g",
                    "proteinContent": "5g"
                }
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = RecipeNutritionValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "RNUT002"));
    }

    // =========================================================================
    // CourseProviderValidator tests
    // =========================================================================

    #[test]
    fn test_cprov_missing_provider_name() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Course".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Course",
                "name": "Rust Programming",
                "provider": {"@type": "Organization"}
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = CourseProviderValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "CPROV001"));
    }

    #[test]
    fn test_cprov_missing_provider_url() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Course".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Course",
                "name": "Rust Programming",
                "provider": {"@type": "Organization", "name": "Udemy"}
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = CourseProviderValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "CPROV002"));
    }

    #[test]
    fn test_cprov_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Course".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Course",
                "name": "Rust Programming",
                "provider": {
                    "@type": "Organization",
                    "name": "Udemy",
                    "url": "https://udemy.com"
                }
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = CourseProviderValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_cprov_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page);
        let findings = CourseProviderValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_cprov_non_course_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Product",
                "name": "Widget"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = CourseProviderValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_cprov_no_provider() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Course".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Course",
                "name": "Rust Programming"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = CourseProviderValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_cprov_both_missing() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Course".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Course",
                "name": "Rust Programming",
                "provider": {"@type": "Organization"}
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = CourseProviderValidator::new().analyze(&ctx);
        assert_eq!(findings.len(), 2);
    }

    #[test]
    fn test_cprov_provider_with_url_only() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Course".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Course",
                "name": "Rust Programming",
                "provider": {
                    "@type": "Organization",
                    "url": "https://udemy.com"
                }
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = CourseProviderValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "CPROV001"));
        assert!(!findings.iter().any(|f| f.code == "CPROV002"));
    }

    // =========================================================================
    // JobPostingSalaryValidator tests
    // =========================================================================

    #[test]
    fn test_jsal_missing_base_salary() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("JobPosting".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "JobPosting",
                "title": "Software Engineer"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = JobPostingSalaryValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "JSAL001"));
    }

    #[test]
    fn test_jsal_missing_employment_type() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("JobPosting".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "JobPosting",
                "title": "Software Engineer",
                "baseSalary": {
                    "@type": "MonetaryAmount",
                    "currency": "USD",
                    "value": {"@type": "QuantitativeValue", "value": 100000}
                }
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = JobPostingSalaryValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "JSAL002"));
    }

    #[test]
    fn test_jsal_valid() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("JobPosting".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "JobPosting",
                "title": "Software Engineer",
                "baseSalary": {
                    "@type": "MonetaryAmount",
                    "currency": "USD",
                    "value": {"@type": "QuantitativeValue", "value": 100000}
                },
                "employmentType": "FULL_TIME"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = JobPostingSalaryValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_jsal_no_structured_data() {
        let page = make_page("https://example.com");
        let ctx = make_ctx(&page);
        let findings = JobPostingSalaryValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_jsal_non_job_posting_ignored() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("Product".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "Product",
                "name": "Widget"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = JobPostingSalaryValidator::new().analyze(&ctx);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_jsal_both_missing() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("JobPosting".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "JobPosting",
                "title": "Software Engineer"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = JobPostingSalaryValidator::new().analyze(&ctx);
        assert_eq!(findings.len(), 2);
    }

    #[test]
    fn test_jsal_with_salary_no_employment_type() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("JobPosting".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "JobPosting",
                "title": "Software Engineer",
                "baseSalary": {
                    "@type": "MonetaryAmount",
                    "currency": "USD",
                    "value": {"@type": "QuantitativeValue", "value": 100000}
                }
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = JobPostingSalaryValidator::new().analyze(&ctx);
        assert!(!findings.iter().any(|f| f.code == "JSAL001"));
        assert!(findings.iter().any(|f| f.code == "JSAL002"));
    }

    #[test]
    fn test_jsal_with_employment_type_no_salary() {
        let mut page = make_page("https://example.com");
        page.structured_data = vec![StructuredData {
            context: Some("https://schema.org".to_string()),
            r#type: Some("JobPosting".to_string()),
            data: serde_json::json!({
                "@context": "https://schema.org",
                "@type": "JobPosting",
                "title": "Software Engineer",
                "employmentType": "FULL_TIME"
            }),
        }];
        let ctx = make_ctx(&page);
        let findings = JobPostingSalaryValidator::new().analyze(&ctx);
        assert!(findings.iter().any(|f| f.code == "JSAL001"));
        assert!(!findings.iter().any(|f| f.code == "JSAL002"));
    }
}
