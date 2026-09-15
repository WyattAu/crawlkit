//! `Queue`-trait adapter over the Redis [`DistributedQueue`] (ADR-015 §5).
//!
//! This is the piece that lets the crawl engine's frontier consume the
//! graduated lease queue: [`CrawlEngine::run_with_callback`] pops
//! [`QueueEntry`] values from any `Arc<dyn Queue>`; this adapter fronts the
//! async lease queue with a sync facade and remembers, per entry, the
//! outstanding lease so the engine can [`ack`](Queue::ack) on success or
//! [`fail`](Queue::fail) into the retry/backoff/dead-letter path.
//!
//! Lease bookkeeping: `pop` registers `lease_id → entry.url` and `ack`/`fail`
//! remove the registration. Leases whose entry is never resolved are
//! reclaimed by the queue's own sweeper (at-least-once semantics preserved:
//! the entry is redelivered, and processing is idempotent).
//!
//! Requires the `unstable` feature (same gate as the queue itself).

use std::collections::HashMap;
use std::sync::Mutex;

use crate::distributed_queue::{DistributedQueue, DistributedQueueEntry, LeaseState};
use crate::queue::QueueEntry;
use crate::queue_trait::{Queue, QueueError};

/// Sync `Queue` facade over an async [`DistributedQueue`].
///
/// All operations use `tokio::task::block_in_place` + `block_on` against a
/// dedicated runtime handle: the engine's dispatch loop is sync by design and
/// runs on a multi-thread runtime, where `block_in_place` is legal and moves
/// the blocking work off the async workers.
pub struct DistributedQueueAdapter {
    queue: DistributedQueue,
    runtime: tokio::runtime::Handle,
    /// Outstanding leases keyed by entry URL. Capacity-bounded indirectly:
    /// every registration is paired with exactly one ack/fail/sweep outcome
    /// in a healthy engine loop.
    leases: Mutex<HashMap<String, String>>,
}

impl DistributedQueueAdapter {
    /// Wrap a queue with the sync facade, using the current tokio runtime's
    /// handle for the blocking bridge.
    ///
    /// # Panics
    ///
    /// Panics if called outside a tokio runtime (the engine's entry points
    /// are all async, so this holds for every supported caller).
    pub fn new(queue: DistributedQueue) -> Self {
        Self {
            queue,
            runtime: tokio::runtime::Handle::current(),
            leases: Mutex::new(HashMap::new()),
        }
    }

    fn block_on<F, T>(&self, fut: F) -> Result<T, QueueError>
    where
        F: std::future::Future<Output = Result<T, crate::distributed_queue::RedisQueueError>>,
    {
        tokio::task::block_in_place(|| self.runtime.block_on(fut))
            .map_err(|e| QueueError::Backend(e.to_string()))
    }
}

impl Queue for DistributedQueueAdapter {
    fn push(&self, entry: QueueEntry) -> Result<bool, QueueError> {
        let now_ms = chrono::Utc::now().timestamp_millis();
        let dq_entry = DistributedQueueEntry::new(
            entry.url.as_str(),
            entry.depth,
            entry.priority.value() as i64,
            now_ms,
        );
        self.block_on(self.queue.push(&dq_entry))
    }

    fn pop(&self) -> Result<Option<QueueEntry>, QueueError> {
        let lease: Option<LeaseState> = self.block_on(self.queue.pop())?;
        let Some(lease) = lease else {
            return Ok(None);
        };
        let url = lease.entry.url.clone();
        let lease_id = lease.lease_id.clone();
        if let Ok(mut leases) = self.leases.lock() {
            leases.insert(url.clone(), lease_id);
        }
        // The engine's QueueEntry needs a parseable Url; queue entries are
        // validated at push time, but a corrupted payload must not kill the
        // crawl — drop it with a fail report (counted, transient) instead.
        match url::Url::parse(&url) {
            Ok(parsed) => Ok(Some(QueueEntry {
                canonical_url: parsed.clone(),
                url: parsed,
                depth: lease.entry.depth,
                priority: crate::queue::Priority::new(lease.entry.priority.clamp(0, 255) as u8),
                discovered_at: chrono::DateTime::from_timestamp_millis(
                    lease.entry.first_enqueued_at,
                )
                .unwrap_or_else(chrono::Utc::now),
                referrer: None,
            })),
            Err(e) => {
                let _ = self.block_on(self.queue.fail(
                    &lease,
                    "malformed",
                    &format!("unparseable queued URL: {e}"),
                    chrono::Utc::now().timestamp_millis(),
                ));
                if let Ok(mut leases) = self.leases.lock() {
                    leases.remove(&url);
                }
                Ok(None)
            }
        }
    }

    fn len(&self) -> Result<usize, QueueError> {
        self.block_on(self.queue.len())
    }

    fn is_empty(&self) -> Result<bool, QueueError> {
        self.block_on(self.queue.is_empty())
    }

    fn contains(&self, url: &str) -> Result<bool, QueueError> {
        self.block_on(self.queue.contains(url))
    }

    fn ack(&self, entry: &QueueEntry) {
        let lease_id = self
            .leases
            .lock()
            .ok()
            .and_then(|mut l| l.remove(entry.url.as_str()));
        if let Some(lease_id) = lease_id {
            let _ = self.block_on(self.queue.ack(&lease_id));
        }
    }

    fn fail(&self, entry: &QueueEntry, kind: &str, reason: &str) {
        // Rebuild the lease handle: the entry round-trips through Redis, so
        // reconstruct the LeaseState fields the fail script needs.
        let lease_id = self
            .leases
            .lock()
            .ok()
            .and_then(|mut l| l.remove(entry.url.as_str()));
        let Some(lease_id) = lease_id else {
            return;
        };
        let now_ms = chrono::Utc::now().timestamp_millis();
        let dq_entry = DistributedQueueEntry {
            url: entry.url.as_str().to_string(),
            depth: entry.depth,
            priority: entry.priority.value() as i64,
            attempt_count: 0, // informational only; the script mutates its own copy
            first_enqueued_at: now_ms,
        };
        let lease = LeaseState {
            lease_id,
            entry: dq_entry,
            expires_at_ms: 0,
        };
        let _ = self.block_on(self.queue.fail(&lease, kind, reason, now_ms));
    }
}

/// Failure-kind classification lives in [`crate::queue_trait`] so the
/// engine's crawl loop can use it in every feature configuration; re-exported
/// here for adapter users.
pub use crate::queue_trait::{failure_is_fatal, failure_kind};

#[cfg(all(test, feature = "unstable"))]
mod redis_tests {
    use super::*;

    fn redis_url() -> String {
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1/".to_string())
    }

    async fn fresh_queue(name: &str) -> DistributedQueueAdapter {
        let q = DistributedQueue::new(&redis_url(), name).unwrap();
        q.clear().await.unwrap();
        DistributedQueueAdapter::new(q)
    }

    fn entry(url: &str) -> QueueEntry {
        QueueEntry {
            url: url::Url::parse(url).unwrap(),
            canonical_url: url::Url::parse(url).unwrap(),
            depth: 0,
            priority: crate::queue::Priority::NORMAL,
            discovered_at: chrono::Utc::now(),
            referrer: None,
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore = "requires running Redis instance"]
    async fn push_pop_ack_roundtrip() {
        let q = fresh_queue("adapter:roundtrip").await;
        assert!(q.push(entry("http://a.example/1")).unwrap());
        let popped = q.pop().unwrap().expect("entry pending");
        assert_eq!(popped.url.as_str(), "http://a.example/1");
        // ack releases the lease; the queue is empty again.
        q.ack(&popped);
        assert!(q.is_empty().unwrap());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore = "requires running Redis instance"]
    async fn fail_transient_requeues_with_backoff() {
        let q = fresh_queue("adapter:retry").await;
        q.push(entry("http://b.example/1")).unwrap();
        let popped = q.pop().unwrap().unwrap();
        q.fail(&popped, "timeout", "connect timed out");
        // Entry is back in the pending set (lease released, retry scheduled).
        assert_eq!(q.len().unwrap(), 1);
        assert!(q.contains("http://b.example/1").unwrap());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore = "requires running Redis instance"]
    async fn fail_fatal_dead_letters() {
        let q = fresh_queue("adapter:fatal").await;
        q.push(entry("http://c.example/gone")).unwrap();
        let popped = q.pop().unwrap().unwrap();
        q.fail(&popped, "gone", "410 per classify taxonomy");
        assert!(q.is_empty().unwrap());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore = "requires running Redis instance"]
    async fn failure_kind_maps_to_taxonomy() {
        use crate::CrawlError;
        assert_eq!(
            failure_kind(&CrawlError::TooManyRedirects(9)),
            "too_many_redirects"
        );
        assert_eq!(
            failure_kind(&CrawlError::InvalidUrl(url::Url::parse("::").unwrap_err())),
            "malformed"
        );
        assert!(failure_is_fatal(&CrawlError::InvalidUrl(
            url::Url::parse("::").unwrap_err()
        )));
    }
}
