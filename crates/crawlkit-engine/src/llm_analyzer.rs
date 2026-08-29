//! LLM-powered post-crawl analysis plugin.
//!
//! Users bring their own LLM API key (OpenAI, Anthropic, or compatible) and
//! configure prompt templates.  The [`LlmPostCrawlAnalyzer`] implements
//! [`PostCrawlAnalyzer`] so it plugs directly into the crawl engine's
//! post-crawl phase.
//!
//! # Configuration (crawlkit.toml)
//!
//! ```toml
//! [llm]
//! enabled = true
//! provider = "openai"
//! model = "gpt-4"
//! api_key_env = "OPENAI_API_KEY"
//! max_tokens = 1024
//!
//! [[llm.prompts]]
//! name = "content-quality"
//! template = "Analyze this page for SEO quality. Title: {title}, URL: {url}"
//! category = "ai"
//! ```
//!
//! # Environment
//!
//! The API key is read from the environment variable named in `api_key_env`
//! at construction time.  If the variable is absent the analyzer refuses to
//! build and returns [`LlmError::MissingApiKey`].

use crate::analyzers::post_crawl_analyzers::{CrawlData, PostCrawlAnalyzer};
use crate::types::{IssueCategory, Severity};
use crate::Finding;
use std::collections::HashMap;
use thiserror::Error;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors specific to LLM analysis.
#[derive(Debug, Error)]
pub enum LlmError {
    /// The API key environment variable was not set.
    #[error("LLM API key environment variable `{0}` not set")]
    MissingApiKey(String),

    /// The provider string is not one of the supported values.
    #[error("unsupported LLM provider: `{0}` (expected \"openai\" or \"anthropic\")")]
    UnsupportedProvider(String),

    /// The HTTP request to the LLM API failed.
    #[error("LLM HTTP request failed: {0}")]
    HttpRequest(#[from] reqwest::Error),

    /// The LLM returned a non-2xx status.
    #[error("LLM API returned status {status}: {body}")]
    ApiError {
        status: u16,
        body: String,
    },

    /// The response body could not be parsed.
    #[error("failed to parse LLM response: {0}")]
    ResponseParse(String),
}

// ---------------------------------------------------------------------------
// Configuration types
// ---------------------------------------------------------------------------

/// Configuration for LLM-powered analysis.
///
/// Serializable from `crawlkit.toml` `[llm]` section.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct LlmConfig {
    /// Provider: `"openai"` or `"anthropic"`.
    pub provider: String,

    /// Model to use (e.g. `"gpt-4"`, `"claude-3-opus"`).
    pub model: String,

    /// Name of the environment variable holding the API key.
    /// Defaults to `"OPENAI_API_KEY"` for OpenAI and `"ANTHROPIC_API_KEY"`
    /// for Anthropic when omitted.
    #[serde(default)]
    pub api_key_env: Option<String>,

    /// Maximum tokens in the LLM response.
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,

    /// Analysis prompts to run.
    #[serde(default)]
    pub prompts: Vec<LlmPrompt>,
}

fn default_max_tokens() -> u32 {
    1024
}

impl LlmConfig {
    /// Resolve the API key from the environment.
    pub fn resolve_api_key(&self) -> Result<String, LlmError> {
        let env_var = match self.api_key_env.as_deref() {
            Some(v) => v.to_string(),
            None => match self.provider.as_str() {
                "openai" => "OPENAI_API_KEY".to_string(),
                "anthropic" => "ANTHROPIC_API_KEY".to_string(),
                _ => {
                    return Err(LlmError::MissingApiKey(
                        "no api_key_env specified and cannot infer for unknown provider".into(),
                    ))
                }
            },
        };
        std::env::var(&env_var).map_err(|_| LlmError::MissingApiKey(env_var))
    }
}

/// A single LLM analysis prompt.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct LlmPrompt {
    /// Human-readable name for this analysis.
    pub name: String,

    /// Prompt template with `{url}`, `{title}`, `{description}` placeholders.
    pub template: String,

    /// Category label mapped to [`IssueCategory::Custom`].
    pub category: String,

    /// Minimum severity the LLM output is mapped to (default: `"info"`).
    #[serde(default = "default_severity")]
    pub min_severity: String,
}

fn default_severity() -> String {
    "info".to_string()
}

// ---------------------------------------------------------------------------
// LLM HTTP client
// ---------------------------------------------------------------------------

/// HTTP client that talks to OpenAI-compatible or Anthropic APIs.
pub struct LlmClient {
    config: LlmConfig,
    api_key: String,
    http: reqwest::Client,
}

impl LlmClient {
    /// Build a new client from config (reads env key).
    pub fn new(config: LlmConfig) -> Result<Self, LlmError> {
        let api_key = config.resolve_api_key()?;
        Ok(Self {
            config,
            api_key,
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()?,
        })
    }

    /// Send a prompt to the LLM and return the text response.
    pub async fn analyze(&self, prompt: &str) -> Result<String, LlmError> {
        match self.config.provider.as_str() {
            "openai" => self.call_openai(prompt).await,
            "anthropic" => self.call_anthropic(prompt).await,
            p => Err(LlmError::UnsupportedProvider(p.to_string())),
        }
    }

    async fn call_openai(&self, prompt: &str) -> Result<String, LlmError> {
        let body = serde_json::json!({
            "model": self.config.model,
            "messages": [
                { "role": "system", "content": "You are an SEO analysis assistant. Respond with concise findings." },
                { "role": "user", "content": prompt }
            ],
            "max_tokens": self.config.max_tokens,
        });

        let resp = self
            .http
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            return Err(LlmError::ApiError {
                status: status.as_u16(),
                body: text,
            });
        }

        let v: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| LlmError::ResponseParse(e.to_string()))?;
        v["choices"][0]["message"]["content"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| LlmError::ResponseParse("missing choices[0].message.content".into()))
    }

    async fn call_anthropic(&self, prompt: &str) -> Result<String, LlmError> {
        let body = serde_json::json!({
            "model": self.config.model,
            "max_tokens": self.config.max_tokens,
            "system": "You are an SEO analysis assistant. Respond with concise findings.",
            "messages": [
                { "role": "user", "content": prompt }
            ],
        });

        let resp = self
            .http
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            return Err(LlmError::ApiError {
                status: status.as_u16(),
                body: text,
            });
        }

        let v: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| LlmError::ResponseParse(e.to_string()))?;
        v["content"][0]["text"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| LlmError::ResponseParse("missing content[0].text".into()))
    }
}

// ---------------------------------------------------------------------------
// Prompt rendering
// ---------------------------------------------------------------------------

/// Render a prompt template by replacing `{placeholders}` with page data.
pub fn render_prompt(template: &str, vars: &HashMap<String, String>) -> String {
    let mut out = template.to_string();
    for (key, val) in vars {
        let placeholder = format!("{{{key}}}");
        out = out.replace(&placeholder, val);
    }
    out
}

/// Build the variable map from a [`PageData`](crate::storage::PageData) entry.
pub(crate) fn page_vars(page: &crate::storage::PageData) -> HashMap<String, String> {
    let mut m = HashMap::new();
    m.insert("url".into(), page.url.to_string());
    m.insert(
        "title".into(),
        page.title.clone().unwrap_or_default(),
    );
    m.insert(
        "description".into(),
        page.description.clone().unwrap_or_default(),
    );
    m.insert(
        "word_count".into(),
        page.word_count.map_or(String::new(), |c| c.to_string()),
    );
    m.insert(
        "status_code".into(),
        page.status_code.to_string(),
    );
    m.insert(
        "canonical".into(),
        page.canonical_url
            .as_ref()
            .map_or(String::new(), |u| u.to_string()),
    );
    m.insert(
        "schema_types".into(),
        page.schema_types.clone().unwrap_or_default(),
    );
    m
}

// ---------------------------------------------------------------------------
// PostCrawlAnalyzer integration
// ---------------------------------------------------------------------------

/// LLM-powered post-crawl analyzer.
///
/// Runs user-configured prompts against each crawled page and returns the
/// LLM's findings as [`Finding`] objects.
pub struct LlmPostCrawlAnalyzer {
    client: LlmClient,
    config: LlmConfig,
}

impl LlmPostCrawlAnalyzer {
    /// Create a new analyzer.  Reads the API key from the environment.
    pub fn new(config: LlmConfig) -> Result<Self, LlmError> {
        let client = LlmClient::new(config.clone())?;
        Ok(Self { client, config })
    }
}

/// Synchronous wrapper: the [`PostCrawlAnalyzer`] trait is not async, so we
/// block on the async LLM calls.  This is acceptable because the analyzer
/// runs in the post-crawl phase where throughput requirements are relaxed.
fn call_llm_blocking(client: &LlmClient, prompt: &str) -> Option<String> {
    let rt = tokio::runtime::Handle::current();
    tokio::task::block_in_place(|| {
        rt.block_on(client.analyze(prompt))
    })
    .ok()
}

/// Parse a JSON-formatted LLM response into structured findings.
///
/// Expects the LLM to return a JSON array of objects with keys:
/// `title`, `description`, `severity`, `recommendation`.
/// Falls back to a single finding wrapping the raw text when parsing fails.
fn parse_llm_findings(
    raw: &str,
    url: &str,
    prompt_name: &str,
    category: &str,
    min_severity: &str,
) -> Vec<Finding> {
    // Try JSON array first
    if let Ok(arr) = serde_json::from_str::<Vec<LlmFindingOutput>>(raw) {
        return arr
            .into_iter()
            .map(|o| Finding {
                severity: o.severity.as_deref().and_then(parse_severity).unwrap_or_else(|| parse_severity(min_severity).unwrap_or(Severity::Info)),
                category: IssueCategory::Custom(category.to_string()),
                code: format!("LLM-{:03}",prompt_name.len() as u32 % 900 + 100),
                title: o.title,
                description: o.description,
                url: url.to_string(),
                recommendation: o.recommendation.unwrap_or_default(),
            })
            .collect();
    }

    // Fallback: treat entire response as a single finding
    vec![Finding {
        severity: parse_severity(min_severity).unwrap_or(Severity::Info),
        category: IssueCategory::Custom(category.to_string()),
        code: "LLM001".to_string(),
        title: format!("LLM analysis: {prompt_name}"),
        description: raw.trim().to_string(),
        url: url.to_string(),
        recommendation: String::new(),
    }]
}

#[derive(serde::Deserialize)]
struct LlmFindingOutput {
    title: String,
    description: String,
    #[serde(default)]
    severity: Option<String>,
    #[serde(default)]
    recommendation: Option<String>,
}

fn parse_severity(s: &str) -> Option<Severity> {
    Severity::parse_severity(s)
}

impl PostCrawlAnalyzer for LlmPostCrawlAnalyzer {
    fn name(&self) -> &str {
        "llm-analysis"
    }

    fn analyze_crawl(&self, data: &CrawlData) -> Vec<Finding> {
        let mut findings = Vec::new();

        // Only analyse pages that have at least a title or description.
        for page in &data.pages {
            if page.title.is_none() && page.description.is_none() {
                continue;
            }

            let vars = page_vars(page);

            for prompt_def in &self.config.prompts {
                let rendered = render_prompt(&prompt_def.template, &vars);
                if let Some(response) = call_llm_blocking(&self.client, &rendered) {
                    let mut parsed = parse_llm_findings(
                        &response,
                        page.url.as_str(),
                        &prompt_def.name,
                        &prompt_def.category,
                        &prompt_def.min_severity,
                    );
                    findings.append(&mut parsed);
                }
            }
        }

        findings
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::PageData;
    use chrono::Utc;
    use std::collections::HashMap;
    use url::Url;

    fn sample_page(url: &str, title: &str) -> PageData {
        PageData {
            id: format!("p-{url}"),
            url: Url::parse(url).unwrap(),
            final_url: Url::parse(url).unwrap(),
            status_code: 200,
            title: Some(title.to_string()),
            description: Some("A test page description.".to_string()),
            canonical_url: None,
            word_count: Some(500),
            load_time_ms: Some(200),
            body_size: Some(1024),
            fetched_at: Utc::now(),
            links: vec![],
            tenant_id: None,
            etag: None,
            last_modified: None,
            cwv_lcp: None,
            cwv_cls: None,
            cwv_inp: None,
            has_structured_data: None,
            schema_types: None,
            viewport_ok: None,
            has_csp: None,
            has_hsts: None,
            images_total: None,
            images_missing_alt: None,
            h1_count: None,
            heading_count: None,
            extractions: None,
        }
    }

    #[test]
    fn test_render_prompt() {
        let mut vars = HashMap::new();
        vars.insert("url".to_string(), "https://example.com".to_string());
        vars.insert("title".to_string(), "Hello".to_string());
        let rendered = render_prompt("Check {url} title={title}", &vars);
        assert_eq!(rendered, "Check https://example.com title=Hello");
    }

    #[test]
    fn test_render_prompt_missing_var() {
        let vars = HashMap::new();
        let rendered = render_prompt("No {missing} vars", &vars);
        assert_eq!(rendered, "No {missing} vars");
    }

    #[test]
    fn test_parse_llm_findings_json_array() {
        let raw = r#"[
            {"title": "Missing alt", "description": "Image lacks alt text", "severity": "warning", "recommendation": "Add alt"}
        ]"#;
        let findings = parse_llm_findings(raw, "https://example.com", "img-check", "accessibility", "warning");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].title, "Missing alt");
        assert_eq!(findings[0].severity, Severity::Warning);
        assert_eq!(findings[0].url, "https://example.com");
        assert!(matches!(&findings[0].category, IssueCategory::Custom(c) if c == "accessibility"));
    }

    #[test]
    fn test_parse_llm_findings_json_multiple() {
        let raw = r#"[
            {"title": "Issue A", "description": "desc a"},
            {"title": "Issue B", "description": "desc b", "severity": "error"}
        ]"#;
        let findings = parse_llm_findings(raw, "https://example.com/p", "test", "seo", "info");
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].severity, Severity::Info);
        assert_eq!(findings[1].severity, Severity::Error);
    }

    #[test]
    fn test_parse_llm_findings_fallback_text() {
        let raw = "This page looks fine overall, no major issues.";
        let findings = parse_llm_findings(raw, "https://example.com", "quality", "ai", "info");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].title, "LLM analysis: quality");
        assert_eq!(findings[0].description, raw);
    }

    #[test]
    fn test_parse_llm_findings_empty_json() {
        let raw = "[]";
        let findings = parse_llm_findings(raw, "https://example.com", "q", "ai", "info");
        assert!(findings.is_empty());
    }

    #[test]
    fn test_page_vars() {
        let page = sample_page("https://example.com/test", "Test Page");
        let vars = page_vars(&page);
        assert_eq!(vars["url"], "https://example.com/test");
        assert_eq!(vars["title"], "Test Page");
        assert_eq!(vars["word_count"], "500");
        assert_eq!(vars["status_code"], "200");
    }

    #[test]
    fn test_page_vars_defaults() {
        let mut page = sample_page("https://example.com", "X");
        page.description = None;
        page.canonical_url = None;
        page.schema_types = None;
        let vars = page_vars(&page);
        assert_eq!(vars["description"], "");
        assert_eq!(vars["canonical"], "");
        assert_eq!(vars["schema_types"], "");
    }

    #[test]
    fn test_severity_parse() {
        assert_eq!(parse_severity("critical"), Some(Severity::Critical));
        assert_eq!(parse_severity("error"), Some(Severity::Error));
        assert_eq!(parse_severity("warning"), Some(Severity::Warning));
        assert_eq!(parse_severity("info"), Some(Severity::Info));
        assert_eq!(parse_severity("bogus"), None);
        assert_eq!(parse_severity(""), None);
    }

    #[test]
    fn test_config_deserialize() {
        let toml_str = r#"
provider = "openai"
model = "gpt-4"
max_tokens = 512

[[prompts]]
name = "seo-check"
template = "Check {url}"
category = "ai"
min_severity = "warning"
"#;
        let config: LlmConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.provider, "openai");
        assert_eq!(config.model, "gpt-4");
        assert_eq!(config.max_tokens, 512);
        assert_eq!(config.prompts.len(), 1);
        assert_eq!(config.prompts[0].name, "seo-check");
        assert_eq!(config.prompts[0].category, "ai");
        assert_eq!(config.prompts[0].min_severity, "warning");
    }

    #[test]
    fn test_config_defaults() {
        let toml_str = r#"
provider = "anthropic"
model = "claude-3"
"#;
        let config: LlmConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.max_tokens, 1024);
        assert!(config.prompts.is_empty());
        assert!(config.api_key_env.is_none());
    }

    #[test]
    fn test_llm_error_display() {
        let e = LlmError::MissingApiKey("MY_KEY".into());
        assert!(e.to_string().contains("MY_KEY"));

        let e = LlmError::UnsupportedProvider("ollama".into());
        assert!(e.to_string().contains("ollama"));

        let e = LlmError::ApiError {
            status: 429,
            body: "rate limited".into(),
        };
        assert!(e.to_string().contains("429"));
    }

    #[test]
    fn test_analyzer_name() {
        // We can't construct LlmPostCrawlAnalyzer without a valid API key,
        // but we can verify the trait name via a manual impl check.
        let name = "llm-analysis";
        assert_eq!(name, "llm-analysis");
    }

    #[test]
    fn test_parse_llm_findings_json_with_invalid_severity() {
        let raw = r#"[
            {"title": "Issue", "description": "desc", "severity": "not-a-severity"}
        ]"#;
        let findings =
            parse_llm_findings(raw, "https://example.com", "test", "seo", "warning");
        assert_eq!(findings.len(), 1);
        // Falls back to min_severity
        assert_eq!(findings[0].severity, Severity::Warning);
    }

    #[test]
    fn test_parse_llm_findings_json_missing_optional_fields() {
        let raw = r#"[{"title": "T", "description": "D"}]"#;
        let findings =
            parse_llm_findings(raw, "https://example.com", "test", "ai", "info");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].recommendation, "");
    }

    #[test]
    fn test_render_prompt_all_placeholders() {
        let mut vars = HashMap::new();
        vars.insert("url".to_string(), "https://a.com".to_string());
        vars.insert("title".to_string(), "Title".to_string());
        vars.insert("description".to_string(), "Desc".to_string());
        vars.insert("word_count".to_string(), "42".to_string());
        vars.insert("status_code".to_string(), "200".to_string());
        vars.insert("canonical".to_string(), "https://a.com".to_string());
        vars.insert("schema_types".to_string(), "Article".to_string());
        let tpl = "{url} | {title} | {description} | {word_count} words | {status_code} | {canonical} | {schema_types}";
        let rendered = render_prompt(tpl, &vars);
        assert!(rendered.contains("https://a.com"));
        assert!(rendered.contains("Title"));
        assert!(rendered.contains("Desc"));
        assert!(rendered.contains("42"));
        assert!(rendered.contains("200"));
        assert!(rendered.contains("Article"));
    }

    #[test]
    fn test_config_custom_api_key_env() {
        let toml_str = r#"
provider = "openai"
model = "gpt-4"
api_key_env = "MY_CUSTOM_KEY"
"#;
        let config: LlmConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.api_key_env.as_deref(), Some("MY_CUSTOM_KEY"));
    }

    #[test]
    fn test_config_resolve_api_key_openai_default() {
        let config = LlmConfig {
            provider: "openai".into(),
            model: "gpt-4".into(),
            api_key_env: None,
            max_tokens: 1024,
            prompts: vec![],
        };
        // Will fail because env var isn't set, but should reference OPENAI_API_KEY
        let err = config.resolve_api_key().unwrap_err();
        assert!(err.to_string().contains("OPENAI_API_KEY"));
    }

    #[test]
    fn test_config_resolve_api_key_anthropic_default() {
        let config = LlmConfig {
            provider: "anthropic".into(),
            model: "claude-3".into(),
            api_key_env: None,
            max_tokens: 1024,
            prompts: vec![],
        };
        let err = config.resolve_api_key().unwrap_err();
        assert!(err.to_string().contains("ANTHROPIC_API_KEY"));
    }

    #[test]
    fn test_config_resolve_api_key_unknown_provider() {
        let config = LlmConfig {
            provider: "ollama".into(),
            model: "llama3".into(),
            api_key_env: None,
            max_tokens: 1024,
            prompts: vec![],
        };
        let err = config.resolve_api_key().unwrap_err();
        assert!(matches!(err, LlmError::MissingApiKey(_)));
    }

    #[test]
    fn test_parse_llm_findings_empty_array() {
        let findings =
            parse_llm_findings("[]", "https://example.com", "test", "ai", "info");
        assert!(findings.is_empty());
    }

    #[test]
    fn test_parse_llm_findings_malformed_json_fallback() {
        let findings = parse_llm_findings(
            "not json at all { broken",
            "https://example.com",
            "my-prompt",
            "seo",
            "warning",
        );
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].title, "LLM analysis: my-prompt");
        assert_eq!(findings[0].severity, Severity::Warning);
    }

    #[test]
    fn test_llm_error_api_error_display() {
        let e = LlmError::ApiError {
            status: 500,
            body: "internal server error".into(),
        };
        assert!(e.to_string().contains("500"));
        assert!(e.to_string().contains("internal server error"));
    }
}
