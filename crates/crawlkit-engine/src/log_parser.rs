use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LogFormat {
    NginxCombined,
    ApacheCombined,
    JsonStructured,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub ip: String,
    pub timestamp: String,
    pub method: String,
    pub url: String,
    pub status: u16,
    pub size: u64,
    pub referer: String,
    pub user_agent: String,
}

#[allow(clippy::expect_used)]
static COMBINED_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"^(\S+) \S+ \S+ \[([^\]]+)\] "(\S+) (\S+) [^"]*" (\d{3}) (\d+|-) "([^"]*)" "([^"]*)""#,
    )
    .expect("valid regex")
});

pub fn parse_log_line(line: &str, format: &LogFormat) -> Option<LogEntry> {
    match format {
        LogFormat::NginxCombined | LogFormat::ApacheCombined => parse_combined(line),
        LogFormat::JsonStructured => serde_json::from_str(line).ok(),
    }
}

fn parse_combined(line: &str) -> Option<LogEntry> {
    let caps = COMBINED_RE.captures(line)?;
    let size_str = caps.get(6)?.as_str();
    let size = if size_str == "-" {
        0
    } else {
        size_str.parse().unwrap_or(0)
    };

    Some(LogEntry {
        ip: caps.get(1)?.as_str().to_string(),
        timestamp: caps.get(2)?.as_str().to_string(),
        method: caps.get(3)?.as_str().to_string(),
        url: caps.get(4)?.as_str().to_string(),
        status: caps.get(5)?.as_str().parse().ok()?,
        size,
        referer: caps.get(7)?.as_str().to_string(),
        user_agent: caps.get(8)?.as_str().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_combined_200() {
        let line = r#"192.168.1.1 - frank [10/Oct/2023:13:55:36 -0700] "GET /index.html HTTP/1.1" 200 2326 "https://example.com" "Mozilla/5.0 (compatible; Googlebot/2.1; +http://www.google.com/bot.html)""#;
        let entry = parse_combined(line).unwrap();
        assert_eq!(entry.ip, "192.168.1.1");
        assert_eq!(entry.method, "GET");
        assert_eq!(entry.url, "/index.html");
        assert_eq!(entry.status, 200);
        assert_eq!(entry.size, 2326);
        assert_eq!(
            entry.user_agent,
            "Mozilla/5.0 (compatible; Googlebot/2.1; +http://www.google.com/bot.html)"
        );
    }

    #[test]
    fn test_parse_combined_dash_size() {
        let line = r#"10.0.0.1 - - [10/Oct/2023:14:00:00 +0000] "GET /api/data HTTP/1.1" 200 - "-" "curl/7.68.0""#;
        let entry = parse_combined(line).unwrap();
        assert_eq!(entry.size, 0);
        assert_eq!(entry.referer, "-");
    }

    #[test]
    fn test_parse_combined_404() {
        let line = r#"172.16.0.5 - - [10/Oct/2023:15:30:00 +0000] "GET /missing HTTP/1.1" 404 512 "https://example.com" "Mozilla/5.0""#;
        let entry = parse_combined(line).unwrap();
        assert_eq!(entry.status, 404);
    }

    #[test]
    fn test_parse_combined_invalid() {
        assert!(parse_combined("not a log line").is_none());
        assert!(parse_combined("").is_none());
    }

    #[test]
    fn test_parse_json_log() {
        let json = r#"{"ip":"1.2.3.4","timestamp":"2023-10-10T12:00:00Z","method":"GET","url":"/page","status":200,"size":1024,"referer":"-","user_agent":"Mozilla/5.0"}"#;
        let entry = parse_log_line(json, &LogFormat::JsonStructured).unwrap();
        assert_eq!(entry.ip, "1.2.3.4");
        assert_eq!(entry.status, 200);
    }

    #[test]
    fn test_parse_json_log_mismatched_field_names() {
        let json = r#"{"remote_addr":"1.2.3.4","request":"GET /page","status":200,"body_bytes_sent":512,"http_user_agent":"Googlebot","http_referer":"https://example.com","time_local":"10/Oct/2023:12:00:00 +0000"}"#;
        let entry = parse_log_line(json, &LogFormat::JsonStructured);
        // Standard field names don't match LogEntry fields, so this should fail
        assert!(entry.is_none());
    }
}
