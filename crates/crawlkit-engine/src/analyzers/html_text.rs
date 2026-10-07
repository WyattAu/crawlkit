//! HTML text and attribute extraction, shared by the analyzers that need to read
//! a document the way a browser would rather than the way a regex would like.
//!
//! # Why this is shared rather than per-analyzer
//!
//! Three separate bugs in this file's logic were each found by a different
//! analyzer, and each was silently wrong in the others at the time:
//!
//! - stripping tags to the first `>` leaks attribute values containing `>`
//!   (Starlight ships `style="--x: a > b"`)
//! - matching `name="description"` misses the unquoted `name=description` that
//!   minifiers emit, on exactly the build-optimised sites most likely to be
//!   client-rendered
//! - comparing text without decoding entities reports `Wyatt&#39;s Notes` and
//!   `Wyatt's Notes` as different titles
//!
//! Each was a false positive in production. Centralising them means the fix
//! lands everywhere at once and the tests live in one place.

/// Read an attribute value from a start tag, quoted or not.
///
/// Returns `None` when `name` is not an attribute of this tag. The name must be
/// delimited, so `name` does not match inside `data-name` or `hostname`.
#[must_use]
pub fn attr_value(tag: &str, name: &str) -> Option<String> {
    let bytes = tag.as_bytes();
    let needle = name.as_bytes();
    let mut from = 0usize;
    while let Some(rel) = tag[from..].find(name) {
        let at = from + rel;
        let delimited = at == 0 || matches!(bytes[at - 1], b' ' | b'\t' | b'\n' | b'\r' | b'/');
        let after = at + needle.len();
        if delimited && matches!(bytes.get(after), Some(b'=')) {
            let rest = &tag[after + 1..];
            let value = match rest.chars().next() {
                Some(q @ ('"' | '\'')) => {
                    let body = &rest[q.len_utf8()..];
                    let end = body.find(q)?;
                    &body[..end]
                }
                // An unquoted value ends at whitespace or at the `>` that
                // closes the tag. The caller passes the whole start tag, so the
                // closing `>` is present in `rest` and must not be taken as part
                // of the value -- which is how `type=application/ld+json` read
                // back with a `>` on the end and matched nothing.
                Some(_) => rest.split(|c: char| c == '>' || c.is_whitespace()).next()?,
                None => return None,
            };
            return Some(value.to_string());
        }
        from = at + needle.len();
    }
    None
}

/// Remove tags and decode the entities that affect text comparison.
///
/// `>` only closes a tag when outside a quoted attribute value. Tag boundaries
/// emit a space, because `<button>A</button><button>B</button>` is two words and
/// without the separator it becomes one fabricated token.
///
/// Comments are dropped whole: an unterminated one runs to the end of the
/// document and nothing after it is text.
#[must_use]
pub fn strip_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut quote: Option<char> = None;
    let mut rest = html;

    while let Some(c) = rest.chars().next() {
        rest = &rest[c.len_utf8()..];
        if in_tag {
            match quote {
                Some(q) if c == q => quote = None,
                Some(_) => {}
                None => match c {
                    '"' | '\'' => quote = Some(c),
                    '>' => {
                        in_tag = false;
                        out.push(' ');
                    }
                    _ => {}
                },
            }
        } else {
            match c {
                '<' => {
                    in_tag = true;
                    out.push(' ');
                    if rest.starts_with("!--") {
                        let Some(end) = rest[3..].find("-->") else {
                            break;
                        };
                        rest = &rest[3 + end + 3..];
                    }
                }
                _ => out.push(c),
            }
        }
    }
    decode_entities(&out)
}

/// Decode the entities that appear in titles, descriptions and body text.
///
/// `&amp;` is decoded last so that `&amp;#39;` does not become a bare `#39`.
#[must_use]
pub fn decode_entities(s: &str) -> String {
    s.replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&quot;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ")
        .replace("&mdash;", "\u{2014}")
        .replace("&ndash;", "\u{2013}")
        .replace("&hellip;", "\u{2026}")
        .replace("&amp;", "&")
}

/// The text of every `<script type="application/ld+json">` block, in order.
///
/// This exists because the parser records that a JSON-LD script is present but
/// not what it contains, and it *silently discards* any block whose JSON does
/// not parse. A malformed block therefore never reaches
/// [`ParsedPage::structured_data`], and no analyzer reading that field can tell
/// a page with broken schema from one with none. Found by the testbed: a fixture
/// with a trailing comma in its JSON-LD produced no finding at all.
///
/// Reading the document again here is deliberate rather than a workaround. The
/// blocks that matter are precisely the ones the parser throws away.
///
/// Nested `</script>` inside a JSON string would end the block early. That
/// requires an escaped closing tag inside the JSON, which does not occur in
/// practice because JSON strings cannot contain a raw `/`-terminated script
/// close without the author having escaped it as `<\/script>` for exactly this
/// reason.
///
/// [`ParsedPage::structured_data`]: crate::parser::ParsedPage::structured_data
#[must_use]
pub fn ldjson_script_bodies(html: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let lower = html.to_ascii_lowercase();
    // Everything below is an absolute offset into `html`/`lower`. An earlier
    // version advanced a pair of slices and computed one advance from a mix of
    // the two coordinate systems; the tests caught it as an out-of-bounds panic
    // on the second block.
    let mut i = 0usize;
    while let Some(rel) = lower[i..].find("<script") {
        let open = i + rel;
        let attrs_start = open + "<script".len();
        let Some(tag_end_rel) = find_tag_end(&lower[attrs_start..]) else {
            break;
        };
        let tag_end = attrs_start + tag_end_rel;
        let open_tag = &html[open..tag_end];
        let is_ld = attr_value(open_tag, "type")
            .map(|t| t.trim().eq_ignore_ascii_case("application/ld+json"))
            .unwrap_or(false);
        let Some(close_rel) = lower[tag_end..].find("</script>") else {
            break;
        };
        let body_end = tag_end + close_rel;
        if is_ld {
            blocks.push(html[tag_end..body_end].trim().to_string());
        }
        i = body_end + "</script>".len();
    }
    blocks
}

/// Offset of the `>` that closes a start tag, honouring quoted values.
fn find_tag_end(s: &str) -> Option<usize> {
    let mut quote: Option<char> = None;
    for (i, c) in s.char_indices() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => {}
            None => match c {
                q @ ('"' | '\'') => quote = Some(q),
                '>' => return Some(i + 1),
                _ => {}
            },
        }
    }
    None
}

/// Every start tag for elements named `name`, as the raw tag text.
///
/// The tag includes its closing `>`, so [`attr_value`] can be called on it
/// directly. Matching is case-insensitive and ignores `</a>` closers, `<!doctype>`
/// and comments, because `find_tag_end` only stops the scan at a `>` that is not
/// inside a quoted attribute value.
#[must_use]
pub fn start_tags(html: &str, name: &str) -> Vec<String> {
    let mut tags = Vec::new();
    let lower = html.to_ascii_lowercase();
    let needle = format!("<{name}");
    let mut i = 0usize;
    while let Some(rel) = lower[i..].find(&needle) {
        let open = i + rel;
        // The next character must be a name boundary, so `<abstract>` does not
        // match a search for `<a`.
        let after = open + needle.len();
        if matches!(lower.as_bytes().get(after), Some(b) if b.is_ascii_alphanumeric()) {
            i = after;
            continue;
        }
        let Some(tag_end_rel) = find_tag_end(&lower[after..]) else {
            break;
        };
        let tag_end = after + tag_end_rel;
        tags.push(html[open..tag_end].to_string());
        i = tag_end;
    }
    tags
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quoted_and_unquoted_attributes() {
        assert_eq!(
            attr_value(r#"<meta name="description" content="Hi">"#, "content").as_deref(),
            Some("Hi")
        );
        assert_eq!(
            attr_value(r#"<meta content=Hi name=description>"#, "content").as_deref(),
            Some("Hi")
        );
        assert_eq!(attr_value(r#"<meta data-name="x">"#, "name"), None);
    }

    #[test]
    fn greater_than_inside_an_attribute_does_not_end_the_tag() {
        let html = r#"<p title="a > b">visible</p>"#;
        assert_eq!(strip_tags(html).trim(), "visible");
    }

    #[test]
    fn adjacent_elements_do_not_merge() {
        assert_eq!(strip_tags("<b>a</b><i>b</i>").split_whitespace().count(), 2);
    }

    #[test]
    fn entities_decode() {
        assert_eq!(decode_entities("Tom &amp; Jerry&#39;s"), "Tom & Jerry's");
        // &amp; last, so a doubly-escaped entity is not half-decoded.
        assert_eq!(decode_entities("&amp;#39;"), "&#39;");
    }

    #[test]
    fn jsonld_blocks_are_extracted() {
        let html = r#"<script type="application/ld+json">{"a":1}</script><p>x</p>"#;
        assert_eq!(ldjson_script_bodies(html), vec![r#"{"a":1}"#]);
    }

    #[test]
    fn minified_unquoted_type_is_recognised() {
        let html = r#"<script type=application/ld+json>{"a":1}</script>"#;
        assert_eq!(ldjson_script_bodies(html).len(), 1);
    }

    #[test]
    fn other_scripts_are_ignored() {
        let html = r#"<script>var a=1;</script><script type="text/javascript">var b=2;</script>"#;
        assert!(ldjson_script_bodies(html).is_empty());
    }

    /// An unquoted value must not swallow the `>` that closes the tag.
    /// `type=application/ld+json>` compared unequal to `application/ld+json`,
    /// so minified JSON-LD was invisible to the extractor.
    #[test]
    fn unquoted_value_stops_at_the_tag_close() {
        let tag = r#"<script type=application/ld+json>"#;
        assert_eq!(
            attr_value(tag, "type").as_deref(),
            Some("application/ld+json")
        );
    }

    #[test]
    fn multiple_blocks_in_order() {
        let html = r#"<script type="application/ld+json">{"a":1}</script>
            <script type="application/ld+json">{"b":2}</script>"#;
        assert_eq!(ldjson_script_bodies(html).len(), 2);
    }

    /// The case that motivated this function: a block whose JSON is broken must
    /// still be *returned*, because returning it is what makes reporting the
    /// breakage possible.
    #[test]
    fn malformed_json_is_still_extracted() {
        let html = r#"<script type="application/ld+json">{"a":1,}</script>"#;
        assert_eq!(ldjson_script_bodies(html), vec![r#"{"a":1,}"#]);
    }

    #[test]
    fn unterminated_script_does_not_loop() {
        let html = r#"<script type="application/ld+json">{"a":1}"#;
        assert!(ldjson_script_bodies(html).is_empty());
    }

    /// `<abstract>` shares a prefix with `<a`, and must not be returned when
    /// scanning for anchors.
    #[test]
    fn start_tags_match_on_a_name_boundary() {
        let html = r#"<a href="/x">anchor</a><abstract>not an anchor</abstract>"#;
        let tags = start_tags(html, "a");
        assert_eq!(tags.len(), 1, "got {tags:?}");
        assert!(tags[0].contains(r#"href="/x""#));
    }

    #[test]
    fn start_tags_are_case_insensitive_and_skip_closers() {
        let html = r#"<A HREF="/x">upper</A><a href="/y">lower</a>"#;
        assert_eq!(start_tags(html, "a").len(), 2);
    }
}
