use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use parking_lot::RwLock;
use tokio::net::lookup_host;
use url::Url;

/// Cached DNS resolution result.
#[derive(Debug, Clone)]
struct DnsEntry {
    /// Resolved socket addresses.
    addresses: Vec<SocketAddr>,
    /// When this entry was resolved.
    resolved_at: Instant,
    /// Time-to-live for this cache entry.
    ttl: Duration,
}

impl DnsEntry {
    /// Returns `true` if the cache entry has expired.
    fn is_expired(&self) -> bool {
        self.resolved_at.elapsed() > self.ttl
    }
}

/// A concurrent DNS cache with background prefetching.
///
/// Caches DNS resolution results with configurable TTL and provides
/// background prefetching for upcoming URLs. Uses `DashMap` for
/// concurrent access without external locking.
///
/// # Examples
///
/// ```rust
/// use crawlkit_engine::DnsCache;
/// use std::time::Duration;
///
/// let cache = DnsCache::new(Duration::from_secs(300), 10_000);
/// cache.insert("example.com", vec!["93.184.216.34:443".parse().unwrap()]);
/// assert!(cache.get_cached("example.com").is_some());
/// ```
pub struct DnsCache {
    /// Domain -> cached resolution.
    cache: DashMap<String, DnsEntry>,
    /// Default TTL for DNS cache entries.
    default_ttl: Duration,
    /// Maximum number of entries in the cache.
    max_entries: usize,
    /// Prefetch queue: domains queued for background resolution.
    prefetch_queue: Arc<RwLock<Vec<String>>>,
    /// Track memory usage (approximate bytes).
    memory_usage: std::sync::atomic::AtomicUsize,
}

impl DnsCache {
    /// Creates a new DNS cache with the given TTL and capacity.
    pub fn new(default_ttl: Duration, max_entries: usize) -> Self {
        Self {
            cache: DashMap::with_capacity(max_entries.min(1024)),
            default_ttl,
            max_entries,
            prefetch_queue: Arc::new(RwLock::new(Vec::new())),
            memory_usage: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// Creates a DNS cache with sensible defaults (5 min TTL, 10k entries).
    pub fn with_defaults() -> Self {
        Self::new(Duration::from_secs(300), 10_000)
    }

    /// Resolves a domain, using the cache if available and not expired.
    pub async fn resolve(&self, domain: &str) -> Result<Vec<SocketAddr>, DnsError> {
        // Check cache first
        if let Some(entry) = self.cache.get(domain) {
            if !entry.is_expired() {
                return Ok(entry.addresses.clone());
            }
        }

        // Cache miss or expired — resolve via system DNS
        let addr = format!("{domain}:443");
        let addresses: Vec<SocketAddr> = lookup_host(&addr)
            .await
            .map_err(|e| DnsError::ResolutionFailed {
                domain: domain.to_string(),
                source: e,
            })?
            .collect();

        if addresses.is_empty() {
            return Err(DnsError::NoRecords {
                domain: domain.to_string(),
            });
        }

        self.insert(domain, addresses.clone());
        Ok(addresses)
    }

    /// Resolves a domain from a URL.
    pub async fn resolve_url(&self, url: &Url) -> Result<Vec<SocketAddr>, DnsError> {
        let domain = url.host_str().ok_or_else(|| DnsError::NoRecords {
            domain: url.to_string(),
        })?;
        self.resolve(domain).await
    }

    /// Inserts a pre-resolved entry into the cache.
    pub fn insert(&self, domain: &str, addresses: Vec<SocketAddr>) {
        // Evict oldest entries if at capacity
        if self.cache.len() >= self.max_entries {
            self.evict_expired();
            // If still at capacity, remove oldest entry
            if self.cache.len() >= self.max_entries {
                if let Some(oldest) = self.find_oldest_entry() {
                    self.cache.remove(&oldest);
                }
            }
        }

        let entry = DnsEntry {
            addresses,
            resolved_at: Instant::now(),
            ttl: self.default_ttl,
        };

        // Update memory tracking
        let new_size = std::mem::size_of::<DnsEntry>() + domain.len();
        // Compute old size BEFORE inserting — drop the DashMap guard explicitly
        // to avoid deadlock when insert re-acquires the shard lock.
        let old_size = {
            self.cache.get(domain).map(|e| {
                std::mem::size_of::<DnsEntry>()
                    + domain.len()
                    + e.addresses.len() * std::mem::size_of::<SocketAddr>()
            })
        }
        .unwrap_or(0);

        self.memory_usage.fetch_add(
            new_size.saturating_sub(old_size),
            std::sync::atomic::Ordering::Relaxed,
        );

        self.cache.insert(domain.to_string(), entry);
    }

    /// Returns cached addresses for a domain without doing DNS resolution.
    pub fn get_cached(&self, domain: &str) -> Option<Vec<SocketAddr>> {
        self.cache.get(domain).and_then(|entry| {
            if entry.is_expired() {
                None
            } else {
                Some(entry.addresses.clone())
            }
        })
    }

    /// Queues domains for background prefetching.
    pub fn enqueue_prefetch(&self, domains: Vec<String>) {
        let mut queue = self.prefetch_queue.write();
        for domain in domains {
            if !self.cache.contains_key(&domain) {
                queue.push(domain);
            }
        }
    }

    /// Dequeues domains for prefetching (called by the background task).
    pub fn dequeue_prefetch(&self, batch_size: usize) -> Vec<String> {
        let mut queue = self.prefetch_queue.write();
        let drain_count = batch_size.min(queue.len());
        queue.drain(..drain_count).collect()
    }

    /// Returns the number of entries in the cache.
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Returns `true` if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Returns the approximate memory usage in bytes.
    pub fn memory_usage(&self) -> usize {
        self.memory_usage.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Removes expired entries from the cache.
    fn evict_expired(&self) {
        let mut evicted = Vec::new();
        for entry in self.cache.iter() {
            if entry.value().is_expired() {
                evicted.push(entry.key().clone());
            }
        }
        for key in evicted {
            if let Some(entry) = self.cache.remove(&key) {
                let size = std::mem::size_of::<DnsEntry>()
                    + key.len()
                    + entry.1.addresses.len() * std::mem::size_of::<SocketAddr>();
                self.memory_usage
                    .fetch_sub(size, std::sync::atomic::Ordering::Relaxed);
            }
        }
    }

    /// Finds the oldest entry in the cache.
    fn find_oldest_entry(&self) -> Option<String> {
        self.cache
            .iter()
            .min_by_key(|e| e.value().resolved_at)
            .map(|e| e.key().clone())
    }
}

impl Default for DnsCache {
    fn default() -> Self {
        Self::with_defaults()
    }
}

/// Background DNS prefetcher that resolves queued domains.
pub struct DnsPrefetcher {
    cache: Arc<DnsCache>,
    /// Interval between prefetch batches.
    prefetch_interval: Duration,
    /// Number of domains to resolve per batch.
    batch_size: usize,
}

impl DnsPrefetcher {
    /// Creates a new prefetcher.
    pub fn new(cache: Arc<DnsCache>, prefetch_interval: Duration, batch_size: usize) -> Self {
        Self {
            cache,
            prefetch_interval,
            batch_size,
        }
    }

    /// Creates a prefetcher with sensible defaults.
    pub fn with_defaults(cache: Arc<DnsCache>) -> Self {
        Self::new(cache, Duration::from_millis(100), 10)
    }

    /// Spawns the background prefetch task.
    ///
    /// Returns a `JoinHandle` that can be used to abort the task.
    pub fn spawn(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                let domains = self.cache.dequeue_prefetch(self.batch_size);
                if !domains.is_empty() {
                    for domain in &domains {
                        if let Err(e) = self.cache.resolve(domain).await {
                            tracing::debug!(
                                domain = domain,
                                error = %e,
                                "DNS prefetch failed"
                            );
                        }
                    }
                }
                tokio::time::sleep(self.prefetch_interval).await;
            }
        })
    }
}

/// Errors that can occur during DNS resolution.
#[derive(Debug, thiserror::Error)]
pub enum DnsError {
    /// DNS resolution failed.
    #[error("DNS resolution failed for {domain}: {source}")]
    ResolutionFailed {
        /// The domain that failed to resolve.
        domain: String,
        /// The underlying I/O error.
        source: std::io::Error,
    },
    /// No DNS records found.
    #[error("no DNS records found for {domain}")]
    NoRecords {
        /// The domain with no records.
        domain: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dns_cache_insert_and_get() {
        let cache = DnsCache::new(Duration::from_secs(60), 100);
        let addrs = vec!["127.0.0.1:443".parse().unwrap()];
        cache.insert("example.com", addrs.clone());

        let cached = cache.get_cached("example.com");
        assert_eq!(cached, Some(addrs));
    }

    #[test]
    fn test_dns_cache_expiry() {
        let cache = DnsCache::new(Duration::from_millis(1), 100);
        let addrs = vec!["127.0.0.1:443".parse().unwrap()];
        cache.insert("example.com", addrs);

        // Wait for expiry
        std::thread::sleep(Duration::from_millis(5));
        assert!(cache.get_cached("example.com").is_none());
    }

    #[test]
    fn test_dns_cache_capacity_eviction() {
        let cache = DnsCache::new(Duration::from_secs(60), 2);

        cache.insert("a.com", vec!["1.1.1.1:443".parse().unwrap()]);
        cache.insert("b.com", vec!["2.2.2.2:443".parse().unwrap()]);
        assert_eq!(cache.len(), 2);

        // Adding a third should evict one
        cache.insert("c.com", vec!["3.3.3.3:443".parse().unwrap()]);
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn test_prefetch_queue() {
        let cache = DnsCache::new(Duration::from_secs(60), 100);
        cache.enqueue_prefetch(vec!["a.com".to_string(), "b.com".to_string()]);

        let batch = cache.dequeue_prefetch(10);
        assert_eq!(batch.len(), 2);
        assert!(cache.dequeue_prefetch(10).is_empty());
    }

    #[test]
    fn test_memory_tracking() {
        let cache = DnsCache::new(Duration::from_secs(60), 100);
        let initial = cache.memory_usage();
        cache.insert("example.com", vec!["1.1.1.1:443".parse().unwrap()]);
        assert!(cache.memory_usage() > initial);
    }

    #[tokio::test]
    async fn test_dns_cache_resolve() {
        let cache = DnsCache::new(Duration::from_secs(30), 100);
        // Resolve localhost — should work on most systems
        let result = cache.resolve("localhost").await;
        assert!(result.is_ok());
        let addrs = result.unwrap();
        assert!(!addrs.is_empty());
    }
}
