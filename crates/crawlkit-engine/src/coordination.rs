use url::Url;

/// Strategy for partitioning URLs across distributed crawl instances.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PartitionStrategy {
    /// Hash-based partitioning: `hash(domain) % instance_count == instance_id`.
    Hash,
    /// Range-based partitioning: alphabetical domain ranges assigned to instances.
    Range,
}

/// Coordinates URL partitioning for distributed crawling.
///
/// Each crawler instance is assigned a unique `instance_id` (0-based) within a
/// fixed `instance_count`. When a URL is popped from the shared queue, the
/// coordinator determines whether this instance should process it based on the
/// domain hash and the partition strategy.
#[derive(Debug, Clone)]
pub struct CrawlCoordinator {
    instance_id: u32,
    instance_count: u32,
    strategy: PartitionStrategy,
}

impl CrawlCoordinator {
    /// Create a new coordinator for the given instance within a cluster.
    ///
    /// # Panics
    ///
    /// Panics if `instance_count == 0` or `instance_id >= instance_count`.
    pub fn new(instance_id: u32, instance_count: u32, strategy: PartitionStrategy) -> Self {
        assert!(instance_count > 0, "instance_count must be > 0");
        assert!(
            instance_id < instance_count,
            "instance_id ({instance_id}) must be < instance_count ({instance_count})"
        );
        Self {
            instance_id,
            instance_count,
            strategy,
        }
    }

    /// Returns `true` if this instance should process the given URL.
    pub fn should_process(&self, url: &str) -> bool {
        match self.strategy {
            PartitionStrategy::Hash => {
                let hash = Self::domain_hash(url);
                hash % self.instance_count == self.instance_id
            }
            PartitionStrategy::Range => {
                let hash = Self::domain_hash(url);
                let range_size = u32::MAX / self.instance_count;
                let start = self.instance_id * range_size;
                let end = start + range_size;
                // Last instance absorbs the remainder
                if self.instance_id == self.instance_count - 1 {
                    hash >= start
                } else {
                    hash >= start && hash < end
                }
            }
        }
    }

    /// FNV-1a hash of the domain portion of a URL.
    pub fn domain_hash(url: &str) -> u32 {
        let domain = Self::extract_domain(url);
        let mut hash: u32 = 2166136261;
        for byte in domain.bytes() {
            hash ^= byte as u32;
            hash = hash.wrapping_mul(16777619);
        }
        hash
    }

    /// Extract the host (domain) from a URL string.
    pub fn extract_domain(url: &str) -> String {
        Url::parse(url)
            .ok()
            .and_then(|u| u.host_str().map(|s| s.to_string()))
            .unwrap_or_default()
    }

    /// Returns this instance's ID.
    pub fn instance_id(&self) -> u32 {
        self.instance_id
    }

    /// Returns the total instance count.
    pub fn instance_count(&self) -> u32 {
        self.instance_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_hash_partition_consistency() {
        // Find which instance owns this URL, then verify it's always the same
        let url = "https://example.com/page";
        let mut owner = None;
        for id in 0..3 {
            let coord = CrawlCoordinator::new(id, 3, PartitionStrategy::Hash);
            if coord.should_process(url) {
                owner = Some(id);
            }
        }
        let owner = owner.expect("URL should be assigned to some instance");
        // Confirm consistency
        let coord = CrawlCoordinator::new(owner, 3, PartitionStrategy::Hash);
        for _ in 0..100 {
            assert!(coord.should_process(url));
        }
    }

    #[test]
    fn test_hash_partition_covers_all_domains() {
        // Every domain maps to exactly one instance
        let urls = vec![
            "https://example.com/",
            "https://google.com/search",
            "https://github.com/rust-lang",
            "https://stackoverflow.com/questions",
            "https://news.ycombinator.com/",
            "https://reddit.com/r/rust",
            "https://amazon.com/dp/B001",
            "https://wikipedia.org/wiki/Rust",
        ];

        for url in &urls {
            let mut assigned = vec![false; 3];
            for id in 0..3 {
                let coord = CrawlCoordinator::new(id, 3, PartitionStrategy::Hash);
                if coord.should_process(url) {
                    assigned[id as usize] = true;
                }
            }
            assert!(
                assigned.iter().filter(|&&x| x).count() == 1,
                "URL {url} should be assigned to exactly one instance, got {assigned:?}"
            );
        }
    }

    #[test]
    fn test_hash_partition_no_overlap_between_instances() {
        let urls: Vec<String> = (0..200)
            .map(|i| format!("https://domain{i}.example.com/"))
            .collect();

        let mut domain_map: HashMap<String, u32> = HashMap::new();
        for url in &urls {
            for id in 0..4 {
                let coord = CrawlCoordinator::new(id, 4, PartitionStrategy::Hash);
                if coord.should_process(url) {
                    let prev = domain_map.insert(url.clone(), id);
                    assert!(prev.is_none(), "URL {url} was already assigned to instance {prev:?}, now assigned to {id}");
                }
            }
        }
        assert_eq!(domain_map.len(), urls.len());
    }

    #[test]
    fn test_range_partition_no_overlap() {
        let urls: Vec<String> = (0..100)
            .map(|i| format!("https://domain{i:03}.example.com/"))
            .collect();

        let mut domain_map: HashMap<String, u32> = HashMap::new();
        for url in &urls {
            for id in 0..3 {
                let coord = CrawlCoordinator::new(id, 3, PartitionStrategy::Range);
                if coord.should_process(url) {
                    let prev = domain_map.insert(url.clone(), id);
                    assert!(prev.is_none(), "URL {url} overlap");
                }
            }
        }
        assert_eq!(domain_map.len(), urls.len());
    }

    #[test]
    fn test_range_partition_last_instance_gets_remainder() {
        // With range partitioning, the last instance should cover the upper range
        let coord_last = CrawlCoordinator::new(2, 3, PartitionStrategy::Range);
        // A very high hash domain should be captured by the last instance
        // "zzzzzzzzzz.com" has a high FNV hash
        let hash = CrawlCoordinator::domain_hash("https://zzzzzzzzzz.com/");
        let range_size = u32::MAX / 3;
        let start_last = 2 * range_size;
        assert!(
            hash >= start_last,
            "Hash {hash} should be >= {start_last} for last instance"
        );
        assert!(coord_last.should_process("https://zzzzzzzzzz.com/"));
    }

    #[test]
    fn test_extract_domain() {
        assert_eq!(
            CrawlCoordinator::extract_domain("https://example.com/path"),
            "example.com"
        );
        assert_eq!(
            CrawlCoordinator::extract_domain("http://localhost:8080/"),
            "localhost"
        );
        assert_eq!(
            CrawlCoordinator::extract_domain("https://sub.domain.co.uk/page"),
            "sub.domain.co.uk"
        );
        assert_eq!(CrawlCoordinator::extract_domain("not-a-url"), "");
    }

    #[test]
    fn test_domain_hash_deterministic() {
        let h1 = CrawlCoordinator::domain_hash("https://example.com/");
        let h2 = CrawlCoordinator::domain_hash("https://example.com/");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_domain_hash_different_domains() {
        let h1 = CrawlCoordinator::domain_hash("https://example.com/");
        let h2 = CrawlCoordinator::domain_hash("https://different.com/");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_single_instance_processes_all() {
        let coord = CrawlCoordinator::new(0, 1, PartitionStrategy::Hash);
        assert!(coord.should_process("https://example.com/"));
        assert!(coord.should_process("https://anything.com/"));
    }

    #[test]
    #[should_panic(expected = "instance_count must be > 0")]
    fn test_zero_instances_panics() {
        CrawlCoordinator::new(0, 0, PartitionStrategy::Hash);
    }

    #[test]
    #[should_panic(expected = "instance_id")]
    fn test_instance_id_out_of_range_panics() {
        CrawlCoordinator::new(5, 3, PartitionStrategy::Hash);
    }

    #[test]
    fn test_partition_strategy_debug() {
        assert_eq!(format!("{:?}", PartitionStrategy::Hash), "Hash");
        assert_eq!(format!("{:?}", PartitionStrategy::Range), "Range");
    }

    #[test]
    fn test_partition_strategy_equality() {
        assert_eq!(PartitionStrategy::Hash, PartitionStrategy::Hash);
        assert_ne!(PartitionStrategy::Hash, PartitionStrategy::Range);
    }
}
