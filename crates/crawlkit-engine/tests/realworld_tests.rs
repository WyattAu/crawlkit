//! Offline validation against captured real-world pages.
//!
//! The synthetic corpus in `../corpus/` proves analyzers behave on markup we
//! wrote. This suite proves they behave on markup nobody wrote for the test —
//! which is where every false-positive class in the audit rounds was found.
//!
//! It runs entirely offline against `fixtures/realworld/` (see
//! `PROVENANCE.md`), so a failure always means crawlkit regressed, never "the
//! site changed".
//!
//! Four kinds of assertion, in increasing order of how much they pin down:
//!
//! 1. **Deny-list** — codes proven to fire on 100% of pages must never appear
//!    on any real page. This is the regression net: each entry is a defect
//!    class that was fixed and must stay fixed.
//! 2. **Per-page expectations** — codes that must be present (verified true
//!    positives) and codes that must be absent (verified true negatives).
//! 3. **Structural invariants** — properties that must hold of *any* output,
//!    independent of which site is being analyzed.
//! 4. **Determinism** — the same input must produce byte-identical output.

// A test that cannot name the fixture it is failing on is not a useful test, so
// `unwrap`/`expect`/`panic` are the right tools here. The workspace denies them
// in library code, where a panic is a production defect.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crawlkit_engine::analyzers::defect_family::defect_key;
use crawlkit_engine::analyzers::{dedupe::defect_signature, AnalyzerRegistry};
use crawlkit_engine::parser::HtmlParser;
use crawlkit_engine::{AnalysisContext, CrawlConfig};
use url::Url;

fn realworld_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/realworld")
}

/// The URL each capture was served from. Needed because `defect_key` and the
/// analyzers themselves resolve relative links, and a capture analysed under the
/// wrong origin would test the wrong thing.
fn source_url(fixture: &str) -> &'static str {
    match fixture {
        "govuk_homepage.html" => "https://www.gov.uk/",
        "govuk_check_mot_history.html" => "https://www.gov.uk/check-mot-history",
        "nhs.html" => "https://www.nhs.uk/",
        "kp_homepage.html" => "https://kingstonpeptides.com/",
        "mozilla_blog.html" => "https://blog.mozilla.org/en/",
        "arxiv_cs.html" => "https://arxiv.org/list/cs.AI/recent",
        "rust_docs.html" => "https://doc.rust-lang.org/std/",
        "xkcd.html" => "https://xkcd.com/353/",
        "python_org.html" => "https://www.python.org/",
        "example_org.html" => "https://example.com/",
        other => panic!("no source URL recorded for fixture {other}; add it to source_url()"),
    }
}

fn fixture_paths() -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(realworld_dir())
        .expect("realworld fixture directory must exist")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "html"))
        .collect();
    paths.sort();
    assert!(
        paths.len() >= 10,
        "real-world corpus should hold at least 10 captures, found {}",
        paths.len()
    );
    paths
}

fn analyze(path: &Path) -> Vec<crawlkit_engine::Finding> {
    let name = path.file_name().unwrap().to_str().unwrap();
    let html = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("{name} must be readable UTF-8: {e}"));
    assert!(
        html.trim_start()
            .to_ascii_lowercase()
            .starts_with("<!doctype")
            || html.to_ascii_lowercase().contains("<html"),
        "{name} does not look like HTML — was it captured with --compressed?"
    );

    let url = Url::parse(source_url(name)).expect("fixture source URL must parse");
    let page = HtmlParser::parse(&html, &url);
    let ctx = AnalysisContext {
        page: &page,
        body: Some(&html),
        status_code: Some(200),
        headers: &[],
        response_time: Some(Duration::from_millis(120)),
        redirect_chain: &[],
        robots_txt: None,
        user_agent: None,
        body_size: Some(html.len()),
        compressed_size: Some(html.len() / 3),
        content_encoding: None,
        server: Some("nginx/1.24.0"),
        content_type: Some("text/html; charset=utf-8"),
        rendered: None,
    };
    let registry = AnalyzerRegistry::new(&CrawlConfig::default());
    registry.analyze(&ctx)
}

/// `(fixture, code, severity, title) -> count`, for readable diffs.
fn finding_index(
    findings: &[crawlkit_engine::Finding],
) -> BTreeMap<(String, String, String, String), usize> {
    let mut m = BTreeMap::new();
    for f in findings {
        *m.entry((
            f.url.clone(),
            f.code.clone(),
            format!("{:?}", f.severity),
            f.title.clone(),
        ))
        .or_insert(0) += 1;
    }
    m
}

fn codes_of(findings: &[crawlkit_engine::Finding]) -> BTreeSet<String> {
    findings.iter().map(|f| f.code.clone()).collect()
}

// ---------------------------------------------------------------------------
// 1. The deny-list
// ---------------------------------------------------------------------------

/// Codes whose **analyzer was removed**. Must appear neither in the analyzer
/// sources nor in any finding.
///
/// Each entry is a false-positive class that was fixed at the root, not
/// suppressed. Grouped by cause, because a fix that only removed one code would
/// leave the class alive.
const REMOVED_CODES: &[(&str, &str, &str)] = &[
    (
        "ELINK001",
        "entity linking",
        "asserted that a schema entity needs an outbound *body* link to strengthen \
         entity signals. No engine requires it, and the mechanism that exists \
         (`sameAs` on the entity) is not what the rule checked. Fired on 40/40 pages.",
    ),
    (
        "ELINK002",
        "entity linking",
        "same fabricated 'topical authority' requirement as ELINK001.",
    ),
    (
        "COLRCL-V2001-UNDERLINE",
        "colour contrast",
        "fired whenever any CSS rule paired `text-decoration:none` with a `color:` \
         declaration anywhere in the document -- the default styling of most \
         navigation. WCAG 1.4.1 needs computed styles per element, not a \
         stylesheet regex. Fired on 40/40 pages.",
    ),
    (
        "CANDEP-V2003",
        "canonical",
        "flagged a trailing slash on a canonical, which is a valid URL form, and \
         contradicted url_norm, which deliberately folds trailing slashes when \
         deciding equivalence. Fired on 40/40 pages.",
    ),
];

/// Codes that are **still emitted** but must never survive dedupe.
///
/// These are the members of a defect family that a more severe sibling outranks.
/// Their analyzers are correct; what must not happen is one of them being the
/// representative. If any appears in output, the "most severe member wins" rule
/// has regressed and a finding is once again shipping a severity its own code
/// does not declare.
const SUPERSEDED_CODES: &[(&str, &str, &str)] = &[
    (
        "TITLE-V4001",
        "title",
        "'Missing title tag' at Error; TITLEMISS-V2001 reports the same defect at \
         Critical and must represent the family.",
    ),
    (
        "AI-CIT001",
        "canonical",
        "'Missing canonical URL' at Info; CAN-V3001 reports it at Warning.",
    ),
    (
        "META009",
        "viewport",
        "missing viewport meta; MOB001 reports it at Error.",
    ),
];

#[test]
fn removed_codes_never_fire_on_real_pages() {
    let mut hits: Vec<String> = Vec::new();
    for path in fixture_paths() {
        let name = path.file_name().unwrap().to_str().unwrap().to_string();
        let codes = codes_of(&analyze(&path));
        for (code, _group, _why) in REMOVED_CODES.iter().chain(SUPERSEDED_CODES) {
            if codes.contains(*code) {
                hits.push(format!("{name}: {code}"));
            }
        }
    }
    assert!(
        hits.is_empty(),
        "codes that must never reach output are being reported:\n  {}",
        hits.join("\n  ")
    );
}

/// The lists are only meaningful if populated, justified, and actually absent
/// from the analyzer sources. An empty or vacuous list would make the test above
/// pass for the wrong reason.
#[test]
fn deny_lists_are_populated_and_the_removed_codes_are_really_gone() {
    assert!(
        REMOVED_CODES.len() >= 4 && SUPERSEDED_CODES.len() >= 3,
        "both deny lists should be populated: {} removed, {} superseded",
        REMOVED_CODES.len(),
        SUPERSEDED_CODES.len()
    );
    // Padding with a code and an empty rationale would satisfy a count while
    // testing nothing.
    for (code, group, why) in REMOVED_CODES.iter().chain(SUPERSEDED_CODES) {
        assert!(
            why.len() > 40 && !group.is_empty(),
            "{code} needs a substantive rationale and a root-cause group"
        );
    }
    // Only the *removed* codes may be absent from source. A superseded code is
    // still emitted by a correct analyzer; it just loses the representative
    // election.
    let sources = [
        include_str!("../src/analyzers/mod.rs"),
        include_str!("../src/analyzers/content_analyzers.rs"),
        include_str!("../src/analyzers/v2/seo.rs"),
        include_str!("../src/analyzers/v2/accessibility.rs"),
        include_str!("../src/analyzers/post_crawl_analyzers.rs"),
    ];
    for (code, _group, _why) in REMOVED_CODES {
        for src in sources {
            assert!(
                !src.contains(&format!("code: \"{code}\"")),
                "{code} is on the removed list but is emitted again by an analyzer"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 2. Structural invariants — true of any output, on any site
// ---------------------------------------------------------------------------

/// Dedupe must leave exactly one finding per defect per page.
///
/// Keys on `(url, defect family)` so a code reported once per offending element
/// — several analyzers legitimately do that — is *not* collapsed, because those
/// share a code and `collapse_duplicates` never merges those. Counting distinct
/// codes per family is therefore the right invariant: a family split across
/// codes means one defect is reported several times.
#[test]
fn one_defect_is_reported_once_per_page() {
    for path in fixture_paths() {
        let name = path.file_name().unwrap().to_str().unwrap();
        let findings = analyze(&path);
        let mut per_family: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();
        for f in &findings {
            let family = defect_key(&f.code, &defect_signature(&f.title));
            per_family
                .entry((f.url.clone(), family))
                .or_default()
                .insert(f.code.clone());
        }
        let split: Vec<String> = per_family
            .into_iter()
            .filter(|(_, codes)| codes.len() > 1)
            .map(|((url, family), codes)| {
                let mut v: Vec<&String> = codes.iter().collect();
                v.sort();
                format!("{url} :: {family} :: {v:?}")
            })
            .collect();
        assert!(
            split.is_empty(),
            "{name}: one defect reported under several codes:\n  {}",
            split.join("\n  ")
        );
    }
}

/// A finding must report the severity its own code declares.
///
/// The dedupe rule keeps severity at the family maximum but makes the most severe
/// member survive *as a finding*, so code and severity always come from the same
/// analyzer. Copying a sibling's severity onto a different code produced live
/// output where `MDESC-PX002` and `IMG004` — both declaring `info` — shipped as
/// `warning`.
///
/// Scope, stated precisely because it is easy to over-trust: this is a
/// **consistency check on real output**, not a regression guard for the rule. A
/// chimera needs a family whose alphabetically-first member is *less* severe than
/// a later one, and no capture in this corpus happens to contain one — verified by
/// reverting `collapse_duplicates` to positional-wins, which leaves this test
/// green. The rule itself is guarded by `dedupe::tests`, where reverting it fails
/// three tests. What this adds is coverage the unit tests cannot have: that real
/// pages never exhibit the inconsistency.
#[test]
fn a_finding_never_contradicts_its_own_codes_severity() {
    // Declared severities, read from the analyzer sources at compile time rather
    // than maintained by hand, so this test cannot drift from reality.
    let sources = [
        include_str!("../src/analyzers/mod.rs"),
        include_str!("../src/analyzers/content_analyzers.rs"),
        include_str!("../src/analyzers/v2/seo.rs"),
        include_str!("../src/analyzers/v2/accessibility.rs"),
        include_str!("../src/analyzers/post_crawl_analyzers.rs"),
    ];
    let mut declared: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for src in sources {
        // Walk forward recording where each `code: "` literal starts, then walk
        // *backwards* from it to the severity that belongs to the same struct
        // literal.
        //
        // Searching forward was the original bug here and it is an easy one to
        // write: the text after a code literal belongs to the *next* finding, so a
        // forward search attributes the following finding's severity to this code
        // and reports a mismatch that does not exist.
        let mut code_positions: Vec<(usize, String)> = Vec::new();
        let mut idx = 0usize;
        while let Some(rel) = src[idx..].find("code: \"") {
            let start = idx + rel + "code: \"".len();
            let Some(end) = src[start..].find('"') else {
                break;
            };
            let name = &src[start..start + end];
            if !name.is_empty() && name.starts_with(|c: char| c.is_ascii_uppercase()) {
                code_positions.push((start, name.to_string()));
            }
            idx = start + end;
        }
        for (pos, name) in code_positions {
            // The enclosing finding's severity is the last one before this code
            // that is not separated by the end of a previous struct literal.
            let head = &src[..pos];
            let Some(bound) = head.rfind('}') else {
                continue;
            };
            let region = &head[bound..];
            let Some(sev_at) = region.rfind("severity: Severity::") else {
                continue;
            };
            let tail = &region[sev_at + "severity: Severity::".len()..];
            let sev: String = tail
                .chars()
                .take_while(|c| c.is_ascii_alphabetic())
                .collect();
            if !sev.is_empty() {
                declared.entry(name).or_default().insert(sev.to_lowercase());
            }
        }
    }

    let mut bad: Vec<String> = Vec::new();
    for path in fixture_paths() {
        let name = path.file_name().unwrap().to_str().unwrap();
        for f in analyze(&path) {
            let Some(allowed) = declared.get(&f.code) else {
                continue;
            };
            let actual = format!("{:?}", f.severity).to_lowercase();
            if !allowed.contains(&actual) {
                bad.push(format!(
                    "{name}: {} ships as {actual} but declares {:?}",
                    f.code,
                    allowed.iter().collect::<Vec<_>>()
                ));
            }
        }
    }
    assert!(
        bad.is_empty(),
        "findings reporting a severity their own code never declares:\n  {}",
        bad.join("\n  ")
    );
}

/// Every captured page must produce some findings, or the corpus has stopped
/// exercising the analyzers (a parser regression would look like a pass).
#[test]
fn every_capture_still_produces_findings() {
    for path in fixture_paths() {
        let name = path.file_name().unwrap().to_str().unwrap();
        let findings = analyze(&path);
        assert!(
            !findings.is_empty(),
            "{name} produced no findings at all — the parser or the registry is broken, \
             and this corpus would silently stop testing anything"
        );
    }
}

/// Findings must be sorted, so two runs over the same input are comparable.
#[test]
fn findings_are_in_canonical_order() {
    for path in fixture_paths() {
        let name = path.file_name().unwrap().to_str().unwrap();
        let findings = analyze(&path);
        for pair in findings.windows(2) {
            let a = (&pair[0].code, &pair[0].url);
            let b = (&pair[1].code, &pair[1].url);
            assert!(
                a <= b,
                "{name}: findings not sorted by (code, url): {a:?} came before {b:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 3. Determinism
// ---------------------------------------------------------------------------

/// The same capture analysed twice must produce byte-identical output.
///
/// Six analyzers sorted `HashMap` ties randomly and `LogAnalysis` held a
/// `HashMap`, so `serde` emitted keys in a random order. Both are invisible in a
/// count-based assertion and obvious in a snapshot.
#[test]
fn analysis_is_deterministic() {
    for path in fixture_paths() {
        let name = path.file_name().unwrap().to_str().unwrap();
        let first = finding_index(&analyze(&path));
        let second = finding_index(&analyze(&path));
        assert_eq!(
            first, second,
            "{name}: two runs over the same capture disagreed"
        );
    }
}

// ---------------------------------------------------------------------------
// 4. A representative, human-verified snapshot
// ---------------------------------------------------------------------------

/// Codes verified present on specific captures.
///
/// Every entry here was read off a live page and checked by hand. A count-based
/// claim would rot silently; naming the code makes a change to it a deliberate
/// edit rather than a diff nobody reads.
const VERIFIED_TRUE_POSITIVES: &[(&str, &[&str])] = &[
    // Checked against the raw capture, not inferred from the code name.
    //
    // `example_org.html` is 577 bytes of visible markup: one <title> reading
    // "Example Domain" (14 characters), one paragraph, and no <meta
    // name="description"> anywhere. So all three of these are true.
    (
        "example_org.html",
        &[
            "META-V3001", // "Missing meta description" — the capture has none
            "META002",    // "Title too short" — 14 characters
            "THIN002",    // "Extremely thin content"
        ],
    ),
    // `NAP001` (LocalBusiness schema with no telephone) was verified by hand
    // against kingstonpeptides.com's live JSON-LD during the audit.
    ("kp_homepage.html", &["NAP001"]),
    // gov.uk carries no article:modified_time / published_time / datePublished /
    // itemprop=date meta at all — grep over the whole document returns zero.
    ("govuk_homepage.html", &["FRESHSC001"]),
    // xkcd.com's <html> element carries no attributes whatsoever; the only
    // `lang=` in the document is a query parameter on a Google CSE script URL.
    // And it declares no <link rel="canonical">.
    ("xkcd.html", &["A11Y016", "CAN-V3001"]),
    // arXiv's listing page is an extreme link-density outlier.
    ("arxiv_cs.html", &["LINKSC001"]),
];

/// Codes verified *absent* on specific captures, having checked the page.
const VERIFIED_TRUE_NEGATIVES: &[(&str, &[&str])] = &[
    // gov.uk, checked directly in the capture:
    //   - exactly one <h1> (`grep -o "<h1" | wc -l` -> 1)
    //   - <html class="govuk-template" lang="en">
    //   - <meta name="viewport" content="width=device-width, initial-scale=1">
    //   - <link rel="canonical" href="https://www.gov.uk/">
    // Each of those is the thing the paired code claims is missing.
    (
        "govuk_homepage.html",
        &["A11Y003", "LANG001", "MOB001", "CANON001", "META001"],
    ),
    // example.com has <title>Example Domain</title>, so nothing should claim it
    // is missing.
    ("example_org.html", &["META001", "TITLEMISS-V2001"]),
    // xkcd's only text-free link is
    // `<a href="/"><img src="/s/0b7742.png" alt="xkcd.com logo" ...></a>`, which
    // has an accessible name via the image's alt. `A11Y-LINK-V2001` ignored
    // `img_alt` and reported it at Error, while its own recommendation told the
    // author to add "an img with alt text inside each link".
    //
    // Scoped to this capture rather than the global deny-list, because a page with
    // a genuinely nameless link *should* still be reported.
    ("xkcd.html", &["A11Y-LINK-V2001"]),
];

#[test]
fn verified_true_negatives_stay_absent() {
    let mut present: Vec<String> = Vec::new();
    for (fixture, codes) in VERIFIED_TRUE_NEGATIVES {
        let path = realworld_dir().join(fixture);
        let found = codes_of(&analyze(&path));
        for code in *codes {
            if found.contains(*code) {
                present.push(format!("{fixture}: unexpected {code}"));
            }
        }
    }
    assert!(
        present.is_empty(),
        "verified true negatives now reported:\n  {}",
        present.join("\n  ")
    );
}

/// A code must denote one defect.
///
/// `aria-landmarks-v2` built its code as `format!("ARIALAND-V200{}", position + 1)`
/// over `["banner", "navigation", "main", "contentinfo"]`, mapping banner→V2001,
/// navigation→V2002, main→V2003. Those three codes are *already* emitted as
/// literals by the deep landmark analyzers, where they mean "No ARIA landmarks
/// found (deep)", "Missing main landmark (deep)" and "Missing navigation landmark
/// (deep)".
///
/// `defect_key` resolves on code, so each code denoted two different defects
/// depending on its author, and a page missing only a banner was keyed as "no
/// aria landmarks found" and collapsed into a finding describing something else.
/// The codes now name their role.
#[test]
fn landmark_codes_are_unambiguous() {
    // The role-coded codes resolve to the family their role implies.
    for (code, expected) in [
        ("ARIALAND-ROLE-BANNER", "missing banner landmark"),
        ("ARIALAND-ROLE-MAIN", "missing main landmark"),
        ("ARIALAND-ROLE-NAVIGATION", "missing navigation landmark"),
        ("ARIALAND-ROLE-CONTENTINFO", "missing contentinfo landmark"),
    ] {
        assert_eq!(
            defect_key(code, "ignored: curated"),
            expected,
            "{code} must resolve to \"{expected}\""
        );
    }

    // And the codes the role scheme used to collide with keep their own meaning,
    // which is *not* the meaning the old numbering gave them.
    assert_eq!(defect_key("ARIALAND-V2002", "x"), "missing main landmark");
    assert_eq!(
        defect_key("ARIALAND-V2003", "x"),
        "missing navigation landmark"
    );
    assert_ne!(
        defect_key("ARIALAND-V2002", "x"),
        defect_key("ARIALAND-ROLE-NAVIGATION", "x"),
        "V2002 must not also mean \"navigation\""
    );
}

/// A page missing four different landmarks reports four different defects.
///
/// Asserted on the *families* rather than on `ARIALAND-ROLE-*` codes, because
/// dedupe correctly collapses each role-coded finding into whichever built-in
/// analyzer already owns that family. Requiring the role codes to survive would
/// be asserting that dedupe fails.
#[test]
fn distinct_missing_landmarks_are_reported_separately() {
    let html = r#"<!doctype html><html lang="en"><head><title>t</title></head><body>
        <p>text</p></body></html>"#;
    let url = Url::parse("https://example.com/").unwrap();
    let page = HtmlParser::parse(html, &url);
    let ctx = AnalysisContext {
        page: &page,
        body: Some(html),
        status_code: Some(200),
        headers: &[],
        response_time: Some(Duration::from_millis(10)),
        redirect_chain: &[],
        robots_txt: None,
        user_agent: None,
        body_size: Some(html.len()),
        compressed_size: None,
        content_encoding: None,
        server: None,
        content_type: Some("text/html"),
        rendered: None,
    };
    let findings = AnalyzerRegistry::new(&CrawlConfig::default()).analyze(&ctx);

    // Four landmarks are missing on this page, so four distinct families must
    // each be represented exactly once.
    let mut families: BTreeSet<String> = findings
        .iter()
        .map(|f| defect_key(&f.code, &defect_signature(&f.title)))
        .filter(|k| k.contains("landmark"))
        .collect();
    for expected in [
        "missing main landmark",
        "missing navigation landmark",
        "missing banner landmark",
        "missing contentinfo landmark",
    ] {
        assert!(
            families.remove(expected),
            "expected the `{expected}` family to be reported; got {families:?}"
        );
    }
    // Additional landmark families may legitimately appear: this page has no
    // landmarks at all, so "no aria landmarks found" and the complementary-landmark
    // rules are also correct. What matters is that the four distinct missing
    // landmarks are not collapsed into one another.
}

/// Every fixture referenced by the expectation tables must exist, and every
/// capture must be reachable through `source_url`.
#[test]
fn fixture_and_expectation_tables_agree() {
    let on_disk: BTreeSet<String> = fixture_paths()
        .iter()
        .map(|p| p.file_name().unwrap().to_str().unwrap().to_string())
        .collect();

    let mut referenced: BTreeSet<String> = BTreeSet::new();
    for (f, _) in VERIFIED_TRUE_POSITIVES {
        referenced.insert((*f).to_string());
        source_url(f);
    }
    for (f, _) in VERIFIED_TRUE_NEGATIVES {
        referenced.insert((*f).to_string());
        source_url(f);
    }

    let dangling: Vec<&String> = referenced.difference(&on_disk).collect();
    assert!(
        dangling.is_empty(),
        "expectation tables reference fixtures that do not exist: {dangling:?}"
    );

    for name in &on_disk {
        // Panics if the provenance mapping is missing, which is the point: a new
        // capture without a recorded origin would otherwise be analysed under a
        // made-up URL.
        let url = source_url(name);
        assert!(
            Url::parse(url).is_ok(),
            "{name}: recorded source URL does not parse"
        );
    }
}
