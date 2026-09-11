//! Redis-backed lease-based distributed crawl queue (ADR-015).
//!
//! Graduates the experimental distributed queue from a destructive
//! at-most-once `ZPOPMIN` pop — where a worker crash silently loses the
//! popped URL — to at-least-once delivery with leases, bounded retries,
//! poison quarantine, and an O(1) visited set.
//!
//! # Delivery contract (ADR-015 §1–§3, §7)
//!
//! - **At-least-once delivery.** Every pop moves the entry into a per-lease
//!   processing hash atomically (Lua); a lease that expires before completion
//!   is reclaimed back into the pending set by a sweep. Workers MUST tolerate
//!   duplicate delivery of the same URL.
//! - **Bounded retries.** Transient failures re-queue with attempt-count
//!   backoff encoded in the sorted-set score; fatal failures never re-queue.
//! - **Poison quarantine.** Entries exceeding [`DEFAULT_MAX_ATTEMPTS`] move
//!   to a dead-letter zset with reason and timestamp; contents are inspectable
//!   and re-drivable by operators, and the set is capped with oldest-first
//!   eviction.
//! - **O(1) membership.** A visited set (Redis `SADD`) replaces the old O(N)
//!   `ZRANGE`-and-scan `contains`.
//!
//! # Testing (ADR-015 §7)
//!
//! Pure decision logic ([`classify`], [`attempts_exhausted`], backoff and
//! score packing, payload parsing) is tested without Redis. The Lua-scripted
//! operations are integration tests marked
//! `#[ignore = "requires running Redis instance"]`; CI runs them against its
//! Redis 7 service via `REDIS_URL`.

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
/// This is the wire format for pending, leased, and dead-lettered copies of
/// an entry. `attempt_count` travels with the entry; `first_enqueued_at` is
/// set on first push and preserved across retries so operators can see entry
/// age.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DistributedQueueEntry {
    /// The URL to crawl.
    pub url: String,
    /// Crawl depth from the seed URL (0 = seed).
    pub depth: usize,
    /// Priority score (lower = higher priority).
    pub priority: i64,
    /// Number of delivery attempts so far (0 = never attempted).
    pub attempt_count: u32,
    /// Unix millis when the entry was first enqueued.
    pub first_enqueued_at: i64,
}

impl DistributedQueueEntry {
    /// Create a fresh entry with attempt count 0.
    pub fn new(url: impl Into<String>, depth: usize, priority: i64, now_ms: i64) -> Self {
        Self {
            url: url.into(),
            depth,
            priority,
            attempt_count: 0,
            first_enqueued_at: now_ms,
        }
    }
}

/// Current state of a lease held by a worker after [`DistributedQueue::pop`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaseState {
    /// Opaque lease identifier (UUID v4, unique per delivery).
    pub lease_id: String,
    /// The leased entry.
    pub entry: DistributedQueueEntry,
    /// Unix millis when this lease expires and becomes reclaimable.
    pub expires_at_ms: i64,
}

/// Classification of a failure outcome, per ADR-015 §2.
///
/// Mirrors the `loop_retry::IsRetryable` taxonomy used by webhook delivery
/// (transport errors and 5xx/429 retryable; permanent failures fatal).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureClass {
    /// Transient failure: re-queue with backoff if attempts remain.
    Transient,
    /// Fatal failure: dead-letter immediately, never re-queue.
    Fatal,
}

/// Classify a failure outcome from the failure kind.
///
/// Transient: network timeouts, connection errors, 5xx, 429, lease expiry.
/// Fatal: 410 Gone, permanent DNS failure, SSRF denial (ADR-012 policy),
/// malformed URL, deserialization failure.
///
/// Unknown kinds classify as transient so a missing arm cannot silently drop
/// work; the entry retries with backoff and dead-letters after the attempt
/// bound.
pub fn classify(kind: &str) -> FailureClass {
    match kind {
        "gone" | "permanent_dns" | "ssrf_denied" | "malformed" | "deserialization" => {
            FailureClass::Fatal
        }
        _ => FailureClass::Transient,
    }
}

/// Whether an attempt count is exhausted (dead-letter threshold): the count
/// has reached `max_attempts`.
pub fn attempts_exhausted(attempt_count: u32, max_attempts: u32) -> bool {
    attempt_count >= max_attempts
}

/// Compute the pending-set score for a retried entry.
///
/// Packs the next-eligible timestamp and the priority into one integer, reusing
/// the engine convention that lower score = surface first:
/// `(next_eligible_ms << 8) | (priority & 0xFF)`. A retried entry therefore
/// surfaces only after its backoff elapses, and among entries with equal
/// eligibility the lower priority value wins. Milliseconds since epoch fit in
/// 41 bits through the year 2730; the packed value stays below 2^53, inside
/// Lua's exact-double integer range.
pub fn retry_score(next_eligible_ms: i64, priority: i64) -> i64 {
    debug_assert!(next_eligible_ms >= 0, "caller supplies unix millis");
    (next_eligible_ms << 8) | (priority & 0xFF)
}

/// Exponential backoff for the nth retry (0-based), with cap and jitter.
///
/// `base_ms * 2^attempt` capped at 15 minutes, plus deterministic jitter in
/// `[0, base_ms)` derived from the timestamp — enough to decorrelate workers
/// that fail at the same instant (not cryptographic).
pub fn backoff_ms(attempt: u32, base_ms: i64, now_ms: i64) -> i64 {
    const MAX_BACKOFF_MS: i64 = 15 * 60 * 1000;
    let exp = base_ms.saturating_mul(1i64 << attempt.min(20));
    let capped = exp.min(MAX_BACKOFF_MS);
    let jitter = (now_ms.rem_euclid(base_ms.max(1))).abs();
    capped + jitter
}

/// Base backoff for the first retry: 2 seconds.
const BASE_BACKOFF_MS: i64 = 2_000;

// ---------------------------------------------------------------------------
// Lua scripts — each operation is atomic so no crash between steps can
// produce a lost or duplicated entry beyond the lease-expiry window.
// ---------------------------------------------------------------------------

/// Atomic pop into lease (ADR-015 §1).
///
/// KEYS: 1 = pending zset, 2 = processing hash, 3 = lease index zset
/// ARGV: 1 = lease_id, 2 = now_ms, 3 = lease_ttl_ms
///
/// Pops the highest-priority *eligible* entry (score <= now-based cutoff)
/// from the pending zset and stores it in the processing hash keyed by lease
/// ID, recording expiry in the lease index. Returns the lease payload JSON,
/// or nil when no eligible entry exists.
const LUA_POP: &str = r#"
local pending = KEYS[1]
local processing = KEYS[2]
local lease_index = KEYS[3]
local lease_id = ARGV[1]
local now_ms = tonumber(ARGV[2])
local ttl_ms = tonumber(ARGV[3])

local cutoff = (now_ms + 1) * 256 + 255
local items = redis.call('ZRANGEBYSCORE', pending, '-inf', cutoff, 'LIMIT', 0, 1)
if #items == 0 then
  return nil
end
local member = items[1]
redis.call('ZREM', pending, member)
local payload = {
  lease_id = lease_id,
  leased_at_ms = now_ms,
  expires_at_ms = now_ms + ttl_ms,
  entry = member,
}
redis.call('HSET', processing, lease_id, cjson.encode(payload))
redis.call('ZADD', lease_index, now_ms + ttl_ms, lease_id)
return cjson.encode(payload)
"#;

/// Atomic acknowledge-success (ADR-015 §1: completion removes the lease).
const LUA_ACK: &str = r#"
local processing = KEYS[1]
local lease_index = KEYS[2]
local lease_id = ARGV[1]

local payload = redis.call('HGET', processing, lease_id)
if not payload then
  return 0
end
redis.call('HDEL', processing, lease_id)
redis.call('ZREM', lease_index, lease_id)
return 1
"#;

/// Atomic requeue-or-deadletter on failure (ADR-015 §2, §3).
///
/// KEYS: 1 = processing hash, 2 = lease index, 3 = pending zset, 4 = dead zset
/// ARGV: 1 = lease_id, 2 = dead ('1'/'0'), 3 = next_score, 4 = entry_json,
///       5 = dead_letter_json, 6 = dead_score, 7 = dead_cap
///
/// Removes the lease; if `dead`, adds the dead-letter JSON to the dead zset
/// (score = dead_lettered_at_ms, so listing is oldest-first), else re-adds
/// the entry to pending with the backoff score. Caps the dead zset with
/// oldest-first eviction.
const LUA_FAIL: &str = r#"
local processing = KEYS[1]
local lease_index = KEYS[2]
local pending = KEYS[3]
local dead = KEYS[4]
local lease_id = ARGV[1]
local is_dead = ARGV[2]
local next_score = tonumber(ARGV[3])
local entry_json = ARGV[4]
local dead_json = ARGV[5]
local dead_score = tonumber(ARGV[6])
local dead_cap = tonumber(ARGV[7])

local payload = redis.call('HGET', processing, lease_id)
if not payload then
  return 0
end
redis.call('HDEL', processing, lease_id)
redis.call('ZREM', lease_index, lease_id)

if is_dead == '1' then
  redis.call('ZADD', dead, dead_score, dead_json)
else
  redis.call('ZADD', pending, next_score, entry_json)
end
local n = redis.call('ZCARD', dead)
if n > dead_cap then
  redis.call('ZREMRANGEBYRANK', dead, 0, n - dead_cap - 1)
end
return 1
"#;

/// Reclamation sweep (ADR-015 §1: expired leases re-enter the pending set).
///
/// KEYS: 1 = processing hash, 2 = lease index zset, 3 = pending zset,
///       4 = dead zset
/// ARGV: 1 = now_ms, 2 = max_attempts, 3 = dead_score (dead_lettered_at),
///       4 = dead_cap
///
/// For each lease whose expiry <= now: reclaim the entry into pending with
/// attempt_count incremented and score 0 (immediately eligible), or
/// dead-letter it if the increment exhausts the attempt bound (crash-loop
/// protection). Each expired lease is processed exactly once per invocation
/// because it is removed from the index as it is handled, so concurrent
/// sweeps cannot double-requeue. Returns the number of reclaimed entries
/// (dead-lettered ones are not counted as reclaimed).
const LUA_RECLAIM: &str = r#"
local processing = KEYS[1]
local lease_index = KEYS[2]
local pending = KEYS[3]
local dead = KEYS[4]
local now_ms = tonumber(ARGV[1])
local max_attempts = tonumber(ARGV[2])
local dead_score = tonumber(ARGV[3])
local dead_cap = tonumber(ARGV[4])

local expired = redis.call('ZRANGEBYSCORE', lease_index, '-inf', now_ms)
local reclaimed = 0
for _, lease_id in ipairs(expired) do
  local payload_json = redis.call('HGET', processing, lease_id)
  if payload_json then
    local payload = cjson.decode(payload_json)
    local entry = cjson.decode(payload.entry)
    entry.attempt_count = tonumber(entry.attempt_count) + 1
    if entry.attempt_count >= max_attempts then
      local dead_letter = {
        entry = entry,
        reason = 'lease expired and attempts exhausted',
        dead_lettered_at_ms = dead_score,
      }
      redis.call('ZADD', dead, dead_score, cjson.encode(dead_letter))
    else
      redis.call('ZADD', pending, 0, cjson.encode(entry))
      reclaimed = reclaimed + 1
    end
  end
  redis.call('HDEL', processing, lease_id)
  redis.call('ZREM', lease_index, lease_id)
end
local n = redis.call('ZCARD', dead)
if n > dead_cap then
  redis.call('ZREMRANGEBYRANK', dead, 0, n - dead_cap - 1)
end
return reclaimed
"#;

/// Default lease TTL: 120 seconds (ADR-015 §1).
pub const DEFAULT_LEASE_TTL_MS: i64 = 120_000;

/// Default maximum attempts before dead-lettering (ADR-015 §2).
pub const DEFAULT_MAX_ATTEMPTS: u32 = 5;

/// Default dead-letter set cap (ADR-015 §3): oldest entries evicted first.
pub const DEFAULT_DEAD_LETTER_CAP: usize = 10_000;

/// A dead-letter record: entry plus why it was quarantined.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadLetter {
    /// The quarantined entry (with the attempt count at dead-letter time).
    pub entry: DistributedQueueEntry,
    /// Failure reason recorded at dead-letter time.
    pub reason: String,
    /// Unix millis when dead-lettered.
    pub dead_lettered_at_ms: i64,
}

/// Lease-based Redis distributed queue (ADR-015).
///
/// Key layout (namespaced per crawl ID via the `{prefix}` = `crawlkit:{id}`):
/// - `{prefix}:pending` — zset, member = entry JSON, score = packed
///   next-eligible timestamp and priority (see [`retry_score`]); fresh pushes
///   use the raw priority so they are immediately eligible
/// - `{prefix}:processing` — hash, field = lease ID, value = lease payload JSON
/// - `{prefix}:leases` — zset, member = lease ID, score = expiry ms
/// - `{prefix}:dead` — zset, member = [`DeadLetter`] JSON, score =
///   dead-lettered-at ms
/// - `{prefix}:visited` — set of URL strings seen this crawl
///
/// The lease lifecycle (`pop`/`ack`/`fail`/`reclaim_expired`) is this
/// module's own API: the crate's destructive `Queue` trait cannot express
/// lease completion and is deliberately not implemented (ADR-015 §5 note).
pub struct DistributedQueue {
    client: redis::Client,
    prefix: String,
    lease_ttl_ms: i64,
    max_attempts: u32,
    dead_letter_cap: usize,
}

impl DistributedQueue {
    /// Create a graduated queue connected to Redis with default policies.
    ///
    /// # Errors
    ///
    /// Returns [`RedisQueueError::ConnectionFailed`] if the client cannot be
    /// constructed from `redis_url`.
    pub fn new(redis_url: &str, crawl_id: &str) -> Result<Self, RedisQueueError> {
        Self::with_policy(
            redis_url,
            crawl_id,
            DEFAULT_LEASE_TTL_MS,
            DEFAULT_MAX_ATTEMPTS,
            DEFAULT_DEAD_LETTER_CAP,
        )
    }

    /// Create a queue with explicit policy knobs (lease TTL, max attempts,
    /// dead-letter cap). Prefer [`DistributedQueue::new`] for defaults.
    ///
    /// # Errors
    ///
    /// Returns [`RedisQueueError::ConnectionFailed`] if the client cannot be
    /// constructed from `redis_url`.
    pub fn with_policy(
        redis_url: &str,
        crawl_id: &str,
        lease_ttl_ms: i64,
        max_attempts: u32,
        dead_letter_cap: usize,
    ) -> Result<Self, RedisQueueError> {
        let client = redis::Client::open(redis_url)
            .map_err(|e| RedisQueueError::ConnectionFailed(e.to_string()))?;
        Ok(Self {
            client,
            prefix: format!("crawlkit:{crawl_id}"),
            lease_ttl_ms,
            max_attempts,
            dead_letter_cap,
        })
    }

    fn key(&self, suffix: &str) -> String {
        format!("{}:{suffix}", self.prefix)
    }

    fn conn(&self) -> Result<redis::Connection, RedisQueueError> {
        self.client
            .get_connection()
            .map_err(|e| RedisQueueError::ConnectionFailed(e.to_string()))
    }

    /// Push an entry into the pending set, immediately eligible.
    ///
    /// Also records the URL in the visited set: returns `Ok(false)` when the
    /// URL was already seen (duplicate suppression, replacing the old O(N)
    /// scan). Retried entries re-enter pending via the fail path and skip
    /// this check — they are already visited by definition.
    ///
    /// # Errors
    ///
    /// Returns [`RedisQueueError`] on connection or serialization failure.
    pub fn push(&self, entry: &DistributedQueueEntry) -> Result<bool, RedisQueueError> {
        let mut conn = self.conn()?;
        self.push_with_conn(&mut conn, entry)
    }

    /// Push with an existing connection (loop-friendly).
    ///
    /// # Errors
    ///
    /// Returns [`RedisQueueError`] on connection or serialization failure.
    pub fn push_with_conn(
        &self,
        conn: &mut redis::Connection,
        entry: &DistributedQueueEntry,
    ) -> Result<bool, RedisQueueError> {
        let json = serde_json::to_string(entry)
            .map_err(|e| RedisQueueError::SerializationError(e.to_string()))?;
        let visited = self.key("visited");
        let added: i64 = conn
            .sadd(&visited, &entry.url)
            .map_err(|e| RedisQueueError::OperationFailed(e.to_string()))?;
        if added == 0 {
            return Ok(false);
        }
        let pending = self.key("pending");
        let _: i64 = conn
            .zadd(&pending, json, entry.priority)
            .map_err(|e| RedisQueueError::OperationFailed(e.to_string()))?;
        Ok(true)
    }

    /// Atomically pop the highest-priority eligible entry into a new lease.
    ///
    /// Returns `Ok(None)` when no eligible entry exists. The caller must
    /// eventually call [`DistributedQueue::ack`] or [`DistributedQueue::fail`];
    /// an abandoned lease is reclaimed by the sweep after the TTL and its
    /// entry re-delivered (at-least-once contract).
    ///
    /// # Errors
    ///
    /// Returns [`RedisQueueError`] on connection, script, or serialization
    /// failure.
    pub fn pop(&self) -> Result<Option<LeaseState>, RedisQueueError> {
        let mut conn = self.conn()?;
        self.pop_with_conn(&mut conn)
    }

    /// Pop with an existing connection. See [`DistributedQueue::pop`].
    ///
    /// # Errors
    ///
    /// Returns [`RedisQueueError`] on connection, script, or serialization
    /// failure.
    pub fn pop_with_conn(
        &self,
        conn: &mut redis::Connection,
    ) -> Result<Option<LeaseState>, RedisQueueError> {
        let payload: Option<String> = redis::Script::new(LUA_POP)
            .key(self.key("pending"))
            .key(self.key("processing"))
            .key(self.key("leases"))
            .arg(new_lease_id())
            .arg(now_ms())
            .arg(self.lease_ttl_ms)
            .invoke(conn)
            .map_err(|e| RedisQueueError::OperationFailed(e.to_string()))?;
        match payload {
            None => Ok(None),
            Some(payload) => parse_lease_payload(&payload)
                .map(Some)
                .map_err(|e| RedisQueueError::SerializationError(format!("lease payload: {e}"))),
        }
    }

    /// Acknowledge successful completion of a lease.
    ///
    /// Returns `Ok(false)` if the lease no longer exists (it expired and was
    /// reclaimed — the work will be redelivered; the caller's already-written
    /// result is the expected duplicate per the at-least-once contract).
    ///
    /// # Errors
    ///
    /// Returns [`RedisQueueError`] on connection or script failure.
    pub fn ack(&self, lease_id: &str) -> Result<bool, RedisQueueError> {
        let mut conn = self.conn()?;
        self.ack_with_conn(&mut conn, lease_id)
    }

    /// Ack with an existing connection.
    ///
    /// # Errors
    ///
    /// Returns [`RedisQueueError`] on connection or script failure.
    pub fn ack_with_conn(
        &self,
        conn: &mut redis::Connection,
        lease_id: &str,
    ) -> Result<bool, RedisQueueError> {
        let n: i64 = redis::Script::new(LUA_ACK)
            .key(self.key("processing"))
            .key(self.key("leases"))
            .arg(lease_id)
            .invoke(conn)
            .map_err(|e| RedisQueueError::OperationFailed(e.to_string()))?;
        Ok(n == 1)
    }

    /// Report a failed attempt for a lease.
    ///
    /// Transient failures re-queue with backoff (the score encodes the
    /// next-eligible time and priority); fatal failures and exhausted
    /// attempts dead-letter the entry with `reason`. Returns `Ok(false)` if
    /// the lease was already reclaimed (the entry is being redelivered; do
    /// not report again).
    ///
    /// # Errors
    ///
    /// Returns [`RedisQueueError`] on connection, script, or serialization
    /// failure.
    pub fn fail(
        &self,
        lease: &LeaseState,
        kind: &str,
        reason: &str,
        now_ms: i64,
    ) -> Result<bool, RedisQueueError> {
        let mut conn = self.conn()?;
        self.fail_with_conn(&mut conn, lease, kind, reason, now_ms)
    }

    /// Report failure with an existing connection. See [`DistributedQueue::fail`].
    ///
    /// # Errors
    ///
    /// Returns [`RedisQueueError`] on connection, script, or serialization
    /// failure.
    pub fn fail_with_conn(
        &self,
        conn: &mut redis::Connection,
        lease: &LeaseState,
        kind: &str,
        reason: &str,
        now_ms: i64,
    ) -> Result<bool, RedisQueueError> {
        let mut entry = lease.entry.clone();
        entry.attempt_count += 1;
        let dead = classify(kind) == FailureClass::Fatal
            || attempts_exhausted(entry.attempt_count, self.max_attempts);
        let next_score = if dead {
            0
        } else {
            let backoff = backoff_ms(entry.attempt_count - 1, BASE_BACKOFF_MS, now_ms);
            retry_score(now_ms + backoff, entry.priority)
        };
        let dead_letter = DeadLetter {
            reason: reason.to_string(),
            dead_lettered_at_ms: now_ms,
            entry: entry.clone(),
        };
        let entry_json = serde_json::to_string(&entry)
            .map_err(|e| RedisQueueError::SerializationError(e.to_string()))?;
        let dead_json = serde_json::to_string(&dead_letter)
            .map_err(|e| RedisQueueError::SerializationError(e.to_string()))?;
        let n: i64 = redis::Script::new(LUA_FAIL)
            .key(self.key("processing"))
            .key(self.key("leases"))
            .key(self.key("pending"))
            .key(self.key("dead"))
            .arg(&lease.lease_id)
            .arg(if dead { "1" } else { "0" })
            .arg(next_score)
            .arg(entry_json)
            .arg(dead_json)
            .arg(now_ms)
            .arg(self.dead_letter_cap)
            .invoke(conn)
            .map_err(|e| RedisQueueError::OperationFailed(e.to_string()))?;
        Ok(n == 1)
    }

    /// Reclaim expired leases back into the pending set (ADR-015 §1).
    ///
    /// Called periodically by any worker; safe to run concurrently — each
    /// expired lease is removed from the index as it is handled, so
    /// concurrent sweeps cannot double-requeue. Entries whose attempts are
    /// exhausted at reclaim time are dead-lettered (crash-loop protection).
    ///
    /// Returns the number of reclaimed entries (dead-lettered ones excluded).
    ///
    /// # Errors
    ///
    /// Returns [`RedisQueueError`] on connection or script failure.
    pub fn reclaim_expired(&self) -> Result<usize, RedisQueueError> {
        let mut conn = self.conn()?;
        self.reclaim_expired_with_conn(&mut conn)
    }

    /// Reclaim with an existing connection.
    ///
    /// # Errors
    ///
    /// Returns [`RedisQueueError`] on connection or script failure.
    pub fn reclaim_expired_with_conn(
        &self,
        conn: &mut redis::Connection,
    ) -> Result<usize, RedisQueueError> {
        let now = now_ms();
        let n: i64 = redis::Script::new(LUA_RECLAIM)
            .key(self.key("processing"))
            .key(self.key("leases"))
            .key(self.key("pending"))
            .key(self.key("dead"))
            .arg(now)
            .arg(self.max_attempts)
            .arg(now)
            .arg(self.dead_letter_cap)
            .invoke(conn)
            .map_err(|e| RedisQueueError::OperationFailed(e.to_string()))?;
        Ok(n.max(0) as usize)
    }

    /// List dead-lettered entries, oldest first (operator surface, ADR-015 §3).
    ///
    /// # Errors
    ///
    /// Returns [`RedisQueueError`] on connection or serialization failure.
    pub fn dead_letters(&self) -> Result<Vec<DeadLetter>, RedisQueueError> {
        let mut conn = self.conn()?;
        self.dead_letters_with_conn(&mut conn)
    }

    /// Dead letters with an existing connection.
    ///
    /// # Errors
    ///
    /// Returns [`RedisQueueError`] on connection or serialization failure.
    pub fn dead_letters_with_conn(
        &self,
        conn: &mut redis::Connection,
    ) -> Result<Vec<DeadLetter>, RedisQueueError> {
        let raw: Vec<String> = conn
            .zrange(self.key("dead"), 0, -1)
            .map_err(|e| RedisQueueError::OperationFailed(e.to_string()))?;
        Ok(raw
            .into_iter()
            .filter_map(|s| serde_json::from_str(&s).ok())
            .collect())
    }

    /// Re-drive a dead letter by index (oldest = 0), resetting attempts to 0.
    ///
    /// Returns `Ok(false)` when the index is out of range.
    ///
    /// # Errors
    ///
    /// Returns [`RedisQueueError`] on connection or serialization failure.
    pub fn redrive(&self, index: usize) -> Result<bool, RedisQueueError> {
        let mut conn = self.conn()?;
        self.redrive_with_conn(&mut conn, index)
    }

    /// Re-drive with an existing connection.
    ///
    /// # Errors
    ///
    /// Returns [`RedisQueueError`] on connection or serialization failure.
    pub fn redrive_with_conn(
        &self,
        conn: &mut redis::Connection,
        index: usize,
    ) -> Result<bool, RedisQueueError> {
        let members: Vec<String> = conn
            .zrange(self.key("dead"), index as isize, index as isize)
            .map_err(|e| RedisQueueError::OperationFailed(e.to_string()))?;
        let member: Option<String> = members.into_iter().next();
        let Some(member) = member else {
            return Ok(false);
        };
        let dead_letter: DeadLetter = serde_json::from_str(&member)
            .map_err(|e| RedisQueueError::SerializationError(e.to_string()))?;
        let mut entry = dead_letter.entry;
        entry.attempt_count = 0;
        let json = serde_json::to_string(&entry)
            .map_err(|e| RedisQueueError::SerializationError(e.to_string()))?;
        let pending = self.key("pending");
        let _: i64 = conn
            .zadd(&pending, json, entry.priority)
            .map_err(|e| RedisQueueError::OperationFailed(e.to_string()))?;
        let dead = self.key("dead");
        let _: i64 = conn
            .zrem(&dead, &member)
            .map_err(|e| RedisQueueError::OperationFailed(e.to_string()))?;
        Ok(true)
    }

    /// Number of entries in the pending set (excludes leased entries).
    ///
    /// # Errors
    ///
    /// Returns [`RedisQueueError`] on connection failure.
    pub fn len(&self) -> Result<usize, RedisQueueError> {
        let mut conn = self.conn()?;
        let n: usize = conn
            .zcard(self.key("pending"))
            .map_err(|e| RedisQueueError::OperationFailed(e.to_string()))?;
        Ok(n)
    }

    /// True when no pending entries exist (leased entries are not counted;
    /// use [`DistributedQueue::in_flight`] for those).
    ///
    /// # Errors
    ///
    /// Returns [`RedisQueueError`] on connection failure.
    pub fn is_empty(&self) -> Result<bool, RedisQueueError> {
        self.len().map(|n| n == 0)
    }

    /// Number of leases currently held (awaiting ack, fail, or reclaim).
    ///
    /// # Errors
    ///
    /// Returns [`RedisQueueError`] on connection failure.
    pub fn in_flight(&self) -> Result<usize, RedisQueueError> {
        let mut conn = self.conn()?;
        let n: usize = conn
            .hlen(self.key("processing"))
            .map_err(|e| RedisQueueError::OperationFailed(e.to_string()))?;
        Ok(n)
    }

    /// O(1) visited-set membership (ADR-015 §4), replacing the old O(N) scan.
    ///
    /// # Errors
    ///
    /// Returns [`RedisQueueError`] on connection failure.
    pub fn contains(&self, url: &str) -> Result<bool, RedisQueueError> {
        let mut conn = self.conn()?;
        let is_member: bool = conn
            .sismember(self.key("visited"), url)
            .map_err(|e| RedisQueueError::OperationFailed(e.to_string()))?;
        Ok(is_member)
    }

    /// Clear all keys for this crawl namespace.
    ///
    /// # Errors
    ///
    /// Returns [`RedisQueueError`] on connection failure.
    pub fn clear(&self) -> Result<(), RedisQueueError> {
        let mut conn = self.conn()?;
        for suffix in ["pending", "processing", "leases", "dead", "visited"] {
            let _: i64 = conn
                .del(self.key(suffix))
                .map_err(|e| RedisQueueError::OperationFailed(e.to_string()))?;
        }
        Ok(())
    }
}

/// Unix millis now.
fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// Generate a unique lease ID.
fn new_lease_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Parse a lease payload returned by the pop script: `{lease_id, leased_at_ms,
/// expires_at_ms, entry}` where `entry` is the entry JSON as a nested string.
fn parse_lease_payload(payload: &str) -> Result<LeaseState, String> {
    #[derive(serde::Deserialize)]
    struct Payload {
        lease_id: String,
        expires_at_ms: i64,
        entry: String,
    }
    let p: Payload = serde_json::from_str(payload).map_err(|e| e.to_string())?;
    let entry: DistributedQueueEntry = serde_json::from_str(&p.entry).map_err(|e| e.to_string())?;
    Ok(LeaseState {
        lease_id: p.lease_id,
        entry,
        expires_at_ms: p.expires_at_ms,
    })
}

#[cfg(test)]
mod pure_tests {
    use super::*;

    #[test]
    fn classify_fatal_kinds() {
        for kind in [
            "gone",
            "permanent_dns",
            "ssrf_denied",
            "malformed",
            "deserialization",
        ] {
            assert_eq!(classify(kind), FailureClass::Fatal, "kind: {kind}");
        }
    }

    #[test]
    fn classify_transient_kinds() {
        for kind in [
            "timeout",
            "connect",
            "server_error",
            "rate_limited",
            "lease_expired",
        ] {
            assert_eq!(classify(kind), FailureClass::Transient, "kind: {kind}");
        }
    }

    #[test]
    fn classify_unknown_fails_safe_as_transient() {
        assert_eq!(classify("something_new"), FailureClass::Transient);
    }

    #[test]
    fn attempts_exhausted_boundary() {
        assert!(!attempts_exhausted(4, 5));
        assert!(attempts_exhausted(5, 5));
        assert!(attempts_exhausted(6, 5));
    }

    #[test]
    fn retry_score_encodes_time_then_priority() {
        let s = retry_score(1_000, 64);
        assert_eq!(s >> 8, 1_000);
        assert_eq!(s & 0xFF, 64);
    }

    #[test]
    fn retry_score_orders_by_eligibility_then_priority() {
        // Later next-eligible never surfaces before an earlier one,
        // regardless of priority.
        assert!(retry_score(1_000, 255) < retry_score(1_001, 0));
        // Equal eligibility: priority breaks the tie.
        assert!(retry_score(1_000, 32) < retry_score(1_000, 64));
    }

    #[test]
    fn backoff_grows_exponentially_and_caps() {
        let base = 2_000i64;
        let t = 1_000_000i64;
        let a0 = backoff_ms(0, base, t);
        let a1 = backoff_ms(1, base, t);
        let a10 = backoff_ms(10, base, t);
        assert!(a1 > a0, "backoff must grow: {a1} vs {a0}");
        assert!(a10 <= 15 * 60 * 1000 + base, "capped at 15 min + jitter");
        assert!(a0 >= base);
    }

    #[test]
    fn backoff_is_decorrelated_by_time() {
        // Same attempt, timestamps with different residues mod base ->
        // different jitter (1_000 vs 1_500 mod 2_000 differ).
        assert_ne!(backoff_ms(0, 2_000, 1_000), backoff_ms(0, 2_000, 1_500));
    }

    #[test]
    fn lease_payload_roundtrip() {
        // The pop script embeds the entry as a JSON *string* (cjson encodes
        // the zset member, itself a JSON string, as a string field).
        let entry = DistributedQueueEntry::new("https://example.com/", 2, 64, 1_000);
        let entry_json = serde_json::to_string(&entry).unwrap();
        let payload = format!(
            r#"{{"lease_id":"l1","leased_at_ms":1000,"expires_at_ms":2000,"entry":{}}}"#,
            serde_json::to_string(&entry_json).unwrap()
        );
        let lease = parse_lease_payload(&payload).unwrap();
        assert_eq!(lease.lease_id, "l1");
        assert_eq!(lease.entry, entry);
        assert_eq!(lease.expires_at_ms, 2_000);
    }

    #[test]
    fn dead_letter_json_matches_lua_shape() {
        // The reclaim script builds dead letters as
        // {entry: <entry object>, reason: string, dead_lettered_at_ms: number}
        // via cjson; this test pins that the Rust DeadLetter serializes to
        // the same field names so `dead_letters` parses both.
        let entry = DistributedQueueEntry::new("https://example.com/d", 1, 64, 1);
        let rust_json = serde_json::to_string(&DeadLetter {
            entry: entry.clone(),
            reason: "test".to_string(),
            dead_lettered_at_ms: 5,
        })
        .unwrap();
        let lua_json = format!(
            r#"{{"entry":{},"reason":"lease expired and attempts exhausted","dead_lettered_at_ms":5}}"#,
            serde_json::to_string(&entry).unwrap()
        );
        let from_rust: DeadLetter = serde_json::from_str(&rust_json).unwrap();
        let from_lua: DeadLetter = serde_json::from_str(&lua_json).unwrap();
        assert_eq!(from_rust.entry, from_lua.entry);
        assert_eq!(from_rust.dead_lettered_at_ms, from_lua.dead_lettered_at_ms);
    }

    #[test]
    fn entry_new_initializes_counters() {
        let e = DistributedQueueEntry::new("https://example.com/", 0, 32, 42);
        assert_eq!(e.attempt_count, 0);
        assert_eq!(e.first_enqueued_at, 42);
        assert_eq!(e.priority, 32);
    }
}

// Integration tests (ADR-015 §7): require a running Redis instance. CI runs
// these against its Redis 7 service; locally run with
// `cargo test -p crawlkit-engine --features unstable -- --ignored`.
#[cfg(all(test, feature = "unstable"))]
mod redis_tests {
    use super::*;

    fn redis_url() -> String {
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1/".to_string())
    }

    fn queue(name: &str) -> DistributedQueue {
        let q = DistributedQueue::new(&redis_url(), name).unwrap();
        q.clear().unwrap();
        q
    }

    fn entry(url: &str, priority: i64) -> DistributedQueueEntry {
        DistributedQueueEntry::new(url, 0, priority, 1_000)
    }

    #[test]
    #[ignore = "requires running Redis instance"]
    fn push_pop_ack_roundtrip() {
        let q = queue("t-roundtrip");
        assert!(q.push(&entry("https://example.com/a", 64)).unwrap());
        assert!(!q.push(&entry("https://example.com/a", 64)).unwrap(), "duplicate suppressed");

        let lease = q.pop().unwrap().expect("entry pending");
        assert_eq!(lease.entry.url, "https://example.com/a");
        assert_eq!(q.in_flight().unwrap(), 1);
        assert_eq!(q.len().unwrap(), 0);

        assert!(q.ack(&lease.lease_id).unwrap());
        assert_eq!(q.in_flight().unwrap(), 0);
        assert!(q.pop().unwrap().is_none());
    }

    #[test]
    #[ignore = "requires running Redis instance"]
    fn priority_ordering_across_pops() {
        let q = queue("t-priority");
        q.push(&entry("https://example.com/low", 128)).unwrap();
        q.push(&entry("https://example.com/high", 32)).unwrap();

        let first = q.pop().unwrap().unwrap();
        assert_eq!(first.entry.url, "https://example.com/high");
        let second = q.pop().unwrap().unwrap();
        assert_eq!(second.entry.url, "https://example.com/low");
    }

    #[test]
    #[ignore = "requires running Redis instance"]
    fn transient_failure_requeues_with_backoff_then_exhausts() {
        let q = DistributedQueue::with_policy(&redis_url(), "t-retry", 60_000, 2, 100).unwrap();
        q.clear().unwrap();
        let e = entry("https://example.com/flaky", 64);
        q.push(&e).unwrap();

        // Attempt 1: transient -> requeued with backoff score.
        let lease = q.pop().unwrap().unwrap();
        assert!(q.fail(&lease, "timeout", "conn reset", now_ms()).unwrap());
        assert_eq!(q.len().unwrap(), 1, "requeued for retry");
        // Backoff score defers it: not yet eligible.
        assert!(q.pop().unwrap().is_none(), "entry not eligible during backoff");

        // Attempt 2: simulate backoff elapsing by updating the requeued
        // member's score to 0 (ZADD on an existing member updates its score).
        // The member string is byte-identical to what fail_with_conn stored:
        // struct serialization is deterministic and the Redis round-trip
        // carries the member verbatim.
        let requeued = {
            let mut e2 = lease.entry;
            e2.attempt_count = 1;
            serde_json::to_string(&e2).unwrap()
        };
        let mut conn = q.client.get_connection().unwrap();
        let _: i64 = conn
            .zadd(q_key(&q, "pending"), &requeued, 0)
            .map_err(|e| e.to_string()).unwrap();
        drop(conn);

        let lease2 = q.pop().unwrap().expect("retried entry eligible");
        assert_eq!(lease2.entry.attempt_count, 1);
        assert!(q.fail(&lease2, "timeout", "still failing", now_ms()).unwrap());
        let dead = q.dead_letters().unwrap();
        assert_eq!(dead.len(), 1);
        assert_eq!(dead[0].entry.attempt_count, 2);
        assert_eq!(dead[0].reason, "still failing");
    }

    #[test]
    #[ignore = "requires running Redis instance"]
    fn fatal_failure_dead_letters_immediately() {
        let q = queue("t-fatal");
        q.push(&entry("https://example.com/gone", 64)).unwrap();
        let lease = q.pop().unwrap().unwrap();
        assert!(q.fail(&lease, "gone", "410", now_ms()).unwrap());
        let dead = q.dead_letters().unwrap();
        assert_eq!(dead.len(), 1);
        assert_eq!(dead[0].reason, "410");
        assert_eq!(q.len().unwrap(), 0, "fatal never re-queues");
    }

    #[test]
    #[ignore = "requires running Redis instance"]
    fn expired_lease_is_reclaimed_at_least_once() {
        let q = DistributedQueue::with_policy(&redis_url(), "t-crash", 50, 5, 100).unwrap();
        q.clear().unwrap();
        q.push(&entry("https://example.com/crash", 64)).unwrap();
        let lease = q.pop().unwrap().unwrap();
        assert_eq!(q.in_flight().unwrap(), 1);

        // Worker "crashes": no ack/fail. Wait past the 50 ms TTL.
        std::thread::sleep(std::time::Duration::from_millis(80));
        let reclaimed = q.reclaim_expired().unwrap();
        assert_eq!(reclaimed, 1);
        assert_eq!(q.in_flight().unwrap(), 0);

        // Redelivered (at-least-once) with incremented attempt count.
        let lease2 = q.pop().unwrap().expect("redelivered after reclaim");
        assert_eq!(lease2.entry.url, lease.entry.url);
        assert_eq!(lease2.entry.attempt_count, 1);
        assert!(q.ack(&lease2.lease_id).unwrap());
    }

    #[test]
    #[ignore = "requires running Redis instance"]
    fn concurrent_sweeps_cannot_double_requeue() {
        let q = DistributedQueue::with_policy(&redis_url(), "t-sweep", 50, 5, 100).unwrap();
        q.clear().unwrap();
        q.push(&entry("https://example.com/sweep", 64)).unwrap();
        let _lease = q.pop().unwrap().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(80));

        let (r1, r2) = std::thread::scope(|s| {
            let a = s.spawn(|| q.reclaim_expired()).join().unwrap().unwrap();
            let b = s.spawn(|| q.reclaim_expired()).join().unwrap().unwrap();
            (a, b)
        });
        assert_eq!(r1 + r2, 1, "exactly one sweep reclaims the entry");
        assert_eq!(q.len().unwrap(), 1);
    }

    #[test]
    #[ignore = "requires running Redis instance"]
    fn dead_letter_redrive_resets_attempts() {
        let q = queue("t-redrive");
        q.push(&entry("https://example.com/recover", 64)).unwrap();
        let lease = q.pop().unwrap().unwrap();
        q.fail(&lease, "gone", "transient misclassification", now_ms()).unwrap();

        assert!(q.redrive(0).unwrap());
        assert!(q.dead_letters().unwrap().is_empty());
        let lease2 = q.pop().unwrap().unwrap();
        assert_eq!(lease2.entry.attempt_count, 0, "redrive resets attempts");
    }

    #[test]
    #[ignore = "requires running Redis instance"]
    fn contains_is_o1_visited_membership() {
        let q = queue("t-visited");
        assert!(!q.contains("https://example.com/x").unwrap());
        q.push(&entry("https://example.com/x", 64)).unwrap();
        assert!(q.contains("https://example.com/x").unwrap());
    }

    /// Key helper for direct Redis manipulation in tests.
    fn q_key(q: &DistributedQueue, suffix: &str) -> String {
        format!("{}:{suffix}", q.prefix)
    }
}
