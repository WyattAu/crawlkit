use crate::analyzers::*;
use crate::meta::MetaTags;
use crate::parser::{ExtractedImage, ExtractedLink, Heading, StructuredData};
use crate::types::{IssueCategory, Severity};
use std::collections::{HashMap, HashSet};
use url::Url;

fn default_config() -> CrawlConfig {
    CrawlConfig::default()
}

fn make_page(url: &str) -> ParsedPage {
    ParsedPage {
        url: url.to_string(),
        meta: MetaTags::default(),
        headings: Vec::new(),
        links: Vec::new(),
        images: Vec::new(),
        forms: Vec::new(),
        scripts: Vec::new(),
        styles: Vec::new(),
        structured_data: Vec::new(),
        word_count: 0,
        sentence_count: 0,
        landmarks: Vec::new(),
        has_skip_link: false,
        has_main_landmark: false,
        has_nav_landmark: false,
        has_positive_tabindex: false,
        tabindex_negative_count: 0,
        aria_role_count: 0,
        aria_label_count: 0,
        has_lang_attribute: false,
        html_lang: None,
        has_aria_hidden: false,
        tables_with_headers: 0,
        tables_total: 0,
        tables_with_captions: 0,
        og_image_width: None,
        og_image_height: None,
    }
}

fn make_ctx<'a>(page: &'a ParsedPage, status: Option<u16>) -> AnalysisContext<'a> {
    AnalysisContext {
        page,
        body: None,
        status_code: status,
        headers: &[],
        response_time: None,
        redirect_chain: &[],
        robots_txt: None,
        body_size: None,
        compressed_size: None,
        server: None,
        content_type: None,
        rendered: None,
    }
}

// ---- HttpStatusAnalyzer ----

#[test]
fn test_http_status_200() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200));
    let findings = HttpStatusAnalyzer::new().analyze(&ctx);
    // Should have info about status category
    assert!(findings.iter().any(|f| f.code == "HTTP006"));
}

#[test]
fn test_http_status_404() {
    let page = make_page("https://example.com/missing");
    let ctx = make_ctx(&page, Some(404));
    let findings = HttpStatusAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "HTTP004"));
}

#[test]
fn test_http_status_500() {
    let page = make_page("https://example.com/error");
    let ctx = make_ctx(&page, Some(500));
    let findings = HttpStatusAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "HTTP005"));
    assert!(findings.iter().any(|f| f.severity == Severity::Critical));
}

#[test]
fn test_http_status_missing() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, None);
    let findings = HttpStatusAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "HTTP001"));
}

#[test]
fn test_http_status_soft_404_empty_body() {
    let mut page = make_page("https://example.com/soft404");
    page.word_count = 0;
    let ctx = make_ctx(&page, Some(200));
    let findings = HttpStatusAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "HTTP003"));
}

#[test]
fn test_http_status_slow_response() {
    let page = make_page("https://example.com/slow");
    let ctx = AnalysisContext {
        page: &page,
        body: None,
        status_code: Some(200),
        headers: &[],
        response_time: Some(Duration::from_secs(10)),
        redirect_chain: &[],
        robots_txt: None,
        body_size: None,
        compressed_size: None,
        server: None,
        content_type: None,
        rendered: None,
    };
    let findings = HttpStatusAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "HTTP002"));
}

// ---- RedirectChainAnalyzer ----

#[test]
fn test_redirect_no_hops() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200));
    let findings = RedirectChainAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_redirect_long_chain() {
    let hops: Vec<RedirectHop> = (0..7)
        .map(|i| RedirectHop {
            from: Url::parse(&format!("https://example.com/page{i}")).unwrap(),
            to: Url::parse(&format!("https://example.com/page{}", i + 1)).unwrap(),
            status_code: 301,
        })
        .collect();
    let page = make_page("https://example.com/page0");
    let ctx = AnalysisContext {
        page: &page,
        body: None,
        status_code: Some(200),
        headers: &[],
        response_time: None,
        redirect_chain: &hops,
        robots_txt: None,
        body_size: None,
        compressed_size: None,
        server: None,
        content_type: None,
        rendered: None,
    };
    let findings = RedirectChainAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "REDIR001"));
}

#[test]
fn test_redirect_loop() {
    let hops = vec![
        RedirectHop {
            from: Url::parse("https://example.com/a").unwrap(),
            to: Url::parse("https://example.com/b").unwrap(),
            status_code: 301,
        },
        RedirectHop {
            from: Url::parse("https://example.com/b").unwrap(),
            to: Url::parse("https://example.com/a").unwrap(),
            status_code: 301,
        },
    ];
    let page = make_page("https://example.com/a");
    let ctx = AnalysisContext {
        page: &page,
        body: None,
        status_code: Some(200),
        headers: &[],
        response_time: None,
        redirect_chain: &hops,
        robots_txt: None,
        body_size: None,
        compressed_size: None,
        server: None,
        content_type: None,
        rendered: None,
    };
    let findings = RedirectChainAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "REDIR002"));
}

#[test]
fn test_redirect_mixed_protocol() {
    let hops = vec![RedirectHop {
        from: Url::parse("http://example.com/page").unwrap(),
        to: Url::parse("https://example.com/page").unwrap(),
        status_code: 301,
    }];
    let page = make_page("http://example.com/page");
    let ctx = AnalysisContext {
        page: &page,
        body: None,
        status_code: Some(200),
        headers: &[],
        response_time: None,
        redirect_chain: &hops,
        robots_txt: None,
        body_size: None,
        compressed_size: None,
        server: None,
        content_type: None,
        rendered: None,
    };
    let findings = RedirectChainAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "REDIR003"));
}

#[test]
fn test_redirect_single_hop() {
    let hops = vec![RedirectHop {
        from: Url::parse("https://example.com/old").unwrap(),
        to: Url::parse("https://example.com/new").unwrap(),
        status_code: 301,
    }];
    let page = make_page("https://example.com/old");
    let ctx = AnalysisContext {
        page: &page,
        body: None,
        status_code: Some(200),
        headers: &[],
        response_time: None,
        redirect_chain: &hops,
        robots_txt: None,
        body_size: None,
        compressed_size: None,
        server: None,
        content_type: None,
        rendered: None,
    };
    let findings = RedirectChainAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "REDIR004"));
}

// ---- CanonicalUrlValidator ----

#[test]
fn test_canonical_missing() {
    let page = make_page("https://example.com/page");
    let ctx = make_ctx(&page, Some(200));
    let findings = CanonicalUrlValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "CANON001"));
}

#[test]
fn test_canonical_self_referencing() {
    let mut page = make_page("https://example.com/page");
    page.meta.canonical = Some(Url::parse("https://example.com/page").unwrap());
    let ctx = make_ctx(&page, Some(200));
    let findings = CanonicalUrlValidator::new().analyze(&ctx);
    // Self-referencing is fine — no mismatch finding
    assert!(!findings.iter().any(|f| f.code == "CANON003"));
}

#[test]
fn test_canonical_mismatch() {
    let mut page = make_page("https://example.com/page");
    page.meta.canonical = Some(Url::parse("https://example.com/other").unwrap());
    let ctx = make_ctx(&page, Some(200));
    let findings = CanonicalUrlValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "CANON003"));
}

// ---- HreflangValidator ----

#[test]
fn test_hreflang_no_tags() {
    let page = make_page("https://example.com/en");
    let ctx = make_ctx(&page, Some(200));
    let findings = HreflangValidator::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_hreflang_missing_x_default() {
    let mut page = make_page("https://example.com/en");
    page.meta.hreflang = vec![
        crate::meta::HreflangTag {
            lang: "en".to_string(),
            url: Url::parse("https://example.com/en").unwrap(),
        },
        crate::meta::HreflangTag {
            lang: "fr".to_string(),
            url: Url::parse("https://example.com/fr").unwrap(),
        },
    ];
    let ctx = make_ctx(&page, Some(200));
    let findings = HreflangValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "HREF001"));
}

#[test]
fn test_hreflang_invalid_locale() {
    let mut page = make_page("https://example.com/en");
    page.meta.hreflang = vec![
        crate::meta::HreflangTag {
            lang: "invalid-locale-code-too-long".to_string(),
            url: Url::parse("https://example.com/invalid").unwrap(),
        },
        crate::meta::HreflangTag {
            lang: "x-default".to_string(),
            url: Url::parse("https://example.com").unwrap(),
        },
    ];
    let ctx = make_ctx(&page, Some(200));
    let findings = HreflangValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "HREF002"));
}

#[test]
fn test_hreflang_duplicate_language() {
    let mut page = make_page("https://example.com/en");
    page.meta.hreflang = vec![
        crate::meta::HreflangTag {
            lang: "en".to_string(),
            url: Url::parse("https://example.com/en").unwrap(),
        },
        crate::meta::HreflangTag {
            lang: "en".to_string(),
            url: Url::parse("https://example.com/en-uk").unwrap(),
        },
        crate::meta::HreflangTag {
            lang: "x-default".to_string(),
            url: Url::parse("https://example.com").unwrap(),
        },
    ];
    let ctx = make_ctx(&page, Some(200));
    let findings = HreflangValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "HREF003"));
}

#[test]
fn test_hreflang_valid_with_x_default() {
    let mut page = make_page("https://example.com/en");
    page.meta.hreflang = vec![
        crate::meta::HreflangTag {
            lang: "en".to_string(),
            url: Url::parse("https://example.com/en").unwrap(),
        },
        crate::meta::HreflangTag {
            lang: "fr".to_string(),
            url: Url::parse("https://example.com/fr").unwrap(),
        },
        crate::meta::HreflangTag {
            lang: "x-default".to_string(),
            url: Url::parse("https://example.com").unwrap(),
        },
    ];
    let ctx = make_ctx(&page, Some(200));
    let findings = HreflangValidator::new().analyze(&ctx);
    // No errors for valid setup
    assert!(!findings.iter().any(|f| f.severity == Severity::Error));
}

// ---- SitemapAnalyzer ----

#[test]
fn test_sitemap_no_data() {
    let page = make_page("https://example.com/page");
    let ctx = make_ctx(&page, Some(200));
    let findings = SitemapAnalyzer::empty().analyze(&ctx);
    // Empty-data analyzers must not emit per-page noise findings.
    assert!(findings.is_empty());
}

#[test]
fn test_sitemap_url_not_found() {
    let mut known = HashSet::new();
    known.insert("https://example.com/other".to_string());
    let analyzer = SitemapAnalyzer::new(known, Vec::new());
    let page = make_page("https://example.com/page");
    let ctx = make_ctx(&page, Some(200));
    let findings = analyzer.analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "SITEMAP002"));
}

#[test]
fn test_sitemap_url_found() {
    let mut known = HashSet::new();
    known.insert("https://example.com/page".to_string());
    let analyzer = SitemapAnalyzer::new(known, Vec::new());
    let page = make_page("https://example.com/page");
    let ctx = make_ctx(&page, Some(200));
    let findings = analyzer.analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "SITEMAP002"));
}

#[test]
fn test_sitemap_invalid_lastmod() {
    let mut known = HashSet::new();
    known.insert("https://example.com/page".to_string());
    let entries = vec![SitemapEntry {
        url: "https://example.com/page".to_string(),
        lastmod: Some("not-a-date".to_string()),
        changefreq: None,
        priority: None,
    }];
    let analyzer = SitemapAnalyzer::new(known, entries);
    let page = make_page("https://example.com/page");
    let ctx = make_ctx(&page, Some(200));
    let findings = analyzer.analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "SITEMAP003"));
}

#[test]
fn test_sitemap_invalid_changefreq() {
    let mut known = HashSet::new();
    known.insert("https://example.com/page".to_string());
    let entries = vec![SitemapEntry {
        url: "https://example.com/page".to_string(),
        lastmod: None,
        changefreq: Some("sometimes".to_string()),
        priority: None,
    }];
    let analyzer = SitemapAnalyzer::new(known, entries);
    let page = make_page("https://example.com/page");
    let ctx = make_ctx(&page, Some(200));
    let findings = analyzer.analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "SITEMAP004"));
}

#[test]
fn test_sitemap_invalid_priority() {
    let mut known = HashSet::new();
    known.insert("https://example.com/page".to_string());
    let entries = vec![SitemapEntry {
        url: "https://example.com/page".to_string(),
        lastmod: None,
        changefreq: None,
        priority: Some(2.5),
    }];
    let analyzer = SitemapAnalyzer::new(known, entries);
    let page = make_page("https://example.com/page");
    let ctx = make_ctx(&page, Some(200));
    let findings = analyzer.analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "SITEMAP005"));
}

#[test]
fn test_sitemap_valid_metadata() {
    let mut known = HashSet::new();
    known.insert("https://example.com/page".to_string());
    let entries = vec![SitemapEntry {
        url: "https://example.com/page".to_string(),
        lastmod: Some("2024-01-15T10:30:00Z".to_string()),
        changefreq: Some("weekly".to_string()),
        priority: Some(0.8),
    }];
    let analyzer = SitemapAnalyzer::new(known, entries);
    let page = make_page("https://example.com/page");
    let ctx = make_ctx(&page, Some(200));
    let findings = analyzer.analyze(&ctx);
    // No errors for valid metadata
    assert!(!findings.iter().any(|f| f.severity == Severity::Error));
}

// ---- RobotsTxtAnalyzer ----

#[test]
fn test_robots_empty() {
    let page = make_page("https://example.com/page");
    let ctx = make_ctx(&page, Some(200));
    let findings = RobotsTxtAnalyzer::empty().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_robots_disallowed() {
    let rules = vec![RobotsRule {
        user_agent: "*".to_string(),
        disallowed_paths: vec!["/admin".to_string()],
        allowed_paths: Vec::new(),
        crawl_delay: None,
        sitemaps: Vec::new(),
    }];
    let analyzer = RobotsTxtAnalyzer::new(rules, Vec::new());
    let page = make_page("https://example.com/admin/secret");
    let ctx = make_ctx(&page, Some(200));
    let findings = analyzer.analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "ROBOT002"));
}

#[test]
fn test_robots_allowed() {
    let rules = vec![RobotsRule {
        user_agent: "*".to_string(),
        disallowed_paths: vec!["/admin".to_string()],
        allowed_paths: Vec::new(),
        crawl_delay: None,
        sitemaps: Vec::new(),
    }];
    let analyzer = RobotsTxtAnalyzer::new(rules, Vec::new());
    let page = make_page("https://example.com/page");
    let ctx = make_ctx(&page, Some(200));
    let findings = analyzer.analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "ROBOT002"));
}

#[test]
fn test_robots_allow_override() {
    let rules = vec![RobotsRule {
        user_agent: "*".to_string(),
        disallowed_paths: vec!["/admin".to_string()],
        allowed_paths: vec!["/admin/public".to_string()],
        crawl_delay: None,
        sitemaps: Vec::new(),
    }];
    let analyzer = RobotsTxtAnalyzer::new(rules, Vec::new());
    let page = make_page("https://example.com/admin/public/page");
    let ctx = make_ctx(&page, Some(200));
    let findings = analyzer.analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "ROBOT001"));
}

#[test]
fn test_robots_high_crawl_delay() {
    let rules = vec![RobotsRule {
        user_agent: "*".to_string(),
        disallowed_paths: Vec::new(),
        allowed_paths: Vec::new(),
        crawl_delay: Some(20.0),
        sitemaps: Vec::new(),
    }];
    let analyzer = RobotsTxtAnalyzer::new(rules, Vec::new());
    let page = make_page("https://example.com/page");
    let ctx = make_ctx(&page, Some(200));
    let findings = analyzer.analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "ROBOT003"));
}

#[test]
fn test_robots_invalid_sitemap_url() {
    let rules = vec![RobotsRule {
        user_agent: "*".to_string(),
        disallowed_paths: Vec::new(),
        allowed_paths: Vec::new(),
        crawl_delay: None,
        sitemaps: Vec::new(),
    }];
    let analyzer = RobotsTxtAnalyzer::new(rules, vec!["not-a-url".to_string()]);
    let page = make_page("https://example.com/page");
    let ctx = make_ctx(&page, Some(200));
    let findings = analyzer.analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "ROBOT004"));
}

// ---- MetaTagAnalyzer ----

#[test]
fn test_meta_missing_title() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200));
    let findings = MetaTagAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "META001"));
}

#[test]
fn test_meta_title_too_short() {
    let mut page = make_page("https://example.com");
    page.meta.title = Some("Hi".to_string());
    let ctx = make_ctx(&page, Some(200));
    let findings = MetaTagAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "META002"));
}

#[test]
fn test_meta_title_too_long() {
    let mut page = make_page("https://example.com");
    page.meta.title = Some("A".repeat(80));
    let ctx = make_ctx(&page, Some(200));
    let findings = MetaTagAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "META003"));
}

#[test]
fn test_meta_title_just_right() {
    let mut page = make_page("https://example.com");
    page.meta.title = Some("A".repeat(45));
    let ctx = make_ctx(&page, Some(200));
    let findings = MetaTagAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "META002"));
    assert!(!findings.iter().any(|f| f.code == "META003"));
}

#[test]
fn test_meta_missing_description() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200));
    let findings = MetaTagAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "META004"));
}

#[test]
fn test_meta_description_too_short() {
    let mut page = make_page("https://example.com");
    page.meta.description = Some("Short".to_string());
    let ctx = make_ctx(&page, Some(200));
    let findings = MetaTagAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "META005"));
}

#[test]
fn test_meta_description_too_long() {
    let mut page = make_page("https://example.com");
    page.meta.description = Some("A".repeat(200));
    let ctx = make_ctx(&page, Some(200));
    let findings = MetaTagAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "META006"));
}

#[test]
fn test_meta_missing_og_tags() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200));
    let findings = SocialMediaAnalyzer::new().analyze(&ctx);
    // Should flag og:title, og:description, og:image, og:url, og:type
    let og_codes: Vec<&str> = findings
        .iter()
        .filter(|f| f.code == "SOCIAL006")
        .map(|f| f.title.as_str())
        .collect();
    assert!(!og_codes.is_empty());
}

#[test]
fn test_meta_missing_twitter_tags() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200));
    let findings = SocialMediaAnalyzer::new().analyze(&ctx);
    // Should flag twitter:card, twitter:title, twitter:image
    let tw_count = findings.iter().filter(|f| f.code == "SOCIAL007").count();
    assert!(tw_count >= 1);
}

#[test]
fn test_meta_missing_viewport() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200));
    let findings = MetaTagAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "META009"));
}

#[test]
fn test_meta_complete_tags() {
    let mut page = make_page("https://example.com");
    page.meta.title = Some("Perfect Title for SEO".to_string());
    page.meta.description = Some("A".repeat(140));
    page.meta.viewport = Some("width=device-width".to_string());
    page.meta.og.title = Some("OG Title".to_string());
    page.meta.og.image = Some("https://example.com/img.png".to_string());
    page.meta.og.url = Some("https://example.com".to_string());
    page.meta.og.r#type = Some("website".to_string());
    page.meta.twitter.card = Some("summary_large_image".to_string());
    page.meta.twitter.title = Some("Twitter Title".to_string());
    page.meta.twitter.image = Some("https://example.com/tw.png".to_string());
    let ctx = make_ctx(&page, Some(200));
    let findings = MetaTagAnalyzer::new().analyze(&ctx);
    // Should have no errors or warnings about missing tags
    assert!(!findings
        .iter()
        .any(|f| f.code == "META001" || f.code == "META004"));
}

// ---- HeadingHierarchyAnalyzer ----

#[test]
fn test_heading_no_headings() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200));
    let findings = HeadingHierarchyAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "HEAD001"));
}

#[test]
fn test_heading_missing_h1() {
    let mut page = make_page("https://example.com");
    page.headings = vec![
        Heading {
            level: 2,
            text: "Section".to_string(),
            length: 7,
        },
        Heading {
            level: 3,
            text: "Sub".to_string(),
            length: 3,
        },
    ];
    let ctx = make_ctx(&page, Some(200));
    let findings = HeadingHierarchyAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "HEAD002"));
}

#[test]
fn test_heading_multiple_h1() {
    let mut page = make_page("https://example.com");
    page.headings = vec![
        Heading {
            level: 1,
            text: "First".to_string(),
            length: 5,
        },
        Heading {
            level: 1,
            text: "Second".to_string(),
            length: 6,
        },
    ];
    let ctx = make_ctx(&page, Some(200));
    let findings = HeadingHierarchyAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "HEAD003"));
}

#[test]
fn test_heading_skipped_level() {
    let mut page = make_page("https://example.com");
    page.headings = vec![
        Heading {
            level: 1,
            text: "Title".to_string(),
            length: 5,
        },
        Heading {
            level: 3,
            text: "Skipped H2".to_string(),
            length: 10,
        },
    ];
    let ctx = make_ctx(&page, Some(200));
    let findings = HeadingHierarchyAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "HEAD004"));
}

#[test]
fn test_heading_valid_hierarchy() {
    let mut page = make_page("https://example.com");
    page.headings = vec![
        Heading {
            level: 1,
            text: "Main".to_string(),
            length: 4,
        },
        Heading {
            level: 2,
            text: "Section".to_string(),
            length: 7,
        },
        Heading {
            level: 2,
            text: "Section 2".to_string(),
            length: 9,
        },
        Heading {
            level: 3,
            text: "Sub".to_string(),
            length: 3,
        },
    ];
    let ctx = make_ctx(&page, Some(200));
    let findings = HeadingHierarchyAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "HEAD004"));
}

#[test]
fn test_heading_deep_hierarchy() {
    let mut page = make_page("https://example.com");
    page.headings = vec![
        Heading {
            level: 1,
            text: "H1".to_string(),
            length: 2,
        },
        Heading {
            level: 2,
            text: "H2".to_string(),
            length: 2,
        },
        Heading {
            level: 3,
            text: "H3".to_string(),
            length: 2,
        },
        Heading {
            level: 4,
            text: "H4".to_string(),
            length: 2,
        },
        Heading {
            level: 5,
            text: "H5".to_string(),
            length: 2,
        },
    ];
    let ctx = make_ctx(&page, Some(200));
    let findings = HeadingHierarchyAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "HEAD005"));
}

// ---- AnalyzerRegistry ----

#[test]
fn test_registry_default() {
    let config = default_config();
    let registry = AnalyzerRegistry::new(&config);
    assert_eq!(registry.len(), 785);
    assert!(!registry.is_empty());
}

#[test]
fn test_registry_analyze() {
    let config = default_config();
    let registry = AnalyzerRegistry::new(&config);
    let mut page = make_page("https://example.com");
    page.meta.title = Some("Good Title Here for SEO".to_string());
    let ctx = make_ctx(&page, Some(200));
    let findings = registry.analyze(&ctx);
    // Should produce findings from multiple analyzers
    assert!(!findings.is_empty());
}

#[test]
fn test_registry_custom() {
    struct DummyAnalyzer;
    impl Analyzer for DummyAnalyzer {
        fn name(&self) -> &str {
            "dummy"
        }
        fn analyze(&self, _ctx: &AnalysisContext) -> Vec<Finding> {
            vec![Finding {
                severity: Severity::Info,
                category: IssueCategory::Custom("test".to_string()),
                code: "DUMMY001".to_string(),
                title: "Dummy finding".to_string(),
                description: "Test".to_string(),
                url: String::new(),
                recommendation: "None".to_string(),
            }]
        }
    }
    let mut registry = AnalyzerRegistry::with_analyzers(Vec::new());
    registry.register(Box::new(DummyAnalyzer));
    assert_eq!(registry.len(), 1);

    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200));
    let findings = registry.analyze(&ctx);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].code, "DUMMY001");
}

#[test]
fn test_registry_analyze_sorts_findings_canonically() {
    struct UnsortedAnalyzer;
    impl Analyzer for UnsortedAnalyzer {
        fn name(&self) -> &str {
            "unsorted"
        }
        fn analyze(&self, ctx: &AnalysisContext) -> Vec<Finding> {
            let finding = |code: &str, url: &str| Finding {
                severity: Severity::Info,
                category: IssueCategory::Custom("test".to_string()),
                code: code.to_string(),
                title: format!("finding {code}"),
                description: "Test".to_string(),
                url: url.to_string(),
                recommendation: "None".to_string(),
            };
            // Deliberately unsorted by (code, url).
            vec![
                finding("ZZZ001", ctx.page.url.as_str()),
                finding("AAA002", "https://example.com/b"),
                finding("AAA002", "https://example.com/a"),
                finding("AAA001", ctx.page.url.as_str()),
            ]
        }
    }
    let registry = AnalyzerRegistry::with_analyzers(vec![Box::new(UnsortedAnalyzer)]);
    let page = make_page("https://example.com/page");
    let ctx = make_ctx(&page, Some(200));
    let findings = registry.analyze(&ctx);

    let keys: Vec<(String, String)> = findings
        .iter()
        .map(|f| (f.code.clone(), f.url.clone()))
        .collect();
    let mut sorted_keys = keys.clone();
    sorted_keys.sort();
    assert_eq!(keys, sorted_keys, "findings must be ordered by (code, url)");
}

#[test]
fn test_registry_analyze_repeated_calls_identical_order() {
    let config = default_config();
    let registry = AnalyzerRegistry::new(&config);
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200));
    let first: Vec<(String, String)> = registry
        .analyze(&ctx)
        .iter()
        .map(|f| (f.code.clone(), f.title.clone()))
        .collect();
    let second: Vec<(String, String)> = registry
        .analyze(&ctx)
        .iter()
        .map(|f| (f.code.clone(), f.title.clone()))
        .collect();
    assert_eq!(first, second);
}

// ---- Edge cases for locale validation ----

#[test]
fn test_valid_locales() {
    assert!(HreflangValidator::is_valid_locale("en"));
    assert!(HreflangValidator::is_valid_locale("fr"));
    assert!(HreflangValidator::is_valid_locale("de"));
    assert!(HreflangValidator::is_valid_locale("en-US"));
    assert!(HreflangValidator::is_valid_locale("fr-CA"));
    assert!(HreflangValidator::is_valid_locale("zh-CN"));
    assert!(HreflangValidator::is_valid_locale("x-default"));
}

#[test]
fn test_invalid_locales() {
    assert!(!HreflangValidator::is_valid_locale("e"));
    assert!(!HreflangValidator::is_valid_locale("english"));
    assert!(!HreflangValidator::is_valid_locale("en-us-extra"));
    assert!(!HreflangValidator::is_valid_locale("123"));
}

// ---- Edge cases for soft 404 detection ----

#[test]
fn test_soft_404_indicators() {
    assert!(HttpStatusAnalyzer::is_soft_404(
        "<html><body>Page Not Found</body></html>"
    ));
    assert!(HttpStatusAnalyzer::is_soft_404(
        "Error 404 — The page you requested does not exist."
    ));
    assert!(HttpStatusAnalyzer::is_soft_404(
        "Sorry, we couldn't find the page you're looking for."
    ));
    assert!(!HttpStatusAnalyzer::is_soft_404(
        "<html><body>Welcome to our site</body></html>"
    ));
}

// ---- Edge cases for robots.txt path matching ----

#[test]
fn test_robots_path_matching() {
    assert!(RobotsTxtAnalyzer::is_disallowed(
        "/admin/secret",
        &["/admin".to_string()]
    ));
    assert!(!RobotsTxtAnalyzer::is_disallowed(
        "/page",
        &["/admin".to_string()]
    ));
    assert!(RobotsTxtAnalyzer::is_allowed(
        "/admin/public",
        &["/admin/public".to_string()]
    ));
    assert!(!RobotsTxtAnalyzer::is_allowed(
        "/admin/secret",
        &["/admin/public".to_string()]
    ));
}

// ---- Finding struct ----

#[test]
fn test_finding_creation() {
    let finding = Finding {
        severity: Severity::Warning,
        category: IssueCategory::Seo,
        code: "TEST001".to_string(),
        title: "Test finding".to_string(),
        description: "A test finding for unit tests".to_string(),
        url: "https://example.com".to_string(),
        recommendation: "Fix it".to_string(),
    };
    assert_eq!(finding.severity, Severity::Warning);
    assert_eq!(finding.category, IssueCategory::Seo);
}

// ---- Sitemap edge cases ----

#[test]
fn test_sitemap_valid_lastmod_formats() {
    assert!(SitemapAnalyzer::is_valid_lastmod("2024-01-15T10:30:00Z"));
    assert!(SitemapAnalyzer::is_valid_lastmod("2024-01-15"));
    assert!(!SitemapAnalyzer::is_valid_lastmod("lastweek"));
}

#[test]
fn test_sitemap_valid_changefreq() {
    assert!(SitemapAnalyzer::is_valid_changefreq("daily"));
    assert!(SitemapAnalyzer::is_valid_changefreq("weekly"));
    assert!(SitemapAnalyzer::is_valid_changefreq("never"));
    assert!(!SitemapAnalyzer::is_valid_changefreq("sometimes"));
    assert!(!SitemapAnalyzer::is_valid_changefreq("often"));
}

#[test]
fn test_sitemap_valid_priority() {
    assert!(SitemapAnalyzer::is_valid_priority(0.0));
    assert!(SitemapAnalyzer::is_valid_priority(0.5));
    assert!(SitemapAnalyzer::is_valid_priority(1.0));
    assert!(!SitemapAnalyzer::is_valid_priority(-0.1));
    assert!(!SitemapAnalyzer::is_valid_priority(1.1));
}

// ---- Full analysis integration ----

#[test]
fn test_full_analysis_minimal_page() {
    let config = default_config();
    let registry = AnalyzerRegistry::new(&config);
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200));
    let findings = registry.analyze(&ctx);

    // A minimal page should produce several findings
    let codes: Vec<&str> = findings.iter().map(|f| f.code.as_str()).collect();
    assert!(codes.contains(&"META001")); // missing title
    assert!(codes.contains(&"META004")); // missing description
    assert!(codes.contains(&"CANON001")); // missing canonical
    assert!(codes.contains(&"HEAD001")); // no headings
}

#[test]
fn test_full_analysis_well_optimized_page() {
    let config = default_config();
    let registry = AnalyzerRegistry::new(&config);

    let mut page = make_page("https://example.com/page");
    page.meta.title = Some("Optimized Page Title for Search".to_string());
    page.meta.description = Some("A".repeat(145));
    page.meta.canonical = Some(Url::parse("https://example.com/page").unwrap());
    page.meta.viewport = Some("width=device-width".to_string());
    page.meta.og.title = Some("OG Title".to_string());
    page.meta.og.image = Some("https://example.com/img.png".to_string());
    page.meta.og.url = Some("https://example.com/page".to_string());
    page.meta.og.r#type = Some("article".to_string());
    page.meta.twitter.card = Some("summary_large_image".to_string());
    page.meta.twitter.title = Some("Twitter Title".to_string());
    page.meta.twitter.image = Some("https://example.com/tw.png".to_string());
    page.headings = vec![
        Heading {
            level: 1,
            text: "Main Topic".to_string(),
            length: 10,
        },
        Heading {
            level: 2,
            text: "Section".to_string(),
            length: 7,
        },
    ];
    page.has_lang_attribute = true;
    page.html_lang = Some("en".to_string());
    page.has_main_landmark = true;
    page.has_nav_landmark = true;
    page.has_skip_link = true;
    page.word_count = 500;

    let ctx = make_ctx(&page, Some(200));
    let findings = registry.analyze(&ctx);

    // Should have few/no errors
    let errors: Vec<&Finding> = findings
        .iter()
        .filter(|f| f.severity == Severity::Error || f.severity == Severity::Critical)
        .collect();
    assert!(
        errors.is_empty(),
        "Well-optimized page should have no errors: {:?}",
        errors.iter().map(|f| &f.code).collect::<Vec<_>>()
    );
}

// =========================================================================
// LinkAnalyzer tests
// =========================================================================

#[test]
fn test_link_analyzer_no_links() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200));
    let findings = LinkAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "LINK001"));
    let link001 = findings.iter().find(|f| f.code == "LINK001").unwrap();
    assert!(link001.description.contains("Internal: 0"));
    assert!(link001.description.contains("External: 0"));
}

#[test]
fn test_link_analyzer_internal_external_counts() {
    let mut page = make_page("https://example.com");
    page.links = vec![
        ExtractedLink {
            href: "/about".to_string(),
            text: "About".to_string(),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: None,
        },
        ExtractedLink {
            href: "/contact".to_string(),
            text: "Contact".to_string(),
            rel: vec![],
            is_external: false,
            aria_label: None,
            img_alt: None,
        },
        ExtractedLink {
            href: "https://external.com".to_string(),
            text: "External".to_string(),
            rel: vec![],
            is_external: true,
            aria_label: None,
            img_alt: None,
        },
    ];
    let ctx = make_ctx(&page, Some(200));
    let findings = LinkAnalyzer::new().analyze(&ctx);
    let link001 = findings.iter().find(|f| f.code == "LINK001").unwrap();
    assert!(link001.description.contains("Internal: 2"));
    assert!(link001.description.contains("External: 1"));
}

#[test]
fn test_link_analyzer_nofollow_detection() {
    let mut page = make_page("https://example.com");
    page.links = vec![ExtractedLink {
        href: "https://external.com".to_string(),
        text: "Nofollow link".to_string(),
        rel: vec!["nofollow".to_string()],
        is_external: true,
        aria_label: None,
        img_alt: None,
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = LinkAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "LINK003"));
}

#[test]
fn test_link_analyzer_empty_anchor_text() {
    let mut page = make_page("https://example.com");
    page.links = vec![ExtractedLink {
        href: "/page".to_string(),
        text: String::new(),
        rel: vec![],
        is_external: false,
        aria_label: None,
        img_alt: None,
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = LinkAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "LINK004"));
}

#[test]
fn test_link_analyzer_short_anchor_text() {
    let mut page = make_page("https://example.com");
    page.links = vec![ExtractedLink {
        href: "/page".to_string(),
        text: "Go".to_string(),
        rel: vec![],
        is_external: false,
        aria_label: None,
        img_alt: None,
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = LinkAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "LINK005"));
}

#[test]
fn test_link_analyzer_broken_page_links() {
    let mut page = make_page("https://example.com");
    page.links = vec![ExtractedLink {
        href: "/broken".to_string(),
        text: "Broken link".to_string(),
        rel: vec![],
        is_external: false,
        aria_label: None,
        img_alt: None,
    }];
    let ctx = make_ctx(&page, Some(404));
    let findings = LinkAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "LINK002"));
}

#[test]
fn test_link_analyzer_orphan_page() {
    let mut inbound = HashMap::new();
    inbound.insert("https://example.com/orphan".to_string(), 0);
    let analyzer = LinkAnalyzer::with_inbound_links(inbound);
    let page = make_page("https://example.com/orphan");
    let ctx = make_ctx(&page, Some(200));
    let findings = analyzer.analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "LINK006"));
}

#[test]
fn test_link_analyzer_not_orphan() {
    let mut inbound = HashMap::new();
    inbound.insert("https://example.com/page".to_string(), 3);
    let analyzer = LinkAnalyzer::with_inbound_links(inbound);
    let page = make_page("https://example.com/page");
    let ctx = make_ctx(&page, Some(200));
    let findings = analyzer.analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "LINK006"));
}

#[test]
fn test_link_analyzer_no_nofollow_when_absent() {
    let mut page = make_page("https://example.com");
    page.links = vec![ExtractedLink {
        href: "/page".to_string(),
        text: "Go".to_string(),
        rel: vec![],
        is_external: false,
        aria_label: None,
        img_alt: None,
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = LinkAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "LINK003"));
}

// =========================================================================
// ImageAnalyzer tests
// =========================================================================

#[test]
fn test_image_analyzer_no_images() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200));
    let findings = ImageAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_image_analyzer_missing_alt_text() {
    let mut page = make_page("https://example.com");
    page.images = vec![ExtractedImage {
        src: "/img.png".to_string(),
        alt: String::new(),
        width: None,
        height: None,
        has_alt: false,
        is_lazy_loaded: false,
        aria_hidden: false,
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = ImageAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "IMG001"));
}

#[test]
fn test_image_analyzer_with_alt_text() {
    let mut page = make_page("https://example.com");
    page.images = vec![ExtractedImage {
        src: "/img.png".to_string(),
        alt: "A photo".to_string(),
        width: Some(100),
        height: Some(200),
        has_alt: true,
        is_lazy_loaded: false,
        aria_hidden: false,
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = ImageAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "IMG001"));
}

#[test]
fn test_image_analyzer_lazy_loading() {
    let mut page = make_page("https://example.com");
    page.images = vec![
        ExtractedImage {
            src: "/a.png".to_string(),
            alt: "A".to_string(),
            width: None,
            height: None,
            has_alt: true,
            is_lazy_loaded: true,
            aria_hidden: false,
        },
        ExtractedImage {
            src: "/b.png".to_string(),
            alt: "B".to_string(),
            width: None,
            height: None,
            has_alt: true,
            is_lazy_loaded: true,
            aria_hidden: false,
        },
        ExtractedImage {
            src: "/c.png".to_string(),
            alt: "C".to_string(),
            width: None,
            height: None,
            has_alt: true,
            is_lazy_loaded: false,
            aria_hidden: false,
        },
    ];
    let ctx = make_ctx(&page, Some(200));
    let findings = ImageAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "IMG003"));
    let img003 = findings.iter().find(|f| f.code == "IMG003").unwrap();
    assert!(img003.description.contains("2 of 3"));
}

#[test]
fn test_image_analyzer_missing_dimensions() {
    let mut page = make_page("https://example.com");
    page.images = vec![ExtractedImage {
        src: "/no-dims.png".to_string(),
        alt: "No dims".to_string(),
        width: None,
        height: None,
        has_alt: true,
        is_lazy_loaded: false,
        aria_hidden: false,
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = ImageAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "IMG004"));
}

#[test]
fn test_image_analyzer_detect_format() {
    assert_eq!(
        ImageAnalyzer::detect_format("photo.jpg"),
        Some("jpeg".into())
    );
    assert_eq!(
        ImageAnalyzer::detect_format("photo.jpeg"),
        Some("jpeg".into())
    );
    assert_eq!(ImageAnalyzer::detect_format("pic.png"), Some("png".into()));
    assert_eq!(ImageAnalyzer::detect_format("anim.gif"), Some("gif".into()));
    assert_eq!(
        ImageAnalyzer::detect_format("modern.webp"),
        Some("webp".into())
    );
    assert_eq!(
        ImageAnalyzer::detect_format("new.avif"),
        Some("avif".into())
    );
    assert_eq!(ImageAnalyzer::detect_format("icon.svg"), Some("svg".into()));
    assert_eq!(
        ImageAnalyzer::detect_format("photo.jpg?v=1"),
        Some("jpeg".into())
    );
    assert_eq!(ImageAnalyzer::detect_format("no-ext"), None);
}

// =========================================================================
// StructuredDataValidator tests
// =========================================================================

#[test]
fn test_structured_data_no_data() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200));
    let findings = StructuredDataValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "SD001"));
}

#[test]
fn test_structured_data_missing_context() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: None,
        r#type: Some("Article".to_string()),
        data: serde_json::json!({"@type": "Article", "headline": "Test"}),
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = StructuredDataValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "SD002"));
}

#[test]
fn test_structured_data_wrong_context() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://example.com/schema".to_string()),
        r#type: Some("Article".to_string()),
        data: serde_json::json!({"@type": "Article"}),
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = StructuredDataValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "SD003"));
}

#[test]
fn test_structured_data_missing_type() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: None,
        data: serde_json::json!({"@context": "https://schema.org"}),
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = StructuredDataValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "SD004"));
}

#[test]
fn test_structured_data_unknown_type() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("CustomWidget".to_string()),
        data: serde_json::json!({"@context": "https://schema.org", "@type": "CustomWidget"}),
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = StructuredDataValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "SD005"));
}

#[test]
fn test_structured_data_missing_required_properties() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Article".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Article"
        }),
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = StructuredDataValidator::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "SD006"));
    let sd006 = findings.iter().find(|f| f.code == "SD006").unwrap();
    assert!(sd006.description.contains("headline"));
    assert!(sd006.description.contains("author"));
}

#[test]
fn test_structured_data_valid_article() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Article".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Article",
            "headline": "Test Article",
            "author": "John Doe"
        }),
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = StructuredDataValidator::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "SD006"));
}

#[test]
fn test_structured_data_valid_product() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Product".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Product",
            "name": "Widget"
        }),
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = StructuredDataValidator::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "SD006"));
}

#[test]
fn test_structured_data_schema_org_context() {
    let mut page = make_page("https://example.com");
    page.structured_data = vec![StructuredData {
        context: Some("schema.org".to_string()),
        r#type: Some("WebSite".to_string()),
        data: serde_json::json!({
            "@context": "schema.org",
            "@type": "WebSite",
            "name": "My Site"
        }),
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = StructuredDataValidator::new().analyze(&ctx);
    // "schema.org" without https is accepted as valid
    assert!(!findings.iter().any(|f| f.code == "SD003"));
}

// =========================================================================
// ContentQualityAnalyzer tests
// =========================================================================

#[test]
fn test_content_quality_empty_page() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200));
    let findings = ContentQualityAnalyzer::new().analyze(&ctx);
    // Should flag zero content
    assert!(findings.iter().any(|f| f.code == "CQ003"));
}

#[test]
fn test_content_quality_thin_content() {
    let mut page = make_page("https://example.com");
    page.word_count = 150;
    let ctx = make_ctx(&page, Some(200));
    let findings = ContentQualityAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "CQ004"));
}

#[test]
fn test_content_quality_long_form() {
    let mut page = make_page("https://example.com");
    page.word_count = 5000;
    let ctx = make_ctx(&page, Some(200));
    let findings = ContentQualityAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "CQ005"));
}

#[test]
fn test_content_quality_readability_score() {
    let mut page = make_page("https://example.com");
    page.word_count = 500;
    page.headings = vec![Heading {
        level: 1,
        text: "This is a simple heading for testing readability scores in content analysis"
            .to_string(),
        length: 66,
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = ContentQualityAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "CQ001"));
}

#[test]
fn test_content_quality_keyword_density() {
    let mut page = make_page("https://example.com");
    page.word_count = 500;
    page.headings = vec![
        Heading {
            level: 1,
            text: "Rust Programming Language Tutorial".to_string(),
            length: 33,
        },
        Heading {
            level: 2,
            text: "Rust Basics for Beginners".to_string(),
            length: 24,
        },
        Heading {
            level: 2,
            text: "Advanced Rust Programming".to_string(),
            length: 24,
        },
    ];
    let ctx = make_ctx(&page, Some(200));
    let findings = ContentQualityAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "CQ002"));
    let cq002 = findings.iter().find(|f| f.code == "CQ002").unwrap();
    assert!(cq002.description.contains("rust"));
}

#[test]
fn test_content_quality_syllable_counting() {
    assert_eq!(count_syllables("cat"), 1);
    assert_eq!(count_syllables("hello"), 2);
    assert_eq!(count_syllables("beautiful"), 3);
    assert_eq!(count_syllables("a"), 1);
    assert_eq!(count_syllables(""), 0);
}

/// Direct fixtures for flesch_kincaid_grade — hand-computed from the
/// formula 0.39*(w/s) + 11.8*(syl/w) - 15.59. Added after the
/// cargo-mutants baseline showed arithmetic-operator mutants surviving
/// in these functions (53.6% kill score).
#[test]
fn test_flesch_kincaid_grade_fixtures() {
    let close = |a: f64, b: f64| assert!((a - b).abs() < 0.01, "{a} vs {b}");

    // Degenerate inputs short-circuit to 0.0
    assert_eq!(flesch_kincaid_grade(0, 0, 0), 0.0);
    assert_eq!(flesch_kincaid_grade(100, 0, 50), 0.0);
    assert_eq!(flesch_kincaid_grade(0, 5, 50), 0.0);

    // 100 words, 5 sentences, 150 syllables:
    // 0.39*20 + 11.8*1.5 - 15.59 = 7.8 + 17.7 - 15.59 = 9.91
    close(flesch_kincaid_grade(100, 5, 150), 9.91);

    // 30 words, 2 sentences, 45 syllables:
    // 0.39*15 + 11.8*1.5 - 15.59 = 5.85 + 17.7 - 15.59 = 7.96
    close(flesch_kincaid_grade(30, 2, 45), 7.96);

    // Monosyllabic short sentences: 10 words, 1 sentence, 10 syllables:
    // 3.9 + 11.8 - 15.59 = 0.11
    close(flesch_kincaid_grade(10, 1, 10), 0.11);
}

/// Direct fixtures for flesch_reading_ease — hand-computed from
/// 206.835 - 1.015*(w/s) - 84.6*(syl/w), clamped to [0, 100].
#[test]
fn test_flesch_reading_ease_fixtures() {
    let close = |a: f64, b: f64| assert!((a - b).abs() < 0.01, "{a} vs {b}");

    assert_eq!(flesch_reading_ease(0, 0, 0), 0.0);

    // 100 words, 5 sentences, 150 syllables:
    // 206.835 - 20.3 - 126.9 = 59.635 (standard difficulty)
    close(flesch_reading_ease(100, 5, 150), 59.635);

    // Easy monosyllabic prose: 60 words, 2 sentences, 60 syllables:
    // 206.835 - 30.45 - 84.6 = 91.785
    close(flesch_reading_ease(60, 2, 60), 91.785);

    // Clamp at floor: 2 words, 1 sentence, 12 syllables:
    // 206.835 - 2.03 - 507.6 = -302.795 → 0.0
    assert_eq!(flesch_reading_ease(2, 1, 12), 0.0);

    // Clamp at ceiling: 50 words, 50 sentences, 50 syllables:
    // 206.835 - 1.015 - 84.6 = 121.22 → 100.0
    assert_eq!(flesch_reading_ease(50, 50, 50), 100.0);
}

#[test]
fn test_content_quality_sentence_counting() {
    assert_eq!(count_sentences("Hello world."), 1);
    assert_eq!(
        count_sentences("First sentence. Second sentence! Third?"),
        3
    );
    assert_eq!(count_sentences(""), 1);
    assert_eq!(count_sentences("   "), 1);
}

#[test]
fn test_content_quality_flesch_kincaid() {
    // Simple text should score higher
    let score = flesch_reading_ease(100, 5, 150);
    assert!(score > 50.0);

    // Complex text should score lower
    let score = flesch_reading_ease(100, 20, 300);
    assert!(score < 50.0);

    // Zero words should return 0
    assert_eq!(flesch_reading_ease(0, 0, 0), 0.0);
}

// =========================================================================
// WordCountAnalyzer tests
// =========================================================================

#[test]
fn test_word_count_analyzer_empty() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200));
    let findings = WordCountAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "WC001"));
    assert!(findings.iter().any(|f| f.code == "WC002"));
}

#[test]
fn test_word_count_analyzer_with_content() {
    let mut page = make_page("https://example.com");
    page.word_count = 150;
    page.headings = vec![Heading {
        level: 1,
        text: "A page with some words".to_string(),
        length: 22,
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = WordCountAnalyzer::new().analyze(&ctx);
    let wc001 = findings.iter().find(|f| f.code == "WC001").unwrap();
    assert!(wc001.description.contains("Words: 150"));
}

#[test]
fn test_word_count_analyzer_very_low() {
    let mut page = make_page("https://example.com");
    page.word_count = 50;
    page.headings = vec![Heading {
        level: 1,
        text: "Short".to_string(),
        length: 5,
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = WordCountAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "WC003"));
}

#[test]
fn test_word_count_analyzer_long_sentences() {
    let mut page = make_page("https://example.com");
    // 100 words in 2 sentences = 50 avg → fires (>25)
    page.word_count = 100;
    page.sentence_count = 2;
    let ctx = make_ctx(&page, Some(200));
    let findings = WordCountAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "WC004"));
}

/// Regression (kingstonpeptides.com dogfood): the old implementation
/// divided full-page words by heading-only sentence counts, reporting
/// absurd averages (147-190 "words/sentence") on every page. With a
/// consistent corpus, a chrome-heavy 150-word page across ~12 short
/// sentences averages ~12.5 and must NOT fire.
#[test]
fn test_word_count_analyzer_consistent_corpus_no_false_positive() {
    let mut page = make_page("https://example.com");
    page.word_count = 150;
    page.sentence_count = 12;
    let ctx = make_ctx(&page, Some(200));
    let findings = WordCountAnalyzer::new().analyze(&ctx);
    assert!(
        findings.iter().all(|f| f.code != "WC004"),
        "150 words / 12 sentences (avg 12.5) must not be a long-sentence page: {:?}",
        findings.iter().map(|f| f.code.as_str()).collect::<Vec<_>>()
    );
}

#[test]
fn test_word_count_analyzer_zero_sentences_treated_as_one() {
    let mut page = make_page("https://example.com");
    // Label-style page: words but no sentence terminators → one
    // implicit sentence, so the average equals the word count.
    page.word_count = 30;
    page.sentence_count = 0;
    let ctx = make_ctx(&page, Some(200));
    let findings = WordCountAnalyzer::new().analyze(&ctx);
    let wc001 = findings
        .iter()
        .find(|f| f.code == "WC001")
        .expect("stats finding");
    assert!(wc001.description.contains("Avg words/sentence: 30.0"));
}

#[test]
fn test_word_count_analyzer_normal_content() {
    let mut page = make_page("https://example.com");
    page.word_count = 500;
    page.headings = vec![
        Heading {
            level: 1,
            text: "Main Title".to_string(),
            length: 10,
        },
        Heading {
            level: 2,
            text: "Section One".to_string(),
            length: 11,
        },
    ];
    let ctx = make_ctx(&page, Some(200));
    let findings = WordCountAnalyzer::new().analyze(&ctx);
    // Should have WC001 but not WC002 or WC003
    assert!(findings.iter().any(|f| f.code == "WC001"));
    assert!(!findings.iter().any(|f| f.code == "WC002"));
    assert!(!findings.iter().any(|f| f.code == "WC003"));
}

// =========================================================================
// SecurityHeaderAnalyzer tests
// =========================================================================

#[test]
fn test_security_headers_none_present() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200));
    let findings = SecurityHeaderAnalyzer::new().analyze(&ctx);
    // Should flag missing CSP, HSTS, XFO, XCTO
    assert!(findings.iter().any(|f| f.code == "SEC001"));
    assert!(findings.iter().any(|f| f.code == "SEC002"));
    assert!(findings.iter().any(|f| f.code == "SEC003"));
    assert!(findings.iter().any(|f| f.code == "SEC005"));
    // Should have a score
    assert!(findings.iter().any(|f| f.code == "SEC012"));
}

#[test]
fn test_security_headers_all_present() {
    let headers = vec![
        (
            "Content-Security-Policy".to_string(),
            "default-src 'self'".to_string(),
        ),
        (
            "Strict-Transport-Security".to_string(),
            "max-age=31536000; includeSubDomains; preload".to_string(),
        ),
        ("X-Frame-Options".to_string(), "DENY".to_string()),
        ("X-Content-Type-Options".to_string(), "nosniff".to_string()),
        (
            "Referrer-Policy".to_string(),
            "strict-origin-when-cross-origin".to_string(),
        ),
        (
            "Permissions-Policy".to_string(),
            "camera=(), microphone=(), geolocation=()".to_string(),
        ),
        (
            "Cross-Origin-Embedder-Policy".to_string(),
            "require-corp".to_string(),
        ),
        (
            "Cross-Origin-Opener-Policy".to_string(),
            "same-origin".to_string(),
        ),
        (
            "Cross-Origin-Resource-Policy".to_string(),
            "same-origin".to_string(),
        ),
    ];
    let page = make_page("https://example.com");
    let ctx = AnalysisContext {
        page: &page,
        body: None,
        status_code: Some(200),
        headers: &headers,
        response_time: None,
        redirect_chain: &[],
        robots_txt: None,
        body_size: None,
        compressed_size: None,
        server: None,
        content_type: None,
        rendered: None,
    };
    let findings = SecurityHeaderAnalyzer::new().analyze(&ctx);
    // Should not flag any missing headers
    assert!(!findings.iter().any(|f| f.code == "SEC001"));
    assert!(!findings.iter().any(|f| f.code == "SEC002"));
    assert!(!findings.iter().any(|f| f.code == "SEC003"));
    assert!(!findings.iter().any(|f| f.code == "SEC005"));
    // Score should be high
    let score_finding = findings.iter().find(|f| f.code == "SEC012").unwrap();
    assert!(score_finding.description.contains("100/100"));
}

#[test]
fn test_security_headers_invalid_xfo() {
    let headers = vec![(
        "X-Frame-Options".to_string(),
        "ALLOW-FROM https://example.com".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = AnalysisContext {
        page: &page,
        body: None,
        status_code: Some(200),
        headers: &headers,
        response_time: None,
        redirect_chain: &[],
        robots_txt: None,
        body_size: None,
        compressed_size: None,
        server: None,
        content_type: None,
        rendered: None,
    };
    let findings = SecurityHeaderAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "SEC004"));
}

#[test]
fn test_security_headers_invalid_xcto() {
    let headers = vec![("X-Content-Type-Options".to_string(), "sniff".to_string())];
    let page = make_page("https://example.com");
    let ctx = AnalysisContext {
        page: &page,
        body: None,
        status_code: Some(200),
        headers: &headers,
        response_time: None,
        redirect_chain: &[],
        robots_txt: None,
        body_size: None,
        compressed_size: None,
        server: None,
        content_type: None,
        rendered: None,
    };
    let findings = SecurityHeaderAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "SEC006"));
}

#[test]
fn test_security_headers_xfo_sameorigin() {
    let headers = vec![("X-Frame-Options".to_string(), "SAMEORIGIN".to_string())];
    let page = make_page("https://example.com");
    let ctx = AnalysisContext {
        page: &page,
        body: None,
        status_code: Some(200),
        headers: &headers,
        response_time: None,
        redirect_chain: &[],
        robots_txt: None,
        body_size: None,
        compressed_size: None,
        server: None,
        content_type: None,
        rendered: None,
    };
    let findings = SecurityHeaderAnalyzer::new().analyze(&ctx);
    // SAMEORIGIN is valid, should not flag SEC003 or SEC004
    assert!(!findings.iter().any(|f| f.code == "SEC003"));
    assert!(!findings.iter().any(|f| f.code == "SEC004"));
}

#[test]
fn test_security_headers_hsts_weak_max_age() {
    let headers = vec![(
        "Strict-Transport-Security".to_string(),
        "max-age=300".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = AnalysisContext {
        page: &page,
        body: None,
        status_code: Some(200),
        headers: &headers,
        response_time: None,
        redirect_chain: &[],
        robots_txt: None,
        body_size: None,
        compressed_size: None,
        server: None,
        content_type: None,
        rendered: None,
    };
    let findings = SecurityHeaderAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "SEC014"));
}

#[test]
fn test_security_headers_hsts_missing_max_age() {
    let headers = vec![(
        "Strict-Transport-Security".to_string(),
        "includeSubDomains".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = AnalysisContext {
        page: &page,
        body: None,
        status_code: Some(200),
        headers: &headers,
        response_time: None,
        redirect_chain: &[],
        robots_txt: None,
        body_size: None,
        compressed_size: None,
        server: None,
        content_type: None,
        rendered: None,
    };
    let findings = SecurityHeaderAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "SEC014"));
}

#[test]
fn test_security_headers_invalid_csp() {
    let headers = vec![(
        "Content-Security-Policy".to_string(),
        "invalid-value".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = AnalysisContext {
        page: &page,
        body: None,
        status_code: Some(200),
        headers: &headers,
        response_time: None,
        redirect_chain: &[],
        robots_txt: None,
        body_size: None,
        compressed_size: None,
        server: None,
        content_type: None,
        rendered: None,
    };
    let findings = SecurityHeaderAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "SEC013"));
}

#[test]
fn test_security_headers_valid_csp() {
    let headers = vec![(
        "Content-Security-Policy".to_string(),
        "default-src 'self'; script-src 'self'".to_string(),
    )];
    let page = make_page("https://example.com");
    let ctx = AnalysisContext {
        page: &page,
        body: None,
        status_code: Some(200),
        headers: &headers,
        response_time: None,
        redirect_chain: &[],
        robots_txt: None,
        body_size: None,
        compressed_size: None,
        server: None,
        content_type: None,
        rendered: None,
    };
    let findings = SecurityHeaderAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "SEC013"));
}

#[test]
fn test_security_headers_case_insensitive_lookup() {
    let headers = vec![
        (
            "content-security-policy".to_string(),
            "default-src 'self'".to_string(),
        ),
        (
            "strict-transport-security".to_string(),
            "max-age=63072000".to_string(),
        ),
        ("x-frame-options".to_string(), "DENY".to_string()),
        ("x-content-type-options".to_string(), "nosniff".to_string()),
    ];
    let page = make_page("https://example.com");
    let ctx = AnalysisContext {
        page: &page,
        body: None,
        status_code: Some(200),
        headers: &headers,
        response_time: None,
        redirect_chain: &[],
        robots_txt: None,
        body_size: None,
        compressed_size: None,
        server: None,
        content_type: None,
        rendered: None,
    };
    let findings = SecurityHeaderAnalyzer::new().analyze(&ctx);
    // Lowercase header names should still be found
    assert!(!findings.iter().any(|f| f.code == "SEC001"));
    assert!(!findings.iter().any(|f| f.code == "SEC002"));
    assert!(!findings.iter().any(|f| f.code == "SEC003"));
    assert!(!findings.iter().any(|f| f.code == "SEC005"));
}

#[test]
fn test_security_headers_score_decreases_with_missing() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200));
    let findings = SecurityHeaderAnalyzer::new().analyze(&ctx);
    let score_finding = findings.iter().find(|f| f.code == "SEC012").unwrap();
    // With multiple missing headers, score should be well below 100
    assert!(score_finding.description.contains("/100"));
    // Parse the score
    let score_str = score_finding
        .description
        .split_whitespace()
        .find(|s| s.contains("/100"))
        .unwrap();
    let score_num: u32 = score_str.split('/').next().unwrap().parse().unwrap();
    assert!(score_num < 90);
}

// =========================================================================
// SslCertificateValidator tests
// =========================================================================

#[test]
fn test_ssl_no_data() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200));
    let findings = SslCertificateValidator::empty().analyze(&ctx);
    // With no certificate data captured, the analyzer emits a single
    // informational finding (SSL000) making the limitation visible,
    // rather than silently returning nothing.
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].code, "SSL000");
    assert_eq!(findings[0].severity, Severity::Info);
}

#[test]
fn test_ssl_expired_certificate() {
    let cert = SslCertificateInfo {
        subject: Some("example.com".to_string()),
        issuer: Some("Let's Encrypt".to_string()),
        san_entries: vec!["example.com".to_string()],
        not_before: Some("2024-01-01T00:00:00Z".to_string()),
        not_after: Some("2025-01-01T00:00:00Z".to_string()),
        is_valid_chain: true,
        is_self_signed: false,
        signature_algorithm: Some("SHA256withRSA".to_string()),
    };
    let validator = SslCertificateValidator::new(Some(cert));
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200));
    let findings = validator.analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "SSL001"));
}

#[test]
fn test_ssl_expiring_soon() {
    let cert = SslCertificateInfo {
        subject: Some("example.com".to_string()),
        issuer: Some("Let's Encrypt".to_string()),
        san_entries: vec!["example.com".to_string()],
        not_before: Some("2025-06-01T00:00:00Z".to_string()),
        not_after: Some("2025-08-01T00:00:00Z".to_string()),
        is_valid_chain: true,
        is_self_signed: false,
        signature_algorithm: Some("SHA256withRSA".to_string()),
    };
    let validator = SslCertificateValidator::new(Some(cert));
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200));
    let findings = validator.analyze(&ctx);
    // Should NOT be expired but should be flagged as expiring soon (or not, depending on
    // the current date relative to 2025-08-01)
    let has_expiry_finding = findings
        .iter()
        .any(|f| f.code == "SSL001" || f.code == "SSL002");
    // Given today is 2026, this cert IS expired
    assert!(has_expiry_finding);
}

#[test]
fn test_ssl_valid_certificate() {
    let cert = SslCertificateInfo {
        subject: Some("example.com".to_string()),
        issuer: Some("Let's Encrypt".to_string()),
        san_entries: vec!["example.com".to_string(), "www.example.com".to_string()],
        not_before: Some("2025-01-01T00:00:00Z".to_string()),
        not_after: Some("2027-01-01T00:00:00Z".to_string()),
        is_valid_chain: true,
        is_self_signed: false,
        signature_algorithm: Some("SHA256withRSA".to_string()),
    };
    let validator = SslCertificateValidator::new(Some(cert));
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200));
    let findings = validator.analyze(&ctx);
    // Should not have critical or error findings
    assert!(!findings.iter().any(|f| f.severity == Severity::Critical));
    assert!(!findings.iter().any(|f| f.severity == Severity::Error));
    // Should have the summary finding
    assert!(findings.iter().any(|f| f.code == "SSL008"));
}

#[test]
fn test_ssl_invalid_chain() {
    let cert = SslCertificateInfo {
        subject: Some("example.com".to_string()),
        issuer: Some("Unknown CA".to_string()),
        san_entries: vec!["example.com".to_string()],
        not_before: Some("2025-01-01T00:00:00Z".to_string()),
        not_after: Some("2027-01-01T00:00:00Z".to_string()),
        is_valid_chain: false,
        is_self_signed: false,
        signature_algorithm: Some("SHA256withRSA".to_string()),
    };
    let validator = SslCertificateValidator::new(Some(cert));
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200));
    let findings = validator.analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "SSL003"));
}

#[test]
fn test_ssl_self_signed() {
    let cert = SslCertificateInfo {
        subject: Some("localhost".to_string()),
        issuer: Some("localhost".to_string()),
        san_entries: vec!["localhost".to_string()],
        not_before: Some("2025-01-01T00:00:00Z".to_string()),
        not_after: Some("2027-01-01T00:00:00Z".to_string()),
        is_valid_chain: true,
        is_self_signed: true,
        signature_algorithm: Some("SHA256withRSA".to_string()),
    };
    let validator = SslCertificateValidator::new(Some(cert));
    let page = make_page("https://localhost");
    let ctx = make_ctx(&page, Some(200));
    let findings = validator.analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "SSL006"));
}

#[test]
fn test_ssl_subject_mismatch() {
    let cert = SslCertificateInfo {
        subject: Some("wrong.example.com".to_string()),
        issuer: Some("Let's Encrypt".to_string()),
        san_entries: vec!["wrong.example.com".to_string()],
        not_before: Some("2025-01-01T00:00:00Z".to_string()),
        not_after: Some("2027-01-01T00:00:00Z".to_string()),
        is_valid_chain: true,
        is_self_signed: false,
        signature_algorithm: Some("SHA256withRSA".to_string()),
    };
    let validator = SslCertificateValidator::new(Some(cert));
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200));
    let findings = validator.analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "SSL004"));
}

#[test]
fn test_ssl_wildcard_match() {
    let cert = SslCertificateInfo {
        subject: Some("*.example.com".to_string()),
        issuer: Some("Let's Encrypt".to_string()),
        san_entries: vec!["*.example.com".to_string()],
        not_before: Some("2025-01-01T00:00:00Z".to_string()),
        not_after: Some("2027-01-01T00:00:00Z".to_string()),
        is_valid_chain: true,
        is_self_signed: false,
        signature_algorithm: Some("SHA256withRSA".to_string()),
    };
    let validator = SslCertificateValidator::new(Some(cert));
    let page = make_page("https://sub.example.com");
    let ctx = make_ctx(&page, Some(200));
    let findings = validator.analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "SSL004"));
}

#[test]
fn test_ssl_weak_algorithm() {
    let cert = SslCertificateInfo {
        subject: Some("example.com".to_string()),
        issuer: Some("Let's Encrypt".to_string()),
        san_entries: vec!["example.com".to_string()],
        not_before: Some("2025-01-01T00:00:00Z".to_string()),
        not_after: Some("2027-01-01T00:00:00Z".to_string()),
        is_valid_chain: true,
        is_self_signed: false,
        signature_algorithm: Some("SHA1withRSA".to_string()),
    };
    let validator = SslCertificateValidator::new(Some(cert));
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200));
    let findings = validator.analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "SSL005"));
}

#[test]
fn test_ssl_strong_algorithm() {
    let cert = SslCertificateInfo {
        subject: Some("example.com".to_string()),
        issuer: Some("Let's Encrypt".to_string()),
        san_entries: vec!["example.com".to_string()],
        not_before: Some("2025-01-01T00:00:00Z".to_string()),
        not_after: Some("2027-01-01T00:00:00Z".to_string()),
        is_valid_chain: true,
        is_self_signed: false,
        signature_algorithm: Some("SHA256withECDSA".to_string()),
    };
    let validator = SslCertificateValidator::new(Some(cert));
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200));
    let findings = validator.analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "SSL005"));
}

#[test]
fn test_ssl_san_match() {
    let cert = SslCertificateInfo {
        subject: Some("other.com".to_string()),
        issuer: Some("Let's Encrypt".to_string()),
        san_entries: vec!["other.com".to_string(), "example.com".to_string()],
        not_before: Some("2025-01-01T00:00:00Z".to_string()),
        not_after: Some("2027-01-01T00:00:00Z".to_string()),
        is_valid_chain: true,
        is_self_signed: false,
        signature_algorithm: Some("SHA256withRSA".to_string()),
    };
    let validator = SslCertificateValidator::new(Some(cert));
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200));
    let findings = validator.analyze(&ctx);
    // example.com is in SANs, so no mismatch
    assert!(!findings.iter().any(|f| f.code == "SSL004"));
}

// =========================================================================
// MobileFriendlinessChecker tests
// =========================================================================

#[test]
fn test_mobile_missing_viewport() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200));
    let findings = MobileFriendlinessChecker::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "MOB001"));
}

#[test]
fn test_mobile_optimal_viewport() {
    let mut page = make_page("https://example.com");
    page.meta.viewport = Some("width=device-width, initial-scale=1".to_string());
    let ctx = make_ctx(&page, Some(200));
    let findings = MobileFriendlinessChecker::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "MOB001"));
    assert!(!findings.iter().any(|f| f.code == "MOB002"));
    assert!(!findings.iter().any(|f| f.code == "MOB003"));
    assert!(!findings.iter().any(|f| f.code == "MOB004"));
}

#[test]
fn test_mobile_user_scalable_no() {
    let mut page = make_page("https://example.com");
    page.meta.viewport = Some("width=device-width, initial-scale=1, user-scalable=no".to_string());
    let ctx = make_ctx(&page, Some(200));
    let findings = MobileFriendlinessChecker::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "MOB004"));
}

#[test]
fn test_mobile_maximum_scale_restricted() {
    let mut page = make_page("https://example.com");
    page.meta.viewport = Some("width=device-width, initial-scale=1, maximum-scale=1.0".to_string());
    let ctx = make_ctx(&page, Some(200));
    let findings = MobileFriendlinessChecker::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "MOB005"));
}

#[test]
fn test_mobile_fixed_width() {
    let mut page = make_page("https://example.com");
    page.meta.viewport = Some("width=980".to_string());
    let ctx = make_ctx(&page, Some(200));
    let findings = MobileFriendlinessChecker::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "MOB003"));
}

#[test]
fn test_mobile_missing_width_directive() {
    let mut page = make_page("https://example.com");
    page.meta.viewport = Some("initial-scale=1".to_string());
    let ctx = make_ctx(&page, Some(200));
    let findings = MobileFriendlinessChecker::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "MOB002"));
}

#[test]
fn test_mobile_non_standard_initial_scale() {
    let mut page = make_page("https://example.com");
    page.meta.viewport = Some("width=device-width, initial-scale=2.0".to_string());
    let ctx = make_ctx(&page, Some(200));
    let findings = MobileFriendlinessChecker::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "MOB009"));
}

#[test]
fn test_mobile_parse_viewport() {
    let vp = MobileFriendlinessChecker::parse_viewport(
        "width=device-width, initial-scale=1, user-scalable=no",
    );
    assert_eq!(vp.get("width").unwrap(), "device-width");
    assert_eq!(vp.get("initial-scale").unwrap(), "1");
    assert_eq!(vp.get("user-scalable").unwrap(), "no");
}

#[test]
fn test_mobile_parse_viewport_empty() {
    let vp = MobileFriendlinessChecker::parse_viewport("");
    assert!(vp.is_empty());
}

#[test]
fn test_mobile_both_user_scalable_and_max_scale() {
    let mut page = make_page("https://example.com");
    page.meta.viewport = Some(
        "width=device-width, initial-scale=1, user-scalable=no, maximum-scale=1.0".to_string(),
    );
    let ctx = make_ctx(&page, Some(200));
    let findings = MobileFriendlinessChecker::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "MOB004"));
    assert!(findings.iter().any(|f| f.code == "MOB005"));
}

// =========================================================================
// AccessibilityAnalyzer tests
// =========================================================================

#[test]
fn test_a11y_images_missing_alt() {
    let mut page = make_page("https://example.com");
    page.images = vec![
        ExtractedImage {
            src: "/a.png".to_string(),
            alt: String::new(),
            width: None,
            height: None,
            has_alt: false,
            is_lazy_loaded: false,
            aria_hidden: false,
        },
        ExtractedImage {
            src: "/b.jpg".to_string(),
            alt: "Good alt".to_string(),
            width: None,
            height: None,
            has_alt: true,
            is_lazy_loaded: false,
            aria_hidden: false,
        },
    ];
    let ctx = make_ctx(&page, Some(200));
    let findings = AccessibilityAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "A11Y001"));
    let f = findings.iter().find(|f| f.code == "A11Y001").unwrap();
    assert!(f.description.contains("/a.png"));
}

#[test]
fn test_a11y_images_all_have_alt() {
    let mut page = make_page("https://example.com");
    page.images = vec![ExtractedImage {
        src: "/a.png".to_string(),
        alt: "Description".to_string(),
        width: None,
        height: None,
        has_alt: true,
        is_lazy_loaded: false,
        aria_hidden: false,
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = AccessibilityAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "A11Y001"));
}

#[test]
fn test_a11y_no_headings() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200));
    let findings = AccessibilityAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "A11Y002"));
}

#[test]
fn test_a11y_missing_h1() {
    let mut page = make_page("https://example.com");
    page.headings = vec![Heading {
        level: 2,
        text: "Section".to_string(),
        length: 7,
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = AccessibilityAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "A11Y003"));
}

#[test]
fn test_a11y_multiple_h1() {
    let mut page = make_page("https://example.com");
    page.headings = vec![
        Heading {
            level: 1,
            text: "First".to_string(),
            length: 5,
        },
        Heading {
            level: 1,
            text: "Second".to_string(),
            length: 6,
        },
    ];
    let ctx = make_ctx(&page, Some(200));
    let findings = AccessibilityAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "A11Y004"));
}

#[test]
fn test_a11y_skipped_heading_level() {
    let mut page = make_page("https://example.com");
    page.headings = vec![
        Heading {
            level: 1,
            text: "H1".to_string(),
            length: 2,
        },
        Heading {
            level: 3,
            text: "H3 skipped H2".to_string(),
            length: 13,
        },
    ];
    let ctx = make_ctx(&page, Some(200));
    let findings = AccessibilityAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "A11Y005"));
}

#[test]
fn test_a11y_valid_heading_hierarchy() {
    let mut page = make_page("https://example.com");
    page.headings = vec![
        Heading {
            level: 1,
            text: "H1".to_string(),
            length: 2,
        },
        Heading {
            level: 2,
            text: "H2".to_string(),
            length: 2,
        },
        Heading {
            level: 2,
            text: "H2b".to_string(),
            length: 3,
        },
        Heading {
            level: 3,
            text: "H3".to_string(),
            length: 2,
        },
    ];
    let ctx = make_ctx(&page, Some(200));
    let findings = AccessibilityAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "A11Y005"));
}

#[test]
fn test_a11y_missing_main_landmark() {
    let mut page = make_page("https://example.com");
    page.has_main_landmark = false;
    let ctx = make_ctx(&page, Some(200));
    let findings = AccessibilityAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "A11Y006"));
}

#[test]
fn test_a11y_has_main_landmark() {
    let mut page = make_page("https://example.com");
    page.has_main_landmark = true;
    let ctx = make_ctx(&page, Some(200));
    let findings = AccessibilityAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "A11Y006"));
}

#[test]
fn test_a11y_empty_link_text() {
    let mut page = make_page("https://example.com");
    page.links = vec![ExtractedLink {
        href: "/page".to_string(),
        text: String::new(),
        rel: vec![],
        is_external: false,
        aria_label: None,
        img_alt: None,
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = AccessibilityAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "A11Y009"));
}

#[test]
fn test_a11y_vague_link_text() {
    let mut page = make_page("https://example.com");
    page.links = vec![ExtractedLink {
        href: "/page".to_string(),
        text: "click here".to_string(),
        rel: vec![],
        is_external: false,
        aria_label: None,
        img_alt: None,
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = AccessibilityAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "A11Y010"));
}

#[test]
fn test_a11y_good_link_text() {
    let mut page = make_page("https://example.com");
    page.links = vec![ExtractedLink {
        href: "/pricing".to_string(),
        text: "View pricing details".to_string(),
        rel: vec![],
        is_external: false,
        aria_label: None,
        img_alt: None,
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = AccessibilityAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "A11Y009"));
    assert!(!findings.iter().any(|f| f.code == "A11Y010"));
}

#[test]
fn test_a11y_form_input_missing_label() {
    use crate::parser::ExtractedInput;
    let mut page = make_page("https://example.com");
    page.forms = vec![crate::parser::ExtractedForm {
        action: None,
        method: "post".to_string(),
        input_count: 1,
        has_file_input: false,
        has_search_input: false,
        inputs: vec![ExtractedInput {
            input_type: Some("text".to_string()),
            name: Some("email".to_string()),
            id: None,
            has_label: false,
            aria_label: None,
            aria_labelledby: None,
            aria_describedby: None,
            placeholder: Some("Enter email".to_string()),
            required: true,
        }],
        has_fieldset: false,
        has_legend: false,
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = AccessibilityAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "A11Y011"));
}

#[test]
fn test_a11y_form_input_with_label() {
    use crate::parser::ExtractedInput;
    let mut page = make_page("https://example.com");
    page.forms = vec![crate::parser::ExtractedForm {
        action: None,
        method: "post".to_string(),
        input_count: 1,
        has_file_input: false,
        has_search_input: false,
        inputs: vec![ExtractedInput {
            input_type: Some("text".to_string()),
            name: Some("email".to_string()),
            id: Some("email-input".to_string()),
            has_label: true,
            aria_label: None,
            aria_labelledby: None,
            aria_describedby: None,
            placeholder: None,
            required: true,
        }],
        has_fieldset: false,
        has_legend: false,
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = AccessibilityAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "A11Y011"));
}

#[test]
fn test_a11y_positive_tabindex() {
    let mut page = make_page("https://example.com");
    page.has_positive_tabindex = true;
    let ctx = make_ctx(&page, Some(200));
    let findings = AccessibilityAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "A11Y012"));
}

#[test]
fn test_a11y_no_positive_tabindex() {
    let mut page = make_page("https://example.com");
    page.has_positive_tabindex = false;
    let ctx = make_ctx(&page, Some(200));
    let findings = AccessibilityAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "A11Y012"));
}

#[test]
fn test_a11y_table_missing_headers() {
    let mut page = make_page("https://example.com");
    page.tables_total = 2;
    page.tables_with_headers = 1;
    let ctx = make_ctx(&page, Some(200));
    let findings = AccessibilityAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "A11Y014"));
    let f = findings.iter().find(|f| f.code == "A11Y014").unwrap();
    assert!(f.description.contains("1 of 2"));
}

#[test]
fn test_a11y_table_with_headers() {
    let mut page = make_page("https://example.com");
    page.tables_total = 2;
    page.tables_with_headers = 2;
    let ctx = make_ctx(&page, Some(200));
    let findings = AccessibilityAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "A11Y014"));
}

#[test]
fn test_a11y_missing_lang_attribute() {
    let mut page = make_page("https://example.com");
    page.has_lang_attribute = false;
    let ctx = make_ctx(&page, Some(200));
    let findings = AccessibilityAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "A11Y016"));
}

#[test]
fn test_a11y_has_lang_attribute() {
    let mut page = make_page("https://example.com");
    page.has_lang_attribute = true;
    page.html_lang = Some("en".to_string());
    let ctx = make_ctx(&page, Some(200));
    let findings = AccessibilityAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "A11Y016"));
}

#[test]
fn test_a11y_no_images_no_findings() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200));
    let findings = AccessibilityAnalyzer::new().analyze(&ctx);
    // Should not have A11Y001 (images missing alt) when there are no images
    assert!(!findings.iter().any(|f| f.code == "A11Y001"));
}

#[test]
fn test_a11y_well_accessible_page() {
    let mut page = make_page("https://example.com");
    page.headings = vec![
        Heading {
            level: 1,
            text: "Page Title".to_string(),
            length: 10,
        },
        Heading {
            level: 2,
            text: "Section".to_string(),
            length: 7,
        },
    ];
    page.has_main_landmark = true;
    page.has_nav_landmark = true;
    page.has_skip_link = true;
    page.has_lang_attribute = true;
    page.html_lang = Some("en".to_string());
    page.links = vec![ExtractedLink {
        href: "/page".to_string(),
        text: "click here".to_string(),
        rel: vec![],
        is_external: false,
        aria_label: None,
        img_alt: None,
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = AccessibilityAnalyzer::new().analyze(&ctx);
    // Well-accessible page should have no errors
    let errors: Vec<&Finding> = findings
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "Unexpected errors: {:?}",
        errors.iter().map(|f| &f.code).collect::<Vec<_>>()
    );
}

// =========================================================================
// SocialMediaAnalyzer tests
// =========================================================================

#[test]
fn test_social_og_image_no_dimensions() {
    let mut page = make_page("https://example.com");
    page.meta.og.image = Some("https://example.com/og.png".to_string());
    let ctx = make_ctx(&page, Some(200));
    let findings = SocialMediaAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "SOCIAL001"));
}

#[test]
fn test_social_og_image_adequate_dimensions() {
    let mut page = make_page("https://example.com");
    page.meta.og.image = Some("https://example.com/og.png".to_string());
    page.og_image_width = Some(1200);
    page.og_image_height = Some(630);
    let ctx = make_ctx(&page, Some(200));
    let findings = SocialMediaAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "SOCIAL001"));
    assert!(!findings.iter().any(|f| f.code == "SOCIAL002"));
    assert!(!findings.iter().any(|f| f.code == "SOCIAL003"));
}

#[test]
fn test_social_og_image_too_small() {
    let mut page = make_page("https://example.com");
    page.meta.og.image = Some("https://example.com/og.png".to_string());
    page.og_image_width = Some(600);
    page.og_image_height = Some(315);
    let ctx = make_ctx(&page, Some(200));
    let findings = SocialMediaAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "SOCIAL002"));
    assert!(findings.iter().any(|f| f.code == "SOCIAL003"));
}

#[test]
fn test_social_og_image_width_only_too_narrow() {
    let mut page = make_page("https://example.com");
    page.meta.og.image = Some("https://example.com/og.png".to_string());
    page.og_image_width = Some(800);
    page.og_image_height = Some(630);
    let ctx = make_ctx(&page, Some(200));
    let findings = SocialMediaAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "SOCIAL002"));
    assert!(!findings.iter().any(|f| f.code == "SOCIAL003"));
}

#[test]
fn test_social_missing_twitter_card() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200));
    let findings = SocialMediaAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "SOCIAL004"));
}

#[test]
fn test_social_valid_twitter_card() {
    let mut page = make_page("https://example.com");
    page.meta.twitter.card = Some("summary_large_image".to_string());
    let ctx = make_ctx(&page, Some(200));
    let findings = SocialMediaAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "SOCIAL004"));
    assert!(!findings.iter().any(|f| f.code == "SOCIAL005"));
}

#[test]
fn test_social_invalid_twitter_card() {
    let mut page = make_page("https://example.com");
    page.meta.twitter.card = Some("invalid_type".to_string());
    let ctx = make_ctx(&page, Some(200));
    let findings = SocialMediaAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "SOCIAL005"));
}

#[test]
fn test_social_incomplete_og_tags() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200));
    let findings = SocialMediaAnalyzer::new().analyze(&ctx);
    // Missing og:title, og:description, og:image, og:url, og:type
    assert!(findings.iter().any(|f| f.code == "SOCIAL006"));
    let f = findings.iter().find(|f| f.code == "SOCIAL006").unwrap();
    assert!(f.description.contains("og:title"));
}

#[test]
fn test_social_complete_og_tags() {
    let mut page = make_page("https://example.com");
    page.meta.og.title = Some("Title".to_string());
    page.meta.og.description = Some("Description".to_string());
    page.meta.og.image = Some("https://example.com/og.png".to_string());
    page.meta.og.url = Some("https://example.com".to_string());
    page.meta.og.r#type = Some("website".to_string());
    let ctx = make_ctx(&page, Some(200));
    let findings = SocialMediaAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "SOCIAL006"));
}

#[test]
fn test_social_completeness_score() {
    let mut page = make_page("https://example.com");
    page.meta.og.title = Some("Title".to_string());
    page.meta.og.description = Some("Description".to_string());
    page.meta.og.image = Some("https://example.com/og.png".to_string());
    page.meta.og.url = Some("https://example.com".to_string());
    page.meta.og.r#type = Some("website".to_string());
    page.meta.twitter.card = Some("summary_large_image".to_string());
    page.meta.twitter.title = Some("Title".to_string());
    page.meta.twitter.description = Some("Desc".to_string());
    page.meta.twitter.image = Some("https://example.com/tw.png".to_string());
    let ctx = make_ctx(&page, Some(200));
    let findings = SocialMediaAnalyzer::new().analyze(&ctx);
    let score = findings.iter().find(|f| f.code == "SOCIAL008").unwrap();
    assert!(score.description.contains("8/8"));
}

#[test]
fn test_social_no_og_image_no_dimension_warning() {
    let mut page = make_page("https://example.com");
    page.meta.og.image = None;
    let ctx = make_ctx(&page, Some(200));
    let findings = SocialMediaAnalyzer::new().analyze(&ctx);
    // No OG image = no dimension warning needed
    assert!(!findings.iter().any(|f| f.code == "SOCIAL001"));
}

#[test]
fn test_social_all_twitter_cards_valid() {
    for card_type in &["summary", "summary_large_image", "app", "player"] {
        let mut page = make_page("https://example.com");
        page.meta.twitter.card = Some(card_type.to_string());
        let ctx = make_ctx(&page, Some(200));
        let findings = SocialMediaAnalyzer::new().analyze(&ctx);
        assert!(
            !findings.iter().any(|f| f.code == "SOCIAL005"),
            "Card type {card_type} should be valid"
        );
    }
}

// =========================================================================
// EntityAnalyzer tests
// =========================================================================

#[test]
fn test_entity_analyzer_empty_page() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200));
    let findings = EntityAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "ENTITY005"));
    assert!(findings.iter().any(|f| f.code == "ENTITY006"));
}

#[test]
fn test_entity_detect_people() {
    let text = "Written by Dr. John Smith and edited by Prof. Jane Doe.";
    let people = EntityAnalyzer::detect_people(text);
    assert!(!people.is_empty());
}

#[test]
fn test_entity_detect_organizations() {
    let text = "This product is made by Acme Corporation and Widget Inc.";
    let orgs = EntityAnalyzer::detect_organizations(text);
    assert!(!orgs.is_empty());
}

#[test]
fn test_entity_detect_locations() {
    let text = "The city of London and the mountain region are beautiful.";
    let locs = EntityAnalyzer::detect_locations(text);
    assert!(!locs.is_empty());
}

#[test]
fn test_entity_topics() {
    let headings = vec![
        Heading {
            level: 1,
            text: "Rust Programming Language Tutorial".to_string(),
            length: 33,
        },
        Heading {
            level: 2,
            text: "Advanced Rust Programming Concepts".to_string(),
            length: 35,
        },
    ];
    let topics = EntityAnalyzer::detect_topics(&headings, 500);
    assert!(topics
        .iter()
        .any(|t| t.contains("rust") || t.contains("programming")));
}

#[test]
fn test_entity_sentiment_positive() {
    let (score, label) = EntityAnalyzer::analyze_sentiment(
        "This is a great and amazing product, truly wonderful and excellent!",
    );
    assert!(score > 0.0);
    assert_eq!(label, "positive");
}

#[test]
fn test_entity_sentiment_negative() {
    let (score, label) = EntityAnalyzer::analyze_sentiment(
        "This is a terrible and horrible product, truly awful and bad!",
    );
    assert!(score < 0.0);
    assert_eq!(label, "negative");
}

#[test]
fn test_entity_sentiment_neutral() {
    let (score, label) =
        EntityAnalyzer::analyze_sentiment("The page contains information about the topic.");
    assert_eq!(label, "neutral");
    assert!((-0.05..=0.05).contains(&score));
}

#[test]
fn test_entity_analyzer_with_people() {
    let mut page = make_page("https://example.com");
    page.headings = vec![Heading {
        level: 1,
        text: "Interview with Dr. Alice Smith".to_string(),
        length: 29,
    }];
    page.word_count = 500;
    let ctx = make_ctx(&page, Some(200));
    let findings = EntityAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "ENTITY001"));
}

// =========================================================================
// EnhancedReadabilityAnalyzer tests
// =========================================================================

#[test]
fn test_readability_empty_page() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200));
    let findings = EnhancedReadabilityAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_readability_simple_text() {
    let mut page = make_page("https://example.com");
    page.word_count = 100;
    page.headings = vec![Heading {
        level: 1,
        text: "The cat sat on the mat. The dog ran in the park.".to_string(),
        length: 48,
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = EnhancedReadabilityAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "READ001"));
    assert!(findings.iter().any(|f| f.code == "READ005"));
    let fre = findings.iter().find(|f| f.code == "READ005").unwrap();
    assert!(fre.description.contains("Score:"));
}

#[test]
fn test_readability_complex_text() {
    let mut page = make_page("https://example.com");
    page.word_count = 200;
    page.headings = vec![Heading {
        level: 1,
        text: "The implementation of sophisticated algorithmic methodologies \
               necessitates comprehensive understanding of computational complexity \
               and theoretical frameworks"
            .to_string(),
        length: 150,
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = EnhancedReadabilityAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "READ001"));
    let fk = findings.iter().find(|f| f.code == "READ001").unwrap();
    assert!(fk.description.contains("Grade level:"));
}

#[test]
fn test_readability_syllable_counting() {
    assert_eq!(count_syllables("cat"), 1);
    assert_eq!(count_syllables("hello"), 2);
    assert_eq!(count_syllables("beautiful"), 3);
    assert_eq!(count_syllables("a"), 1);
    assert_eq!(count_syllables(""), 0);
}

#[test]
fn test_readability_indices() {
    let mut page = make_page("https://example.com");
    page.word_count = 500;
    page.headings = vec![Heading {
        level: 1,
        text: "A comprehensive guide to understanding modern web development \
               practices and techniques for beginners and experienced developers"
            .to_string(),
        length: 140,
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = EnhancedReadabilityAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "READ001"));
    assert!(findings.iter().any(|f| f.code == "READ002"));
    assert!(findings.iter().any(|f| f.code == "READ003"));
    assert!(findings.iter().any(|f| f.code == "READ004"));
    assert!(findings.iter().any(|f| f.code == "READ005"));
}

#[test]
fn test_readability_grade_label() {
    assert_eq!(
        EnhancedReadabilityAnalyzer::grade_label(0.5),
        "kindergarten"
    );
    assert_eq!(
        EnhancedReadabilityAnalyzer::grade_label(4.0),
        "elementary school"
    );
    assert_eq!(
        EnhancedReadabilityAnalyzer::grade_label(7.0),
        "middle school"
    );
    assert_eq!(
        EnhancedReadabilityAnalyzer::grade_label(10.0),
        "high school"
    );
    assert_eq!(EnhancedReadabilityAnalyzer::grade_label(14.0), "college");
    assert_eq!(
        EnhancedReadabilityAnalyzer::grade_label(18.0),
        "postgraduate"
    );
}

#[test]
fn test_readability_ease_label() {
    assert_eq!(
        EnhancedReadabilityAnalyzer::reading_ease_label(95.0),
        "very easy"
    );
    assert_eq!(
        EnhancedReadabilityAnalyzer::reading_ease_label(85.0),
        "easy"
    );
    assert_eq!(
        EnhancedReadabilityAnalyzer::reading_ease_label(75.0),
        "fairly easy"
    );
    assert_eq!(
        EnhancedReadabilityAnalyzer::reading_ease_label(65.0),
        "standard"
    );
    assert_eq!(
        EnhancedReadabilityAnalyzer::reading_ease_label(55.0),
        "fairly difficult"
    );
    assert_eq!(
        EnhancedReadabilityAnalyzer::reading_ease_label(35.0),
        "difficult"
    );
    assert_eq!(
        EnhancedReadabilityAnalyzer::reading_ease_label(20.0),
        "very difficult"
    );
}

// =========================================================================
// KeywordAnalyzer tests
// =========================================================================

#[test]
fn test_keyword_analyzer_empty_page() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200));
    let findings = KeywordAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_keyword_analyzer_with_content() {
    let mut page = make_page("https://example.com");
    page.word_count = 500;
    page.headings = vec![
        Heading {
            level: 1,
            text: "Rust Programming Language Tutorial for Beginners".to_string(),
            length: 48,
        },
        Heading {
            level: 2,
            text: "Learn Rust Programming Today".to_string(),
            length: 28,
        },
    ];
    let ctx = make_ctx(&page, Some(200));
    let findings = KeywordAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "KW001"));
    assert!(findings.iter().any(|f| f.code == "KW002"));
}

#[test]
fn test_keyword_tokenize() {
    let tokens = KeywordAnalyzer::tokenize("Rust Programming Language Tutorial");
    assert!(tokens.contains(&"rust".to_string()));
    assert!(tokens.contains(&"programming".to_string()));
    assert!(tokens.contains(&"language".to_string()));
    assert!(tokens.contains(&"tutorial".to_string()));
}

#[test]
fn test_keyword_tf() {
    let tokens = vec![
        "rust".to_string(),
        "programming".to_string(),
        "rust".to_string(),
        "language".to_string(),
    ];
    let tf = KeywordAnalyzer::compute_tf(&tokens);
    assert!(*tf.get("rust").unwrap() > 0.4);
}

#[test]
fn test_keyword_density() {
    let tokens = vec![
        "rust".to_string(),
        "programming".to_string(),
        "rust".to_string(),
    ];
    let density = KeywordAnalyzer::keyword_density(&tokens, 100);
    assert!(*density.get("rust").unwrap() > 1.5);
}

#[test]
fn test_keyword_prominent_detection() {
    let mut density = HashMap::new();
    density.insert("rust".to_string(), 3.5);
    density.insert("programming".to_string(), 1.0);
    let prominent = KeywordAnalyzer::detect_prominent_keywords(&density);
    assert_eq!(prominent.len(), 1);
    assert_eq!(prominent[0].0, "rust");
}

#[test]
fn test_keyword_cooccurrence() {
    let tokens = vec![
        "rust".to_string(),
        "programming".to_string(),
        "language".to_string(),
        "rust".to_string(),
        "programming".to_string(),
    ];
    let cooccur = KeywordAnalyzer::cooccurrence(&tokens, 2);
    assert!(!cooccur.is_empty());
}

#[test]
fn test_keyword_analyzer_prominent_warning() {
    let mut page = make_page("https://example.com");
    page.word_count = 100;
    page.headings = vec![Heading {
        level: 1,
        text: "rust rust rust rust rust rust rust rust rust rust rust".to_string(),
        length: 55,
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = KeywordAnalyzer::new().analyze(&ctx);
    let kw003 = findings.iter().find(|f| f.code == "KW003");
    assert!(kw003.is_some());
    assert_eq!(kw003.unwrap().severity, Severity::Warning);
}

// =========================================================================
// EcommerceSignalsAnalyzer tests
// =========================================================================

#[test]
fn test_ecom_empty_page() {
    let page = make_page("https://example.com");
    let ctx = make_ctx(&page, Some(200));
    let findings = EcommerceSignalsAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn test_ecom_product_schema() {
    let mut page = make_page("https://example.com/product");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Product".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Product",
            "name": "Widget",
            "offers": {
                "@type": "Offer",
                "price": "29.99",
                "priceCurrency": "USD",
                "availability": "https://schema.org/InStock"
            }
        }),
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = EcommerceSignalsAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "ECOM001"));
    assert!(findings.iter().any(|f| f.code == "ECOM002"));
    assert!(findings.iter().any(|f| f.code == "ECOM003"));
    assert!(findings.iter().any(|f| f.code == "ECOM005"));
}

#[test]
fn test_ecom_product_with_reviews() {
    let mut page = make_page("https://example.com/product");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Product".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Product",
            "name": "Widget",
            "aggregateRating": {
                "@type": "AggregateRating",
                "ratingValue": "4.5",
                "reviewCount": "120"
            }
        }),
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = EcommerceSignalsAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "ECOM004"));
}

#[test]
fn test_ecom_price_without_product() {
    let mut page = make_page("https://example.com/product");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Offer".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Offer",
            "price": "19.99",
            "priceCurrency": "USD"
        }),
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = EcommerceSignalsAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "ECOM006"));
}

#[test]
fn test_ecom_no_price() {
    let mut page = make_page("https://example.com/product");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Product".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Product",
            "name": "Widget"
        }),
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = EcommerceSignalsAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "ECOM001"));
    assert!(!findings.iter().any(|f| f.code == "ECOM002"));
}

#[test]
fn test_ecom_non_product_schema() {
    let mut page = make_page("https://example.com/article");
    page.structured_data = vec![StructuredData {
        context: Some("https://schema.org".to_string()),
        r#type: Some("Article".to_string()),
        data: serde_json::json!({
            "@context": "https://schema.org",
            "@type": "Article",
            "headline": "Test"
        }),
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = EcommerceSignalsAnalyzer::new().analyze(&ctx);
    assert!(findings.is_empty());
}

// =========================================================================
// InternationalSeoAnalyzer tests
// =========================================================================

#[test]
fn test_iseo_empty_page() {
    let mut page = make_page("https://example.com");
    page.html_lang = Some("en".to_string());
    let ctx = make_ctx(&page, Some(200));
    let findings = InternationalSeoAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "ISEO004"));
}

#[test]
fn test_iseo_detect_locale_from_url() {
    assert_eq!(
        InternationalSeoAnalyzer::detect_locale_from_url("https://example.com/en/page"),
        Some("en".to_string())
    );
    assert_eq!(
        InternationalSeoAnalyzer::detect_locale_from_url("https://example.com/fr/page"),
        Some("fr".to_string())
    );
    assert_eq!(
        InternationalSeoAnalyzer::detect_locale_from_url("https://example.com/en-US/page"),
        Some("en-US".to_string())
    );
    assert_eq!(
        InternationalSeoAnalyzer::detect_locale_from_url("https://example.com/page"),
        None
    );
}

#[test]
fn test_iseo_is_locale_segment() {
    assert!(InternationalSeoAnalyzer::is_locale_segment("en"));
    assert!(InternationalSeoAnalyzer::is_locale_segment("fr"));
    assert!(InternationalSeoAnalyzer::is_locale_segment("en-US"));
    assert!(InternationalSeoAnalyzer::is_locale_segment("zh-CN"));
    assert!(!InternationalSeoAnalyzer::is_locale_segment("page"));
    assert!(!InternationalSeoAnalyzer::is_locale_segment("e"));
}

#[test]
fn test_iseo_hreflang_url_locale_mismatch() {
    let mut page = make_page("https://example.com/en");
    page.meta.hreflang = vec![crate::meta::HreflangTag {
        lang: "fr".to_string(),
        url: Url::parse("https://example.com/en/about").unwrap(),
    }];
    let ctx = make_ctx(&page, Some(200));
    let findings = InternationalSeoAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "ISEO001"));
}

#[test]
fn test_iseo_missing_x_default() {
    let mut page = make_page("https://example.com/en");
    page.meta.hreflang = vec![
        crate::meta::HreflangTag {
            lang: "en".to_string(),
            url: Url::parse("https://example.com/en").unwrap(),
        },
        crate::meta::HreflangTag {
            lang: "fr".to_string(),
            url: Url::parse("https://example.com/fr").unwrap(),
        },
    ];
    let ctx = make_ctx(&page, Some(200));
    let findings = InternationalSeoAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "ISEO002"));
}

#[test]
fn test_iseo_duplicate_language() {
    let mut page = make_page("https://example.com/en");
    page.meta.hreflang = vec![
        crate::meta::HreflangTag {
            lang: "en".to_string(),
            url: Url::parse("https://example.com/en").unwrap(),
        },
        crate::meta::HreflangTag {
            lang: "en".to_string(),
            url: Url::parse("https://example.com/en-uk").unwrap(),
        },
    ];
    let ctx = make_ctx(&page, Some(200));
    let findings = InternationalSeoAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().any(|f| f.code == "ISEO003"));
}

#[test]
fn test_iseo_multilang_content() {
    // Multilingual presence is deliberately NOT a finding (noise on
    // correctly configured sites); only hreflang *validation* findings
    // fire.
    let mut page = make_page("https://example.com/en");
    page.meta.hreflang = vec![
        crate::meta::HreflangTag {
            lang: "en".to_string(),
            url: Url::parse("https://example.com/en").unwrap(),
        },
        crate::meta::HreflangTag {
            lang: "x-default".to_string(),
            url: Url::parse("https://example.com").unwrap(),
        },
    ];
    page.html_lang = Some("en".to_string());
    let ctx = make_ctx(&page, Some(200));
    let findings = InternationalSeoAnalyzer::new().analyze(&ctx);
    assert!(findings.iter().all(|f| f.code != "ISEO006"));
}

#[test]
fn test_iseo_valid_hreflang_with_xdefault() {
    let mut page = make_page("https://example.com/en");
    page.meta.hreflang = vec![
        crate::meta::HreflangTag {
            lang: "en".to_string(),
            url: Url::parse("https://example.com/en").unwrap(),
        },
        crate::meta::HreflangTag {
            lang: "fr".to_string(),
            url: Url::parse("https://example.com/fr").unwrap(),
        },
        crate::meta::HreflangTag {
            lang: "x-default".to_string(),
            url: Url::parse("https://example.com").unwrap(),
        },
    ];
    let ctx = make_ctx(&page, Some(200));
    let findings = InternationalSeoAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.severity == Severity::Error));
}

#[test]
fn test_iseo_locale_detection_from_url() {
    let locale =
        InternationalSeoAnalyzer::detect_locale_from_url("https://example.com/de/products/widget");
    assert_eq!(locale, Some("de".to_string()));
}

#[test]
fn test_iseo_no_locale_no_hreflang() {
    let mut page = make_page("https://example.com/products");
    page.html_lang = None;
    let ctx = make_ctx(&page, Some(200));
    let findings = InternationalSeoAnalyzer::new().analyze(&ctx);
    assert!(!findings.iter().any(|f| f.code == "ISEO004"));
}
