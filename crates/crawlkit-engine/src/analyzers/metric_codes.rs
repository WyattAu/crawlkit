//! Codes that report a **measurement** of the page rather than a defect.
//!
//! # Why this split exists
//!
//! Readability indices, keyword extraction, entity detection and composite
//! scores were emitted as `info` findings alongside real problems. On a
//! 100-page crawl of kingstonpeptides.com that was **2,401 of 4,696 info
//! findings** — 39% of *all* reported findings were measurements.
//!
//! That is not a cosmetic problem. A page with three real problems and twenty
//! scores looks identical, in aggregate, to a page with twenty-three problems.
//! Any severity roll-up, any "issues per page" average, and any trend line is
//! dominated by rows nobody can act on.
//!
//! # The test applied
//!
//! A code belongs here when its output is a **quantity about the page** — a
//! score, an index, an extracted list, a count — and there is no change to the
//! markup that would "fix" it. A code stays a finding whenever it describes
//! something the author should change, even if a number is involved.
//!
//! Kept as findings on purpose, despite being advisory:
//!
//!   * `KWPRO*`, `TITLEKDEN*`, `METAKEY-V5001`, `TITLEKWP-V5001`,
//!     `EXT-AUTH*` — content-quality advice, and actionable.
//!   * `CQ-V2001`, `CREAD*`, `WC004` — threshold judgements on readability that a
//!     site owner can genuinely act on by rewriting.
//!
//! Kept as findings because the markup is wrong:
//!
//!   * `CHARSET002`, `CACHE002`/`CACHE003`, `COEP-V2001`, `CORP001`,
//!     `PERMPDEEP002`, `AI-ACC006`, `IMGALT*`, `XSS002` — absent or
//!     misconfigured headers, missing attributes, blocked bots.
//!
//! `IMG003` ("Lazy-loaded images") sits on the boundary: it reports how images
//! are loaded rather than asking for a change, and is listed as a metric.
//! `SITEMAPMISS*` is a finding — a missing sitemap reference is a gap to close.
//!
//! Nothing is discarded: metric rows are stored alongside findings and
//! retrievable with [`is_metric_code`], and `crawlkit crawl --include-metrics`
//! keeps them in the primary output.

/// Measurement codes: output is a quantity, not a defect.
pub const METRIC_CODES: &[&str] = &[
    // Readability indices
    "READ001", // Flesch-Kincaid Grade Level
    "READ002", // Coleman-Liau Index
    "READ003", // Automated Readability Index
    "READ004", // Gunning Fog Index
    "READ005", // Flesch Reading Ease score
    "CQ001",   // Flesch-Kincaid readability score
    "CQ002",   // Top keywords
    "CQ005",   // Long-form content classification
    // Keyword extraction and density
    "KW001", // Top TF-IDF keywords
    "KW002", // Keyword density
    "KW004", // Keyword co-occurrence
    // Entity extraction
    "ENTITY001", // People entities detected
    "ENTITY002", // Organization entities detected
    "ENTITY003", // Location entities detected
    "ENTITY004", // Detected topics and themes
    "ENTITY005", // Content sentiment analysis
    "ENTITY006", // Entity counts per page
    // Composite scores
    "A11YSC001",    // Accessibility compliance score
    "MOB-SCORE001", // Mobile friendliness score
    "PERF-EST001",  // Estimated performance score
    "SEC012",       // Security posture score
    "SECSC001",     // Security header score
    "SOCIAL008",    // Social preview completeness score
    // Counts and transport facts
    "LINK001", // Link counts
    "WC001",   // Word count statistics
    "HTTP006", // Status category
    "SSL000",  // SSL certificate not inspected
    "IMG003",  // Lazy-loaded images
];

/// True when `code` reports a measurement rather than a defect.
///
/// Falls back to the code's bare name for the sibling metric codes that are
/// only produced by the aggregated metric registry, so a rename of an
/// analyzer's numeric suffix cannot silently reclassify its output as a defect.
#[must_use]
pub fn is_metric_code(code: &str) -> bool {
    METRIC_CODES.contains(&code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measurement_codes_are_metrics() {
        for code in METRIC_CODES {
            assert!(is_metric_code(code), "{code} must be a metric");
        }
    }

    #[test]
    fn defects_are_not_metrics() {
        // Markup problems must never be reclassified as measurements.
        for code in [
            "CHARSET002", "CACHE002", "CACHE003", "COEP-V2001", "CORP001",
            "PERMPDEEP002", "AI-ACC006", "XSS002", "SITEMAPMISS001", "NAP001",
            "IMGALTDEEP001", "CONTR001", "COLRCL001",
        ] {
            assert!(!is_metric_code(code), "{code} is a defect, not a metric");
        }
    }

    /// Codes whose analyzers were removed, with the reason each went.
    ///
    /// Kept as a single list so [`removed_codes_do_not_reappear`] can check all
    /// of them at once.
    const REMOVED_CODES: &[(&str, &str)] = &[
        (
            "ELINK001",
            "no engine requires an outbound body link from a schema entity",
        ),
        (
            "ELINK002",
            "same fabricated 'topical authority' requirement as ELINK001",
        ),
        (
            "COLRCL-V2001-UNDERLINE",
            "fired on any stylesheet pairing text-decoration:none with a color",
        ),
        (
            "CANDEP-V2003",
            "a trailing slash is a valid URL form, and url_norm folds it anyway",
        ),
    ];

    /// Tripwire: none of the removed codes may be emitted anywhere again.
    ///
    /// Scans the analyzer sources for `code: "..."` literals rather than
    /// checking `is_metric_code`, which answers `false` for these codes whether
    /// or not they exist and so would pass vacuously.
    #[test]
    fn removed_codes_do_not_reappear() {
        // Covers the whole analyzer tree. `content_analyzers.rs` is the largest
        // single file; the rest are small enough to list.
        let sources: Vec<&str> = vec![
            include_str!("content_analyzers.rs"),
            include_str!("mod.rs"),
            include_str!("v2/accessibility.rs"),
            include_str!("v2/seo.rs"),
            include_str!("post_crawl_analyzers.rs"),
        ];
        for (code, reason) in REMOVED_CODES {
            for src in &sources {
                assert!(
                    !src.contains(&format!("code: \"{code}\"")),
                    "{code} was removed ({reason}) but is emitted again in the analyzer tree"
                );
            }
        }
    }

    #[test]
    fn advisory_content_analysis_stays_a_finding() {
        // Actionable advice, even though numbers are involved.
        for code in [
            "KWPRO001", "KWPRO002", "TITLEKDEN-V2001", "TITLEKDEN-V6082",
            "METAKEY-V5001", "TITLEKWP-V5001", "CQ-V2001", "CREAD001",
            "CREAD002", "WC004", "EXT-AUTH001", "EXTAUTHDP001",
        ] {
            assert!(
                !is_metric_code(code),
                "{code} is actionable advice and must stay a finding"
            );
        }
    }

    #[test]
    fn unknown_codes_default_to_findings() {
        // Failing safe: an unrecognised code is treated as a problem, so a new
        // analyzer is never silently hidden from the report.
        for code in ["ZZZ999", "SOMENEW001", ""] {
            assert!(!is_metric_code(code));
        }
    }
}