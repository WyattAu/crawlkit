//! Analyzer trait for plugins.

use crate::context::AnalysisContext;
use crate::finding::Finding;

/// Trait for page analyzers.
///
/// Implement this trait to create a custom SEO analyzer.
/// The analyzer receives page content and returns findings.
pub trait Analyzer {
    /// Get the analyzer name.
    fn name(&self) -> &str;

    /// Analyze the page content and return findings.
    fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding>;
}
