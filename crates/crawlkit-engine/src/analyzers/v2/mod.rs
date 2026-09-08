//! Category-based modules for the V2 analyzer set.

//! This module is a pure structural split of the former `v2_analyzers.rs`;
//! every type is re-exported so the public API is unchanged.

use crate::analyzers::AnalysisContext;

pub mod accessibility;
pub mod content;
pub mod performance;
pub mod schema;
pub mod scoring;
pub mod security;
pub mod seo;

pub use accessibility::*;
pub use content::*;
pub use performance::*;
pub use schema::*;
pub use scoring::*;
pub use security::*;
pub use seo::*;

/// Returns true when the response is an HTML document worth page analysis.
///
/// Analyzers that parse markup or inspect page structure should skip
/// non-HTML payloads (JSON APIs, feeds, plain text, sitemaps served to a
/// crawling bot, and so on) — otherwise their "missing X" checks are noise.
/// When the Content-Type header is absent, the page is assumed to be HTML.
pub(crate) fn is_html_response(ctx: &AnalysisContext<'_>) -> bool {
    match ctx.content_type {
        Some(ct) => {
            let lowered = ct.to_ascii_lowercase();
            let base = lowered.split(';').next().unwrap_or("").trim();
            base.is_empty()
                || base == "text/html"
                || base == "application/xhtml+xml"
                || base.starts_with("text/html")
        }
        // No Content-Type available: assume HTML rather than skipping analysis.
        None => true,
    }
}

/// Returns true when the page responded with 200 OK (or the status is unknown).
///
/// Findings about missing titles, headings, links, or security headers are
/// meaningless on error pages (4xx/5xx) and redirect responses (3xx): the
/// final destination page is what search engines evaluate.
pub(crate) fn is_success_page(ctx: &AnalysisContext<'_>) -> bool {
    match ctx.status_code {
        Some(code) => code == 200,
        None => true,
    }
}
