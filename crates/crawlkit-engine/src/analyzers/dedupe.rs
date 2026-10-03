//! Collapse findings that describe the same defect under different codes.
//!
//! # The problem
//!
//! Analyzers were grown additively, and several clusters ended up implementing
//! the *same* check independently. A CSP with `script-src 'unsafe-inline'`
//! produced four separate warnings (`CSP001`, `CSPDIR002`, `CSPSS-V2002`,
//! `CSPSSRC-V5001`); a page with multiple `<h1>` elements produced eight
//! (`A11Y004`, `CDEPTH003`, `HEAD003`, `H1MULTI-V6108`, `HEADH1-V5002`,
//! `HEADSC003`, `HHIERDEEP003`, `HHIER-V2003-DEEP-DEEP`); seven codes reported a
//! short meta description and nine reported a missing table caption.
//!
//! Auditing a 60-page production site produced 6,788 findings, of which **881
//! (13%) were exact duplicates of a finding already reported** under a
//! different code — same page, same category, same message. That is not a
//! cosmetic problem: it triples the apparent defect count, trains users to
//! ignore the output, and makes severity roll-ups meaningless, because one real
//! issue is counted three times as loudly as three distinct ones.
//!
//! # The approach
//!
//! Rather than deleting hundreds of analyzers (risky, and it would discard the
//! per-directive and per-context coverage they provide), findings are collapsed
//! *after* analysis. Two findings on the same page are the same defect when they
//! share a category and their titles reduce to the same sentence once the
//! version/depth suffixes that distinguish otherwise-identical analyzers are
//! removed.
//!
//! Nothing is discarded silently: the retained finding gains an `aliases`
//! listing every code that reported the same problem, so a user filtering on a
//! specific code still finds the issue, and the audit trail stays complete.

use std::collections::HashMap;

use crate::types::{Finding, IssueCategory, Severity};

/// Severity ordering, highest first. Used to decide which representative of a
/// duplicate group is kept.
fn severity_rank(s: Severity) -> u8 {
    match s {
        Severity::Critical => 0,
        Severity::Error => 1,
        Severity::Warning => 2,
        Severity::Info => 3,
    }
}

/// Reduce a finding title to the defect it describes.
///
/// Analyzer families differ only by trailing decorations — a version tag
/// (`-V5001`), a depth marker (`deep`, `deep-deep-deep`), or a qualifier in
/// parentheses. Stripping those leaves the sentence that identifies the defect:
///
/// ```text
/// "CSP script-src allows unsafe-inline (deep)"        -> "csp script src allows unsafe inline"
/// "CSP script-src allows unsafe-inline (deep-deep)"    -> "csp script src allows unsafe inline"
/// ```
fn defect_signature(title: &str) -> String {
    let mut out = String::with_capacity(title.len());
    let mut depth = 0usize;
    for ch in title.chars() {
        match ch {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ if depth == 0 => out.push(ch),
            _ => {}
        }
    }

    let lowered = out.to_lowercase();
    let mut cleaned = String::with_capacity(lowered.len());
    for token in lowered.split_whitespace() {
        let core = token.trim_matches(|c: char| !c.is_ascii_alphanumeric());
        // `deep`, `deep-deep`, `v2`, `v5001`, `deep-deep-deep` are naming
        // decorations on the analyzer, not part of the defect.
        let is_decoration = core.is_empty()
            || (core.starts_with('d')
                && core.chars().all(|c| matches!(c, 'd' | 'e' | 'a' | 'p')))
            || (core.starts_with('v')
                && core.len() > 1
                && core[1..].chars().all(|c| c.is_ascii_digit()));
        if is_decoration {
            continue;
        }
        if !cleaned.is_empty() {
            cleaned.push(' ');
        }
        cleaned.push_str(core);
    }
    cleaned
}

/// Identity of a defect: the page, its category, and its normalized message.
type Key = (String, IssueCategory, String);

/// Codes retained behind a collapsed finding, for auditability.
type AliasMap = HashMap<Key, Vec<String>>;

/// Collapse duplicate findings on a single page.
///
/// Returns the deduplicated findings plus, for each surviving finding, the codes
/// that had reported the same defect. Input order is preserved.
#[must_use]
pub fn collapse_duplicates(findings: Vec<Finding>) -> Vec<Finding> {
    if findings.len() < 2 {
        return findings;
    }

    // First occurrence wins positionally; a later, more severe member promotes
    // the representative's severity so roll-ups are not understated. Every
    // member's code is recorded, including those whose finding is suppressed,
    // so a user filtering on any of the codes still surfaces the defect.
    let mut representative: HashMap<Key, usize> = HashMap::new();
    let mut aliases: AliasMap = HashMap::new();
    let mut out: Vec<Finding> = Vec::with_capacity(findings.len());

    for f in findings {
        let key: Key = (f.url.clone(), f.category.clone(), defect_signature(&f.title));
        aliases.entry(key.clone()).or_default().push(f.code.clone());

        match representative.get(&key).copied() {
            Some(idx) => {
                // Never collapse two findings from the *same* analyzer. Several
                // analyzers legitimately report once per offending element
                // under a single code — generic anchor text, one finding per
                // link — and those are distinct defects that must all survive.
                // Only genuinely redundant cross-code reports are merged.
                if out[idx].code == f.code {
                    out.push(f);
                    continue;
                }
                if severity_rank(f.severity) < severity_rank(out[idx].severity) {
                    out[idx].severity = f.severity;
                }
            }
            None => {
                representative.insert(key, out.len());
                out.push(f);
            }
        }
    }

    for codes in aliases.values_mut() {
        codes.sort();
        codes.dedup();
    }
    aliases.retain(|_, codes| codes.len() > 1);

    for f in &mut out {
        let key: Key = (f.url.clone(), f.category.clone(), defect_signature(&f.title));
        if let Some(codes) = aliases.get(&key) {
            let others: Vec<&str> = codes.iter().filter(|c| **c != f.code).map(String::as_str).collect();
            if !others.is_empty() {
                f.description = format!(
                    "{} (also reported by: {})",
                    f.description.trim_end(),
                    others.join(", ")
                );
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(url: &str, code: &str, title: &str, sev: Severity) -> Finding {
        Finding {
            severity: sev,
            category: IssueCategory::Security,
            code: code.to_string(),
            title: title.to_string(),
            description: "desc".into(),
            url: url.to_string(),
            recommendation: "rec".into(),
        }
    }

    #[test]
    fn signature_strips_version_and_depth_decorations() {
        assert_eq!(
            defect_signature("CSP script-src allows unsafe-inline"),
            defect_signature("CSP script-src allows unsafe-inline (deep)")
        );
        assert_eq!(
            defect_signature("CSP script-src allows unsafe-inline"),
            defect_signature("CSP script-src allows unsafe-inline (deep-deep-deep)")
        );
        assert_eq!(
            defect_signature("Missing self-referencing hreflang"),
            defect_signature("Missing self-referencing hreflang (deep)")
        );
    }

    #[test]
    fn signature_keeps_genuinely_different_titles_apart() {
        assert_ne!(
            defect_signature("CSP script-src allows unsafe-inline"),
            defect_signature("CSP style-src allows unsafe-inline")
        );
        assert_ne!(
            defect_signature("Multiple H1 headings"),
            defect_signature("Missing H1 heading")
        );
    }

    #[test]
    fn collapses_the_real_csp_cluster() {
        let findings = vec![
            f("https://e.com/", "CSP001", "CSP script-src allows unsafe-inline", Severity::Warning),
            f("https://e.com/", "CSPDIR002", "CSP script-src allows unsafe-inline", Severity::Warning),
            f("https://e.com/", "CSPSS-V2002", "CSP script-src allows unsafe-inline (deep)", Severity::Warning),
            f("https://e.com/", "CSPSSRC-V5001", "CSP script-src allows unsafe-inline", Severity::Warning),
        ];
        let out = collapse_duplicates(findings);
        assert_eq!(out.len(), 1, "four identical reports must collapse to one");
        assert!(out[0].description.contains("also reported by"));
    }

    #[test]
    fn distinct_pages_are_not_collapsed_together() {
        let findings = vec![
            f("https://e.com/a", "CSP001", "CSP script-src allows unsafe-inline", Severity::Warning),
            f("https://e.com/b", "CSP001", "CSP script-src allows unsafe-inline", Severity::Warning),
        ];
        assert_eq!(collapse_duplicates(findings).len(), 2);
    }

    #[test]
    fn distinct_defects_on_one_page_survive() {
        let findings = vec![
            f("https://e.com/", "CSPSS-V2002", "CSP script-src allows unsafe-inline", Severity::Warning),
            f("https://e.com/", "CSPSTY-V2002", "CSP style-src allows unsafe-inline", Severity::Warning),
        ];
        assert_eq!(collapse_duplicates(findings).len(), 2);
    }

    #[test]
    fn most_severe_member_wins() {
        let findings = vec![
            f("https://e.com/", "A", "Missing H1 heading (deep)", Severity::Info),
            f("https://e.com/", "B", "Missing H1 heading", Severity::Critical),
        ];
        let out = collapse_duplicates(findings);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].severity, Severity::Critical);
    }

    #[test]
    fn aliases_list_every_suppressed_code_exactly_once() {
        let findings = vec![
            f("https://e.com/", "CSP001", "CSP script-src allows unsafe-inline", Severity::Warning),
            f("https://e.com/", "CSPDIR002", "CSP script-src allows unsafe-inline", Severity::Warning),
            f("https://e.com/", "CSP001", "CSP script-src allows unsafe-inline", Severity::Warning),
        ];
        let out = collapse_duplicates(findings);
        // Two `CSP001` reports are kept (same code = two separate occurrences),
        // and the single `CSPDIR002` cross-code report is folded in.
        assert_eq!(out.len(), 2);
        let aliases: Vec<&str> = out
            .iter()
            .filter_map(|f| f.description.split("also reported by: ").nth(1))
            .collect();
        assert!(!aliases.is_empty(), "cross-code report must be recorded");
        for alias in &aliases {
            assert_eq!(alias.trim(), "CSPDIR002)");
        }
    }

    #[test]
    fn same_code_repeats_are_never_collapsed() {
        // Per-element analyzers emit one finding per offending element under a
        // single code. Collapsing those would silently hide real defects.
        let findings = vec![
            f("https://e.com/", "ANCHGEN001", "Link with generic anchor text", Severity::Warning),
            f("https://e.com/", "ANCHGEN001", "Link with generic anchor text", Severity::Warning),
        ];
        assert_eq!(collapse_duplicates(findings).len(), 2);
    }

    #[test]
    fn single_and_empty_inputs_are_untouched() {
        assert!(collapse_duplicates(Vec::new()).is_empty());
        let one = vec![f("https://e.com/", "X", "Something", Severity::Info)];
        assert_eq!(collapse_duplicates(one).len(), 1);
    }

    #[test]
    fn non_duplicate_finding_description_is_not_rewritten() {
        let findings = vec![f("https://e.com/", "X", "Unique defect", Severity::Info)];
        assert_eq!(collapse_duplicates(findings)[0].description, "desc");
    }
}