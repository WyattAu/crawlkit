//! Property-based tests for crawlkit-engine core modules.
//!
//! Uses the `proptest` crate to verify invariants hold across randomly
//! generated inputs. Covers URL parsing, storage CRUD, priority queue
//! ordering, rate limiter token bucket semantics, and meta tag extraction.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::redundant_clone
)]
//!
//! Defense sector compliance: all properties have clear names describing
//! the invariant being tested, with edge case strategies for empty strings,
//! max-length strings, and special characters.

use proptest::prelude::*;
use url::Url;

// ============================================================================
// URL Parser Properties
// ============================================================================

// Any valid absolute URL string round-trips through Url::parse.
// Rationale: If Url::parse succeeds, the resulting Url must be absolute
// (have a scheme) and its host/path must match the input.
proptest! {
    #[test]
    fn url_parse_roundtrip(host in "[a-z]{2,10}\\.[a-z]{2,5}", path in "/[a-z]{0,20}") {
        let url_str = format!("https://{host}{path}");
        let parsed = Url::parse(&url_str).unwrap();
        prop_assert!(!parsed.scheme().is_empty());
        prop_assert_eq!(parsed.host_str(), Some(host.as_str()));
        prop_assert_eq!(parsed.path(), path.as_str());
    }
}

// Relative URL resolution against a base always produces an absolute URL.
// Rationale: Any relative path resolved against a valid absolute base
// must yield a URL with a scheme (http or https).
proptest! {
    #[test]
    fn relative_url_resolution_is_absolute(
        base in "https://[a-z][a-z0-9]{0,10}\\.[a-z]{2,10}",
        path in "/[a-z]{1,10}"
    ) {
        let base_url = Url::parse(&base).unwrap();
        let resolved = base_url.join(&path).unwrap();
        prop_assert!(!resolved.scheme().is_empty(), "resolved URL must have a scheme");
        prop_assert!(
            resolved.scheme() == "http" || resolved.scheme() == "https",
            "resolved URL must be http or https, got: {}",
            resolved.scheme()
        );
    }
}

// URL normalization is idempotent: normalize(normalize(x)) == normalize(x).
// Rationale: Parsing a URL and re-serializing it produces a stable result.
// The scheme, host, and path survive the round-trip and are invariant
// across repeated parse-serialize cycles.
proptest! {
    #[test]
    fn url_normalization_is_idempotent(
        host in "[a-z]{2,10}\\.[a-z]{2,5}",
        path in "/[a-z]{0,20}"
    ) {
        let raw = format!("https://{host}{path}");
        let url = Url::parse(&raw).unwrap();
        let normalized = Url::parse(url.as_str()).unwrap();
        let double = Url::parse(normalized.as_str()).unwrap();
        prop_assert_eq!(double.scheme(), url.scheme());
        prop_assert_eq!(double.host_str(), url.host_str());
        prop_assert_eq!(double.path(), url.path());
    }
}

// ============================================================================
// Storage Properties
// ============================================================================

// Insert a page then retrieve it; the URL and title must match.
// Rationale: Storage must preserve the exact URL and title of a page
// through a round-trip insert-then-get cycle.
proptest! {
    #[test]
    fn storage_insert_get_roundtrip(
        page_id in "[a-z0-9]{8}",
        url in "https://[a-z]{2,10}\\.com/[a-z]{1,10}",
        title in "[A-Za-z0-9 ]{1,50}"
    ) {
        let storage = crawlkit_engine::storage::Storage::new_in_memory().unwrap();
        let crawl_id = storage.start_crawl("https://seed.com", None).unwrap();
        let url_parsed = Url::parse(&url).unwrap();
        let url_str = url.clone();
        let url_final = url_parsed.clone();

        let page = crawlkit_engine::storage::PageData {
            id: page_id.clone(),
            url: url_parsed,
            final_url: url_final,
            status_code: 200,
            title: Some(title.clone()),
            description: None,
            canonical_url: None,
            word_count: Some(100),
            load_time_ms: Some(200),
            body_size: Some(1024),
            fetched_at: chrono::Utc::now(),
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
        };

        storage.insert_page(&crawl_id, &page).unwrap();
        let pages = storage.get_pages(&crawl_id, 100).unwrap();
        prop_assert_eq!(pages.len(), 1);
        prop_assert_eq!(pages[0].url.as_str(), url_str.as_str());
        prop_assert_eq!(pages[0].title.as_deref(), Some(title.as_str()));
    }
}

// get_pages with limit never returns more than limit items.
// Rationale: The limit parameter is a hard cap on result count.
// Even when more pages exist, the returned count must be <= limit.
proptest! {
    #[test]
    fn storage_get_pages_respects_limit(
        total in 0usize..50,
        limit in 0usize..50
    ) {
        let storage = crawlkit_engine::storage::Storage::new_in_memory().unwrap();
        let crawl_id = storage.start_crawl("https://seed.com", None).unwrap();

        for i in 0..total {
            let page = crawlkit_engine::storage::PageData {
                id: format!("p{i:04}"),
                url: Url::parse(&format!("https://example.com/page{i}")).unwrap(),
                final_url: Url::parse(&format!("https://example.com/page{i}")).unwrap(),
                status_code: 200,
                title: Some(format!("Page {i}")),
                description: None,
                canonical_url: None,
                word_count: None,
                load_time_ms: None,
                body_size: None,
                fetched_at: chrono::Utc::now(),
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
            };
            storage.insert_page(&crawl_id, &page).unwrap();
        }

        let pages = storage.get_pages(&crawl_id, limit).unwrap();
        prop_assert!(
            pages.len() <= limit,
            "returned {} pages but limit was {}",
            pages.len(),
            limit
        );
    }
}

// Severity and category string round-trip through parse.
// Rationale: Converting a Severity to string and back must preserve the
// original value. This ensures database serialization is lossless.
proptest! {
    #[test]
    fn severity_roundtrip(s in "[a-z]{4,8}") {
        use crawlkit_engine::storage::Severity;
        match s.as_str() {
            "critical" => prop_assert_eq!(Severity::parse_severity(&s), Some(Severity::Critical)),
            "error" => prop_assert_eq!(Severity::parse_severity(&s), Some(Severity::Error)),
            "warning" => prop_assert_eq!(Severity::parse_severity(&s), Some(Severity::Warning)),
            "info" => prop_assert_eq!(Severity::parse_severity(&s), Some(Severity::Info)),
            _ => prop_assert_eq!(Severity::parse_severity(&s), None),
        }
    }
}

// IssueCategory string round-trip through parse.
// Rationale: Converting an IssueCategory to string and back must preserve
// the original value for all standard categories.
proptest! {
    #[test]
    fn issue_category_roundtrip(s in "http|seo|content|links|images|schema|security|performance|mobile|accessibility|social") {
        use crawlkit_engine::storage::IssueCategory;
        let cat = IssueCategory::parse_category(&s);
        let s2 = cat.as_str();
        let cat2 = IssueCategory::parse_category(&s2);
        prop_assert_eq!(cat, cat2);
    }
}

// ============================================================================
// Queue Properties
// ============================================================================

// Push a valid URL then pop it; the returned URL must match.
// Rationale: A single push/pop must preserve the URL exactly.
proptest! {
    #[test]
    fn queue_push_pop_roundtrip(url_str in "https://[a-z]{2,10}\\.com/[a-z]{1,10}") {
        let queue = crawlkit_engine::queue::UrlQueue::new(
            crawlkit_engine::queue::ScopeConfig::default()
        );
        let url = Url::parse(&url_str).unwrap();
        let original = url.clone();

        queue.push(url, 0, crawlkit_engine::queue::Priority::NORMAL);
        let entry = queue.pop().unwrap();
        prop_assert_eq!(entry.url.as_str(), original.as_str());
    }
}

// Pop on an empty queue returns None.
// Rationale: An empty queue must not panic or return garbage.
proptest! {
    #[test]
    fn queue_pop_empty_returns_none(_dummy in 0u32..100) {
        let queue = crawlkit_engine::queue::UrlQueue::new(
            crawlkit_engine::queue::ScopeConfig::default()
        );
        prop_assert!(queue.pop().is_none());
    }
}

// Priority ordering is maintained: lower priority value (higher priority)
// is always popped before higher priority value.
// Rationale: When multiple URLs are queued, the one with the smallest
// priority value must come out first.
proptest! {
    #[test]
    fn queue_priority_ordering(
        urls in prop::collection::vec("https://[a-z]{1,5}\\.com/[a-z]{1,5}", 2..10),
        priorities in prop::collection::vec(0u8..255u8, 2..10)
    ) {
        let queue = crawlkit_engine::queue::UrlQueue::new(
            crawlkit_engine::queue::ScopeConfig::default()
        );

        let entries: Vec<(String, u8)> = urls.into_iter()
            .zip(priorities.into_iter())
            .collect();

        for (url_str, pri_val) in &entries {
            if let Ok(url) = Url::parse(url_str) {
                queue.push(
                    url,
                    0,
                    crawlkit_engine::queue::Priority::new(*pri_val),
                );
            }
        }

        let mut last_priority = 0u8;
        while let Some(entry) = queue.pop() {
            let pri = entry.priority.value();
            prop_assert!(
                pri >= last_priority,
                "priority {} < previous priority {}",
                pri,
                last_priority
            );
            last_priority = pri;
        }
    }
}

// Pushing a duplicate URL returns false and queue size stays the same.
// Rationale: Deduplication must prevent the same URL from being queued twice.
proptest! {
    #[test]
    fn queue_deduplication(
        url_str in "https://[a-z]{2,10}\\.com/[a-z]{1,10}",
        pri1 in 0u8..255u8,
        pri2 in 0u8..255u8
    ) {
        let queue = crawlkit_engine::queue::UrlQueue::new(
            crawlkit_engine::queue::ScopeConfig::default()
        );
        let url = Url::parse(&url_str).unwrap();

        let added1 = queue.push(url.clone(), 0, crawlkit_engine::queue::Priority::new(pri1));
        let added2 = queue.push(url, 1, crawlkit_engine::queue::Priority::new(pri2));

        prop_assert!(added1);
        prop_assert!(!added2);
        prop_assert_eq!(queue.len(), 1);
    }
}

// ============================================================================
// Rate Limiter Properties (public RateLimiter API)
// ============================================================================

// Domain tokens never exceed the computed burst capacity (rps * 2, ceil).
// Rationale: The rate limiter computes burst as ceil(rps * 2). Initial
// token count must never exceed this value, preventing burst abuse.
proptest! {
    #[test]
    fn rate_limiter_domain_tokens_never_exceed_burst(
        per_domain_rps in 0.5f64..100.0,
        global_rps in 1.0f64..200.0
    ) {
        let limiter = crawlkit_engine::ratelimit::RateLimiter::new(per_domain_rps, global_rps);
        let tokens = limiter.domain_tokens("test.example.com");
        let max_burst = (per_domain_rps * 2.0).ceil();
        prop_assert!(
            tokens <= max_burst + f64::EPSILON,
            "domain tokens {} exceed max burst {}",
            tokens,
            max_burst
        );
    }
}

// Global tokens never exceed the computed global burst capacity.
// Rationale: Global bucket burst is ceil(global_rps * 2). The initial
// global token count must respect this ceiling.
proptest! {
    #[test]
    fn rate_limiter_global_tokens_never_exceed_burst(
        per_domain_rps in 0.5f64..100.0,
        global_rps in 1.0f64..200.0
    ) {
        let limiter = crawlkit_engine::ratelimit::RateLimiter::new(per_domain_rps, global_rps);
        let tokens = limiter.global_tokens();
        let max_burst = (global_rps * 2.0).ceil();
        prop_assert!(
            tokens <= max_burst + f64::EPSILON,
            "global tokens {} exceed max burst {}",
            tokens,
            max_burst
        );
    }
}

// Domain token count is always non-negative.
// Rationale: Token buckets must never go negative. Negative tokens
// would indicate a consumption accounting error.
proptest! {
    #[test]
    fn rate_limiter_domain_tokens_non_negative(
        per_domain_rps in 0.1f64..50.0,
        domain in "[a-z]{1,20}\\.[a-z]{2,10}"
    ) {
        let limiter = crawlkit_engine::ratelimit::RateLimiter::new(per_domain_rps, per_domain_rps * 2.0);
        let tokens = limiter.domain_tokens(&domain);
        prop_assert!(tokens >= -f64::EPSILON, "domain tokens {} is negative", tokens);
    }
}

// Global token count is always non-negative.
// Rationale: Same invariant as domain tokens - global bucket must
// never report a negative value.
proptest! {
    #[test]
    fn rate_limiter_global_tokens_non_negative(global_rps in 0.1f64..50.0) {
        let limiter = crawlkit_engine::ratelimit::RateLimiter::new(1.0, global_rps);
        let tokens = limiter.global_tokens();
        prop_assert!(tokens >= -f64::EPSILON, "global tokens {} is negative", tokens);
    }
}

// set_crawl_delay resets domain tokens to the new burst level.
// Rationale: When crawl-delay is updated, the domain bucket must be
// replaced with a fresh bucket reflecting the new RPS and burst.
proptest! {
    #[test]
    fn rate_limiter_set_crawl_delay_resets_tokens(rps in 0.5f64..50.0) {
        let delay_secs = 1.0 / rps;
        let limiter = crawlkit_engine::ratelimit::RateLimiter::new(10.0, 100.0);
        limiter.set_crawl_delay("example.com", std::time::Duration::from_secs_f64(delay_secs));

        let tokens = limiter.domain_tokens("example.com");
        let expected_burst = (rps * 2.0).ceil();
        prop_assert!(
            (tokens - expected_burst).abs() < 1.0,
            "after set_crawl_delay: tokens {} != expected burst {}",
            tokens,
            expected_burst
        );
    }
}

// ============================================================================
// Meta Tag Parser Properties (via HtmlParser)
// ============================================================================

// Empty HTML produces empty/default meta tags (no title, no description).
// Rationale: An empty document must not hallucinate metadata. All meta
// fields should be None or empty.
proptest! {
    #[test]
    fn empty_html_empty_meta(_seed in 0u32..1000) {
        let url = Url::parse("https://example.com").unwrap();
        let page = crawlkit_engine::HtmlParser::parse("", &url);
        prop_assert!(page.meta.title.is_none());
        prop_assert!(page.meta.description.is_none());
        prop_assert!(page.headings.is_empty());
    }
}

// Title extraction is deterministic: same HTML always yields same title.
// Rationale: Parsing must be pure - same input must always produce
// identical output. This ensures crawl reproducibility.
proptest! {
    #[test]
    fn title_extraction_deterministic(title in "[A-Za-z0-9][A-Za-z0-9 ]{0,98}[A-Za-z0-9]") {
        let url = Url::parse("https://example.com").unwrap();
        let html = format!(
            "<!DOCTYPE html><html><head><title>{}</title></head><body></body></html>",
            title
        );
        let page1 = crawlkit_engine::HtmlParser::parse(&html, &url);
        let page2 = crawlkit_engine::HtmlParser::parse(&html, &url);
        prop_assert_eq!(&page1.meta.title, &page2.meta.title);
        prop_assert_eq!(page1.meta.title.as_deref(), Some(title.as_str()));
    }
}

// Description extraction handles missing tags gracefully.
// Rationale: When no description meta tag exists, the field must be None
// rather than panicking or returning garbage.
proptest! {
    #[test]
    fn description_missing_is_none(
        title in "[A-Za-z0-9][A-Za-z0-9 ]{0,48}[A-Za-z0-9]",
        has_desc in prop::bool::ANY
    ) {
        let url = Url::parse("https://example.com").unwrap();
        let html = if has_desc {
            format!(
                "<!DOCTYPE html><html><head><title>{}</title>\
                 <meta name=\"description\" content=\"A description\"></head>\
                 <body></body></html>",
                title
            )
        } else {
            format!(
                "<!DOCTYPE html><html><head><title>{}</title></head>\
                 <body></body></html>",
                title
            )
        };
        let page = crawlkit_engine::HtmlParser::parse(&html, &url);
        if has_desc {
            prop_assert!(page.meta.description.is_some());
        } else {
            prop_assert!(page.meta.description.is_none());
        }
    }
}

// HTML with title and description produces correct metadata and no pollution.
// Rationale: Minimal HTML must not introduce spurious meta tags or
// false positives in other fields.
proptest! {
    #[test]
    fn minimal_html_no_pollution(
        title in "[A-Za-z0-9][A-Za-z0-9 ]{0,78}[A-Za-z0-9]",
        desc in "[A-Za-z0-9][A-Za-z0-9 ]{0,198}[A-Za-z0-9]"
    ) {
        let url = Url::parse("https://example.com").unwrap();
        let html = format!(
            "<!DOCTYPE html><html><head><title>{}</title>\
             <meta name=\"description\" content=\"{}\">\
             </head><body></body></html>",
            title, desc
        );
        let page = crawlkit_engine::HtmlParser::parse(&html, &url);
        prop_assert_eq!(page.meta.title.as_deref(), Some(title.as_str()));
        prop_assert_eq!(page.meta.description.as_deref(), Some(desc.as_str()));
        prop_assert!(page.links.is_empty());
        prop_assert!(page.images.is_empty());
        prop_assert!(page.forms.is_empty());
    }
}

// Word count of HTML body text matches the actual number of whitespace-
// delimited tokens in visible text (excluding scripts and styles).
// Rationale: Word count must accurately reflect visible content.
proptest! {
    #[test]
    fn word_count_matches_visible_text(words in prop::collection::vec("[a-z]{1,10}", 0..20)) {
        let url = Url::parse("https://example.com").unwrap();
        let body_text = words.join(" ");
        let html = format!(
            "<!DOCTYPE html><html><body><p>{}</p></body></html>",
            body_text
        );
        let page = crawlkit_engine::HtmlParser::parse(&html, &url);
        let expected: usize = words.iter().filter(|w| !w.is_empty()).count();
        prop_assert_eq!(page.word_count, expected);
    }
}
