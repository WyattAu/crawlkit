use super::*;

fn test_url() -> Url {
    Url::parse("https://example.com/page").unwrap()
}

#[test]
fn test_parse_title() {
    let html = r#"<!DOCTYPE html><html><head><title>My Page</title></head><body></body></html>"#;
    let page = HtmlParser::parse(html, &test_url()).unwrap();
    assert_eq!(page.meta.title.as_deref(), Some("My Page"));
}

#[test]
fn test_parse_meta_description() {
    let html = r#"<!DOCTYPE html><html><head>
        <meta name="description" content="A great page">
    </head><body></body></html>"#;
    let page = HtmlParser::parse(html, &test_url()).unwrap();
    assert_eq!(page.meta.description.as_deref(), Some("A great page"));
}

#[test]
fn test_parse_canonical() {
    let html = r#"<!DOCTYPE html><html><head>
        <link rel="canonical" href="/canonical-page">
    </head><body></body></html>"#;
    let page = HtmlParser::parse(html, &test_url()).unwrap();
    assert!(page.meta.canonical.is_some());
    assert!(page
        .meta
        .canonical
        .unwrap()
        .as_str()
        .contains("canonical-page"));
}

#[test]
fn test_parse_open_graph() {
    let html = r#"<!DOCTYPE html><html><head>
        <meta property="og:title" content="OG Title">
        <meta property="og:description" content="OG Desc">
        <meta property="og:image" content="https://example.com/img.png">
        <meta property="og:url" content="https://example.com">
        <meta property="og:type" content="website">
    </head><body></body></html>"#;
    let page = HtmlParser::parse(html, &test_url()).unwrap();
    assert_eq!(page.meta.og.title.as_deref(), Some("OG Title"));
    assert_eq!(page.meta.og.description.as_deref(), Some("OG Desc"));
    assert_eq!(
        page.meta.og.image.as_deref(),
        Some("https://example.com/img.png")
    );
    assert_eq!(page.meta.og.r#type.as_deref(), Some("website"));
}

#[test]
fn test_parse_twitter_cards() {
    let html = r#"<!DOCTYPE html><html><head>
        <meta name="twitter:card" content="summary_large_image">
        <meta name="twitter:site" content="@example">
        <meta name="twitter:creator" content="@author">
        <meta name="twitter:title" content="TW Title">
        <meta name="twitter:description" content="TW Desc">
        <meta name="twitter:image" content="https://example.com/tw.png">
    </head><body></body></html>"#;
    let page = HtmlParser::parse(html, &test_url()).unwrap();
    assert_eq!(
        page.meta.twitter.card.as_deref(),
        Some("summary_large_image")
    );
    assert_eq!(page.meta.twitter.site.as_deref(), Some("@example"));
    assert_eq!(page.meta.twitter.creator.as_deref(), Some("@author"));
    assert_eq!(page.meta.twitter.title.as_deref(), Some("TW Title"));
}

#[test]
fn test_parse_headings() {
    let html = r#"<!DOCTYPE html><html><body>
        <h1>Main Title</h1>
        <h2>Section</h2>
        <h2>Another</h2>
        <h3>Sub</h3>
    </body></html>"#;
    let page = HtmlParser::parse(html, &test_url()).unwrap();
    assert_eq!(page.headings.len(), 4);
    assert_eq!(page.headings[0].level, 1);
    assert_eq!(page.headings[0].text, "Main Title");
    assert_eq!(page.headings[1].level, 2);
    assert_eq!(page.headings[2].level, 2);
    assert_eq!(page.headings[3].level, 3);
}

#[test]
fn test_extract_links() {
    let html = r#"<!DOCTYPE html><html><body>
        <a href="/internal">Internal</a>
        <a href="https://external.com/page">External</a>
        <a href="/page" rel="nofollow noopen">Nofollow</a>
    </body></html>"#;
    let page = HtmlParser::parse(html, &test_url()).unwrap();
    assert_eq!(page.links.len(), 3);

    assert_eq!(page.links[0].href, "https://example.com/internal");
    assert_eq!(page.links[0].text, "Internal");
    assert!(!page.links[0].is_external);

    assert_eq!(page.links[1].href, "https://external.com/page");
    assert!(page.links[1].is_external);

    assert!(page.links[2].rel.contains(&"nofollow".to_string()));
}

#[test]
fn test_extract_images() {
    let html = r#"<!DOCTYPE html><html><body>
        <img src="/img1.png" alt="Picture" width="100" height="200">
        <img src="/img2.jpg">
        <img src="/img3.webp" loading="lazy" data-src="/img3-real.webp">
    </body></html>"#;
    let page = HtmlParser::parse(html, &test_url()).unwrap();
    assert_eq!(page.images.len(), 3);

    assert_eq!(page.images[0].src, "/img1.png");
    assert_eq!(page.images[0].alt, "Picture");
    assert_eq!(page.images[0].width, Some(100));
    assert_eq!(page.images[0].height, Some(200));
    assert!(page.images[0].has_alt);
    assert!(!page.images[0].is_lazy_loaded);

    assert!(!page.images[1].has_alt);

    assert!(page.images[2].is_lazy_loaded);
}

#[test]
fn test_extract_forms() {
    let html = r#"<!DOCTYPE html><html><body>
        <form action="/submit" method="post">
            <input type="text" name="q">
            <input type="file" name="doc">
        </form>
        <form>
            <input type="search" name="s">
        </form>
    </body></html>"#;
    let page = HtmlParser::parse(html, &test_url()).unwrap();
    assert_eq!(page.forms.len(), 2);

    assert_eq!(page.forms[0].action.as_deref(), Some("/submit"));
    assert_eq!(page.forms[0].method, "post");
    assert_eq!(page.forms[0].input_count, 2);
    assert!(page.forms[0].has_file_input);

    assert!(page.forms[1].has_search_input);
}

#[test]
fn test_extract_scripts() {
    let html = r#"<!DOCTYPE html><html><body>
        <script src="/app.js" async></script>
        <script defer src="/lib.js"></script>
        <script type="application/ld+json">{"@type":"WebSite"}</script>
        <script>console.log("hi")</script>
    </body></html>"#;
    let page = HtmlParser::parse(html, &test_url()).unwrap();
    // 4 total script tags
    assert_eq!(page.scripts.len(), 4);
    assert!(page.scripts[0].r#async);
    assert!(page.scripts[1].defer);
    assert_eq!(
        page.scripts[2].script_type.as_deref(),
        Some("application/ld+json")
    );
}

#[test]
fn test_extract_styles() {
    let html = r#"<!DOCTYPE html><html><head>
        <link rel="stylesheet" href="/style.css">
        <link rel="stylesheet" href="/print.css" media="print">
        <style>body { margin: 0; }</style>
    </head><body></body></html>"#;
    let page = HtmlParser::parse(html, &test_url()).unwrap();
    assert_eq!(page.styles.len(), 3);
    assert!(!page.styles[0].is_inline);
    assert_eq!(page.styles[0].href.as_deref(), Some("/style.css"));
    assert_eq!(page.styles[1].media.as_deref(), Some("print"));
    assert!(page.styles[2].is_inline);
}

#[test]
fn test_extract_json_ld() {
    let html = r#"<!DOCTYPE html><html><body>
        <script type="application/ld+json">
        {
            "@context": "https://schema.org",
            "@type": "Article",
            "headline": "Test"
        }
        </script>
    </body></html>"#;
    let page = HtmlParser::parse(html, &test_url()).unwrap();
    assert_eq!(page.structured_data.len(), 1);
    assert_eq!(
        page.structured_data[0].context.as_deref(),
        Some("https://schema.org")
    );
    assert_eq!(page.structured_data[0].r#type.as_deref(), Some("Article"));
}

#[test]
fn test_word_count() {
    let html = r#"<!DOCTYPE html><html><body>
        <h1>Hello World</h1>
        <p>This is a test paragraph with some words.</p>
        <script>var x = 1;</script>
        <style>.a { color: red; }</style>
    </body></html>"#;
    let page = HtmlParser::parse(html, &test_url()).unwrap();
    // "Hello World" = 2, "This is a test paragraph with some words." = 8
    assert_eq!(page.word_count, 10);
}

#[test]
fn test_word_count_excludes_script_and_style() {
    let html = r#"<!DOCTYPE html><html><body>
        <p>Visible text here.</p>
        <script>function foo() { return "not counted"; }</script>
        <style>.hidden { display: none; }</style>
        <noscript>JavaScript is required</noscript>
    </body></html>"#;
    let page = HtmlParser::parse(html, &test_url()).unwrap();
    // Only "Visible text here." should count = 3
    assert_eq!(page.word_count, 3);
}

#[test]
fn test_hreflang() {
    let html = r#"<!DOCTYPE html><html><head>
        <link rel="alternate" hreflang="en" href="https://example.com/en">
        <link rel="alternate" hreflang="fr" href="https://example.com/fr">
        <link rel="alternate" hreflang="x-default" href="https://example.com">
    </head><body></body></html>"#;
    let page = HtmlParser::parse(html, &test_url()).unwrap();
    assert_eq!(page.meta.hreflang.len(), 3);
    assert_eq!(page.meta.hreflang[0].lang, "en");
    assert_eq!(page.meta.hreflang[1].lang, "fr");
    assert_eq!(page.meta.hreflang[2].lang, "x-default");
}

#[test]
fn test_robots_meta() {
    let html = r#"<!DOCTYPE html><html><head>
        <meta name="robots" content="noindex, nofollow">
    </head><body></body></html>"#;
    let page = HtmlParser::parse(html, &test_url()).unwrap();
    assert!(page.meta.is_noindex());
    assert!(page.meta.is_nofollow());
}

#[test]
fn test_language_and_charset() {
    let html = r#"<!DOCTYPE html><html lang="en"><head>
        <meta charset="utf-8">
        <meta name="viewport" content="width=device-width, initial-scale=1">
    </head><body></body></html>"#;
    let page = HtmlParser::parse(html, &test_url()).unwrap();
    assert_eq!(page.meta.language.as_deref(), Some("en"));
    assert!(page.meta.charset.is_some());
    assert!(page.meta.viewport.is_some());
}

#[test]
fn test_empty_html() {
    let page = HtmlParser::parse("", &test_url()).unwrap();
    assert!(page.meta.title.is_none());
    assert!(page.headings.is_empty());
    assert!(page.links.is_empty());
    assert!(page.images.is_empty());
    assert_eq!(page.word_count, 0);
}

#[test]
fn test_title_fallback_to_og() {
    let html = r#"<!DOCTYPE html><html><head>
        <meta property="og:title" content="OG Fallback Title">
    </head><body></body></html>"#;
    let page = HtmlParser::parse(html, &test_url()).unwrap();
    assert_eq!(page.meta.title.as_deref(), Some("OG Fallback Title"));
}

#[test]
fn test_parsed_page_serialization() {
    let html = r#"<!DOCTYPE html><html><head><title>Test</title></head>
    <body><h1>Hi</h1><a href="/link">link</a></body></html>"#;
    let page = HtmlParser::parse(html, &test_url()).unwrap();
    let json = serde_json::to_string(&page).unwrap();
    let deser: ParsedPage = serde_json::from_str(&json).unwrap();
    assert_eq!(page.meta.title, deser.meta.title);
    assert_eq!(page.headings.len(), deser.headings.len());
    assert_eq!(page.word_count, deser.word_count);
}

#[test]
fn test_streaming_parser_new() {
    let parser = StreamingHtmlParser::new();
    assert_eq!(parser.buffer_size(), 0);
    assert!(!parser.has_complete_document());
}

#[test]
fn test_streaming_parser_default() {
    let parser = StreamingHtmlParser::default();
    assert_eq!(parser.buffer_size(), 0);
}

#[test]
fn test_streaming_parser_feed() {
    let mut parser = StreamingHtmlParser::new();
    parser.feed("<html><body>");
    assert_eq!(parser.buffer_size(), 12);
    assert!(!parser.has_complete_document());

    parser.feed("<h1>Hello</h1>");
    assert_eq!(parser.buffer_size(), 26);
    assert!(!parser.has_complete_document());
}

#[test]
fn test_streaming_parser_complete_document_html() {
    let mut parser = StreamingHtmlParser::new();
    parser.feed("<!DOCTYPE html><html><head><title>Test</title></head>");
    parser.feed("<body><h1>Hello</h1></body></html>");
    assert!(parser.has_complete_document());
}

#[test]
fn test_streaming_parser_complete_document_body() {
    let mut parser = StreamingHtmlParser::new();
    parser.feed("<!DOCTYPE html><html><head><title>Test</title></head>");
    parser.feed("<body><h1>Hello</h1></body>");
    assert!(parser.has_complete_document());
}

#[test]
fn test_streaming_parser_parse() {
    let mut parser = StreamingHtmlParser::new();
    parser.feed(r#"<!DOCTYPE html><html><head><title>Stream Test</title></head>"#);
    parser.feed(r#"<body><h1>Hello World</h1><a href="/link">link</a></body></html>"#);

    let page = parser.parse().unwrap();
    assert_eq!(page.meta.title.as_deref(), Some("Stream Test"));
    assert_eq!(page.url, "about:blank");
}

#[test]
fn test_streaming_parser_parse_incomplete() {
    let mut parser = StreamingHtmlParser::new();
    parser.feed(r#"<!DOCTYPE html><html><head><title>Incomplete</title></head>"#);
    parser.feed(r#"<body><h1>Partial content"#);

    // Should still parse, even without complete document markers
    let page = parser.parse().unwrap();
    assert_eq!(page.meta.title.as_deref(), Some("Incomplete"));
}

#[test]
fn test_streaming_parser_clear() {
    let mut parser = StreamingHtmlParser::new();
    parser.feed("<html><body><h1>Hello</h1></body></html>");
    assert_eq!(parser.buffer_size(), 40);

    parser.clear();
    assert_eq!(parser.buffer_size(), 0);
    assert!(!parser.has_complete_document());
}

#[test]
fn test_streaming_parser_into_inner() {
    let mut parser = StreamingHtmlParser::new();
    parser.feed("<html><body></body></html>");
    let buffer = parser.into_inner();
    assert_eq!(buffer, "<html><body></body></html>");
}

#[test]
fn test_streaming_parser_multiple_chunks() {
    let mut parser = StreamingHtmlParser::new();
    parser.feed("<!DOCTYPE html>");
    parser.feed("<html>");
    parser.feed("<head>");
    parser.feed("<title>Multi Chunk</title>");
    parser.feed("</head>");
    parser.feed("<body>");
    parser.feed("<p>Content</p>");
    parser.feed("</body>");
    parser.feed("</html>");

    assert!(parser.has_complete_document());
    let page = parser.parse().unwrap();
    assert_eq!(page.meta.title.as_deref(), Some("Multi Chunk"));
    assert_eq!(page.word_count, 1);
}

#[cfg(test)]
mod lang_test {
    use super::*;
    use url::Url;

    #[test]
    fn test_lang_attribute_from_html() {
        let html = r#"<!DOCTYPE html><html lang="en" data-theme="midnight-navy"><head><title>Test</title></head><body></body></html>"#;
        let url = Url::parse("https://example.com").unwrap();
        let page = HtmlParser::parse(html, &url).unwrap();
        // Check the accessibility section
        assert!(
            page.has_lang_attribute,
            "should detect lang attribute from raw HTML fallback"
        );
        assert_eq!(page.html_lang.as_deref(), Some("en"));
    }
}

#[cfg(all(test, feature = "full"))]
#[allow(clippy::unwrap_used)]
mod streaming_tests {
    use super::*;

    fn test_url() -> Url {
        Url::parse("https://example.com/page").unwrap()
    }

    #[tokio::test]
    async fn test_parse_stream_basic() {
        let (tx, rx) = tokio::sync::mpsc::channel::<Vec<u8>>(16);
        let mut events = HtmlParser::parse_stream(rx, test_url());

        tx.send(b"<!DOCTYPE html><html><head><title>Stream</title></head>".into())
            .await
            .unwrap();
        tx.send(b"<body><a href=\"/link\">Link</a></body></html>".into())
            .await
            .unwrap();
        drop(tx);

        let mut chunks = 0usize;
        let mut links_received = false;
        let mut done_page = None;

        while let Some(event) = events.recv().await {
            match event {
                ParserEvent::Chunk(n) => {
                    chunks += n;
                }
                ParserEvent::Links(links) => {
                    links_received = true;
                    assert!(!links.is_empty());
                }
                ParserEvent::Done(page) => {
                    done_page = Some(page);
                }
                ParserEvent::Error(e) => panic!("unexpected error: {e}"),
                _ => {}
            }
        }

        assert!(chunks > 0);
        assert!(links_received);
        let page = done_page.expect("should receive Done event");
        assert_eq!(page.meta.title.as_deref(), Some("Stream"));
        assert_eq!(page.links.len(), 1);
        assert_eq!(page.links[0].href, "https://example.com/link");
    }

    #[tokio::test]
    async fn test_parse_stream_meta_emitted() {
        let (tx, rx) = tokio::sync::mpsc::channel::<Vec<u8>>(16);
        let mut events = HtmlParser::parse_stream(rx, test_url());

        tx.send(
            r#"<!DOCTYPE html><html><head>
        <meta name="description" content="A description">
        <title>Meta Test</title>
    </head><body></body></html>"#
                .as_bytes()
                .to_vec(),
        )
        .await
        .unwrap();
        drop(tx);

        let mut meta_received = false;
        while let Some(event) = events.recv().await {
            if let ParserEvent::Meta(meta) = event {
                meta_received = true;
                assert_eq!(meta.title.as_deref(), Some("Meta Test"));
                assert_eq!(meta.description.as_deref(), Some("A description"));
            }
        }
        assert!(meta_received);
    }

    #[tokio::test]
    async fn test_parse_stream_deduplicates_links() {
        let (tx, rx) = tokio::sync::mpsc::channel::<Vec<u8>>(16);
        let mut events = HtmlParser::parse_stream(rx, test_url());

        // Send same chunk twice — links should appear only once
        let chunk = br#"<!DOCTYPE html><html><body>
        <a href="/dup">Dup</a>
        <a href="/dup">Dup Again</a>
    </body></html>"#;

        tx.send(chunk.to_vec()).await.unwrap();
        tx.send(chunk.to_vec()).await.unwrap();
        drop(tx);

        let mut all_links = Vec::new();
        while let Some(event) = events.recv().await {
            if let ParserEvent::Links(links) = event {
                all_links.extend(links);
            }
        }

        let dup_links: Vec<_> = all_links
            .iter()
            .filter(|l| l.href == "https://example.com/dup")
            .collect();
        assert_eq!(dup_links.len(), 1, "duplicate link should appear only once");
    }

    #[tokio::test]
    async fn test_parse_stream_error_on_empty() {
        let (tx, rx) = tokio::sync::mpsc::channel::<Vec<u8>>(16);
        let mut events = HtmlParser::parse_stream(rx, test_url());
        drop(tx);

        let mut got_done = false;
        while let Some(event) = events.recv().await {
            if let ParserEvent::Done(page) = event {
                got_done = true;
                assert!(page.meta.title.is_none());
                assert!(page.links.is_empty());
            }
        }
        assert!(got_done);
    }

    #[tokio::test]
    async fn test_parse_stream_external_links() {
        let (tx, rx) = tokio::sync::mpsc::channel::<Vec<u8>>(16);
        let mut events = HtmlParser::parse_stream(rx, test_url());

        tx.send(
            br#"<!DOCTYPE html><html><body>
        <a href="https://other.com/ext">External</a>
        <a href="/internal">Internal</a>
    </body></html>"#
                .to_vec(),
        )
        .await
        .unwrap();
        drop(tx);

        let mut all_links = Vec::new();
        while let Some(event) = events.recv().await {
            if let ParserEvent::Links(links) = event {
                all_links.extend(links);
            }
        }

        let external: Vec<_> = all_links.iter().filter(|l| l.is_external).collect();
        let internal: Vec<_> = all_links.iter().filter(|l| !l.is_external).collect();
        assert_eq!(external.len(), 1);
        assert_eq!(internal.len(), 1);
    }
}
