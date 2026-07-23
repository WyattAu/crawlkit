use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use url::Url;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A single backlink pointing to a page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Backlink {
    /// The source page URL (the page containing the link).
    pub source_url: String,
    /// The target page URL (the page being linked to).
    pub target_url: String,
    /// Anchor text of the link.
    pub anchor_text: String,
    /// Whether the link is followed (not nofollow).
    pub is_followed: bool,
    /// Whether the link is internal (same domain as target).
    pub is_internal: bool,
}

/// PageRank-like score for a URL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageScore {
    /// The URL.
    pub url: String,
    /// PageRank score (0.0 – 1.0).
    pub pagerank: f64,
    /// Number of inbound links (backlinks).
    pub inbound_links: usize,
    /// Number of outbound links.
    pub outbound_links: usize,
    /// Number of unique referring domains.
    pub referring_domains: usize,
}

/// Result of a backlink analysis for a single page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacklinkReport {
    /// The analyzed page URL.
    pub url: String,
    /// All backlinks to this page.
    pub backlinks: Vec<Backlink>,
    /// Total number of backlinks.
    pub total_backlinks: usize,
    /// Number of followed (non-nofollow) backlinks.
    pub followed_backlinks: usize,
    /// Number of unique referring domains.
    pub referring_domains: usize,
    /// Number of internal backlinks.
    pub internal_backlinks: usize,
    /// Number of external backlinks.
    pub external_backlinks: usize,
    /// PageRank-like score.
    pub pagerank_score: f64,
}

/// Summary of the entire site's backlink profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacklinkSummary {
    /// PageRank scores for all pages, sorted by score descending.
    pub pages: Vec<PageScore>,
    /// Total number of internal links.
    pub total_internal_links: usize,
    /// Total number of external links.
    pub total_external_links: usize,
    /// Total number of unique referring domains.
    pub total_referring_domains: usize,
    /// Pages with zero inbound links (orphans).
    pub orphan_pages: Vec<String>,
}

// ---------------------------------------------------------------------------
// BacklinkAnalyzer
// ---------------------------------------------------------------------------

/// Analyzes internal link graphs and computes PageRank-like scores.
pub struct BacklinkAnalyzer {
    /// Adjacency list: source -> set of targets.
    outgoing: HashMap<String, HashSet<String>>,
    /// Reverse adjacency: target -> set of sources.
    incoming: HashMap<String, HashSet<String>>,
    /// All known URLs.
    all_urls: HashSet<String>,
    /// External link data.
    external_backlinks: Vec<Backlink>,
}

impl BacklinkAnalyzer {
    /// Creates a new empty `BacklinkAnalyzer`.
    pub fn new() -> Self {
        Self {
            outgoing: HashMap::new(),
            incoming: HashMap::new(),
            all_urls: HashSet::new(),
            external_backlinks: Vec::new(),
        }
    }

    /// Adds an internal link from `source` to `target`.
    ///
    /// Both URLs should be normalized (e.g. trailing slash stripped).
    pub fn add_link(&mut self, source: &str, target: &str) {
        self.all_urls.insert(source.to_string());
        self.all_urls.insert(target.to_string());

        self.outgoing
            .entry(source.to_string())
            .or_default()
            .insert(target.to_string());

        self.incoming
            .entry(target.to_string())
            .or_default()
            .insert(source.to_string());
    }

    /// Adds a backlink record (used for external backlink tracking).
    pub fn add_backlink(&mut self, backlink: Backlink) {
        if backlink.is_internal {
            self.add_link(&backlink.source_url, &backlink.target_url);
        } else {
            self.all_urls.insert(backlink.target_url.clone());
            self.external_backlinks.push(backlink);
        }
    }

    /// Bulk-load internal links from crawl data.
    ///
    /// `pages` should be a slice of `(page_url, [link_urls])` tuples.
    pub fn load_from_crawl_data(&mut self, pages: &[(String, Vec<String>)]) {
        for (page_url, links) in pages {
            for link_url in links {
                self.add_link(page_url, link_url);
            }
        }
    }

    /// Computes PageRank scores for all known URLs.
    ///
    /// Uses the iterative PageRank algorithm with damping factor `d`.
    /// Typically `d = 0.85`.
    pub fn compute_pagerank(&self, damping: f64, iterations: usize) -> HashMap<String, f64> {
        let n = self.all_urls.len();
        if n == 0 {
            return HashMap::new();
        }

        let initial_score = 1.0 / n as f64;
        let mut scores: HashMap<String, f64> = self
            .all_urls
            .iter()
            .map(|url| (url.clone(), initial_score))
            .collect();

        let dangling_contrib = damping / n as f64;

        for _ in 0..iterations {
            let mut new_scores: HashMap<String, f64> = HashMap::with_capacity(n);

            // Collect dangling contribution
            let dangling_sum: f64 = self
                .all_urls
                .iter()
                .filter(|url| !self.outgoing.contains_key(*url))
                .map(|url| scores.get(url).copied().unwrap_or(initial_score))
                .sum::<f64>()
                * dangling_contrib;

            for url in &self.all_urls {
                let base = (1.0 - damping) / n as f64 + dangling_sum;

                let link_contribution: f64 = self
                    .incoming
                    .get(url)
                    .map(|sources| {
                        sources
                            .iter()
                            .filter_map(|src| {
                                let out_degree = self
                                    .outgoing
                                    .get(src)
                                    .map(|t| t.len() as f64)
                                    .unwrap_or(1.0);
                                scores.get(src).map(|s| s / out_degree)
                            })
                            .sum()
                    })
                    .unwrap_or(0.0)
                    * damping;

                new_scores.insert(url.clone(), base + link_contribution);
            }

            scores = new_scores;
        }

        scores
    }

    /// Generates a backlink report for a specific URL.
    pub fn report_for_url(
        &self,
        url: &str,
        pagerank_scores: &HashMap<String, f64>,
    ) -> BacklinkReport {
        let internal_backlinks: Vec<Backlink> = self
            .incoming
            .get(url)
            .map(|sources| {
                sources
                    .iter()
                    .map(|src| Backlink {
                        source_url: src.clone(),
                        target_url: url.to_string(),
                        anchor_text: String::new(),
                        is_followed: true,
                        is_internal: true,
                    })
                    .collect()
            })
            .unwrap_or_default();

        let external_backlinks: Vec<Backlink> = self
            .external_backlinks
            .iter()
            .filter(|bl| bl.target_url == url)
            .cloned()
            .collect();

        let mut all_backlinks = internal_backlinks;
        all_backlinks.extend(external_backlinks);

        let internal_count = all_backlinks.iter().filter(|bl| bl.is_internal).count();
        let external_count = all_backlinks.iter().filter(|bl| !bl.is_internal).count();
        let followed_count = all_backlinks.iter().filter(|bl| bl.is_followed).count();

        let referring_domains: HashSet<String> = all_backlinks
            .iter()
            .filter_map(|bl| Url::parse(&bl.source_url).ok())
            .filter_map(|u| u.domain().map(String::from))
            .collect();

        BacklinkReport {
            url: url.to_string(),
            total_backlinks: all_backlinks.len(),
            followed_backlinks: followed_count,
            referring_domains: referring_domains.len(),
            internal_backlinks: internal_count,
            external_backlinks: external_count,
            pagerank_score: pagerank_scores.get(url).copied().unwrap_or(0.0),
            backlinks: all_backlinks,
        }
    }

    /// Generates a full backlink summary for the entire site.
    pub fn summarize(&self) -> BacklinkSummary {
        let scores = self.compute_pagerank(0.85, 20);

        let mut pages: Vec<PageScore> = self
            .all_urls
            .iter()
            .map(|url| {
                let inbound = self.incoming.get(url).map_or(0, |s| s.len());
                let outbound = self.outgoing.get(url).map_or(0, |s| s.len());

                let referring_domains: HashSet<String> = self
                    .incoming
                    .get(url)
                    .map(|sources| {
                        sources
                            .iter()
                            .filter_map(|src| Url::parse(src).ok())
                            .filter_map(|u| u.domain().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();

                let external_inbound = self
                    .external_backlinks
                    .iter()
                    .filter(|bl| bl.target_url == *url)
                    .count();

                PageScore {
                    url: url.clone(),
                    pagerank: scores.get(url).copied().unwrap_or(0.0),
                    inbound_links: inbound + external_inbound,
                    outbound_links: outbound,
                    referring_domains: referring_domains.len(),
                }
            })
            .collect();

        pages.sort_by(|a, b| {
            b.pagerank
                .partial_cmp(&a.pagerank)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let orphan_pages: Vec<String> = self
            .all_urls
            .iter()
            .filter(|url| {
                !self.incoming.contains_key(*url)
                    && self
                        .external_backlinks
                        .iter()
                        .all(|bl| bl.target_url != **url)
            })
            .cloned()
            .collect();

        let total_internal: usize = self.outgoing.values().map(|v| v.len()).sum();
        let total_external = self.external_backlinks.len();

        let all_referring_domains: HashSet<String> = self
            .external_backlinks
            .iter()
            .filter_map(|bl| Url::parse(&bl.source_url).ok())
            .filter_map(|u| u.domain().map(String::from))
            .collect();

        BacklinkSummary {
            pages,
            total_internal_links: total_internal,
            total_external_links: total_external,
            total_referring_domains: all_referring_domains.len(),
            orphan_pages,
        }
    }

    /// Returns the total number of known URLs.
    pub fn url_count(&self) -> usize {
        self.all_urls.len()
    }

    /// Returns the total number of internal links.
    pub fn link_count(&self) -> usize {
        self.outgoing.values().map(|v| v.len()).sum()
    }

    /// Returns the set of all known URLs.
    pub fn known_urls(&self) -> &HashSet<String> {
        &self.all_urls
    }
}

impl Default for BacklinkAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_analyzer() {
        let analyzer = BacklinkAnalyzer::new();
        assert_eq!(analyzer.url_count(), 0);
        assert_eq!(analyzer.link_count(), 0);
    }

    #[test]
    fn test_add_link() {
        let mut analyzer = BacklinkAnalyzer::new();
        analyzer.add_link("https://example.com/", "https://example.com/about");
        analyzer.add_link("https://example.com/", "https://example.com/contact");

        assert_eq!(analyzer.url_count(), 3);
        assert_eq!(analyzer.link_count(), 2);
    }

    #[test]
    fn test_load_from_crawl_data() {
        let mut analyzer = BacklinkAnalyzer::new();
        let pages = vec![
            (
                "https://example.com/".to_string(),
                vec![
                    "https://example.com/about".to_string(),
                    "https://example.com/blog".to_string(),
                ],
            ),
            (
                "https://example.com/about".to_string(),
                vec!["https://example.com/".to_string()],
            ),
        ];

        analyzer.load_from_crawl_data(&pages);
        assert_eq!(analyzer.url_count(), 3);
        assert_eq!(analyzer.link_count(), 3);
    }

    #[test]
    fn test_pagerank_simple_linear() {
        // A -> B -> C
        let mut analyzer = BacklinkAnalyzer::new();
        analyzer.add_link("https://a.com", "https://b.com");
        analyzer.add_link("https://b.com", "https://c.com");

        let scores = analyzer.compute_pagerank(0.85, 50);

        // C should have highest score (receives link from B, which receives from A)
        // B receives from A, C receives from B
        let score_a = scores.get("https://a.com").unwrap();
        let score_b = scores.get("https://b.com").unwrap();
        let score_c = scores.get("https://c.com").unwrap();

        // Scores should sum to ~1.0
        let total = score_a + score_b + score_c;
        assert!(
            (total - 1.0).abs() < 0.01,
            "scores should sum to ~1.0, got {total}"
        );

        // C should have the highest score in this chain
        assert!(
            score_c > score_a,
            "C ({score_c}) should rank higher than A ({score_a})"
        );
    }

    #[test]
    fn test_pagerank_cycle() {
        // A <-> B (bidirectional)
        let mut analyzer = BacklinkAnalyzer::new();
        analyzer.add_link("https://a.com", "https://b.com");
        analyzer.add_link("https://b.com", "https://a.com");

        let scores = analyzer.compute_pagerank(0.85, 50);

        let score_a = scores.get("https://a.com").unwrap();
        let score_b = scores.get("https://b.com").unwrap();

        // Should be roughly equal
        assert!(
            (score_a - score_b).abs() < 0.05,
            "A ({score_a}) and B ({score_b}) should have similar scores"
        );
    }

    #[test]
    fn test_pagerank_hub_page() {
        // Hub page links to many pages, all pages link back to hub
        let mut analyzer = BacklinkAnalyzer::new();
        let pages: Vec<String> = (0..10)
            .map(|i| format!("https://example.com/page{i}"))
            .collect();

        for page in &pages {
            analyzer.add_link("https://example.com/hub", page);
            analyzer.add_link(page, "https://example.com/hub");
        }

        let scores = analyzer.compute_pagerank(0.85, 50);

        let hub_score = scores.get("https://example.com/hub").unwrap();
        let page0_score = scores.get("https://example.com/page0").unwrap();

        // Hub should have a much higher score
        assert!(
            hub_score > page0_score,
            "Hub ({hub_score}) should rank higher than page0 ({page0_score})"
        );
    }

    #[test]
    fn test_report_for_url() {
        let mut analyzer = BacklinkAnalyzer::new();
        analyzer.add_link("https://example.com/", "https://example.com/about");
        analyzer.add_link("https://example.com/blog", "https://example.com/about");
        analyzer.add_link("https://example.com/contact", "https://example.com/about");

        let scores = analyzer.compute_pagerank(0.85, 20);
        let report = analyzer.report_for_url("https://example.com/about", &scores);

        assert_eq!(report.url, "https://example.com/about");
        assert_eq!(report.total_backlinks, 3);
        assert_eq!(report.internal_backlinks, 3);
        assert_eq!(report.referring_domains, 1); // all from example.com
        assert!(report.pagerank_score > 0.0);
    }

    #[test]
    fn test_report_external_backlinks() {
        let mut analyzer = BacklinkAnalyzer::new();
        analyzer.add_backlink(Backlink {
            source_url: "https://external.com/mention".to_string(),
            target_url: "https://example.com/about".to_string(),
            anchor_text: "Great site".to_string(),
            is_followed: true,
            is_internal: false,
        });
        analyzer.add_link("https://example.com/", "https://example.com/about");

        let scores = analyzer.compute_pagerank(0.85, 20);
        let report = analyzer.report_for_url("https://example.com/about", &scores);

        assert_eq!(report.total_backlinks, 2);
        assert_eq!(report.external_backlinks, 1);
        assert_eq!(report.referring_domains, 2); // example.com + external.com
    }

    #[test]
    fn test_summarize() {
        let mut analyzer = BacklinkAnalyzer::new();
        analyzer.add_link("https://example.com/", "https://example.com/about");
        analyzer.add_link("https://example.com/", "https://example.com/blog");
        analyzer.add_link("https://example.com/about", "https://example.com/");

        let summary = analyzer.summarize();

        assert_eq!(summary.pages.len(), 3);
        assert_eq!(summary.total_internal_links, 3);
        assert_eq!(summary.total_external_links, 0);

        // / is linked from /about, /about is linked from /
        // /blog is only linked from / → orphan? No, it has an inbound link
        // All pages have at least one inbound link
        assert!(summary.orphan_pages.is_empty());
    }

    #[test]
    fn test_orphan_detection() {
        let mut analyzer = BacklinkAnalyzer::new();
        analyzer.add_link("https://example.com/", "https://example.com/about");
        // /orphan has no inbound links
        analyzer
            .all_urls
            .insert("https://example.com/orphan".to_string());

        let summary = analyzer.summarize();
        assert!(summary
            .orphan_pages
            .contains(&"https://example.com/orphan".to_string()));
    }

    #[test]
    fn test_dangling_nodes() {
        // Page with outgoing links but no incoming → PageRank should still work
        let mut analyzer = BacklinkAnalyzer::new();
        analyzer.add_link("https://a.com", "https://b.com");
        // b.com has no outgoing links (dangling node)

        let scores = analyzer.compute_pagerank(0.85, 50);
        assert!(scores.contains_key("https://a.com"));
        assert!(scores.contains_key("https://b.com"));

        let total: f64 = scores.values().sum();
        assert!((total - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_pagerank_single_node() {
        let mut analyzer = BacklinkAnalyzer::new();
        analyzer.all_urls.insert("https://solo.com".to_string());

        let scores = analyzer.compute_pagerank(0.85, 20);
        let score = scores.get("https://solo.com").unwrap();
        assert!(
            (*score - 1.0).abs() < 0.01,
            "single node should have score ~1.0"
        );
    }

    #[test]
    fn test_known_urls() {
        let mut analyzer = BacklinkAnalyzer::new();
        analyzer.add_link("https://a.com", "https://b.com");

        let urls = analyzer.known_urls();
        assert!(urls.contains("https://a.com"));
        assert!(urls.contains("https://b.com"));
    }

    #[test]
    fn test_backlink_serialization() {
        let bl = Backlink {
            source_url: "https://src.com".to_string(),
            target_url: "https://tgt.com".to_string(),
            anchor_text: "click here".to_string(),
            is_followed: true,
            is_internal: false,
        };

        let json = serde_json::to_string(&bl).unwrap();
        let deser: Backlink = serde_json::from_str(&json).unwrap();
        assert_eq!(bl.source_url, deser.source_url);
        assert_eq!(bl.target_url, deser.target_url);
    }
}
