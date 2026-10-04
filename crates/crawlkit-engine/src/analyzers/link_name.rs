//! One definition of "does this link have an accessible name?".
//!
//! # Why this exists
//!
//! Six analyzers need to answer the same question — is a link's accessible name
//! empty? — and they had drifted into three different answers:
//!
//! ```text
//! l.text.trim().is_empty() && l.aria_label.is_none() && l.img_alt.is_none()
//! ```
//!
//! treats `aria-label=""` and `alt=""` as *present*, because `Some("")` is not
//! `None`. An empty attribute names nothing, so the link is nameless and the check
//! misses it. Two other sites used a bare `is_none()` on `aria_label` with the same
//! hole. Only two used the correct form.
//!
//! The inverse drift is worse and was also live: `A11Y-LINK-V2001` checked neither
//! `img_alt` nor an empty `aria-label`, and reported
//! `<a href="/"><img alt="xkcd.com logo"></a>` as a link with empty text at **Error**
//! — while its own recommendation told the author to add "an img with alt text
//! inside each link". Verified against a capture of xkcd.com.
//!
//! The three sources of an accessible name for a link, in the order a browser
//! resolves them:
//!
//! 1. `aria-label` — wins outright when present and non-empty.
//! 2. Visible link text.
//! 3. An `alt` on a descendant image.
//!
//! All three must be empty for the link to be nameless, and "empty" must mean
//! empty-after-trim for each. Centralising this is the fix; a comment at each of
//! six call sites asking the next reader to be careful is not.

/// True when a link exposes no accessible name to assistive technology.
///
/// * `text` — the link's visible text.
/// * `aria_label` — the `aria-label` attribute, if any.
/// * `img_alt` — the `alt` of an image inside the link, if any.
#[must_use]
pub fn is_nameless(text: &str, aria_label: Option<&str>, img_alt: Option<&str>) -> bool {
    let text_empty = text.trim().is_empty();
    let aria_empty = aria_label.is_none_or(|v| v.trim().is_empty());
    let img_empty = img_alt.is_none_or(|v| v.trim().is_empty());
    text_empty && aria_empty && img_empty
}

#[cfg(test)]
mod tests {
    use super::is_nameless;
    use crate::parser::ExtractedLink;

    fn link(text: &str, aria: Option<&str>, alt: Option<&str>) -> ExtractedLink {
        ExtractedLink {
            href: "/x".to_string(),
            text: text.to_string(),
            rel: vec![],
            is_external: false,
            aria_label: aria.map(str::to_string),
            img_alt: alt.map(str::to_string),
        }
    }

    fn nameless(l: &ExtractedLink) -> bool {
        is_nameless(&l.text, l.aria_label.as_deref(), l.img_alt.as_deref())
    }

    /// Every way a link can end up with a name.
    #[test]
    fn each_naming_mechanism_rescues_the_link() {
        assert!(!nameless(&link("Archive", None, None)), "visible text");
        assert!(!nameless(&link("", Some("Home"), None)), "aria-label");
        assert!(!nameless(&link("", None, Some("xkcd logo"))), "image alt");
    }

    /// The regression that motivated this module: an image with `alt` inside a link
    /// gives that link its accessible name.
    #[test]
    fn image_alt_names_the_link() {
        assert!(!nameless(&link("", None, Some("xkcd.com logo"))));
    }

    /// The drift this module exists to prevent. `Some("")` is not `None`, so an
    /// `is_none()` check would treat these as named and miss them.
    #[test]
    fn empty_attributes_do_not_name_anything() {
        for l in [
            link("", Some(""), None),
            link("", None, Some("")),
            link("", Some(""), Some("")),
            link("   ", Some("  "), Some("\t\n")),
            link("", None, Some("   ")),
        ] {
            assert!(
                nameless(&l),
                "empty attributes leave the link nameless: text={:?} aria={:?} alt={:?}",
                l.text,
                l.aria_label,
                l.img_alt
            );
        }
    }

    /// The genuine defect still reports.
    #[test]
    fn nameless_links_are_detected() {
        for l in [
            link("", None, None),
            link("   ", None, None),
            link("", Some(""), None),
            link("", None, Some(" ")),
        ] {
            assert!(nameless(&l));
        }
    }

    /// Whitespace-only visible text is not a name; this is the case a naive
    /// `text.is_empty()` misses.
    #[test]
    fn whitespace_only_text_is_not_a_name() {
        assert!(nameless(&link("\n\t  ", None, None)));
    }
}
