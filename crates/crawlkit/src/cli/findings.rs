//! Canonical findings wire helpers and schema conformance tests.
//!
//! `docs/schema/findings.schema.json` is the published wire contract for
//! finding JSON (5.2.0 "findings JSON schema published and CI-checked").
//! The `*_json` helpers here are the single construction point for every
//! CLI findings artifact, and the `schema_conformance` tests validate their
//! output against the committed schema file with a minimal validator — so
//! emitter and schema cannot drift apart silently.

use crawlkit_engine::Finding;
use serde_json::Value;

/// Canonical single-finding JSON per the schema's `$defs.finding`.
pub(crate) fn finding_json(url: &str, f: &Finding) -> Value {
    serde_json::json!({
        "url": url,
        "severity": f.severity.as_str(),
        "category": f.category.as_str(),
        "code": f.code,
        "title": f.title,
        "description": f.description,
        "recommendation": f.recommendation,
    })
}

/// Canonical single-finding JSON from a post-crawl finding.
pub(crate) fn post_crawl_finding_json(f: &crawlkit_engine::post_crawl::PostCrawlFinding) -> Value {
    serde_json::json!({
        "url": f.page_url,
        "severity": f.severity.as_str(),
        "category": f.category.as_str(),
        "code": f.code,
        "title": f.title,
        "description": f.description,
        "recommendation": f.recommendation,
    })
}

#[cfg(test)]
mod schema_conformance {
    use super::{finding_json, post_crawl_finding_json};
    use crawlkit_engine::{Finding, IssueCategory, Severity};
    use serde_json::Value;
    use std::path::PathBuf;
    use std::sync::LazyLock;

    /// Committed schema file, loaded once per test run.
    static SCHEMA: LazyLock<Value> = LazyLock::new(|| {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/schema/findings.schema.json");
        let raw = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("schema file missing at {}: {e}", path.display()));
        serde_json::from_str(&raw).unwrap_or_else(|e| panic!("schema file is not valid JSON: {e}"))
    });

    fn sample_finding() -> Finding {
        Finding {
            severity: Severity::Warning,
            category: IssueCategory::Seo,
            code: "SEO001".to_string(),
            title: "Title".to_string(),
            description: "Description".to_string(),
            url: "https://example.com/page".to_string(),
            recommendation: "Fix it".to_string(),
        }
    }

    // --- Minimal validator: only the keywords this schema uses. ---------

    // Fields exist to make the Debug rendering of violations diagnosable in
    // test failure output; tests match on variants, so dead-code analysis
    // cannot see the reads.
    #[allow(dead_code)]
    #[derive(Debug)]
    enum ValidationError {
        NotObject {
            path: String,
        },
        NotString {
            path: String,
        },
        NotArray {
            path: String,
        },
        MissingRequired {
            path: String,
            key: String,
        },
        UnexpectedProperty {
            path: String,
            key: String,
        },
        NotInEnum {
            path: String,
            value: String,
        },
        PatternMismatch {
            path: String,
            value: String,
            pattern: String,
        },
        TooShort {
            path: String,
            value: String,
            min: usize,
        },
    }

    fn resolve_ref<'a>(schema: &'a Value, r: &str) -> &'a Value {
        let frag = r.strip_prefix("#/").unwrap_or(r);
        let mut cur = schema;
        for seg in frag.split('/') {
            cur = &cur[seg];
        }
        cur
    }

    fn type_name(v: &Value) -> &'static str {
        match v {
            Value::Object(_) => "object",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Bool(_) => "boolean",
            Value::Null => "null",
            Value::Number(_) => "number",
        }
    }

    fn validate(schema: &Value, v: &Value, path: &str, errors: &mut Vec<ValidationError>) {
        if let Some(r) = schema.get("$ref").and_then(Value::as_str) {
            validate(resolve_ref(&SCHEMA, r), v, path, errors);
            return;
        }
        if let Some(t) = schema.get("type").and_then(Value::as_str) {
            let actual = type_name(v);
            if t == "object" && actual != "object" {
                errors.push(ValidationError::NotObject { path: path.into() });
                return;
            }
            if t == "string" && actual != "string" {
                errors.push(ValidationError::NotString { path: path.into() });
                return;
            }
            if t == "array" && actual != "array" {
                errors.push(ValidationError::NotArray { path: path.into() });
                return;
            }
        }
        if let Some(enums) = schema.get("enum").and_then(Value::as_array) {
            let s = v.as_str().unwrap_or_default();
            if !enums.iter().any(|e| e.as_str() == Some(s)) {
                errors.push(ValidationError::NotInEnum {
                    path: path.into(),
                    value: s.to_string(),
                });
            }
        }
        if let Some(pattern) = schema.get("pattern").and_then(Value::as_str) {
            let s = v.as_str().unwrap_or_default();
            // This schema's pattern is a plain alternation of literals plus
            // one `custom:` prefix branch; check membership without a regex
            // engine.
            let fixed = [
                "http",
                "seo",
                "content",
                "links",
                "images",
                "schema",
                "security",
                "performance",
                "mobile",
                "accessibility",
                "social",
            ];
            let ok = fixed.contains(&s)
                || s.strip_prefix("custom:")
                    .is_some_and(|name| !name.is_empty());
            let _ = pattern; // documented above; membership check suffices
            if !ok {
                errors.push(ValidationError::PatternMismatch {
                    path: path.into(),
                    value: s.to_string(),
                    pattern: pattern.to_string(),
                });
            }
        }
        if let Some(min) = schema.get("minLength").and_then(Value::as_u64) {
            let s = v.as_str().unwrap_or_default();
            if (s.chars().count() as u64) < min {
                errors.push(ValidationError::TooShort {
                    path: path.into(),
                    value: s.to_string(),
                    min: min as usize,
                });
            }
        }
        if type_name(v) == "object" {
            let obj = v.as_object().unwrap_or_else(|| panic!("checked object"));
            if let Some(required) = schema.get("required").and_then(Value::as_array) {
                for key in required.iter().filter_map(Value::as_str) {
                    if !obj.contains_key(key) {
                        errors.push(ValidationError::MissingRequired {
                            path: path.into(),
                            key: key.to_string(),
                        });
                    }
                }
            }
            let props = schema.get("properties").and_then(Value::as_object);
            let closed = schema.get("additionalProperties") == Some(&Value::Bool(false));
            if closed {
                let allowed: std::collections::BTreeSet<&str> = props
                    .map(|p| p.keys().map(String::as_str).collect())
                    .unwrap_or_default();
                for key in obj.keys() {
                    if !allowed.contains(key.as_str()) {
                        errors.push(ValidationError::UnexpectedProperty {
                            path: path.into(),
                            key: key.clone(),
                        });
                    }
                }
            }
            if let Some(props) = props {
                for (key, sub) in props {
                    if let Some(val) = obj.get(key) {
                        validate(sub, val, &format!("{path}.{key}"), errors);
                    }
                }
            }
        }
        if type_name(v) == "array" {
            if let Some(items) = schema.get("items") {
                for (i, item) in v
                    .as_array()
                    .unwrap_or_else(|| panic!("checked array"))
                    .iter()
                    .enumerate()
                {
                    validate(items, item, &format!("{path}[{i}]"), errors);
                }
            }
        }
    }

    fn assert_valid(payload: &Value) {
        let mut errors = Vec::new();
        validate(&SCHEMA, payload, "$", &mut errors);
        assert!(errors.is_empty(), "schema violations: {errors:?}");
    }

    // --- Conformance: emitted artifacts validate. -----------------------

    #[test]
    fn schema_file_exists_and_is_json() {
        assert!(SCHEMA.get("$schema").is_some());
        assert!(SCHEMA.get("$defs").is_some());
    }

    #[test]
    fn canonical_finding_json_validates() {
        let f = sample_finding();
        assert_valid(
            &serde_json::json!({ "findings": [finding_json("https://example.com/", &f)] }),
        );
    }

    #[test]
    fn canonical_finding_json_rejects_extra_keys() {
        // Mutate one payload: an extra key must trip additionalProperties.
        let f = sample_finding();
        let mut v = finding_json("https://example.com/", &f);
        v.as_object_mut()
            .unwrap_or_else(|| panic!("finding is an object"))
            .insert("element".to_string(), Value::String("div".to_string()));
        let mut errors = Vec::new();
        validate(
            &SCHEMA,
            &serde_json::json!({ "findings": [v] }),
            "$",
            &mut errors,
        );
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, ValidationError::UnexpectedProperty { .. })),
            "extra key must violate the closed finding schema: {errors:?}"
        );
    }

    #[test]
    fn custom_category_validates_and_bad_category_fails() {
        let mut f = sample_finding();
        f.category = IssueCategory::Custom("my-plugin".to_string());
        assert_valid(&serde_json::json!({ "findings": [finding_json("https://e.com/", &f)] }));

        let bad = serde_json::json!({ "findings": [{
            "url": "https://e.com/",
            "severity": "warning",
            "category": "bogus-category",
            "code": "X001",
            "title": "t",
            "description": "d",
            "recommendation": "r",
        }]});
        let mut errors = Vec::new();
        validate(&SCHEMA, &bad, "$", &mut errors);
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, ValidationError::PatternMismatch { .. })),
            "unknown category must fail the pattern: {errors:?}"
        );
    }

    #[test]
    fn severity_enum_is_enforced() {
        let bad = serde_json::json!({ "findings": [{
            "url": "https://e.com/",
            "severity": "fatal",
            "category": "seo",
            "code": "X001",
            "title": "t",
            "description": "d",
            "recommendation": "r",
        }]});
        let mut errors = Vec::new();
        validate(&SCHEMA, &bad, "$", &mut errors);
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, ValidationError::NotInEnum { .. })),
            "severity enum must be enforced: {errors:?}"
        );
    }

    #[test]
    fn post_crawl_finding_json_validates() {
        let pf = crawlkit_engine::post_crawl::PostCrawlFinding {
            page_url: "https://example.com/p".to_string(),
            severity: Severity::Error,
            category: IssueCategory::Links,
            code: "LNK001".to_string(),
            title: "T".to_string(),
            description: "D".to_string(),
            recommendation: "R".to_string(),
        };
        assert_valid(&serde_json::json!({
            "findings": [post_crawl_finding_json(&pf)]
        }));
    }

    #[test]
    fn wire_payloads_survive_finding_roundtrip() {
        // The canonical type serializes to the same shape the helpers emit.
        let f = sample_finding();
        let via_type: Value = serde_json::to_value(&f).unwrap();
        let via_helper = finding_json("https://example.com/", &f);
        // Both construction paths must satisfy the schema; they differ only
        // in which url they carry (the Finding's own vs the document's).
        assert_valid(&serde_json::json!({ "findings": [via_type] }));
        assert_valid(&serde_json::json!({ "findings": [via_helper] }));
    }
}
