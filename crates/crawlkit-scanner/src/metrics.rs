//! Scanner operational metrics (SCANNER_RUNBOOK §4 — Phase 5.1 alignment).
//!
//! The runbook names four dashboards as required before GA: scan success
//! rate and p95 latency, rejection rates by cause, per-target budget
//! saturation, and egress bytes/day. This module is the data plane for
//! those dashboards: a process-global counter set, incremented inline at
//! each decision point (zero-cost on the happy path, no allocation), and
//! a `GET /metrics` endpoint that renders a snapshot.
//!
//! Design constraints from the surrounding architecture:
//!
//! - **No external metric crate.** The scanner's posture is a small,
//!   auditable binary; a Prometheus dependency for ten counters is not a
//!   trade this codebase makes (ADR-012's "minimum surface" rule). The
//!   JSON snapshot is scrapeable by anything; a text exposition can be
//!   added at the same source points if a Prometheus server shows up.
//! - **Rejections carry a cause enum, not a string.** The runbook's
//!   paging rule (SSRF-denial spike = attack signal) requires exact
//!   buckets; free-form reasons would fragment under dashboards.
//! - **p95 latency is tracked as a reservoir sample.** Full latency
//!   histories are unbounded state; a bounded sample (4k, split by
//!   posture) approximates p95 within dashboard noise at scanner scale.
//! - **Egress counts application-level body bytes** (what `read_body`
//!   actually pulled off the wire, truncated at the body cap), not NIC
//!   bytes — it is the cost driver the operator controls.

use rand::Rng;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

/// Why a submission was refused before a token was issued (or before a
/// job was accepted). Buckets mirror the runbook §4 dashboard rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectionCause {
    /// Static target validation failed (scheme/host/port rules).
    MalformedInput,
    /// The resolver filtered every answer for the host (private/reserved
    /// address — the ADR-012 SSRF boundary). Page on a spike.
    SsrfDenied,
    /// Per-IP fixed-window limiter refused the submission.
    IpRateLimited,
    /// Service-wide daily budget exhausted (the cost lever).
    DailyBudget,
    /// Per-target politeness window exhausted.
    TargetBudget,
    /// No worker slot free (inline posture) or enqueue failed closed
    /// (queue posture).
    QueueBusy,
}

impl RejectionCause {
    /// Stable key for JSON output; also the dashboard dimension.
    pub fn as_str(self) -> &'static str {
        match self {
            RejectionCause::MalformedInput => "malformed_input",
            RejectionCause::SsrfDenied => "ssrf_denied",
            RejectionCause::IpRateLimited => "ip_rate_limited",
            RejectionCause::DailyBudget => "daily_budget",
            RejectionCause::TargetBudget => "target_budget",
            RejectionCause::QueueBusy => "queue_busy",
        }
    }
}

/// Reservoir sample for p95 latency estimation. Uniform-random retention
/// over a bounded `Vec`; `size` bounds memory, and the sample stays exact
/// until full, then swaps uniformly (Vitter's algorithm R).
#[derive(Debug)]
struct Reservoir {
    samples: Vec<u64>,
    size: usize,
    seen: u64,
}

impl Reservoir {
    fn new(size: usize) -> Self {
        Self {
            samples: Vec::with_capacity(size),
            size,
            seen: 0,
        }
    }

    fn record(&mut self, value_ms: u64) {
        self.seen = self.seen.saturating_add(1);
        if self.samples.len() < self.size {
            self.samples.push(value_ms);
            return;
        }
        // Vitter's algorithm R: keep the i-th item with probability size/seen.
        let i = rand::thread_rng().gen_range(0..self.seen) as usize;
        if i < self.size {
            self.samples[i] = value_ms;
        }
    }

    fn percentile(&self, p: usize) -> Option<u64> {
        if self.samples.is_empty() {
            return None;
        }
        let mut sorted = self.samples.clone();
        sorted.sort_unstable();
        let idx = (sorted.len() * p / 100)
            .saturating_sub(1)
            .min(sorted.len() - 1);
        Some(sorted[idx])
    }

    fn len(&self) -> usize {
        self.samples.len()
    }
}

/// All counters the scanner increments. Cheap to clone (shared atomics).
#[derive(Debug)]
pub struct Metrics {
    // --- submission outcomes (server-side, both postures) ---
    accepted: AtomicU64,
    rejected_by_cause: [AtomicU64; 6],

    // --- scan outcomes (worker-side; queue posture aggregates the pool) ---
    completed: AtomicU64,
    robots_blocked: AtomicU64,
    scan_rejected: AtomicU64,
    budget_exhausted_outcomes: AtomicU64,

    // --- egress (the cost lever) ---
    egress_bytes: AtomicU64,

    // --- latency samples, split by posture so queueing delay (which
    // includes wait-for-worker time) never pollutes the fetch-path p95 ---
    inline_latencies: Mutex<Reservoir>,
    queue_latencies: Mutex<Reservoir>,
}

const RESERVOIR_SIZE: usize = 4096;

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            accepted: AtomicU64::new(0),
            rejected_by_cause: Default::default(),
            completed: AtomicU64::new(0),
            robots_blocked: AtomicU64::new(0),
            scan_rejected: AtomicU64::new(0),
            budget_exhausted_outcomes: AtomicU64::new(0),
            egress_bytes: AtomicU64::new(0),
            inline_latencies: Mutex::new(Reservoir::new(RESERVOIR_SIZE)),
            queue_latencies: Mutex::new(Reservoir::new(RESERVOIR_SIZE)),
        }
    }

    fn cause_index(cause: RejectionCause) -> usize {
        match cause {
            RejectionCause::MalformedInput => 0,
            RejectionCause::SsrfDenied => 1,
            RejectionCause::IpRateLimited => 2,
            RejectionCause::DailyBudget => 3,
            RejectionCause::TargetBudget => 4,
            RejectionCause::QueueBusy => 5,
        }
    }

    /// Record a submission accepted (token issued, job queued or inline run).
    pub fn record_accepted(&self) {
        self.accepted.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a refusal with its dashboard bucket.
    pub fn record_rejected(&self, cause: RejectionCause) {
        self.rejected_by_cause[Self::cause_index(cause)].fetch_add(1, Ordering::Relaxed);
    }

    /// Add fetched-body bytes to the egress counter. Called from the
    /// fetcher for every response body read (including truncated ones —
    /// the bytes did cross the wire).
    pub fn record_egress_bytes(&self, bytes: u64) {
        self.egress_bytes.fetch_add(bytes, Ordering::Relaxed);
    }
    /// Record a terminal scan outcome (worker or inline completion).
    ///
    /// A scan rejected at static validation (`ScanOutcome::Rejected`) is
    /// also a submission-level rejection: the reason string comes from
    /// `guard::GuardError::InvalidTarget`'s fixed `&'static str` messages,
    /// so classification by content is stable. Literal private IPs are
    /// counted as SSRF probing (they target internal infrastructure
    /// directly); everything else is malformed input.
    pub fn record_outcome(&self, outcome: &crate::scan::ScanOutcome) {
        match outcome {
            crate::scan::ScanOutcome::Complete(_) => {
                self.completed.fetch_add(1, Ordering::Relaxed);
            }
            crate::scan::ScanOutcome::RobotsBlocked { .. } => {
                self.robots_blocked.fetch_add(1, Ordering::Relaxed);
            }
            crate::scan::ScanOutcome::Rejected { reason } => {
                self.scan_rejected.fetch_add(1, Ordering::Relaxed);
                let cause = if reason.contains("not a public address") {
                    RejectionCause::SsrfDenied
                } else {
                    RejectionCause::MalformedInput
                };
                self.record_rejected(cause);
            }
            crate::scan::ScanOutcome::BudgetExhausted { .. } => {
                self.budget_exhausted_outcomes
                    .fetch_add(1, Ordering::Relaxed);
                self.record_rejected(RejectionCause::TargetBudget);
            }
        }
    }

    /// Record submit→result-ready latency for a queue-posture scan: from
    /// `first_enqueued_at` (preserved across retries and crash recovery) to
    /// completion. This is the runbook §4 latency row for the multi-replica
    /// posture; the inline posture measures the same span via
    /// [`Self::record_inline_latency_ms`].
    pub fn record_scan_latency_ms(&self, ms: u64) {
        let mut r = self
            .queue_latencies
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        r.record(ms);
    }

    /// Record end-to-end submission latency for the inline posture
    /// (validate → scan → stored). Queue-posture submissions record the
    /// fetch-path latency via [`Self::record_outcome`] instead.
    pub fn record_inline_latency_ms(&self, ms: u64) {
        let mut r = self
            .inline_latencies
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        r.record(ms);
    }

    /// Serialize a snapshot for `/metrics`. Counts are cumulative since
    /// process start (dashboard agents handle rates).
    pub fn snapshot_json(&self) -> serde_json::Value {
        let causes: serde_json::Map<String, serde_json::Value> = self
            .rejected_by_cause
            .iter()
            .zip(
                [
                    RejectionCause::MalformedInput,
                    RejectionCause::SsrfDenied,
                    RejectionCause::IpRateLimited,
                    RejectionCause::DailyBudget,
                    RejectionCause::TargetBudget,
                    RejectionCause::QueueBusy,
                ]
                .iter(),
            )
            .map(|(c, k)| {
                (
                    k.as_str().to_string(),
                    serde_json::json!(c.load(Ordering::Relaxed)),
                )
            })
            .collect();

        let inline = self
            .inline_latencies
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        let queued = self
            .queue_latencies
            .lock()
            .unwrap_or_else(|p| p.into_inner());

        serde_json::json!({
            "submissions": {
                "accepted": self.accepted.load(Ordering::Relaxed),
                "rejected_total": self.rejected_by_cause.iter().map(|c| c.load(Ordering::Relaxed)).sum::<u64>(),
                "rejected_by_cause": causes,
            },
            "outcomes": {
                "complete": self.completed.load(Ordering::Relaxed),
                "robots_blocked": self.robots_blocked.load(Ordering::Relaxed),
                "rejected": self.scan_rejected.load(Ordering::Relaxed),
                "budget_exhausted": self.budget_exhausted_outcomes.load(Ordering::Relaxed),
            },
            "egress_bytes_total": self.egress_bytes.load(Ordering::Relaxed),
            "latency_ms": {
                "inline_p50": inline.percentile(50),
                "inline_p95": inline.percentile(95),
                "inline_sample_n": inline.len(),
                "scan_p50": queued.percentile(50),
                "scan_p95": queued.percentile(95),
                "scan_sample_n": queued.len(),
            },
        })
    }
}

/// Process-global handle. The scanner is one process per replica; the
/// endpoint serves the local process's view (per-replica dashboards are
/// the standard scrape model; aggregates happen at the dashboard layer).
pub static METRICS: Metrics = Metrics {
    accepted: AtomicU64::new(0),
    rejected_by_cause: [
        AtomicU64::new(0),
        AtomicU64::new(0),
        AtomicU64::new(0),
        AtomicU64::new(0),
        AtomicU64::new(0),
        AtomicU64::new(0),
    ],
    completed: AtomicU64::new(0),
    robots_blocked: AtomicU64::new(0),
    scan_rejected: AtomicU64::new(0),
    budget_exhausted_outcomes: AtomicU64::new(0),
    egress_bytes: AtomicU64::new(0),
    inline_latencies: Mutex::new(Reservoir {
        samples: Vec::new(),
        size: RESERVOIR_SIZE,
        seen: 0,
    }),
    queue_latencies: Mutex::new(Reservoir {
        samples: Vec::new(),
        size: RESERVOIR_SIZE,
        seen: 0,
    }),
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejection_causes_map_to_stable_keys() {
        // The runbook's paging rule keys on these names; renaming one is a
        // dashboard break and must be a deliberate diff.
        assert_eq!(RejectionCause::SsrfDenied.as_str(), "ssrf_denied");
        assert_eq!(RejectionCause::DailyBudget.as_str(), "daily_budget");
        assert_eq!(RejectionCause::MalformedInput.as_str(), "malformed_input");
        assert_eq!(RejectionCause::IpRateLimited.as_str(), "ip_rate_limited");
        assert_eq!(RejectionCause::TargetBudget.as_str(), "target_budget");
        assert_eq!(RejectionCause::QueueBusy.as_str(), "queue_busy");
    }

    #[test]
    fn counters_count_and_snapshot_shapes() {
        let m = Metrics::new();
        m.record_accepted();
        m.record_accepted();
        m.record_rejected(RejectionCause::SsrfDenied);
        m.record_rejected(RejectionCause::SsrfDenied);
        m.record_rejected(RejectionCause::IpRateLimited);
        m.record_egress_bytes(1500);

        let snap = m.snapshot_json();
        assert_eq!(snap["submissions"]["accepted"], 2);
        assert_eq!(snap["submissions"]["rejected_by_cause"]["ssrf_denied"], 2);
        assert_eq!(
            snap["submissions"]["rejected_by_cause"]["ip_rate_limited"],
            1
        );
        assert_eq!(snap["submissions"]["rejected_total"], 3);
        assert_eq!(snap["egress_bytes_total"], 1500);
        // Latency percentiles absent with no samples — not zero, which
        // would be indistinguishable from "everything answered instantly".
        assert!(snap["latency_ms"]["inline_p95"].is_null());
    }

    #[test]
    fn reservoir_p95_tracks_distribution() {
        let mut r = Reservoir::new(1000);
        for ms in 0u64..1000 {
            r.record(ms * 10); // 0..9990ms, uniform
        }
        let p95 = r.percentile(95).expect("samples present");
        // Uniform 0..10s: p95 lands in 9.0–9.9s with overwhelming
        // probability; exact value depends on reservoir luck.
        assert!(
            (9000..=9990).contains(&p95),
            "p95 {p95} outside uniform tail"
        );
    }

    #[test]
    fn reservoir_is_bounded() {
        let mut r = Reservoir::new(64);
        for ms in 0u64..10_000 {
            r.record(ms);
        }
        assert_eq!(r.len(), 64, "reservoir never grows past its size");
        assert!(r.percentile(95).is_some());
    }

    #[test]
    fn outcomes_bucket_into_dashboard_rows() {
        let m = Metrics::new();
        m.record_outcome(&crate::scan::ScanOutcome::RobotsBlocked {
            submitted_url: "https://example.com".into(),
        });
        m.record_outcome(&crate::scan::ScanOutcome::Rejected {
            reason: "test".into(),
        });
        m.record_outcome(&crate::scan::ScanOutcome::BudgetExhausted {
            retry_after_secs: 60,
        });
        let snap = m.snapshot_json();
        assert_eq!(snap["outcomes"]["robots_blocked"], 1);
        assert_eq!(snap["outcomes"]["rejected"], 1);
        assert_eq!(snap["outcomes"]["budget_exhausted"], 1);
    }

    #[test]
    fn scan_latency_uses_submit_to_ready_span() {
        let m = Metrics::new();
        m.record_scan_latency_ms(1500);
        m.record_scan_latency_ms(4500);
        let snap = m.snapshot_json();
        assert_eq!(snap["latency_ms"]["scan_sample_n"], 2);
        assert!(!snap["latency_ms"]["scan_p50"].is_null());
    }
}
