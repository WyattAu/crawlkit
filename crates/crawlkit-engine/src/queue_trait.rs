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
/// Redis-backed [`DistributedQueue`](crate::distributed_queue::DistributedQueue)
/// implement this trait, allowing callers to swap backends without changing
/// queue-consuming code.
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
}
