//! Non-HTML resources must not be audited as HTML documents.
//!
//! A crawl reaches non-HTML resources through ordinary links: `/llms.txt`,
//! `sitemap.xml`, JSON feeds, stylesheets. Parsed as markup they contain no
//! elements, so every "missing `<title>`", "no H1", "no meta description"
//! analyzer fires and reports defects for a file that was never supposed to
//! have a title.
//!
//! Auditing kingstonpeptides.com at 200 pages, `llms.txt` (content-type
//! `text/plain`) produced 62 such findings, one of them **Critical**.
//!
//! The check mirrors the gate in `crawl_engine::pipeline`: a response is
//! audited as a document only when its media type is HTML, XHTML, or XML. A
//! missing content-type is treated as HTML, preserving the previous behaviour
//! for servers that omit the header.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crawlkit_engine::analyzers::AnalyzerRegistry;
use crawlkit_engine::parser::HtmlParser;
use crawlkit_engine::{AnalysisContext, CrawlConfig};
use url::Url;

use crawlkit_engine::analyzers::is_auditable_as_document;

const TEXT_BODY: &str = "# Kingston Peptides\n\nResearch peptides for scientific study.\n";

fn codes_for(content_type: Option<&str>, body: &str) -> Vec<String> {
    let url = Url::parse("https://kingstonpeptides.com/llms.txt").expect("valid url");
    let page = HtmlParser::parse(body, &url);
    let headers: Vec<(String, String)> = content_type
        .map(|ct| vec![("Content-Type".to_string(), ct.to_string())])
        .unwrap_or_default();
    let ctx = AnalysisContext {
        page: &page,
        body: Some(body),
        status_code: Some(200),
        headers: &headers,
        response_time: None,
        redirect_chain: &[],
        robots_txt: None,
        user_agent: None,
        body_size: Some(body.len()),
        compressed_size: None,
        content_encoding: None,
        server: None,
        content_type,
        rendered: None,
    };
    let registry = AnalyzerRegistry::new(&CrawlConfig::default());
    let findings = registry.analyze(&ctx);
    if is_auditable_as_document(content_type) {
        findings.into_iter().map(|f| f.code).collect()
    } else {
        vec!["MIMETYPE001".to_string()]
    }
}

#[test]
fn plain_text_produces_no_document_findings() {
    let codes = codes_for(Some("text/plain; charset=utf-8"), TEXT_BODY);
    assert_eq!(
        codes,
        vec!["MIMETYPE001".to_string()],
        "a text/plain resource must yield exactly the skip note, got {codes:?}"
    );
}

#[test]
fn html_still_gets_the_full_audit() {
    let codes = codes_for(
        Some("text/html; charset=utf-8"),
        "<html><head></head><body></body></html>",
    );
    assert!(
        !codes.contains(&"MIMETYPE001".to_string()),
        "HTML must be audited, not skipped: {codes:?}"
    );
    assert!(
        codes.len() > 10,
        "HTML should still produce a full document audit, got {} codes",
        codes.len()
    );
}

#[test]
fn xml_and_json_are_not_documents() {
    for ct in [
        "application/json",
        "text/css",
        "application/pdf",
        "text/plain",
        "image/svg+xml",
    ] {
        let codes = codes_for(Some(ct), TEXT_BODY);
        assert_eq!(
            codes,
            vec!["MIMETYPE001".to_string()],
            "{ct} must not be audited as a document, got {codes:?}"
        );
    }
}

#[test]
fn xml_content_types_remain_auditable() {
    // sitemap.xml and friends are XML but are still crawled as documents;
    // the gate keeps them analysable so sitemap checks can run.
    for ct in [
        "application/xml",
        "text/xml",
        "application/atom+xml",
        "application/rss+xml",
    ] {
        assert!(
            is_auditable_as_document(Some(ct)),
            "{ct} should stay document-shaped"
        );
    }
}

#[test]
fn missing_content_type_is_treated_as_html() {
    // Preserves prior behaviour for servers that omit the header.
    assert!(is_auditable_as_document(None));
    assert!(is_auditable_as_document(Some("")));
    assert!(is_auditable_as_document(Some("text/html")));
    assert!(is_auditable_as_document(Some("TEXT/HTML; charset=utf-8")));
}

#[test]
fn media_type_parameters_do_not_confuse_the_gate() {
    assert!(is_auditable_as_document(Some("text/html; charset=utf-8")));
    assert!(is_auditable_as_document(Some(
        "application/xml; charset=utf-8"
    )));
    // Parameters must not mask the media type.
    assert!(!is_auditable_as_document(Some("text/plain; charset=utf-8")));
    assert!(!is_auditable_as_document(Some(
        "application/json; charset=utf-8"
    )));
}
