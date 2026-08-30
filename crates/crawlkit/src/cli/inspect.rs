use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;

use super::util::decrypt_field;

/// Deep single-page analysis: fetch, parse, run all analyzers, optional CrUX/GSC.
pub async fn run(
    url_str: &str,
    output: Option<&Path>,
    format: &str,
    _javascript: bool,
    user_agent: Option<&str>,
    feature_flags: &crawlkit_engine::FeatureFlags,
) -> Result<()> {
    use crawlkit_engine::analyzers::AnalyzerRegistry;
    use crawlkit_engine::http::HttpClient;
    use crawlkit_engine::HtmlParser;

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner()),
    );

    let url = url::Url::parse(url_str).with_context(|| format!("Invalid URL: {url_str}"))?;

    let encryption = crawlkit_engine::EncryptionManager::default();

    let default_ua = format!("crawlkit/{}", env!("CARGO_PKG_VERSION"));
    let ua = user_agent.unwrap_or(&default_ua);
    let http_config = crawlkit_engine::http::HttpClientConfig {
        timeout: std::time::Duration::from_secs(30),
        max_redirects: 20,
        retry_policy: crawlkit_engine::http::RetryPolicy::default(),
        user_agent: std::sync::Arc::new(crawlkit_engine::http::UserAgentRotator::new(vec![
            ua.to_string()
        ])),
        max_body_size: 10 * 1024 * 1024,
        pool_max_idle_per_host: 32,
        pool_max_idle: 64,
        tcp_keepalive: Some(std::time::Duration::from_secs(30)),
        pool_idle_timeout: std::time::Duration::from_secs(90),
        allow_http: false,
        connect_timeout: std::time::Duration::from_secs(10),
        seed: None,
    };
    let client = HttpClient::new(http_config).context("Failed to create HTTP client")?;
    let config = crawlkit_engine::CrawlConfig::default();
    let registry = AnalyzerRegistry::new(&config);

    pb.set_message(format!("Fetching {url_str}..."));
    let start = std::time::Instant::now();
    let result = client.fetch(&url).await.context("Failed to fetch URL")?;
    let fetch_time = start.elapsed();

    pb.set_message("Parsing HTML...");
    let parsed = HtmlParser::parse(&result.body, &url);

    pb.set_message("Running analyzers...");
    let headers_vec: Vec<(String, String)> = result.headers.clone();
    let empty_chain: Vec<crawlkit_engine::RedirectHop> = vec![];
    let server = headers_vec
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("server"))
        .map(|(_, v)| v.as_str());
    let content_type = headers_vec
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
        .map(|(_, v)| v.as_str());
    let ctx = crawlkit_engine::analyzers::AnalysisContext {
        page: &parsed,
        body: Some(&result.body),
        status_code: Some(result.status_code),
        headers: &headers_vec,
        response_time: Some(fetch_time),
        redirect_chain: &empty_chain,
        robots_txt: None,
        body_size: Some(result.body.len()),
        compressed_size: None,
        server,
        content_type,
        rendered: None,
    };
    let findings = registry.analyze(&ctx);

    let crux_data = if feature_flags.get(crawlkit_engine::feature_flags::FLAG_RUM_INTEGRATION) {
        let adapter = crawlkit_engine::CruxAdapter::from_env();
        if adapter.is_available() {
            pb.set_message("Fetching CrUX data from PageSpeed Insights...");
            adapter.fetch_crux_data(url_str).await.ok().flatten()
        } else {
            None
        }
    } else {
        None
    };

    let issues_by_severity: std::collections::HashMap<String, usize> = {
        let mut map = std::collections::HashMap::new();
        for f in &findings {
            *map.entry(f.severity.as_str().to_string()).or_insert(0) += 1;
        }
        map
    };
    let issues_by_category: std::collections::HashMap<String, usize> = {
        let mut map = std::collections::HashMap::new();
        for f in &findings {
            *map.entry(f.category.as_str().to_string()).or_insert(0) += 1;
        }
        map
    };

    let output_str = match format {
        "json" => build_json(
            url_str,
            &result,
            &parsed,
            &findings,
            &issues_by_severity,
            &issues_by_category,
            fetch_time,
            &encryption,
            &crux_data,
        )?,
        "md" => build_markdown(
            url_str,
            &result,
            &parsed,
            &findings,
            fetch_time,
            &encryption,
            &crux_data,
        ),
        _ => {
            return Err(anyhow::anyhow!(
                "Unsupported format: {format}. Use json or md."
            ))
        }
    };

    pb.finish_with_message(format!(
        "Inspection complete: {} findings ({} errors, {} warnings)",
        findings.len(),
        issues_by_severity.get("Error").unwrap_or(&0),
        issues_by_severity.get("Warning").unwrap_or(&0),
    ));

    if let Some(out) = output {
        std::fs::write(out, &output_str)?;
        tracing::info!("Wrote inspection to {}", out.display());
    } else {
        println!("{output_str}");
    }

    Ok(())
}

fn build_json(
    url_str: &str,
    result: &crawlkit_engine::FetchResult,
    parsed: &crawlkit_engine::ParsedPage,
    findings: &[crawlkit_engine::Finding],
    issues_by_severity: &std::collections::HashMap<String, usize>,
    issues_by_category: &std::collections::HashMap<String, usize>,
    fetch_time: std::time::Duration,
    encryption: &crawlkit_engine::EncryptionManager,
    crux_data: &Option<crawlkit_engine::CruxData>,
) -> Result<String> {
    let issues: Vec<serde_json::Value> = findings
        .iter()
        .map(|f| {
            serde_json::json!({
                "severity": f.severity.as_str(),
                "category": f.category.as_str(),
                "code": f.code,
                "title": f.title,
                "description": f.description,
                "recommendation": f.recommendation,
            })
        })
        .collect();

    Ok(serde_json::to_string_pretty(&serde_json::json!({
        "url": url_str,
        "status_code": result.status_code,
        "final_url": result.final_url.to_string(),
        "fetch_time_ms": fetch_time.as_millis(),
        "body_size": result.body.len(),
        "title": decrypt_field(encryption, &parsed.meta.title),
        "description": decrypt_field(encryption, &parsed.meta.description),
        "canonical": parsed.meta.canonical,
        "word_count": parsed.word_count,
        "links": parsed.links.len(),
        "images": parsed.images.len(),
        "headings": parsed.headings.len(),
        "findings_count": findings.len(),
        "issues_by_severity": issues_by_severity,
        "issues_by_category": issues_by_category,
        "findings": issues,
        "crux": crux_data.as_ref().map(|d| serde_json::json!({
            "lcp_p75_ms": d.lcp_p75,
            "inp_p75_ms": d.inp_p75,
            "cls_p75": d.cls_p75,
            "fcp_p75_ms": d.fcp_p75,
            "ttfb_p75_ms": d.ttfb_p75,
        })),
    }))?)
}

fn build_markdown(
    url_str: &str,
    result: &crawlkit_engine::FetchResult,
    parsed: &crawlkit_engine::ParsedPage,
    findings: &[crawlkit_engine::Finding],
    fetch_time: std::time::Duration,
    encryption: &crawlkit_engine::EncryptionManager,
    crux_data: &Option<crawlkit_engine::CruxData>,
) -> String {
    let mut md = format!(
        "# Page Inspection: {url_str}\n\n\
        - **Status:** {}\n\
        - **Final URL:** {}\n\
        - **Fetch Time:** {:.0}ms\n\
        - **Body Size:** {} bytes\n\
        - **Title:** {}\n\
        - **Description:** {}\n\
        - **Canonical:** {}\n\
        - **Word Count:** {}\n\
        - **Links:** {} internal/external\n\
        - **Images:** {}\n\
        - **Headings:** {}\n",
        result.status_code,
        result.final_url,
        fetch_time.as_millis(),
        result.body.len(),
        parsed
            .meta
            .title
            .as_ref()
            .and_then(|t| decrypt_field(encryption, &Some(t.clone())))
            .as_deref()
            .unwrap_or("(none)"),
        parsed
            .meta
            .description
            .as_ref()
            .and_then(|d| decrypt_field(encryption, &Some(d.clone())))
            .as_deref()
            .unwrap_or("(none)"),
        parsed
            .meta
            .canonical
            .as_ref()
            .map(|u| u.as_str())
            .unwrap_or("(none)"),
        parsed.word_count,
        parsed.links.len(),
        parsed.images.len(),
        parsed.headings.len(),
    );

    md.push_str(&format!("\n## Findings ({} total)\n\n", findings.len()));

    if findings.is_empty() {
        md.push_str("No issues found.\n");
    } else {
        md.push_str("| Severity | Code | Title |\n");
        md.push_str("|----------|------|-------|\n");
        for f in findings {
            md.push_str(&format!(
                "| {} | {} | {} |\n",
                f.severity.as_str(),
                f.code,
                f.title
            ));
        }
    }

    if let Some(d) = crux_data {
        md.push_str("\n## Core Web Vitals (CrUX p75)\n\n");
        md.push_str("| Metric | Value |\n|---|---|\n");
        if let Some(v) = d.lcp_p75 {
            md.push_str(&format!("| LCP | {v:.0}ms |\n"));
        }
        if let Some(v) = d.cls_p75 {
            md.push_str(&format!("| CLS | {v:.3} |\n"));
        }
        if let Some(v) = d.inp_p75 {
            md.push_str(&format!("| INP | {v:.0}ms |\n"));
        }
        if let Some(v) = d.fcp_p75 {
            md.push_str(&format!("| FCP | {v:.0}ms |\n"));
        }
        if let Some(v) = d.ttfb_p75 {
            md.push_str(&format!("| TTFB | {v:.0}ms |\n"));
        }
    }

    md
}
