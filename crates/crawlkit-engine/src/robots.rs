//! robots.txt fetching, parsing, and caching.
//!
//! Provides a `RobotsTxtCache` that fetches and parses robots.txt files
//! once per domain, then serves cached results to the crawl loop and
//! analyzers.

use std::sync::Arc;

use dashmap::DashMap;

use crate::analyzers::RobotsRule;
use crate::http::HttpClient;
use crate::CrawlConfig;

/// Per-domain robots.txt cache.
///
/// Thread-safe. Fetches robots.txt once per unique domain, parses it,
/// and caches the rules. Subsequent requests for the same domain return
/// the cached result.
pub struct RobotsTxtCache {
    /// Domain -> parsed rules.
    cache: DashMap<String, CachedRobotsTxt>,
    /// HTTP client for fetching.
    client: Arc<HttpClient>,
    /// Default user-agent for rule matching.
    user_agent: String,
}

struct CachedRobotsTxt {
    rules: Vec<RobotsRule>,
    sitemap_urls: Vec<String>,
    raw_content: String,
}

impl RobotsTxtCache {
    /// Create a new cache with the given HTTP client.
    pub fn new(client: Arc<HttpClient>, config: &CrawlConfig) -> Self {
        Self {
            cache: DashMap::new(),
            client,
            user_agent: config.user_agent.clone(),
        }
    }

    /// Get robots.txt rules for a domain.
    ///
    /// Returns `(rules, sitemap_urls, raw_content)`. If the domain has
    /// no robots.txt, returns empty vectors and an empty string.
    /// Fetches from the network on first access for each domain.
    pub async fn get(&self, scheme: &str, domain: &str) -> (Vec<RobotsRule>, Vec<String>, String) {
        // Check cache first
        if let Some(entry) = self.cache.get(domain) {
            return (
                entry.rules.clone(),
                entry.sitemap_urls.clone(),
                entry.raw_content.clone(),
            );
        }

        // Fetch robots.txt
        let robots_url = format!("{scheme}://{domain}/robots.txt");
        let url = match url::Url::parse(&robots_url) {
            Ok(u) => u,
            Err(_) => {
                self.insert_empty(domain);
                return (Vec::new(), Vec::new(), String::new());
            }
        };

        let content = match self.client.fetch(&url).await {
            Ok(result) => {
                if result.status_code == 200 {
                    result.body
                } else {
                    // 404 or other error: treat as no robots.txt
                    self.insert_empty(domain);
                    return (Vec::new(), Vec::new(), String::new());
                }
            }
            Err(_) => {
                self.insert_empty(domain);
                return (Vec::new(), Vec::new(), String::new());
            }
        };

        let (rules, sitemap_urls) = parse_robots_txt(&content, &self.user_agent);

        let entry = CachedRobotsTxt {
            rules: rules.clone(),
            sitemap_urls: sitemap_urls.clone(),
            raw_content: content.clone(),
        };
        self.cache.insert(domain.to_string(), entry);

        (rules, sitemap_urls, content)
    }

    /// Check if a path is disallowed for the given domain and user-agent.
    pub async fn is_disallowed(&self, scheme: &str, domain: &str, path: &str) -> bool {
        let (rules, _, _) = self.get(scheme, domain).await;
        is_path_disallowed(path, &rules, &self.user_agent)
    }

    /// Get crawl-delay for a domain (in seconds).
    pub async fn crawl_delay(&self, scheme: &str, domain: &str) -> Option<f64> {
        let (rules, _, _) = self.get(scheme, domain).await;
        get_crawl_delay(&rules, &self.user_agent)
    }

    /// Get all sitemap URLs discovered from robots.txt for a domain.
    pub async fn sitemaps(&self, scheme: &str, domain: &str) -> Vec<String> {
        let (_, sitemap_urls, _) = self.get(scheme, domain).await;
        sitemap_urls
    }

    /// Get the raw robots.txt content for a domain.
    pub async fn raw_content(&self, scheme: &str, domain: &str) -> String {
        let (_, _, raw) = self.get(scheme, domain).await;
        raw
    }

    fn insert_empty(&self, domain: &str) {
        self.cache.insert(
            domain.to_string(),
            CachedRobotsTxt {
                rules: Vec::new(),
                sitemap_urls: Vec::new(),
                raw_content: String::new(),
            },
        );
    }
}

/// Parse robots.txt content into rules for a specific user-agent.
///
/// Implements the standard robots.txt protocol:
/// - User-agent lines apply to subsequent rules until next User-agent or end
/// - Disallow/Allow paths are prefix-matched
/// - Crawl-delay specifies delay between requests
/// - Sitemap directives are collected
fn parse_robots_txt(content: &str, target_agent: &str) -> (Vec<RobotsRule>, Vec<String>) {
    let mut rules = Vec::new();
    let mut sitemap_urls = Vec::new();
    let mut current_agent = String::new();
    let mut current_disallowed = Vec::new();
    let mut current_allowed = Vec::new();
    let mut current_delay: Option<f64> = None;
    let mut applies_to_all = false;

    for line in content.lines() {
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Split on first ':'
        let (key, value) = match line.split_once(':') {
            Some((k, v)) => (k.trim().to_lowercase(), v.trim().to_string()),
            None => continue,
        };

        match key.as_str() {
            "user-agent" => {
                // Save previous block if it applies to our target
                if (!current_agent.is_empty() && user_agent_matches(&current_agent, target_agent))
                    || applies_to_all
                {
                    rules.push(RobotsRule {
                        user_agent: current_agent.clone(),
                        disallowed_paths: current_disallowed.clone(),
                        allowed_paths: current_allowed.clone(),
                        crawl_delay: current_delay,
                        sitemaps: Vec::new(), // sitemaps are global
                    });
                }
                current_agent = value;
                current_disallowed.clear();
                current_allowed.clear();
                current_delay = None;
                applies_to_all = current_agent == "*";
            }
            "disallow" => {
                if !value.is_empty() {
                    current_disallowed.push(value);
                }
            }
            "allow" => {
                if !value.is_empty() {
                    current_allowed.push(value);
                }
            }
            "crawl-delay" => {
                if let Ok(delay) = value.parse::<f64>() {
                    current_delay = Some(delay);
                }
            }
            "sitemap" if !value.is_empty() => {
                sitemap_urls.push(value);
            }
            "sitemap" => {} // Empty sitemap value, ignore
            _ => {}         // Ignore unknown directives
        }
    }

    // Save last block
    if (!current_agent.is_empty() && user_agent_matches(&current_agent, target_agent))
        || applies_to_all
    {
        rules.push(RobotsRule {
            user_agent: current_agent,
            disallowed_paths: current_disallowed,
            allowed_paths: current_allowed,
            crawl_delay: current_delay,
            sitemaps: Vec::new(),
        });
    }

    (rules, sitemap_urls)
}

/// Check if a user-agent string matches the target.
///
/// Case-insensitive substring match. `*` matches everything.
fn user_agent_matches(rule_agent: &str, target: &str) -> bool {
    if rule_agent == "*" {
        return true;
    }
    let rule_lower = rule_agent.to_lowercase();
    let target_lower = target.to_lowercase();
    target_lower.contains(&rule_lower) || rule_lower.contains(&target_lower)
}

/// Check if a path is disallowed by any rule.
fn is_path_disallowed(path: &str, rules: &[RobotsRule], user_agent: &str) -> bool {
    let mut disallowed = Vec::new();
    let mut allowed = Vec::new();

    for rule in rules {
        if user_agent_matches(&rule.user_agent, user_agent) || rule.user_agent == "*" {
            disallowed.extend(rule.disallowed_paths.iter());
            allowed.extend(rule.allowed_paths.iter());
        }
    }

    // Allow overrides Disallow for matching prefixes
    for pattern in &allowed {
        if path.starts_with(pattern.as_str()) {
            return false;
        }
    }

    for pattern in &disallowed {
        if path.starts_with(pattern.as_str()) {
            return true;
        }
    }

    false
}

/// Get crawl-delay for a user-agent from the rules.
fn get_crawl_delay(rules: &[RobotsRule], user_agent: &str) -> Option<f64> {
    for rule in rules {
        if user_agent_matches(&rule.user_agent, user_agent) || rule.user_agent == "*" {
            if let Some(delay) = rule.crawl_delay {
                return Some(delay);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_robots_txt_basic() {
        let content = r#"
User-agent: *
Disallow: /admin/
Disallow: /private/
Allow: /admin/public/
Crawl-delay: 2
Sitemap: https://example.com/sitemap.xml
"#;
        let (rules, sitemaps) = parse_robots_txt(content, "MyBot/1.0");
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].disallowed_paths, vec!["/admin/", "/private/"]);
        assert_eq!(rules[0].allowed_paths, vec!["/admin/public/"]);
        assert_eq!(rules[0].crawl_delay, Some(2.0));
        assert_eq!(sitemaps, vec!["https://example.com/sitemap.xml"]);
    }

    #[test]
    fn test_parse_robots_txt_specific_agent() {
        let content = r#"
User-agent: Googlebot
Disallow: /no-google/

User-agent: *
Disallow: /no-all/
"#;
        let (rules, _) = parse_robots_txt(content, "Googlebot/2.1");
        // Should match both the specific rule and the wildcard
        assert!(rules
            .iter()
            .any(|r| r.disallowed_paths.contains(&"/no-google/".to_string())));
        assert!(rules
            .iter()
            .any(|r| r.disallowed_paths.contains(&"/no-all/".to_string())));
    }

    #[test]
    fn test_is_path_disallowed() {
        let rules = vec![RobotsRule {
            user_agent: "*".to_string(),
            disallowed_paths: vec!["/admin/".to_string(), "/private/".to_string()],
            allowed_paths: vec!["/admin/public/".to_string()],
            crawl_delay: None,
            sitemaps: Vec::new(),
        }];

        assert!(is_path_disallowed("/admin/settings", &rules, "AnyBot"));
        assert!(!is_path_disallowed("/admin/public/page", &rules, "AnyBot"));
        assert!(!is_path_disallowed("/public/page", &rules, "AnyBot"));
    }

    #[test]
    fn test_user_agent_matches() {
        assert!(user_agent_matches("*", "AnyBot"));
        assert!(user_agent_matches("Googlebot", "Googlebot/2.1"));
        assert!(user_agent_matches(
            "Googlebot",
            "Mozilla/5.0 (compatible; Googlebot/2.1)"
        ));
        assert!(!user_agent_matches("Bingbot", "Googlebot/2.1"));
    }

    #[test]
    fn test_get_crawl_delay() {
        let rules = vec![RobotsRule {
            user_agent: "*".to_string(),
            disallowed_paths: Vec::new(),
            allowed_paths: Vec::new(),
            crawl_delay: Some(5.0),
            sitemaps: Vec::new(),
        }];

        assert_eq!(get_crawl_delay(&rules, "AnyBot"), Some(5.0));
    }
}
