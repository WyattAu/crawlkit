//! Abuse-surface limits (SCANNER_RUNBOOK §6 GA checklist items 2 and 4).
//!
//! Two independent ceilings sit in front of the per-target politeness
//! budget (ADR-012 §2), which only bounds requests against one *target*:
//!
//! 1. [`GlobalDailyBudget`] — a daily ceiling on scans accepted service-wide
//!    (the runbook's "daily global scan budget"). A cost lever: exceeding it
//!    degrades to explicit refusals, never silent truncation. Counted in
//!    Redis (shared across replicas) when the shared backend is configured,
//!    in-process otherwise — mirroring the ADR-015 §A1 posture split.
//! 2. [`IpRateLimiter`] — a fixed-window per-client-IP submission limit (the
//!    runbook's "per-IP rate limit ... implemented and tested"). Bounds how
//!    fast one actor can spend the global budget; in-process by design (the
//!    proxy terminates TLS per replica; for multi-replica deployments behind
//!    a shared proxy the proxy's own rate limit covers the aggregate).
//!
//! Both return typed refusals that the API surfaces as 429 with Retry-After,
//! consistent with the budget-exhausted contract (explicit degradation).

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

/// One UTC calendar day in seconds.
const DAY_SECS: u64 = 24 * 60 * 60;

/// Which backend counts the daily global budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DailyBudgetBackend {
    /// Counted in-process (single-replica posture). Resets on restart —
    /// the documented weaker guarantee of the ADR-015 §A1 split.
    InMemory,
    /// Counted in Redis (multi-replica posture); fail-closed on outage.
    Redis,
}

/// Errors returned by the abuse-surface limits.
#[derive(Debug, thiserror::Error)]
pub enum AbuseLimitError {
    /// The service-wide daily scan budget is spent.
    #[error("daily scan budget exhausted; resets in {retry_after_secs}s")]
    DailyBudgetExhausted { retry_after_secs: u64 },
    /// This client IP is submitting too fast.
    #[error("submission rate exceeded; retry after {retry_after_secs}s")]
    IpRateLimited { retry_after_secs: u64 },
}

/// A daily global scan-budget counter.
///
/// The limit is a ceiling: deployment configuration may lower it, never
/// raise it. The in-process and Redis variants have distinct consume
/// methods (`consume_local` / `consume_shared`) so neither can be invoked
/// with the wrong connection contract.
pub struct GlobalDailyBudget {
    backend: DailyBudgetBackend,
    limit: u64,
    /// day_key -> count. InMemory backend only.
    local: Mutex<HashMap<u64, u64>>,
    /// Redis key root (e.g. `crawlkit:scanner:daily`). Shared backend only.
    #[cfg(feature = "shared-budget")]
    redis_key: String,
}

/// Seconds from `now_ms` to the next UTC midnight (min 1).
fn secs_to_midnight(now_ms: u64) -> u64 {
    let day = now_ms / 1000 / DAY_SECS;
    let midnight_ms = (day + 1) * DAY_SECS * 1000;
    (midnight_ms.saturating_sub(now_ms) / 1000).max(1)
}

/// The UTC-day bucket key for `now_ms`.
fn day_bucket(now_ms: u64) -> u64 {
    now_ms / 1000 / DAY_SECS
}

impl GlobalDailyBudget {
    /// Create an in-process daily budget.
    pub fn in_memory(limit: u64) -> Self {
        Self {
            backend: DailyBudgetBackend::InMemory,
            limit,
            local: Mutex::new(HashMap::new()),
            #[cfg(feature = "shared-budget")]
            redis_key: String::new(),
        }
    }

    /// Create a Redis-backed daily budget sharing `redis_key` across
    /// replicas.
    #[cfg(feature = "shared-budget")]
    pub fn shared(redis_key: impl Into<String>, limit: u64) -> Self {
        Self {
            backend: DailyBudgetBackend::Redis,
            limit,
            local: Mutex::new(HashMap::new()),
            redis_key: redis_key.into(),
        }
    }

    /// The backend this counter uses.
    pub fn backend(&self) -> DailyBudgetBackend {
        self.backend
    }

    /// Consume one slot of today's budget (in-process counting).
    ///
    /// Only valid on an `in_memory` counter; panics otherwise (a shared
    /// counter called without a connection is a programming error, not a
    /// runtime condition).
    ///
    /// # Errors
    ///
    /// Returns [`AbuseLimitError::DailyBudgetExhausted`] when today's count
    /// is at `limit`, with the seconds until UTC midnight.
    pub fn consume_local(&self, now_ms: u64) -> Result<(), AbuseLimitError> {
        assert_eq!(
            self.backend,
            DailyBudgetBackend::InMemory,
            "consume_local called on a shared (Redis) daily budget"
        );
        let day = day_bucket(now_ms);
        let retry_after_secs = secs_to_midnight(now_ms);
        // Lock with poison recovery: a panicked holder leaves intact
        // per-day counts, which are safe to continue from.
        let mut map = self
            .local
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        // Opportunistic cleanup of yesterday's buckets.
        map.retain(|d, _| *d >= day);
        let count = map.entry(day).or_insert(0);
        if *count >= self.limit {
            return Err(AbuseLimitError::DailyBudgetExhausted { retry_after_secs });
        }
        *count += 1;
        Ok(())
    }

    /// Consume one slot of today's budget (Redis INCR counting).
    ///
    /// Atomic per key; the ceiling holds under concurrent submitters.
    /// Fail-closed by design: a Redis error refuses the scan with a 60s
    /// retry window rather than admitting an uncounted scan (ADR-015 §A1
    /// multi-replica posture — shared state IS the anti-abuse guarantee).
    ///
    /// Only valid on a `shared` counter (asserted).
    ///
    /// # Errors
    ///
    /// Returns [`AbuseLimitError::DailyBudgetExhausted`] when today's count
    /// exceeds `limit`, or when the backend errors (fail-closed).
    #[cfg(feature = "shared-budget")]
    pub async fn consume_shared(
        &self,
        conn: &mut redis::aio::ConnectionManager,
        now_ms: u64,
    ) -> Result<(), AbuseLimitError> {
        assert_eq!(
            self.backend,
            DailyBudgetBackend::Redis,
            "consume_shared called on an in-memory daily budget"
        );
        let day = day_bucket(now_ms);
        let key = format!("{}:{day}", self.redis_key);
        let count: Option<u64> = redis::cmd("INCR").arg(&key).query_async(conn).await.ok();
        let Some(count) = count else {
            // Fail closed: no budget record → no scan (mirrors the
            // politeness budget's outage contract).
            return Err(AbuseLimitError::DailyBudgetExhausted {
                retry_after_secs: 60,
            });
        };
        let ttl_secs = secs_to_midnight(now_ms) + 60;
        let _: Result<i64, _> = redis::cmd("EXPIRE")
            .arg(&key)
            .arg(ttl_secs)
            .query_async(conn)
            .await;
        if count > self.limit {
            // Over ceiling: also unwind the over-admitted INCR so the
            // counter reflects admitted scans, not attempts.
            let _: Result<i64, _> = redis::cmd("DECR").arg(&key).query_async(conn).await;
            return Err(AbuseLimitError::DailyBudgetExhausted {
                retry_after_secs: secs_to_midnight(now_ms),
            });
        }
        Ok(())
    }

    /// Today's count (in-process counting). Best-effort.
    pub fn count_today_local(&self, now_ms: u64) -> u64 {
        assert_eq!(
            self.backend,
            DailyBudgetBackend::InMemory,
            "count_today_local called on a shared (Redis) daily budget"
        );
        let day = day_bucket(now_ms);
        self.local
            .lock()
            .map(|m| *m.get(&day).unwrap_or(&0))
            .unwrap_or(0)
    }
}

/// Fixed-window per-client-IP submission limiter (GA checklist item 4).
///
/// Bounded memory: at most [`MAX_TRACKED_IPS`] IPs are tracked; stale
/// entries are evicted first and, failing that, an arbitrary one, so an
/// attacker rotating source IPs cannot grow the map unboundedly (they just
/// lose their own slot's history).
pub struct IpRateLimiter {
    limit: u32,
    window: Duration,
    /// ip -> (window_start_ms, count).
    windows: Mutex<HashMap<String, (u64, u32)>>,
}

/// Maximum IPs tracked at once (see struct docs for the eviction story).
pub const MAX_TRACKED_IPS: usize = 10_000;

impl IpRateLimiter {
    /// Create a limiter allowing `limit` submissions per `window` per IP.
    pub fn new(limit: u32, window: Duration) -> Self {
        Self {
            limit,
            window,
            windows: Mutex::new(HashMap::new()),
        }
    }

    /// Record a submission from `ip` at `now_ms`.
    ///
    /// # Errors
    ///
    /// Returns [`AbuseLimitError::IpRateLimited`] when this IP has already
    /// made `limit` submissions inside the current window.
    pub fn check(&self, ip: &str, now_ms: u64) -> Result<(), AbuseLimitError> {
        let window_ms = self.window.as_millis() as u64;
        let mut map = self
            .windows
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if map.len() >= MAX_TRACKED_IPS {
            let stale: Vec<String> = map
                .iter()
                .filter(|(_, (start, _))| now_ms.saturating_sub(*start) >= window_ms)
                .map(|(k, _)| k.clone())
                .collect();
            if stale.is_empty() {
                if let Some(k) = map.keys().next().cloned() {
                    map.remove(&k);
                }
            } else {
                for k in stale {
                    map.remove(&k);
                }
            }
        }
        let entry = map
            .entry(ip.to_string())
            .and_modify(|(start, count)| {
                if now_ms.saturating_sub(*start) >= window_ms {
                    *start = now_ms;
                    *count = 0;
                }
            })
            .or_insert((now_ms, 0));
        if entry.1 >= self.limit {
            let retry_after_secs =
                (window_ms.saturating_sub(now_ms.saturating_sub(entry.0)) / 1000).max(1);
            return Err(AbuseLimitError::IpRateLimited { retry_after_secs });
        }
        entry.1 += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Arbitrary "now" in millis (2026-ish epoch seconds * 1000).
    const T0: u64 = 1_789_000_000_000;

    #[test]
    fn daily_budget_counts_and_refuses() {
        let budget = GlobalDailyBudget::in_memory(3);
        for _ in 0..3 {
            assert!(budget.consume_local(T0).is_ok());
        }
        let err = budget.consume_local(T0).unwrap_err();
        let AbuseLimitError::DailyBudgetExhausted { retry_after_secs } = err else {
            panic!("wrong error variant");
        };
        assert!(retry_after_secs > 0);
        assert_eq!(budget.count_today_local(T0), 3);
    }

    #[test]
    fn daily_budget_resets_next_day() {
        let budget = GlobalDailyBudget::in_memory(1);
        let day1 = T0 + DAY_SECS * 1000;
        assert!(budget.consume_local(T0).is_ok());
        assert!(budget.consume_local(T0).is_err());
        assert!(
            budget.consume_local(day1).is_ok(),
            "new UTC day resets the budget"
        );
    }

    #[test]
    fn ip_limiter_fixed_window() {
        let limiter = IpRateLimiter::new(2, Duration::from_secs(60));
        assert!(limiter.check("203.0.113.7", T0).is_ok());
        assert!(limiter.check("203.0.113.7", T0 + 1_000).is_ok());
        let err = limiter.check("203.0.113.7", T0 + 2_000).unwrap_err();
        let AbuseLimitError::IpRateLimited { retry_after_secs } = err else {
            panic!("wrong error variant");
        };
        assert!(retry_after_secs > 0);
        // A new window admits the IP again.
        assert!(limiter.check("203.0.113.7", T0 + 61_000).is_ok());
        // Other IPs are independent.
        assert!(limiter.check("198.51.100.9", T0 + 2_000).is_ok());
    }

    #[test]
    fn ip_limiter_memory_is_bounded() {
        let limiter = IpRateLimiter::new(1, Duration::from_secs(60));
        for i in 0..(MAX_TRACKED_IPS as u32 + 50) {
            let ip = format!("10.{i}.0.1");
            let _ = limiter.check(&ip, T0);
        }
        assert!(
            limiter.windows.lock().unwrap().len() <= MAX_TRACKED_IPS,
            "tracked IPs must stay capped"
        );
    }

    #[cfg(feature = "shared-budget")]
    #[tokio::test]
    #[ignore = "requires running Redis instance"]
    async fn shared_daily_budget_counts_across_keys() {
        // Redis-backed evidence for the multi-replica posture: two counters
        // on the same key share the ceiling (what two replicas see).
        let url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1/".to_string());
        let client = redis::Client::open(url).unwrap();
        let mut conn = client.get_connection_manager().await.unwrap();
        let key = format!("crawlkit:scanner:daily:test-{}", std::process::id());
        let budget_a = GlobalDailyBudget::shared(key.clone(), 2);
        let budget_b = GlobalDailyBudget::shared(key.clone(), 2);
        let now = T0;
        assert!(budget_a.consume_shared(&mut conn, now).await.is_ok());
        assert!(budget_b.consume_shared(&mut conn, now).await.is_ok());
        assert!(
            budget_a.consume_shared(&mut conn, now).await.is_err(),
            "third scan must exceed the shared ceiling of 2"
        );
        // Cleanup the test key.
        let day = day_bucket(now);
        let _: Result<i64, _> = redis::cmd("DEL")
            .arg(format!("{key}:{day}"))
            .query_async(&mut conn)
            .await;
    }
}
