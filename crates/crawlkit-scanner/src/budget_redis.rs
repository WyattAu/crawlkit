//! Hosted free-scan scanner (ADR-012) — Redis-backed budget store.
//!
//! Multi-replica posture per ADR-015 §A1: the global per-target politeness
//! budget lives in shared state (the engine's `PolitenessBudget`, ADR-015
//! §6), so N replicas enforcing locally still bound the *global* request
//! rate against any single target. Fail-closed by design: a Redis outage
//! surfaces as scan rejection, never as degraded budget enforcement.
//!
//! Feature-gated behind `shared-budget`; the single-replica default remains
//! `InMemoryBudgetStore` per ADR-015 §A1.

use crate::bounds::{BudgetError, BudgetStore, BUDGET_WINDOW};

/// Redis-backed shared-state budget store (ADR-015 §6, §A1 multi-replica).
///
/// Wraps the engine's atomic sliding-window budget. Cloneable; all clones
/// share one multiplexed connection.
#[derive(Clone)]
pub struct RedisBudgetStore {
    inner: crawlkit_engine::politeness::PolitenessBudget,
}

impl RedisBudgetStore {
    /// Create a store against `redis_url` with the ADR-012 scanner limits
    /// (30 requests / 10 min per target).
    ///
    /// # Errors
    ///
    /// Returns the engine error when the Redis client cannot be constructed.
    pub fn new(redis_url: &str) -> Result<Self, crawlkit_engine::politeness::PolitenessError> {
        Ok(Self {
            inner: crawlkit_engine::politeness::PolitenessBudget::new(redis_url)?,
        })
    }

    /// Create a store with explicit window and limit (deployment tuning).
    ///
    /// # Errors
    ///
    /// Returns the engine error when the Redis client cannot be constructed.
    pub fn with_policy(
        redis_url: &str,
        window_ms: i64,
        limit: i64,
    ) -> Result<Self, crawlkit_engine::politeness::PolitenessError> {
        Ok(Self {
            inner: crawlkit_engine::politeness::PolitenessBudget::with_policy(
                redis_url, window_ms, limit,
            )?,
        })
    }
}

#[async_trait::async_trait]
impl BudgetStore for RedisBudgetStore {
    async fn consume(&self, host: &str, _now_ms: u64) -> Result<(), BudgetError> {
        // The engine budget derives its window from server-side timestamps;
        // the caller's now_ms is advisory (single-clock deployments only),
        // so the shared store intentionally ignores it.
        match self.inner.check_and_consume(host).await {
            Ok(true) => Ok(()),
            Ok(false) => {
                // Conservative hint: the engine does not expose oldest-slot
                // age, so the full window is the safe upper bound.
                Err(BudgetError::Exhausted {
                    retry_after_secs: BUDGET_WINDOW.as_secs(),
                })
            }
            Err(e) => {
                // Fail-closed (ADR-015 §A1): a Redis outage must read as
                // budget exhaustion to the scan path — the scan is rejected,
                // budget enforcement is never silently degraded. The log
                // preserves the cause for operators.
                tracing::warn!(
                    target = %host,
                    error = %e,
                    "budget backend unavailable; failing closed"
                );
                Err(BudgetError::Exhausted {
                    retry_after_secs: BUDGET_WINDOW.as_secs(),
                })
            }
        }
    }

    async fn sweep(&self, _now_ms: u64) -> usize {
        // The engine's sliding window prunes expired entries on every
        // check-and-consume; no separate sweep needed.
        0
    }
}
