//! Queue operator surface (ADR-015 §3): inspect and re-drive dead-lettered
//! crawl entries.
//!
//! The graduated Redis queue quarantines poison entries — deserialization
//! failures, invariant violations, entries that exhaust `MAX_ATTEMPTS` — in
//! a dead-letter zset so a poison message is a visible, actionable event
//! rather than a silent loss or an infinite retry loop. This command is the
//! operator half of that contract:
//!
//! - `crawlkit queue dead-letter list` — inspect quarantined entries with
//!   their failure reasons.
//! - `crawlkit queue dead-letter redrive <INDEX>` — re-queue an entry after
//!   fixing the defect (oldest = index 0), resetting its attempt count.
//!
//! Connection target comes from `CRAWLKIT_REDIS_URL` (default
//! `redis://127.0.0.1/`). The queue is namespaced per crawl ID; use
//! `--crawl-id` to select the namespace, or omit it to inspect every
//! namespace's dead-letter set via the shared `crawlkit:*:dead` pattern.

use anyhow::{bail, Context, Result};
use crawlkit_engine::distributed_queue::DistributedQueue;
use serde_json::json;

/// Which dead-letter set(s) to operate on.
#[derive(Debug, Clone)]
pub enum Target {
    /// One crawl namespace.
    One(String),
    /// Every `crawlkit:*:dead` namespace.
    All,
}

/// Extract the crawl ID from a `crawlkit:{id}:dead` key, or `None` when the
/// key is not a dead-letter namespace key. Pure so the pattern contract is
/// unit-testable; mirrors the queue's `crawlkit:{id}:dead` key layout.
fn parse_dead_key(key: &str) -> Option<&str> {
    let rest = key.strip_prefix("crawlkit:")?;
    let id = rest.strip_suffix(":dead")?;
    Some(id)
}

/// Resolve the crawl namespaces to operate on. `Target::All` scans
/// `crawlkit:*:dead` and extracts the crawl ID segment; `Target::One` yields
/// the single ID without a round-trip.
async fn resolve_crawl_ids(
    conn: &mut redis::aio::ConnectionManager,
    target: &Target,
) -> Result<Vec<String>> {
    match target {
        Target::One(id) => Ok(vec![id.clone()]),
        Target::All => {
            let keys: Vec<String> = redis::cmd("KEYS")
                .arg("crawlkit:*:dead")
                .query_async(conn)
                .await
                .context("cannot scan dead-letter namespaces")?;
            let mut ids: Vec<String> = keys
                .iter()
                .filter_map(|k| parse_dead_key(k))
                .map(str::to_string)
                .collect();
            ids.sort();
            Ok(ids)
        }
    }
}

/// List dead letters, one JSON object per line (NDJSON) so output pipes into
/// `jq` without quoting hazards. Timestamps are rendered as ISO-8601 in
/// addition to raw unix millis for operator readability. The `index` field
/// is the global row order across all listed namespaces — the same index
/// `redrive` accepts.
pub async fn list(target: &Target, redis_url: &str) -> Result<()> {
    let client = redis::Client::open(redis_url).context("invalid CRAWLKIT_REDIS_URL")?;
    let mut conn = client
        .get_connection_manager()
        .await
        .context("cannot connect to Redis")?;

    let crawl_ids = resolve_crawl_ids(&mut conn, target).await?;
    let mut row: usize = 0;
    for crawl_id in crawl_ids {
        let queue =
            DistributedQueue::new(redis_url, &crawl_id).context("cannot construct queue handle")?;
        for dl in queue.dead_letters().await? {
            let record = json!({
                "index": row,
                "crawl_id": crawl_id,
                "url": dl.entry.url,
                "depth": dl.entry.depth,
                "attempt_count": dl.entry.attempt_count,
                "reason": dl.reason,
                "dead_lettered_at_ms": dl.dead_lettered_at_ms,
                "dead_lettered_at": chrono::DateTime::from_timestamp_millis(dl.dead_lettered_at_ms)
                    .map(|t| t.to_rfc3339())
                    .unwrap_or_else(|| "invalid-timestamp".to_string()),
            });
            println!("{record}");
            row += 1;
        }
    }
    Ok(())
}

/// Re-drive the dead letter at `index` (global row order across all listed
/// namespaces, matching the `index` field emitted by `list`), resetting its
/// attempt count and re-queueing it into the pending set.
pub async fn redrive(index: usize, target: &Target, redis_url: &str) -> Result<()> {
    let client = redis::Client::open(redis_url).context("invalid CRAWLKIT_REDIS_URL")?;
    let mut conn = client
        .get_connection_manager()
        .await
        .context("cannot connect to Redis")?;

    let crawl_ids = resolve_crawl_ids(&mut conn, target).await?;
    let mut lengths = Vec::with_capacity(crawl_ids.len());
    for crawl_id in &crawl_ids {
        let queue =
            DistributedQueue::new(redis_url, crawl_id).context("cannot construct queue handle")?;
        lengths.push(queue.dead_letters().await?.len());
    }
    let Some((crawl_id, local_index)) = plan_redrive(&crawl_ids, &lengths, index) else {
        bail!(
            "index {index} is out of range; run `crawlkit queue dead-letter list` to see entries"
        );
    };
    let queue =
        DistributedQueue::new(redis_url, &crawl_id).context("cannot construct queue handle")?;
    if !queue.redrive(local_index).await? {
        bail!("entry {index} vanished while redriving; re-run `crawlkit queue dead-letter list`");
    }
    println!("redrived entry {index} from crawl {crawl_id}");
    Ok(())
}

/// Map a global row index onto its namespace and namespace-local index.
///
/// `crawl_ids[i]` has `lengths[i]` dead letters; the global index space is
/// their concatenation in order. Returns `None` when `index` is out of
/// range. Pure so the arithmetic is unit-testable without Redis.
fn plan_redrive(crawl_ids: &[String], lengths: &[usize], index: usize) -> Option<(String, usize)> {
    let mut remaining = index;
    for (crawl_id, len) in crawl_ids.iter().zip(lengths) {
        if remaining < *len {
            return Some((crawl_id.clone(), remaining));
        }
        remaining -= len;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("crawl-{i}")).collect()
    }

    #[test]
    fn plan_redrive_first_namespace() {
        let plan = plan_redrive(&ids(2), &[3, 2], 1);
        assert_eq!(plan, Some(("crawl-0".to_string(), 1)));
    }

    #[test]
    fn plan_redrive_crosses_namespace_boundary() {
        let plan = plan_redrive(&ids(3), &[3, 2, 5], 3);
        assert_eq!(plan, Some(("crawl-1".to_string(), 0)));
        let plan = plan_redrive(&ids(3), &[3, 2, 5], 9);
        assert_eq!(plan, Some(("crawl-2".to_string(), 4)));
    }

    #[test]
    fn plan_redrive_out_of_range() {
        assert_eq!(plan_redrive(&ids(2), &[3, 2], 5), None);
        assert_eq!(plan_redrive(&ids(2), &[0, 0], 0), None);
        assert_eq!(plan_redrive(&[], &[], 0), None);
    }

    #[test]
    fn plan_redrive_zero_index_hits_first_nonempty() {
        let plan = plan_redrive(&ids(3), &[0, 2, 5], 0);
        assert_eq!(plan, Some(("crawl-1".to_string(), 0)));
    }

    #[test]
    fn parse_dead_key_extracts_crawl_id() {
        assert_eq!(
            parse_dead_key("crawlkit:my-crawl-123:dead"),
            Some("my-crawl-123")
        );
        assert_eq!(parse_dead_key("crawlkit::dead"), Some(""));
        // Not a dead-letter key
        assert_eq!(parse_dead_key("crawlkit:crawl:pending"), None);
        assert_eq!(parse_dead_key("crawlkit:politeness:host"), None);
        assert_eq!(parse_dead_key("unrelated"), None);
        // Suffix must be exactly ":dead"
        assert_eq!(parse_dead_key("crawlkit:crawl:deader"), None);
    }

    // --- Redis-backed handler tests (ADR-015 §7 crash-recovery evidence
    // for the operator surface; run against the CI Redis service) ---

    fn redis_url() -> String {
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1/".to_string())
    }

    fn now_ms() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64
    }

    /// Unique crawl ID per run — the queue namespaces all keys per crawl, so
    /// this isolates the test's key space without flushing shared state.
    fn unique_crawl_id(tag: &str) -> String {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        format!("qops-{tag}-{}-{n}", std::process::id())
    }

    /// Push and fatally fail an entry so it lands in the dead-letter set.
    async fn dead_letter_one(queue: &DistributedQueue, url: &str) {
        let entry =
            crawlkit_engine::distributed_queue::DistributedQueueEntry::new(url, 0, 10, now_ms());
        queue.push(&entry).await.unwrap();
        let lease = queue.pop().await.unwrap().expect("entry available");
        let dead = queue
            .fail(&lease, "gone", "410 test", now_ms())
            .await
            .unwrap();
        assert!(dead, "fatal failure must dead-letter");
    }

    #[tokio::test]
    #[ignore = "requires running Redis instance"]
    async fn list_runs_against_live_dead_letters() {
        let crawl_id = unique_crawl_id("list");
        let queue = DistributedQueue::new(&redis_url(), &crawl_id).unwrap();
        queue.clear().await.unwrap();
        dead_letter_one(&queue, &format!("https://example.com/{}", now_ms())).await;

        // Exercises resolve → connect → render end-to-end; per-row output
        // shape is asserted via the underlying state above.
        list(&Target::One(crawl_id.clone()), &redis_url())
            .await
            .unwrap();
        assert_eq!(queue.dead_letters().await.unwrap().len(), 1);
        queue.clear().await.unwrap();
    }

    #[tokio::test]
    #[ignore = "requires running Redis instance"]
    async fn redrive_requeues_and_resets_attempts() {
        let crawl_id = unique_crawl_id("redrive");
        let url = format!("https://example.com/{}", now_ms());
        let queue = DistributedQueue::new(&redis_url(), &crawl_id).unwrap();
        queue.clear().await.unwrap();
        dead_letter_one(&queue, &url).await;

        redrive(0, &Target::One(crawl_id.clone()), &redis_url())
            .await
            .unwrap();
        assert_eq!(queue.dead_letters().await.unwrap().len(), 0);
        assert_eq!(
            queue.len().await.unwrap(),
            1,
            "redriven entry is pending again"
        );
        let lease = queue.pop().await.unwrap().expect("redriven entry");
        assert_eq!(lease.entry.attempt_count, 0, "redrive resets attempts");
        assert_eq!(lease.entry.url, url);
        queue.clear().await.unwrap();
    }

    #[tokio::test]
    #[ignore = "requires running Redis instance"]
    async fn redrive_out_of_range_is_an_error() {
        let crawl_id = unique_crawl_id("range");
        let queue = DistributedQueue::new(&redis_url(), &crawl_id).unwrap();
        queue.clear().await.unwrap();
        assert!(
            redrive(7, &Target::One(crawl_id.clone()), &redis_url())
                .await
                .is_err(),
            "empty namespace index 7 must be out of range"
        );
        queue.clear().await.unwrap();
    }
}
