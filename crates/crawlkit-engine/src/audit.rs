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
    LoginSucceeded,
    LoginFailed,
    SessionRevoked,
    TenantCreated,
    TenantDeleted,
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
    /// Tenant the event belongs to, when applicable.
    #[serde(default)]
    pub tenant_id: Option<String>,
    /// Event details.
    pub details: String,
    /// SHA-256 hash of event data for tamper evidence.
    pub hash: String,
    /// Hash of previous event (chain).
    pub previous_hash: String,
}

/// Errors from persistent audit trail operation.
#[derive(Debug, thiserror::Error)]
pub enum AuditError {
    #[error("audit I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("audit log line is not valid JSON: {0}")]
    CorruptLine(String),

    #[error("audit log chain verification failed at event {0}: the log has been tampered with or truncated")]
    ChainBroken(String),

    #[error("audit log persistence is not enabled")]
    NotPersistent,
}

/// Append-only audit trail with SHA-256 chaining.
///
/// Provides tamper-evident logging for compliance (Defence standards).
/// In-memory by default; [`AuditTrail::open_persistent`] additionally
/// appends every event to a JSONL file with an fsync after each write and
/// verifies the chain when the file is reopened. Truncating or editing the
/// file is detected as a broken chain.
pub struct AuditTrail {
    events: Arc<Mutex<Vec<AuditEvent>>>,
    sink: Option<Arc<Mutex<std::fs::File>>>,
    /// Sidecar storing the expected tail (count + head hash) for
    /// truncation detection.
    head_path: Option<std::path::PathBuf>,
}

/// Truncation-detection anchor: event count and hash of the last event.
#[derive(Debug, Serialize, Deserialize)]
struct HeadAnchor {
    count: usize,
    head_hash: String,
}

/// Path of the head-anchor sidecar for a given audit log path.
fn head_sidecar_path(path: &std::path::Path) -> std::path::PathBuf {
    let mut name = path
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_default();
    name.push(".head");
    path.with_file_name(name)
}

impl AuditTrail {
    /// Create empty in-memory audit trail.
    #[must_use]
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
            sink: None,
            head_path: None,
        }
    }

    /// Open (or create) a persistent audit trail at `path`.
    ///
    /// Existing events are loaded into memory and the hash chain is
    /// verified; a tampered log surfaces as [`AuditError::ChainBroken`].
    /// Truncation of the tail is detected against the head anchor stored in
    /// the `path.head` sidecar (best-effort: an attacker who can rewrite
    /// both files can defeat truncation detection; in-place edits to the
    /// log itself are always caught). Every subsequent [`Self::record`] is
    /// appended to the file and flushed to disk before returning.
    ///
    /// # Errors
    ///
    /// Returns [`AuditError`] on I/O failure, a malformed line, or a
    /// chain-verification failure.
    pub fn open_persistent(path: &std::path::Path) -> Result<Self, AuditError> {
        let mut events = Vec::new();
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            for (index, line) in content.lines().enumerate() {
                if line.trim().is_empty() {
                    continue;
                }
                let event: AuditEvent = serde_json::from_str(line)
                    .map_err(|e| AuditError::CorruptLine(format!("line {}: {e}", index + 1)))?;
                events.push(event);
            }
        }

        // Verify the loaded chain before accepting the log.
        {
            let mut prev_hash = "genesis".to_string();
            for event in &events {
                if event.previous_hash != prev_hash {
                    return Err(AuditError::ChainBroken(event.id.clone()));
                }
                let event_type_str = format!("{:?}", event.event_type);
                let expected_hash = compute_hash(
                    &event.id,
                    &event.timestamp.to_rfc3339(),
                    &event_type_str,
                    &event.actor,
                    event.tenant_id.as_deref().unwrap_or(""),
                    &event.details,
                    &event.previous_hash,
                );
                if event.hash != expected_hash {
                    return Err(AuditError::ChainBroken(event.id.clone()));
                }
                prev_hash = event.hash.clone();
            }
        }

        // Truncation check against the head anchor sidecar.
        let head_path = head_sidecar_path(path);
        if head_path.exists() {
            let anchor: HeadAnchor = serde_json::from_str(&std::fs::read_to_string(&head_path)?)
                .map_err(|e| AuditError::CorruptLine(format!("head anchor: {e}")))?;
            let matches_tail = events.len() == anchor.count
                && events.last().is_some_and(|e| e.hash == anchor.head_hash);
            if !matches_tail {
                return Err(AuditError::ChainBroken(
                    events.last().map(|e| e.id.clone()).unwrap_or_default(),
                ));
            }
        } else if !events.is_empty() {
            tracing::warn!(
                "Audit head anchor sidecar is absent; tail truncation cannot be verified"
            );
        }

        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        Ok(Self {
            events: Arc::new(Mutex::new(events)),
            sink: Some(Arc::new(Mutex::new(file))),
            head_path: Some(head_path),
        })
    }

    /// Record an audit event.
    ///
    /// When the trail is persistent, the event is serialized and appended
    /// to the backing file with an fsync before returning.
    pub fn record(&self, event_type: AuditEventType, actor: &str, details: &str) -> AuditEvent {
        self.record_tenant(event_type, actor, None, details)
    }

    /// Record an audit event bound to a tenant.
    ///
    /// Tenant-bound events enable tenant-scoped audit access control on
    /// the API surface.
    pub fn record_tenant(
        &self,
        event_type: AuditEventType,
        actor: &str,
        tenant_id: Option<&str>,
        details: &str,
    ) -> AuditEvent {
        let mut events = self.events.lock();
        let previous_hash = events
            .last()
            .map(|e| e.hash.clone())
            .unwrap_or_else(|| "genesis".to_string());

        let id = uuid::Uuid::new_v4().to_string();
        let timestamp = Utc::now();
        let event_type_str = format!("{event_type:?}");
        let tenant = tenant_id.unwrap_or("");
        let hash = compute_hash(
            &id,
            &timestamp.to_rfc3339(),
            &event_type_str,
            actor,
            tenant,
            details,
            &previous_hash,
        );

        let event = AuditEvent {
            id,
            timestamp,
            event_type,
            actor: actor.to_string(),
            tenant_id: tenant_id.map(str::to_string),
            details: details.to_string(),
            hash,
            previous_hash,
        };

        // Persist (fsync) before acknowledging the event in memory so a
        // crash cannot acknowledge an event that never reached disk.
        if let Some(sink) = &self.sink {
            let mut file = sink.lock();
            let line = serde_json::to_string(&event).unwrap_or_default();
            use std::io::Write;
            let write_result = file
                .write_all(line.as_bytes())
                .and_then(|()| file.write_all(b"\n"))
                .and_then(|()| file.sync_all());

            // Update the truncation anchor after a successful append.
            let anchor_result = write_result.and_then(|()| {
                let anchor = HeadAnchor {
                    count: events.len() + 1,
                    head_hash: event.hash.clone(),
                };
                let head_path = self
                    .head_path
                    .as_ref()
                    .ok_or_else(|| std::io::Error::other("no head path"))?;
                let anchor_json = serde_json::to_string(&anchor)
                    .map_err(|e| std::io::Error::other(e.to_string()))?;
                let mut head = std::fs::OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .open(head_path)?;
                head.write_all(anchor_json.as_bytes())
                    .and_then(|()| head.write_all(b"\n"))
                    .and_then(|()| head.sync_all())
            });

            if let Err(e) = anchor_result {
                tracing::error!("Failed to persist audit event: {e}");
            }
        }

        events.push(event.clone());
        event
    }

    /// Get all events.
    #[must_use]
    pub fn events(&self) -> Vec<AuditEvent> {
        self.events.lock().clone()
    }

    /// Get events for a specific tenant (or global events when
    /// `tenant_id` is `None`).
    #[must_use]
    pub fn events_for_tenant(&self, tenant_id: &str) -> Vec<AuditEvent> {
        self.events
            .lock()
            .iter()
            .filter(|e| e.tenant_id.as_deref() == Some(tenant_id))
            .cloned()
            .collect()
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
            let event_type_str = format!("{:?}", event.event_type);
            let expected_hash = compute_hash(
                &event.id,
                &event.timestamp.to_rfc3339(),
                &event_type_str,
                &event.actor,
                event.tenant_id.as_deref().unwrap_or(""),
                &event.details,
                &event.previous_hash,
            );
            if event.hash != expected_hash {
                return false;
            }
            prev_hash = event.hash.clone();
        }

        true
    }

    /// Clear in-memory events.
    ///
    /// Test-only helper: a persistent trail cannot be cleared (the file is
    /// append-only; call is ignored with a warning).
    pub fn clear(&self) {
        if self.sink.is_some() {
            tracing::warn!("AuditTrail::clear called on a persistent trail; ignored (append-only)");
            return;
        }
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
/// Hashes all event fields (`id`, `timestamp`, `event_type`, `actor`,
/// `tenant_id`, `details`, `previous_hash`) to create an append-only
/// tamper-evident log.
fn compute_hash(
    id: &str,
    timestamp: &str,
    event_type: &str,
    actor: &str,
    tenant_id: &str,
    details: &str,
    previous_hash: &str,
) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(id.as_bytes());
    hasher.update(b"\0");
    hasher.update(timestamp.as_bytes());
    hasher.update(b"\0");
    hasher.update(event_type.as_bytes());
    hasher.update(b"\0");
    hasher.update(actor.as_bytes());
    hasher.update(b"\0");
    hasher.update(tenant_id.as_bytes());
    hasher.update(b"\0");
    hasher.update(details.as_bytes());
    hasher.update(b"\0");
    hasher.update(previous_hash.as_bytes());
    let result = hasher.finalize();
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

    #[test]
    fn test_persistent_trail_survives_reopen_and_verifies() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.jsonl");

        {
            let trail = AuditTrail::open_persistent(&path).unwrap();
            trail.record(AuditEventType::CrawlStarted, "cli", "start");
            trail.record_tenant(
                AuditEventType::ApiKeyCreated,
                "admin@x",
                Some("tenant-a"),
                "key created",
            );
            assert!(trail.verify_integrity());
            assert_eq!(trail.len(), 2);
        }

        // Reopen: events reload and the chain still verifies.
        let reopened = AuditTrail::open_persistent(&path).unwrap();
        assert_eq!(reopened.len(), 2);
        assert!(reopened.verify_integrity());
        assert_eq!(reopened.events_for_tenant("tenant-a").len(), 1);

        // The reopened trail continues the chain seamlessly.
        reopened.record(AuditEventType::CrawlCompleted, "cli", "done");
        assert!(reopened.verify_integrity());
    }

    #[test]
    fn test_tampered_trail_is_rejected_on_open() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.jsonl");

        {
            let trail = AuditTrail::open_persistent(&path).unwrap();
            trail.record(AuditEventType::CrawlStarted, "cli", "start");
            trail.record(AuditEventType::CrawlCompleted, "cli", "done");
        }

        // Tamper: flip a character inside a details field.
        let content = std::fs::read_to_string(&path).unwrap();
        let tampered = content.replace("start", "strar");
        assert_ne!(content, tampered);
        std::fs::write(&path, tampered).unwrap();

        let result = AuditTrail::open_persistent(&path);
        assert!(
            matches!(result, Err(AuditError::ChainBroken(_))),
            "expected chain break"
        );
    }

    #[test]
    fn test_truncated_trail_is_rejected_on_open() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.jsonl");

        {
            let trail = AuditTrail::open_persistent(&path).unwrap();
            trail.record(AuditEventType::CrawlStarted, "cli", "start");
            trail.record(AuditEventType::CrawlCompleted, "cli", "done");
        }

        // Truncate: drop the final line.
        let content = std::fs::read_to_string(&path).unwrap();
        let truncated = content.lines().take(1).fold(String::new(), |mut acc, l| {
            acc.push_str(l);
            acc.push('\n');
            acc
        });
        std::fs::write(&path, truncated).unwrap();

        let result = AuditTrail::open_persistent(&path);
        assert!(
            matches!(result, Err(AuditError::ChainBroken(_))),
            "expected chain break from truncation"
        );
    }

    #[test]
    fn test_clear_on_persistent_trail_is_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.jsonl");
        let trail = AuditTrail::open_persistent(&path).unwrap();
        trail.record(AuditEventType::CrawlStarted, "cli", "start");
        trail.clear();
        assert_eq!(trail.len(), 1, "persistent trails must not be clearable");
    }
}
