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
}
