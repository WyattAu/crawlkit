//! robots.txt group selection, scoped to the crawling user agent.
//!
//! robots.txt is a sequence of groups. Each group starts with one or more
//! `User-agent:` lines and is followed by its own `Allow:` / `Disallow:`
//! rules. A crawler matches exactly **one** group: the most specific
//! (longest) user-agent token that matches its own product token, falling back
//! to `*` when nothing else matches.
//!
//! Collapsing all groups into one bag of directives — which is what a naive
//! `lines().filter(|l| l.starts_with("Disallow:"))` does — reports rules that
//! were never in effect for the auditing agent. A very common real-world
//! layout is:
//!
//! ```text
//! User-agent: *
//! Allow: /
//!
//! User-agent: CCBot
//! Disallow: /            # training crawlers only
//! ```
//!
//! Flattened, that reads as "Disallow: /" and makes every page look blocked,
//! even though the audit crawler is allowed everywhere.

use std::collections::HashMap;

/// Extract the crawler product token from a full user-agent string.
///
/// Matches how robots.txt matching actually works: the product token is the
/// first whitespace-delimited component (`Mozilla/5.0 (compatible; Googlebot/2.1)`
/// matches `Googlebot`, not the whole header).
pub fn product_token(user_agent: &str) -> &str {
    user_agent.split_whitespace().next().unwrap_or("")
}

/// Decide whether a `Disallow:` path under a given group actually blocks the
/// crawler for `important_path`.
///
/// Applies the standard longest-match-wins rule between `Allow` and `Disallow`.
fn path_is_disallowed(rules: &[(&str, bool)], important_path: &str) -> bool {
    let mut best: Option<(usize, bool)> = None;
    for (pattern, is_allow) in rules {
        // Wildcard suffix: `Disallow: /adm` covers `/admin`.
        let matched_len = if *pattern == "/" {
            1
        } else if important_path.starts_with(pattern) {
            pattern.len()
        } else {
            continue;
        };
        // Longest match wins; `Allow` wins ties.
        match best {
            Some((len, allow)) if len > matched_len || (len == matched_len && allow) => {}
            _ => best = Some((matched_len, *is_allow)),
        }
    }
    matches!(best, Some((_, false)))
}

/// True when `robots.txt` disallows `important_path` **for this crawler**.
///
/// Returns `false` when the file is absent, empty, or grants access — i.e. the
/// caller should only act on a `true`.
pub fn disallowed_for_user_agent(robots_txt: &str, user_agent: &str, important_path: &str) -> bool {
    let token = product_token(user_agent).to_ascii_lowercase();

    // Collect rules per group, preserving declaration order.
    let mut groups: Vec<(Vec<String>, Vec<(&str, bool)>)> = Vec::new();
    let mut current_agents: Vec<String> = Vec::new();
    let mut current_rules: Vec<(&str, bool)> = Vec::new();
    let mut in_rules = false;

    for raw in robots_txt.lines() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let (key, value) = match line.split_once(':') {
            Some((k, v)) => (k.trim().to_ascii_lowercase(), v.trim().to_string()),
            None => continue,
        };
        match key.as_str() {
            "user-agent" => {
                // Consecutive User-agent lines share one rule block.
                if in_rules {
                    groups.push((
                        std::mem::take(&mut current_agents),
                        std::mem::take(&mut current_rules),
                    ));
                    in_rules = false;
                }
                current_agents.push(value.to_ascii_lowercase());
            }
            "allow" => {
                in_rules = true;
                current_rules.push((leak(value), true));
            }
            "disallow" => {
                in_rules = true;
                // An empty `Disallow:` means "allow everything" — record it so
                // it can override a shorter inherited match.
                if !value.is_empty() {
                    current_rules.push((leak(value), false));
                }
            }
            _ => {}
        }
    }
    if !current_agents.is_empty() {
        groups.push((current_agents, current_rules));
    }

    // Pick the single most specific matching group.
    let mut best_len = 0usize;
    let mut chosen: Option<&Vec<(&str, bool)>> = None;
    for (agents, rules) in &groups {
        for agent in agents {
            let is_match = if agent == "*" {
                true
            } else {
                token == *agent
                    || token.starts_with(agent.as_str())
                    || agent.starts_with(token.as_str())
            };
            if is_match && agent.len() >= best_len {
                best_len = agent.len();
                chosen = Some(rules);
            }
        }
    }

    match chosen {
        Some(rules) => path_is_disallowed(rules, important_path),
        None => false,
    }
}

// `Vec<(&str, bool)>` borrows from `robots_txt`, which outlives the call. This
// transmute-free helper just documents that the string slices are derived from
// the input and live as long as it.
fn leak(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

/// Group the rules of `robots.txt` by user agent, for reporting purposes.
///
/// Returns a map of user-agent token to the `Disallow` paths that apply to it.
pub fn disallow_paths_by_agent(robots_txt: &str) -> HashMap<String, Vec<String>> {
    let mut out: HashMap<String, Vec<String>> = HashMap::new();
    let mut current_agents: Vec<String> = Vec::new();
    let mut in_rules = false;

    for raw in robots_txt.lines() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        match key.trim().to_ascii_lowercase().as_str() {
            "user-agent" => {
                // Consecutive `User-agent:` lines share one rule block; once a
                // rule has been seen, the next agent starts a *new* group and
                // must not inherit the previous group's disallows.
                if in_rules {
                    current_agents.clear();
                    in_rules = false;
                }
                current_agents.push(value.trim().to_ascii_lowercase());
            }
            "disallow" => {
                in_rules = true;
                if !value.trim().is_empty() {
                    for agent in &current_agents {
                        out.entry(agent.clone())
                            .or_default()
                            .push(value.trim().to_string());
                    }
                }
            }
            "allow" => {
                in_rules = true;
            }
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const KP_ROBOTS: &str = "\
# Kingston Peptides robots.txt

User-agent: *
Allow: /

# Block admin and API paths
Disallow: /admin
Disallow: /api/

User-agent: GPTBot
Allow: /

User-agent: CCBot
Disallow: /

User-agent: ByteSpider
Disallow: /
";

    #[test]
    fn star_agent_allowed_is_not_blocked() {
        // The exact false positive: CCBot's `Disallow: /` must not be
        // attributed to a general-purpose crawler.
        assert!(!disallowed_for_user_agent(KP_ROBOTS, "crawlkit/6.0.0", "/"));
        assert!(!disallowed_for_user_agent(
            KP_ROBOTS,
            "Mozilla/5.0 (compatible; Googlebot/2.1)",
            "/"
        ));
    }

    #[test]
    fn targeted_disallow_still_detected_for_star_agent() {
        assert!(disallowed_for_user_agent(
            KP_ROBOTS,
            "crawlkit/6.0.0",
            "/admin"
        ));
        assert!(disallowed_for_user_agent(
            KP_ROBOTS,
            "crawlkit/6.0.0",
            "/api/products"
        ));
    }

    #[test]
    fn blocked_training_crawler_is_blocked() {
        assert!(disallowed_for_user_agent(KP_ROBOTS, "CCBot/2.0", "/"));
        assert!(disallowed_for_user_agent(
            KP_ROBOTS,
            "Bytespider; spider-feedback@bytedance.com",
            "/"
        ));
    }

    #[test]
    fn longest_most_specific_group_wins() {
        let robots = "User-agent: *\nDisallow: /\n\nUser-agent: Googlebot\nAllow: /\n";
        assert!(!disallowed_for_user_agent(robots, "Googlebot/2.1", "/"));
        assert!(disallowed_for_user_agent(robots, "Bingbot/2.0", "/"));
    }

    #[test]
    fn allow_overrides_shorter_disallow() {
        let robots = "User-agent: *\nDisallow: /private\nAllow: /private/public\n";
        assert!(disallowed_for_user_agent(
            robots,
            "crawlkit",
            "/private/secret"
        ));
        assert!(!disallowed_for_user_agent(
            robots,
            "crawlkit",
            "/private/public/page"
        ));
    }

    #[test]
    fn empty_disallow_means_allow_all() {
        let robots = "User-agent: *\nDisallow:\n";
        assert!(!disallowed_for_user_agent(robots, "crawlkit", "/"));
    }

    #[test]
    fn comments_and_crlf_are_tolerated() {
        let robots = "User-agent: *\r\nDisallow: /admin  # block admin\r\nAllow: /\r\n";
        assert!(disallowed_for_user_agent(robots, "crawlkit", "/admin"));
        assert!(!disallowed_for_user_agent(robots, "crawlkit", "/"));
    }

    #[test]
    fn missing_or_empty_file_never_blocks() {
        assert!(!disallowed_for_user_agent("", "crawlkit", "/"));
        assert!(!disallowed_for_user_agent(
            "# nothing here\n",
            "crawlkit",
            "/"
        ));
    }

    #[test]
    fn disallow_paths_by_agent_reports_per_agent() {
        let by_agent = disallow_paths_by_agent(KP_ROBOTS);
        assert_eq!(by_agent.get("*").unwrap(), &vec!["/admin", "/api/"]);
        assert_eq!(by_agent.get("ccbot").unwrap(), &vec!["/"]);
        assert!(!by_agent.contains_key("gptbot"));
    }
}
