// Post-crawl analysis for cross-page SEO checks.

use crate::storage::{IssueCategory, Severity};
use crate::storage_trait::StorageBackend;
use std::collections::{HashMap, HashSet};

pub struct PostCrawlAnalysis {
    pub findings: Vec<PostCrawlFinding>,
    pub stats: PostCrawlStats,
}

pub struct PostCrawlFinding {
    pub page_url: String,
    pub severity: Severity,
    pub category: IssueCategory,
    pub code: String,
    pub title: String,
    pub description: String,
    pub recommendation: String,
}

#[derive(Default)]
pub struct PostCrawlStats {
    pub pages_analyzed: usize,
    pub canonical_mismatches: usize,
    pub sitemap_issues: usize,
}

pub fn run_post_crawl_analysis(
    storage: &dyn StorageBackend,
    crawl_id: &str,
) -> PostCrawlAnalysis {
    let mut findings = Vec::new();
    let mut stats = PostCrawlStats::default();

    let pages = match storage.get_pages(crawl_id, 10000) {
        Ok(p) => p,
        Err(_) => return PostCrawlAnalysis { findings, stats },
    };
    stats.pages_analyzed = pages.len();

    let mut incoming: HashMap<String, HashSet<String>> = HashMap::new();
    if let Ok(links) = storage.get_links_for_crawl(crawl_id) {
        for (src, targets) in &links {
            for tgt in targets {
                incoming.entry(tgt.clone()).or_default().insert(src.clone());
            }
        }
    }

    for page in &pages {
        if let Some(ref canonical) = page.canonical_url {
            let c = canonical.to_string();
            let p = page.url.to_string();
            if c != p {
                if incoming.get(&c).is_none_or(|s| s.is_empty()) {
                    findings.push(PostCrawlFinding {
                        page_url: page.url.to_string(),
                        severity: Severity::Warning,
                        category: IssueCategory::Seo,
                        code: "CANON005".into(),
                        title: "Canonical URL has no incoming links".into(),
                        description: format!(
                            "Canonical \"{}\" is not linked from any crawled page.",
                            canonical
                        ),
                        recommendation: "Add internal links to the canonical URL.".into(),
                    });
                    stats.canonical_mismatches += 1;
                }
                findings.push(PostCrawlFinding {
                    page_url: page.url.to_string(),
                    severity: Severity::Warning,
                    category: IssueCategory::Seo,
                    code: "SITEMAP006".into(),
                    title: "Non-canonical page may be in sitemap".into(),
                    description: format!("Page has canonical \"{}\" but was crawled.", canonical),
                    recommendation: "Remove non-canonical pages from the sitemap.".into(),
                });
                stats.sitemap_issues += 1;
            }
        }
    }

    PostCrawlAnalysis { findings, stats }
}
