//! Unit tests for [`crate::analyzers::robots_txt_star_blanket_disallows_all`].
//!
//! The helper reports "blocks all crawlers" only when a `Disallow: /` sits in
//! the `User-agent: *` group; named-bot groups such as `CCBot` must not
//! trigger it (a real false positive observed on kingstonpeptides.com).

use crate::analyzers::robots_txt_star_blanket_disallows_all;

#[test]
fn named_bot_blanket_disallow_does_not_block_generic_crawler() {
    // Kingston Peptides: the blanket `Disallow: /` applies only to CCBot and
    // ByteSpider; the `*` group explicitly allows crawling.
    let robots = "# Kingston Peptides robots.txt
User-agent: *
Allow: /
Disallow: /admin
Disallow: /api/
User-agent: CCBot
Disallow: /
User-agent: ByteSpider
Disallow: /
";
    assert!(!robots_txt_star_blanket_disallows_all(robots));
}

#[test]
fn star_group_blanket_disallow_blocks_all() {
    let robots = "User-agent: *\nDisallow: /\n";
    assert!(robots_txt_star_blanket_disallows_all(robots));
}

#[test]
fn star_group_allow_after_disallow_wins_tie() {
    // Equal-priority rules resolve to the last one: the trailing Allow: /
    // leaves the crawler unblocked.
    let robots = "User-agent: *\nDisallow: /\nAllow: /\n";
    assert!(!robots_txt_star_blanket_disallows_all(robots));
}

#[test]
fn star_group_disallow_after_allow_still_blocks() {
    let robots = "User-agent: *\nAllow: /\nDisallow: /\n";
    assert!(robots_txt_star_blanket_disallows_all(robots));
}

#[test]
fn rules_without_any_user_agent_match_no_crawler() {
    let robots = "Disallow: /\n";
    assert!(!robots_txt_star_blanket_disallows_all(robots));
}

#[test]
fn consecutive_user_agent_lines_form_one_group() {
    // The blanket rule follows a header listing both `*` and CCBot, so the
    // `*` agent is blocked too (RFC 9309 group semantics).
    let robots = "User-agent: *\nUser-agent: CCBot\nDisallow: /\n";
    assert!(robots_txt_star_blanket_disallows_all(robots));
}

#[test]
fn path_specific_disallows_do_not_block() {
    let robots = "User-agent: *\nDisallow: /admin\nDisallow: /api/\n";
    assert!(!robots_txt_star_blanket_disallows_all(robots));
}

#[test]
fn comment_on_disallow_line_is_ignored() {
    let robots = "User-agent: *\nDisallow: / # block everything\n";
    assert!(robots_txt_star_blanket_disallows_all(robots));
}

#[test]
fn empty_and_non_rule_lines_are_ignored() {
    assert!(!robots_txt_star_blanket_disallows_all(""));
    assert!(!robots_txt_star_blanket_disallows_all("# only a comment\n"));
    assert!(!robots_txt_star_blanket_disallows_all(
        "Sitemap: https://x/sitemap.xml\n"
    ));
}
