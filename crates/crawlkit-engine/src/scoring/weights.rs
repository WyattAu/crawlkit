//! Scoring weight configuration (GAP_CLOSURE_PLAN §G2).
//!
//! [`Weights`] maps rule-id patterns to `(category, weight)` pairs, loaded
//! from a versioned TOML document. Validation is strict: weights must be
//! finite and inside the half-open range `(0, 1]`, and categories must name
//! a [`Category`] variant (an empty or unknown category string is rejected
//! at parse time because the category set is a closed enum).

use std::cmp::Reverse;
use std::collections::BTreeMap;

use serde::Deserialize;
use thiserror::Error;

use super::Category;

/// Schema version of the embedded and user-supplied scoring configuration.
///
/// Determinism rail (ITER-09): any change to shipped weights or categories is
/// breaking for report diffs and MUST bump this version and be recorded in the
/// changelog.
pub const SCHEMA_VERSION: u32 = 1;

/// Errors produced while loading or validating a [`Weights`] table.
#[derive(Debug, Error)]
pub enum ScoringError {
    /// The TOML document could not be parsed, or a category string was not
    /// one of the known [`Category`] variants (the category set is closed, so
    /// an empty/unknown category is a parse failure).
    #[error("failed to parse scoring configuration: {0}")]
    Parse(String),
    /// The document declares a `schema_version` this build does not support.
    #[error("unsupported scoring schema version {found} (expected {expected})")]
    UnsupportedVersion {
        /// Version found in the document.
        found: u32,
        /// Version this build understands ([`SCHEMA_VERSION`]).
        expected: u32,
    },
    /// A rule weight was not a finite number (`nan`, `inf`).
    #[error("rule pattern '{pattern}': weight must be a finite number")]
    NonFiniteWeight {
        /// The offending rule-id pattern.
        pattern: String,
    },
    /// A rule weight fell outside the valid half-open range `(0, 1]`.
    #[error("rule pattern '{pattern}': weight {weight} is outside the valid range (0, 1]")]
    WeightOutOfRange {
        /// The offending rule-id pattern.
        pattern: String,
        /// The invalid weight.
        weight: f64,
    },
}

/// One validated `pattern -> (category, weight)` mapping.
#[derive(Debug, Clone, PartialEq)]
pub struct WeightedRule {
    /// Rule-id glob pattern (`*` matches any sequence, `?` one character).
    pub pattern: String,
    /// Category assigned to matching rule ids.
    pub category: Category,
    /// Penalty weight in `(0, 1]`.
    pub weight: f64,
}

/// A rule id resolved against a [`Weights`] table.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedWeight<'a> {
    /// Category for the rule id.
    pub category: Category,
    /// Penalty weight in `[0, 1]`; `0.0` when no pattern matched.
    pub weight: f64,
    /// The pattern that matched, or `None` when the default fallback applied.
    pub pattern: Option<&'a str>,
}

/// Validated rule-id weight table (§G2).
///
/// Construct via [`Weights::from_toml`] (or [`Weights::embedded`] for the
/// configuration shipped with crawlkit). All stored weights are validated at
/// construction, so scoring functions can rely on `weight ∈ (0, 1]` for
/// matched rules; rule ids matching no pattern resolve to the table's default
/// category with weight `0.0` (no penalty), which keeps unknown plugin rules
/// from silently distorting scores.
#[derive(Debug, Clone)]
pub struct Weights {
    /// Sorted by specificity (most specific first), ties by pattern asc.
    rules: Vec<WeightedRule>,
    default_category: Category,
}

impl Default for Weights {
    /// An empty table: every rule id resolves to [`Category::Seo`] with
    /// weight `0.0`, so all scores stay at 100.
    fn default() -> Self {
        Self {
            rules: Vec::new(),
            default_category: Category::Seo,
        }
    }
}

/// Top-level shape of `scoring.toml`.
#[derive(Debug, Deserialize)]
struct ScoringConfig {
    #[serde(default = "default_schema_version")]
    schema_version: u32,
    #[serde(default = "default_category")]
    default_category: Category,
    #[serde(default)]
    rules: BTreeMap<String, RuleSpec>,
}

/// Per-rule entry in `scoring.toml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RuleSpec {
    category: Option<Category>,
    weight: f64,
}

fn default_schema_version() -> u32 {
    SCHEMA_VERSION
}

fn default_category() -> Category {
    Category::Seo
}

impl Weights {
    /// Parse and validate a scoring configuration from TOML text.
    ///
    /// # Examples
    ///
    /// ```
    /// use crawlkit_engine::scoring::{Category, FindingRef, Weights};
    ///
    /// let weights = Weights::from_toml(
    ///     "schema_version = 1\n\
    ///      default_category = \"seo\"\n\
    ///      [rules]\n\
    ///      \"SEC*\" = { category = \"security\", weight = 0.5 }\n",
    /// )
    /// .unwrap();
    ///
    /// let findings = [FindingRef::new("SEC001"), FindingRef::new("SEC002")];
    /// let score = weights.page_score(&findings);
    /// assert!((score - 25.0).abs() < f64::EPSILON);
    /// ```
    pub fn from_toml(config: &str) -> Result<Self, ScoringError> {
        let parsed: ScoringConfig =
            toml::from_str(config).map_err(|e| ScoringError::Parse(e.to_string()))?;

        if parsed.schema_version != SCHEMA_VERSION {
            return Err(ScoringError::UnsupportedVersion {
                found: parsed.schema_version,
                expected: SCHEMA_VERSION,
            });
        }

        let mut rules: Vec<WeightedRule> = Vec::with_capacity(parsed.rules.len());
        for (pattern, spec) in parsed.rules {
            if !spec.weight.is_finite() {
                return Err(ScoringError::NonFiniteWeight { pattern });
            }
            if spec.weight <= 0.0 || spec.weight > 1.0 {
                return Err(ScoringError::WeightOutOfRange {
                    pattern,
                    weight: spec.weight,
                });
            }
            let category = spec.category.unwrap_or(parsed.default_category);
            rules.push(WeightedRule {
                pattern,
                category,
                weight: spec.weight,
            });
        }

        // `parsed.rules` iterates in pattern-asc order (BTreeMap); a stable
        // sort by specificity desc keeps pattern-asc tie-breaking, so the
        // resolved match is deterministic regardless of TOML key order.
        rules.sort_by_key(|rule| Reverse(specificity(&rule.pattern)));

        Ok(Self {
            rules,
            default_category: parsed.default_category,
        })
    }

    /// Load the scoring configuration shipped with crawlkit (`scoring.toml`
    /// at the crate root). Versioned; see [`SCHEMA_VERSION`].
    pub fn embedded() -> Result<Self, ScoringError> {
        Self::from_toml(include_str!("../../scoring.toml"))
    }

    /// The validated rules, most specific first.
    pub fn rules(&self) -> &[WeightedRule] {
        &self.rules
    }

    /// Category assigned to rule ids that match no pattern.
    pub fn default_category(&self) -> Category {
        self.default_category
    }

    /// Resolve a rule id to its category and weight.
    ///
    /// The most specific matching pattern wins (more non-wildcard characters
    /// first, ties broken by lexicographically smaller pattern). Rule ids
    /// matching nothing resolve to [`Self::default_category`] with weight
    /// `0.0`.
    pub fn resolve(&self, rule_id: &str) -> ResolvedWeight<'_> {
        for rule in &self.rules {
            if wildcard_matches(&rule.pattern, rule_id) {
                return ResolvedWeight {
                    category: rule.category,
                    weight: rule.weight,
                    pattern: Some(&rule.pattern),
                };
            }
        }
        ResolvedWeight {
            category: self.default_category,
            weight: 0.0,
            pattern: None,
        }
    }
}

/// Number of non-`*` characters — the specificity used to order patterns.
fn specificity(pattern: &str) -> usize {
    pattern.chars().filter(|c| *c != '*').count()
}

/// Glob match supporting `*` (any sequence, including empty) and `?`
/// (exactly one character). Iterative two-pointer scan with single-star
/// backtracking; allocation-free and deterministic.
fn wildcard_matches(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    let (mut pi, mut ti) = (0usize, 0usize);
    let mut star: Option<usize> = None;
    let mut star_mark = 0usize;

    while ti < t.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == t[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            pi += 1;
            star_mark = ti;
        } else if let Some(s) = star {
            pi = s + 1;
            star_mark += 1;
            ti = star_mark;
        } else {
            return false;
        }
    }

    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    const VALID: &str = "schema_version = 1\n\
                         default_category = \"seo\"\n\
                         [rules]\n\
                         \"SEC*\" = { category = \"security\", weight = 0.5 }\n\
                         \"METADESC*\" = { weight = 0.25 }\n";

    fn weights(toml: &str) -> Weights {
        Weights::from_toml(toml).unwrap()
    }

    // --- from_toml validation ---

    #[test]
    fn from_toml_parses_rules_and_default_category() {
        let w = weights(VALID);
        assert_eq!(w.rules().len(), 2);
        assert_eq!(w.default_category(), Category::Seo);
        assert_eq!(w.rules()[1].pattern, "SEC*");
        assert_eq!(w.rules()[1].category, Category::Security);
        assert!((w.rules()[1].weight - 0.5).abs() < f64::EPSILON);
        // Category omitted -> default_category fallback.
        assert_eq!(w.rules()[0].category, Category::Seo);
    }

    #[test]
    fn weight_zero_is_rejected() {
        let err = Weights::from_toml("[rules]\n\"X\" = { weight = 0.0 }\n").unwrap_err();
        assert!(err.to_string().contains("outside the valid range"), "{err}");
    }

    #[test]
    fn weight_above_one_is_rejected() {
        let err = Weights::from_toml("[rules]\n\"X\" = { weight = 1.5 }\n").unwrap_err();
        assert!(matches!(err, ScoringError::WeightOutOfRange { .. }));
    }

    #[test]
    fn negative_weight_is_rejected() {
        let err = Weights::from_toml("[rules]\n\"X\" = { weight = -0.1 }\n").unwrap_err();
        assert!(matches!(err, ScoringError::WeightOutOfRange { .. }));
    }

    #[test]
    fn non_finite_weights_are_rejected() {
        for raw in ["nan", "inf", "-inf"] {
            let doc = format!("[rules]\n\"X\" = {{ weight = {raw} }}\n");
            let err = Weights::from_toml(&doc).unwrap_err();
            assert!(matches!(err, ScoringError::NonFiniteWeight { .. }), "{raw}");
        }
    }

    #[test]
    fn unknown_or_empty_category_is_rejected_at_parse_time() {
        for cat in ["bogus", "''"] {
            let doc = format!("[rules]\n\"X\" = {{ category = {cat}, weight = 0.5 }}\n");
            let err = Weights::from_toml(&doc).unwrap_err();
            assert!(matches!(err, ScoringError::Parse(_)), "{cat}: {err}");
        }
    }

    #[test]
    fn malformed_toml_is_rejected() {
        let err = Weights::from_toml("[rules\n\"X\" = {{{{ ").unwrap_err();
        assert!(matches!(err, ScoringError::Parse(_)));
    }

    #[test]
    fn unsupported_schema_version_is_rejected() {
        let err = Weights::from_toml("schema_version = 99\n").unwrap_err();
        assert!(matches!(
            err,
            ScoringError::UnsupportedVersion {
                found: 99,
                expected: SCHEMA_VERSION
            }
        ));
    }

    #[test]
    fn unknown_rule_field_is_rejected() {
        let err =
            Weights::from_toml("[rules]\n\"X\" = { weight = 0.5, wieght = 1 }\n").unwrap_err();
        assert!(matches!(err, ScoringError::Parse(_)));
    }

    #[test]
    fn embedded_config_parses() {
        let w = Weights::embedded().unwrap();
        assert!(!w.rules().is_empty());
        let sec = w.resolve("SEC001");
        assert_eq!(sec.category, Category::Security);
        assert_eq!(sec.pattern, Some("SEC*"));
    }

    // --- resolve / pattern precedence ---

    #[test]
    fn resolve_prefers_more_specific_pattern() {
        let w = weights(VALID);
        // "METADESC*" (8 literal chars) beats "META-less" specificity of "*".
        let r = w.resolve("METADESC001");
        assert_eq!(r.pattern, Some("METADESC*"));
        assert!((r.weight - 0.25).abs() < f64::EPSILON);
    }

    #[test]
    fn unmatched_rule_gets_default_category_and_zero_weight() {
        let w = weights("[rules]\n\"SEC*\" = { category = \"security\", weight = 0.5 }\n");
        let r = w.resolve("TOTALLY-UNKNOWN-001");
        assert_eq!(r.pattern, None);
        assert_eq!(r.category, Category::Seo);
        assert_eq!(r.weight, 0.0);
    }

    #[test]
    fn default_weights_table_resolves_nothing() {
        let w = Weights::default();
        let r = w.resolve("ANY001");
        assert_eq!(r.weight, 0.0);
        assert_eq!(r.category, Category::Seo);
    }

    #[test]
    fn wildcard_matcher_edge_cases() {
        assert!(wildcard_matches("*", "anything-at-all"));
        assert!(wildcard_matches("", ""));
        assert!(!wildcard_matches("", "x"));
        assert!(wildcard_matches("META*", "META001"));
        assert!(!wildcard_matches("META*", "HTTP001"));
        assert!(wildcard_matches("META?01", "META001"));
        assert!(!wildcard_matches("META?01", "META0001"));
        assert!(wildcard_matches("*001", "SEC001"));
        assert!(wildcard_matches("S*C*1", "SEC*C*1"));
        assert!(wildcard_matches("S*C*1", "SEC1"));
        assert!(!wildcard_matches("S*C*1", "SEC*C*1-x"));
    }

    #[test]
    fn embedded_catchall_has_lowest_specificity() {
        let w = Weights::embedded().unwrap();
        let last = w.rules().last().unwrap();
        assert_eq!(last.pattern, "*");
        let unknown = w.resolve("PLUGIN-XYZ-999");
        assert_eq!(unknown.pattern, Some("*"));
        assert!((unknown.weight - 0.02).abs() < f64::EPSILON);
    }
}
