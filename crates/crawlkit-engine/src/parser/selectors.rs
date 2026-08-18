//! Cached CSS selectors compiled once, reused on every parse call.
//!
//! `OnceLock` guarantees thread-safe lazy initialization with zero cost after
//! first use. All selector patterns are static compile-time-known strings,
//! so a failed parse is a programming error and panics with a clear message.
#![allow(clippy::expect_used)]

use scraper::Selector;
use std::sync::OnceLock;

#[allow(dead_code)]
pub fn html() -> &'static Selector {
    static CELL: OnceLock<Selector> = OnceLock::new();
    CELL.get_or_init(|| Selector::parse("html").expect("static CSS selector is valid"))
}
pub fn header() -> &'static Selector {
    static CELL: OnceLock<Selector> = OnceLock::new();
    CELL.get_or_init(|| Selector::parse("header").expect("static CSS selector is valid"))
}
pub fn nav() -> &'static Selector {
    static CELL: OnceLock<Selector> = OnceLock::new();
    CELL.get_or_init(|| Selector::parse("nav").expect("static CSS selector is valid"))
}
pub fn main() -> &'static Selector {
    static CELL: OnceLock<Selector> = OnceLock::new();
    CELL.get_or_init(|| Selector::parse("main").expect("static CSS selector is valid"))
}
pub fn aside() -> &'static Selector {
    static CELL: OnceLock<Selector> = OnceLock::new();
    CELL.get_or_init(|| Selector::parse("aside").expect("static CSS selector is valid"))
}
pub fn footer() -> &'static Selector {
    static CELL: OnceLock<Selector> = OnceLock::new();
    CELL.get_or_init(|| Selector::parse("footer").expect("static CSS selector is valid"))
}
pub fn form() -> &'static Selector {
    static CELL: OnceLock<Selector> = OnceLock::new();
    CELL.get_or_init(|| Selector::parse("form").expect("static CSS selector is valid"))
}
pub fn section_aria() -> &'static Selector {
    static CELL: OnceLock<Selector> = OnceLock::new();
    CELL.get_or_init(|| {
        Selector::parse("section[aria-label], section[aria-labelledby]")
            .expect("static CSS selector is valid")
    })
}
pub fn role_banner() -> &'static Selector {
    static CELL: OnceLock<Selector> = OnceLock::new();
    CELL.get_or_init(|| Selector::parse("[role=banner]").expect("static CSS selector is valid"))
}
pub fn role_navigation() -> &'static Selector {
    static CELL: OnceLock<Selector> = OnceLock::new();
    CELL.get_or_init(|| Selector::parse("[role=navigation]").expect("static CSS selector is valid"))
}
pub fn role_main() -> &'static Selector {
    static CELL: OnceLock<Selector> = OnceLock::new();
    CELL.get_or_init(|| Selector::parse("[role=main]").expect("static CSS selector is valid"))
}
pub fn role_complementary() -> &'static Selector {
    static CELL: OnceLock<Selector> = OnceLock::new();
    CELL.get_or_init(|| {
        Selector::parse("[role=complementary]").expect("static CSS selector is valid")
    })
}
pub fn role_contentinfo() -> &'static Selector {
    static CELL: OnceLock<Selector> = OnceLock::new();
    CELL.get_or_init(|| {
        Selector::parse("[role=contentinfo]").expect("static CSS selector is valid")
    })
}
pub fn input_select_textarea() -> &'static Selector {
    static CELL: OnceLock<Selector> = OnceLock::new();
    CELL.get_or_init(|| {
        Selector::parse("input, select, textarea").expect("static CSS selector is valid")
    })
}
pub fn link_hreflang() -> &'static Selector {
    static CELL: OnceLock<Selector> = OnceLock::new();
    CELL.get_or_init(|| Selector::parse("link[hreflang]").expect("static CSS selector is valid"))
}
pub fn anchor_href() -> &'static Selector {
    static CELL: OnceLock<Selector> = OnceLock::new();
    CELL.get_or_init(|| Selector::parse("a[href]").expect("static CSS selector is valid"))
}
pub fn img() -> &'static Selector {
    static CELL: OnceLock<Selector> = OnceLock::new();
    CELL.get_or_init(|| Selector::parse("img").expect("static CSS selector is valid"))
}
pub fn label() -> &'static Selector {
    static CELL: OnceLock<Selector> = OnceLock::new();
    CELL.get_or_init(|| Selector::parse("label").expect("static CSS selector is valid"))
}
pub fn fieldset() -> &'static Selector {
    static CELL: OnceLock<Selector> = OnceLock::new();
    CELL.get_or_init(|| Selector::parse("fieldset").expect("static CSS selector is valid"))
}
pub fn legend() -> &'static Selector {
    static CELL: OnceLock<Selector> = OnceLock::new();
    CELL.get_or_init(|| Selector::parse("legend").expect("static CSS selector is valid"))
}
pub fn script() -> &'static Selector {
    static CELL: OnceLock<Selector> = OnceLock::new();
    CELL.get_or_init(|| Selector::parse("script").expect("static CSS selector is valid"))
}
pub fn link_stylesheet() -> &'static Selector {
    static CELL: OnceLock<Selector> = OnceLock::new();
    CELL.get_or_init(|| {
        Selector::parse("link[rel=stylesheet]").expect("static CSS selector is valid")
    })
}
pub fn style() -> &'static Selector {
    static CELL: OnceLock<Selector> = OnceLock::new();
    CELL.get_or_init(|| Selector::parse("style").expect("static CSS selector is valid"))
}
pub fn script_ld_json() -> &'static Selector {
    static CELL: OnceLock<Selector> = OnceLock::new();
    CELL.get_or_init(|| {
        Selector::parse("script[type=\"application/ld+json\"]")
            .expect("static CSS selector is valid")
    })
}
pub fn body() -> &'static Selector {
    static CELL: OnceLock<Selector> = OnceLock::new();
    CELL.get_or_init(|| Selector::parse("body").expect("static CSS selector is valid"))
}
pub fn noscript() -> &'static Selector {
    static CELL: OnceLock<Selector> = OnceLock::new();
    CELL.get_or_init(|| Selector::parse("noscript").expect("static CSS selector is valid"))
}
pub fn anchor_fragment() -> &'static Selector {
    static CELL: OnceLock<Selector> = OnceLock::new();
    CELL.get_or_init(|| Selector::parse("a[href^=\"#\"]").expect("static CSS selector is valid"))
}
pub fn table() -> &'static Selector {
    static CELL: OnceLock<Selector> = OnceLock::new();
    CELL.get_or_init(|| Selector::parse("table").expect("static CSS selector is valid"))
}
pub fn th() -> &'static Selector {
    static CELL: OnceLock<Selector> = OnceLock::new();
    CELL.get_or_init(|| Selector::parse("th").expect("static CSS selector is valid"))
}
pub fn caption() -> &'static Selector {
    static CELL: OnceLock<Selector> = OnceLock::new();
    CELL.get_or_init(|| Selector::parse("caption").expect("static CSS selector is valid"))
}
