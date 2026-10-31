//! `crawlkit md` — site → Markdown (GAP G8).
//!
//! Fetches pages through the shared polite fetch loop
//! ([`super::site_fetch`]), extracts content with the existing HTML parser
//! plus a small structural converter, and writes one `.md` per page — or,
//! with `--merge`, a single `site.md` in which byte-identical content (the
//! same dedup question the crawl engine's content-hash asks) appears once.
//!
//! The converter is intentionally narrow: headings, paragraphs, and list
//! items. Scripts, styles, and navigation chrome that produces no `<p>` or
//! heading text disappears naturally; what survives is the readable core.

use anyhow::{Context, Result};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use url::Url;

use super::site_fetch::{fetch_site, SiteFetchOptions};
use crawlkit_engine::analyzers::html_text::{decode_entities, strip_tags};
use crawlkit_engine::HtmlParser;

/// Arguments for `crawlkit md`.
pub(crate) struct MdParams {
    /// Site to convert.
    pub url: String,
    /// Output directory.
    pub out: PathBuf,
    /// Merge every page into one `site.md`, dropping duplicate content.
    pub merge: bool,
    /// Page budget.
    pub max_pages: Option<usize>,
    /// Delay between requests in milliseconds.
    pub delay: Option<u64>,
}

/// Run the site→Markdown conversion.
pub async fn run(params: MdParams) -> Result<()> {
    let start = Url::parse(&params.url).with_context(|| format!("invalid URL: {}", params.url))?;
    std::fs::create_dir_all(&params.out)
        .with_context(|| format!("creating output directory {}", params.out.display()))?;

    let opts = SiteFetchOptions {
        start,
        max_pages: params.max_pages.unwrap_or(50),
        request_delay: std::time::Duration::from_millis(params.delay.unwrap_or(500)),
        ..SiteFetchOptions::defaults()
    };
    let (pages, stats) = fetch_site(&opts).await?;

    let mut documents: Vec<(String, String)> = Vec::new();
    for page in &pages {
        if !page.is_html {
            continue;
        }
        let parsed = HtmlParser::parse(&page.body, &page.final_url);
        let markdown = page_to_markdown(&page.final_url, &parsed.meta.title, &page.body);
        if markdown.trim().is_empty() {
            continue;
        }
        let name = document_name(&page.final_url);
        documents.push((name, markdown));
    }

    let written = if params.merge {
        write_merged(&params.out, &documents)?
    } else {
        write_split(&params.out, &documents)?
    };

    println!(
        "Wrote {written} markdown file(s) to {} from {} fetched page(s) ({} failed).",
        params.out.display(),
        stats.fetched,
        stats.failed,
    );
    Ok(())
}

fn write_split(out: &std::path::Path, documents: &[(String, String)]) -> Result<usize> {
    for (name, markdown) in documents {
        let path = out.join(format!("{name}.md"));
        std::fs::write(&path, markdown).with_context(|| format!("writing {}", path.display()))?;
    }
    Ok(documents.len())
}

/// Write one merged `site.md`, keeping the first copy of content seen twice
/// (content-hash dedup, the same question the crawl engine's dedup asks).
fn write_merged(out: &std::path::Path, documents: &[(String, String)]) -> Result<usize> {
    let mut seen: HashSet<u64> = HashSet::new();
    let mut merged = String::new();
    let mut kept = 0usize;
    let mut dropped = 0usize;
    for (_name, markdown) in documents {
        if !seen.insert(content_fingerprint(markdown)) {
            dropped += 1;
            continue;
        }
        merged.push_str("\n\n---\n\n");
        merged.push_str(markdown);
        kept += 1;
    }
    if dropped > 0 {
        merged.push_str(&format!(
            "\n\n<!-- {dropped} duplicate page(s) omitted -->\n"
        ));
    }
    std::fs::write(out.join("site.md"), merged)
        .with_context(|| format!("writing {}", out.join("site.md").display()))?;
    Ok(kept)
}

/// Stable 64-bit content fingerprint for merge dedup.
fn content_fingerprint(markdown: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    markdown.hash(&mut hasher);
    hasher.finish()
}

/// File name (without extension) for a page, derived from its path.
pub(crate) fn document_name(url: &Url) -> String {
    let raw = url.path();
    let segments: Vec<String> = raw
        .split('/')
        .filter(|s| !s.is_empty() && *s != "." && *s != "..")
        .map(|s| {
            s.chars()
                .map(|c| {
                    if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' {
                        c
                    } else {
                        '-'
                    }
                })
                .collect()
        })
        .collect();
    if segments.is_empty() {
        return "index".to_string();
    }
    let joined = segments.join("-");
    let trimmed = joined.trim_end_matches('-');
    let base = trimmed.trim_end_matches(".html").trim_end_matches(".htm");
    if base.is_empty() {
        "index".to_string()
    } else {
        base.to_string()
    }
}

/// Build the Markdown document for one page.
///
/// The parser supplies the title; the converter walks the raw HTML so
/// paragraphs keep their document order relative to headings.
pub(crate) fn page_to_markdown(url: &Url, title: &Option<String>, html: &str) -> String {
    let mut out = String::new();
    let heading = title
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .unwrap_or("Untitled");
    out.push_str("# ");
    out.push_str(&markdown_escape(heading));
    out.push_str("\n\n");
    out.push_str(&format!("*Source: `{url}`*\n"));
    let body = html_to_markdown(html);
    if !body.trim().is_empty() {
        out.push('\n');
        out.push_str(&body);
    }
    out
}

/// Convert a document's visible structure to Markdown.
///
/// Headings keep their level (`<h2>` → `##`), `<p>` becomes a paragraph,
/// `<li>` becomes a `- ` bullet. `<script>`/`<style>` contents are dropped
/// whole. Inline markup inside captured text is flattened through
/// [`strip_tags`] + [`decode_entities`].
pub(crate) fn html_to_markdown(html: &str) -> String {
    let lower = html.to_ascii_lowercase();
    let mut out = String::new();
    let mut cursor = 0usize;
    while let Some(rel) = lower[cursor..].find('<') {
        let at = cursor + rel;
        let tag = &lower[at..];
        // Capture codes: 1–6 heading level, 0 paragraph, 9 list item.
        let capture = HEADINGS
            .iter()
            .find(|(name, _)| tag_is(tag, name))
            .map(|(_, level)| *level)
            .or_else(|| tag_is(tag, "p").then_some(0))
            .or_else(|| tag_is(tag, "li").then_some(9));
        let Some(capture) = capture else {
            if tag_is(tag, "script") || tag_is(tag, "style") {
                // Skip the element wholesale so its content never becomes text.
                cursor = match find_tag_end_offset(&lower[at..], "</script>", "</style>") {
                    Some(end) => at + end,
                    None => html.len(),
                };
                continue;
            }
            cursor = at + 1;
            continue;
        };
        let Some((inner, next)) = capture_inner(html, at, capture) else {
            // Unterminated element: nothing after this can be trusted.
            break;
        };
        let text = flatten(&inner);
        if !text.is_empty() {
            if (1..=6).contains(&capture) {
                out.push_str(&format!("\n{} {}\n\n", "#".repeat(capture as usize), text));
            } else if capture == 9 {
                out.push_str(&format!("- {text}\n"));
            } else {
                out.push_str(&text);
                out.push_str("\n\n");
            }
        }
        cursor = next;
    }
    out
}

/// Heading tags in capture order: tag name → heading level.
const HEADINGS: [(&str, u8); 6] = [
    ("h1", 1),
    ("h2", 2),
    ("h3", 3),
    ("h4", 4),
    ("h5", 5),
    ("h6", 6),
];

/// Whether the (lowercased, at-`<`) text opens tag `name`: the name must be
/// followed by `>`, whitespace, or `/` — so `<li` does not match `<link`.
fn tag_is(tag: &str, name: &str) -> bool {
    let Some(rest) = tag.strip_prefix('<') else {
        return false;
    };
    let Some(rest) = rest.strip_prefix(name) else {
        return false;
    };
    rest.chars()
        .next()
        .is_none_or(|c| c == '>' || c.is_ascii_whitespace() || c == '/')
}

/// Where `</tag>` (either spelling) closes the element opened at `start`.
fn find_tag_end_offset(lower_from_start: &str, script: &str, style: &str) -> Option<usize> {
    let s = lower_from_start.find(script);
    let t = lower_from_start.find(style);
    match (s, t) {
        (Some(a), Some(b)) => {
            let (idx, tag) = if a < b { (a, script) } else { (b, style) };
            Some(idx + tag.len())
        }
        (Some(a), None) => Some(a + script.len()),
        (None, Some(b)) => Some(b + style.len()),
        (None, None) => None,
    }
}

/// Capture the inner HTML of the element opening at `at`.
///
/// `capture` 1–6 captures `h<capture>…</h<capture>>`, 9 captures `li…</li>`,
/// 0 captures `p…</p>`.
fn capture_inner(html: &str, at: usize, capture: u8) -> Option<(String, usize)> {
    let lower = html.to_ascii_lowercase();
    let open_end_rel = lower[at..].find('>')? + 1;
    let close = match capture {
        1..=6 => format!("</h{capture}>"),
        9 => "</li>".to_string(),
        _ => "</p>".to_string(),
    };
    let inner_end_rel = lower[at + open_end_rel..].find(&close)? + at + open_end_rel;
    Some((
        html[at + open_end_rel..inner_end_rel].to_string(),
        inner_end_rel + close.len(),
    ))
}

/// Flatten captured inner HTML to plain text: strip tags, decode entities,
/// collapse the whitespace tag boundaries introduce.
fn flatten(inner: &str) -> String {
    let text = strip_tags(inner);
    let decoded = decode_entities(&text);
    let mut collapsed = String::with_capacity(decoded.len());
    let mut in_space = false;
    for c in decoded.chars() {
        if c.is_whitespace() {
            if !in_space {
                collapsed.push(' ');
            }
            in_space = true;
        } else {
            in_space = false;
            collapsed.push(c);
        }
    }
    collapsed.trim().to_string()
}

/// Escape characters that would turn heading text into markup.
fn markdown_escape(text: &str) -> String {
    text.replace('#', "\\#")
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- markdown extraction --------------------------------------------

    #[test]
    fn headings_render_at_their_level() {
        let md = html_to_markdown("<h1>Title</h1><h2>Section</h2><h3>Sub</h3>");
        assert!(md.contains("# Title\n"), "{md}");
        assert!(md.contains("## Section\n"), "{md}");
        assert!(md.contains("### Sub"), "{md}");
    }

    #[test]
    fn paragraphs_and_entities_survive_flattening() {
        let md = html_to_markdown("<p>First &amp; foremost.</p><p>It&#39;s <b>bold</b> move</p>");
        assert!(md.contains("First & foremost."), "{md}");
        assert!(md.contains("It's bold move"), "{md}");
        // Paragraphs are separated by a blank line.
        assert!(md.contains("foremost.\n\nIt's"), "{md}");
    }

    #[test]
    fn script_and_style_contents_are_excluded() {
        let md = html_to_markdown(
            "<style>p { color: red }</style><script>var x = 1;</script>\
             <p>real content</p>",
        );
        assert!(!md.contains("color: red"), "{md}");
        assert!(!md.contains("var x"), "{md}");
        assert!(md.contains("real content"), "{md}");
    }

    #[test]
    fn list_items_become_bullets() {
        let md = html_to_markdown("<ul><li>alpha</li><li>beta</li></ul>");
        assert!(md.contains("- alpha\n"), "{md}");
        assert!(md.contains("- beta\n"), "{md}");
    }

    #[test]
    fn unterminated_element_does_not_loop_forever() {
        let md = html_to_markdown("<p>truncated without close");
        // Terminates; the truncated paragraph may or may not be captured.
        let _ = md.len();
        let md = html_to_markdown("<h2>no close");
        let _ = md.len();
    }

    #[test]
    fn page_document_includes_title_and_source() {
        let url = Url::parse("https://site.test/docs").unwrap();
        let md = page_to_markdown(&url, &Some("Docs Home".to_string()), "<p>body text</p>");
        assert!(md.starts_with("# Docs Home\n"), "{md}");
        assert!(md.contains("`https://site.test/docs`"), "{md}");
        assert!(md.contains("body text"), "{md}");
    }

    #[test]
    fn empty_title_falls_back_to_untitled() {
        let url = Url::parse("https://site.test/").unwrap();
        let md = page_to_markdown(&url, &None, "<p>x</p>");
        assert!(md.starts_with("# Untitled\n"), "{md}");
    }

    #[test]
    fn heading_hashes_in_text_are_escaped() {
        let url = Url::parse("https://site.test/").unwrap();
        let md = page_to_markdown(&url, &Some("C# and F# notes".to_string()), "");
        assert!(md.starts_with("# C\\# and F\\# notes\n"), "{md}");
    }

    // ---- naming and merge dedup -------------------------------------------

    #[test]
    fn document_names_are_stable_and_collision_free() {
        assert_eq!(
            document_name(&Url::parse("https://s.test/").unwrap()),
            "index"
        );
        assert_eq!(
            document_name(&Url::parse("https://s.test/blog/my-post.html").unwrap()),
            "blog-my-post"
        );
        assert_eq!(
            document_name(&Url::parse("https://s.test/a/b/").unwrap()),
            "a-b"
        );
    }

    #[test]
    fn merge_drops_identical_documents_keeps_first() {
        let tmp = tempfile::tempdir().unwrap();
        let a = ("index".to_string(), "# Same\n\nbody\n".to_string());
        let b = ("other".to_string(), "# Same\n\nbody\n".to_string());
        let c = ("unique".to_string(), "# Different\n".to_string());
        let kept = write_merged(tmp.path(), &[a, b, c]).unwrap();
        assert_eq!(kept, 2, "identical pages collapse to one");
        let merged = std::fs::read_to_string(tmp.path().join("site.md")).unwrap();
        assert_eq!(merged.matches("# Same").count(), 1, "{merged}");
        assert!(merged.contains("# Different"));
        assert!(merged.contains("1 duplicate page(s) omitted"));
    }

    #[test]
    fn link_and_pre_tags_do_not_trigger_paragraph_capture() {
        let md = html_to_markdown(
            r#"<head><link href="/css/a.css" rel="stylesheet"></head><pre>code</pre><p>para</p>"#,
        );
        assert!(
            !md.contains("stylesheet"),
            "<link> must not be captured as list content: {md}"
        );
        assert!(
            !md.contains("- code"),
            "<pre> must not be captured as list content: {md}"
        );
        assert!(md.contains("para"), "{md}");
    }
}
