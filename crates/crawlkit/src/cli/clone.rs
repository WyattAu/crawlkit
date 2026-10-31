//! `crawlkit clone` — offline site mirroring (GAP G8).
//!
//! Fetches pages and same-origin assets through the shared polite fetch loop
//! ([`super::site_fetch`]) and writes them under `--out` with relative-link
//! rewriting, so the mirror browses offline. Same-origin only by default:
//! off-origin links are left as absolute URLs, which keeps the mirror honest
//! about what it did not copy.
//!
//! The URL→path mapping mirrors the site's own path layout (one namespace for
//! pages and assets), so a rewritten link is a plain relative path and no
//! per-kind registry is needed.

use anyhow::{Context, Result};
use std::path::{Component, Path, PathBuf};
use std::time::Duration;
use url::Url;

use super::site_fetch::{fetch_site, SiteFetchOptions};

/// Arguments for `crawlkit clone`.
pub(crate) struct CloneParams {
    /// Site to mirror.
    pub url: String,
    /// Output directory.
    pub out: PathBuf,
    /// Page budget (pages and assets together).
    pub max_pages: Option<usize>,
    /// Delay between requests in milliseconds.
    pub delay: Option<u64>,
}

/// Run the clone: fetch politely, write the mirror, rewrite links.
pub async fn run(params: CloneParams) -> Result<()> {
    let start = Url::parse(&params.url).with_context(|| format!("invalid URL: {}", params.url))?;
    std::fs::create_dir_all(&params.out)
        .with_context(|| format!("creating output directory {}", params.out.display()))?;

    let opts = SiteFetchOptions {
        start,
        max_pages: params.max_pages.unwrap_or(50),
        request_delay: Duration::from_millis(params.delay.unwrap_or(500)),
        ..SiteFetchOptions::defaults()
    };

    let (pages, stats) = fetch_site(&opts).await?;
    for page in &pages {
        let page_path = local_path_for_url(&page.final_url, &params.out);
        if let Some(parent) = page_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        let body = if page.is_html {
            rewrite_html(&page.body, &page.final_url, &page_path, &params.out)
        } else {
            page.body.clone()
        };
        std::fs::write(&page_path, body.as_bytes())
            .with_context(|| format!("writing {}", page_path.display()))?;
    }

    let html_pages = pages.iter().filter(|p| p.is_html).count();
    println!(
        "Cloned {} resources to {} ({} html pages with rewritten links, {} failed, {} off-origin links left absolute).",
        stats.fetched,
        params.out.display(),
        html_pages,
        stats.failed,
        stats.skipped_external,
    );
    Ok(())
}

/// Map a URL to its mirror path under `root`.
///
/// `https://site/a/b` → `root/a/b`; directory-shaped URLs (trailing slash, or
/// a final segment without a file extension) become `…/index.html`, and the
/// empty path is the site root's `index.html`. Segments are sanitized: `..`
/// and `.` are dropped and filesystem-hostile characters are replaced, so a
/// crafted URL cannot escape `root`.
pub(crate) fn local_path_for_url(url: &Url, root: &Path) -> PathBuf {
    let mut path = root.to_path_buf();
    let raw = url.path();
    let segments: Vec<&str> = raw.split('/').filter(|s| !s.is_empty()).collect();
    if segments.is_empty() {
        return path.join("index.html");
    }
    for segment in &segments {
        if *segment == ".." || *segment == "." {
            continue;
        }
        path.push(sanitize_segment(segment));
    }
    let directory_shaped = raw.ends_with('/')
        || segments
            .last()
            .is_some_and(|s| !s.contains('.') && !s.is_empty());
    if directory_shaped {
        path.push("index.html");
    }
    path
}

/// Make one path segment safe for a filesystem name.
fn sanitize_segment(segment: &str) -> String {
    let cleaned: String = segment
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        "_".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Relative link from the page at `from` to the resource at `to`.
///
/// Both paths are mirror-relative. The result uses `/` separators and, when
/// it climbs, `../` prefixes, so a browser resolves it from the page's
/// directory exactly as the original resolved from the page's URL.
pub(crate) fn relative_path(from: &Path, to: &Path) -> String {
    let from_parts = path_parts(from);
    let to_parts = path_parts(to);
    let from_dirs = from_parts.len().saturating_sub(1);
    let common = from_dirs.min(to_parts.len()).min(
        from_parts
            .iter()
            .zip(to_parts.iter())
            .take(from_dirs)
            .take_while(|(a, b)| a == b)
            .count(),
    );
    let mut parts: Vec<&str> = vec![];
    parts.extend(std::iter::repeat_n("..", from_dirs - common));
    parts.extend(to_parts[common..].iter().copied());
    if parts.is_empty() {
        return "./".to_string();
    }
    parts.join("/")
}

fn path_parts(path: &Path) -> Vec<&str> {
    path.components()
        .filter_map(|c| match c {
            Component::Normal(s) => Some(s.to_str().unwrap_or("")),
            _ => None,
        })
        .collect()
}

/// Rewrite one `href`/`src` attribute value to a mirror-relative path.
///
/// Returns `None` when the value must stay as-is: empty, fragment-only
/// (`#top`), unparseable, off-origin (off-origin links stay absolute so the
/// mirror never pretends to have copied them), and non-navigation schemes
/// (`mailto:`, `tel:`, `data:`, `javascript:`). A fragment on an in-mirror
/// target is preserved.
pub(crate) fn rewrite_link(
    page_url: &Url,
    page_path: &Path,
    target: &str,
    root: &Path,
) -> Option<String> {
    let trimmed = target.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
        return None;
    }
    if let Some(scheme_end) = trimmed.find(':') {
        let scheme = &trimmed[..scheme_end];
        let looks_like_scheme = !scheme.is_empty()
            && scheme
                .chars()
                .next()
                .is_some_and(|c: char| c.is_ascii_alphabetic())
            && scheme
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'));
        if looks_like_scheme && scheme != "http" && scheme != "https" {
            return None;
        }
    }
    let resolved = page_url.join(trimmed).ok()?;
    if !super::site_fetch::is_same_origin(page_url, &resolved) {
        return None;
    }
    let target_path = local_path_for_url(&resolved, root);
    let mut rewritten = relative_path(page_path, &target_path);
    if let Some(fragment) = resolved.fragment() {
        rewritten.push('#');
        rewritten.push_str(fragment);
    }
    Some(rewritten)
}

/// Rewrite every same-origin `href`/`src` value in an HTML document.
///
/// A deliberately small attribute scanner (the same approach the analyzers
/// use for generator metas): quoted attribute values after `href=` / `src=`,
/// case-insensitive names and values, with a guard so `data-href=` is not
/// misread. Unquoted values are left alone — a wrong rewrite is worse than
/// an untouched link.
pub(crate) fn rewrite_html(html: &str, page_url: &Url, page_path: &Path, root: &Path) -> String {
    const ATTRS: [&str; 2] = ["href=", "src="];
    let lower = html.to_ascii_lowercase();
    let mut out = String::with_capacity(html.len());
    let mut cursor = 0usize;
    loop {
        // Earliest quoted href=/src= at or after `cursor`.
        let next = ATTRS
            .iter()
            .filter_map(|attr| {
                let mut from = cursor;
                while let Some(rel) = lower[from..].find(attr) {
                    let at = from + rel;
                    let boundary_ok = at == 0
                        || !lower
                            .as_bytes()
                            .get(at - 1)
                            .is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'-' || *b == b'_');
                    let value_at = at + attr.len();
                    let quoted = matches!(lower.as_bytes().get(value_at), Some(b'"') | Some(b'\''));
                    if boundary_ok && quoted {
                        return Some((at, value_at));
                    }
                    from = at + attr.len();
                }
                None
            })
            .min_by_key(|(at, _)| *at);
        let Some((_at, value_start)) = next else {
            break;
        };
        let quote = lower.as_bytes()[value_start] as char;
        let Some(value_end_rel) = lower[value_start + 1..].find(quote) else {
            // Unterminated attribute: copy the rest verbatim.
            out.push_str(&html[cursor..]);
            return out;
        };
        let value_end = value_start + 1 + value_end_rel;
        out.push_str(&html[cursor..value_start + 1]);
        let original = &html[value_start + 1..value_end];
        match rewrite_link(page_url, page_path, original, root) {
            Some(rewritten) => out.push_str(&rewritten),
            None => out.push_str(original),
        }
        cursor = value_end;
    }
    out.push_str(&html[cursor..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn site_url(path: &str) -> Url {
        Url::parse(&format!("https://site.test{path}")).unwrap()
    }

    // ---- URL → mirror path ---------------------------------------------

    #[test]
    fn root_url_maps_to_index_html() {
        let root = Path::new("out");
        assert_eq!(
            local_path_for_url(&site_url("/"), root),
            Path::new("out/index.html")
        );
        assert_eq!(
            local_path_for_url(&site_url(""), root),
            Path::new("out/index.html")
        );
    }

    #[test]
    fn directory_urls_get_index_html_extensionless_paths_do_not() {
        let root = Path::new("out");
        assert_eq!(
            local_path_for_url(&site_url("/about/"), root),
            Path::new("out/about/index.html")
        );
        assert_eq!(
            local_path_for_url(&site_url("/about"), root),
            Path::new("out/about/index.html"),
            "extensionless final segment is a page, not a file"
        );
        assert_eq!(
            local_path_for_url(&site_url("/blog/post-1.html"), root),
            Path::new("out/blog/post-1.html")
        );
        assert_eq!(
            local_path_for_url(&site_url("/static/logo.png"), root),
            Path::new("out/static/logo.png")
        );
    }

    #[test]
    fn traversal_segments_are_dropped_not_resolved() {
        let root = Path::new("out");
        let path = local_path_for_url(&site_url("/a/../../etc/passwd"), root);
        assert!(!path.components().any(|c| c == Component::ParentDir));
        assert!(path.starts_with("out"));
    }

    // ---- relative path computation --------------------------------------

    #[test]
    fn sibling_files_reference_each_other_directly() {
        assert_eq!(
            relative_path(Path::new("a/index.html"), Path::new("a/b.html")),
            "b.html"
        );
    }

    #[test]
    fn nested_page_reaches_site_root_with_dots() {
        assert_eq!(
            relative_path(Path::new("blog/2024/post.html"), Path::new("index.html")),
            "../../index.html"
        );
    }

    #[test]
    fn root_page_to_nested_and_cross_branch_targets() {
        assert_eq!(
            relative_path(Path::new("index.html"), Path::new("css/site.css")),
            "css/site.css"
        );
        assert_eq!(
            relative_path(Path::new("a/index.html"), Path::new("b/c/page.html")),
            "../b/c/page.html"
        );
    }

    // ---- link rewriting ---------------------------------------------------

    #[test]
    fn same_origin_link_becomes_relative() {
        let page_url = site_url("/blog/post.html");
        let page_path = Path::new("out/blog/post.html");
        let root = Path::new("out");
        assert_eq!(
            rewrite_link(&page_url, page_path, "/about", root),
            Some("../about/index.html".to_string())
        );
        assert_eq!(
            rewrite_link(&page_url, page_path, "../tags/seo/", root),
            Some("../tags/seo/index.html".to_string())
        );
    }

    #[test]
    fn fragment_is_preserved_on_in_mirror_target() {
        let page_url = site_url("/index.html");
        let page_path = Path::new("out/index.html");
        assert_eq!(
            rewrite_link(&page_url, page_path, "/guide#setup", Path::new("out")),
            Some("guide/index.html#setup".to_string())
        );
        // Fragment-only links have no destination to rewrite.
        assert_eq!(
            rewrite_link(&page_url, page_path, "#top", Path::new("out")),
            None
        );
    }

    #[test]
    fn off_origin_and_special_schemes_stay_absolute() {
        let page_url = site_url("/index.html");
        let page_path = Path::new("out/index.html");
        let root = Path::new("out");
        assert_eq!(
            rewrite_link(&page_url, page_path, "https://cdn.other.test/x.png", root),
            None
        );
        assert_eq!(
            rewrite_link(&page_url, page_path, "http://site.test/page", root),
            None,
            "http vs https is a different origin"
        );
        assert_eq!(
            rewrite_link(&page_url, page_path, "mailto:a@b.test", root),
            None
        );
        assert_eq!(
            rewrite_link(&page_url, page_path, "data:image/png;base64,AA==", root),
            None
        );
        assert_eq!(
            rewrite_link(&page_url, page_path, "javascript:void(0)", root),
            None
        );
    }

    // ---- HTML rewriting -----------------------------------------------------

    #[test]
    fn rewrite_html_rewrites_href_and_src_case_insensitively() {
        let page_url = site_url("/index.html");
        let page_path = Path::new("out/index.html");
        let root = Path::new("out");
        let html = r#"<A HREF="/about">About</A><img SRC="/img/logo.png" alt="x"><link href="/css/site.css" rel="stylesheet">"#;
        let out = rewrite_html(html, &page_url, page_path, root);
        assert!(out.contains(r#"HREF="about/index.html""#), "{out}");
        assert!(out.contains(r#"SRC="img/logo.png""#), "{out}");
        assert!(out.contains(r#"href="css/site.css""#), "{out}");
        assert!(
            out.contains(r#"rel="stylesheet""#),
            "non-target attrs untouched: {out}"
        );
    }

    #[test]
    fn rewrite_html_leaves_off_origin_and_anchors_alone() {
        let page_url = site_url("/index.html");
        let page_path = Path::new("out/index.html");
        let root = Path::new("out");
        let html = r##"<a href="https://other.test/x">ext</a><a href="#top">top</a><a href="mailto:a@b.test">mail</a>"##;
        assert_eq!(rewrite_html(html, &page_url, page_path, root), html);
    }

    #[test]
    fn rewrite_html_preserves_surrounding_markup_and_text() {
        let page_url = site_url("/index.html");
        let page_path = Path::new("out/index.html");
        let root = Path::new("out");
        let html = r#"<p>See the <a class="nav" href="/guide">guide</a> &amp; <a href="/about">about</a>.</p>"#;
        let out = rewrite_html(html, &page_url, page_path, root);
        assert!(out.contains("<p>See the "), "{out}");
        assert!(out.contains("&amp;"), "{out}");
        assert!(out.contains(r#"href="guide/index.html""#), "{out}");
        assert!(out.contains(r#"href="about/index.html""#), "{out}");
        assert!(out.ends_with("</p>"), "{out}");
    }

    #[test]
    fn rewrite_html_survives_unterminated_tag() {
        let page_url = site_url("/index.html");
        let page_path = Path::new("out/index.html");
        let root = Path::new("out");
        let html = r#"<a href="/about">truncated"#;
        let out = rewrite_html(html, &page_url, page_path, root);
        assert!(out.contains(r#"href="about/index.html""#), "{out}");
        assert!(out.contains("truncated"), "{out}");
    }

    #[test]
    fn rewrite_html_ignores_data_href_and_unquoted_values() {
        let page_url = site_url("/index.html");
        let page_path = Path::new("out/index.html");
        let root = Path::new("out");
        let html = r#"<div data-href="/not-a-link"><a href=/about/unquoted>plain</a>"#;
        assert_eq!(rewrite_html(html, &page_url, page_path, root), html);
    }
}
