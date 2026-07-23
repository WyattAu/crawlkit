use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// Feature flags for runtime configuration.
///
/// Flags are immutable per-crawl session once set.
/// Supports TOML configuration and programmatic access.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlags {
    #[serde(flatten)]
    flags: HashMap<String, bool>,
}

impl FeatureFlags {
    /// Create empty feature flags.
    #[must_use]
    pub fn new() -> Self {
        Self {
            flags: HashMap::new(),
        }
    }

    /// Create from TOML configuration.
    ///
    /// # Errors
    /// Returns error if TOML is invalid.
    pub fn from_toml(toml_str: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(toml_str)
    }

    /// Set a feature flag.
    pub fn set(&mut self, key: &str, value: bool) {
        self.flags.insert(key.to_string(), value);
    }

    /// Get a feature flag value.
    #[must_use]
    pub fn get(&self, key: &str) -> bool {
        *self.flags.get(key).unwrap_or(&false)
    }

    /// Check if a flag exists.
    #[must_use]
    pub fn has(&self, key: &str) -> bool {
        self.flags.contains_key(key)
    }

    /// Get all flags.
    #[must_use]
    pub fn all(&self) -> &HashMap<String, bool> {
        &self.flags
    }

    /// Merge with another set of flags (other overrides self).
    #[must_use]
    pub fn merge(mut self, other: FeatureFlags) -> Self {
        for (k, v) in other.flags {
            self.flags.insert(k, v);
        }
        self
    }
}

impl Default for FeatureFlags {
    fn default() -> Self {
        let mut flags = HashMap::new();
        // Default flags
        flags.insert("js_rendering".to_string(), false);
        flags.insert("ai_analyzers".to_string(), true);
        flags.insert("wasm_analyzers".to_string(), true);
        flags.insert("backlink_analysis".to_string(), false);
        flags.insert("rum_integration".to_string(), false);
        flags.insert("encryption_at_rest".to_string(), false);
        flags.insert("audit_trail".to_string(), true);
        flags.insert("observability".to_string(), true);
        Self { flags }
    }
}

/// Shared feature flag state for concurrent access.
#[derive(Clone)]
pub struct SharedFeatureFlags {
    flags: Arc<RwLock<FeatureFlags>>,
}

impl SharedFeatureFlags {
    /// Create shared feature flags.
    #[must_use]
    pub fn new(flags: FeatureFlags) -> Self {
        Self {
            flags: Arc::new(RwLock::new(flags)),
        }
    }

    /// Get a flag value (thread-safe).
    #[must_use]
    pub fn get(&self, key: &str) -> bool {
        self.flags.read().get(key)
    }

    /// Set a flag value (thread-safe).
    pub fn set(&self, key: &str, value: bool) {
        self.flags.write().set(key, value);
    }

    /// Get all flags (thread-safe).
    #[must_use]
    pub fn snapshot(&self) -> FeatureFlags {
        self.flags.read().clone()
    }
}

// ---------------------------------------------------------------------------
// Standard feature flag keys
// ---------------------------------------------------------------------------

/// Feature flag: Enable Playwright JS rendering.
pub const FLAG_JS_RENDERING: &str = "js_rendering";

/// Feature flag: Enable AI search optimization analyzers.
pub const FLAG_AI_ANALYZERS: &str = "ai_analyzers";

/// Feature flag: Enable WASM error detection analyzers.
pub const FLAG_WASM_ANALYZERS: &str = "wasm_analyzers";

/// Feature flag: Enable backlink analysis.
pub const FLAG_BACKLINK_ANALYSIS: &str = "backlink_analysis";

/// Feature flag: Enable RUM (Real User Monitoring) integration.
pub const FLAG_RUM_INTEGRATION: &str = "rum_integration";

/// Feature flag: Enable encryption at rest.
pub const FLAG_ENCRYPTION_AT_REST: &str = "encryption_at_rest";

/// Feature flag: Enable audit trail logging.
pub const FLAG_AUDIT_TRAIL: &str = "audit_trail";

/// Feature flag: Enable observability stack.
pub const FLAG_OBSERVABILITY: &str = "observability";

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_flags_default() {
        let flags = FeatureFlags::default();
        assert!(!flags.get(FLAG_JS_RENDERING));
        assert!(flags.get(FLAG_AI_ANALYZERS));
        assert!(flags.get(FLAG_WASM_ANALYZERS));
        assert!(!flags.get(FLAG_BACKLINK_ANALYSIS));
    }

    #[test]
    fn test_feature_flags_set_get() {
        let mut flags = FeatureFlags::new();
        flags.set("custom_flag", true);
        assert!(flags.get("custom_flag"));
        flags.set("custom_flag", false);
        assert!(!flags.get("custom_flag"));
    }

    #[test]
    fn test_feature_flags_from_toml() {
        let toml_str = r#"
            js_rendering = true
            ai_analyzers = false
        "#;
        let flags = FeatureFlags::from_toml(toml_str).unwrap();
        assert!(flags.get("js_rendering"));
        assert!(!flags.get("ai_analyzers"));
    }

    #[test]
    fn test_feature_flags_merge() {
        let mut base = FeatureFlags::default();
        base.set("custom", false);

        let mut override_flags = FeatureFlags::new();
        override_flags.set("custom", true);
        override_flags.set("js_rendering", true);

        let merged = base.merge(override_flags);
        assert!(merged.get("custom"));
        assert!(merged.get("js_rendering"));
    }

    #[test]
    fn test_shared_feature_flags() {
        let flags = FeatureFlags::default();
        let shared = SharedFeatureFlags::new(flags);

        assert!(!shared.get(FLAG_JS_RENDERING));
        shared.set(FLAG_JS_RENDERING, true);
        assert!(shared.get(FLAG_JS_RENDERING));
    }
}
