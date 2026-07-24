#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crawlkit_core::analyzers::AnalyzerRegistry;
use crawlkit_core::backlinks::BacklinkAnalyzer;
use crawlkit_core::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
use crawlkit_core::feature_flags::FeatureFlags;
use crawlkit_core::meta::MetaTags;
use crawlkit_core::parser::{Heading, HtmlParser, ParsedPage, ScriptInfo};
use crawlkit_core::queue::{Priority, ScopeConfig, UrlQueue};
use crawlkit_core::storage::{Issue, IssueCategory, PageData, Severity, Storage};
use crawlkit_core::CrawlConfig;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::time::Duration;
use url::Url;

fn make_bench_page() -> ParsedPage {
    ParsedPage {
        url: "https://example.com/benchmark".to_string(),
        meta: MetaTags {
            title: Some("Benchmark Page".to_string()),
            description: Some("A page for benchmarking".to_string()),
            canonical: Some(Url::parse("https://example.com/benchmark").unwrap()),
            ..Default::default()
        },
        headings: vec![
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
        ],
        links: Vec::new(),
        images: Vec::new(),
        forms: Vec::new(),
        scripts: vec![ScriptInfo {
            src: Some("app.js".to_string()),
            r#async: false,
            defer: false,
            script_type: None,
        }],
        styles: Vec::new(),
        structured_data: Vec::new(),
        word_count: 500,
        landmarks: Vec::new(),
        has_skip_link: false,
        has_main_landmark: true,
        has_nav_landmark: true,
        has_positive_tabindex: false,
        tabindex_negative_count: 0,
        aria_role_count: 5,
        aria_label_count: 3,
        has_lang_attribute: true,
        html_lang: Some("en".to_string()),
        has_aria_hidden: false,
        tables_with_headers: 0,
        tables_total: 0,
        tables_with_captions: 0,
        og_image_width: None,
        og_image_height: None,
    }
}

fn make_5kb_html() -> String {
    let mut html = String::from(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Sample Page for Benchmarking - A Complete HTML Document</title>
    <meta name="description" content="This is a sample HTML page used for benchmarking the parser performance.">
    <link rel="canonical" href="https://example.com/benchmark">
    <meta property="og:title" content="Sample Page">
    <meta property="og:description" content="Benchmark page">
    <meta property="og:image" content="https://example.com/image.jpg">
    <meta name="twitter:card" content="summary_large_image">
    <link rel="stylesheet" href="/styles/main.css">
    <link rel="stylesheet" href="/styles/components.css">
    <script type="application/ld+json">
    {
        "@context": "https://schema.org",
        "@type": "WebPage",
        "name": "Sample Page",
        "description": "A sample page"
    }
    </script>
</head>
<body>
    <header>
        <nav aria-label="Main navigation">
            <ul>
                <li><a href="/">Home</a></li>
                <li><a href="/about">About</a></li>
                <li><a href="/services">Services</a></li>
                <li><a href="/portfolio">Portfolio</a></li>
                <li><a href="/blog">Blog</a></li>
                <li><a href="/contact">Contact</a></li>
            </ul>
        </nav>
    </header>
    <main id="content" role="main">
        <h1>Welcome to Our Sample Website</h1>
        <p class="intro">Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.</p>
        <section>
            <h2>About Our Services</h2>
            <p>Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.</p>
            <img src="/images/team.jpg" alt="Our team working together" width="800" height="400">
            <h3>Web Development</h3>
            <p>Sed ut perspiciatis unde omnis iste natus error sit voluptatem accusantium doloremque laudantium, totam rem aperiam, eaque ipsa quae ab illo inventore veritatis et quasi architecto beatae vitae dicta sunt explicabo.</p>
            <h3>SEO Analysis</h3>
            <p>Nemo enim ipsam voluptatem quia voluptas sit aspernatur aut odit aut fugit, sed quia consequuntur magni dolores eos qui ratione voluptatem sequi nesciunt.</p>
            <h3>Digital Marketing</h3>
            <p>At vero eos et accusamus et iusto odio dignissimos ducimus qui blanditiis praesentium voluptatum deleniti atque corrupti quos dolores et quas molestias excepturi sint occaecati cupiditate non provident.</p>
        </section>
        <section>
            <h2>Our Portfolio</h2>
            <div class="portfolio-grid">
                <article>
                    <h3>Project Alpha</h3>
                    <p>A comprehensive web application built with modern technologies.</p>
                    <a href="/portfolio/alpha">View Project</a>
                </article>
                <article>
                    <h3>Project Beta</h3>
                    <p>An e-commerce platform with advanced analytics integration.</p>
                    <a href="/portfolio/beta">View Project</a>
                </article>
                <article>
                    <h3>Project Gamma</h3>
                    <p>A mobile-first responsive website for a local business.</p>
                    <a href="/portfolio/gamma">View Project</a>
                </article>
            </div>
            <img src="/images/portfolio.jpg" alt="Portfolio showcase" width="1200" height="600">
        </section>
        <section>
            <h2>Contact Us</h2>
            <form action="/contact" method="POST">
                <label for="name">Name</label>
                <input type="text" id="name" name="name" required>
                <label for="email">Email</label>
                <input type="email" id="email" name="email" required>
                <label for="message">Message</label>
                <textarea id="message" name="message" rows="5" required></textarea>
                <button type="submit">Send Message</button>
            </form>
        </section>
        <section>
            <h2>Latest Blog Posts</h2>
            <article>
                <h3><a href="/blog/post1">Understanding Web Performance</a></h3>
                <p>Published on January 15, 2024 by John Doe</p>
                <p>Learn how to optimize your website for better performance and user experience.</p>
            </article>
            <article>
                <h3><a href="/blog/post2">SEO Best Practices for 2024</a></h3>
                <p>Published on January 22, 2024 by Jane Smith</p>
                <p>Discover the latest trends and techniques in search engine optimization.</p>
            </article>
            <article>
                <h3><a href="/blog/post3">Building Accessible Websites</a></h3>
                <p>Published on February 1, 2024 by John Doe</p>
                <p>Why accessibility matters and how to implement it in your projects.</p>
            </article>
        </section>
        <table>
            <caption>Company Statistics</caption>
            <thead>
                <tr>
                    <th>Metric</th>
                    <th>2022</th>
                    <th>2023</th>
                    <th>2024</th>
                </tr>
            </thead>
            <tbody>
                <tr>
                    <td>Projects Completed</td>
                    <td>45</td>
                    <td>62</td>
                    <td>78</td>
                </tr>
                <tr>
                    <td>Happy Clients</td>
                    <td>30</td>
                    <td>45</td>
                    <td>58</td>
                </tr>
                <tr>
                    <td>Team Members</td>
                    <td>8</td>
                    <td>12</td>
                    <td>15</td>
                </tr>
            </tbody>
        </table>
    </main>
    <footer>
        <p>&copy; 2024 Sample Website. All rights reserved.</p>
        <nav aria-label="Footer navigation">
            <a href="/privacy">Privacy Policy</a>
            <a href="/terms">Terms of Service</a>
            <a href="/sitemap.xml">Sitemap</a>
        </nav>
    </footer>
    <script src="/js/app.js" defer></script>
    <script src="/js/analytics.js" async></script>
</body>
</html>"#,
    );
    // Pad to ~5KB
    while html.len() < 5120 {
        html.push_str("\n<!-- padding for benchmark -->\n");
        for i in 0..10 {
            html.push_str(&format!(
                "<!-- padding line {i}: Lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod -->\n"
            ));
        }
    }
    html.truncate(5120);
    html
}

fn bench_html_parser(c: &mut Criterion) {
    let url = Url::parse("https://example.com/test").unwrap();
    let html_5kb = make_5kb_html();

    let mut group = c.benchmark_group("html_parser");
    group.bench_function("parse_small_page", |b| {
        let small_html = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <title>Test Page</title>
    <meta name="description" content="Test">
    <link rel="canonical" href="https://example.com">
</head>
<body>
    <h1>Main Title</h1>
    <h2>Section 1</h2>
    <p>Lorem ipsum dolor sit amet.</p>
    <a href="/about">About</a>
    <img src="/image.jpg" alt="Image" width="100" height="200">
    <script type="application/ld+json">{"@type": "WebPage"}</script>
</body>
</html>"#;
        b.iter(|| HtmlParser::parse(black_box(small_html), &url))
    });

    group.bench_function("parse_5kb_page", |b| {
        b.iter(|| HtmlParser::parse(black_box(&html_5kb), &url))
    });

    group.finish();
}

fn bench_analyzer_registry(c: &mut Criterion) {
    let config = CrawlConfig::default();
    let registry = AnalyzerRegistry::new(&config);
    let page = make_bench_page();

    c.bench_function("analyzer_registry_full_suite", |b| {
        b.iter(|| {
            let ctx = crawlkit_core::AnalysisContext {
                page: black_box(&page),
                status_code: Some(200),
                headers: &[],
                response_time: Some(Duration::from_millis(100)),
                redirect_chain: &[],
                robots_txt: None,
            };
            registry.analyze(&ctx, &config)
        })
    });
}

fn bench_queue_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("url_queue");

    group.bench_function("push_1000_urls", |b| {
        b.iter_batched(
            || UrlQueue::new(ScopeConfig::default()),
            |queue| {
                for i in 0..1000 {
                    let url = Url::parse(&format!("https://example.com/page/{i}")).unwrap();
                    queue.push(url, 0, Priority::NORMAL);
                }
                black_box(&queue);
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.bench_function("pop_1000_urls", |b| {
        b.iter_batched(
            || {
                let queue = UrlQueue::new(ScopeConfig::default());
                for i in 0..1000 {
                    let url = Url::parse(&format!("https://example.com/page/{i}")).unwrap();
                    queue.push(url, 0, Priority::NORMAL);
                }
                queue
            },
            |queue| {
                for _ in 0..1000 {
                    black_box(queue.pop());
                }
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.bench_function("push_pop_mixed_priorities", |b| {
        b.iter_batched(
            || UrlQueue::new(ScopeConfig::default()),
            |queue| {
                for i in 0usize..1000 {
                    let url = Url::parse(&format!("https://example.com/page/{i}")).unwrap();
                    let priority = match i % 5 {
                        0 => Priority::HIGHEST,
                        1 => Priority::HIGH,
                        2 => Priority::NORMAL,
                        3 => Priority::LOW,
                        _ => Priority::LOWEST,
                    };
                    queue.push(url, i % 10, priority);
                }
                for _ in 0..1000 {
                    black_box(queue.pop());
                }
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn bench_storage_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("storage");

    group.bench_function("insert_100_pages", |b| {
        b.iter_batched(
            || {
                let storage = Storage::new_in_memory().unwrap();
                let crawl_id = storage.start_crawl("https://example.com", None).unwrap();
                (storage, crawl_id)
            },
            |(storage, crawl_id)| {
                let pages: Vec<PageData> = (0..100)
                    .map(|i| PageData {
                        id: format!("page-{i}"),
                        url: Url::parse(&format!("https://example.com/page/{i}")).unwrap(),
                        final_url: Url::parse(&format!("https://example.com/page/{i}")).unwrap(),
                        status_code: 200,
                        title: Some(format!("Page {i}")),
                        description: None,
                        canonical_url: None,
                        word_count: Some(500 + i),
                        load_time_ms: Some(100 + i as u64),
                        body_size: Some(4096),
                        fetched_at: chrono::Utc::now(),
                        links: vec![],
                    })
                    .collect();
                storage.insert_pages(&crawl_id, &pages).unwrap();
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.bench_function("query_pages", |b| {
        b.iter_batched(
            || {
                let storage = Storage::new_in_memory().unwrap();
                let crawl_id = storage.start_crawl("https://example.com", None).unwrap();
                let pages: Vec<PageData> = (0..100)
                    .map(|i| PageData {
                        id: format!("page-{i}"),
                        url: Url::parse(&format!("https://example.com/page/{i}")).unwrap(),
                        final_url: Url::parse(&format!("https://example.com/page/{i}")).unwrap(),
                        status_code: 200,
                        title: Some(format!("Page {i}")),
                        description: None,
                        canonical_url: None,
                        word_count: Some(500),
                        load_time_ms: Some(100),
                        body_size: Some(4096),
                        fetched_at: chrono::Utc::now(),
                        links: vec![],
                    })
                    .collect();
                storage.insert_pages(&crawl_id, &pages).unwrap();
                (storage, crawl_id)
            },
            |(storage, crawl_id)| {
                let pages = storage.get_pages(&crawl_id, 100).unwrap();
                black_box(pages);
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.bench_function("insert_and_query_issues", |b| {
        b.iter_batched(
            || {
                let storage = Storage::new_in_memory().unwrap();
                let crawl_id = storage.start_crawl("https://example.com", None).unwrap();
                let page = PageData {
                    id: "page-0".to_string(),
                    url: Url::parse("https://example.com").unwrap(),
                    final_url: Url::parse("https://example.com").unwrap(),
                    status_code: 200,
                    title: Some("Page".to_string()),
                    description: None,
                    canonical_url: None,
                    word_count: Some(500),
                    load_time_ms: Some(100),
                    body_size: Some(4096),
                    fetched_at: chrono::Utc::now(),
                    links: vec![],
                };
                storage.insert_page(&crawl_id, &page).unwrap();
                (storage, crawl_id)
            },
            |(storage, crawl_id)| {
                let issues: Vec<Issue> = (0..50)
                    .map(|i| Issue {
                        id: format!("issue-{i}"),
                        page_id: "page-0".to_string(),
                        category: IssueCategory::Seo,
                        severity: Severity::Warning,
                        code: format!("SEO{i:03}"),
                        title: format!("Issue {i}"),
                        description: format!("Description for issue {i}"),
                        element: None,
                        recommendation: "Fix this".to_string(),
                    })
                    .collect();
                storage.insert_issues(&issues).unwrap();
                let retrieved = storage
                    .get_issues(&crawl_id, &crawlkit_core::storage::IssueFilter::default())
                    .unwrap();
                black_box(retrieved);
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn bench_circuit_breaker(c: &mut Criterion) {
    let config = CircuitBreakerConfig {
        failure_threshold: 5,
        success_threshold: 3,
        cooldown: Duration::from_secs(60),
    };

    c.bench_function("circuit_breaker_state_check", |b| {
        let cb = CircuitBreaker::new(config.clone());
        b.iter(|| {
            black_box(cb.is_allowed());
        })
    });
}

fn bench_link_graph_pagerank(c: &mut Criterion) {
    let mut analyzer = BacklinkAnalyzer::new();
    for i in 0_usize..100 {
        let source = format!("https://example.com/page{}", i);
        let target = format!("https://example.com/page{}", (i + 1) % 100);
        analyzer.add_link(&source, &target);
        if i % 10 == 0 {
            let backlink = format!("https://example.com/page{}", (i + 50) % 100);
            analyzer.add_link(&backlink, &source);
        }
    }

    c.bench_function("link_graph_pagerank_100_nodes", |b| {
        b.iter(|| {
            black_box(analyzer.compute_pagerank(black_box(0.85), 20));
        })
    });
}

fn bench_feature_flags(c: &mut Criterion) {
    let flags = FeatureFlags::default();
    c.bench_function("feature_flags_get", |b| {
        b.iter(|| {
            black_box(flags.get("ai_analyzers"));
        })
    });
}

criterion_group!(
    benches,
    bench_html_parser,
    bench_analyzer_registry,
    bench_queue_operations,
    bench_storage_operations,
    bench_circuit_breaker,
    bench_link_graph_pagerank,
    bench_feature_flags,
);

criterion_main!(benches);
