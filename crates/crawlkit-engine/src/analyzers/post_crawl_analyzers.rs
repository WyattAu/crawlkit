use std::collections::{HashMap, HashSet};

use crate::storage::{Issue, PageData};
use crate::types::{IssueCategory, Severity};
use crate::Finding;

/// Normalize a URL string for consistent HashMap keying.
///
/// `Url::as_str()` adds a trailing slash to root URLs (e.g. `https://example.com/`),
/// but link data from the crawl graph uses plain strings without trailing slashes.
/// This function strips trailing slashes to ensure consistent lookups.
fn normalize_url(url: &str) -> String {
    url.strip_suffix('/').unwrap_or(url).to_string()
}

/// Data collected during a complete crawl, available for cross-page analysis.
///
/// Constructed from [`StorageBackend`](crate::storage_trait::StorageBackend)
/// after the main crawl loop finishes so that [`PostCrawlAnalyzer`]s can
/// inspect the full crawl graph.
pub struct CrawlData {
    /// All pages stored during this crawl.
    pub pages: Vec<PageData>,
    /// Per-page link graph: `(source_url, [target_urls])`.
    pub links: Vec<(String, Vec<String>)>,
    /// All issues found across all pages.
    pub issues: Vec<Issue>,
    /// The original seed / starting URL of the crawl.
    pub seed_url: String,
}

/// Trait for analyzers that need full crawl data (not just per-page data).
///
/// Implementors receive a [`CrawlData`] snapshot after the crawl completes
/// and may return cross-page findings such as orphan detection, broken
/// internal link chains, or sitemap inconsistencies.
pub trait PostCrawlAnalyzer: Send + Sync {
    /// Returns the human-readable name of this analyzer.
    fn name(&self) -> &str;

    /// Analyze the full crawl data and return any findings.
    fn analyze_crawl(&self, data: &CrawlData) -> Vec<Finding>;
}

/// Registry of [`PostCrawlAnalyzer`] implementations.
pub struct PostCrawlAnalyzerRegistry {
    analyzers: Vec<Box<dyn PostCrawlAnalyzer>>,
}

impl PostCrawlAnalyzerRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            analyzers: Vec::new(),
        }
    }

    /// Add a post-crawl analyzer to the registry.
    pub fn register(&mut self, analyzer: Box<dyn PostCrawlAnalyzer>) {
        self.analyzers.push(analyzer);
    }

    /// Run all registered post-crawl analyzers and collect findings.
    pub fn analyze_crawl(&self, data: &CrawlData) -> Vec<Finding> {
        let mut findings: Vec<Finding> = self
            .analyzers
            .iter()
            .flat_map(|a| a.analyze_crawl(data))
            .collect();
        findings.sort_by(|a, b| a.code.cmp(&b.code).then_with(|| a.url.cmp(&b.url)));
        findings
    }

    /// Returns the number of registered analyzers.
    pub fn len(&self) -> usize {
        self.analyzers.len()
    }

    /// Returns true if no analyzers are registered.
    pub fn is_empty(&self) -> bool {
        self.analyzers.is_empty()
    }

    /// Iterate over registered analyzers.
    pub fn iter(&self) -> impl Iterator<Item = &dyn PostCrawlAnalyzer> {
        self.analyzers.iter().map(|a| a.as_ref())
    }
}

impl Default for PostCrawlAnalyzerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Build the default post-crawl analyzer registry with all built-in analyzers.
pub fn build_post_crawl_registry() -> PostCrawlAnalyzerRegistry {
    let mut registry = PostCrawlAnalyzerRegistry::new();
    registry.register(Box::new(InternalLinkGraphAnalyzer::new()));
    registry.register(Box::new(CrossPageDuplicateContentDetector::new()));
    registry.register(Box::new(CannibalizationDetector::new()));
    registry.register(Box::new(OrphanPageDetector::new()));
    registry.register(Box::new(SitemapCoverageAnalyzer::new()));
    registry.register(Box::new(LinkEquityDistributor::new()));
    registry
}

// ---------------------------------------------------------------------------
// 1. InternalLinkGraphAnalyzer
// ---------------------------------------------------------------------------

/// Computes PageRank-like scores and detects link clusters.
///
/// Checks for orphan pages, link spam, excessive navigation depth, and
/// authority concentration across the crawl graph.
pub struct InternalLinkGraphAnalyzer;

impl Default for InternalLinkGraphAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl InternalLinkGraphAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Extract the domain from a URL for same-origin comparisons.
    fn extract_domain(url: &str) -> &str {
        url.split("://").nth(1).unwrap_or(url).split('/').next().unwrap_or(url)
    }

    /// Check whether two URLs share the same origin.
    fn same_origin(a: &str, b: &str) -> bool {
        Self::extract_domain(a) == Self::extract_domain(b)
    }
}

impl PostCrawlAnalyzer for InternalLinkGraphAnalyzer {
    fn name(&self) -> &str {
        "internal-link-graph"
    }

    fn analyze_crawl(&self, data: &CrawlData) -> Vec<Finding> {
        let mut findings = Vec::new();
        if data.pages.is_empty() {
            return findings;
        }

        let normalized_seed = normalize_url(&data.seed_url);

        // Build incoming link counts (internal only), using normalized keys
        let mut incoming: HashMap<String, usize> = HashMap::new();
        // Build outgoing link counts per page
        let mut outgoing: HashMap<String, usize> = HashMap::new();
        // Total outgoing links across all pages
        let mut total_outgoing: usize = 0;

        for (source, targets) in &data.links {
            let norm_source = normalize_url(source);
            for target in targets {
                total_outgoing += 1;
                *outgoing.entry(norm_source.clone()).or_insert(0) += 1;
                if Self::same_origin(source, target) && source != target {
                    let norm_target = normalize_url(target);
                    *incoming.entry(norm_target).or_insert(0) += 1;
                }
            }
        }

        // GRAPH001: orphan pages (no incoming internal links from crawled pages)
        for page in &data.pages {
            let url_str = normalize_url(page.url.as_str());
            if incoming.get(&url_str).copied().unwrap_or(0) == 0 && url_str != normalized_seed {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "GRAPH001".to_string(),
                    title: "Orphan page detected".to_string(),
                    description: format!(
                        "Page '{}' has no incoming internal links from other crawled pages.",
                        page.url
                    ),
                    url: page.url.to_string(),
                    recommendation: "Add internal links from other pages to improve discoverability and crawlability."
                        .to_string(),
                });
            }
        }

        // GRAPH002: pages with >50 outgoing links (link spam)
        for (url, count) in &outgoing {
            if *count > 50 {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "GRAPH002".to_string(),
                    title: "Link spam detected".to_string(),
                    description: format!(
                        "Page '{url}' has {count} outgoing links, which may dilute link equity."
                    ),
                    url: url.clone(),
                    recommendation: "Reduce the number of links per page to keep link equity focused."
                        .to_string(),
                });
            }
        }

        // GRAPH003: navigation depth > 5 from seed (BFS)
        {
            let mut depth: HashMap<String, usize> = HashMap::new();
            let mut queue = std::collections::VecDeque::new();
            let norm_seed = normalize_url(&data.seed_url);
            depth.insert(norm_seed.clone(), 0);
            queue.push_back(norm_seed);

            // Build adjacency list from links
            let mut adj: HashMap<String, Vec<String>> = HashMap::new();
            for (source, targets) in &data.links {
                let norm_source = normalize_url(source);
                let internal: Vec<String> = targets
                    .iter()
                    .filter(|t| Self::same_origin(source, t))
                    .map(|t| normalize_url(t))
                    .collect();
                adj.entry(norm_source).or_default().extend(internal);
            }

            while let Some(current) = queue.pop_front() {
                let d = depth[&current];
                if let Some(neighbors) = adj.get(&current) {
                    for neighbor in neighbors {
                        if !depth.contains_key(neighbor) {
                            depth.insert(neighbor.clone(), d + 1);
                            queue.push_back(neighbor.clone());
                        }
                    }
                }
            }

            for (url, d) in &depth {
                if *d > 5 {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Seo,
                        code: "GRAPH003".to_string(),
                        title: "Deep navigation depth".to_string(),
                        description: format!(
                            "Page '{url}' is {d} links away from the seed URL, which exceeds the recommended maximum depth of 5."
                        ),
                        url: url.clone(),
                        recommendation: "Flatten site architecture or add shortcuts (e.g. sitemap, related links) to reduce navigation depth."
                            .to_string(),
                    });
                }
            }
        }

        // GRAPH004: single page receives >30% of all internal links
        if total_outgoing > 0 {
            if let Some((top_url, top_count)) =
                incoming.iter().max_by_key(|(_, c)| *c)
            {
                let pct = (*top_count as f64) / (total_outgoing as f64) * 100.0;
                if pct > 30.0 && *top_count > 1 {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Seo,
                        code: "GRAPH004".to_string(),
                        title: "Authority concentration".to_string(),
                        description: format!(
                            "Page '{top_url}' receives {top_count} internal links ({pct:.1}% of total), concentrating link authority."
                        ),
                        url: top_url.clone(),
                        recommendation: "Distribute internal links more evenly across important pages to balance authority."
                            .to_string(),
                    });
                }
            }
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// 2. CrossPageDuplicateContentDetector
// ---------------------------------------------------------------------------

/// Cross-page duplicate content detector.
///
/// Compares titles and descriptions across pages to find near-duplicates
/// that may confuse search engines.
pub struct CrossPageDuplicateContentDetector;

impl Default for CrossPageDuplicateContentDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl CrossPageDuplicateContentDetector {
    pub fn new() -> Self {
        Self
    }

    fn tokenize(text: &str) -> Vec<String> {
        text.to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty())
            .map(|w| w.to_string())
            .collect()
    }

    fn cosine_similarity(a: &[String], b: &[String]) -> f64 {
        if a.is_empty() || b.is_empty() {
            return 0.0;
        }
        let set_b: HashSet<&String> = b.iter().collect();
        let intersection = a.iter().filter(|w| set_b.contains(w)).count();
        let len_a = a.len() as f64;
        let len_b = b.len() as f64;
        if len_a == 0.0 || len_b == 0.0 {
            return 0.0;
        }
        intersection as f64 / (len_a * len_b).sqrt()
    }
}

impl PostCrawlAnalyzer for CrossPageDuplicateContentDetector {
    fn name(&self) -> &str {
        "cross-page-duplicate-content"
    }

    fn analyze_crawl(&self, data: &CrawlData) -> Vec<Finding> {
        let mut findings = Vec::new();
        let pages = &data.pages;
        if pages.len() < 2 {
            return findings;
        }

        // Check title similarity across pages
        for i in 0..pages.len() {
            for j in (i + 1)..pages.len() {
                if let (Some(ta), Some(tb)) = (&pages[i].title, &pages[j].title) {
                    let ta = ta.trim();
                    let tb = tb.trim();
                    if !ta.is_empty() && !tb.is_empty() {
                        let words_a = Self::tokenize(ta);
                        let words_b = Self::tokenize(tb);
                        let sim = Self::cosine_similarity(&words_a, &words_b);
                        if sim > 0.9 {
                            findings.push(Finding {
                                severity: Severity::Warning,
                                category: IssueCategory::Content,
                                code: "DUP-CROSS001".to_string(),
                                title: "Duplicate titles across pages".to_string(),
                                description: format!(
                                    "Pages '{}' and '{}' have similar titles ({sim:.0}% word overlap).",
                                    pages[i].url, pages[j].url
                                ),
                                url: pages[i].url.to_string(),
                                recommendation: "Use unique, descriptive titles for each page to help search engines differentiate content."
                                    .to_string(),
                            });
                        }
                    }
                }

                // Check description similarity
                if let (Some(da), Some(db)) = (&pages[i].description, &pages[j].description) {
                    let da = da.trim();
                    let db = db.trim();
                    if !da.is_empty() && !db.is_empty() {
                        let words_a = Self::tokenize(da);
                        let words_b = Self::tokenize(db);
                        let sim = Self::cosine_similarity(&words_a, &words_b);
                        if sim > 0.9 {
                            findings.push(Finding {
                                severity: Severity::Warning,
                                category: IssueCategory::Content,
                                code: "DUP-CROSS002".to_string(),
                                title: "Duplicate descriptions across pages".to_string(),
                                description: format!(
                                    "Pages '{}' and '{}' have similar meta descriptions ({sim:.0}% word overlap).",
                                    pages[i].url, pages[j].url
                                ),
                                url: pages[i].url.to_string(),
                                recommendation: "Write unique meta descriptions that accurately reflect each page's content."
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

// ---------------------------------------------------------------------------
// 3. CannibalizationDetector
// ---------------------------------------------------------------------------

/// Detects keyword cannibalization and canonical URL conflicts.
///
/// Identifies when multiple pages target the same primary keyword or
/// share the same canonical URL, which can confuse search engines.
pub struct CannibalizationDetector;

impl Default for CannibalizationDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl CannibalizationDetector {
    pub fn new() -> Self {
        Self
    }

    /// Extract primary keyword from a title (first significant word(s)).
    fn extract_primary_keyword(title: &str) -> String {
        let stop_words: HashSet<&str> = [
            "a", "an", "the", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with",
            "by", "is", "are", "was", "were", "be", "been", "being", "have", "has", "had", "do",
            "does", "did", "will", "would", "could", "should", "may", "might", "shall", "can",
            "it", "its", "this", "that", "how", "what", "which", "who", "where", "when", "why",
        ]
        .iter()
        .copied()
        .collect();

        let lowered = title.to_lowercase();
        let words: Vec<&str> = lowered
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty() && !stop_words.contains(*w))
            .collect();

        // Take the first 1-3 meaningful words as the primary keyword
        let take = words.len().min(3);
        words[..take].join(" ")
    }
}

impl PostCrawlAnalyzer for CannibalizationDetector {
    fn name(&self) -> &str {
        "cannibalization-detector"
    }

    fn analyze_crawl(&self, data: &CrawlData) -> Vec<Finding> {
        let mut findings = Vec::new();
        let pages = &data.pages;
        if pages.len() < 2 {
            return findings;
        }

        // CANNIB001: Multiple pages targeting same primary keyword
        {
            let mut keyword_pages: HashMap<String, Vec<&str>> = HashMap::new();
            for page in pages {
                if let Some(title) = &page.title {
                    let kw = Self::extract_primary_keyword(title);
                    if !kw.is_empty() {
                        keyword_pages
                            .entry(kw)
                            .or_default()
                            .push(page.url.as_str());
                    }
                }
            }

            for (keyword, urls) in &keyword_pages {
                if urls.len() > 1 {
                    findings.push(Finding {
                        severity: Severity::Warning,
                        category: IssueCategory::Seo,
                        code: "CANNIB001".to_string(),
                        title: "Keyword cannibalization detected".to_string(),
                        description: format!(
                            "Multiple pages target the primary keyword '{}': {}.",
                            keyword,
                            urls.join(", ")
                        ),
                        url: urls[0].to_string(),
                        recommendation: "Consolidate pages targeting the same keyword or differentiate their content focus."
                            .to_string(),
                    });
                }
            }
        }

        // CANNIB002: Multiple pages with same canonical URL
        {
            let mut canonical_pages: HashMap<String, Vec<&str>> = HashMap::new();
            for page in pages {
                if let Some(canonical) = &page.canonical_url {
                    let canonical_str = canonical.as_str().to_string();
                    canonical_pages
                        .entry(canonical_str)
                        .or_default()
                        .push(page.url.as_str());
                }
            }

            for (canonical, urls) in &canonical_pages {
                if urls.len() > 1 {
                    findings.push(Finding {
                        severity: Severity::Error,
                        category: IssueCategory::Seo,
                        code: "CANNIB002".to_string(),
                        title: "Duplicate canonical URLs".to_string(),
                        description: format!(
                            "Multiple pages declare canonical '{}': {}.",
                            canonical,
                            urls.join(", ")
                        ),
                        url: urls[0].to_string(),
                        recommendation: "Ensure each page has a unique self-referencing canonical URL."
                            .to_string(),
                    });
                }
            }
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// 4. OrphanPageDetector
// ---------------------------------------------------------------------------

/// Detects orphan pages with zero incoming internal links.
///
/// An orphan page is one that no other crawled page links to, making it
/// invisible to search engine crawlers that follow links.
pub struct OrphanPageDetector;

impl Default for OrphanPageDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl OrphanPageDetector {
    pub fn new() -> Self {
        Self
    }

    fn extract_domain(url: &str) -> &str {
        url.split("://").nth(1).unwrap_or(url).split('/').next().unwrap_or(url)
    }

    fn same_origin(a: &str, b: &str) -> bool {
        Self::extract_domain(a) == Self::extract_domain(b)
    }
}

impl PostCrawlAnalyzer for OrphanPageDetector {
    fn name(&self) -> &str {
        "orphan-page-detector"
    }

    fn analyze_crawl(&self, data: &CrawlData) -> Vec<Finding> {
        let mut findings = Vec::new();
        if data.pages.len() < 2 {
            return findings;
        }

        // Count incoming internal links for each crawled page
        let mut incoming: HashMap<String, usize> = HashMap::new();
        for (source, targets) in &data.links {
            for target in targets {
                if Self::same_origin(source, target) && source != target {
                    let norm_target = normalize_url(target);
                    *incoming.entry(norm_target).or_insert(0) += 1;
                }
            }
        }

        let normalized_seed = normalize_url(&data.seed_url);
        for page in &data.pages {
            let url_str = normalize_url(page.url.as_str());
            // Seed URL is never orphan; check only crawled pages
            if url_str != normalized_seed && incoming.get(&url_str).copied().unwrap_or(0) == 0 {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "ORPHAN001".to_string(),
                    title: "Orphan page".to_string(),
                    description: format!(
                        "Page '{}' has zero incoming internal links from other crawled pages.",
                        page.url
                    ),
                    url: page.url.to_string(),
                    recommendation: "Add internal links from relevant pages to ensure this page is discoverable by users and search engines."
                        .to_string(),
                });
            }
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// 5. SitemapCoverageAnalyzer
// ---------------------------------------------------------------------------

/// Analyzes sitemap coverage for crawled pages.
///
/// Detects pages that are in the crawl but not mentioned in any sitemap URL.
/// Uses a heuristic: checks if page URL appears in link text/anchors of
/// other pages that link to sitemap-like paths.
pub struct SitemapCoverageAnalyzer;

impl Default for SitemapCoverageAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl SitemapCoverageAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Check if a URL looks like a sitemap.
    fn is_sitemap_url(url: &str) -> bool {
        let lower = url.to_lowercase();
        // Match sitemap.xml, sitemap_index.xml, sitemap.xml.gz, etc.
        // Must end with .xml or .xml.gz and contain "sitemap" in the filename (not just the path)
        let path_part = lower.split("://").nth(1).unwrap_or(&lower);
        let filename = path_part.rsplit('/').next().unwrap_or(path_part);
        filename.starts_with("sitemap") && (filename.ends_with(".xml") || filename.ends_with(".xml.gz"))
    }
}

impl PostCrawlAnalyzer for SitemapCoverageAnalyzer {
    fn name(&self) -> &str {
        "sitemap-coverage"
    }

    fn analyze_crawl(&self, data: &CrawlData) -> Vec<Finding> {
        let mut findings = Vec::new();
        if data.pages.is_empty() {
            return findings;
        }

        // Identify pages that are sitemaps
        let sitemap_urls: Vec<&str> = data
            .pages
            .iter()
            .filter(|p| Self::is_sitemap_url(p.url.as_str()))
            .map(|p| p.url.as_str())
            .collect();

        // Build a set of URLs mentioned in sitemap pages (by checking link targets from sitemap pages)
        let mut sitemap_mentioned: HashSet<String> = HashSet::new();
        for (source, targets) in &data.links {
            if sitemap_urls.contains(&source.as_str()) || Self::is_sitemap_url(source) {
                for target in targets {
                    sitemap_mentioned.insert(normalize_url(target));
                }
            }
        }

        // If no sitemaps were crawled, we can't do coverage analysis
        if sitemap_urls.is_empty() && sitemap_mentioned.is_empty() {
            return findings;
        }

        let normalized_seed = normalize_url(&data.seed_url);
        // COVERAGE001: pages in crawl but not mentioned in any sitemap
        for page in &data.pages {
            let url_str = normalize_url(page.url.as_str());
            if url_str == normalized_seed {
                continue;
            }
            if !sitemap_mentioned.contains(&url_str) {
                // Only report if we have sitemap data to compare against
                if !sitemap_mentioned.is_empty() {
                    findings.push(Finding {
                        severity: Severity::Info,
                        category: IssueCategory::Seo,
                        code: "COVERAGE001".to_string(),
                        title: "Page not in sitemap".to_string(),
                        description: format!(
                            "Page '{}' is not mentioned in any crawled sitemap.",
                            page.url
                        ),
                        url: page.url.to_string(),
                        recommendation: "Include important pages in your sitemap.xml to ensure search engines discover them."
                            .to_string(),
                    });
                }
            }
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// 6. LinkEquityDistributor
// ---------------------------------------------------------------------------

/// Analyzes link equity distribution across the site.
///
/// Detects when the seed page dominates outgoing links or when pages have
/// extremely unbalanced internal/external link ratios.
pub struct LinkEquityDistributor;

impl Default for LinkEquityDistributor {
    fn default() -> Self {
        Self::new()
    }
}

impl LinkEquityDistributor {
    pub fn new() -> Self {
        Self
    }

    fn extract_domain(url: &str) -> &str {
        url.split("://").nth(1).unwrap_or(url).split('/').next().unwrap_or(url)
    }

    fn same_origin(a: &str, b: &str) -> bool {
        Self::extract_domain(a) == Self::extract_domain(b)
    }
}

impl PostCrawlAnalyzer for LinkEquityDistributor {
    fn name(&self) -> &str {
        "link-equity-distributor"
    }

    fn analyze_crawl(&self, data: &CrawlData) -> Vec<Finding> {
        let mut findings = Vec::new();
        if data.pages.is_empty() {
            return findings;
        }

        let normalized_seed = normalize_url(&data.seed_url);
        let mut total_outgoing: usize = 0;
        let mut seed_outgoing: usize = 0;

        // Per-page internal/external counts
        let mut page_internal: HashMap<String, usize> = HashMap::new();
        let mut page_external: HashMap<String, usize> = HashMap::new();

        for (source, targets) in &data.links {
            let norm_source = normalize_url(source);
            for target in targets {
                total_outgoing += 1;
                if Self::same_origin(source, target) {
                    *page_internal.entry(norm_source.clone()).or_insert(0) += 1;
                } else {
                    *page_external.entry(norm_source.clone()).or_insert(0) += 1;
                }
            }
            if norm_source == normalized_seed {
                seed_outgoing += targets.len();
            }
        }

        // LINK-EQ001: seed page has >20% of all outgoing links
        if total_outgoing > 0 {
            let pct = (seed_outgoing as f64) / (total_outgoing as f64) * 100.0;
            if pct > 20.0 {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Links,
                    code: "LINK-EQ001".to_string(),
                    title: "Seed page dominates link equity".to_string(),
                    description: format!(
                        "The seed page domain accounts for {seed_outgoing} outgoing links ({pct:.1}% of total {total_outgoing})."
                    ),
                    url: data.seed_url.clone(),
                    recommendation: "Distribute outgoing links across more pages to balance link equity distribution."
                        .to_string(),
                });
            }
        }

        // LINK-EQ002: no page has balanced internal/external ratio (>90% external)
        let all_pages_have_extreme = !data.pages.is_empty()
            && data.pages.iter().all(|page| {
                let url_str = normalize_url(page.url.as_str());
                let internal = page_internal.get(&url_str).copied().unwrap_or(0);
                let external = page_external.get(&url_str).copied().unwrap_or(0);
                let total = internal + external;
                if total == 0 {
                    return false; // pages with no links are not considered extreme
                }
                let ext_ratio = external as f64 / total as f64;
                ext_ratio > 0.9
            });

        if all_pages_have_extreme && !data.pages.is_empty() {
            // Only flag if there are pages with links
            let pages_with_links = data.pages.iter().any(|page| {
                let url_str = normalize_url(page.url.as_str());
                let total = page_internal.get(&url_str).copied().unwrap_or(0)
                    + page_external.get(&url_str).copied().unwrap_or(0);
                total > 0
            });
            if pages_with_links {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Links,
                    code: "LINK-EQ002".to_string(),
                    title: "Unbalanced link ratio".to_string(),
                    description: "All pages with links have >90% external links, leaving no internal link equity.".to_string(),
                    url: data.seed_url.clone(),
                    recommendation: "Add more internal links between pages to build topical relevance and distribute link equity within the site."
                        .to_string(),
                });
            }
        }

        findings
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{IssueCategory, Severity};
    use chrono::Utc;
    use url::Url;

    struct DummyPostCrawlAnalyzer;

    impl PostCrawlAnalyzer for DummyPostCrawlAnalyzer {
        fn name(&self) -> &str {
            "dummy-post-crawl"
        }

        fn analyze_crawl(&self, data: &CrawlData) -> Vec<Finding> {
            let mut findings = Vec::new();
            if data.pages.is_empty() {
                findings.push(Finding {
                    severity: Severity::Warning,
                    category: IssueCategory::Custom("post-crawl".to_string()),
                    code: "PCRAWL001".to_string(),
                    title: "No pages crawled".to_string(),
                    description: "The crawl produced zero pages.".to_string(),
                    url: data.seed_url.clone(),
                    recommendation: "Check the seed URL.".to_string(),
                });
            }
            findings
        }
    }

    fn test_page(url: &str) -> PageData {
        PageData {
            id: format!("p-{}", url),
            url: Url::parse(url).unwrap(),
            final_url: Url::parse(url).unwrap(),
            status_code: 200,
            title: Some("Page".to_string()),
            description: None,
            canonical_url: None,
            word_count: Some(100),
            load_time_ms: Some(200),
            body_size: Some(1024),
            fetched_at: Utc::now(),
            links: vec![],
            tenant_id: None,
            etag: None,
            last_modified: None,
            cwv_lcp: None,
            cwv_cls: None,
            cwv_inp: None,
        }
    }

    fn page_with_title(url: &str, title: &str) -> PageData {
        let mut p = test_page(url);
        p.title = Some(title.to_string());
        p
    }

    fn page_no_title(url: &str) -> PageData {
        let mut p = test_page(url);
        p.title = None;
        p
    }

    fn page_with_desc(url: &str, desc: &str) -> PageData {
        let mut p = test_page(url);
        p.description = Some(desc.to_string());
        p
    }

    fn page_with_canonical(url: &str, canonical: &str) -> PageData {
        let mut p = test_page(url);
        p.canonical_url = Some(Url::parse(canonical).unwrap());
        p
    }

    #[test]
    fn test_crawl_data_construction() {
        let pages = vec![test_page("https://example.com")];
        let links = vec![(
            "https://example.com".to_string(),
            vec!["https://example.com/about".to_string()],
        )];
        let issues = vec![Issue {
            id: "i1".to_string(),
            page_id: "p1".to_string(),
            category: IssueCategory::Seo,
            severity: Severity::Error,
            code: "SEO001".to_string(),
            title: "Missing title".to_string(),
            description: "Page has no title".to_string(),
            element: None,
            recommendation: "Add a title tag".to_string(),
            tenant_id: None,
        }];

        let data = CrawlData {
            pages,
            links,
            issues,
            seed_url: "https://example.com".to_string(),
        };

        assert_eq!(data.pages.len(), 1);
        assert_eq!(data.links.len(), 1);
        assert_eq!(data.issues.len(), 1);
        assert_eq!(data.seed_url, "https://example.com");
    }

    #[test]
    fn test_post_crawl_analyzer_registry_empty() {
        let registry = PostCrawlAnalyzerRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_post_crawl_analyzer_registry_register_and_run() {
        let mut registry = PostCrawlAnalyzerRegistry::new();
        registry.register(Box::new(DummyPostCrawlAnalyzer));
        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());

        let data = CrawlData {
            pages: vec![],
            links: vec![],
            issues: vec![],
            seed_url: "https://example.com".to_string(),
        };

        let findings = registry.analyze_crawl(&data);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "PCRAWL001");
    }

    #[test]
    fn test_post_crawl_analyzer_no_findings_when_pages_exist() {
        let mut registry = PostCrawlAnalyzerRegistry::new();
        registry.register(Box::new(DummyPostCrawlAnalyzer));

        let data = CrawlData {
            pages: vec![test_page("https://example.com")],
            links: vec![],
            issues: vec![],
            seed_url: "https://example.com".to_string(),
        };

        let findings = registry.analyze_crawl(&data);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_post_crawl_analyzer_sorts_findings_canonically() {
        struct TwoFindingsAnalyzer;
        impl PostCrawlAnalyzer for TwoFindingsAnalyzer {
            fn name(&self) -> &str {
                "two"
            }
            fn analyze_crawl(&self, _data: &CrawlData) -> Vec<Finding> {
                vec![
                    Finding {
                        severity: Severity::Info,
                        category: IssueCategory::Custom("test".to_string()),
                        code: "ZZZ001".to_string(),
                        title: "Z".to_string(),
                        description: "Z".to_string(),
                        url: "https://example.com/b".to_string(),
                        recommendation: "Fix".to_string(),
                    },
                    Finding {
                        severity: Severity::Info,
                        category: IssueCategory::Custom("test".to_string()),
                        code: "AAA002".to_string(),
                        title: "A".to_string(),
                        description: "A".to_string(),
                        url: "https://example.com/a".to_string(),
                        recommendation: "Fix".to_string(),
                    },
                ]
            }
        }

        let mut registry = PostCrawlAnalyzerRegistry::new();
        registry.register(Box::new(TwoFindingsAnalyzer));

        let data = CrawlData {
            pages: vec![],
            links: vec![],
            issues: vec![],
            seed_url: "https://example.com".to_string(),
        };

        let findings = registry.analyze_crawl(&data);
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].code, "AAA002");
        assert_eq!(findings[1].code, "ZZZ001");
    }

    // ===== InternalLinkGraphAnalyzer tests =====

    #[test]
    fn test_graph_no_findings_empty_crawl() {
        let analyzer = InternalLinkGraphAnalyzer::new();
        let data = CrawlData {
            pages: vec![],
            links: vec![],
            issues: vec![],
            seed_url: "https://example.com".to_string(),
        };
        assert!(analyzer.analyze_crawl(&data).is_empty());
    }

    #[test]
    fn test_graph_orphan_page() {
        let analyzer = InternalLinkGraphAnalyzer::new();
        let data = CrawlData {
            pages: vec![
                test_page("https://example.com"),
                test_page("https://example.com/orphan"),
            ],
            links: vec![],
            issues: vec![],
            seed_url: "https://example.com".to_string(),
        };
        let findings = analyzer.analyze_crawl(&data);
        assert!(findings.iter().any(|f| f.code == "GRAPH001"));
    }

    #[test]
    fn test_graph_no_orphan_when_linked() {
        let analyzer = InternalLinkGraphAnalyzer::new();
        let data = CrawlData {
            pages: vec![
                test_page("https://example.com"),
                test_page("https://example.com/about"),
            ],
            links: vec![(
                "https://example.com".to_string(),
                vec!["https://example.com/about".to_string()],
            )],
            issues: vec![],
            seed_url: "https://example.com".to_string(),
        };
        let findings = analyzer.analyze_crawl(&data);
        assert!(!findings.iter().any(|f| f.code == "GRAPH001"));
    }

    #[test]
    fn test_graph_link_spam() {
        let analyzer = InternalLinkGraphAnalyzer::new();
        let targets: Vec<String> = (0..55)
            .map(|i| format!("https://example.com/page{i}"))
            .collect();
        let pages: Vec<PageData> = targets
            .iter()
            .map(|u| test_page(u))
            .collect();
        let mut all_links = pages.iter().map(|p| test_page(p.url.as_str())).collect::<Vec<_>>();
        all_links.insert(0, test_page("https://example.com"));
        let data = CrawlData {
            pages: all_links,
            links: vec![("https://example.com".to_string(), targets)],
            issues: vec![],
            seed_url: "https://example.com".to_string(),
        };
        let findings = analyzer.analyze_crawl(&data);
        assert!(findings.iter().any(|f| f.code == "GRAPH002"));
    }

    #[test]
    fn test_graph_no_spam_under_threshold() {
        let analyzer = InternalLinkGraphAnalyzer::new();
        let targets: Vec<String> = (0..50)
            .map(|i| format!("https://example.com/page{i}"))
            .collect();
        let mut pages: Vec<PageData> = targets.iter().map(|u| test_page(u)).collect();
        pages.insert(0, test_page("https://example.com"));
        let data = CrawlData {
            pages,
            links: vec![("https://example.com".to_string(), targets)],
            issues: vec![],
            seed_url: "https://example.com".to_string(),
        };
        let findings = analyzer.analyze_crawl(&data);
        assert!(!findings.iter().any(|f| f.code == "GRAPH002"));
    }

    #[test]
    fn test_graph_deep_navigation() {
        let analyzer = InternalLinkGraphAnalyzer::new();
        let pages: Vec<PageData> = (0..7)
            .map(|i| test_page(&format!("https://example.com/d{i}")))
            .collect();
        let links: Vec<(String, Vec<String>)> = (0..6)
            .map(|i| {
                (
                    format!("https://example.com/d{i}"),
                    vec![format!("https://example.com/d{}", i + 1)],
                )
            })
            .collect();
        let data = CrawlData {
            pages,
            links,
            issues: vec![],
            seed_url: "https://example.com/d0".to_string(),
        };
        let findings = analyzer.analyze_crawl(&data);
        assert!(findings.iter().any(|f| f.code == "GRAPH003"));
    }

    #[test]
    fn test_graph_no_deep_navigation() {
        let analyzer = InternalLinkGraphAnalyzer::new();
        let pages: Vec<PageData> = (0..3)
            .map(|i| test_page(&format!("https://example.com/l{i}")))
            .collect();
        let links: Vec<(String, Vec<String>)> = (0..2)
            .map(|i| {
                (
                    format!("https://example.com/l{i}"),
                    vec![format!("https://example.com/l{}", i + 1)],
                )
            })
            .collect();
        let data = CrawlData {
            pages,
            links,
            issues: vec![],
            seed_url: "https://example.com/l0".to_string(),
        };
        let findings = analyzer.analyze_crawl(&data);
        assert!(!findings.iter().any(|f| f.code == "GRAPH003"));
    }

    #[test]
    fn test_graph_authority_concentration() {
        let analyzer = InternalLinkGraphAnalyzer::new();
        let mut pages = vec![test_page("https://example.com")];
        let mut links = Vec::new();
        // 20 pages each link to the same target
        for i in 0..20 {
            let src = format!("https://example.com/s{i}");
            pages.push(test_page(&src));
            links.push((src, vec!["https://example.com/hub".to_string()]));
        }
        pages.push(test_page("https://example.com/hub"));
        // hub has no outgoing internal links (just receives)
        let data = CrawlData {
            pages,
            links,
            issues: vec![],
            seed_url: "https://example.com".to_string(),
        };
        let findings = analyzer.analyze_crawl(&data);
        assert!(findings.iter().any(|f| f.code == "GRAPH004"));
    }

    #[test]
    fn test_graph_no_authority_concentration() {
        let analyzer = InternalLinkGraphAnalyzer::new();
        let pages: Vec<PageData> = (0..10)
            .map(|i| test_page(&format!("https://example.com/p{i}")))
            .collect();
        // Each page links to a different target
        let links: Vec<(String, Vec<String>)> = (0..10)
            .map(|i| {
                (
                    format!("https://example.com/p{i}"),
                    vec![format!("https://example.com/t{i}")],
                )
            })
            .collect();
        let data = CrawlData {
            pages,
            links,
            issues: vec![],
            seed_url: "https://example.com".to_string(),
        };
        let findings = analyzer.analyze_crawl(&data);
        assert!(!findings.iter().any(|f| f.code == "GRAPH004"));
    }

    // ===== CrossPageDuplicateContentDetector tests =====

    #[test]
    fn test_dup_cross_no_findings_single_page() {
        let analyzer = CrossPageDuplicateContentDetector::new();
        let data = CrawlData {
            pages: vec![page_with_title("https://example.com", "About Us")],
            links: vec![],
            issues: vec![],
            seed_url: "https://example.com".to_string(),
        };
        assert!(analyzer.analyze_crawl(&data).is_empty());
    }

    #[test]
    fn test_dup_cross_duplicate_titles() {
        let analyzer = CrossPageDuplicateContentDetector::new();
        let data = CrawlData {
            pages: vec![
                page_with_title("https://example.com/a", "Best Running Shoes"),
                page_with_title("https://example.com/b", "Best Running Shoes"),
            ],
            links: vec![],
            issues: vec![],
            seed_url: "https://example.com".to_string(),
        };
        let findings = analyzer.analyze_crawl(&data);
        assert!(findings.iter().any(|f| f.code == "DUP-CROSS001"));
    }

    #[test]
    fn test_dup_cross_no_duplicate_titles() {
        let analyzer = CrossPageDuplicateContentDetector::new();
        let data = CrawlData {
            pages: vec![
                page_with_title("https://example.com/a", "Running Shoes Guide"),
                page_with_title("https://example.com/b", "Trail Running Shoes Review"),
            ],
            links: vec![],
            issues: vec![],
            seed_url: "https://example.com".to_string(),
        };
        let findings = analyzer.analyze_crawl(&data);
        assert!(!findings.iter().any(|f| f.code == "DUP-CROSS001"));
    }

    #[test]
    fn test_dup_cross_duplicate_descriptions() {
        let analyzer = CrossPageDuplicateContentDetector::new();
        let data = CrawlData {
            pages: vec![
                page_with_desc("https://example.com/a", "We sell the best running shoes for all occasions"),
                page_with_desc("https://example.com/b", "We sell the best running shoes for all occasions"),
            ],
            links: vec![],
            issues: vec![],
            seed_url: "https://example.com".to_string(),
        };
        let findings = analyzer.analyze_crawl(&data);
        assert!(findings.iter().any(|f| f.code == "DUP-CROSS002"));
    }

    #[test]
    fn test_dup_cross_no_duplicate_descriptions() {
        let analyzer = CrossPageDuplicateContentDetector::new();
        let data = CrawlData {
            pages: vec![
                page_with_desc("https://example.com/a", "Premium running shoes for marathons"),
                page_with_desc("https://example.com/b", "Budget trail shoes for hiking"),
            ],
            links: vec![],
            issues: vec![],
            seed_url: "https://example.com".to_string(),
        };
        let findings = analyzer.analyze_crawl(&data);
        assert!(!findings.iter().any(|f| f.code == "DUP-CROSS002"));
    }

    #[test]
    fn test_dup_cross_empty_titles_ignored() {
        let analyzer = CrossPageDuplicateContentDetector::new();
        let data = CrawlData {
            pages: vec![
                page_with_title("https://example.com/a", ""),
                page_with_title("https://example.com/b", ""),
            ],
            links: vec![],
            issues: vec![],
            seed_url: "https://example.com".to_string(),
        };
        assert!(analyzer.analyze_crawl(&data).is_empty());
    }

    #[test]
    fn test_dup_cross_missing_titles_ignored() {
        let analyzer = CrossPageDuplicateContentDetector::new();
        let data = CrawlData {
            pages: vec![page_no_title("https://example.com/a"), page_no_title("https://example.com/b")],
            links: vec![],
            issues: vec![],
            seed_url: "https://example.com".to_string(),
        };
        assert!(analyzer.analyze_crawl(&data).is_empty());
    }

    #[test]
    fn test_dup_cross_empty_descriptions_ignored() {
        let analyzer = CrossPageDuplicateContentDetector::new();
        let data = CrawlData {
            pages: vec![
                {
                    let mut p = page_no_title("https://example.com/a");
                    p.description = Some(String::new());
                    p
                },
                {
                    let mut p = page_no_title("https://example.com/b");
                    p.description = Some(String::new());
                    p
                },
            ],
            links: vec![],
            issues: vec![],
            seed_url: "https://example.com".to_string(),
        };
        assert!(analyzer.analyze_crawl(&data).is_empty());
    }

    #[test]
    fn test_dup_cross_both_title_and_description_duplicate() {
        let analyzer = CrossPageDuplicateContentDetector::new();
        let mut p1 = page_with_title("https://example.com/a", "Best Shoes Online");
        p1.description = Some("We sell the best shoes online for everyone".to_string());
        let mut p2 = page_with_title("https://example.com/b", "Best Shoes Online");
        p2.description = Some("We sell the best shoes online for everyone".to_string());
        let data = CrawlData {
            pages: vec![p1, p2],
            links: vec![],
            issues: vec![],
            seed_url: "https://example.com".to_string(),
        };
        let findings = analyzer.analyze_crawl(&data);
        assert!(findings.iter().any(|f| f.code == "DUP-CROSS001"));
        assert!(findings.iter().any(|f| f.code == "DUP-CROSS002"));
    }

    // ===== CannibalizationDetector tests =====

    #[test]
    fn test_cannib_no_findings_single_page() {
        let analyzer = CannibalizationDetector::new();
        let data = CrawlData {
            pages: vec![page_with_title("https://example.com", "Running Shoes")],
            links: vec![],
            issues: vec![],
            seed_url: "https://example.com".to_string(),
        };
        assert!(analyzer.analyze_crawl(&data).is_empty());
    }

    #[test]
    fn test_cannib_keyword_cannibalization() {
        let analyzer = CannibalizationDetector::new();
        let data = CrawlData {
            pages: vec![
                page_with_title("https://example.com/a", "Running Shoes for Beginners Guide"),
                page_with_title("https://example.com/b", "Running Shoes for Beginners Review"),
                page_with_title("https://example.com/c", "Running Shoes for Beginners Tips"),
            ],
            links: vec![],
            issues: vec![],
            seed_url: "https://example.com".to_string(),
        };
        let findings = analyzer.analyze_crawl(&data);
        assert!(findings.iter().any(|f| f.code == "CANNIB001"));
    }

    #[test]
    fn test_cannib_no_cannibalization_different_keywords() {
        let analyzer = CannibalizationDetector::new();
        let data = CrawlData {
            pages: vec![
                page_with_title("https://example.com/a", "Running Shoes Guide"),
                page_with_title("https://example.com/b", "Trail Hiking Boots"),
                page_with_title("https://example.com/c", "Swimming Goggles Review"),
            ],
            links: vec![],
            issues: vec![],
            seed_url: "https://example.com".to_string(),
        };
        let findings = analyzer.analyze_crawl(&data);
        assert!(!findings.iter().any(|f| f.code == "CANNIB001"));
    }

    #[test]
    fn test_cannib_duplicate_canonical_urls() {
        let analyzer = CannibalizationDetector::new();
        let data = CrawlData {
            pages: vec![
                page_with_canonical("https://example.com/a", "https://example.com/canonical"),
                page_with_canonical("https://example.com/b", "https://example.com/canonical"),
            ],
            links: vec![],
            issues: vec![],
            seed_url: "https://example.com".to_string(),
        };
        let findings = analyzer.analyze_crawl(&data);
        assert!(findings.iter().any(|f| f.code == "CANNIB002"));
    }

    #[test]
    fn test_cannib_unique_canonical_urls() {
        let analyzer = CannibalizationDetector::new();
        let data = CrawlData {
            pages: vec![
                page_with_canonical("https://example.com/a", "https://example.com/a"),
                page_with_canonical("https://example.com/b", "https://example.com/b"),
            ],
            links: vec![],
            issues: vec![],
            seed_url: "https://example.com".to_string(),
        };
        let findings = analyzer.analyze_crawl(&data);
        assert!(!findings.iter().any(|f| f.code == "CANNIB002"));
    }

    #[test]
    fn test_cannib_no_canonical_urls() {
        let analyzer = CannibalizationDetector::new();
        let data = CrawlData {
            pages: vec![
                page_no_title("https://example.com/a"),
                page_no_title("https://example.com/b"),
            ],
            links: vec![],
            issues: vec![],
            seed_url: "https://example.com".to_string(),
        };
        let findings = analyzer.analyze_crawl(&data);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_cannib_mixed_canonical_and_keyword() {
        let analyzer = CannibalizationDetector::new();
        let data = CrawlData {
            pages: vec![
                page_with_canonical("https://example.com/a", "https://example.com/shared"),
                page_with_canonical("https://example.com/b", "https://example.com/shared"),
                page_with_title("https://example.com/c", "Shoes Online Store"),
                page_with_title("https://example.com/d", "Shoes Online Store"),
            ],
            links: vec![],
            issues: vec![],
            seed_url: "https://example.com".to_string(),
        };
        let findings = analyzer.analyze_crawl(&data);
        assert!(findings.iter().any(|f| f.code == "CANNIB001"));
        assert!(findings.iter().any(|f| f.code == "CANNIB002"));
    }

    #[test]
    fn test_cannib_stop_words_stripped() {
        let analyzer = CannibalizationDetector::new();
        let data = CrawlData {
            pages: vec![
                page_with_title("https://example.com/a", "The Best Running Shoes"),
                page_with_title("https://example.com/b", "Best Running Shoes Guide"),
            ],
            links: vec![],
            issues: vec![],
            seed_url: "https://example.com".to_string(),
        };
        let findings = analyzer.analyze_crawl(&data);
        assert!(findings.iter().any(|f| f.code == "CANNIB001"));
    }

    #[test]
    fn test_cannib_single_page_no_duplicate_canonical() {
        let analyzer = CannibalizationDetector::new();
        let data = CrawlData {
            pages: vec![page_with_canonical(
                "https://example.com/a",
                "https://example.com/a",
            )],
            links: vec![],
            issues: vec![],
            seed_url: "https://example.com".to_string(),
        };
        assert!(analyzer.analyze_crawl(&data).is_empty());
    }

    // ===== OrphanPageDetector tests =====

    #[test]
    fn test_orphan_no_findings_empty_crawl() {
        let analyzer = OrphanPageDetector::new();
        let data = CrawlData {
            pages: vec![],
            links: vec![],
            issues: vec![],
            seed_url: "https://example.com".to_string(),
        };
        assert!(analyzer.analyze_crawl(&data).is_empty());
    }

    #[test]
    fn test_orphan_single_page_no_findings() {
        let analyzer = OrphanPageDetector::new();
        let data = CrawlData {
            pages: vec![test_page("https://example.com")],
            links: vec![],
            issues: vec![],
            seed_url: "https://example.com".to_string(),
        };
        assert!(analyzer.analyze_crawl(&data).is_empty());
    }

    #[test]
    fn test_orphan_page_detected() {
        let analyzer = OrphanPageDetector::new();
        let data = CrawlData {
            pages: vec![
                test_page("https://example.com"),
                test_page("https://example.com/orphan"),
            ],
            links: vec![],
            issues: vec![],
            seed_url: "https://example.com".to_string(),
        };
        let findings = analyzer.analyze_crawl(&data);
        assert!(findings.iter().any(|f| f.code == "ORPHAN001"));
    }

    #[test]
    fn test_orphan_not_orphan_when_linked() {
        let analyzer = OrphanPageDetector::new();
        let data = CrawlData {
            pages: vec![
                test_page("https://example.com"),
                test_page("https://example.com/about"),
            ],
            links: vec![(
                "https://example.com".to_string(),
                vec!["https://example.com/about".to_string()],
            )],
            issues: vec![],
            seed_url: "https://example.com".to_string(),
        };
        let findings = analyzer.analyze_crawl(&data);
        assert!(!findings.iter().any(|f| f.code == "ORPHAN001"));
    }

    #[test]
    fn test_orphan_seed_never_orphan() {
        let analyzer = OrphanPageDetector::new();
        let data = CrawlData {
            pages: vec![test_page("https://example.com")],
            links: vec![],
            issues: vec![],
            seed_url: "https://example.com".to_string(),
        };
        let findings = analyzer.analyze_crawl(&data);
        assert!(!findings.iter().any(|f| f.code == "ORPHAN001"));
    }

    #[test]
    fn test_orphan_multiple_orphans() {
        let analyzer = OrphanPageDetector::new();
        let data = CrawlData {
            pages: vec![
                test_page("https://example.com"),
                test_page("https://example.com/a"),
                test_page("https://example.com/b"),
                test_page("https://example.com/c"),
            ],
            links: vec![(
                "https://example.com".to_string(),
                vec!["https://example.com/a".to_string()],
            )],
            issues: vec![],
            seed_url: "https://example.com".to_string(),
        };
        let findings = analyzer.analyze_crawl(&data);
        let orphan_findings: Vec<_> = findings.iter().filter(|f| f.code == "ORPHAN001").collect();
        assert_eq!(orphan_findings.len(), 2);
    }

    #[test]
    fn test_orphan_external_links_not_counted() {
        let analyzer = OrphanPageDetector::new();
        let data = CrawlData {
            pages: vec![
                test_page("https://example.com"),
                test_page("https://example.com/orphan"),
            ],
            links: vec![(
                "https://example.com".to_string(),
                vec!["https://other.com/link-to-orphan".to_string()],
            )],
            issues: vec![],
            seed_url: "https://example.com".to_string(),
        };
        let findings = analyzer.analyze_crawl(&data);
        assert!(findings.iter().any(|f| f.code == "ORPHAN001"));
    }

    #[test]
    fn test_orphan_self_link_not_counted() {
        let analyzer = OrphanPageDetector::new();
        let data = CrawlData {
            pages: vec![
                test_page("https://example.com"),
                test_page("https://example.com/page"),
            ],
            links: vec![(
                "https://example.com/page".to_string(),
                vec!["https://example.com/page".to_string()],
            )],
            issues: vec![],
            seed_url: "https://example.com".to_string(),
        };
        let findings = analyzer.analyze_crawl(&data);
        assert!(findings.iter().any(|f| f.code == "ORPHAN001"));
    }

    // ===== SitemapCoverageAnalyzer tests =====

    #[test]
    fn test_sitemap_coverage_empty_crawl() {
        let analyzer = SitemapCoverageAnalyzer::new();
        let data = CrawlData {
            pages: vec![],
            links: vec![],
            issues: vec![],
            seed_url: "https://example.com".to_string(),
        };
        assert!(analyzer.analyze_crawl(&data).is_empty());
    }

    #[test]
    fn test_sitemap_coverage_no_sitemaps_no_findings() {
        let analyzer = SitemapCoverageAnalyzer::new();
        let data = CrawlData {
            pages: vec![
                test_page("https://example.com"),
                test_page("https://example.com/about"),
            ],
            links: vec![],
            issues: vec![],
            seed_url: "https://example.com".to_string(),
        };
        assert!(analyzer.analyze_crawl(&data).is_empty());
    }

    #[test]
    fn test_sitemap_coverage_page_not_in_sitemap() {
        let analyzer = SitemapCoverageAnalyzer::new();
        let mut sitemap_page = test_page("https://example.com/sitemap.xml");
        sitemap_page.links = vec![Url::parse("https://example.com/about").unwrap()];
        let data = CrawlData {
            pages: vec![
                test_page("https://example.com"),
                test_page("https://example.com/about"),
                test_page("https://example.com/secret"),
                sitemap_page,
            ],
            links: vec![(
                "https://example.com/sitemap.xml".to_string(),
                vec!["https://example.com/about".to_string()],
            )],
            issues: vec![],
            seed_url: "https://example.com".to_string(),
        };
        let findings = analyzer.analyze_crawl(&data);
        assert!(findings.iter().any(|f| f.code == "COVERAGE001"));
    }

    #[test]
    fn test_sitemap_coverage_page_in_sitemap() {
        let analyzer = SitemapCoverageAnalyzer::new();
        let data = CrawlData {
            pages: vec![
                test_page("https://example.com"),
                test_page("https://example.com/about"),
                test_page("https://example.com/sitemap.xml"),
            ],
            links: vec![(
                "https://example.com/sitemap.xml".to_string(),
                vec!["https://example.com/about".to_string()],
            )],
            issues: vec![],
            seed_url: "https://example.com".to_string(),
        };
        let findings = analyzer.analyze_crawl(&data);
        // The about page is in the sitemap, so it should not be flagged
        assert!(!findings.iter().any(|f| f.url == "https://example.com/about"));
    }

    #[test]
    fn test_sitemap_coverage_seed_not_flagged() {
        let analyzer = SitemapCoverageAnalyzer::new();
        let data = CrawlData {
            pages: vec![
                test_page("https://example.com"),
                test_page("https://example.com/sitemap.xml"),
            ],
            links: vec![(
                "https://example.com/sitemap.xml".to_string(),
                vec![],
            )],
            issues: vec![],
            seed_url: "https://example.com".to_string(),
        };
        let findings = analyzer.analyze_crawl(&data);
        // Seed URL should not be flagged even if not in sitemap
        assert!(!findings.iter().any(|f| f.url == "https://example.com"));
    }

    #[test]
    fn test_sitemap_coverage_xml_gz_sitemap() {
        let analyzer = SitemapCoverageAnalyzer::new();
        let data = CrawlData {
            pages: vec![
                test_page("https://example.com"),
                test_page("https://example.com/page1"),
                test_page("https://example.com/sitemap.xml.gz"),
            ],
            links: vec![(
                "https://example.com/sitemap.xml.gz".to_string(),
                vec!["https://example.com/page1".to_string()],
            )],
            issues: vec![],
            seed_url: "https://example.com".to_string(),
        };
        let findings = analyzer.analyze_crawl(&data);
        // page1 is in the sitemap, so it should not be flagged
        assert!(!findings.iter().any(|f| f.url == "https://example.com/page1"));
    }

    #[test]
    fn test_sitemap_coverage_partial_coverage() {
        let analyzer = SitemapCoverageAnalyzer::new();
        let data = CrawlData {
            pages: vec![
                test_page("https://example.com"),
                test_page("https://example.com/a"),
                test_page("https://example.com/b"),
                test_page("https://example.com/sitemap.xml"),
            ],
            links: vec![(
                "https://example.com/sitemap.xml".to_string(),
                vec!["https://example.com/a".to_string()],
            )],
            issues: vec![],
            seed_url: "https://example.com".to_string(),
        };
        let findings = analyzer.analyze_crawl(&data);
        // /a is in sitemap, /b is not
        let b_flagged = findings.iter().any(|f| f.url == "https://example.com/b");
        let a_flagged = findings.iter().any(|f| f.url == "https://example.com/a");
        assert!(b_flagged);
        assert!(!a_flagged);
    }

    #[test]
    fn test_sitemap_coverage_is_sitemap_url_heuristic() {
        assert!(SitemapCoverageAnalyzer::is_sitemap_url("https://example.com/sitemap.xml"));
        assert!(SitemapCoverageAnalyzer::is_sitemap_url("https://example.com/sitemap_index.xml"));
        assert!(SitemapCoverageAnalyzer::is_sitemap_url("https://example.com/sitemap.xml.gz"));
        assert!(!SitemapCoverageAnalyzer::is_sitemap_url("https://example.com/about"));
        assert!(!SitemapCoverageAnalyzer::is_sitemap_url("https://example.com/sitemap-page"));
        assert!(!SitemapCoverageAnalyzer::is_sitemap_url("https://example.com/not-sitemap.xml"));
    }

    // ===== LinkEquityDistributor tests =====

    #[test]
    fn test_link_equity_empty_crawl() {
        let analyzer = LinkEquityDistributor::new();
        let data = CrawlData {
            pages: vec![],
            links: vec![],
            issues: vec![],
            seed_url: "https://example.com".to_string(),
        };
        assert!(analyzer.analyze_crawl(&data).is_empty());
    }

    #[test]
    fn test_link_equity_seed_dominates() {
        let analyzer = LinkEquityDistributor::new();
        // Seed has 30 outgoing links, total is 100
        let seed_links: Vec<String> = (0..30)
            .map(|i| format!("https://example.com/ext{i}"))
            .collect();
        let other_links: Vec<(String, Vec<String>)> = (0..70)
            .map(|i| {
                (
                    format!("https://example.com/p{i}"),
                    vec![format!("https://external{i}.com/page")],
                )
            })
            .collect();
        let pages: Vec<PageData> = (0..70)
            .map(|i| test_page(&format!("https://example.com/p{i}")))
            .collect();
        let mut all_pages = pages;
        all_pages.insert(0, test_page("https://example.com"));

        let mut all_links = vec![("https://example.com".to_string(), seed_links)];
        all_links.extend(other_links);

        let data = CrawlData {
            pages: all_pages,
            links: all_links,
            issues: vec![],
            seed_url: "https://example.com".to_string(),
        };
        let findings = analyzer.analyze_crawl(&data);
        assert!(findings.iter().any(|f| f.code == "LINK-EQ001"));
    }

    #[test]
    fn test_link_equity_seed_balanced() {
        let analyzer = LinkEquityDistributor::new();
        // Seed has 5 outgoing, total is 100
        let seed_links: Vec<String> = (0..5)
            .map(|i| format!("https://example.com/t{i}"))
            .collect();
        let mut all_links = vec![("https://example.com".to_string(), seed_links)];
        let mut all_pages = vec![test_page("https://example.com")];
        for i in 0..95 {
            let src = format!("https://example.com/p{i}");
            all_pages.push(test_page(&src));
            all_links.push((src, vec![format!("https://other.com/page{i}")]));
        }
        let data = CrawlData {
            pages: all_pages,
            links: all_links,
            issues: vec![],
            seed_url: "https://example.com".to_string(),
        };
        let findings = analyzer.analyze_crawl(&data);
        assert!(!findings.iter().any(|f| f.code == "LINK-EQ001"));
    }

    #[test]
    fn test_link_equity_all_external() {
        let analyzer = LinkEquityDistributor::new();
        let data = CrawlData {
            pages: vec![
                test_page("https://example.com"),
                test_page("https://example.com/a"),
            ],
            links: vec![
                (
                    "https://example.com".to_string(),
                    vec!["https://external.com/page".to_string()],
                ),
                (
                    "https://example.com/a".to_string(),
                    vec!["https://external2.com/page".to_string()],
                ),
            ],
            issues: vec![],
            seed_url: "https://example.com".to_string(),
        };
        let findings = analyzer.analyze_crawl(&data);
        assert!(findings.iter().any(|f| f.code == "LINK-EQ002"));
    }

    #[test]
    fn test_link_equity_balanced_internal_external() {
        let analyzer = LinkEquityDistributor::new();
        let data = CrawlData {
            pages: vec![
                test_page("https://example.com"),
                test_page("https://example.com/a"),
            ],
            links: vec![
                (
                    "https://example.com".to_string(),
                    vec![
                        "https://example.com/a".to_string(),
                        "https://external.com/page".to_string(),
                    ],
                ),
                (
                    "https://example.com/a".to_string(),
                    vec!["https://example.com".to_string()],
                ),
            ],
            issues: vec![],
            seed_url: "https://example.com".to_string(),
        };
        let findings = analyzer.analyze_crawl(&data);
        assert!(!findings.iter().any(|f| f.code == "LINK-EQ002"));
    }

    #[test]
    fn test_link_equity_pages_with_no_links_not_flagged() {
        let analyzer = LinkEquityDistributor::new();
        let data = CrawlData {
            pages: vec![
                test_page("https://example.com"),
                test_page("https://example.com/a"),
            ],
            links: vec![],
            issues: vec![],
            seed_url: "https://example.com".to_string(),
        };
        let findings = analyzer.analyze_crawl(&data);
        assert!(!findings.iter().any(|f| f.code == "LINK-EQ002"));
    }

    #[test]
    fn test_link_equity_mixed_ratios_not_all_extreme() {
        let analyzer = LinkEquityDistributor::new();
        let data = CrawlData {
            pages: vec![
                test_page("https://example.com"),
                test_page("https://example.com/a"),
                test_page("https://example.com/b"),
            ],
            links: vec![
                (
                    "https://example.com".to_string(),
                    vec!["https://external.com/page".to_string()],
                ),
                (
                    "https://example.com/a".to_string(),
                    vec![
                        "https://example.com".to_string(),
                        "https://external.com/page2".to_string(),
                    ],
                ),
            ],
            issues: vec![],
            seed_url: "https://example.com".to_string(),
        };
        let findings = analyzer.analyze_crawl(&data);
        // /a has 50/50 split, so not all pages are extreme
        assert!(!findings.iter().any(|f| f.code == "LINK-EQ002"));
    }

    #[test]
    fn test_link_equity_single_page_all_external() {
        let analyzer = LinkEquityDistributor::new();
        let data = CrawlData {
            pages: vec![test_page("https://example.com")],
            links: vec![(
                "https://example.com".to_string(),
                vec!["https://external.com/page".to_string()],
            )],
            issues: vec![],
            seed_url: "https://example.com".to_string(),
        };
        let findings = analyzer.analyze_crawl(&data);
        // Single page with only external links -> all pages have >90% external
        assert!(findings.iter().any(|f| f.code == "LINK-EQ002"));
    }

    // ===== build_post_crawl_registry tests =====

    #[test]
    fn test_build_registry_returns_six_analyzers() {
        let registry = build_post_crawl_registry();
        assert_eq!(registry.len(), 6);
    }

    #[test]
    fn test_build_registry_analyzer_names() {
        let registry = build_post_crawl_registry();
        let names: Vec<&str> = registry.iter().map(|a| a.name()).collect();
        assert!(names.contains(&"internal-link-graph"));
        assert!(names.contains(&"cross-page-duplicate-content"));
        assert!(names.contains(&"cannibalization-detector"));
        assert!(names.contains(&"orphan-page-detector"));
        assert!(names.contains(&"sitemap-coverage"));
        assert!(names.contains(&"link-equity-distributor"));
    }

    #[test]
    fn test_build_registry_run_on_empty_data() {
        let registry = build_post_crawl_registry();
        let data = CrawlData {
            pages: vec![],
            links: vec![],
            issues: vec![],
            seed_url: "https://example.com".to_string(),
        };
        let findings = registry.analyze_crawl(&data);
        assert!(findings.is_empty());
    }
}
