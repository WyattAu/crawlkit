use serde::{Deserialize, Serialize};

/// SPA detection indicators.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpaIndicators {
    /// Common SPA framework root elements.
    pub root_elements: Vec<String>,
    /// JavaScript framework signatures.
    pub framework_signatures: Vec<String>,
    /// API endpoint patterns.
    pub api_patterns: Vec<String>,
}

impl Default for SpaIndicators {
    fn default() -> Self {
        Self {
            root_elements: vec![
                "id=\"app\"".to_string(),
                "id=\"root\"".to_string(),
                "id=\"__next\"".to_string(),
                "id=\"__nuxt\"".to_string(),
                "id=\"svelte\"".to_string(),
            ],
            framework_signatures: vec![
                "__NEXT_DATA__".to_string(),
                "__NUXT__".to_string(),
                "window.__SVELTEKIT".to_string(),
                "React.createElement".to_string(),
                "Vue.createApp".to_string(),
                "angular".to_string(),
            ],
            api_patterns: vec![
                "/api/".to_string(),
                "/graphql".to_string(),
                "/_next/".to_string(),
                "/_nuxt/".to_string(),
            ],
        }
    }
}

/// Decision engine for whether to use JavaScript rendering.
pub struct JsRenderDecisionEngine {
    indicators: SpaIndicators,
    /// URL patterns that should always use JS rendering.
    force_js_patterns: Vec<String>,
    /// URL patterns that should never use JS rendering.
    skip_js_patterns: Vec<String>,
}

impl JsRenderDecisionEngine {
    /// Create new decision engine.
    #[must_use]
    pub fn new() -> Self {
        Self {
            indicators: SpaIndicators::default(),
            force_js_patterns: Vec::new(),
            skip_js_patterns: Vec::new(),
        }
    }

    /// Add URL pattern that forces JS rendering.
    pub fn add_force_js_pattern(&mut self, pattern: String) {
        self.force_js_patterns.push(pattern);
    }

    /// Add URL pattern that skips JS rendering.
    pub fn add_skip_js_pattern(&mut self, pattern: String) {
        self.skip_js_patterns.push(pattern);
    }

    /// Decide if a URL needs JavaScript rendering.
    #[must_use]
    pub fn should_render_js(&self, url: &str, html_hint: Option<&str>) -> JsRenderDecision {
        // Check skip patterns first
        for pattern in &self.skip_js_patterns {
            if url.contains(pattern.as_str()) {
                return JsRenderDecision::Skip {
                    reason: format!("URL matches skip pattern: {}", pattern),
                };
            }
        }

        // Check force patterns
        for pattern in &self.force_js_patterns {
            if url.contains(pattern.as_str()) {
                return JsRenderDecision::Render {
                    reason: format!("URL matches force pattern: {}", pattern),
                };
            }
        }

        // Check HTML hints for SPA indicators
        if let Some(html) = html_hint {
            for element in &self.indicators.root_elements {
                if html.contains(element.as_str()) {
                    return JsRenderDecision::Render {
                        reason: format!("SPA root element detected: {}", element),
                    };
                }
            }

            for signature in &self.indicators.framework_signatures {
                if html.contains(signature.as_str()) {
                    return JsRenderDecision::Render {
                        reason: format!("Framework signature detected: {}", signature),
                    };
                }
            }
        }

        JsRenderDecision::Skip {
            reason: "No SPA indicators detected".to_string(),
        }
    }
}

impl Default for JsRenderDecisionEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Decision on whether to render with JavaScript.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JsRenderDecision {
    Render { reason: String },
    Skip { reason: String },
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spa_indicators_default() {
        let indicators = SpaIndicators::default();
        assert!(!indicators.root_elements.is_empty());
        assert!(!indicators.framework_signatures.is_empty());
    }

    #[test]
    fn test_js_render_decision_skip() {
        let engine = JsRenderDecisionEngine::new();
        let decision = engine.should_render_js("https://example.com/page", None);
        match decision {
            JsRenderDecision::Skip { .. } => {}
            _ => panic!("Expected Skip"),
        }
    }

    #[test]
    fn test_js_render_decision_nextjs() {
        let engine = JsRenderDecisionEngine::new();
        let html = r#"<div id="__next">Hello</div>"#;
        let decision = engine.should_render_js("https://example.com/page", Some(html));
        match decision {
            JsRenderDecision::Render { reason } => {
                assert!(reason.contains("SPA root element"));
            }
            _ => panic!("Expected Render"),
        }
    }

    #[test]
    fn test_js_render_decision_force_pattern() {
        let mut engine = JsRenderDecisionEngine::new();
        engine.add_force_js_pattern("/dashboard".to_string());
        let decision = engine.should_render_js("https://example.com/dashboard", None);
        match decision {
            JsRenderDecision::Render { reason } => {
                assert!(reason.contains("force pattern"));
            }
            _ => panic!("Expected Render"),
        }
    }
}
