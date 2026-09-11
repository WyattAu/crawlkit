//! Redis-backed global per-target politeness budget (ADR-015 §6).
//!
//! Implements the shared-state primitive the hosted scanner's GA gate
//! depends on (ADR-012): a sliding-window request budget **per target
//! across all users**. Per-IP rate limits alone permit distributed abuse
//! of one victim; this budget is keyed by target host in shared state, so
//! N replicas enforcing it locally still bound the global request rate
//! against any single target.
//!
//! The check-and-consume step is a single Lua script — atomic, so two
//! replicas racing for the last budget slot cannot both win.
//!
//! # Testing
//!
//! Key normalization and policy constants are tested without Redis; the
//! Redis-backed behavior (deny at limit, window expiry restores budget,
//! per-target isolation) is an `#[ignore]`-gated integration test that CI
//! runs against its Redis 7 service.

use redis::Commands;
use thiserror::Error;

/// Errors from politeness budget operations.
#[derive(Debug, Error)]
pub enum PolitenessError {
    /// Redis connection failed.
    #[error("redis connection failed: {0}")]
    ConnectionFailed(String),

    /// Redis operation failed.
    #[error("redis operation failed: {0}")]
    OperationFailed(String),
}

/// Scanner budget window: 10 minutes.
pub const SCANNER_WINDOW_MS: i64 = 10 * 60 * 1000;

/// Scanner budget limit: 30 requests per target per window.
///
/// Per ADR-012 as amended: the floor is not arbitrary — a full scan needs
/// robots (1) + pages (≤25) requests within the window, so the global
/// budget must exceed that maximum legitimate scan (30 ≥ 26).
pub const SCANNER_LIMIT: i64 = 30;

/// Lua: prune the window, deny if at limit, else consume a slot — atomically.
///
/// KEYS: 1 = target budget zset
/// ARGV: 1 = now_ms, 2 = window_ms, 3 = limit, 4 = request_id
///
/// Returns 1 when the slot was granted, 0 when the target is at budget.
const LUA_CONSUME: &str = r#"
local key = KEYS[1]
local now_ms = tonumber(ARGV[1])
local window_ms = tonumber(ARGV[2])
local limit = tonumber(ARGV[3])
local req_id = ARGV[4]

redis.call('ZREMRANGEBYSCORE', key, 0, now_ms - window_ms)
if redis.call('ZCARD', key) >= limit then
  return 0
end
redis.call('ZADD', key, now_ms, req_id)
redis.call('PEXPIRE', key, window_ms)
return 1
"#;

/// Redis-backed per-target sliding-window budget.
///
/// One instance per process; all instances pointing at the same Redis and
/// key prefix share budgets globally (that is the point).
pub struct PolitenessBudget {
    client: redis::Client,
    key_prefix: String,
    window_ms: i64,
    limit: i64,
}

impl PolitenessBudget {
    /// Create a budget with the scanner defaults (30 requests / 10 min per
    /// target, per ADR-012 as amended and ADR-015 §6).
    ///
    /// # Errors
    ///
    /// Returns [`PolitenessError::ConnectionFailed`] if the client cannot be
    /// constructed from `redis_url`.
    pub fn new(redis_url: &str) -> Result<Self, PolitenessError> {
        Self::with_policy(redis_url, SCANNER_WINDOW_MS, SCANNER_LIMIT)
    }

    /// Create a budget with explicit window and limit.
    ///
    /// # Errors
    ///
    /// Returns [`PolitenessError::ConnectionFailed`] if the client cannot be
    /// constructed from `redis_url`.
    pub fn with_policy(
        redis_url: &str,
        window_ms: i64,
        limit: i64,
    ) -> Result<Self, PolitenessError> {
        let client = redis::Client::open(redis_url)
            .map_err(|e| PolitenessError::ConnectionFailed(e.to_string()))?;
        Ok(Self {
            client,
            key_prefix: "crawlkit:politeness".to_string(),
            window_ms,
            limit,
        })
    }

    /// Normalize a target host for budget keying: lowercase, trailing dot
    /// stripped. `Example.COM.` and `example.com` share a budget.
    pub fn normalize_target(target: &str) -> String {
        target.trim().trim_end_matches('.').to_ascii_lowercase()
    }

    fn key_for(&self, target: &str) -> String {
        format!("{}:{}", self.key_prefix, Self::normalize_target(target))
    }

    /// Try to consume one request slot against `target`.
    ///
    /// Returns `Ok(true)` when granted, `Ok(false)` when the target is at
    /// budget for the current window. Atomic across all replicas sharing
    /// this Redis.
    ///
    /// # Errors
    ///
    /// Returns [`PolitenessError`] on connection or script failure.
    pub fn check_and_consume(&self, target: &str) -> Result<bool, PolitenessError> {
        let mut conn = self
            .client
            .get_connection()
            .map_err(|e| PolitenessError::ConnectionFailed(e.to_string()))?;
        self.check_and_consume_with_conn(&mut conn, target)
    }

    /// Consume with an existing connection.
    ///
    /// # Errors
    ///
    /// Returns [`PolitenessError`] on connection or script failure.
    pub fn check_and_consume_with_conn(
        &self,
        conn: &mut redis::Connection,
        target: &str,
    ) -> Result<bool, PolitenessError> {
        let granted: i64 = redis::Script::new(LUA_CONSUME)
            .key(self.key_for(target))
            .arg(now_ms())
            .arg(self.window_ms)
            .arg(self.limit)
            .arg(uuid::Uuid::new_v4().to_string())
            .invoke(conn)
            .map_err(|e| PolitenessError::OperationFailed(e.to_string()))?;
        Ok(granted == 1)
    }

    /// Remaining budget for `target` in the current window (observability).
    ///
    /// # Errors
    ///
    /// Returns [`PolitenessError`] on connection or script failure.
    pub fn remaining(&self, target: &str) -> Result<i64, PolitenessError> {
        let mut conn = self
            .client
            .get_connection()
            .map_err(|e| PolitenessError::ConnectionFailed(e.to_string()))?;
        self.remaining_with_conn(&mut conn, target)
    }

    /// Remaining with an existing connection.
    ///
    /// # Errors
    ///
    /// Returns [`PolitenessError`] on connection or script failure.
    pub fn remaining_with_conn(
        &self,
        conn: &mut redis::Connection,
        target: &str,
    ) -> Result<i64, PolitenessError> {
        let key = self.key_for(target);
        let now = now_ms();
        let _: () = conn
            .zrembyscore(&key, 0, now - self.window_ms)
            .map_err(|e| PolitenessError::OperationFailed(e.to_string()))?;
        let used: usize = conn
            .zcard(&key)
            .map_err(|e| PolitenessError::OperationFailed(e.to_string()))?;
        Ok((self.limit - used as i64).max(0))
    }
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

#[cfg(test)]
mod pure_tests {
    use super::*;

    #[test]
    fn normalizes_target_hosts() {
        assert_eq!(
            PolitenessBudget::normalize_target("Example.COM."),
            PolitenessBudget::normalize_target("example.com")
        );
        assert_eq!(PolitenessBudget::normalize_target(" EXAMPLE.com "), "example.com");
        assert_ne!(
            PolitenessBudget::normalize_target("example.com"),
            PolitenessBudget::normalize_target("example.org")
        );
    }

    #[test]
    fn scanner_constants_match_adr012_floor() {
        // ADR-012 invariant: budget >= robots (1) + max pages (25).
        const { assert!(SCANNER_LIMIT >= 26); }
        assert_eq!(SCANNER_WINDOW_MS, 10 * 60 * 1000);
    }
}

#[cfg(all(test, feature = "unstable"))]
mod redis_tests {
    use super::*;

    fn redis_url() -> String {
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1/".to_string())
    }

    #[test]
    #[ignore = "requires running Redis instance"]
    fn grants_until_limit_then_denies() {
        let budget = PolitenessBudget::with_policy(&redis_url(), 60_000, 3).unwrap();
        let target = format!("test-{}-limit.example", uuid::Uuid::new_v4());
        assert!(budget.check_and_consume(&target).unwrap());
        assert!(budget.check_and_consume(&target).unwrap());
        assert!(budget.check_and_consume(&target).unwrap());
        assert!(!budget.check_and_consume(&target).unwrap(), "at limit");
        assert_eq!(budget.remaining(&target).unwrap(), 0);
    }

    #[test]
    #[ignore = "requires running Redis instance"]
    fn window_expiry_restores_budget() {
        let budget = PolitenessBudget::with_policy(&redis_url(), 80, 1).unwrap();
        let target = format!("test-{}-window.example", uuid::Uuid::new_v4());
        assert!(budget.check_and_consume(&target).unwrap());
        assert!(!budget.check_and_consume(&target).unwrap());
        std::thread::sleep(std::time::Duration::from_millis(120));
        assert!(budget.check_and_consume(&target).unwrap(), "window expired");
    }

    #[test]
    #[ignore = "requires running Redis instance"]
    fn budgets_are_isolated_per_target() {
        let budget = PolitenessBudget::with_policy(&redis_url(), 60_000, 1).unwrap();
        let a = format!("test-{}-a.example", uuid::Uuid::new_v4());
        let b = format!("test-{}-b.example", uuid::Uuid::new_v4());
        assert!(budget.check_and_consume(&a).unwrap());
        assert!(!budget.check_and_consume(&a).unwrap());
        assert!(budget.check_and_consume(&b).unwrap(), "other target unaffected");
    }

    #[test]
    #[ignore = "requires running Redis instance"]
    fn normalized_targets_share_budget() {
        let budget = PolitenessBudget::with_policy(&redis_url(), 60_000, 1).unwrap();
        let base = uuid::Uuid::new_v4();
        let a = format!("shared-{base}.example");
        let b = format!("SHARED-{base}.example.");
        assert!(budget.check_and_consume(&a).unwrap());
        assert!(!budget.check_and_consume(&b).unwrap(), "case/trailing-dot equal");
    }
}
