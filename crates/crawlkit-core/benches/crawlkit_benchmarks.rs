use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::time::Duration;
use url::Url;

use crawlkit_core::analyzers::{
    AnalysisContext, AnalyzerRegistry, HeadingHierarchyAnalyzer, HttpStatusAnalyzer, LinkAnalyzer,
    MetaTagAnalyzer,
};
use crawlkit_core::meta::MetaTags;
use crawlkit_core::parser::{HtmlParser, ParsedPage};
use crawlkit_core::queue::{Priority, ScopeConfig, UrlQueue};
use crawlkit_core::storage::Storage;
use crawlkit_core::{Analyzer, CrawlConfig};

fn make_parsed_page(url: &str, word_count: usize) -> ParsedPage {
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
        word_count,
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

fn bench_parser_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser_throughput");

    let small_html = r#"<!DOCTYPE html><html><head><title>Test</title>
        <meta name="description" content="A test page">
        <link rel="canonical" href="/test">
    </head><body>
        <h1>Hello World</h1>
        <p>This is a test paragraph with some words.</p>
        <a href="/link1">Link 1</a>
        <a href="/link2">Link 2</a>
        <img src="/img.png" alt="Image">
    </body></html>"#;

    let large_html = format!(
        r#"<!DOCTYPE html><html><head><title>{}</title>
        <meta name="description" content="{}">
    </head><body>"#,
        "Test Page Title That Is Reasonably Long",
        "This is a longer description that provides more context about the page content."
    );
    let large_html = format!(
        "{}<h1>Main Content</h1>{}</body></html>",
        large_html,
        (0..500)
            .map(|i| format!(
                "<p>Paragraph {} with some content here that adds to the word count.</p>",
                i
            ))
            .collect::<Vec<_>>()
            .join("\n")
    );

    let url = Url::parse("https://example.com/page").unwrap();

    group.bench_function("small_html", |b| {
        b.iter(|| {
            HtmlParser::parse(black_box(small_html), black_box(&url)).unwrap();
        });
    });

    group.bench_function("large_html_500_paragraphs", |b| {
        b.iter(|| {
            HtmlParser::parse(black_box(&large_html), black_box(&url)).unwrap();
        });
    });

    group.finish();
}

fn bench_analyzer_execution(c: &mut Criterion) {
    let mut group = c.benchmark_group("analyzer_execution");
    let config = CrawlConfig::default();

    let page = make_parsed_page("https://example.com/page", 500);
    let ctx = AnalysisContext {
        page: &page,
        status_code: Some(200),
        headers: &[],
        response_time: Some(Duration::from_millis(200)),
        redirect_chain: &[],
    };

    group.bench_function("meta_tag_analyzer", |b| {
        let analyzer = MetaTagAnalyzer::new();
        b.iter(|| {
            analyzer.analyze(black_box(&ctx), black_box(&config));
        });
    });

    group.bench_function("heading_hierarchy_analyzer", |b| {
        let analyzer = HeadingHierarchyAnalyzer::new();
        b.iter(|| {
            analyzer.analyze(black_box(&ctx), black_box(&config));
        });
    });

    group.bench_function("link_analyzer", |b| {
        let analyzer = LinkAnalyzer::new();
        b.iter(|| {
            analyzer.analyze(black_box(&ctx), black_box(&config));
        });
    });

    group.bench_function("http_status_analyzer", |b| {
        let analyzer = HttpStatusAnalyzer::new();
        b.iter(|| {
            analyzer.analyze(black_box(&ctx), black_box(&config));
        });
    });

    group.finish();
}

fn bench_analyzer_registry(c: &mut Criterion) {
    let mut group = c.benchmark_group("analyzer_registry");
    let config = CrawlConfig::default();
    let registry = AnalyzerRegistry::new(&config);

    let page = make_parsed_page("https://example.com/page", 500);
    let ctx = AnalysisContext {
        page: &page,
        status_code: Some(200),
        headers: &[],
        response_time: Some(Duration::from_millis(200)),
        redirect_chain: &[],
    };

    group.bench_function("all_analyzers_sequential", |b| {
        b.iter(|| {
            registry.analyze(black_box(&ctx), black_box(&config));
        });
    });

    group.finish();
}

fn bench_url_queue(c: &mut Criterion) {
    let mut group = c.benchmark_group("url_queue");

    group.bench_function("push_1000_urls", |b| {
        b.iter(|| {
            let queue = UrlQueue::new(ScopeConfig::default());
            for i in 0..1000 {
                let url = Url::parse(&format!("https://example.com/page{}", i)).unwrap();
                queue.push(url, 0, Priority::NORMAL);
            }
            black_box(&queue);
        });
    });

    group.bench_function("push_pop_1000_urls", |b| {
        b.iter(|| {
            let queue = UrlQueue::new(ScopeConfig::default());
            for i in 0..1000 {
                let url = Url::parse(&format!("https://example.com/page{}", i)).unwrap();
                queue.push(url, i % 5, Priority::new((i % 5) as u8));
            }
            while queue.pop().is_some() {}
            black_box(&queue);
        });
    });

    group.finish();
}

fn bench_storage(c: &mut Criterion) {
    let mut group = c.benchmark_group("storage");

    group.bench_function("insert_100_pages", |b| {
        b.iter_with_setup(
            || {
                let storage = Storage::new_in_memory().unwrap();
                let crawl_id = storage.start_crawl("https://example.com", None).unwrap();
                (storage, crawl_id)
            },
            |(storage, crawl_id)| {
                for i in 0..100 {
                    let page = crawlkit_core::storage::PageData {
                        id: format!("page_{}", i),
                        url: Url::parse(&format!("https://example.com/page{}", i)).unwrap(),
                        final_url: Url::parse(&format!("https://example.com/page{}", i)).unwrap(),
                        status_code: 200,
                        title: Some(format!("Page {}", i)),
                        description: None,
                        canonical_url: None,
                        word_count: Some(500),
                        load_time_ms: Some(200),
                        body_size: Some(1024),
                        fetched_at: chrono::Utc::now(),
                        links: vec![],
                    };
                    storage.insert_page(&crawl_id, &page).unwrap();
                }
            },
        );
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_parser_throughput,
    bench_analyzer_execution,
    bench_analyzer_registry,
    bench_url_queue,
    bench_storage,
);
criterion_main!(benches);
