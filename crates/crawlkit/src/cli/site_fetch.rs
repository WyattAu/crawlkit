//! Shared polite site-fetch loop behind `crawlkit clone` and `crawlkit md`
//! (GAP G8).
//!
//! This is deliberately a thin composition of the existing crawl engine's
//! building blocks — [`HttpClient`] for fetching, [`RateLimiter`] built from
//! a real [`CrawlConfig`] for pacing, [`RobotsTxtCache`] for robots
//! compliance, and [`HtmlParser`] for link extraction and URL
//! canonicalization. It is not a second crawler: every request a clone or
//! markdown run makes passes through the same rate limiter a crawl uses, so
//! the politeness invariant holds.

use anyhow::{Context, Result};
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use std::time::Duration;
use url::Url;

use crawlkit_engine::http::HttpClient;
use crawlkit_engine::parser::ParsedPage;
use crawlkit_engine::ratelimit::RateLimiter;
use crawlkit_engine::robots::RobotsTxtCache;
use crawlkit_engine::{CrawlConfig, HtmlParser};

/// Options shared by `clone` and `md`.
#[derive(Debug, Clone)]
pub(crate) struct SiteFetchOptions {
    /// Where to start crawling.
    pub start: Url,
    /// Page budget. Includes the start page.
    pub max_pages: usize,
    /// Delay between same-domain requests (drives the shared rate limiter).
    pub request_delay: Duration,
    /// User-Agent to identify as.
    pub user_agent: String,
    /// Per-request timeout.
    pub timeout: Duration,
    /// Whether robots.txt rules are honored.
    pub respect_robots: bool,
}

impl SiteFetchOptions {
    /// Options with crawlkit's default pacing and politeness for the flags
    /// the `clone`/`md` commands do not expose: identify honestly, honor
    /// robots.txt, use the crawl-default 30s request timeout.
    pub(crate) fn defaults() -> Self {
        Self {
            start: Url::parse("https://example.invalid/")
                .unwrap_or_else(|_| unreachable!("static URL string is always valid")),
            max_pages: 50,
            request_delay: Duration::from_millis(500),
            user_agent: format!("crawlkit-clone/{}", env!("CARGO_PKG_VERSION")),
            timeout: Duration::from_secs(30),
            respect_robots: true,
        }
    }
}

/// One fetched document, as the mirror needs it.
#[derive(Debug, Clone)]
pub(crate) struct FetchedPage {
    /// The URL after redirects (where relative links actually resolve).
    pub final_url: Url,
    /// Response body.
    pub body: String,
    /// Whether the response is an HTML document worth link-following.
    pub is_html: bool,
}

/// Summary of one site fetch.
#[derive(Debug, Clone, Default)]
pub(crate) struct SiteFetchStats {
    /// Pages and assets fetched (2xx responses).
    pub fetched: usize,
    /// Responses with a non-success status, or transport errors.
    pub failed: usize,
    /// Links not queued because they leave the start origin.
    pub skipped_external: usize,
    /// URLs skipped by robots.txt.
    pub skipped_robots: usize,
}

/// Whether `candidate` stays on the origin `start` was fetched from.
///
/// Same-origin (scheme + host + port), not merely same-host: cloning `http://`
/// pages into an `https://` mirror would rewrite links to a scheme the files
/// no longer speak.
pub(crate) fn is_same_origin(start: &Url, candidate: &Url) -> bool {
    start.origin() == candidate.origin() && matches!(candidate.scheme(), "http" | "https")
}

/// The robots-cache key for a URL: `host:port` (or bare host), matching the
/// engine's `authority_of` so non-default ports get their own robots.txt.
pub(crate) fn authority_of(url: &Url) -> String {
    let host = url.host_str().unwrap_or_default();
    match url.port() {
        Some(port) => format!("{host}:{port}"),
        None => host.to_string(),
    }
}

/// Fetch a site breadth-first, politely, same-origin only.
///
/// Pages come back in crawl order (start first), which makes both consumers
/// deterministic: `clone` writes `index.html` for the start URL and `md`
/// leads the merged document with the entry page.
pub(crate) async fn fetch_site(
    opts: &SiteFetchOptions,
) -> Result<(Vec<FetchedPage>, SiteFetchStats)> {
    let crawl_config = CrawlConfig {
        start_url: opts.start.clone(),
        max_pages: opts.max_pages,
        request_delay: opts.request_delay,
        request_timeout: opts.timeout,
        user_agent: opts.user_agent.clone(),
        respect_robots_txt: opts.respect_robots,
        ..CrawlConfig::default()
    };

    let http_config = crawlkit_engine::http::HttpClientConfig {
        timeout: opts.timeout,
        max_redirects: crawl_config.max_redirects,
        retry_policy: crawlkit_engine::http::RetryPolicy::default(),
        user_agent: Arc::new(crawlkit_engine::http::UserAgentRotator::new(vec![opts
            .user_agent
            .clone()])),
        max_body_size: 32 * 1024 * 1024,
        pool_max_idle_per_host: 8,
        pool_max_idle: 16,
        tcp_keepalive: Some(Duration::from_secs(30)),
        pool_idle_timeout: Duration::from_secs(90),
        // Local fixture servers and plain-http origins are legitimate clone
        // targets; pointing the command at an http origin is an explicit act.
        allow_http: true,
        connect_timeout: Duration::from_secs(10),
        seed: None,
    };
    let client = Arc::new(HttpClient::new(http_config).context("creating HTTP client")?);

    // The same rate limiter the crawl engine builds from a CrawlConfig:
    // per-domain token buckets paced by `request_delay`, plus a global
    // bucket. Clone must never crawl faster than a crawl would.
    let rate_limiter = RateLimiter::from_crawl_config(&crawl_config);
    let robots = RobotsTxtCache::new(Arc::clone(&client), &crawl_config);

    let mut stats = SiteFetchStats::default();
    let mut seen: HashSet<Url> = HashSet::new();
    let mut queue: VecDeque<Url> = VecDeque::new();
    queue.push_back(opts.start.clone());
    seen.insert(opts.start.clone());

    let mut pages: Vec<FetchedPage> = Vec::new();

    while let Some(url) = queue.pop_front() {
        if pages.len() >= opts.max_pages {
            break;
        }

        if opts.respect_robots {
            let domain = authority_of(&url);
            let scheme = url.scheme();
            if robots.is_disallowed(scheme, &domain, url.path()).await {
                stats.skipped_robots += 1;
                continue;
            }
            // A robots crawl-delay tightens the shared limiter for this
            // domain, exactly as the crawl engine does.
            if let Some(delay_secs) = robots.crawl_delay(scheme, &domain).await {
                rate_limiter.set_crawl_delay(&domain, Duration::from_secs_f64(delay_secs));
            }
        }

        // Acquire from the shared limiter before every request.
        rate_limiter
            .acquire(url.host_str().unwrap_or_default())
            .await
            .context("rate limiter wait")?;

        let result = match client.fetch(&url).await {
            Ok(result) if (200..300).contains(&result.status_code) => result,
            Ok(result) => {
                stats.failed += 1;
                tracing::debug!("site fetch: {} returned {}", url, result.status_code);
                continue;
            }
            Err(error) => {
                stats.failed += 1;
                tracing::debug!("site fetch: {} failed: {error}", url);
                continue;
            }
        };

        let content_type = result
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
            .map(|(_, v)| v.to_ascii_lowercase());
        let media = content_type
            .as_deref()
            .and_then(|ct| ct.split(';').next())
            .map(str::trim)
            .map(str::to_string);
        let is_html = media.as_deref().is_none_or(|m| {
            m == "text/html" || m == "application/xhtml+xml" || m.ends_with("+xml")
        });

        let page_url = result.final_url.clone();
        let page = FetchedPage {
            final_url: page_url.clone(),
            body: result.body,
            is_html,
        };

        // Follow same-origin links (and fetch same-origin assets) out of
        // HTML documents, within the remaining page budget.
        if is_html && pages.len() < opts.max_pages {
            let parsed = HtmlParser::parse(&page.body, &page_url);
            let links = same_origin_links(&parsed, &opts.start, &mut stats);
            let assets = same_origin_assets(&parsed, &opts.start, &mut stats);
            for target in links.into_iter().chain(assets) {
                if seen.insert(target.clone()) {
                    queue.push_back(target);
                }
            }
        }

        stats.fetched += 1;
        pages.push(page);
    }

    Ok((pages, stats))
}

/// Same-origin, queueable links from a parsed page. The parser has already
/// resolved hrefs against the page URL; this filters to the start origin and
/// canonicalizes queue keys (no fragments, no default ports).
fn same_origin_links(parsed: &ParsedPage, start: &Url, stats: &mut SiteFetchStats) -> Vec<Url> {
    let mut out = Vec::new();
    for link in &parsed.links {
        let Ok(url) = Url::parse(&link.href) else {
            continue;
        };
        if !is_same_origin(start, &url) {
            stats.skipped_external += 1;
            continue;
        }
        out.push(canonical_queue_url(&url));
    }
    out.sort();
    out.dedup();
    out
}

/// Same-origin asset URLs (images, scripts, stylesheets) from a parsed page.
fn same_origin_assets(parsed: &ParsedPage, start: &Url, stats: &mut SiteFetchStats) -> Vec<Url> {
    let mut raw: Vec<String> = Vec::new();
    for image in &parsed.images {
        if !image.src.is_empty() {
            raw.push(image.src.clone());
        }
    }
    for script in &parsed.scripts {
        if let Some(src) = &script.src {
            raw.push(src.clone());
        }
    }
    for style in &parsed.styles {
        if let Some(href) = &style.href {
            raw.push(href.clone());
        }
    }
    let mut out = Vec::new();
    for href in raw {
        let Ok(url) = start.join(&href) else {
            continue;
        };
        if !is_same_origin(start, &url) {
            stats.skipped_external += 1;
            continue;
        }
        out.push(canonical_queue_url(&url));
    }
    out.sort();
    out.dedup();
    out
}

/// Queue-key canonicalization: strip the fragment and default ports so
/// `seen` does not enqueue the same document twice under two spellings.
pub(crate) fn canonical_queue_url(url: &Url) -> Url {
    let mut normalized = url.clone();
    normalized.set_fragment(None);
    if (normalized.scheme() == "http" && normalized.port() == Some(80))
        || (normalized.scheme() == "https" && normalized.port() == Some(443))
    {
        normalized.set_port(None).ok();
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_origin_requires_scheme_host_and_port() {
        let start = Url::parse("https://example.test/docs").unwrap();
        assert!(is_same_origin(
            &start,
            &Url::parse("https://example.test/x").unwrap()
        ));
        assert!(is_same_origin(
            &start,
            &Url::parse("https://example.test:443/y").unwrap()
        ));
        assert!(!is_same_origin(
            &start,
            &Url::parse("http://example.test/").unwrap()
        ));
        assert!(!is_same_origin(
            &start,
            &Url::parse("https://other.test/").unwrap()
        ));
        assert!(!is_same_origin(
            &start,
            &Url::parse("https://example.test:8443/").unwrap()
        ));
    }

    #[test]
    fn canonical_queue_url_drops_fragments_and_default_ports() {
        let a = canonical_queue_url(&Url::parse("https://e.test/p#sec").unwrap());
        assert_eq!(a.as_str(), "https://e.test/p");
        let b = canonical_queue_url(&Url::parse("http://e.test:80/p").unwrap());
        assert_eq!(b.as_str(), "http://e.test/p");
    }

    #[test]
    fn authority_matches_engine_host_port_spelling() {
        assert_eq!(
            authority_of(&Url::parse("https://e.test:8443/p").unwrap()),
            "e.test:8443"
        );
        assert_eq!(
            authority_of(&Url::parse("https://e.test/p").unwrap()),
            "e.test"
        );
    }
}
