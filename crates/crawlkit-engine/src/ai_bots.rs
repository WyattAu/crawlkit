use serde::{Deserialize, Serialize};

use crate::types::Severity;

/// Represents an AI crawler/bot that may access web content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiBot {
    /// Identifier string used in robots.txt User-agent directive.
    pub name: &'static str,
    /// Organization that operates the bot.
    pub owner: &'static str,
    /// Purpose of the bot (training, search, etc.).
    pub purpose: &'static str,
    /// Severity when this bot is blocked.
    pub severity: Severity,
}

/// Registry of known AI bots for robots.txt analysis.
///
/// Maintains a static list of AI crawlers with their metadata.
/// Extensible via TOML configuration (Phase 8.5).
pub struct AiBotRegistry {
    bots: &'static [AiBot],
}

impl AiBotRegistry {
    /// Create registry with default bot list.
    #[must_use]
    pub fn default_registry() -> Self {
        Self { bots: AI_BOTS }
    }

    /// Create registry with custom bot list.
    #[must_use]
    pub fn with_bots(bots: &'static [AiBot]) -> Self {
        Self { bots }
    }

    /// Get all registered bots.
    #[must_use]
    pub fn bots(&self) -> &'static [AiBot] {
        self.bots
    }

    /// Find a bot by name (case-insensitive).
    #[must_use]
    pub fn find(&self, name: &str) -> Option<&AiBot> {
        let lower = name.to_lowercase();
        self.bots.iter().find(|b| b.name.to_lowercase() == lower)
    }
}

impl AiBot {
    /// Get ordinal index for finding code generation.
    #[must_use]
    pub fn ordinal(&self) -> usize {
        AI_BOTS
            .iter()
            .position(|b| std::ptr::eq(b, self))
            .unwrap_or(0)
            + 1
    }
}

/// Default list of known AI bots.
pub static AI_BOTS: &[AiBot] = &[
    AiBot {
        name: "GPTBot",
        owner: "OpenAI",
        purpose: "ChatGPT training and browsing",
        severity: Severity::Info,
    },
    AiBot {
        name: "Google-Extended",
        owner: "Google",
        purpose: "Gemini/AI Overviews training",
        severity: Severity::Info,
    },
    AiBot {
        name: "PerplexityBot",
        owner: "Perplexity AI",
        purpose: "Real-time search answers",
        severity: Severity::Info,
    },
    AiBot {
        name: "ClaudeBot",
        owner: "Anthropic",
        purpose: "Claude web search",
        severity: Severity::Info,
    },
    AiBot {
        name: "Anthropic-AI",
        owner: "Anthropic",
        purpose: "Claude training",
        severity: Severity::Info,
    },
    AiBot {
        name: "Bytespider",
        owner: "ByteDance",
        purpose: "TikTok/Douyin AI",
        severity: Severity::Info,
    },
    AiBot {
        name: "Amazonbot",
        owner: "Amazon",
        purpose: "Alexa/shopping AI",
        severity: Severity::Info,
    },
    AiBot {
        name: "Meta-ExternalAgent",
        owner: "Meta",
        purpose: "LLaMA training",
        severity: Severity::Info,
    },
    AiBot {
        name: "Applebot-Extended",
        owner: "Apple",
        purpose: "Apple Intelligence",
        severity: Severity::Info,
    },
    AiBot {
        name: "cohere-ai",
        owner: "Cohere",
        purpose: "Command R training",
        severity: Severity::Info,
    },
];

/// Parse robots.txt content and check if a specific bot is disallowed.
///
/// # Arguments
/// * `robots_txt` - Raw robots.txt content
/// * `bot_name` - User-agent string to check
///
/// # Returns
/// `true` if the bot is disallowed for all paths.
#[must_use]
pub fn robots_txt_disallows_bot(robots_txt: &str, bot_name: &str) -> bool {
    let mut matching_block: Option<bool> = None;
    let mut disallow_all = false;

    for line in robots_txt.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.starts_with('#') {
            if let Some(is_matching) = matching_block {
                if is_matching && disallow_all {
                    return true;
                }
            }
            matching_block = None;
            disallow_all = false;
            continue;
        }

        if let Some((key, value)) = parse_directive(trimmed) {
            match key.as_str() {
                "user-agent" => {
                    if let Some(true) = matching_block {
                        if disallow_all {
                            return true;
                        }
                    }
                    let matches = value == "*" || value.to_lowercase() == bot_name.to_lowercase();
                    matching_block = Some(matches);
                    disallow_all = false;
                }
                "disallow" if matching_block.unwrap_or(false) => {
                    if value == "/" {
                        disallow_all = true;
                    } else if value.is_empty() {
                        disallow_all = false;
                    }
                }
                _ => {}
            }
        }
    }

    if let Some(true) = matching_block {
        if disallow_all {
            return true;
        }
    }

    false
}

/// Parse a robots.txt directive into (key, value) pair.
fn parse_directive(line: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = line.splitn(2, ':').collect();
    if parts.len() == 2 {
        Some((parts[0].trim().to_lowercase(), parts[1].trim().to_string()))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ai_bot_registry_default() {
        let registry = AiBotRegistry::default_registry();
        assert!(!registry.bots().is_empty());
        assert!(registry.find("GPTBot").is_some());
        assert!(registry.find("gptbot").is_some()); // case-insensitive
        assert!(registry.find("nonexistent").is_none());
    }

    #[test]
    fn test_robots_txt_blocks_gptbot() {
        let robots_txt = "User-agent: GPTBot\nDisallow: /";
        assert!(robots_txt_disallows_bot(robots_txt, "GPTBot"));
    }

    #[test]
    fn test_robots_txt_allows_gptbot() {
        let robots_txt = "User-agent: GPTBot\nAllow: /";
        assert!(!robots_txt_disallows_bot(robots_txt, "GPTBot"));
    }

    #[test]
    fn test_robots_txt_blocks_all_bots() {
        let robots_txt = "User-agent: *\nDisallow: /";
        assert!(robots_txt_disallows_bot(robots_txt, "GPTBot"));
        assert!(robots_txt_disallows_bot(robots_txt, "AnyBot"));
    }

    #[test]
    fn test_robots_txt_empty() {
        assert!(!robots_txt_disallows_bot("", "GPTBot"));
    }

    #[test]
    fn test_robots_txt_multiple_blocks() {
        let robots_txt = "User-agent: *\nAllow: /\n\nUser-agent: GPTBot\nDisallow: /";
        assert!(!robots_txt_disallows_bot(robots_txt, "Googlebot"));
        assert!(robots_txt_disallows_bot(robots_txt, "GPTBot"));
    }
}
