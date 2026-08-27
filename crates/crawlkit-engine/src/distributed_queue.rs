use redis::Commands;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur with the distributed Redis queue.
#[derive(Debug, Error)]
pub enum RedisQueueError {
    /// Failed to connect to Redis.
    #[error("redis connection failed: {0}")]
    ConnectionFailed(String),

    /// Failed to serialize/deserialize queue entry.
    #[error("serialization error: {0}")]
    SerializationError(String),

    /// Redis operation failed.
    #[error("redis operation failed: {0}")]
    OperationFailed(String),
}

/// A URL entry stored in the Redis distributed queue.
///
/// Contains the URL string and crawl depth. This is a simplified representation
/// optimized for network transfer; full queue entries with metadata can be
/// reconstructed when popping from the queue.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DistributedQueueEntry {
    /// The URL to crawl.
    pub url: String,
    /// Crawl depth from the seed URL (0 = seed).
    pub depth: usize,
}

/// Redis-backed distributed URL queue for multi-instance crawling.
///
/// Uses Redis sorted sets to maintain priority ordering across multiple
/// crawler instances. Each queue is namespaced by a crawl ID to prevent
/// collisions between different crawl sessions.
///
/// # Examples
///
/// ```rust,no_run
/// use crawlkit_engine::distributed_queue::DistributedQueue;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let queue = DistributedQueue::new("redis://127.0.0.1/", "crawl-123")?;
/// queue.push("https://example.com", 0, 64)?;
///
/// if let Some(entry) = queue.pop()? {
///     println!("Popped: {}", entry.url);
/// }
/// # Ok(())
/// # }
/// ```
pub struct DistributedQueue {
    client: redis::Client,
    prefix: String,
}

impl DistributedQueue {
    /// Create a new distributed queue connected to Redis.
    ///
    /// # Arguments
    ///
    /// * `redis_url` - Redis connection URL (e.g., `redis://127.0.0.1/`)
    /// * `crawl_id` - Unique identifier for this crawl session
    ///
    /// # Errors
    ///
    /// Returns [`RedisQueueError::ConnectionFailed`] if unable to connect to Redis.
    pub fn new(redis_url: &str, crawl_id: &str) -> Result<Self, RedisQueueError> {
        let client = redis::Client::open(redis_url)
            .map_err(|e| RedisQueueError::ConnectionFailed(e.to_string()))?;
        Ok(Self {
            client,
            prefix: format!("crawlkit:{crawl_id}:queue"),
        })
    }

    /// Push a URL with priority into the queue.
    ///
    /// Uses Redis sorted set with score = priority (lower value = higher priority).
    ///
    /// # Arguments
    ///
    /// * `url` - The URL to enqueue
    /// * `depth` - Crawl depth from seed URL
    /// * `priority` - Priority score (lower = higher priority)
    ///
    /// # Errors
    ///
    /// Returns [`RedisQueueError`] on connection or serialization failure.
    pub fn push(&self, url: &str, depth: usize, priority: i64) -> Result<(), RedisQueueError> {
        let mut conn = self
            .client
            .get_connection()
            .map_err(|e| RedisQueueError::ConnectionFailed(e.to_string()))?;

        let entry = serde_json::to_string(&DistributedQueueEntry {
            url: url.to_string(),
            depth,
        })
        .map_err(|e| RedisQueueError::SerializationError(e.to_string()))?;

        let _: () = conn
            .zadd(&self.prefix, entry, priority)
            .map_err(|e| RedisQueueError::OperationFailed(e.to_string()))?;

        Ok(())
    }

    /// Pop the highest-priority URL from the queue.
    ///
    /// Uses Redis ZPOPMIN to atomically retrieve and remove the entry
    /// with the lowest score. This matches the engine-wide convention
    /// (`queue::Priority`): lower score = higher priority.
    ///
    /// # Errors
    ///
    /// Returns [`RedisQueueError`] on connection or deserialization failure.
    pub fn pop(&self) -> Result<Option<DistributedQueueEntry>, RedisQueueError> {
        let mut conn = self
            .client
            .get_connection()
            .map_err(|e| RedisQueueError::ConnectionFailed(e.to_string()))?;

        // ZPOPMIN returns a flat [member, score, ...] array; decoding as a
        // vec handles the empty-set case cleanly (empty vec -> None).
        let results: Vec<(String, i64)> = conn
            .zpopmin(&self.prefix, 1)
            .map_err(|e| RedisQueueError::OperationFailed(e.to_string()))?;

        match results.into_iter().next() {
            Some((entry_json, _)) => {
                let entry = serde_json::from_str(&entry_json)
                    .map_err(|e| RedisQueueError::SerializationError(e.to_string()))?;
                Ok(Some(entry))
            }
            None => Ok(None),
        }
    }

    /// Check if queue is empty.
    ///
    /// # Errors
    ///
    /// Returns [`RedisQueueError`] on connection failure.
    pub fn is_empty(&self) -> Result<bool, RedisQueueError> {
        let mut conn = self
            .client
            .get_connection()
            .map_err(|e| RedisQueueError::ConnectionFailed(e.to_string()))?;

        let len: usize = conn
            .zcard(&self.prefix)
            .map_err(|e| RedisQueueError::OperationFailed(e.to_string()))?;

        Ok(len == 0)
    }

    /// Get queue length.
    ///
    /// # Errors
    ///
    /// Returns [`RedisQueueError`] on connection failure.
    pub fn len(&self) -> Result<usize, RedisQueueError> {
        let mut conn = self
            .client
            .get_connection()
            .map_err(|e| RedisQueueError::ConnectionFailed(e.to_string()))?;

        let len: usize = conn
            .zcard(&self.prefix)
            .map_err(|e| RedisQueueError::OperationFailed(e.to_string()))?;

        Ok(len)
    }

    /// Clear the queue.
    ///
    /// # Errors
    ///
    /// Returns [`RedisQueueError`] on connection failure.
    pub fn clear(&self) -> Result<(), RedisQueueError> {
        let mut conn = self
            .client
            .get_connection()
            .map_err(|e| RedisQueueError::ConnectionFailed(e.to_string()))?;

        let _: () = conn
            .del(&self.prefix)
            .map_err(|e| RedisQueueError::OperationFailed(e.to_string()))?;

        Ok(())
    }
}

#[cfg(all(feature = "full", feature = "unstable"))]
impl crate::queue_trait::Queue for DistributedQueue {
    fn push(
        &self,
        entry: crate::queue::QueueEntry,
    ) -> Result<bool, crate::queue_trait::QueueError> {
        let url_str = entry.url.to_string();
        self.push(&url_str, entry.depth, entry.priority.value() as i64)
            .map_err(|e| crate::queue_trait::QueueError::Backend(e.to_string()))?;
        Ok(true)
    }

    fn pop(&self) -> Result<Option<crate::queue::QueueEntry>, crate::queue_trait::QueueError> {
        match self.pop() {
            Ok(Some(entry)) => {
                let url = url::Url::parse(&entry.url)
                    .map_err(|e| crate::queue_trait::QueueError::Backend(e.to_string()))?;
                Ok(Some(crate::queue::QueueEntry {
                    url: url.clone(),
                    canonical_url: url,
                    depth: entry.depth,
                    priority: crate::queue::Priority::NORMAL,
                    discovered_at: chrono::Utc::now(),
                    referrer: None,
                }))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(crate::queue_trait::QueueError::Backend(e.to_string())),
        }
    }

    fn len(&self) -> Result<usize, crate::queue_trait::QueueError> {
        self.len()
            .map_err(|e| crate::queue_trait::QueueError::Backend(e.to_string()))
    }

    fn is_empty(&self) -> Result<bool, crate::queue_trait::QueueError> {
        self.is_empty()
            .map_err(|e| crate::queue_trait::QueueError::Backend(e.to_string()))
    }

    fn contains(&self, url: &str) -> Result<bool, crate::queue_trait::QueueError> {
        let mut conn = self
            .client
            .get_connection()
            .map_err(|e| crate::queue_trait::QueueError::Backend(e.to_string()))?;

        let members: Vec<String> = redis::Commands::zrange(&mut conn, &self.prefix, 0, -1)
            .map_err(|e| crate::queue_trait::QueueError::Backend(e.to_string()))?;

        for member in &members {
            if let Ok(entry) = serde_json::from_str::<DistributedQueueEntry>(member) {
                if entry.url == url {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    // Tests requiring a running Redis instance are marked #[ignore]
    // Run with: cargo test --features full -- --ignored

    fn redis_url() -> String {
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1/".to_string())
    }

    #[test]
    #[ignore = "requires running Redis instance"]
    fn test_distributed_queue_push_and_pop() {
        let queue = DistributedQueue::new(&redis_url(), "test-push-pop").unwrap();
        queue.clear().unwrap();

        queue.push("https://example.com/a", 0, 64).unwrap();
        queue.push("https://example.com/b", 0, 32).unwrap();

        assert_eq!(queue.len().unwrap(), 2);

        // Higher priority (lower score) should come first
        let entry = queue.pop().unwrap().unwrap();
        assert_eq!(entry.url, "https://example.com/b");
        assert_eq!(entry.depth, 0);

        let entry = queue.pop().unwrap().unwrap();
        assert_eq!(entry.url, "https://example.com/a");

        assert!(queue.is_empty().unwrap());
    }

    #[test]
    #[ignore = "requires running Redis instance"]
    fn test_distributed_queue_empty_pop() {
        let queue = DistributedQueue::new(&redis_url(), "test-empty-pop").unwrap();
        queue.clear().unwrap();

        let entry = queue.pop().unwrap();
        assert!(entry.is_none());
    }

    #[test]
    #[ignore = "requires running Redis instance"]
    fn test_distributed_queue_is_empty() {
        let queue = DistributedQueue::new(&redis_url(), "test-is-empty").unwrap();
        queue.clear().unwrap();

        assert!(queue.is_empty().unwrap());

        queue.push("https://example.com", 0, 64).unwrap();
        assert!(!queue.is_empty().unwrap());
    }

    #[test]
    #[ignore = "requires running Redis instance"]
    fn test_distributed_queue_clear() {
        let queue = DistributedQueue::new(&redis_url(), "test-clear").unwrap();

        queue.push("https://example.com/1", 0, 64).unwrap();
        queue.push("https://example.com/2", 0, 64).unwrap();
        assert_eq!(queue.len().unwrap(), 2);

        queue.clear().unwrap();
        assert!(queue.is_empty().unwrap());
    }

    #[test]
    #[ignore = "requires running Redis instance"]
    fn test_distributed_queue_different_crawl_ids() {
        let queue1 = DistributedQueue::new(&redis_url(), "test-crawl-1").unwrap();
        let queue2 = DistributedQueue::new(&redis_url(), "test-crawl-2").unwrap();
        queue1.clear().unwrap();
        queue2.clear().unwrap();

        queue1.push("https://a.com", 0, 64).unwrap();
        queue2.push("https://b.com", 0, 64).unwrap();

        assert_eq!(queue1.len().unwrap(), 1);
        assert_eq!(queue2.len().unwrap(), 1);

        let entry1 = queue1.pop().unwrap().unwrap();
        assert_eq!(entry1.url, "https://a.com");

        let entry2 = queue2.pop().unwrap().unwrap();
        assert_eq!(entry2.url, "https://b.com");
    }

    #[test]
    fn test_distributed_queue_entry_serialization() {
        let entry = DistributedQueueEntry {
            url: "https://example.com".to_string(),
            depth: 3,
        };

        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: DistributedQueueEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(entry, deserialized);
    }

    #[test]
    fn test_redis_queue_error_display() {
        let err = RedisQueueError::ConnectionFailed("refused".to_string());
        assert!(err.to_string().contains("refused"));

        let err = RedisQueueError::SerializationError("bad json".to_string());
        assert!(err.to_string().contains("bad json"));

        let err = RedisQueueError::OperationFailed("timeout".to_string());
        assert!(err.to_string().contains("timeout"));
    }
}
