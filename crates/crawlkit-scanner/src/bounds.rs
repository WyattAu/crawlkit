//! Hosted free-scan scanner (ADR-012) — bounds and politeness budget.
//!
//! Every constant here is a marketing-facing number (ADR-012 "Compliance"):
//! it appears in user-visible copy, so it must live in code, not docs. The
//! `[capabilities.hosted_scanner]` entry in `docs/capabilities.toml` cites
//! this module as evidence and must be regenerated whenever these change.

use std::time::Duration;

/// Maximum pages scanned per submission (ADR-012 §3; hard ceiling in the
/// engine config, not politeness).
pub const MAX_PAGES: usize = 25;

/// Maximum link depth from the submitted URL.
pub const MAX_DEPTH: usize = 4;

/// Maximum wall-clock time for one scan. At the bound the worker is stopped
/// and the partial result is returned with a truncation notice.
pub const MAX_WALL_CLOCK: Duration = Duration::from_secs(30);

/// Maximum memory per scan worker (documented bound; soft enforcement via
/// page-size limits in the prototype).
pub const MAX_MEMORY_BYTES: u64 = 512 * 1024 * 1024;

/// Maximum response body size per page (bounded slice of the memory budget).
pub const MAX_BODY_BYTES: u64 = 2 * 1024 * 1024;

/// Maximum redirect hops per request (matches the engine limit).
pub const MAX_REDIRECTS: usize = 8;

/// Scan-result retention window (ADR-012 §4). Results are deleted after
/// this period; the token is the only handle and no account linkage exists.
pub const RESULT_RETENTION: Duration = Duration::from_secs(24 * 60 * 60);

/// Sliding-window length for the global per-target politeness budget.
pub const BUDGET_WINDOW: Duration = Duration::from_secs(10 * 60);

/// Requests allowed per target host per [`BUDGET_WINDOW`] (ADR-012 §2).
///
/// Must accommodate at least one full scan: robots fetch (1) + [`MAX_PAGES`]
/// page fetches (25) = 26, plus headroom for concurrent legitimate
/// submitters. Deployment configuration may lower this; it can never be
/// raised. A value below 26 would make the page ceiling unreachable and
/// every scan report a misleading truncation reason.
pub const BUDGET_PAGES_PER_WINDOW: u32 = 30;

/// Minimum entropy (bits) for result tokens (ADR-012 §4).
pub const RESULT_TOKEN_BITS: usize = 128;

/// Errors produced by the politeness budget.
#[derive(Debug, thiserror::Error)]
pub enum BudgetError {
    /// The target's window budget is exhausted; retry after the given delay.
    #[error("target budget exhausted; retry after {retry_after_secs}s")]
    Exhausted {
        retry_after_secs: u64,
    },
}

/// Shared-state politeness budget for one target host (ADR-012 §2).
///
/// The prototype ships an in-process store (`InMemoryBudgetStore`); the ADR
/// requires the production deployment to back this trait with shared state
/// (Redis or the API's queue infrastructure) so the cap holds across
/// replicas. Implementations must be atomic per host: check-and-consume may
/// not race between replicas.
pub trait BudgetStore: Send + Sync {
    /// Consume one slot for `host` inside the sliding window.
    ///
    /// Returns `Ok(())` when a slot was consumed, `Err(BudgetError::Exhausted)`
    /// with the retry delay when the host's window is full.
    fn consume(&self, host: &str, now_ms: u64) -> Result<(), BudgetError>;

    /// Drain expired entries (24h-style housekeeping). Returns entries removed.
    fn sweep(&self, now_ms: u64) -> usize;
}

/// In-process budget store for the prototype and for tests.
///
/// Not replica-safe: two scanner processes each holding this store would
/// double the effective per-target budget. ADR-012 requires a shared backend
/// before any multi-replica deployment.
pub struct InMemoryBudgetStore {
    /// host -> timestamps (ms) of consumed slots inside the window.
    windows: dashmap::DashMap<String, Vec<u64>>,
}

impl InMemoryBudgetStore {
    pub fn new() -> Self {
        Self {
            windows: dashmap::DashMap::new(),
        }
    }
}

impl Default for InMemoryBudgetStore {
    fn default() -> Self {
        Self::new()
    }
}

impl BudgetStore for InMemoryBudgetStore {
    fn consume(&self, host: &str, now_ms: u64) -> Result<(), BudgetError> {
        let window_start = now_ms.saturating_sub(BUDGET_WINDOW.as_millis() as u64);
        let mut entry = self.windows.entry(host.to_string()).or_default();
        entry.retain(|ts| *ts > window_start);
        if entry.len() >= BUDGET_PAGES_PER_WINDOW as usize {
            // Retry when the oldest in-window slot ages out.
            let oldest = entry.iter().copied().min().unwrap_or(now_ms);
            let retry_at = oldest + BUDGET_WINDOW.as_millis() as u64;
            return Err(BudgetError::Exhausted {
                retry_after_secs: retry_at.saturating_sub(now_ms).max(1) / 1000 + 1,
            });
        }
        entry.push(now_ms);
        Ok(())
    }

    fn sweep(&self, now_ms: u64) -> usize {
        let window_start = now_ms.saturating_sub(BUDGET_WINDOW.as_millis() as u64);
        let mut removed = 0usize;
        for mut entry in self.windows.iter_mut() {
            let before = entry.value().len();
            entry.value_mut().retain(|ts| *ts > window_start);
            removed += before - entry.value().len();
        }
        removed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adr_constants_match_documented_values() {
        assert_eq!(MAX_PAGES, 25);
        assert_eq!(MAX_DEPTH, 4);
        assert_eq!(MAX_WALL_CLOCK, Duration::from_secs(30));
        assert_eq!(BUDGET_PAGES_PER_WINDOW, 30);
        assert_eq!(BUDGET_WINDOW, Duration::from_secs(600));
        assert_eq!(RESULT_RETENTION, Duration::from_secs(86_400));
        assert_eq!(MAX_REDIRECTS, 8);
        const _: () = assert!(RESULT_TOKEN_BITS >= 128);
    }

    #[test]
    fn budget_allows_up_to_cap_then_exhausts() {
        let store = InMemoryBudgetStore::new();
        let now: u64 = 1_000_000;
        for i in 0..BUDGET_PAGES_PER_WINDOW {
            assert!(
                store.consume("target.example", now + u64::from(i)).is_ok(),
                "slot {i} within cap must be granted"
            );
        }
        let err = store.consume("target.example", now).unwrap_err();
        let BudgetError::Exhausted { retry_after_secs } = err;
        assert!(retry_after_secs > 0);
    }

    #[test]
    fn budget_window_accommodates_one_full_scan() {
        // ADR-012 invariant: robots + MAX_PAGES must fit inside one window,
        // otherwise the page ceiling is unreachable in production.
        assert!(
            BUDGET_PAGES_PER_WINDOW as usize > MAX_PAGES,
            "budget ({BUDGET_PAGES_PER_WINDOW}) must cover robots + {MAX_PAGES} pages"
        );
    }

    #[test]
    fn budget_windows_are_independent_per_host() {
        let store = InMemoryBudgetStore::new();
        let now: u64 = 2_000_000;
        for i in 0..BUDGET_PAGES_PER_WINDOW {
            let _ = store.consume("a.example", now + u64::from(i));
        }
        assert!(store.consume("a.example", now).is_err());
        // Different host is unaffected.
        assert!(store.consume("b.example", now).is_ok());
    }

    #[test]
    fn budget_slots_expire_after_window() {
        let store = InMemoryBudgetStore::new();
        let now: u64 = 3_000_000;
        for i in 0..BUDGET_PAGES_PER_WINDOW {
            let _ = store.consume("c.example", now + u64::from(i));
        }
        assert!(store.consume("c.example", now).is_err());
        // After the window slides past all consumed slots, capacity returns.
        let later = now + BUDGET_WINDOW.as_millis() as u64 + 1;
        assert!(store.consume("c.example", later).is_ok());
    }

    #[test]
    fn sweep_removes_expired_entries() {
        let store = InMemoryBudgetStore::new();
        let now: u64 = 4_000_000;
        let _ = store.consume("d.example", now);
        let later = now + BUDGET_WINDOW.as_millis() as u64 + 1;
        assert!(store.sweep(later) >= 1);
    }
}
