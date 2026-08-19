use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

/// Determinism controller for reproducible crawl results.
///
/// Ensures same input + same config → same output.
/// Uses seed-based PRNG for any randomized operations.
///
/// # Examples
///
/// ```rust
/// use crawlkit_engine::DeterminismController;
///
/// let ctrl = DeterminismController::new(42);
/// assert_eq!(ctrl.seed(), 42);
///
/// // derive_seed is a pure hash of (seed, context): the same context
/// // always yields the same derived seed, on any controller instance.
/// assert_eq!(ctrl.derive_seed("context1"), ctrl.derive_seed("context1"));
/// let ctrl2 = DeterminismController::new(42);
/// assert_eq!(ctrl.derive_seed("context1"), ctrl2.derive_seed("context1"));
///
/// // derive_seed_stream mixes in an internal counter, so repeated calls
/// // with the same context are unique (but call-order sensitive).
/// assert_ne!(
///     ctrl.derive_seed_stream("context1"),
///     ctrl.derive_seed_stream("context1")
/// );
/// ```
pub struct DeterminismController {
    /// Base seed for reproducible randomness.
    seed: u64,
    /// Whether determinism is enforced.
    enforced: Arc<AtomicBool>,
    /// Counter for unique seeding in [`DeterminismController::derive_seed_stream`].
    counter: AtomicU64,
}

impl DeterminismController {
    /// Create a new determinism controller.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            enforced: Arc::new(AtomicBool::new(true)),
            counter: AtomicU64::new(0),
        }
    }

    /// Create with default seed (0).
    #[must_use]
    pub fn with_default_seed() -> Self {
        Self::new(0)
    }

    /// Get the base seed.
    #[must_use]
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Generate a deterministic seed for a specific context.
    ///
    /// This is a **pure** function of `(self.seed, context)`: no internal
    /// state is read or mutated, so the same controller (or a fresh one
    /// with the same base seed) always returns the same value for the
    /// same context, regardless of call order or concurrency. Prefer this
    /// over [`derive_seed_stream`](Self::derive_seed_stream) whenever the
    /// context alone identifies the use site.
    #[must_use]
    pub fn derive_seed(&self, context: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.seed.hash(&mut hasher);
        context.hash(&mut hasher);
        hasher.finish()
    }

    /// Generate a stream of unique seeds for a repeated context.
    ///
    /// Unlike [`derive_seed`](Self::derive_seed), this mixes in an internal
    /// atomic counter, so each call returns a fresh value even for the same
    /// context. This makes it **order-sensitive**: results depend on how
    /// many calls happened before, so only use it when you genuinely need a
    /// non-repeating stream (e.g. simulating random draws) and not for
    /// replaying deterministic crawls.
    #[must_use]
    pub fn derive_seed_stream(&self, context: &str) -> u64 {
        let counter = self.counter.fetch_add(1, Ordering::AcqRel);
        let mut hasher = DefaultHasher::new();
        self.seed.hash(&mut hasher);
        context.hash(&mut hasher);
        counter.hash(&mut hasher);
        hasher.finish()
    }

    /// Check if determinism is enforced.
    #[must_use]
    pub fn is_enforced(&self) -> bool {
        self.enforced.load(Ordering::Acquire)
    }

    /// Enable or disable determinism enforcement.
    pub fn set_enforced(&self, enforced: bool) {
        self.enforced.store(enforced, Ordering::Release);
    }

    /// Compute deterministic hash for content.
    #[must_use]
    pub fn content_hash(content: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        hasher.finish()
    }

    /// Compute deterministic hash for URL.
    #[must_use]
    pub fn url_hash(url: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        url.hash(&mut hasher);
        hasher.finish()
    }
}

impl Default for DeterminismController {
    fn default() -> Self {
        Self::with_default_seed()
    }
}

/// Deterministic URL ordering for consistent output.
pub fn deterministic_sort<T>(items: &mut [T], key_fn: impl Fn(&T) -> u64) {
    items.sort_by_key(|a| key_fn(a));
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_determinism_controller_seed() {
        let ctrl = DeterminismController::new(42);
        assert_eq!(ctrl.seed(), 42);
        assert!(ctrl.is_enforced());
    }

    #[test]
    fn test_determinism_controller_derive_seed() {
        let ctrl = DeterminismController::new(42);
        let seed1 = ctrl.derive_seed("context1");
        let seed2 = ctrl.derive_seed("context2");
        assert_ne!(seed1, seed2);
    }

    #[test]
    fn test_derive_seed_is_pure_for_same_context() {
        let ctrl = DeterminismController::new(42);
        assert_eq!(ctrl.derive_seed("ctx"), ctrl.derive_seed("ctx"));
        // Interleaving other calls must not change the result.
        let first = ctrl.derive_seed("ctx");
        let _ = ctrl.derive_seed("other");
        let _ = ctrl.derive_seed_stream("other");
        assert_eq!(ctrl.derive_seed("ctx"), first);
    }

    #[test]
    fn test_derive_seed_matches_across_instances() {
        let ctrl1 = DeterminismController::new(7);
        let ctrl2 = DeterminismController::new(7);
        assert_eq!(ctrl1.derive_seed("page/1"), ctrl2.derive_seed("page/1"));
        // A different base seed must (statistically) change the derivation.
        let ctrl3 = DeterminismController::new(8);
        assert_ne!(ctrl1.derive_seed("page/1"), ctrl3.derive_seed("page/1"));
    }

    #[test]
    fn test_derive_seed_stream_is_unique_per_call() {
        let ctrl = DeterminismController::new(42);
        let s1 = ctrl.derive_seed_stream("ctx");
        let s2 = ctrl.derive_seed_stream("ctx");
        let s3 = ctrl.derive_seed_stream("ctx");
        assert_ne!(s1, s2);
        assert_ne!(s2, s3);
        assert_ne!(s1, s3);
        // Stream values differ from the pure derivation for the same context.
        assert_ne!(ctrl.derive_seed("ctx"), ctrl.derive_seed_stream("ctx"));
    }

    #[test]
    fn test_determinism_controller_same_seed() {
        let ctrl1 = DeterminismController::new(42);
        let ctrl2 = DeterminismController::new(42);
        assert_eq!(ctrl1.seed(), ctrl2.seed());
    }

    #[test]
    fn test_content_hash_deterministic() {
        let hash1 = DeterminismController::content_hash("hello");
        let hash2 = DeterminismController::content_hash("hello");
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_content_hash_different() {
        let hash1 = DeterminismController::content_hash("hello");
        let hash2 = DeterminismController::content_hash("world");
        assert_ne!(hash1, hash2);
    }
}
