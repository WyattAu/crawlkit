use std::collections::HashMap;

use crate::types::{IssueCategory, Severity};
use crate::CrawlConfig;

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

    fn analyze(&self, ctx: &AnalysisContext, _config: &CrawlConfig) -> Vec<Finding> {
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

    fn analyze(&self, ctx: &AnalysisContext, _config: &CrawlConfig) -> Vec<Finding> {
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

    fn analyze(&self, ctx: &AnalysisContext, _config: &CrawlConfig) -> Vec<Finding> {
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

    fn analyze(&self, ctx: &AnalysisContext, _config: &CrawlConfig) -> Vec<Finding> {
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
