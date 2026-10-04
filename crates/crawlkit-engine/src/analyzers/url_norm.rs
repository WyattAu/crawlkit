//! Shared URL equivalence helpers for canonical and hreflang comparison.
//!
//! Every canonical/hreflang analyzer needs to answer the same question: "do
//! these two URLs address the same resource?" Historically each analyzer
//! answered it with a raw `==` on either the full string or `Url::path()`.
//! Because `Url::path()` preserves a trailing slash, `/en/` and `/en` — two
//! spellings of one resource that servers and CDNs routinely serve
//! interchangeably — compared unequal, and every such page produced a
//! "canonical does not self-reference" / "missing self-referencing hreflang"
//! finding.
//!
//! These helpers centralize the comparison so the answer is consistent
//! everywhere, and so adding a new canonical analyzer cannot reintroduce the
//! bug by accident.

use url::Url;

/// Normalize a URL *path* for equality comparison.
///
/// `Url::path()` keeps a trailing slash, so `/en/` and `/en` must be folded
/// together. The root path keeps its slash so that `/` never normalizes to the
/// empty string (which would make `https://example.com/` compare equal to a
/// malformed relative reference).
pub fn normalize_path(path: &str) -> &str {
    if path.len() > 1 {
        path.strip_suffix('/').unwrap_or(path)
    } else {
        path
    }
}

/// True when two URL paths address the same resource.
///
/// Treats a trailing slash as insignificant; everything else compares exactly.
pub fn paths_equivalent(a: &str, b: &str) -> bool {
    normalize_path(a) == normalize_path(b)
}

/// True when two absolute URLs address the same resource for SEO purposes.
///
/// Folds together the differences that search engines ignore when resolving a
/// canonical or a hreflang self-reference:
///
/// - a trailing slash on the path (`/en/` == `/en`),
/// - the fragment, which never reaches the server,
/// - scheme and host letter case,
/// - an explicit default port (`:443` for https, `:80` for http),
/// - but *not* the query string — a canonical that drops or reorders query
///   parameters is a real difference worth reporting.
pub fn urls_equivalent(a: &str, b: &str) -> bool {
    match (Url::parse(a), Url::parse(b)) {
        (Ok(x), Ok(y)) => {
            x.scheme().eq_ignore_ascii_case(y.scheme())
                && x.host_str().map(str::to_ascii_lowercase)
                    == y.host_str().map(str::to_ascii_lowercase)
                && x.port_or_known_default() == y.port_or_known_default()
                && paths_equivalent(x.path(), y.path())
                && x.query() == y.query()
        }
        // Fall back to a trailing-slash-insensitive string comparison when
        // either side fails to parse, so a malformed canonical degrades to
        // "different" rather than panicking or silently matching.
        _ => a.trim_end_matches('/') == b.trim_end_matches('/'),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_path_strips_trailing_slash_except_root() {
        assert_eq!(normalize_path("/en/"), "/en");
        assert_eq!(normalize_path("/en"), "/en");
        assert_eq!(normalize_path("/"), "/");
        assert_eq!(normalize_path(""), "");
        assert_eq!(normalize_path("/a/b/"), "/a/b");
    }

    #[test]
    fn paths_equivalent_ignores_trailing_slash() {
        assert!(paths_equivalent("/en/", "/en"));
        assert!(paths_equivalent("/en", "/en/"));
        assert!(!paths_equivalent("/en", "/fr"));
        assert!(paths_equivalent("/", "/"));
    }

    #[test]
    fn urls_equivalent_ignores_trailing_slash() {
        // The exact false positive seen on kingstonpeptides.com.
        assert!(urls_equivalent(
            "https://kingstonpeptides.com/en/",
            "https://kingstonpeptides.com/en"
        ));
        assert!(urls_equivalent(
            "https://example.com/",
            "https://example.com"
        ));
    }

    #[test]
    fn urls_equivalent_ignores_fragment_case_and_default_port() {
        assert!(urls_equivalent(
            "https://example.com/page#section",
            "https://example.com/page"
        ));
        assert!(urls_equivalent(
            "https://EXAMPLE.com/page",
            "https://example.com/page"
        ));
        assert!(urls_equivalent(
            "https://example.com:443/page",
            "https://example.com/page"
        ));
    }

    #[test]
    fn urls_equivalent_still_detects_real_differences() {
        assert!(!urls_equivalent(
            "https://example.com/en/",
            "https://example.com/fr/"
        ));
        assert!(!urls_equivalent(
            "https://example.com/page",
            "http://example.com/page"
        ));
        assert!(!urls_equivalent(
            "https://example.com/page",
            "https://other.com/page"
        ));
        // A canonical that changes the query string is a genuine difference.
        assert!(!urls_equivalent(
            "https://example.com/page",
            "https://example.com/page?utm_source=x"
        ));
    }

    #[test]
    fn urls_equivalent_handles_malformed_input_without_panicking() {
        assert!(!urls_equivalent("not a url", "https://example.com/"));
        // Both sides unparseable: falls back to a trailing-slash comparison.
        assert!(urls_equivalent("relative/path/", "relative/path"));
    }

    #[test]
    fn root_path_does_not_collapse_to_empty() {
        let parsed = Url::parse("https://example.com/").expect("valid url");
        assert_eq!(normalize_path(parsed.path()), "/");
    }
}
