#![allow(
    clippy::unwrap_used,
    clippy::manual_range_contains,
    clippy::redundant_closure,
    clippy::collapsible_if,
    clippy::unnecessary_map_or,
    clippy::default_constructed_unit_structs,
    clippy::needless_return,
    clippy::needless_range_loop,
    clippy::useless_format,
    clippy::if_same_then_else,
    clippy::derivable_impls,
    clippy::manual_pattern_char_comparison,
    clippy::manual_contains,
    unused_imports
)]
use crate::types::{IssueCategory, Severity};

// All analyzer implementations have been extracted to focused modules
// (Phase 2 SRP decomposition). This file remains as the legacy module path:
// it re-exports every analyzer so `security_analyzers::X` paths keep working,
// and hosts the legacy test modules below. The imports below are consumed by
// those test modules via `super::*`.
use super::{AnalysisContext, Analyzer, Finding};

pub use super::accessibility_aggregator_analyzers::AccessibilityAnalyzer;
pub use super::accessibility_v2_analyzers::{
    AriaRolesAnalyzerV2, FormAccessibilityAnalyzerV2, HeadingHierarchyAnalyzerV2,
    ImageAccessibilityAnalyzerV2, LanguageAttributeAnalyzerV2, LinkAccessibilityAnalyzerV2,
    TabindexAnalyzerV2, TableAccessibilityAnalyzerV2,
};
pub use super::aria_focus_link_validator_analyzers::{
    AnchorTextGenericAnalyzer, AriaRequiredAttributesAnalyzer, FocusOrderPositiveTabindexAnalyzer,
};
pub use super::aria_label_analyzers::AriaLabelAnalyzer;
pub use super::basic_accessibility_analyzers::{
    AriaRoleAnalyzer, FormInputLabelAnalyzer, ImageAltTextAnalyzer, LinkTextAnalyzer,
};
pub use super::csp_color_contrast_analyzers::{
    ColorContrastLinkAnalyzer, ColorContrastTextAnalyzer, CspDirectiveValidator,
};
pub use super::dns_sri_cors_analyzers::{
    CorsMisconfigurationAnalyzer, DnsRebindingAnalyzer, SubresourceIntegrityAnalyzer,
};
pub use super::form_table_validator_analyzers::{
    FormLabelAssociationAnalyzer, TableCaptionPresenceAnalyzer, TableHeaderScopeAnalyzer,
};
pub use super::landmark_heading_validator_analyzers::{
    HeadingLevelSkipAnalyzer, LandmarkBannerAnalyzer, LandmarkMainAnalyzer, LandmarkNavAnalyzer,
};
pub use super::language_accessibility_analyzers::LanguageAttributeAnalyzer;
pub use super::mixed_content_validator_analyzers::{
    MixedContentFormValidator, MixedContentImageValidator, MixedContentScriptValidator,
};
pub use super::permissions_policy_v2_analyzers::PermissionsPolicyAnalyzerV2;
pub use super::security_header_aggregator_analyzers::SecurityHeaderAnalyzer;
pub use super::security_header_v2_analyzers::{
    ContentTypeSniffingAnalyzerV2, HstsPreloadListValidator, StrictTransportSecurityAnalyzerV2,
    XssProtectionAnalyzerV2,
};
pub use super::security_header_v3_analyzers::{
    ContentSecurityPolicyAnalyzerV2, CrossOriginIsolationAnalyzerV2,
    CrossOriginOpenerPolicyAnalyzerV2, PermissionsPolicyAnalyzerV3, ReferrerPolicyAnalyzerV2,
    StrictTransportSecurityAnalyzerV3, XContentTypeOptionsAnalyzerV2, XFrameOptionsAnalyzerV2,
};
pub use super::skip_link_analyzers::SkipLinkAnalyzer;
pub use super::tabindex_analyzers::TabindexAnalyzer;
pub use super::table_caption_analyzers::TableCaptionAnalyzer;

// ---------------------------------------------------------------------------
// 14. Security Header Analyzer — extracted to security_header_aggregator_analyzers.rs
// 17. Accessibility Analyzer — extracted to accessibility_aggregator_analyzers.rs
// (Phase 2 SRP step). Legacy module path re-exports both.
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// HSTS Preload Analyzer
// ---------------------------------------------------------------------------

// HstsPreloadAnalyzer extracted to hsts_analyzer.rs (Phase 2 SRP step).

// SRI, PermissionPolicy, CrossOriginIsolation, CSP, ReferrerPolicy,
// XFrameOptions, MixedContent analyzers extracted to
// security_header_analyzers.rs (Phase 2 SRP step).

// {name} extracted to cookies.rs (Phase 2 SRP step).

// =========================================================================
// XContentTypeOptionsAnalyzer, XPermittedCrossDomainPoliciesAnalyzer, and
// CrossOriginResourcePolicyAnalyzer extracted to x_header_analyzers.rs
// (Phase 2 SRP step).

// StrictTransportSecurityAnalyzer, COOP/COEP, Feature-Policy, Expect-CT, CT,
// CSP/CORS, and cookie analyzers were extracted into focused modules.

#[cfg(test)]
mod new_analyzer_tests;
#[cfg(test)]
mod tests;
