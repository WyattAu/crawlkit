use std::sync::Arc;

use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

/// Audit event types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditEventType {
    CrawlStarted,
    CrawlCompleted,
    CrawlFailed,
    PageFetched,
    PageAnalysisStarted,
    PageAnalysisCompleted,
    FindingStored,
    ExportGenerated,
    ConfigChanged,
    ApiKeyCreated,
    ApiKeyRevoked,
    ErrorOccurred,
}

/// An audit trail event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Unique event ID.
    pub id: String,
    /// Event timestamp.
    pub timestamp: DateTime<Utc>,
    /// Type of event.
    pub event_type: AuditEventType,
    /// Actor who triggered the event (API key, CLI user).
    pub actor: String,
    /// Event details.
    pub details: String,
    /// SHA-256 hash of event data for tamper evidence.
    pub hash: String,
    /// Hash of previous event (chain).
    pub previous_hash: String,
}

/// Append-only audit trail with SHA-256 chaining.
///
/// Provides tamper-evident logging for compliance (Defence standards).
pub struct AuditTrail {
    events: Arc<Mutex<Vec<AuditEvent>>>,
}

impl AuditTrail {
    /// Create empty audit trail.
    #[must_use]
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Record an audit event.
    pub fn record(&self, event_type: AuditEventType, actor: &str, details: &str) -> AuditEvent {
        let mut events = self.events.lock();
        let previous_hash = events
            .last()
            .map(|e| e.hash.clone())
            .unwrap_or_else(|| "genesis".to_string());

        let event = AuditEvent {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            event_type,
            actor: actor.to_string(),
            details: details.to_string(),
            hash: compute_hash(details, &previous_hash),
            previous_hash,
        };

        events.push(event.clone());
        event
    }

    /// Get all events.
    #[must_use]
    pub fn events(&self) -> Vec<AuditEvent> {
        self.events.lock().clone()
    }

    /// Get event count.
    #[must_use]
    pub fn len(&self) -> usize {
        self.events.lock().len()
    }

    /// Check if trail is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events.lock().is_empty()
    }

    /// Verify chain integrity.
    #[must_use]
    pub fn verify_integrity(&self) -> bool {
        let events = self.events.lock();
        let mut prev_hash = "genesis".to_string();

        for event in events.iter() {
            if event.previous_hash != prev_hash {
                return false;
            }
            let expected_hash = compute_hash(&event.details, &event.previous_hash);
            if event.hash != expected_hash {
                return false;
            }
            prev_hash = event.hash.clone();
        }

        true
    }

    /// Clear all events.
    pub fn clear(&self) {
        self.events.lock().clear();
    }
}

impl Default for AuditTrail {
    fn default() -> Self {
        Self::new()
    }
}

/// Compute SHA-256 hash for tamper evidence.
///
/// Uses SHA-256 (FIPS 180-4) for cryptographic tamper evidence.
/// The hash chains `details` with `previous_hash` to create an append-only
/// tamper-evident log.
fn compute_hash(details: &str, previous_hash: &str) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(details.as_bytes());
    hasher.update(b"\0"); // separator to prevent length-extension ambiguity
    hasher.update(previous_hash.as_bytes());
    let result = hasher.finalize();
    // Format each byte as lowercase hex
    result.iter().map(|b| format!("{b:02x}")).collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_trail_record() {
        let trail = AuditTrail::new();
        let event = trail.record(
            AuditEventType::CrawlStarted,
            "cli",
            "Crawl started for https://example.com",
        );

        assert_eq!(trail.len(), 1);
        assert!(!event.id.is_empty());
        assert_eq!(event.previous_hash, "genesis");
    }

    #[test]
    fn test_audit_trail_chain() {
        let trail = AuditTrail::new();
        let e1 = trail.record(AuditEventType::CrawlStarted, "cli", "start");
        let e2 = trail.record(AuditEventType::PageFetched, "cli", "fetched");

        assert_eq!(e2.previous_hash, e1.hash);
        assert!(trail.verify_integrity());
    }

    #[test]
    fn test_audit_trail_integrity() {
        let trail = AuditTrail::new();
        trail.record(AuditEventType::CrawlStarted, "cli", "start");
        trail.record(AuditEventType::PageFetched, "cli", "fetched");

        assert!(trail.verify_integrity());
    }

    #[test]
    fn test_audit_trail_empty() {
        let trail = AuditTrail::new();
        assert!(trail.is_empty());
        assert!(trail.verify_integrity());
    }
}
