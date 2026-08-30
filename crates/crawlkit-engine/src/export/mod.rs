pub mod csv;
mod helpers;
pub mod html;
pub mod json;
pub mod markdown;

pub use csv::{export_csv, CsvColumnSelector};
pub use helpers::ExportError;
pub use html::export_html;
pub use json::{export_json, JsonCrawlMeta, JsonExport, JsonIssue, JsonPage, JSON_SCHEMA_VERSION};
pub use markdown::export_markdown;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{Issue, IssueCategory, PageData, Severity, Storage};
    use chrono::Utc;
    use url::Url;

    fn seed_data(storage: &Storage, crawl_id: &str) {
        let pages = vec![
            PageData {
                id: "p1".into(),
                url: Url::parse("https://example.com/").unwrap(),
                final_url: Url::parse("https://example.com/").unwrap(),
                status_code: 200,
                title: Some("Home".into()),
                description: Some("Home page".into()),
                canonical_url: Some(Url::parse("https://example.com/").unwrap()),
                word_count: Some(1200),
                load_time_ms: Some(150),
                body_size: Some(4096),
                fetched_at: Utc::now(),
                links: vec![Url::parse("https://example.com/about").unwrap()],
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
                extractions: None,
            },
            PageData {
                id: "p2".into(),
                url: Url::parse("https://example.com/about").unwrap(),
                final_url: Url::parse("https://example.com/about").unwrap(),
                status_code: 404,
                title: None,
                description: None,
                canonical_url: None,
                word_count: Some(50),
                load_time_ms: Some(80),
                body_size: Some(512),
                fetched_at: Utc::now(),
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
                extractions: None,
            },
            PageData {
                id: "p3".into(),
                url: Url::parse("https://example.com/blog").unwrap(),
                final_url: Url::parse("https://example.com/blog").unwrap(),
                status_code: 200,
                title: Some("Blog".into()),
                description: Some("Blog listing".into()),
                canonical_url: None,
                word_count: Some(3000),
                load_time_ms: Some(300),
                body_size: Some(8192),
                fetched_at: Utc::now(),
                links: vec![
                    Url::parse("https://example.com/").unwrap(),
                    Url::parse("https://external.com/other").unwrap(),
                ],
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
                extractions: None,
            },
        ];
        storage.insert_pages(crawl_id, &pages).unwrap();

        let issues = vec![
            Issue {
                id: "i1".into(),
                page_id: "p1".into(),
                category: IssueCategory::Seo,
                severity: Severity::Error,
                code: "SEO001".into(),
                title: "Missing meta description".into(),
                description: "Page has no meta description".into(),
                element: Some("meta[name=description]".into()),
                recommendation: "Add a meta description".into(),
                tenant_id: None,
            },
            Issue {
                id: "i2".into(),
                page_id: "p2".into(),
                category: IssueCategory::Http,
                severity: Severity::Critical,
                code: "HTTP001".into(),
                title: "404 Not Found".into(),
                description: "Page returns 404".into(),
                element: None,
                recommendation: "Fix or remove the page".into(),
                tenant_id: None,
            },
            Issue {
                id: "i3".into(),
                page_id: "p1".into(),
                category: IssueCategory::Images,
                severity: Severity::Warning,
                code: "IMG001".into(),
                title: "Image missing alt text".into(),
                description: "An image has no alt attribute".into(),
                element: Some("img.logo".into()),
                recommendation: "Add descriptive alt text".into(),
                tenant_id: None,
            },
            Issue {
                id: "i4".into(),
                page_id: "p3".into(),
                category: IssueCategory::Seo,
                severity: Severity::Warning,
                code: "SEO002".into(),
                title: "No canonical tag".into(),
                description: "Page is missing a canonical URL".into(),
                element: None,
                recommendation: "Add a canonical link tag".into(),
                tenant_id: None,
            },
            Issue {
                id: "i5".into(),
                page_id: "p3".into(),
                category: IssueCategory::Performance,
                severity: Severity::Info,
                code: "PERF001".into(),
                title: "Slow load time".into(),
                description: "Page took over 200ms to load".into(),
                element: None,
                recommendation: "Optimize page load speed".into(),
                tenant_id: None,
            },
        ];
        storage.insert_issues(&issues).unwrap();
        storage.finish_crawl(crawl_id, 3, 5).unwrap();
    }

    // --- CSV tests ---

    #[test]
    fn test_csv_all_columns() {
        let storage = Storage::new_in_memory().unwrap();
        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();
        seed_data(&storage, &crawl_id);

        let selector = CsvColumnSelector::all();
        let csv_bytes = export_csv(&storage, &crawl_id, &selector).unwrap();
        let csv_str = String::from_utf8(csv_bytes).unwrap();

        assert!(csv_str.contains("url,status_code,title"));
        assert!(csv_str.contains("https://example.com/"));
        assert!(csv_str.contains("https://example.com/about"));
        assert!(csv_str.contains("https://example.com/blog"));
    }

    #[test]
    fn test_csv_subset_columns() {
        let storage = Storage::new_in_memory().unwrap();
        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();
        seed_data(&storage, &crawl_id);

        let selector = CsvColumnSelector {
            url: true,
            status_code: true,
            issue_count: true,
            ..Default::default()
        };
        let csv_bytes = export_csv(&storage, &crawl_id, &selector).unwrap();
        let csv_str = String::from_utf8(csv_bytes).unwrap();

        let lines: Vec<&str> = csv_str.lines().collect();
        assert!(lines[0].contains("url"));
        assert!(lines[0].contains("status_code"));
        assert!(lines[0].contains("issue_count"));
        assert!(!lines[0].contains("title"));
    }

    #[test]
    fn test_csv_nested_json_escaping() {
        let storage = Storage::new_in_memory().unwrap();
        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();
        seed_data(&storage, &crawl_id);

        let selector = CsvColumnSelector {
            issues_json: true,
            ..Default::default()
        };
        let csv_bytes = export_csv(&storage, &crawl_id, &selector).unwrap();
        let csv_str = String::from_utf8(csv_bytes).unwrap();

        // p1 has 2 issues, p3 has 2 issues, p2 has 1 issue
        assert!(csv_str.contains("Missing meta description"));
    }

    #[test]
    fn test_csv_empty_crawl() {
        let storage = Storage::new_in_memory().unwrap();
        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();

        let selector = CsvColumnSelector::all();
        let csv_bytes = export_csv(&storage, &crawl_id, &selector).unwrap();
        let csv_str = String::from_utf8(csv_bytes).unwrap();

        let lines: Vec<&str> = csv_str.lines().collect();
        assert_eq!(lines.len(), 1); // header only
    }

    // --- JSON tests ---

    #[test]
    fn test_json_pretty() {
        let storage = Storage::new_in_memory().unwrap();
        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();
        seed_data(&storage, &crawl_id);

        let json_str = export_json(&storage, &crawl_id, true).unwrap();

        assert!(json_str.contains("schema_version"));
        assert!(json_str.contains(JSON_SCHEMA_VERSION));
        assert!(json_str.contains("https://example.com/"));
        // Pretty-printed has newlines
        assert!(json_str.contains('\n'));
    }

    #[test]
    fn test_json_compact() {
        let storage = Storage::new_in_memory().unwrap();
        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();
        seed_data(&storage, &crawl_id);

        let json_str = export_json(&storage, &crawl_id, false).unwrap();

        // Compact has no extra whitespace
        assert!(!json_str.contains('\n'));
        let v: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(v["schema_version"], JSON_SCHEMA_VERSION);
        assert_eq!(v["crawl"]["pages_crawled"], 3);
    }

    #[test]
    fn test_json_page_issues_and_links() {
        let storage = Storage::new_in_memory().unwrap();
        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();
        seed_data(&storage, &crawl_id);

        let json_str = export_json(&storage, &crawl_id, true).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        let pages = v["pages"].as_array().unwrap();
        let home = pages
            .iter()
            .find(|p| p["url"] == "https://example.com/")
            .unwrap();
        assert_eq!(home["issues"].as_array().unwrap().len(), 2);
        assert_eq!(home["links"].as_array().unwrap().len(), 1);
    }

    // --- Canonical ordering tests ---

    #[test]
    fn test_export_pages_and_issues_sorted() {
        let storage = Storage::new_in_memory().unwrap();
        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();
        seed_data(&storage, &crawl_id);

        let json_str = export_json(&storage, &crawl_id, false).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        let pages = v["pages"].as_array().unwrap();

        let urls: Vec<&str> = pages.iter().map(|p| p["url"].as_str().unwrap()).collect();
        let mut sorted_urls = urls.clone();
        sorted_urls.sort();
        assert_eq!(urls, sorted_urls, "pages must be sorted by URL");

        for page in pages {
            let codes: Vec<&str> = page["issues"]
                .as_array()
                .map(|a| a.iter().map(|i| i["code"].as_str().unwrap()).collect())
                .unwrap_or_default();
            let mut sorted_codes = codes.clone();
            sorted_codes.sort();
            assert_eq!(codes, sorted_codes, "issues must be sorted by code");
        }
    }

    #[test]
    fn test_export_byte_identical_across_calls() {
        let storage = Storage::new_in_memory().unwrap();
        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();
        seed_data(&storage, &crawl_id);

        let json1 = export_json(&storage, &crawl_id, false).unwrap();
        let json2 = export_json(&storage, &crawl_id, false).unwrap();
        assert_eq!(json1, json2);

        let md1 = export_markdown(&storage, &crawl_id).unwrap();
        let md2 = export_markdown(&storage, &crawl_id).unwrap();
        assert_eq!(md1, md2);

        let html1 = export_html(&storage, &crawl_id).unwrap();
        let html2 = export_html(&storage, &crawl_id).unwrap();
        assert_eq!(html1, html2);

        let selector = CsvColumnSelector::all();
        let csv1 = export_csv(&storage, &crawl_id, &selector).unwrap();
        let csv2 = export_csv(&storage, &crawl_id, &selector).unwrap();
        assert_eq!(csv1, csv2);
    }

    // --- Markdown tests ---

    #[test]
    fn test_markdown_structure() {
        let storage = Storage::new_in_memory().unwrap();
        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();
        seed_data(&storage, &crawl_id);

        let md = export_markdown(&storage, &crawl_id).unwrap();

        assert!(md.contains("# Crawl Report"));
        assert!(md.contains("## Summary"));
        assert!(md.contains("## Issues by Severity"));
        assert!(md.contains("## Issues by Category"));
        assert!(md.contains("## Top Issues"));
        assert!(md.contains("3")); // total pages
        assert!(md.contains("5")); // total issues
    }

    #[test]
    fn test_markdown_empty_crawl() {
        let storage = Storage::new_in_memory().unwrap();
        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();

        let md = export_markdown(&storage, &crawl_id).unwrap();

        assert!(md.contains("| Pages crawled | 0 |"));
        assert!(md.contains("| Total issues | 0 |"));
    }

    // --- HTML tests ---

    #[test]
    fn test_html_structure() {
        let storage = Storage::new_in_memory().unwrap();
        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();
        seed_data(&storage, &crawl_id);

        let html = export_html(&storage, &crawl_id).unwrap();

        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("Crawl Report"));
        assert!(html.contains("status-error"));
        assert!(html.contains("https://example.com/"));
        assert!(html.contains("filterTable()"));
    }

    #[test]
    fn test_html_empty_crawl() {
        let storage = Storage::new_in_memory().unwrap();
        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();

        let html = export_html(&storage, &crawl_id).unwrap();

        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("0")); // zero counts
    }

    // --- Helper access tests ---

    #[test]
    fn test_column_selector_headers() {
        let sel = CsvColumnSelector::all();
        assert_eq!(sel.headers().len(), 22);

        let sel = CsvColumnSelector {
            url: true,
            status_code: true,
            ..Default::default()
        };
        assert_eq!(sel.headers(), vec!["url", "status_code"]);
    }

    // --- Escaping tests ---

    #[test]
    fn test_escape_html_escapes_dangerous_characters() {
        assert_eq!(helpers::escape_html("&<>\"'"), "&amp;&lt;&gt;&quot;&#39;");
        assert_eq!(helpers::escape_html("plain text"), "plain text");
        assert_eq!(
            helpers::escape_html("<script>alert(1)</script>"),
            "&lt;script&gt;alert(1)&lt;/script&gt;"
        );
    }

    #[test]
    fn test_escape_markdown_escapes_structural_characters() {
        assert_eq!(
            helpers::escape_markdown("[link](`code`) | cell"),
            "\\[link\\](\\`code\\`) \\| cell"
        );
        assert_eq!(helpers::escape_markdown("# heading"), "\\# heading");
        assert_eq!(helpers::escape_markdown("- item"), "\\- item");
        // Mid-text markers that cannot start a structure stay readable.
        assert_eq!(
            helpers::escape_markdown("mid # hash - dash"),
            "mid # hash - dash"
        );
    }

    #[test]
    fn test_html_escapes_hostile_target_url_and_finding_text() {
        let storage = Storage::new_in_memory().unwrap();
        let target = "\"><script>alert(3)</script>";
        let crawl_id = storage.start_crawl(target, None).unwrap();

        let page = PageData {
            id: "p1".into(),
            url: Url::parse("https://example.com/").unwrap(),
            final_url: Url::parse("https://example.com/").unwrap(),
            status_code: 200,
            title: Some("T".into()),
            description: None,
            canonical_url: None,
            word_count: None,
            load_time_ms: None,
            body_size: None,
            fetched_at: Utc::now(),
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
            extractions: None,
        };
        storage.insert_pages(&crawl_id, &[page]).unwrap();

        let issues = vec![Issue {
            id: "i1".into(),
            page_id: "p1".into(),
            category: IssueCategory::Seo,
            severity: Severity::Error,
            code: "SEO\"><script>".into(),
            title: "<script>alert(4)</script>".into(),
            description: "desc".into(),
            element: None,
            recommendation: "rec".into(),
            tenant_id: None,
        }];
        storage.insert_issues(&issues).unwrap();
        storage.finish_crawl(&crawl_id, 1, 1).unwrap();

        let html = export_html(&storage, &crawl_id).unwrap();

        assert!(!html.contains("<script>alert"), "raw script tag leaked");
        assert!(html.contains("&quot;&gt;&lt;script&gt;alert(3)&lt;/script&gt;"));
        assert!(html.contains("&lt;script&gt;alert(4)&lt;/script&gt;"));
        assert!(html.contains("SEO&quot;&gt;&lt;script&gt;"));
    }

    #[test]
    fn test_markdown_escapes_hostile_finding_text() {
        let storage = Storage::new_in_memory().unwrap();
        let crawl_id = storage.start_crawl("https://example.com", None).unwrap();

        let page = PageData {
            id: "p1".into(),
            url: Url::parse("https://example.com/").unwrap(),
            final_url: Url::parse("https://example.com/").unwrap(),
            status_code: 200,
            title: Some("T".into()),
            description: None,
            canonical_url: None,
            word_count: None,
            load_time_ms: None,
            body_size: None,
            fetched_at: Utc::now(),
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
            extractions: None,
        };
        storage.insert_pages(&crawl_id, &[page]).unwrap();

        let issues = vec![Issue {
            id: "i1".into(),
            page_id: "p1".into(),
            category: IssueCategory::Seo,
            severity: Severity::Error,
            code: "SEO001".into(),
            title: "[inject](https://evil)`|`".into(),
            description: "desc".into(),
            element: None,
            recommendation: "rec".into(),
            tenant_id: None,
        }];
        storage.insert_issues(&issues).unwrap();
        storage.finish_crawl(&crawl_id, 1, 1).unwrap();

        let md = export_markdown(&storage, &crawl_id).unwrap();

        // Brackets, code spans, and pipes must not render as structures.
        assert!(!md.contains("[inject]"));
        assert!(!md.contains("`|`"));
        assert!(md.contains("\\[inject\\]"));
        assert!(md.contains("\\`\\|\\`"));
    }
}
