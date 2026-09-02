use std::sync::Arc;

use thiserror::Error;

use crate::queue::QueueEntry;

/// Errors that can occur with queue operations.
#[derive(Debug, Error)]
pub enum QueueError {
    /// A backend-specific error (e.g., Redis connection failure).
    #[error("queue error: {0}")]
    Backend(String),
}

/// A trait abstracting URL queue backends.
///
/// Both the in-memory [`UrlQueue`](crate::queue::UrlQueue) and the
/// Redis-backed `DistributedQueue` implement this trait, allowing callers
/// to swap backends without changing queue-consuming code.
pub trait Queue: Send + Sync {
    /// Push an entry into the queue.
    ///
    /// Returns `Ok(true)` if the entry was accepted, `Ok(false)` if
    /// rejected (e.g., duplicate URL, out of scope), or `Err` on
    /// backend failure.
    fn push(&self, entry: QueueEntry) -> Result<bool, QueueError>;

    /// Pop the highest-priority entry from the queue.
    ///
    /// Returns `Ok(None)` when the queue is empty.
    fn pop(&self) -> Result<Option<QueueEntry>, QueueError>;

    /// Returns the number of entries currently in the queue.
    fn len(&self) -> Result<usize, QueueError>;

    /// Returns `Ok(true)` if the queue contains no entries.
    fn is_empty(&self) -> Result<bool, QueueError>;

    /// Returns `Ok(true)` if the queue contains an entry for the given URL.
    fn contains(&self, url: &str) -> Result<bool, QueueError>;
}

/// Async wrapper around a sync [`Queue`] implementation.
///
/// Spawns blocking queue operations onto the Tokio blocking pool so
/// the async runtime is never stalled by lock contention or I/O.
pub struct AsyncQueue {
    inner: Arc<dyn Queue>,
}

impl AsyncQueue {
    /// Wrap a sync queue for async use.
    pub fn new(queue: Arc<dyn Queue>) -> Self {
        Self { inner: queue }
    }

    /// Push an entry asynchronously.
    pub async fn push_async(&self, entry: QueueEntry) -> Result<bool, QueueError> {
        let q = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || q.push(entry))
            .await
            .map_err(|e| QueueError::Backend(e.to_string()))?
    }

    /// Pop the highest-priority entry asynchronously.
    pub async fn pop_async(&self) -> Result<Option<QueueEntry>, QueueError> {
        let q = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || q.pop())
            .await
            .map_err(|e| QueueError::Backend(e.to_string()))?
    }

    /// Return the queue length asynchronously.
    pub async fn len_async(&self) -> Result<usize, QueueError> {
        let q = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || q.len())
            .await
            .map_err(|e| QueueError::Backend(e.to_string()))?
    }

    /// Return whether the queue is empty asynchronously.
    pub async fn is_empty_async(&self) -> Result<bool, QueueError> {
        let q = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || q.is_empty())
            .await
            .map_err(|e| QueueError::Backend(e.to_string()))?
    }

    /// Check if a URL is in the queue asynchronously.
    pub async fn contains_async(&self, url: &str) -> Result<bool, QueueError> {
        let url = url.to_string();
        let q = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || q.contains(&url))
            .await
            .map_err(|e| QueueError::Backend(e.to_string()))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::queue::{Priority, ScopeConfig, UrlQueue};
    use chrono::Utc;
    use url::Url;

    fn make_entry(url: &str) -> QueueEntry {
        let url = Url::parse(url).unwrap();
        QueueEntry {
            url: url.clone(),
            canonical_url: url,
            depth: 0,
            priority: Priority::NORMAL,
            discovered_at: Utc::now(),
            referrer: None,
        }
    }

    #[test]
    fn test_trait_push_and_pop() {
        let queue = UrlQueue::new(ScopeConfig::default());
        let entry = make_entry("https://example.com");

        assert!(Queue::push(&queue, entry.clone()).unwrap());
        assert_eq!(Queue::len(&queue).unwrap(), 1);
        assert!(!Queue::is_empty(&queue).unwrap());

        let popped = Queue::pop(&queue).unwrap().unwrap();
        assert_eq!(popped.url, entry.url);
        assert!(Queue::is_empty(&queue).unwrap());
    }

    #[test]
    fn test_trait_contains() {
        let queue = UrlQueue::new(ScopeConfig::default());
        let entry = make_entry("https://example.com");

        assert!(!Queue::contains(&queue, "https://example.com/").unwrap());
        Queue::push(&queue, entry).unwrap();
        assert!(Queue::contains(&queue, "https://example.com/").unwrap());
    }

    #[test]
    fn test_trait_priority_ordering() {
        let queue = UrlQueue::new(ScopeConfig::default());

        let mut low = make_entry("https://example.com/low");
        low.priority = Priority::LOW;
        let mut high = make_entry("https://example.com/high");
        high.priority = Priority::HIGH;

        Queue::push(&queue, low).unwrap();
        Queue::push(&queue, high).unwrap();

        let popped = Queue::pop(&queue).unwrap().unwrap();
        assert_eq!(popped.url.as_str(), "https://example.com/high");
    }

    #[test]
    fn test_trait_empty_queue() {
        let queue = UrlQueue::new(ScopeConfig::default());
        assert!(Queue::pop(&queue).unwrap().is_none());
        assert_eq!(Queue::len(&queue).unwrap(), 0);
        assert!(Queue::is_empty(&queue).unwrap());
    }

    #[test]
    fn test_trait_object_safety() {
        fn _assert_queue_object_safe(_: &dyn Queue) {}
        let queue = UrlQueue::new(ScopeConfig::default());
        _assert_queue_object_safe(&queue);
    }

    #[tokio::test]
    async fn test_async_queue_push_pop() {
        use super::AsyncQueue;

        let queue: Arc<dyn Queue> = Arc::new(UrlQueue::new(ScopeConfig::default()));
        let async_queue = AsyncQueue::new(queue);

        let mut e1 = make_entry("https://example.com/low");
        e1.priority = Priority::LOW;
        let mut e2 = make_entry("https://example.com/high");
        e2.priority = Priority::HIGH;
        let e3 = make_entry("https://example.com/normal");

        async_queue.push_async(e1.clone()).await.unwrap();
        async_queue.push_async(e2.clone()).await.unwrap();
        async_queue.push_async(e3.clone()).await.unwrap();

        assert_eq!(async_queue.len_async().await.unwrap(), 3);

        let first = async_queue.pop_async().await.unwrap().unwrap();
        assert_eq!(first.url, e2.url);
        assert_eq!(first.priority, Priority::HIGH);

        let second = async_queue.pop_async().await.unwrap().unwrap();
        assert_eq!(second.url, e3.url);
        assert_eq!(second.priority, Priority::NORMAL);

        let third = async_queue.pop_async().await.unwrap().unwrap();
        assert_eq!(third.url, e1.url);
        assert_eq!(third.priority, Priority::LOW);

        assert!(async_queue.pop_async().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_async_queue_len() {
        use super::AsyncQueue;

        let queue: Arc<dyn Queue> = Arc::new(UrlQueue::new(ScopeConfig::default()));
        let async_queue = AsyncQueue::new(queue);

        for i in 0..5 {
            let entry = make_entry(&format!("https://example.com/page{i}"));
            async_queue.push_async(entry).await.unwrap();
        }

        assert_eq!(async_queue.len_async().await.unwrap(), 5);
        assert!(!async_queue.is_empty_async().await.unwrap());
    }
}
