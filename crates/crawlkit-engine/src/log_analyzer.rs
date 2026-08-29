use crate::log_parser::LogEntry;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogAnalysis {
    pub total_requests: usize,
    pub crawler_breakdown: HashMap<String, usize>,
    pub status_breakdown: HashMap<u16, usize>,
    pub top_urls: Vec<(String, usize)>,
    pub error_urls: Vec<(String, u16)>,
}

pub fn analyze_log_entries(entries: &[LogEntry]) -> LogAnalysis {
    let mut crawler_breakdown: HashMap<String, usize> = HashMap::new();
    let mut status_breakdown: HashMap<u16, usize> = HashMap::new();
    let mut url_counts: HashMap<String, usize> = HashMap::new();
    let mut error_urls: Vec<(String, u16)> = Vec::new();

    for entry in entries {
        let category = classify_user_agent(&entry.user_agent);
        *crawler_breakdown.entry(category).or_insert(0) += 1;

        *status_breakdown.entry(entry.status).or_insert(0) += 1;

        *url_counts.entry(entry.url.clone()).or_insert(0) += 1;

        if entry.status >= 400 {
            error_urls.push((entry.url.clone(), entry.status));
        }
    }

    let mut top_urls: Vec<(String, usize)> = url_counts.into_iter().collect();
    top_urls.sort_by_key(|entry| std::cmp::Reverse(entry.1));
    top_urls.truncate(20);

    error_urls.sort();
    error_urls.dedup();

    LogAnalysis {
        total_requests: entries.len(),
        crawler_breakdown,
        status_breakdown,
        top_urls,
        error_urls,
    }
}

pub fn classify_user_agent(ua: &str) -> String {
    let ua_lower = ua.to_lowercase();
    if ua_lower.contains("googlebot") {
        "Googlebot".to_string()
    } else if ua_lower.contains("bingbot") {
        "Bingbot".to_string()
    } else if ua_lower.contains("yandex") {
        "YandexBot".to_string()
    } else if ua_lower.contains("slackbot") {
        "Slackbot".to_string()
    } else if ua_lower.contains("bot")
        || ua_lower.contains("crawler")
        || ua_lower.contains("spider")
    {
        "Other Bot".to_string()
    } else {
        "Human".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log_parser::LogEntry;

    fn make_entry(ua: &str, url: &str, status: u16) -> LogEntry {
        LogEntry {
            ip: "127.0.0.1".into(),
            timestamp: "-".into(),
            method: "GET".into(),
            url: url.into(),
            status,
            size: 0,
            referer: "-".into(),
            user_agent: ua.into(),
        }
    }

    #[test]
    fn test_classify_googlebot() {
        assert_eq!(
            classify_user_agent("Mozilla/5.0 (compatible; Googlebot/2.1)"),
            "Googlebot"
        );
    }

    #[test]
    fn test_classify_bingbot() {
        assert_eq!(
            classify_user_agent("Mozilla/5.0 (compatible; bingbot/2.0)"),
            "Bingbot"
        );
    }

    #[test]
    fn test_classify_yandex() {
        assert_eq!(
            classify_user_agent("Mozilla/5.0 (compatible; YandexBot/3.0)"),
            "YandexBot"
        );
    }

    #[test]
    fn test_classify_slackbot() {
        assert_eq!(
            classify_user_agent("Slackbot-LinkExpanding 1.0"),
            "Slackbot"
        );
    }

    #[test]
    fn test_classify_other_bot() {
        assert_eq!(
            classify_user_agent("SomeCustomCrawler/1.0"),
            "Other Bot"
        );
    }

    #[test]
    fn test_classify_human() {
        assert_eq!(
            classify_user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"),
            "Human"
        );
    }

    #[test]
    fn test_analyze_log_entries() {
        let entries = vec![
            make_entry("Mozilla/5.0 (compatible; Googlebot/2.1)", "/page1", 200),
            make_entry("Mozilla/5.0 (compatible; Googlebot/2.1)", "/page2", 200),
            make_entry("Mozilla/5.0 (Windows NT 10.0)", "/page1", 200),
            make_entry("Mozilla/5.0 (compatible; bingbot/2.0)", "/page3", 404),
            make_entry("Mozilla/5.0", "/missing", 500),
        ];

        let analysis = analyze_log_entries(&entries);
        assert_eq!(analysis.total_requests, 5);
        assert_eq!(analysis.crawler_breakdown.get("Googlebot"), Some(&2));
        assert_eq!(analysis.crawler_breakdown.get("Human"), Some(&2));
        assert_eq!(analysis.crawler_breakdown.get("Bingbot"), Some(&1));
        assert_eq!(analysis.status_breakdown.get(&200), Some(&3));
        assert_eq!(analysis.status_breakdown.get(&404), Some(&1));
        assert_eq!(analysis.status_breakdown.get(&500), Some(&1));
        assert_eq!(analysis.error_urls.len(), 2);
    }

    #[test]
    fn test_top_urls_ordering() {
        let entries = vec![
            make_entry("Mozilla/5.0", "/rare", 200),
            make_entry("Mozilla/5.0", "/popular", 200),
            make_entry("Mozilla/5.0", "/popular", 200),
            make_entry("Mozilla/5.0", "/popular", 200),
            make_entry("Mozilla/5.0", "/medium", 200),
            make_entry("Mozilla/5.0", "/medium", 200),
        ];

        let analysis = analyze_log_entries(&entries);
        assert_eq!(analysis.top_urls[0], ("/popular".to_string(), 3));
        assert_eq!(analysis.top_urls[1], ("/medium".to_string(), 2));
        assert_eq!(analysis.top_urls[2], ("/rare".to_string(), 1));
    }
}
