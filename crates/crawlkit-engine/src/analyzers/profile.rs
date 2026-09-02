//! Analyzer profile selection for the crawl engine.
//!
//! A profile describes *which family* of built-in analyzer set the engine
//! should construct, without requiring the engine configuration to own a
//! non-cloneable [`AnalyzerRegistry`]. The registry itself is built once per
//! crawl by [`crate::analyzers::AnalyzerRegistry`] from the profile value.

use crate::analyzers::AnalyzerRegistry;
use crate::CrawlConfig;

/// Which built-in analyzer set the crawl engine should use.
///
/// The default is [`AnalyzerProfile::Full`] for backward compatibility:
/// feature flags then control whether the optional AI and WASM analyzer
/// groups are included. The other profiles construct a fixed, curated set
/// and are intentionally independent of the AI/WASM flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AnalyzerProfile {
    /// Complete built-in registry (default; honours AI/WASM feature flags).
    #[default]
    Full,
    /// Conservative foundational profile (9 analyzers).
    Core,
    /// One canonical analyzer per common family (21 analyzers).
    Standard,
    /// Focused deep security/accessibility/SEO analyzers (20 analyzers).
    Deep,
}

impl AnalyzerProfile {
    /// Parse a profile name, returning `None` for unknown names.
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "full" => Some(Self::Full),
            "core" => Some(Self::Core),
            "standard" => Some(Self::Standard),
            "deep" => Some(Self::Deep),
            _ => None,
        }
    }

    /// The canonical name of this profile (as accepted by [`Self::parse`]).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Core => "core",
            Self::Standard => "standard",
            Self::Deep => "deep",
        }
    }

    /// Build the concrete registry for this profile.
    pub fn build_registry(self, config: &CrawlConfig) -> AnalyzerRegistry {
        match self {
            Self::Full => AnalyzerRegistry::new(config),
            Self::Core => AnalyzerRegistry::core(config),
            Self::Standard => AnalyzerRegistry::standard(config),
            Self::Deep => AnalyzerRegistry::deep(config),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_round_trips_known_names() {
        for name in ["full", "core", "standard", "deep"] {
            let profile = AnalyzerProfile::parse(name).unwrap();
            assert_eq!(profile.as_str(), name);
        }
    }

    #[test]
    fn parse_rejects_unknown_names() {
        assert!(AnalyzerProfile::parse("").is_none());
        assert!(AnalyzerProfile::parse("FULL").is_none());
        assert!(AnalyzerProfile::parse("everything").is_none());
    }

    #[test]
    fn default_is_full() {
        assert_eq!(AnalyzerProfile::default(), AnalyzerProfile::Full);
    }

    #[test]
    fn profiles_build_distinct_registry_sizes() {
        let config = CrawlConfig::default();
        let full = AnalyzerProfile::Full.build_registry(&config).len();
        let core = AnalyzerProfile::Core.build_registry(&config).len();
        let standard = AnalyzerProfile::Standard.build_registry(&config).len();
        let deep = AnalyzerProfile::Deep.build_registry(&config).len();

        assert!(core < standard, "core={core} standard={standard}");
        assert!(standard < full, "standard={standard} full={full}");
        assert_eq!(core, 9);
        assert_eq!(standard, 21);
        assert_eq!(deep, 20);
    }
}
