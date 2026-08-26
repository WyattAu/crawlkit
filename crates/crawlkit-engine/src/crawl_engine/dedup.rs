use crate::DeterminismController;
use dashmap::DashSet;

/// Content-hash deduplication set. The strategy is chosen once at crawl start
/// based on whether deterministic (seeded) mode is enabled.
pub(crate) enum ContentHashes {
    Deterministic(DashSet<u64>),
    Sha256(DashSet<String>),
}

impl ContentHashes {
    pub(crate) fn deterministic() -> Self {
        Self::Deterministic(DashSet::new())
    }

    pub(crate) fn sha256() -> Self {
        Self::Sha256(DashSet::new())
    }

    /// Insert the body's hash; returns `false` if the content was already seen.
    pub(crate) fn insert(&self, body: &str) -> bool {
        match self {
            Self::Deterministic(set) => set.insert(DeterminismController::content_hash(body)),
            Self::Sha256(set) => {
                use sha2::{Digest, Sha256};
                let digest = Sha256::digest(body.as_bytes());
                set.insert(digest.iter().map(|b| format!("{b:02x}")).collect())
            }
        }
    }
}
