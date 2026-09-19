//! Render budgets (6.0.0-alpha.2 "Render Budgets"): per-crawl and per-page
//! ceilings on Playwright rendering, with explicit degradation — the same
//! posture as the scanner's daily budget (ADR-012) and the warehouse
//! schema contract (ADR-016): a budget is a *cost lever*, and exceeding it
//! degrades explicitly, never silently.
//!
//! Two independent ceilings:
//!
//! 1. **Per-crawl render quota** ([`RenderBudget::max_renders_per_crawl`]) —
//!    a hard cap on how many pages one crawl may render. Exhaustion means
//!    remaining pages fall back to static analysis and each emits a
//!    `RENDER001` degradation finding.
//! 2. **Per-page render budget** ([`RenderBudget::per_page_timeout`]) — the
//!    wall-clock ceiling for a single render. Exhaustion (or any render
//!    failure) emits `RENDER002` so a degraded page is visible in results,
//!    not a silent static fallback.
//!
//! Rendering telemetry rides the engine's atomic [`Metrics`]
//! (`renders_started` / `renders_completed` / `renders_quota_exhausted` /
//! `renders_budget_exhausted`), matching the zero-allocation hot-path
//! pattern of every other counter.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Outcome of asking the budget for permission to render one page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderGrant {
    /// Rendering allowed.
    Allowed,
    /// The per-crawl quota is exhausted: do not render; the caller must
    /// record a `RENDER001` degradation finding for this page.
    QuotaExhausted,
}

/// Ceilings on JavaScript rendering for one crawl. `None` fields mean
/// "unbounded" (the historical behavior), so existing configurations are
/// unaffected until an operator sets a budget.
#[derive(Debug, Clone)]
pub struct RenderBudget {
    /// Maximum pages one crawl may render. `None` = unbounded.
    pub max_renders_per_crawl: Option<u64>,
    /// Wall-clock ceiling for one render. `None` = engine default (30 s
    /// timeout in the pipeline).
    pub per_page_timeout: Option<Duration>,
    /// Renders attempted so far in this crawl (shared across worker tasks).
    used: Arc<AtomicU64>,
}

impl RenderBudget {
    /// Unbounded budget — identical to the pre-budget behavior.
    #[must_use]
    pub fn unbounded() -> Self {
        Self {
            max_renders_per_crawl: None,
            per_page_timeout: None,
            used: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Budget with a per-crawl render quota and optional per-page ceiling.
    #[must_use]
    pub fn new(max_renders_per_crawl: Option<u64>, per_page_timeout: Option<Duration>) -> Self {
        Self {
            max_renders_per_crawl,
            per_page_timeout,
            used: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Ask for one render slot. Granted slots are counted immediately
    /// (atomic CAS-free increment): under a concurrent crawl this
    /// over-counts slightly rather than over-renders — the safe bias for a
    /// cost lever.
    pub fn acquire(&self) -> RenderGrant {
        match self.max_renders_per_crawl {
            None => RenderGrant::Allowed,
            Some(quota) => {
                let prev = self.used.fetch_add(1, Ordering::Relaxed);
                if prev >= quota {
                    RenderGrant::QuotaExhausted
                } else {
                    RenderGrant::Allowed
                }
            }
        }
    }

    /// The effective per-page timeout, or the engine default when unset.
    #[must_use]
    pub fn effective_timeout(&self) -> Duration {
        self.per_page_timeout.unwrap_or(Duration::from_secs(30))
    }

    /// Renders granted so far (telemetry / tests).
    #[must_use]
    pub fn used(&self) -> u64 {
        self.used.load(Ordering::Relaxed)
    }
}

impl Default for RenderBudget {
    fn default() -> Self {
        Self::unbounded()
    }
}

/// Telemetry counters for rendering, embedded in the engine's [`Metrics`].
/// Atomic + `Relaxed` like every other counter (zero-allocation hot path).
#[derive(Debug, Default)]
pub struct RenderMetrics {
    /// Renders started (granted and dispatched to the renderer).
    pub renders_started: AtomicU64,
    /// Renders that produced a rendered page.
    pub renders_completed: AtomicU64,
    /// Pages denied by the per-crawl quota (RENDER001).
    pub renders_quota_exhausted: AtomicU64,
    /// Renders that hit the per-page budget or otherwise failed (RENDER002).
    pub renders_budget_exhausted: AtomicU64,
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn unbounded_grants_everything() {
        let b = RenderBudget::unbounded();
        for _ in 0..1000 {
            assert_eq!(b.acquire(), RenderGrant::Allowed);
        }
        assert_eq!(b.effective_timeout(), Duration::from_secs(30));
    }

    #[test]
    fn quota_exhausts_explicitly() {
        let b = RenderBudget::new(Some(2), None);
        assert_eq!(b.acquire(), RenderGrant::Allowed);
        assert_eq!(b.acquire(), RenderGrant::Allowed);
        assert_eq!(b.acquire(), RenderGrant::QuotaExhausted);
        assert_eq!(b.acquire(), RenderGrant::QuotaExhausted);
        assert_eq!(b.used(), 4, "denied attempts are counted for telemetry");
    }

    #[test]
    fn zero_quota_denies_immediately() {
        let b = RenderBudget::new(Some(0), Some(Duration::from_secs(5)));
        assert_eq!(b.acquire(), RenderGrant::QuotaExhausted);
        assert_eq!(b.effective_timeout(), Duration::from_secs(5));
    }

    #[test]
    fn quota_is_shared_across_clones() {
        // Clones model the multi-worker crawl: one shared counter.
        let b = Arc::new(RenderBudget::new(Some(1), None));
        let b2 = b.clone();
        assert_eq!(b.acquire(), RenderGrant::Allowed);
        assert_eq!(b2.acquire(), RenderGrant::QuotaExhausted);
    }
}
