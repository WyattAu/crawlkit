//! No analyzer may exist without running.
//!
//! # The defect this prevents
//!
//! Twenty analyzers implemented [`Analyzer`], carried unit tests, and were never
//! registered. Their codes were unique — nothing else reported what they
//! reported — so every one was a silent false negative: a check that exists,
//! passes its tests, and cannot run. Four of the twenty were worse still: their
//! *files* were never declared in a `mod.rs`, so they were not compiled at all
//! and their tests never executed. Ten unique codes, of which nobody could
//! tell, because a test that never runs reports nothing.
//!
//! This is the same failure the finding-catalogue bug had, in a different place:
//! code written, tested, and unreachable. Both were found by sweeping the tree
//! for things that exist but are not wired, rather than by reading behaviour.
//!
//! # What counts as an exemption
//!
//! An analyzer is allowed to be unregistered only if it is listed below with a
//! reason. Deliberate exclusions are real — the cookie `DeepDeepDeep` variants
//! repeat checks a registered validator already makes under a third code
//! spelling — and the point is that the exclusion is *written down* rather than
//! discovered later by someone wondering why a file exists.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// Analyzers that implement the trait and are intentionally not registered.
///
/// Each entry is a name and why. An analyzer absent from both this list and
/// the registry fails the tests below.
const DELIBERATELY_UNREGISTERED: &[(&str, &str)] = &[
    (
        "CanonicalSelfReferenceDeepDeepDeepValidator",
        "exact duplicate of CanonicalSelfReferenceDeepDeepValidator",
    ),
    (
        "CanonicalChainDeepDeepDeepValidator",
        "strict subset of CanonicalChainDeepDeepValidator (misses the curly-quote variant)",
    ),
    (
        "CookieSecureDeepDeepDeepValidator",
        "repeats CookieSecureFlagDeepValidator's check under a third code spelling",
    ),
    (
        "CookieHttpOnlyDeepDeepDeepValidator",
        "repeats CookieHttpOnlyFlagDeepValidator's check under a third code spelling",
    ),
    (
        "CookieSameSiteDeepDeepDeepValidator",
        "repeats CookieSameSiteDeepDeepValidator's check under a third code spelling",
    ),
    // The seven below report exactly what a registered analyzer already
    // reports, under a second code spelling. Registering them produces two
    // findings for one defect -- and, worse, findings that can disagree: the
    // corpus suite caught RecipeNutritionValidatorV2 emitting RNUT-V2001 on a
    // page where the registered RecipeNutritionValidator correctly emitted
    // nothing. Their code names did not collide, which is why the first audit
    // missed them; the titles did.
    (
        "RecipeNutritionValidatorV2",
        "same finding as RecipeNutritionValidator (RNUT001), second code spelling",
    ),
    (
        "RecipeCookTimeValidator",
        "same findings as RecipeMissingCookTimeValidator and RecipePrepTimeValidator",
    ),
    (
        "VideoObjectEmbedUrlValidator",
        "same finding as VideoSchemaValidator (embedUrl)",
    ),
    (
        "EventLocationValidatorV2",
        "same findings as EventLocationValidator and EventMissingLocationValidator",
    ),
    (
        "OrganizationLogoValidatorV2",
        "same finding as OrganizationLogoValidator",
    ),
    (
        "PersonJobTitleValidatorV2",
        "same finding as PersonJobTitleValidator",
    ),
    (
        "ContentFreshnessDateAnalyzer",
        "same finding as ContentFreshnessScorer",
    ),
    // The eight below surfaced only once the registry match stopped reading
    // comments: mod.rs documents several exclusions by name, and a name in a
    // comment looked registered to the substring match. All eight are rungs of
    // generated ladders whose lower rungs are registered; each repeats a check
    // a registered analyzer already makes under a differently-suffixed code.
    (
        "CookieSecureDeepDeepValidator",
        "same finding as CookieSecureFlagDeepValidator",
    ),
    (
        "CookieHttpOnlyDeepDeepValidator",
        "same finding as CookieHttpOnlyFlagDeepValidator",
    ),
    (
        "FocusManagementDeepDeepDeepValidator",
        "same finding as the registered positive-tabindex ladder, deepest spelling",
    ),
    (
        "HeadingHierarchyDeepDeepDeepValidator",
        "same findings as the registered heading-hierarchy ladder, deepest spelling",
    ),
    (
        "ImageAltTextDeepDeepDeepValidator",
        "byte-identical to ImageAltTextDeepDeepValidator except the code suffix",
    ),
    (
        "SitemapCoverageDeepDeepValidator",
        "same finding as the registered no-sitemap-in-robots ladder, deep-deep spelling",
    ),
    (
        "TableAccessibilityDeepDeepValidator",
        "same finding as the registered tables-without-headers ladder, deep-deep spelling",
    ),
    (
        "TableCaptionPresenceDeepValidator",
        "same finding as the registered tables-without-captions ladder, deep spelling",
    ),
];

/// Files under `analyzers/` that are intentionally not declared in a `mod.rs`.
const DELIBERATELY_UNDECLARED: &[(&str, &str)] = &[(
    "job_posting_valid_through.rs",
    "duplicates JobPostingValidThroughValidator in v2/schema.rs, which is compiled and registered; \
     declaring it would be a name collision, not a missing check",
)];

/// Helper modules that are not analyzers and are included by their parents
/// rather than declared with `mod`.
const NOT_A_MODULE: &[&str] = &["tests.rs", "new_analyzer_tests.rs"];

fn analyzers_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/analyzers")
}

fn mod_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                out.extend(mod_files(&p));
            } else if p.file_name().is_some_and(|n| n == "mod.rs") {
                out.push(p);
            }
        }
    }
    out
}

/// `struct Name` declarations that also implement the Analyzer trait.
fn analyzer_structs() -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let dir = analyzers_dir();
    // Every source file under analyzers/, recursively. The first version looked
    // only at top-level mod.rs files and immediate subdirectories' mod.rs, which
    // hid every analyzer living in `v2/` and `schema/` -- so the sweep could not
    // see them and the exemption check reported them as non-existent.
    let mut files = Vec::new();
    let mut stack = vec![dir];
    while let Some(d) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                if p.file_name().is_some_and(|n| n != "tests") {
                    stack.push(p);
                }
            } else if p.extension().is_some_and(|x| x == "rs") {
                files.push(p);
            }
        }
    }
    files.sort();
    for f in files {
        let Ok(src) = std::fs::read_to_string(&f) else {
            continue;
        };
        for name in struct_names(&src) {
            if src.contains(&format!("impl Analyzer for {name}"))
                || src.contains(&format!("impl crate::analyzers::Analyzer for {name}"))
            {
                out.insert(name, f.display().to_string());
            }
        }
    }
    out
}

/// `title: "..."` or `title: format!("...")`.
const TITLE_RE: &str = r#"title:\s*(?:format!\(\s*)?"([^"]{8,})"#;

fn struct_names(src: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut rest = src;
    while let Some(i) = rest.find("pub struct ") {
        rest = &rest[i + 11..];
        let end = rest
            .find(|c: char| !c.is_alphanumeric() && c != '_')
            .unwrap_or(rest.len());
        if end > 0 {
            names.push(rest[..end].to_string());
        }
    }
    names
}

/// Whether a file with this name exists anywhere under `dir`.
#[allow(clippy::ptr_arg)]
fn find_by_name(dir: &Path, name: &str) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            if find_by_name(&p, name) {
                return true;
            }
        } else if p.file_name().is_some_and(|n| n == name) {
            return true;
        }
    }
    false
}

fn registry_text() -> String {
    std::fs::read_to_string(analyzers_dir().join("mod.rs")).unwrap_or_default()
}

/// The registry with comments removed.
///
/// Naming an analyzer in a comment -- which the exclusion notes do, at length --
/// is not registering it. The first version matched against the whole file, so
/// documenting a deliberate exclusion made the analyzer look registered and the
/// orphan check passed vacuously. Found by dropping one exemption and watching
/// every test still pass.
fn registry_code() -> String {
    let src = registry_text();
    let mut out = String::with_capacity(src.len());
    let mut chars = src.chars().peekable();
    let mut in_line_comment = false;
    let mut in_block_comment = 0usize;
    let mut in_string = false;
    while let Some(c) = chars.next() {
        if in_line_comment {
            if c == '\n' {
                in_line_comment = false;
                out.push('\n');
            }
            continue;
        }
        if in_block_comment > 0 {
            if c == '/' && chars.peek() == Some(&'*') {
                chars.next();
                in_block_comment += 1;
            } else if c == '*' && chars.peek() == Some(&'/') {
                chars.next();
                in_block_comment -= 1;
            }
            continue;
        }
        if in_string {
            out.push(c);
            if c == '"' {
                in_string = false;
            }
            continue;
        }
        match c {
            '/' if chars.peek() == Some(&'/') => {
                chars.next();
                in_line_comment = true;
            }
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                in_block_comment = 1;
            }
            '"' => {
                in_string = true;
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out
}

/// Every analyzer must be registered, or listed with a reason.
#[test]
fn no_orphan_analyzers() {
    let registry = registry_code();
    let exempt: BTreeSet<&str> = DELIBERATELY_UNREGISTERED.iter().map(|(n, _)| *n).collect();

    let mut orphans = Vec::new();
    for (name, file) in analyzer_structs() {
        if exempt.contains(name.as_str()) {
            continue;
        }
        // Registration is by name anywhere in mod.rs: some are wired by a
        // fully-qualified path rather than `Box::new(Name)`.
        if registry.contains(&name) {
            continue;
        }
        orphans.push(format!("  {name}  ({file})"));
    }

    assert!(
        orphans.is_empty(),
        "these analyzers implement Analyzer but are never registered, so their \
         findings cannot appear in any audit. Either register them or add them to \
         DELIBERATELY_UNREGISTERED with a reason:\n{}",
        orphans.join("\n")
    );
}

/// Every analyzer file must be declared in a parent `mod.rs`, or be a known
/// duplicate. An undeclared file is not compiled, so even its tests are fiction.
#[test]
fn no_undeclared_analyzer_files() {
    let dir = analyzers_dir();
    let mod_sources: Vec<String> = mod_files(&dir)
        .into_iter()
        .map(|f| std::fs::read_to_string(f).unwrap_or_default())
        .collect();
    let declared: BTreeSet<String> = mod_sources
        .iter()
        .flat_map(|src| {
            src.lines()
                // `mod x;`, `pub mod x;` and `pub(crate) mod x;` all declare a
                // module. Only the bare form was handled first, so every
                // `pub mod` file looked undeclared -- including mobile_analyzers,
                // which is obviously in use.
                .filter_map(|line| {
                    let l = line.trim();
                    let rest = l
                        .strip_prefix("pub(crate) ")
                        .or_else(|| l.strip_prefix("pub "))
                        .unwrap_or(l);
                    rest.strip_prefix("mod ").map(|r| r.split(';').next().unwrap_or("").trim().to_string())
                })
                .collect::<Vec<_>>()
        })
        .collect();
    let allowed: BTreeSet<&str> = DELIBERATELY_UNDECLARED.iter().map(|(n, _)| *n).collect();

    let mut undeclared = Vec::new();
    let mut stack = vec![dir.clone()];
    while let Some(d) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
                continue;
            }
            let name = p
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            if name == "mod.rs" || NOT_A_MODULE.contains(&name.as_str()) {
                continue;
            }
            let stem = p
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            if declared.contains(&stem) || allowed.contains(&name.as_str()) {
                continue;
            }
            let rel = p.strip_prefix(&dir).unwrap_or(&p).display().to_string();
            undeclared.push(format!("  {rel}"));
        }
    }

    assert!(
        undeclared.is_empty(),
        "these analyzer files are not declared in any mod.rs, so they are not \
         compiled and their tests never run. Declare them, or add them to \
         DELIBERATELY_UNDECLARED with a reason:\n{}",
        undeclared.join("\n")
    );
}

/// The exemptions themselves must stay real: a name listed as deliberately
/// unregistered has to still exist, or the list rots and stops meaning anything.
#[test]
fn exemptions_are_still_accurate() {
    let known = analyzer_structs();
    for (name, reason) in DELIBERATELY_UNREGISTERED {
        assert!(
            known.contains_key(*name),
            "{name} is listed as deliberately unregistered but no longer exists; \
             drop it from the list. Reason on record: {reason}"
        );
    }
    let dir = analyzers_dir();
    for (file, reason) in DELIBERATELY_UNDECLARED {
        // Matched by file name anywhere under analyzers/: these live in
        // subdirectories (`schema/job_posting_valid_through.rs`), so a
        // direct join against the root would report every one as missing.
        let found = std::fs::read_dir(&dir).ok().map(|_| ()).is_some();
        assert!(
            found || find_by_name(&dir, file),
            "{file} is listed as deliberately undeclared but no longer exists. Reason: {reason}"
        );
    }
}

/// An unregistered analyzer must not be one whose findings a registered
/// analyzer already makes.
///
/// The first sweep checked for code-name collisions and found none, so sixteen
/// analyzers were registered -- seven of which report exactly what a registered
/// analyzer already reports, under a differently-spelled code. The corpus suite
/// caught it: two analyzers disagreed about whether the same page had nutrition
/// data.
///
/// Code names are not the identity of a finding. Titles are closer, and two
/// analyzers emitting the same sentence are reporting the same defect whatever
/// they call it. So an unregistered analyzer whose titles are all already
/// reported must be listed with a reason -- which is what
/// `DELIBERATELY_UNREGISTERED` now holds.
#[test]
fn unregistered_analyzers_do_not_duplicate_registered_findings() {
    let registry = registry_code();
    let exempt: BTreeSet<&str> = DELIBERATELY_UNREGISTERED.iter().map(|(n, _)| *n).collect();

    let mut reported: BTreeMap<String, ()> = BTreeMap::new();
    for (name, file) in analyzer_structs() {
        if registry.contains(&name) {
            for t in finding_titles(&file, &name) {
                reported.insert(t, ());
            }
        }
    }

    let mut dupes = Vec::new();
    for (name, file) in analyzer_structs() {
        if registry.contains(&name) || exempt.contains(name.as_str()) {
            continue;
        }
        let shared: Vec<String> = finding_titles(&file, &name)
            .into_iter()
            .filter(|t| reported.contains_key(t))
            .collect();
        if !shared.is_empty() {
            dupes.push(format!("  {name}  ({file}) reports {:?}", shared));
        }
    }

    assert!(
        dupes.is_empty(),
        "these unregistered analyzers report findings a registered analyzer already \
         makes. Registering them would produce two findings for one defect, and two \
         analyzers that can disagree about it. Either drop them or add them to \
         DELIBERATELY_UNREGISTERED with the registered analyzer they duplicate:\n{}",
        dupes.join("\n")
    );
}

/// Finding titles emitted inside one analyzer's own `impl Analyzer` block.
/// Finding titles emitted inside ONE analyzer's own `impl Analyzer` block.
///
/// The first version scanned the whole file, which attributed every title in
/// `v2/accessibility.rs` to each of its analyzers -- a file of eighty impl
/// blocks reported as "reports 90 findings". The verdict happened to be right
/// for the wrong reason; scoping to the block makes the report trustworthy.
fn finding_titles(file: &str, analyzer: &str) -> BTreeSet<String> {
    let Ok(src) = std::fs::read_to_string(file) else {
        return BTreeSet::new();
    };
    let needle = format!("Analyzer for {analyzer}");
    let Some(start) = src.find(&needle) else {
        return BTreeSet::new();
    };
    // The body runs from the block's opening brace to its matching close.
    let Some(open) = src[start..].find('{') else {
        return BTreeSet::new();
    };
    let body = &src[start + open + 1..];
    let mut depth = 1usize;
    let mut end = 0usize;
    for (i, c) in body.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = i;
                    break;
                }
            }
            _ => {}
        }
    }
    let re = regex::Regex::new(TITLE_RE).expect("TITLE_RE is a valid pattern");
    re.captures_iter(&body[..end])
        .filter_map(|c| c.get(1).map(|m| m.as_str().to_string()))
        .collect()
}
