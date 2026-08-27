use crate::storage::{Issue, PageData};
use crate::Finding;

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
}
