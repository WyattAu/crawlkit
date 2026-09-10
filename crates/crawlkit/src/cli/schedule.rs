//! Schedule subcommands: manage recurring crawls on a crawlkit-api server.
//!
//! Thin client over the existing REST endpoints (`POST/GET/PATCH/DELETE
//! /api/v1/schedules`); no new persistence or server behavior. Credentials
//! come from `CRAWLKIT_API_KEY` (sent as `X-API-Key`) or
//! `CRAWLKIT_JWT` (sent as `Authorization: Bearer`), matching the API's
//! documented auth middleware.

use anyhow::{bail, Context, Result};

/// Base URL of the crawlkit-api server (no trailing slash handling needed;
/// normalized before use).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthKind {
    ApiKey,
    Bearer,
}

/// Resolve credentials from the environment. Errors name the missing variable
/// so operators can fix CI secrets without reading source.
fn resolve_auth() -> Result<(String, AuthKind)> {
    if let Ok(key) = std::env::var("CRAWLKIT_API_KEY") {
        if !key.trim().is_empty() {
            return Ok((key, AuthKind::ApiKey));
        }
    }
    if let Ok(token) = std::env::var("CRAWLKIT_JWT") {
        if !token.trim().is_empty() {
            return Ok((token, AuthKind::Bearer));
        }
    }
    bail!(
        "no credentials: set CRAWLKIT_API_KEY (X-API-Key) or CRAWLKIT_JWT (Bearer) \
         for the target crawlkit-api server"
    );
}

fn normalize_base(base: &str) -> String {
    base.trim_end_matches('/').to_string()
}

/// Issue a JSON request against the schedules API and return the status code
/// plus raw body. Uses the engine's HTTP stack via `reqwest` (already a
/// dependency of the `full` feature set).
/// Returns the `(header name, header value)` pair for the given credentials,
/// matching the API's middleware expectations (`X-API-Key` or Bearer JWT).
fn auth_header(credentials: String, kind: AuthKind) -> (String, String) {
    match kind {
        AuthKind::ApiKey => ("X-API-Key".to_string(), credentials),
        AuthKind::Bearer => ("Authorization".to_string(), format!("Bearer {credentials}")),
    }
}

async fn request(
    base: &str,
    method: reqwest::Method,
    path: &str,
    body: Option<String>,
) -> Result<(u16, String)> {
    let (auth, kind) = resolve_auth()?;
    let url = format!("{}{}", normalize_base(base), path);

    let mut builder = reqwest::Client::new()
        .request(method, &url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json");

    let (header_name, header_value) = auth_header(auth, kind);
    builder = builder.header(header_name, header_value);

    if let Some(json) = body {
        builder = builder.body(json);
    }

    let response = builder.send().await.with_context(|| format!("request to {url} failed"))?;
    let status = u16::from(response.status());
    let text = response.text().await.unwrap_or_default();
    Ok((status, text))
}

fn print_schedule_list(body: &str) -> Result<()> {
    #[derive(serde::Deserialize)]
    struct ScheduleRow {
        id: String,
        start_url: String,
        interval_secs: u64,
        enabled: bool,
        next_run: chrono::DateTime<chrono::Utc>,
    }
    let rows: Vec<ScheduleRow> =
        serde_json::from_str(body).context("unexpected response body from list endpoint")?;
    if rows.is_empty() {
        println!("no schedules");
        return Ok(());
    }
    println!("{:<38} {:<45} {:>8} {:<8} NEXT RUN", "ID", "START URL", "INTERVAL", "STATE");
    for r in rows {
        println!(
            "{:<38} {:<45} {:>7}s {:<8} {}",
            r.id,
            r.start_url,
            r.interval_secs,
            if r.enabled { "enabled" } else { "disabled" },
            r.next_run.to_rfc3339(),
        );
    }
    Ok(())
}

fn require_created(status: u16, body: &str) -> Result<()> {
    if status == 201 {
        println!("schedule created");
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(body) {
            if let Some(id) = v.get("id").and_then(|i| i.as_str()) {
                println!("  id: {id}");
            }
        }
        Ok(())
    } else {
        bail!("create failed (HTTP {status}): {body}")
    }
}

/// Subcommands parsed in `cli::mod` and dispatched here.
#[derive(Debug, Clone)]
pub enum ScheduleAction {
    Add {
        base: String,
        url: String,
        interval_secs: u64,
        max_pages: usize,
        delay_ms: u64,
        concurrency: usize,
    },
    List {
        base: String,
    },
    Enable {
        base: String,
        id: String,
        enabled: bool,
    },
    Remove {
        base: String,
        id: String,
    },
}

/// Execute a schedule action. `async` because every path performs HTTP I/O.
pub async fn run(action: ScheduleAction) -> Result<()> {
    match action {
        ScheduleAction::Add {
            base,
            url,
            interval_secs,
            max_pages,
            delay_ms,
            concurrency,
        } => {
            let payload = serde_json::json!({
                "start_url": url,
                "interval_secs": interval_secs,
                "max_pages": max_pages,
                "request_delay_ms": delay_ms,
                "concurrency": concurrency,
            });
            let (status, body) = request(&base, reqwest::Method::POST, "/api/v1/schedules", Some(payload.to_string())).await?;
            require_created(status, &body)
        }
        ScheduleAction::List { base } => {
            let (status, body) = request(&base, reqwest::Method::GET, "/api/v1/schedules", None).await?;
            if status != 200 {
                bail!("list failed (HTTP {status}): {body}");
            }
            print_schedule_list(&body)
        }
        ScheduleAction::Enable {
            base,
            id,
            enabled,
        } => {
            let path = format!("/api/v1/schedules/{id}");
            let payload = serde_json::json!({ "enabled": enabled });
            let (status, body) = request(&base, reqwest::Method::PATCH, &path, Some(payload.to_string())).await?;
            if status == 200 {
                println!(
                    "schedule {id} {}",
                    if enabled { "enabled" } else { "disabled" }
                );
                Ok(())
            } else {
                bail!("update failed (HTTP {status}): {body}")
            }
        }
        ScheduleAction::Remove { base, id } => {
            let path = format!("/api/v1/schedules/{id}");
            let (status, body) = request(&base, reqwest::Method::DELETE, &path, None).await?;
            if status == 204 {
                println!("schedule {id} removed");
                Ok(())
            } else {
                bail!("remove failed (HTTP {status}): {body}")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_base_strips_trailing_slash() {
        assert_eq!(normalize_base("http://localhost:8080/"), "http://localhost:8080");
        assert_eq!(normalize_base("http://localhost:8080"), "http://localhost:8080");
    }

    #[test]
    fn auth_header_forms() {
        let (name, value) = auth_header("k1".into(), AuthKind::ApiKey);
        assert_eq!(format!("{name}: {value}"), "X-API-Key: k1");
        let (name, value) = auth_header("t1".into(), AuthKind::Bearer);
        assert_eq!(format!("{name}: {value}"), "Authorization: Bearer t1");
    }

    #[test]
    fn missing_credentials_error_names_variable() {
        // Ensure neither var is set in the test environment for determinism.
        std::env::remove_var("CRAWLKIT_API_KEY");
        std::env::remove_var("CRAWLKIT_JWT");
        let err = resolve_auth().unwrap_err().to_string();
        assert!(err.contains("CRAWLKIT_API_KEY"), "error must name the env var: {err}");
    }

    #[test]
    fn auth_kind_selection_prefers_api_key() {
        std::env::set_var("CRAWLKIT_API_KEY", "k-test");
        std::env::set_var("CRAWLKIT_JWT", "t-test");
        let (auth, kind) = resolve_auth().expect("credentials present");
        assert_eq!(auth, "k-test");
        assert_eq!(kind, AuthKind::ApiKey);
        std::env::remove_var("CRAWLKIT_API_KEY");
        std::env::remove_var("CRAWLKIT_JWT");
    }
}
