//! Usage metering and quotas (ADR-017, 6.0.0-alpha.3 "Metering").
//!
//! Per-tenant accounting for the Scale phase: append-only counter events
//! written inline with acceptance/persistence — never post-hoc aggregation.
//! The honest-scope rule from the ADR: a number is "metered" only where its
//! recording point is deterministic under retry/failure; everything else
//! (wall-time, egress, renders) stays telemetry.
//!
//! Default posture is **unmetered**: a tenant with no quota rows has no
//! limits — self-hosted single-tenant users must never meet a quota they
//! did not set (ADR-017 §3). Quota exhaustion is explicit and
//! machine-readable, never silent truncation.

use std::fmt;

/// A metered unit of work (ADR-017 §1).
///
/// Only units whose recording point is deterministic under retry/failure
/// are metered; retries re-record idempotently (dedupe by logical key),
/// so an over-count is possible but an under-count is not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MeteredUnit {
    /// One page fetched and analyzed (ack'd in the crawl-frontier sense).
    Pages,
    /// One crawl accepted — the single most billable event.
    CrawlStarted,
    /// Findings persisted (analyzer output volume; drives warehouse cost).
    Findings,
    /// Bytes emitted by a warehouse export.
    ExportBytes,
    /// Hosted-scanner submissions accepted.
    ScanSubmitted,
}

impl MeteredUnit {
    /// Canonical wire/storage name.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pages => "pages",
            Self::CrawlStarted => "crawl_started",
            Self::Findings => "findings",
            Self::ExportBytes => "export_bytes",
            Self::ScanSubmitted => "scan_submitted",
        }
    }

    /// Parses the canonical name; unknown names are `None` (callers decide
    /// whether that is an error or telemetry).
    pub fn from_str_opt(name: &str) -> Option<Self> {
        match name {
            "pages" => Some(Self::Pages),
            "crawl_started" => Some(Self::CrawlStarted),
            "findings" => Some(Self::Findings),
            "export_bytes" => Some(Self::ExportBytes),
            "scan_submitted" => Some(Self::ScanSubmitted),
            _ => None,
        }
    }
}

impl fmt::Display for MeteredUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One recorded metering delta (ADR-017 §2 schema).
///
/// Rows are keyed `(tenant_id, day_utc, unit)` with delta upserts; the
/// event carries the logical key that made the recording idempotent.
#[derive(Debug, Clone, PartialEq)]
pub struct MeterEvent {
    /// Tenant the work is attributed to. `None` is only valid for
    /// un-tenanted self-hosted crawls, which are not metered.
    pub tenant_id: String,
    /// UTC day (start of day) the usage is attributed to.
    pub day_utc: chrono::DateTime<chrono::Utc>,
    /// What was consumed.
    pub unit: MeteredUnit,
    /// How much (always positive; the caller rolls up).
    pub delta: i64,
    /// Logical dedupe key (e.g. `crawl_id`, `page_id`) — retries with the
    /// same key re-record the same rollup rather than double-counting.
    pub dedupe_key: String,
}

impl MeterEvent {
    /// Convenience constructor for a whole-day rollup event.
    pub fn rollup(tenant_id: &str, unit: MeteredUnit, delta: i64, dedupe_key: &str) -> Self {
        Self {
            tenant_id: tenant_id.to_string(),
            day_utc: day_floor(chrono::Utc::now()),
            unit,
            delta,
            dedupe_key: dedupe_key.to_string(),
        }
    }
}

/// Truncates a timestamp to the start of its UTC day.
///
/// Storage-schema helper (the counter row key); exposed crate-wide so the
/// storage layer and this module agree on the day boundary.
pub fn day_floor(t: chrono::DateTime<chrono::Utc>) -> chrono::DateTime<chrono::Utc> {
    // `date_naive().and_hms_opt(0,0,0)` is always `Some` (midnight is a
    // valid time); `and_utc()` is the canonical construction for naive
    // UTC dates.
    t.date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap_or_default()
        .and_utc()
}

/// A configured per-tenant, per-UTC-day quota for one unit.
///
/// `None` limits mean unmetered for that unit (the default posture).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Quota {
    /// Max `pages` per UTC day; `None` = unlimited.
    pub pages: Option<i64>,
    /// Max `crawl_started` per UTC day; `None` = unlimited.
    pub crawl_started: Option<i64>,
    /// Max `findings` per UTC day; `None` = unlimited.
    pub findings: Option<i64>,
    /// Max `export_bytes` per UTC day; `None` = unlimited.
    pub export_bytes: Option<i64>,
    /// Max `scan_submitted` per UTC day; `None` = unlimited.
    pub scan_submitted: Option<i64>,
}

impl Quota {
    /// The limit configured for `unit`, if any.
    pub fn limit_for(&self, unit: MeteredUnit) -> Option<i64> {
        match unit {
            MeteredUnit::Pages => self.pages,
            MeteredUnit::CrawlStarted => self.crawl_started,
            MeteredUnit::Findings => self.findings,
            MeteredUnit::ExportBytes => self.export_bytes,
            MeteredUnit::ScanSubmitted => self.scan_submitted,
        }
    }

    /// The verdict for a would-be consumption of `requested` against
    /// `used_today`.
    ///
    /// In-flight work finishes and records overage as telemetry (ADR-017
    /// §3): an over-quota *read* is allowed but explicitly marked.
    pub fn verdict(&self, unit: MeteredUnit, used_today: i64, requested: i64) -> QuotaVerdict {
        match self.limit_for(unit) {
            None => QuotaVerdict::Allowed,
            Some(limit) if used_today + requested <= limit => QuotaVerdict::Allowed,
            Some(limit) if used_today < limit => QuotaVerdict::Partial {
                limit,
                remaining: limit - used_today,
            },
            Some(limit) => QuotaVerdict::Exhausted { limit },
        }
    }
}

/// Outcome of a quota check (ADR-017 §3 exhaustion semantics).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuotaVerdict {
    /// Under limit — proceed.
    Allowed,
    /// Would exceed the limit but some headroom remains. Callers may take
    /// the remaining slice and record the rest as overage telemetry.
    Partial {
        /// The configured daily limit.
        limit: i64,
        /// How much may still be consumed today.
        remaining: i64,
    },
    /// Limit reached — new work is refused until the UTC boundary with a
    /// machine-readable cause.
    Exhausted {
        /// The configured daily limit.
        limit: i64,
    },
}

impl QuotaVerdict {
    /// True when new work may be accepted at all.
    pub fn is_acceptable(&self) -> bool {
        matches!(self, Self::Allowed | Self::Partial { .. })
    }
}

impl fmt::Display for QuotaVerdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Allowed => write!(f, "allowed"),
            Self::Partial { remaining, .. } => write!(f, "partial ({remaining} remaining)"),
            Self::Exhausted { limit } => write!(f, "exhausted (limit {limit})"),
        }
    }
}

/// Aggregate usage read model for the `GET /tenants/{id}/usage` read path.
#[derive(Debug, Clone, PartialEq)]
pub struct UsageEntry {
    /// Unit consumed.
    pub unit: MeteredUnit,
    /// UTC day of the rollup row.
    pub day_utc: chrono::DateTime<chrono::Utc>,
    /// Total recorded that day.
    pub total: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    // Verdict semantics ---------------------------------------------------

    #[test]
    fn unmetered_is_always_allowed() {
        let q = Quota::default();
        assert_eq!(
            q.verdict(MeteredUnit::Pages, 10_000, 1),
            QuotaVerdict::Allowed
        );
    }

    #[test]
    fn under_limit_is_allowed() {
        let q = Quota {
            pages: Some(100),
            ..Quota::default()
        };
        assert_eq!(q.verdict(MeteredUnit::Pages, 90, 10), QuotaVerdict::Allowed);
    }

    #[test]
    fn crossing_limit_is_partial_with_remaining() {
        let q = Quota {
            pages: Some(100),
            ..Quota::default()
        };
        assert_eq!(
            q.verdict(MeteredUnit::Pages, 90, 20),
            QuotaVerdict::Partial {
                limit: 100,
                remaining: 10
            }
        );
    }

    #[test]
    fn at_limit_is_exhausted() {
        let q = Quota {
            crawl_started: Some(5),
            ..Quota::default()
        };
        assert_eq!(
            q.verdict(MeteredUnit::CrawlStarted, 5, 1),
            QuotaVerdict::Exhausted { limit: 5 }
        );
    }

    #[test]
    fn verdict_is_unit_scoped() {
        let q = Quota {
            pages: Some(1),
            ..Quota::default()
        };
        assert_eq!(
            q.verdict(MeteredUnit::CrawlStarted, 999, 1),
            QuotaVerdict::Allowed
        );
    }

    #[test]
    fn partial_is_acceptable_exhausted_is_not() {
        assert!(QuotaVerdict::Allowed.is_acceptable());
        assert!(QuotaVerdict::Partial {
            limit: 10,
            remaining: 2
        }
        .is_acceptable());
        assert!(!QuotaVerdict::Exhausted { limit: 10 }.is_acceptable());
    }

    // Units ---------------------------------------------------------------

    #[test]
    fn unit_names_round_trip() {
        for (unit, name) in [
            (MeteredUnit::Pages, "pages"),
            (MeteredUnit::CrawlStarted, "crawl_started"),
            (MeteredUnit::Findings, "findings"),
            (MeteredUnit::ExportBytes, "export_bytes"),
            (MeteredUnit::ScanSubmitted, "scan_submitted"),
        ] {
            assert_eq!(unit.as_str(), name);
            assert_eq!(MeteredUnit::from_str_opt(name), Some(unit));
        }
        assert_eq!(MeteredUnit::from_str_opt("wall_time"), None);
    }

    // Rollup helpers ------------------------------------------------------

    #[test]
    fn day_floor_truncates_to_utc_midnight() {
        let t = chrono::DateTime::parse_from_rfc3339("2026-09-25T13:45:12Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let floored = day_floor(t);
        assert_eq!(floored.to_rfc3339(), "2026-09-25T00:00:00+00:00");
    }

    #[test]
    fn rollup_event_floors_day_and_carries_dedupe_key() {
        let ev = MeterEvent::rollup("tenant-a", MeteredUnit::Findings, 42, "crawl-1");
        assert_eq!(ev.tenant_id, "tenant-a");
        assert_eq!(ev.unit, MeteredUnit::Findings);
        assert_eq!(ev.delta, 42);
        assert_eq!(ev.dedupe_key, "crawl-1");
        assert_eq!(
            ev.day_utc.to_rfc3339(),
            day_floor(chrono::Utc::now()).to_rfc3339()
        );
    }

    // Open questions from the ADR, resolved per its proposals --------------

    #[test]
    fn open_question_1_refusals_are_not_metered_pages() {
        // ADR-017 open question 1: robots/SSRF-denied attempts are NOT
        // `pages` (only analyzed pages count). There is deliberately no
        // unit for refusal events — they are telemetry. This test pins the
        // unit vocabulary so a refusal unit cannot silently appear.
        assert!(MeteredUnit::from_str_opt("robots_denied").is_none());
        assert!(MeteredUnit::from_str_opt("ssrf_denied").is_none());
    }

    #[test]
    fn open_question_2_quota_of_zero_refuses_immediately() {
        // A zero limit is a hard pause: everything is refused from the
        // first unit (used 0 + requested 1 > 0, and no headroom).
        let q = Quota {
            scan_submitted: Some(0),
            ..Quota::default()
        };
        assert_eq!(
            q.verdict(MeteredUnit::ScanSubmitted, 0, 1),
            QuotaVerdict::Exhausted { limit: 0 }
        );
    }
}
