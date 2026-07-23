use std::cmp::Ordering;
use std::collections::BinaryHeap;

use chrono::{DateTime, Utc};
use dashmap::DashSet;
use parking_lot::Mutex;
use url::Url;

use crate::CrawlConfig;

/// Priority score for a URL entry (lower = higher priority).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Priority(u8);

impl Priority {
    /// Highest priority (e.g. sitemap, seed URLs).
    pub const HIGHEST: Self = Self(0);
    /// High priority (e.g. linked from important pages).
    pub const HIGH: Self = Self(32);
    /// Normal priority (default).
    pub const NORMAL: Self = Self(64);
    /// Low priority (e.g. deep pages, low-value links).
    pub const LOW: Self = Self(128);
    /// Lowest priority (e.g. archived, noindex pages).
    pub const LOWEST: Self = Self(255);

    /// Creates a priority from a raw u8 value.
    pub fn new(value: u8) -> Self {
        Self(value)
    }

    /// Returns the raw priority value.
    pub fn value(&self) -> u8 {
        self.0
    }
}

impl Default for Priority {
    fn default() -> Self {
        Self::NORMAL
    }
}

impl PartialOrd for Priority {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Priority {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

use serde::{Deserialize, Serialize};

/// A URL entry in the crawl queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueEntry {
    /// The URL to crawl.
    pub url: Url,
    /// The canonical (normalized) URL for deduplication.
    pub canonical_url: Url,
    /// Crawl depth from the seed URL (0 = seed).
    pub depth: usize,
    /// Priority score (lower = higher priority).
    pub priority: Priority,
    /// When this URL was discovered.
    pub discovered_at: DateTime<Utc>,
    /// The URL that discovered this one.
    pub referrer: Option<Url>,
}

// BinaryHeap is a max-heap, so we invert the ordering for min-heap behavior.
impl PartialEq for QueueEntry {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.url == other.url
    }
}

impl Eq for QueueEntry {}

impl PartialOrd for QueueEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for QueueEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // Min-heap: lower priority value = higher priority = should come first
        // BinaryHeap pops the largest, so we reverse the comparison.
        other
            .priority
            .cmp(&self.priority)
            .then_with(|| other.url.as_str().cmp(self.url.as_str()))
    }
}

/// Scope control for URL filtering.
#[derive(Debug, Clone, Default)]
pub struct ScopeConfig {
    /// Allowed domain patterns (e.g. `["example.com", "*.example.org"]`).
    pub allowed_domains: Vec<String>,
    /// Blocked domain patterns.
    pub blocked_domains: Vec<String>,
    /// Allowed URL path prefixes.
    pub allowed_paths: Vec<String>,
    /// Blocked URL path prefixes.
    pub blocked_paths: Vec<String>,
    /// Maximum crawl depth (None = unlimited).
    pub max_depth: Option<usize>,
}

impl From<&CrawlConfig> for ScopeConfig {
    fn from(config: &CrawlConfig) -> Self {
        Self {
            allowed_domains: Vec::new(),
            blocked_domains: Vec::new(),
            allowed_paths: config.allowed_patterns.clone(),
            blocked_paths: config.disallowed_patterns.clone(),
            max_depth: None,
        }
    }
}

/// Priority URL queue with deduplication and scope control.
pub struct UrlQueue {
    heap: Mutex<BinaryHeap<QueueEntry>>,
    seen: DashSet<String>,
    scope: ScopeConfig,
    domain_counts: dashmap::DashMap<String, usize>,
}

impl UrlQueue {
    /// Creates a new empty queue with the given scope configuration.
    pub fn new(scope: ScopeConfig) -> Self {
        Self {
            heap: Mutex::new(BinaryHeap::new()),
            seen: DashSet::new(),
            scope,
            domain_counts: dashmap::DashMap::new(),
        }
    }

    /// Creates a queue from a `CrawlConfig`.
    pub fn from_crawl_config(config: &CrawlConfig) -> Self {
        Self::new(ScopeConfig::from(config))
    }

    /// Pushes a URL into the queue with the given depth and priority.
    ///
    /// Returns `true` if the URL was added, `false` if it was a duplicate
    /// or rejected by scope control.
    pub fn push(&self, url: Url, depth: usize, priority: Priority) -> bool {
        self.push_with_referrer(url, depth, priority, None)
    }

    /// Pushes a URL with an optional referrer.
    ///
    /// Performs deduplication, depth check, and scope filtering.
    pub fn push_with_referrer(
        &self,
        url: Url,
        depth: usize,
        priority: Priority,
        referrer: Option<Url>,
    ) -> bool {
        // Deduplication check
        let url_str = url.to_string();
        if !self.seen.insert(url_str) {
            return false;
        }

        // Depth check
        if let Some(max_depth) = self.scope.max_depth {
            if depth > max_depth {
                return false;
            }
        }

        // Scope check
        if !self.is_in_scope(&url) {
            return false;
        }

        // Track domain
        if let Some(domain) = url.domain() {
            *self.domain_counts.entry(domain.to_string()).or_insert(0) += 1;
        }

        let entry = QueueEntry {
            url: url.clone(),
            canonical_url: url,
            depth,
            priority,
            discovered_at: Utc::now(),
            referrer,
        };

        self.heap.lock().push(entry);
        true
    }

    /// Pops the highest-priority entry from the queue.
    pub fn pop(&self) -> Option<QueueEntry> {
        self.heap.lock().pop()
    }

    /// Returns the number of entries in the queue.
    pub fn len(&self) -> usize {
        self.heap.lock().len()
    }

    /// Returns `true` if the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.heap.lock().is_empty()
    }

    /// Returns the number of unique URLs seen (including popped ones).
    pub fn seen_count(&self) -> usize {
        self.seen.len()
    }

    /// Returns the number of URLs discovered for a given domain.
    pub fn domain_count(&self, domain: &str) -> usize {
        self.domain_counts.get(domain).map_or(0, |c| *c)
    }

    /// Returns all unique domains that have been queued.
    pub fn domains(&self) -> Vec<String> {
        self.domain_counts
            .iter()
            .map(|entry| entry.key().clone())
            .collect()
    }

    /// Peeks at the highest-priority entry without removing it.
    pub fn peek(&self) -> Option<QueueEntry> {
        self.heap.lock().peek().cloned()
    }

    /// Drains all entries from the queue.
    pub fn drain(&self) -> Vec<QueueEntry> {
        let mut entries = Vec::new();
        let mut heap = self.heap.lock();
        while let Some(entry) = heap.pop() {
            entries.push(entry);
        }
        entries
    }

    /// Checks if a URL is within the configured scope.
    fn is_in_scope(&self, url: &Url) -> bool {
        let domain = match url.domain() {
            Some(d) => d,
            None => return false,
        };

        // Check blocked domains
        for pattern in &self.scope.blocked_domains {
            if domain_matches_pattern(domain, pattern) {
                return false;
            }
        }

        // Check allowed domains (if non-empty, only those are allowed)
        if !self.scope.allowed_domains.is_empty() {
            let mut allowed = false;
            for pattern in &self.scope.allowed_domains {
                if domain_matches_pattern(domain, pattern) {
                    allowed = true;
                    break;
                }
            }
            if !allowed {
                return false;
            }
        }

        let path = url.path();

        // Check blocked paths
        for pattern in &self.scope.blocked_paths {
            if path.starts_with(pattern) {
                return false;
            }
        }

        // Check allowed paths (if non-empty, only those are allowed)
        if !self.scope.allowed_paths.is_empty() {
            let mut allowed = false;
            for pattern in &self.scope.allowed_paths {
                if path.starts_with(pattern) {
                    allowed = true;
                    break;
                }
            }
            if !allowed {
                return false;
            }
        }

        true
    }
}

/// Checks if a domain matches a pattern (supports leading `*` wildcard).
fn domain_matches_pattern(domain: &str, pattern: &str) -> bool {
    if let Some(suffix) = pattern.strip_prefix("*.") {
        domain.ends_with(&format!(".{suffix}"))
    } else {
        domain == pattern
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_ordering() {
        assert!(Priority::HIGHEST < Priority::HIGH);
        assert!(Priority::HIGH < Priority::NORMAL);
        assert!(Priority::NORMAL < Priority::LOW);
        assert!(Priority::LOW < Priority::LOWEST);
    }

    #[test]
    fn test_queue_push_and_pop() {
        let queue = UrlQueue::new(ScopeConfig::default());
        let url1 = Url::parse("https://example.com/a").unwrap();
        let url2 = Url::parse("https://example.com/b").unwrap();

        assert!(queue.push(url1.clone(), 0, Priority::LOW));
        assert!(queue.push(url2.clone(), 0, Priority::HIGH));

        assert_eq!(queue.len(), 2);

        // HIGH priority should come first
        let entry = queue.pop().unwrap();
        assert_eq!(entry.url, url2);
        assert_eq!(entry.priority, Priority::HIGH);

        let entry = queue.pop().unwrap();
        assert_eq!(entry.url, url1);
        assert_eq!(entry.priority, Priority::LOW);
    }

    #[test]
    fn test_queue_deduplication() {
        let queue = UrlQueue::new(ScopeConfig::default());
        let url = Url::parse("https://example.com/page").unwrap();

        assert!(queue.push(url.clone(), 0, Priority::NORMAL));
        assert!(!queue.push(url, 1, Priority::HIGH)); // duplicate

        assert_eq!(queue.len(), 1);
        assert_eq!(queue.seen_count(), 1);
    }

    #[test]
    fn test_queue_depth_limit() {
        let scope = ScopeConfig {
            max_depth: Some(2),
            ..Default::default()
        };
        let queue = UrlQueue::new(scope);

        assert!(queue.push(
            Url::parse("https://example.com/p0").unwrap(),
            0,
            Priority::NORMAL,
        ));
        assert!(queue.push(
            Url::parse("https://example.com/p1").unwrap(),
            1,
            Priority::NORMAL,
        ));
        assert!(queue.push(
            Url::parse("https://example.com/p2").unwrap(),
            2,
            Priority::NORMAL,
        ));
        assert!(!queue.push(
            Url::parse("https://example.com/p3").unwrap(),
            3,
            Priority::NORMAL,
        )); // exceeds max depth

        assert_eq!(queue.len(), 3);
    }

    #[test]
    fn test_queue_domain_scope() {
        let scope = ScopeConfig {
            allowed_domains: vec!["example.com".to_string()],
            ..Default::default()
        };
        let queue = UrlQueue::new(scope);

        let url1 = Url::parse("https://example.com/page").unwrap();
        let url2 = Url::parse("https://other.com/page").unwrap();

        assert!(queue.push(url1, 0, Priority::NORMAL));
        assert!(!queue.push(url2, 0, Priority::NORMAL)); // not in allowed domains

        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn test_queue_blocked_domain() {
        let scope = ScopeConfig {
            blocked_domains: vec!["spam.com".to_string()],
            ..Default::default()
        };
        let queue = UrlQueue::new(scope);

        let url1 = Url::parse("https://example.com/page").unwrap();
        let url2 = Url::parse("https://spam.com/page").unwrap();

        assert!(queue.push(url1, 0, Priority::NORMAL));
        assert!(!queue.push(url2, 0, Priority::NORMAL)); // blocked

        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn test_queue_wildcard_domain_pattern() {
        let scope = ScopeConfig {
            allowed_domains: vec!["*.example.com".to_string()],
            ..Default::default()
        };
        let queue = UrlQueue::new(scope);

        assert!(queue.push(
            Url::parse("https://www.example.com/page").unwrap(),
            0,
            Priority::NORMAL,
        ));
        assert!(queue.push(
            Url::parse("https://sub.example.com/page").unwrap(),
            0,
            Priority::NORMAL,
        ));
        assert!(!queue.push(
            Url::parse("https://example.com/page").unwrap(),
            0,
            Priority::NORMAL,
        )); // doesn't match *.example.com

        assert_eq!(queue.len(), 2);
    }

    #[test]
    fn test_queue_domain_tracking() {
        let queue = UrlQueue::new(ScopeConfig::default());

        queue.push(Url::parse("https://a.com/1").unwrap(), 0, Priority::NORMAL);
        queue.push(Url::parse("https://a.com/2").unwrap(), 0, Priority::NORMAL);
        queue.push(Url::parse("https://b.com/1").unwrap(), 0, Priority::NORMAL);

        assert_eq!(queue.domain_count("a.com"), 2);
        assert_eq!(queue.domain_count("b.com"), 1);
        assert_eq!(queue.domain_count("c.com"), 0);

        let mut domains = queue.domains();
        domains.sort();
        assert_eq!(domains, vec!["a.com", "b.com"]);
    }

    #[test]
    fn test_queue_is_empty() {
        let queue = UrlQueue::new(ScopeConfig::default());
        assert!(queue.is_empty());

        queue.push(
            Url::parse("https://example.com").unwrap(),
            0,
            Priority::NORMAL,
        );
        assert!(!queue.is_empty());
    }

    #[test]
    fn test_queue_peek() {
        let queue = UrlQueue::new(ScopeConfig::default());
        assert!(queue.peek().is_none());

        let url = Url::parse("https://example.com").unwrap();
        queue.push(url, 0, Priority::NORMAL);

        let peeked = queue.peek().unwrap();
        assert_eq!(peeked.url.as_str(), "https://example.com/");
        assert_eq!(queue.len(), 1); // peek doesn't remove
    }

    #[test]
    fn test_queue_drain() {
        let queue = UrlQueue::new(ScopeConfig::default());
        queue.push(Url::parse("https://a.com").unwrap(), 0, Priority::HIGH);
        queue.push(Url::parse("https://b.com").unwrap(), 0, Priority::LOW);

        let entries = queue.drain();
        assert_eq!(entries.len(), 2);
        assert!(queue.is_empty());
    }

    #[test]
    fn test_domain_matches_pattern() {
        assert!(domain_matches_pattern("example.com", "example.com"));
        assert!(domain_matches_pattern("www.example.com", "*.example.com"));
        assert!(domain_matches_pattern("sub.example.com", "*.example.com"));
        assert!(!domain_matches_pattern("example.com", "*.example.com"));
        assert!(!domain_matches_pattern("notexample.com", "example.com"));
        assert!(!domain_matches_pattern("evil-example.com", "*.example.com"));
    }

    #[test]
    fn test_queue_push_with_referrer() {
        let queue = UrlQueue::new(ScopeConfig::default());
        let url = Url::parse("https://example.com/page").unwrap();
        let referrer = Url::parse("https://example.com/other").unwrap();

        queue.push_with_referrer(url, 0, Priority::NORMAL, Some(referrer.clone()));

        let entry = queue.pop().unwrap();
        assert_eq!(entry.referrer.as_ref(), Some(&referrer));
    }

    #[test]
    fn test_scope_config_from_crawl_config() {
        let crawl_config = CrawlConfig {
            allowed_patterns: vec!["/blog".to_string()],
            disallowed_patterns: vec!["/admin".to_string()],
            ..Default::default()
        };

        let scope = ScopeConfig::from(&crawl_config);
        assert_eq!(scope.allowed_paths, vec!["/blog"]);
        assert_eq!(scope.blocked_paths, vec!["/admin"]);
    }

    #[test]
    fn test_queue_path_scope() {
        let scope = ScopeConfig {
            blocked_paths: vec!["/admin".to_string()],
            ..Default::default()
        };
        let queue = UrlQueue::new(scope);

        assert!(queue.push(
            Url::parse("https://example.com/page").unwrap(),
            0,
            Priority::NORMAL,
        ));
        assert!(!queue.push(
            Url::parse("https://example.com/admin/secret").unwrap(),
            0,
            Priority::NORMAL,
        ));

        assert_eq!(queue.len(), 1);
    }
}
