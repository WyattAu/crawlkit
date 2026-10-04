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

use crate::types::{Finding, Severity};

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
                && core[1..].chars().all(|c| c.is_ascii_digit()))
            // "schema", "structured", "data" name the technology being checked,
            // not the defect. Without this, "Article schema missing headline"
            // and "Article missing headline" are two signatures and the same
            // missing `headline` is reported under two codes - on gov.uk that
            // alone tripled a single real issue into 117 findings.
            || matches!(core, "schema" | "structured");
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

/// Identity of a defect: the page and a defect key.
///
/// The key comes from [`defect_family::defect_key`], which resolves curated
/// code families ("Missing main landmark" / "Missing main landmark region" /
/// "Page missing main landmark" are one defect) and otherwise falls back to the
/// normalized title. `category` is deliberately **not** part of the key:
/// "Multiple H1 headings" arrives as `accessibility` (A11Y004), `content`
/// (CDEPTH003) and `seo` (HEAD003), and the category is metadata about a defect
/// rather than part of its identity.
type Key = (String, String);

/// Defect identity for one finding: the page plus its family-or-title key.
fn defect_key_of(f: &Finding) -> Key {
    (
        f.url.clone(),
        crate::analyzers::defect_family::defect_key(&f.code, &defect_signature(&f.title)),
    )
}

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

    // The most severe member represents the family, and *it* is what survives —
    // code, title, description and severity all come from one finding.
    //
    // The previous rule kept the first finding positionally and copied a later,
    // more severe member's severity onto it. That produced a chimera: a finding
    // whose surviving code declared `Info` shipped at `warning`, because the
    // severity came from a sibling that had been discarded. Live output showed
    // exactly this — `MDESC-PX002` (declared `Info`) reported as `warning`
    // because its family also contains `META005` (declared `Warning`).
    //
    // Keeping severity at the family maximum is the right call on its own: when
    // several analyzers independently assess one defect and disagree, the most
    // severe credible assessment should stand. 77 of 202 curated families contain
    // members that disagree, so this is the common case, not an edge case. What
    // was wrong was decoupling it from the code that earned it.
    //
    // Every member's code is recorded, including those whose finding is
    // suppressed, so a user filtering on any of the codes still surfaces the
    // defect.
    let mut representative: HashMap<Key, usize> = HashMap::new();
    let mut aliases: AliasMap = HashMap::new();
    let mut out: Vec<Finding> = Vec::with_capacity(findings.len());

    for f in findings {
        let key = defect_key_of(&f);
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
                // Replace the whole representative rather than copying severity
                // across. Strictly-more-severe keeps this order-independent: the
                // maximum survives regardless of the order members arrive in, and
                // ties are broken by the caller's (code, url) sort.
                if severity_rank(f.severity) < severity_rank(out[idx].severity) {
                    out[idx] = f;
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
        let key = defect_key_of(f);
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
    use crate::types::IssueCategory;

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

    /// The real `script-src 'unsafe-inline'` cluster: three codes, one defect.
    ///
    /// `CSPDIR002` is deliberately absent — it reports `style-src`, a different
    /// defect, and the family table pins it there regardless of the title it is
    /// handed. Pairing it here with a script-src title was an artifact of the
    /// old title-only keying and no longer describes any real page.
    #[test]
    fn collapses_the_real_csp_cluster() {
        let findings = vec![
            f("https://e.com/", "CSP001", "CSP script-src allows unsafe-inline", Severity::Warning),
            f("https://e.com/", "CSPSS-V2002", "CSP script-src allows unsafe-inline (deep)", Severity::Warning),
            f("https://e.com/", "CSPSSRC-V5001", "CSP script-src allows unsafe-inline", Severity::Warning),
        ];
        let out = collapse_duplicates(findings);
        assert_eq!(out.len(), 1, "three identical reports must collapse to one");
        assert!(out[0].description.contains("also reported by"));
    }

    #[test]
    fn script_src_and_style_src_never_merge_despite_similar_titles() {
        let findings = vec![
            f("https://e.com/", "CSP001", "CSP script-src allows unsafe-inline", Severity::Warning),
            f("https://e.com/", "CSPSTY-V2002", "CSP style-src allows unsafe-inline", Severity::Warning),
        ];
        let out = collapse_duplicates(findings);
        assert_eq!(
            out.len(),
            2,
            "different CSP directives are different defects: {:?}",
            out.iter().map(|x| x.code.clone()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn same_defect_under_different_categories_is_collapsed() {
        // Regression: "Multiple H1 headings" is reported as accessibility,
        // content and seo by three analyzers. Keying on category let all three
        // survive, so the defect still counted three times.
        let mk = |code: &str, cat: IssueCategory| Finding {
            severity: Severity::Warning,
            category: cat,
            code: code.to_string(),
            title: "Multiple H1 headings".to_string(),
            description: "desc".into(),
            url: "https://e.com/".to_string(),
            recommendation: "rec".into(),
        };
        let findings = vec![
            mk("A11Y004", IssueCategory::Accessibility),
            mk("CDEPTH003", IssueCategory::Content),
            mk("HEAD003", IssueCategory::Seo),
        ];
        let out = collapse_duplicates(findings);
        assert_eq!(
            out.len(),
            1,
            "one defect under three categories must collapse to one"
        );
    }

    #[test]
    fn technology_words_do_not_split_one_defect() {
        // gov.uk: one Article `headline` gap, reported by ART001 and ART-HL001
        // (and aliased ARTHL-V2001) purely because one title says "schema" and
        // the other does not.
        let findings = vec![
            f("https://e.com/", "ART001", "Article schema missing headline", Severity::Error),
            f("https://e.com/", "ART-HL001", "Article missing headline", Severity::Error),
        ];
        let out = collapse_duplicates(findings);
        assert_eq!(out.len(), 1, "one missing headline must report once");
    }

    #[test]
    fn stripping_schema_does_not_merge_different_defects() {
        assert_ne!(
            defect_signature("Article schema missing headline"),
            defect_signature("Article schema missing author")
        );
        assert_ne!(
            defect_signature("Product schema missing price"),
            defect_signature("Product schema missing availability")
        );
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
            f("https://e.com/", "CSPSSRC-V5001", "CSP script-src allows unsafe-inline", Severity::Warning),
            f("https://e.com/", "CSP001", "CSP script-src allows unsafe-inline", Severity::Warning),
        ];
        let out = collapse_duplicates(findings);
        // Two `CSP001` reports are kept (same code = two separate occurrences),
        // and the single `CSPSSRC-V5001` cross-code report is folded in.
        assert_eq!(out.len(), 2);
        let aliases: Vec<&str> = out
            .iter()
            .filter_map(|f| f.description.split("also reported by: ").nth(1))
            .collect();
        assert!(!aliases.is_empty(), "cross-code report must be recorded");
        for alias in &aliases {
            assert_eq!(alias.trim(), "CSPSSRC-V5001)");
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

    /// The invariant the previous rule broke: a surviving finding reports the
    /// severity its *own* code declared.
    ///
    /// `MDESC-PX002` declares `Info` and `META005` declares `Warning`; both are
    /// the "meta description too short" family. Under positional-wins the first
    /// survived with the second's severity copied onto it, so live output showed
    /// an `Info` code shipping as `warning`.
    #[test]
    fn surviving_finding_keeps_its_own_severity() {
        let findings = vec![
            f("https://e.com/", "MDESC-PX002", "Meta description too short", Severity::Info),
            f("https://e.com/", "META005", "Meta description too short", Severity::Warning),
        ];
        let out = collapse_duplicates(findings);
        assert_eq!(out.len(), 1, "one defect must survive as one finding");
        // The most severe member represents the family, and it is what survives.
        assert_eq!(out[0].code, "META005");
        assert_eq!(out[0].severity, Severity::Warning);
    }

    /// The same result regardless of the order members arrive in.
    ///
    /// `analyze` sorts by `(code, url)` before calling, but this must not depend
    /// on it: a rule that only holds for one input ordering is a rule that will
    /// drift the day the sort changes.
    #[test]
    fn representative_is_independent_of_input_order() {
        let a = f("https://e.com/", "MDESC-PX002", "Meta description too short", Severity::Info);
        let b = f("https://e.com/", "META005", "Meta description too short", Severity::Warning);
        for findings in [vec![a.clone(), b.clone()], vec![b, a]] {
            let out = collapse_duplicates(findings);
            assert_eq!(out.len(), 1);
            assert_eq!(out[0].code, "META005");
            assert_eq!(out[0].severity, Severity::Warning);
        }
    }

    /// Equal severities keep the first, which the caller's `(code, url)` sort
    /// makes deterministic.
    #[test]
    fn equal_severity_keeps_first_and_is_stable() {
        let findings = vec![
            f("https://e.com/", "AAA", "Same defect", Severity::Warning),
            f("https://e.com/", "BBB", "Same defect", Severity::Warning),
        ];
        let out = collapse_duplicates(findings.clone());
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].code, "AAA", "ties go to the first in sorted order");
        let again = collapse_duplicates(findings);
        assert_eq!(out[0].code, again[0].code);
    }

    /// Family severity is still the maximum — the fix changed which finding
    /// represents the family, not how loud the family is.
    #[test]
    fn family_severity_remains_the_maximum() {
        let findings = vec![
            f("https://e.com/", "CCC", "Same defect", Severity::Info),
            f("https://e.com/", "DDD", "Same defect", Severity::Error),
            f("https://e.com/", "EEE", "Same defect", Severity::Warning),
        ];
        let out = collapse_duplicates(findings);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].severity, Severity::Error);
    }

    /// The suppressed codes stay reachable, so filtering on any member's code
    /// still surfaces the defect.
    #[test]
    fn suppressed_codes_are_recorded_in_the_description() {
        let findings = vec![
            f("https://e.com/", "MDESC-PX002", "Meta description too short", Severity::Info),
            f("https://e.com/", "META005", "Meta description too short", Severity::Warning),
        ];
        let out = collapse_duplicates(findings);
        assert!(
            out[0].description.contains("MDESC-PX002"),
            "the discarded member's code must remain visible: {}",
            out[0].description
        );
    }
}