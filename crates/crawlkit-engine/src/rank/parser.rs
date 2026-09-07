//! DuckDuckGo SERP HTML parser.
//!
//! The `html.duckduckgo.com/html/` endpoint returns server-rendered
//! HTML without JavaScript requirements. Organic results live in
//! `div.result` containers; each contains an anchor `a.result__a`
//! whose `href` is either a direct URL or a DuckDuckGo redirect
//! (`//duckduckgo.com/l/?uddg=<urlencoded>&rut=...`).

use crate::rank::OrganicResult;
use scraper::{Html, Selector};

/// Parse DuckDuckGo HTML SERP output into organic results.
///
/// Ad results (`div.result--ad`) are skipped, as are results without a
/// usable link. Positions are assigned by 1-based index over the
/// surviving organic results.
#[must_use]
pub fn parse_ddg_results(html: &str) -> Vec<OrganicResult> {
    let document = Html::parse_document(html);
    let result_sel = match Selector::parse("div.result") {
        Ok(sel) => sel,
        Err(_) => return Vec::new(),
    };
    let ad_sel = match Selector::parse("div.result--ad") {
        Ok(sel) => sel,
        Err(_) => return Vec::new(),
    };
    let link_sel = match Selector::parse("a.result__a") {
        Ok(sel) => sel,
        Err(_) => return Vec::new(),
    };

    let mut results = Vec::new();
    for element in document.select(&result_sel) {
        // Skip sponsored placements — they are not organic positions.
        // Ad containers carry both `result` and `result--ad` classes.
        if ad_sel.matches(&element) {
            continue;
        }
        let Some(anchor) = element.select(&link_sel).next() else {
            continue;
        };
        let Some(href) = anchor.value().attr("href") else {
            continue;
        };
        let Some(url) = decode_ddg_redirect(href) else {
            continue;
        };
        let title = anchor
            .text()
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        if title.is_empty() {
            continue;
        }
        let position = u16::try_from(results.len() + 1).unwrap_or(u16::MAX);
        results.push(OrganicResult {
            position,
            url,
            title,
        });
    }
    results
}

/// Decode a DuckDuckGo result href into the real destination URL.
///
/// Handles three shapes:
/// - direct URLs (`https://example.com/`)
/// - protocol-relative redirects (`//duckduckgo.com/l/?uddg=...`)
/// - scheme-relative with host (`https://duckduckgo.com/l/?uddg=...`)
///
/// Returns `None` for redirect links without a decodable `uddg`
/// parameter or for empty hrefs.
#[must_use]
pub fn decode_ddg_redirect(href: &str) -> Option<String> {
    let href = href.trim();
    if href.is_empty() {
        return None;
    }

    let absolute = if let Some(rest) = href.strip_prefix("//") {
        format!("https://{rest}")
    } else {
        href.to_string()
    };

    if let Ok(url) = url::Url::parse(&absolute) {
        if url
            .host_str()
            .is_some_and(|h| h.ends_with("duckduckgo.com"))
        {
            let uddg = url
                .query_pairs()
                .find(|(k, _)| k == "uddg")
                .map(|(_, v)| v.into_owned())?;
            return normalize_ddg_url(&uddg);
        }
        return normalize_ddg_url(&absolute);
    }

    // Not a parseable URL — treat it as a relative link (unusable).
    None
}

/// Validate and normalize a decoded destination URL.
fn normalize_ddg_url(candidate: &str) -> Option<String> {
    let parsed = url::Url::parse(candidate).ok()?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return None;
    }
    Some(parsed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_DDG_HTML: &str = r#"
    <html><body>
    <div class="result results_links results_links_deep web-result result--ad">
      <div class="links_main links_deep result__body">
        <h2 class="result__title">
          <a rel="nofollow" class="result__a" href="//duckduckgo.com/l/?uddg=https%3A%2F%2Fads.example.com%2F&amp;rut=aaaa">Sponsored Result</a>
        </h2>
      </div>
    </div>
    <div class="result results_links results_links_deep web-result">
      <div class="links_main links_deep result__body">
        <h2 class="result__title">
          <a rel="nofollow" class="result__a" href="//duckduckgo.com/l/?uddg=https%3A%2F%2Fwww.rust-lang.org%2F&amp;rut=bbbb">Rust Programming Language</a>
        </h2>
        <a class="result__snippet" href="//duckduckgo.com/l/?uddg=https%3A%2F%2Fwww.rust-lang.org%2F&amp;rut=bbbb">A language empowering everyone...</a>
      </div>
    </div>
    <div class="result results_links results_links_deep web-result">
      <div class="links_main links_deep result__body">
        <h2 class="result__title">
          <a rel="nofollow" class="result__a" href="https://blog.rust-lang.org/">The Rust Blog</a>
        </h2>
      </div>
    </div>
    <div class="result results_links results_links_deep web-result">
      <div class="links_main links_deep result__body">
        <h2 class="result__title">
          <a rel="nofollow" class="result__a" href="//duckduckgo.com/l/?uddg=https%3A%2F%2Fdocs.rs%2F&amp;rut=cccc">docs.rs</a>
        </h2>
      </div>
    </div>
    <div class="result results_links results_links_deep web-result">
      <div class="links_main links_deep result__body">
        <h2 class="result__title">
          <a rel="nofollow" class="result__a" href=""></a>
        </h2>
      </div>
    </div>
    </body></html>
    "#;

    #[test]
    fn test_parse_ddg_results_basic() {
        let results = parse_ddg_results(SAMPLE_DDG_HTML);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].position, 1);
        assert_eq!(results[0].url, "https://www.rust-lang.org/");
        assert_eq!(results[0].title, "Rust Programming Language");
    }

    #[test]
    fn test_parse_ddg_results_skips_ads() {
        let results = parse_ddg_results(SAMPLE_DDG_HTML);
        assert!(results.iter().all(|r| !r.url.contains("ads.example.com")));
    }

    #[test]
    fn test_parse_ddg_results_positions_are_sequential() {
        let results = parse_ddg_results(SAMPLE_DDG_HTML);
        let positions: Vec<u16> = results.iter().map(|r| r.position).collect();
        assert_eq!(positions, vec![1, 2, 3]);
    }

    #[test]
    fn test_parse_ddg_results_handles_direct_links() {
        let results = parse_ddg_results(SAMPLE_DDG_HTML);
        assert_eq!(results[1].url, "https://blog.rust-lang.org/");
    }

    #[test]
    fn test_parse_ddg_results_empty_input() {
        assert!(parse_ddg_results("").is_empty());
        assert!(parse_ddg_results("<html><body></body></html>").is_empty());
    }

    #[test]
    fn test_decode_ddg_redirect_encoded() {
        let decoded = decode_ddg_redirect(
            "//duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com%2Fpage%3Fa%3D1&rut=abc",
        );
        assert_eq!(decoded.as_deref(), Some("https://example.com/page?a=1"));
    }

    #[test]
    fn test_decode_ddg_redirect_direct_https() {
        let decoded = decode_ddg_redirect("https://example.com/");
        assert_eq!(decoded.as_deref(), Some("https://example.com/"));
    }

    #[test]
    fn test_decode_ddg_redirect_protocol_relative_direct() {
        let decoded = decode_ddg_redirect("//example.com/page");
        assert_eq!(decoded.as_deref(), Some("https://example.com/page"));
    }

    #[test]
    fn test_decode_ddg_redirect_missing_uddg() {
        let decoded = decode_ddg_redirect("//duckduckgo.com/l/?rut=abc");
        assert!(decoded.is_none());
    }

    #[test]
    fn test_decode_ddg_redirect_empty() {
        assert!(decode_ddg_redirect("").is_none());
        assert!(decode_ddg_redirect("   ").is_none());
    }

    #[test]
    fn test_decode_ddg_redirect_rejects_javascript() {
        assert!(decode_ddg_redirect("javascript:void(0)").is_none());
    }

    #[test]
    fn test_parse_ddg_results_normalizes_whitespace_in_titles() {
        let html = r#"
        <div class="result">
          <h2><a class="result__a" href="https://example.com/">
              Multi   line
              title
          </a></h2>
        </div>
        "#;
        let results = parse_ddg_results(html);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Multi line title");
    }
}
