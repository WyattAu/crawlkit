//! Queue trait contract harness.
//!
//! Defines invariants that *any* `Queue` implementation must satisfy.
//! The in-memory `UrlQueue` is the reference baseline. When a second
//! implementation (e.g. Redis) is added, it must call
//! `assert_queue_contract` so the two cannot silently diverge.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use chrono::Utc;
use crawlkit_engine::queue::{Priority, QueueEntry, ScopeConfig, UrlQueue};
use crawlkit_engine::queue_trait::Queue;
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

/// Assert the full queue contract against a freshly-constructed queue.
#[allow(dead_code)]
pub fn assert_queue_contract(queue: &dyn Queue) {
    // --- empty queue ---
    assert!(queue.is_empty().unwrap());
    assert_eq!(queue.len().unwrap(), 0);
    assert!(queue.pop().unwrap().is_none());

    // --- push + pop round-trip ---
    let entry = make_entry("https://example.com/page1");
    let url = entry.url.clone();
    assert!(queue.push(entry).unwrap());
    assert_eq!(queue.len().unwrap(), 1);
    assert!(!queue.is_empty().unwrap());

    let popped = queue.pop().unwrap().unwrap();
    assert_eq!(popped.url, url);
    assert!(queue.is_empty().unwrap());

    // --- deduplication ---
    let dup = make_entry("https://example.com/dup");
    let dup_url = dup.url.clone();
    assert!(queue.push(dup).unwrap(), "first push of a URL must succeed");
    assert!(
        !queue.push(make_entry("https://example.com/dup")).unwrap(),
        "second push of the same URL must be rejected"
    );
    assert_eq!(queue.len().unwrap(), 1);
    let popped = queue.pop().unwrap().unwrap();
    assert_eq!(popped.url, dup_url);

    // --- priority ordering ---
    let mut low = make_entry("https://example.com/low");
    low.priority = Priority::LOW;
    let mut high = make_entry("https://example.com/high");
    high.priority = Priority::HIGH;
    let normal = make_entry("https://example.com/normal");

    queue.push(low).unwrap();
    queue.push(high).unwrap();
    queue.push(normal).unwrap();

    let first = queue.pop().unwrap().unwrap();
    assert_eq!(first.priority, Priority::HIGH, "HIGH must come out first");
    let second = queue.pop().unwrap().unwrap();
    assert_eq!(
        second.priority,
        Priority::NORMAL,
        "NORMAL must come out second"
    );
    let third = queue.pop().unwrap().unwrap();
    assert_eq!(third.priority, Priority::LOW, "LOW must come out last");

    // --- contains ---
    let check = make_entry("https://example.com/check");
    let check_url = check.url.clone();
    assert!(
        !queue.contains(check_url.as_str()).unwrap(),
        "URL not yet pushed must not be present"
    );
    queue.push(check).unwrap();
    assert!(
        queue.contains("https://example.com/check").unwrap(),
        "pushed URL must be present"
    );
}

#[test]
fn in_memory_url_queue_satisfies_queue_contract() {
    let queue = UrlQueue::new(ScopeConfig::default());
    assert_queue_contract(&queue);
}
