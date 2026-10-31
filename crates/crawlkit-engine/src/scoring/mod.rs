//! Weighted scoring engine (GAP_CLOSURE_PLAN §G2).
//!
//! Turns per-page analyzer findings into deterministic scores:
//!
//! - [`Weights::page_score`] — `100 × Π (1 − wᵢ)` over the page's findings.
//!   Multiplicative so one catastrophic issue (weight `1.0`) dominates the
//!   result, and monotone: removing a finding never lowers the score.
//! - [`Weights::category_scores`] — the same formula per
//!   [`Category`], in a `BTreeMap` for deterministic iteration.
//! - [`site_score`] — page-count weighted mean of per-page scores, rounded to
//!   two fixed decimals (deterministic; deliberately not traffic-weighted).
//! - [`Weights::prioritized_fixes`] — findings ranked by
//!   `weight × pages_affected` descending, ties broken by rule id ascending.
//!
//! # Determinism rails (ITER-09)
//!
//! Scores must be reproducible bit-for-bit for identical crawls. Float
//! summation/multiplication order affects the low bits, so all reductions
//! sort their operands (`total_cmp`, descending) before folding; category
//! maps use `BTreeMap` ordering; fix ranking uses a total order. Weight
//! tables are versioned — see [`weights::SCHEMA_VERSION`].
//!
//! # Examples
//!
//! ```
//! use crawlkit_engine::scoring::{site_score, FindingRef, Weights};
//!
//! let weights = Weights::embedded().unwrap();
//! let findings = [FindingRef::new("SEC001"), FindingRef::new("META001")];
//! let page = weights.page_score(&findings);
//! assert!((0.0..=100.0).contains(&page));
//!
//! let site = site_score(&[("a".to_string(), page), ("b".to_string(), 100.0)]);
//! assert!((0.0..=100.0).contains(&site));
//! ```

mod weights;

use std::collections::{BTreeMap, BTreeSet};

pub use weights::{ResolvedWeight, ScoringError, WeightedRule, Weights};

/// Scoring category for a rule id (§G2: seo/performance/accessibility/
/// security/best-practices, per competitor reporting convention).
///
/// The set is closed so configuration typos fail at load time; declaration
/// order defines the deterministic `BTreeMap` iteration order.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum Category {
    /// Search-engine visibility issues (titles, meta, links, canonicals).
    Seo,
    /// Speed and Core Web Vitals issues.
    Performance,
    /// Accessibility issues (ARIA, contrast, landmarks).
    Accessibility,
    /// Security issues (headers, TLS, mixed content).
    Security,
    /// General best-practice issues (mobile, charset, deprecated markup).
    BestPractices,
}

impl Category {
    /// Canonical kebab-case name used in `scoring.toml`, reports, and the
    /// score history schema (§G6).
    pub fn as_str(self) -> &'static str {
        match self {
            Category::Seo => "seo",
            Category::Performance => "performance",
            Category::Accessibility => "accessibility",
            Category::Security => "security",
            Category::BestPractices => "best-practices",
        }
    }
}

impl std::fmt::Display for Category {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Minimal view of a finding for scoring: the machine-readable rule id
/// (e.g. `"SEC001"`). Deliberately narrow so scoring never depends on the
/// full [`crawlkit_types::Finding`] payload and stays usable on aggregated
/// or streamed data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FindingRef<'a> {
    /// Machine-readable issue code the weight table is keyed on.
    pub rule_id: &'a str,
}

impl<'a> FindingRef<'a> {
    /// Wrap a rule id.
    pub const fn new(rule_id: &'a str) -> Self {
        Self { rule_id }
    }
}

impl<'a> From<&'a crawlkit_types::Finding> for FindingRef<'a> {
    fn from(finding: &'a crawlkit_types::Finding) -> Self {
        Self {
            rule_id: &finding.code,
        }
    }
}

/// One prioritized fix recommendation (§G2): a distinct rule id with its
/// resolved category/weight and the affected-page count, ranked by impact.
#[derive(Debug, Clone, PartialEq)]
pub struct FixRec {
    /// Machine-readable issue code.
    pub rule_id: String,
    /// Category the rule id resolves to.
    pub category: Category,
    /// Penalty weight in `[0, 1]` (`0.0` when unmatched by the table).
    pub weight: f64,
    /// Number of pages affected (caller-supplied; `0` when unknown).
    pub pages_affected: usize,
    /// Ranking key: `weight × pages_affected`.
    pub impact: f64,
}

/// Site-wide score: the page-count weighted mean of per-page scores, rounded
/// to two fixed decimals (§G2 — deterministic, deliberately not
/// traffic-weighted).
///
/// Inputs are clamped to `[0, 100]` so stray out-of-range page scores cannot
/// push the site score out of bounds. An empty page list yields `100.0`
/// (vacuously: every crawled page is clean). Page scores are summed in
/// sorted order so shuffled page lists produce identical results bit-for-bit.
pub fn site_score(pages: &[(String, f64)]) -> f64 {
    if pages.is_empty() {
        return 100.0;
    }
    let mut scores: Vec<f64> = pages.iter().map(|(_, s)| s.clamp(0.0, 100.0)).collect();
    scores.sort_by(|a, b| b.total_cmp(a));
    let mean: f64 = scores.iter().sum::<f64>() / scores.len() as f64;
    (mean * 100.0).round() / 100.0
}

/// Sort weights descending (`total_cmp`: NaN-safe) and fold the product, so
/// the result is order-independent bit-for-bit regardless of input order.
fn penalty_product(mut weights: Vec<f64>) -> f64 {
    weights.sort_by(|a, b| b.total_cmp(a));
    weights.into_iter().fold(1.0, |acc, w| acc * (1.0 - w))
}

impl Weights {
    /// Page score in `[0, 100]`: `100 × Π (1 − wᵢ)` over the page's
    /// findings. An empty finding list yields `100.0`.
    pub fn page_score(&self, findings: &[FindingRef<'_>]) -> f64 {
        let penalties = findings
            .iter()
            .map(|f| self.resolve(f.rule_id).weight)
            .collect::<Vec<_>>();
        100.0 * penalty_product(penalties)
    }

    /// Per-category page scores in `[0, 100]` for every category that has at
    /// least one finding on the page. `BTreeMap` iteration is deterministic.
    pub fn category_scores(&self, findings: &[FindingRef<'_>]) -> BTreeMap<Category, f64> {
        let mut by_category: BTreeMap<Category, Vec<f64>> = BTreeMap::new();
        for finding in findings {
            let resolved = self.resolve(finding.rule_id);
            by_category
                .entry(resolved.category)
                .or_default()
                .push(resolved.weight);
        }
        by_category
            .into_iter()
            .map(|(category, weights)| (category, 100.0 * penalty_product(weights)))
            .collect()
    }

    /// Findings ranked as fix recommendations (§G2): sorted by
    /// `weight × pages_affected` descending, ties broken by rule id
    /// ascending. Findings are deduplicated by rule id (one fix per rule);
    /// rule ids absent from `pages_affected` count as affecting `0` pages.
    pub fn prioritized_fixes(
        &self,
        findings: &[FindingRef<'_>],
        pages_affected: &BTreeMap<String, usize>,
    ) -> Vec<FixRec> {
        // BTreeSet deduplicates and seeds rule_id-asc order.
        let rule_ids: BTreeSet<&str> = findings.iter().map(|f| f.rule_id).collect();
        let mut fixes: Vec<FixRec> = rule_ids
            .into_iter()
            .map(|rule_id| {
                let resolved = self.resolve(rule_id);
                let pages = pages_affected.get(rule_id).copied().unwrap_or(0);
                let impact = resolved.weight * pages as f64;
                FixRec {
                    rule_id: rule_id.to_owned(),
                    category: resolved.category,
                    weight: resolved.weight,
                    pages_affected: pages,
                    impact,
                }
            })
            .collect();
        // Total order (impact desc, rule id asc): deterministic ranking.
        fixes.sort_by(|a, b| {
            b.impact
                .total_cmp(&a.impact)
                .then(a.rule_id.cmp(&b.rule_id))
        });
        fixes
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    /// Small test table: SEC* = 0.5 security, A* = 0.25 accessibility,
    /// catch-all = 0.125 seo.
    fn table() -> Weights {
        Weights::from_toml(
            "schema_version = 1\n\
             default_category = \"seo\"\n\
             [rules]\n\
             \"SEC*\" = { category = \"security\", weight = 0.5 }\n\
             \"A*\" = { category = \"accessibility\", weight = 0.25 }\n\
             \"*\" = { weight = 0.125 }\n",
        )
        .unwrap()
    }

    fn refs<'a>(ids: &[&'a str]) -> Vec<FindingRef<'a>> {
        ids.iter().map(|id| FindingRef::new(id)).collect()
    }

    // --- page_score ---

    #[test]
    fn empty_findings_score_100() {
        assert_eq!(table().page_score(&[]), 100.0);
        assert_eq!(Weights::default().page_score(&[]), 100.0);
    }

    #[test]
    fn score_multiplicative_formula() {
        // 100 × (1−0.5) × (1−0.25) × (1−0.125) = 32.8125
        let f = refs(&["SEC001", "A001", "OTHER001"]);
        assert!((table().page_score(&f) - 32.8125).abs() < 1e-12);
    }

    #[test]
    fn score_always_within_bounds() {
        let w = Weights::embedded().unwrap();
        let cases: Vec<Vec<FindingRef<'_>>> = vec![
            refs(&[]),
            refs(&["SEC001"]),
            refs(&["SEC001", "A001", "META001", "CWV001", "MOB001"]),
            refs(&["SEC001"; 50]),
            refs(&["PLUGIN-UNKNOWN-1", "PLUGIN-UNKNOWN-2"]),
        ];
        for f in &cases {
            let s = w.page_score(f);
            assert!((0.0..=100.0).contains(&s), "score {s} out of range");
        }
    }

    #[test]
    fn weight_one_finding_zeroes_score() {
        let w =
            Weights::from_toml("[rules]\n\"FATAL*\" = { category = \"security\", weight = 1.0 }\n")
                .unwrap();
        let f = refs(&["FATAL001", "SEC001"]);
        assert_eq!(w.page_score(&f), 0.0);
        // Without the fatal finding the score recovers above zero.
        assert!(w.page_score(&refs(&["SEC001"])) > 0.0);
    }

    // --- monotonicity ---

    #[test]
    fn monotonicity_removing_any_finding_never_lowers_score() {
        let w = table();
        let full = refs(&["SEC001", "SEC002", "A001", "A002", "OTHER001"]);
        let base = w.page_score(&full);
        for i in 0..full.len() {
            let mut reduced = full.clone();
            reduced.remove(i);
            let s = w.page_score(&reduced);
            assert!(s >= base, "removing index {i} lowered score: {s} < {base}");
        }
    }

    #[test]
    fn monotonicity_nested_subsets_never_lowers_score() {
        let w = table();
        let mut full = refs(&["SEC001", "A001", "OTHER001", "OTHER002"]);
        let mut previous = w.page_score(&full);
        // Peel findings off one at a time; the score must be non-decreasing.
        while !full.is_empty() {
            full.pop();
            let s = w.page_score(&full);
            assert!(s >= previous, "subset score {s} below previous {previous}");
            previous = s;
        }
        assert_eq!(previous, 100.0);
    }

    #[test]
    fn monotonicity_zero_weight_finding_keeps_score_equal() {
        let w =
            Weights::from_toml("[rules]\n\"SEC*\" = { category = \"security\", weight = 0.5 }\n")
                .unwrap();
        let with = refs(&["SEC001", "UNMAPPED001"]);
        let without = refs(&["SEC001"]);
        // Unmapped findings carry weight 0.0: removal keeps the score equal.
        assert_eq!(w.page_score(&with), w.page_score(&without));
    }

    #[test]
    fn adding_finding_never_raises_score() {
        let w = Weights::embedded().unwrap();
        let ids = ["SEC001", "A001", "META001", "CWV001", "MOB001", "CACHE001"];
        let mut current = Vec::new();
        let mut previous = w.page_score(&current);
        for id in ids {
            current.push(FindingRef::new(id));
            let s = w.page_score(&current);
            assert!(s <= previous, "adding {id} raised score: {s} > {previous}");
            previous = s;
        }
    }

    // --- order independence ---

    #[test]
    fn order_independence_shuffled_lists_bit_identical() {
        let w = Weights::embedded().unwrap();
        let base = refs(&[
            "SEC001", "A001", "META001", "CWV001", "MOB001", "CACHE001", "SRI001",
        ]);
        let expected = w.page_score(&base).to_bits();

        let mut shuffled = base.clone();
        for _ in 0..shuffled.len() {
            shuffled.rotate_left(1);
            assert_eq!(w.page_score(&shuffled).to_bits(), expected, "rotation");
        }
        let mut reversed = base.clone();
        reversed.reverse();
        assert_eq!(w.page_score(&reversed).to_bits(), expected, "reversed");
        let mut swapped = base.clone();
        swapped.swap(0, 4);
        swapped.swap(1, 5);
        assert_eq!(w.page_score(&swapped).to_bits(), expected, "swapped");
    }

    // --- category_scores ---

    #[test]
    fn category_scores_group_findings_and_stay_in_bounds() {
        let w = table();
        let scores = w.category_scores(&refs(&["SEC001", "SEC002", "A001", "OTHER001"]));
        assert_eq!(scores.len(), 3);
        assert!((scores[&Category::Security] - 25.0).abs() < 1e-12);
        assert!((scores[&Category::Accessibility] - 75.0).abs() < 1e-12);
        assert!((scores[&Category::Seo] - 87.5).abs() < 1e-12);
        for s in scores.values() {
            assert!((0.0..=100.0).contains(s));
        }
    }

    #[test]
    fn category_scores_deterministic_key_order() {
        let w = table();
        let scores = w.category_scores(&refs(&["OTHER001", "A001", "SEC001"]));
        let keys: Vec<Category> = scores.keys().copied().collect();
        // Declaration order: Seo < Accessibility < Security (BTreeMap).
        assert_eq!(
            keys,
            vec![Category::Seo, Category::Accessibility, Category::Security]
        );
    }

    // --- site_score ---

    #[test]
    fn site_score_is_mean_rounded_to_two_decimals() {
        let pages = vec![("a".to_string(), 80.0), ("b".to_string(), 90.0)];
        assert!((site_score(&pages) - 85.0).abs() < f64::EPSILON);

        let thirds = vec![
            ("a".to_string(), 33.0),
            ("b".to_string(), 34.0),
            ("c".to_string(), 33.0),
        ];
        assert!((site_score(&thirds) - 33.33).abs() < 1e-9);

        let clamped = vec![("a".to_string(), 150.0), ("b".to_string(), 100.0)];
        assert_eq!(site_score(&clamped), 100.0);
    }

    #[test]
    fn site_score_empty_is_100() {
        assert_eq!(site_score(&[]), 100.0);
    }

    #[test]
    fn site_score_order_independent_bit_identical() {
        let mut pages = vec![
            ("a".to_string(), 100.0),
            ("b".to_string(), 87.5),
            ("c".to_string(), 33.333333),
            ("d".to_string(), 66.666667),
        ];
        let expected = site_score(&pages).to_bits();
        for _ in 0..pages.len() {
            pages.rotate_left(1);
            assert_eq!(site_score(&pages).to_bits(), expected);
        }
        pages.reverse();
        assert_eq!(site_score(&pages).to_bits(), expected);
    }

    // --- prioritized_fixes ---

    #[test]
    fn prioritized_fixes_sorted_by_impact_desc() {
        let w = table();
        let findings = refs(&["SEC001", "A001", "OTHER001"]);
        let mut affected = BTreeMap::new();
        affected.insert("SEC001".to_string(), 10usize); // 0.5 × 10 = 5.0
        affected.insert("A001".to_string(), 4usize); // 0.25 × 4 = 1.0
        affected.insert("OTHER001".to_string(), 100usize); // 0.125 × 100 = 12.5

        let fixes = w.prioritized_fixes(&findings, &affected);
        let ids: Vec<&str> = fixes.iter().map(|f| f.rule_id.as_str()).collect();
        assert_eq!(ids, vec!["OTHER001", "SEC001", "A001"]);
        assert!((fixes[0].impact - 12.5).abs() < 1e-9);
        assert_eq!(fixes[0].category, Category::Seo);
        assert_eq!(fixes[0].pages_affected, 100);
    }

    #[test]
    fn prioritized_fixes_tie_breaks_by_rule_id_asc() {
        let w = table();
        // Same impact (0.5 × 2 = 1.0) for two security rules.
        let findings = refs(&["SEC002", "SEC001"]);
        let mut affected = BTreeMap::new();
        affected.insert("SEC001".to_string(), 2usize);
        affected.insert("SEC002".to_string(), 2usize);

        let fixes = w.prioritized_fixes(&findings, &affected);
        assert_eq!(fixes.len(), 2);
        assert_eq!(fixes[0].rule_id, "SEC001");
        assert_eq!(fixes[1].rule_id, "SEC002");
        assert!(fixes[0].impact.total_cmp(&fixes[1].impact).is_eq());
    }

    #[test]
    fn prioritized_fixes_dedups_and_defaults_missing_counts_to_zero() {
        let w = table();
        // Duplicated rule id across pages collapses to one record.
        let findings = refs(&["SEC001", "SEC001", "UNMAPPED001"]);
        let mut affected = BTreeMap::new();
        affected.insert("SEC001".to_string(), 3usize);

        let fixes = w.prioritized_fixes(&findings, &affected);
        assert_eq!(fixes.len(), 2);
        // SEC001 (impact 1.5) first; UNMAPPED001 has weight 0.0 and no
        // count entry, so it lands last with impact 0.
        assert_eq!(fixes[0].rule_id, "SEC001");
        assert_eq!(fixes[1].rule_id, "UNMAPPED001");
        assert_eq!(fixes[1].pages_affected, 0);
        assert_eq!(fixes[1].impact, 0.0);
        // Equal-impact (both 0.0) fallback still deterministic: rule id asc.
        let none: BTreeMap<String, usize> = BTreeMap::new();
        let fallback = w.prioritized_fixes(&refs(&["B2", "A1", "C3"]), &none);
        let ids: Vec<&str> = fallback.iter().map(|f| f.rule_id.as_str()).collect();
        assert_eq!(ids, vec!["A1", "B2", "C3"]);
    }

    // --- Category ---

    #[test]
    fn category_names_roundtrip() {
        for (cat, name) in [
            (Category::Seo, "seo"),
            (Category::Performance, "performance"),
            (Category::Accessibility, "accessibility"),
            (Category::Security, "security"),
            (Category::BestPractices, "best-practices"),
        ] {
            assert_eq!(cat.as_str(), name);
            assert_eq!(cat.to_string(), name);
            let parsed: Category = serde_json::from_str(&format!("\"{name}\"")).unwrap();
            assert_eq!(parsed, cat);
        }
    }
}
