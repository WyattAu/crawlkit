use serde::{Deserialize, Serialize};

/// Configuration for declarative custom extraction rules.
///
/// Users define CSS selector + regex extraction rules in `crawlkit.toml`.
/// Extracted fields are stored per-page and available in exports.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ExtractionConfig {
    /// Whether custom extraction is enabled.
    #[serde(default)]
    pub enabled: bool,
    /// The list of extraction rules to apply to each page.
    #[serde(default)]
    pub rules: Vec<ExtractionRule>,
}

/// A single extraction rule defining how to extract a field from a page.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExtractionRule {
    /// Unique name for this extracted field.
    pub name: String,
    /// CSS selector to match elements.
    pub selector: String,
    /// What to extract from matched elements: "text", "html", or an attribute name.
    #[serde(default = "default_attribute")]
    pub attribute: String,
    /// Optional regex to apply to the raw extracted value.
    #[serde(default)]
    pub regex: Option<String>,
    /// Which regex capture group to use (default: 0 = full match).
    #[serde(default)]
    pub capture_group: Option<usize>,
    /// Fallback value when no match is found.
    #[serde(default)]
    pub default: String,
    /// Maximum character length for the extracted value (truncated with "...").
    #[serde(default)]
    pub max_length: Option<usize>,
}

fn default_attribute() -> String {
    "text".to_string()
}

/// Result of extracting a single rule against a page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionResult {
    /// The rule name.
    pub rule_name: String,
    /// All matched values (one per matching element, or a single default).
    pub values: Vec<String>,
}

/// Apply extraction rules to a page's HTML and return results for each rule.
///
/// Each rule is evaluated independently:
/// 1. Parse the CSS selector (skip on invalid selectors).
/// 2. Select matching elements.
/// 3. Extract the raw value per element (text, html, or attribute).
/// 4. Apply optional regex capture.
/// 5. Apply optional max_length truncation.
/// 6. Filter out empty values; fall back to the rule's default if none remain.
pub fn extract_page(page_html: &str, rules: &[ExtractionRule]) -> Vec<ExtractionResult> {
    let document = scraper::Html::parse_document(page_html);
    let mut results = Vec::with_capacity(rules.len());

    for rule in rules {
        let selector = match scraper::Selector::parse(&rule.selector) {
            Ok(s) => s,
            Err(_) => continue,
        };

        let mut values = Vec::new();
        for element in document.select(&selector) {
            let raw = match rule.attribute.as_str() {
                "text" => element.text().collect::<String>(),
                "html" => element.inner_html(),
                attr => element
                    .value()
                    .attr(attr)
                    .unwrap_or(&rule.default)
                    .to_string(),
            };

            let value = if let Some(ref regex) = rule.regex {
                if let Ok(re) = regex::Regex::new(regex) {
                    let group = rule.capture_group.unwrap_or(0);
                    re.captures(&raw)
                        .and_then(|cap| cap.get(group))
                        .map(|m| m.as_str().to_string())
                        .unwrap_or_else(|| rule.default.clone())
                } else {
                    rule.default.clone()
                }
            } else {
                raw
            };

            let value = if let Some(max) = rule.max_length {
                if value.len() > max {
                    let truncated: String = value.chars().take(max).collect();
                    format!("{}...", truncated)
                } else {
                    value
                }
            } else {
                value
            };

            if !value.is_empty() {
                values.push(value);
            }
        }

        if values.is_empty() {
            values.push(rule.default.clone());
        }

        results.push(ExtractionResult {
            rule_name: rule.name.clone(),
            values,
        });
    }

    results
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn rule(name: &str, selector: &str) -> ExtractionRule {
        ExtractionRule {
            name: name.to_string(),
            selector: selector.to_string(),
            attribute: "text".to_string(),
            regex: None,
            capture_group: None,
            default: String::new(),
            max_length: None,
        }
    }

    #[test]
    fn test_extract_text_from_element() {
        let html = r#"<html><body><p class="date">2025-01-15</p></body></html>"#;
        let rules = vec![ExtractionRule {
            name: "date".to_string(),
            selector: "p.date".to_string(),
            attribute: "text".to_string(),
            regex: None,
            capture_group: None,
            default: String::new(),
            max_length: None,
        }];
        let results = extract_page(html, &rules);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].rule_name, "date");
        assert_eq!(results[0].values, vec!["2025-01-15"]);
    }

    #[test]
    fn test_extract_attribute_value() {
        let html =
            r#"<html><body><time datetime="2025-01-15T10:30:00Z">Jan 15</time></body></html>"#;
        let rules = vec![ExtractionRule {
            name: "article_date".to_string(),
            selector: "time[datetime]".to_string(),
            attribute: "datetime".to_string(),
            regex: None,
            capture_group: None,
            default: String::new(),
            max_length: None,
        }];
        let results = extract_page(html, &rules);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].values, vec!["2025-01-15T10:30:00Z"]);
    }

    #[test]
    fn test_extract_html_content() {
        let html = r#"<html><body><div class="bio"><b>Jane</b> is a writer</div></body></html>"#;
        let rules = vec![ExtractionRule {
            name: "author_bio".to_string(),
            selector: ".bio".to_string(),
            attribute: "html".to_string(),
            regex: None,
            capture_group: None,
            default: String::new(),
            max_length: None,
        }];
        let results = extract_page(html, &rules);
        assert_eq!(results.len(), 1);
        assert!(results[0].values[0].contains("<b>Jane</b>"));
    }

    #[test]
    fn test_extract_with_regex_capture() {
        let html = r#"<html><body><span class="price">$49.99</span></body></html>"#;
        let rules = vec![ExtractionRule {
            name: "price".to_string(),
            selector: ".price".to_string(),
            attribute: "text".to_string(),
            regex: Some(r"\$(\d+\.?\d*)".to_string()),
            capture_group: Some(1),
            default: String::new(),
            max_length: None,
        }];
        let results = extract_page(html, &rules);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].values, vec!["49.99"]);
    }

    #[test]
    fn test_extract_with_regex_full_match() {
        let html = r#"<html><body><span class="price">$49.99</span></body></html>"#;
        let rules = vec![ExtractionRule {
            name: "price_full".to_string(),
            selector: ".price".to_string(),
            attribute: "text".to_string(),
            regex: Some(r"\$\d+\.?\d*".to_string()),
            capture_group: None, // defaults to 0
            default: String::new(),
            max_length: None,
        }];
        let results = extract_page(html, &rules);
        assert_eq!(results[0].values, vec!["$49.99"]);
    }

    #[test]
    fn test_extract_max_length() {
        let html = r#"<html><body><p class="bio">This is a very long biography text that should be truncated at some point</p></body></html>"#;
        let rules = vec![ExtractionRule {
            name: "bio".to_string(),
            selector: ".bio".to_string(),
            attribute: "text".to_string(),
            regex: None,
            capture_group: None,
            default: String::new(),
            max_length: Some(20),
        }];
        let results = extract_page(html, &rules);
        assert_eq!(results.len(), 1);
        assert!(results[0].values[0].ends_with("..."));
        assert!(results[0].values[0].len() <= 23); // 20 + "..."
    }

    #[test]
    fn test_extract_fallback_to_default() {
        let html = r#"<html><body><p>No matching element</p></body></html>"#;
        let rules = vec![ExtractionRule {
            name: "missing".to_string(),
            selector: ".nonexistent".to_string(),
            attribute: "text".to_string(),
            regex: None,
            capture_group: None,
            default: "N/A".to_string(),
            max_length: None,
        }];
        let results = extract_page(html, &rules);
        assert_eq!(results[0].values, vec!["N/A"]);
    }

    #[test]
    fn test_extract_invalid_selector_skipped() {
        let html = r#"<html><body><p>Content</p></body></html>"#;
        let rules = vec![ExtractionRule {
            name: "bad".to_string(),
            selector: ">>>invalid<<<".to_string(),
            attribute: "text".to_string(),
            regex: None,
            capture_group: None,
            default: String::new(),
            max_length: None,
        }];
        let results = extract_page(html, &rules);
        assert!(results.is_empty(), "Invalid selector should be skipped");
    }

    #[test]
    fn test_extract_multiple_elements() {
        let html = r#"<html><body>
            <li class="tag">rust</li>
            <li class="tag">seo</li>
            <li class="tag">crawler</li>
        </body></html>"#;
        let rules = vec![rule("tags", ".tag")];
        let results = extract_page(html, &rules);
        assert_eq!(results[0].values, vec!["rust", "seo", "crawler"]);
    }

    #[test]
    fn test_extract_multiple_rules() {
        let html = r#"<html><body>
            <h1>Main Title</h1>
            <time datetime="2025-06-01">June 1</time>
            <span class="price">$29.99</span>
        </body></html>"#;
        let rules = vec![
            ExtractionRule {
                name: "title".to_string(),
                selector: "h1".to_string(),
                attribute: "text".to_string(),
                regex: None,
                capture_group: None,
                default: String::new(),
                max_length: None,
            },
            ExtractionRule {
                name: "date".to_string(),
                selector: "time[datetime]".to_string(),
                attribute: "datetime".to_string(),
                regex: None,
                capture_group: None,
                default: String::new(),
                max_length: None,
            },
            ExtractionRule {
                name: "price".to_string(),
                selector: ".price".to_string(),
                attribute: "text".to_string(),
                regex: Some(r"\$(\d+)".to_string()),
                capture_group: Some(1),
                default: String::new(),
                max_length: None,
            },
        ];
        let results = extract_page(html, &rules);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].values, vec!["Main Title"]);
        assert_eq!(results[1].values, vec!["2025-06-01"]);
        assert_eq!(results[2].values, vec!["29"]);
    }

    #[test]
    fn test_extract_no_regex_returns_raw() {
        let html = r#"<html><body><span class="price">$49.99</span></body></html>"#;
        let rules = vec![rule("price", ".price")];
        let results = extract_page(html, &rules);
        assert_eq!(results[0].values, vec!["$49.99"]);
    }

    #[test]
    fn test_extract_regex_no_match_falls_back_to_default() {
        let html = r#"<html><body><span class="text">hello world</span></body></html>"#;
        let rules = vec![ExtractionRule {
            name: "number".to_string(),
            selector: ".text".to_string(),
            attribute: "text".to_string(),
            regex: Some(r"\d+".to_string()),
            capture_group: None,
            default: "none".to_string(),
            max_length: None,
        }];
        let results = extract_page(html, &rules);
        assert_eq!(results[0].values, vec!["none"]);
    }

    #[test]
    fn test_extract_empty_html_returns_defaults() {
        let html = "";
        let rules = vec![ExtractionRule {
            name: "field".to_string(),
            selector: "div".to_string(),
            attribute: "text".to_string(),
            regex: None,
            capture_group: None,
            default: "fallback".to_string(),
            max_length: None,
        }];
        let results = extract_page(html, &rules);
        assert_eq!(results[0].values, vec!["fallback"]);
    }

    #[test]
    fn test_extraction_config_serialization() {
        let config = ExtractionConfig {
            enabled: true,
            rules: vec![ExtractionRule {
                name: "date".to_string(),
                selector: "time[datetime]".to_string(),
                attribute: "datetime".to_string(),
                regex: None,
                capture_group: None,
                default: String::new(),
                max_length: None,
            }],
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: ExtractionConfig = serde_json::from_str(&json).unwrap();
        assert!(deserialized.enabled);
        assert_eq!(deserialized.rules.len(), 1);
        assert_eq!(deserialized.rules[0].name, "date");
    }

    #[cfg(feature = "full")]
    #[test]
    fn test_extraction_config_toml_roundtrip() {
        let toml_str = r#"
enabled = true

[[rules]]
name = "article_date"
selector = "time[datetime]"
attribute = "datetime"

[[rules]]
name = "price_range"
selector = ".price"
regex = "\\$(\\d+\\.?\\d*)"
capture_group = 1

[[rules]]
name = "author_bio"
selector = ".author-bio"
attribute = "text"
max_length = 500
default = ""
"#;
        let config: ExtractionConfig = toml::from_str(toml_str).unwrap();
        assert!(config.enabled);
        assert_eq!(config.rules.len(), 3);
        assert_eq!(config.rules[0].name, "article_date");
        assert_eq!(config.rules[1].name, "price_range");
        assert_eq!(config.rules[1].regex, Some(r"\$(\d+\.?\d*)".to_string()));
        assert_eq!(config.rules[1].capture_group, Some(1));
        assert_eq!(config.rules[2].name, "author_bio");
        assert_eq!(config.rules[2].max_length, Some(500));
    }

    #[test]
    fn test_extraction_result_serialization() {
        let result = ExtractionResult {
            rule_name: "price".to_string(),
            values: vec!["49.99".to_string(), "29.99".to_string()],
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("price"));
        assert!(json.contains("49.99"));
    }
}
