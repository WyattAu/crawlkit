#!/usr/bin/env python3
"""Generate the finding-code catalog from analyzer source and enforce drift.

Phase 4 of docs/ANALYZER_AUDIT.md left two debts, both closed here:

1. **Static cross-analyzer code collisions** (the runtime fixture guard
   `test_registry_finding_codes_unique_on_fixture` can only catch
   collisions reachable from one fixture). This script attributes every
   finding-code emit site to its owning `impl Analyzer for <Type>` block
   via brace matching and fails when a code is newly shared by two
   analyzers without being recorded as known.

2. **A generated finding-code catalog** (docs/FINDING_CODES.md) so
   analyzer/finding counts come from the registry source instead of
   hand-maintained docs (the COMPETITIVE_MATRIX action: "generate
   README/catalog numbers from the registry in CI").

Usage:
    python3 scripts/generate_finding_catalog.py            # regenerate docs/FINDING_CODES.md
    python3 scripts/generate_finding_catalog.py --check    # gate: fail if the doc is stale or a new code is shared
    python3 scripts/generate_finding_catalog.py --self-test  # scanner edge cases (no repo access needed)
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / "crates" / "crawlkit-engine" / "src"
DOC = ROOT / "docs" / "FINDING_CODES.md"

# Matches any trait implementation, not only `impl Analyzer for`. The twenty
# cross-page analyzers implement `PostCrawlAnalyzer`, and their emit sites were
# therefore attributed to no analyzer at all: their codes fired in production
# while the catalog recorded nothing, so `--check` could not flag a new
# cross-page code as unrecorded -- the exact failure this script exists to
# prevent.
IMPL_RE = re.compile(r"impl\s+(?:[A-Za-z0-9_]+\s+for\s+)?([A-Za-z0-9_]+)\s*\{")
CODE_RE = re.compile(r'code:\s*"([A-Z][A-Z0-9-]+)"')
TITLE_RE = re.compile(r'title:\s*"([^"]*)"')
CHAR_RE = re.compile(r"'(\\.|[^'\\])'")
# A `#[cfg(test)]` item was where test code USED to be assumed to start, and
# everything from the first one to EOF was dropped. That is wrong whenever a
# test module sits mid-file: content_analyzers.rs has `#[cfg(test)] mod
# schema_context_tests` at line 2078 of 10481, and 135 emitted codes across
# ten files -- including live production analyzers -- were invisible to the
# catalog as a result. RNUT-V2001, whose duplicate registration the corpus
# suite caught, could not be gated against exactly because the scanner could
# not see it. Test items are now blanked individually (see
# test_item_spans()); production code before, between, and after them is read.
TEST_ITEM_RE = re.compile(r"#\[cfg\(test\)\]")

# Codes deliberately shared by two registered analyzers, with the owning
# convention (ANALYZER_AUDIT Phase 4: generation suffixes identify
# ownership). Anything NOT in this list that becomes shared is a gate
# failure — namespace it (e.g. -DEEP, -DEEP-DEEP, -VALIDATOR, -SCHEMA)
# or consolidate the analyzers.
KNOWN_SHARED: dict[str, str] = {
    "COOKIEHTTP001": "base CookieHttpOnlyFlagValidator owns the code; the exact-duplicate deep-deep emitter was unregistered 2026-09-27 (impl remains exported)",
    "COOKIESEC001": "base CookieSecurityFlagAnalyzer owns the code; the duplicate deep-deep emitter was unregistered 2026-09-27 (impl remains exported)",
    "SITEMAPDEEP-V2001": "SitemapCoverageDeepAnalyzerV2 owns the code; the exact-duplicate deep-deep emitter was unregistered 2026-09-27 (impl remains exported)",
    "TBLCAP-V2001": "TableCaptionPresenceAnalyzerV2 owns the code; the exact-duplicate deep emitter was unregistered 2026-09-27 (impl remains exported)",
    "TBLSCOP-V2001": "complementary, not duplicates: TableHeaderScopeAnalyzerV2 fires on <th> elements lacking scope attributes; TableHeaderScopeDeepValidator fires on pages whose tables have no header cells at all (mutually exclusive preconditions)",
    "XFODEEP-V2001": "complementary, not duplicates: XFrameOptionsDeepAnalyzerV2 fires when neither X-Frame-Options nor CSP frame-ancestors is present; XFrameOptionsDeepDeepValidator fires when the header exists but carries an invalid value (mutually exclusive preconditions, pinned by fixture)",
}


def _blank(body: str, out: list[str], start: int, end: int) -> None:
    """Blank out[start:end] in the sanitized buffer, keeping newlines (and
    therefore all later offsets) intact."""
    for k in range(start, min(end, len(body))):
        if out[k] != "\n":
            out[k] = " "


def _find_raw_end(body: str, hashes_start: int, quote: int) -> int:
    """End (exclusive) of a raw string whose opening quote is at `quote` with
    `hashes_start` pointing at the first `#` of its hash run."""
    hashes = quote - hashes_start
    end = body.find('"' + "#" * hashes, quote + 1)
    return len(body) if end == -1 else end + 1 + hashes


def sanitize(body: str) -> str:
    """Blank string/char literals and comments, preserving every offset.

    String contents (ordinary, raw `r#"…"#`, byte `b"…"`, `br#"…"#`) and
    comment text (line, block — including nested block comments, and doc
    comments) are replaced with spaces so that brace matching sees only
    structural tokens. Newlines are kept, so every index into the
    returned text indexes into the original body.

    Two classes of mis-scan motivated this pass:

    - an unbalanced `{` inside a comment (`// a { color: #fff`) made a
      naive brace counter never close the enclosing impl block, which
      then swallowed every following impl in the file and mis-attributed
      their finding codes (e.g. all `v2/accessibility.rs` deep codes to
      `ColorContrastLinkDeepValidator`);
    - char literals such as `('"',)` inside tuple arrays ended a naive
      string scan early/at the wrong place, corrupting spans (e.g.
      `MIXSCR001` attributed to `MixedContentFormValidator`).

    Doc comments are blanked too, so Rust examples inside `///` no longer
    produce phantom `impl Analyzer for` blocks or emit sites.
    """
    out = list(body)
    n = len(body)
    i = 0
    while i < n:
        c = body[i]
        if c == "/" and i + 1 < n:
            if body[i + 1] == "/":
                j = body.find("\n", i)
                j = n if j == -1 else j
                _blank(body, out, i, j)
                i = j
                continue
            if body[i + 1] == "*":
                depth, k = 1, i + 2
                while k + 1 < n and depth:
                    if body[k] == "/" and body[k + 1] == "*":
                        depth += 1
                        k += 2
                        continue
                    if body[k] == "*" and body[k + 1] == "/":
                        depth -= 1
                        k += 2
                        continue
                    k += 1
                k = min(k, n)
                _blank(body, out, i, k)
                i = k
                continue
        if c == "r" and (i == 0 or not (body[i - 1].isalnum() or body[i - 1] == "_")):
            j = i + 1
            while j < n and body[j] == "#":
                j += 1
            if j < n and body[j] == '"':
                end = _find_raw_end(body, i + 1, j)
                _blank(body, out, i, end)
                i = end
                continue
        if c == "b" and (i == 0 or not (body[i - 1].isalnum() or body[i - 1] == "_")):
            j = i + 1
            if j < n and body[j] == "r":
                k = j + 1
                while k < n and body[k] == "#":
                    k += 1
                if k < n and body[k] == '"':
                    end = _find_raw_end(body, j + 1, k)
                    _blank(body, out, i, end)
                    i = end
                    continue
            if j < n and body[j] == '"':
                k = j + 1
                while k < n:
                    if body[k] == "\\":
                        k += 2
                        continue
                    if body[k] == '"':
                        break
                    k += 1
                end = min(k + 1, n)
                _blank(body, out, i, end)
                i = end
                continue
        if c == '"':
            k = i + 1
            while k < n:
                if body[k] == "\\":
                    k += 2
                    continue
                if body[k] == '"':
                    break
                k += 1
            end = min(k + 1, n)
            _blank(body, out, i, end)
            i = end
            continue
        if c == "'":
            m = CHAR_RE.match(body, i)
            if m:
                _blank(body, out, i, m.end())
                i = m.end()
                continue
        i += 1
    return "".join(out)


def _match_brace(body: str, open_brace: int) -> int:
    """Index of the brace closing the one at `open_brace` (must be sanitized
    text: braces inside strings or comments would break the count)."""
    depth, i, n = 0, open_brace, len(body)
    while i < n:
        if body[i] == "{":
            depth += 1
        elif body[i] == "}":
            depth -= 1
            if depth == 0:
                return i
        i += 1
    return n - 1


def test_item_spans(clean: str) -> list[tuple[int, int]]:
    """Spans of `#[cfg(test)]` items in sanitized source.

    The item after the attribute is brace-matched when it has a body
    (`mod`, `fn`, `impl`, …) and run to the terminator when it does not
    (`use`, `include!`). Blanking these spans -- and only these spans --
    removes test code wherever it sits while keeping every offset valid.
    """
    spans: list[tuple[int, int]] = []
    n = len(clean)
    for m in TEST_ITEM_RE.finditer(clean):
        j = m.end()
        while j < n:
            while j < n and clean[j] in " \t\n":
                j += 1
            if clean.startswith("//", j):
                k = clean.find("\n", j)
                j = n if k == -1 else k
                continue
            if clean.startswith("/*", j):
                k = clean.find("*/", j)
                j = n if k == -1 else k + 2
                continue
            if clean.startswith("#[", j):
                k = clean.find("]", j)
                j = k + 1 if k != -1 else n
                continue
            break
        if j < n and (clean.startswith("mod", j) or clean.startswith("fn", j)
                      or clean.startswith("impl", j)):
            b = clean.find("{", j)
            end = _match_brace(clean, b) + 1 if b != -1 else j
        else:
            semi = clean.find(";", j)
            nl = clean.find("\n", j)
            end = min(x for x in (semi, nl, n) if x != -1) + 1
        spans.append((m.start(), min(end, n)))
    return spans


def blank_spans(body: str, spans: list[tuple[int, int]]) -> str:
    """Blank `spans` in `body`, keeping newlines so offsets stay aligned."""
    out = list(body)
    for s, e in spans:
        _blank(body, out, s, e)
    return "".join(out)


def impl_spans(body: str) -> list[tuple[int, int, str]]:
    """Return (start, end, type_name) for every `impl Analyzer for X { … }`.

    `body` must already be `sanitize()`d: braces inside string literals,
    char literals, or comments never reach this function, so plain
    counting is exact.
    """
    spans: list[tuple[int, int, str]] = []
    n = len(body)
    for im in IMPL_RE.finditer(body):
        depth, i = 0, im.end() - 1
        while i < n:
            if body[i] == "{":
                depth += 1
            elif body[i] == "}":
                depth -= 1
                if depth == 0:
                    break
            i += 1
        spans.append((im.start(), i + 1, im.group(1)))
    return spans


def registered_analyzers() -> set[str]:
    """Names wired into a registry via `Box::new(<path>::Name::new(...))`.

    Read from the two registration sites, with comments and string
    literals blanked first: mod.rs documents deliberate exclusions by
    name, and a name in a comment is not a registration -- trusting the
    raw text made eight unregistered analyzers look live (the same
    failure `tests/no_orphan_analyzers.rs` guards against on the Rust
    side; the two definitions must agree).

    `tests/…` registration helpers are excluded because collect() reads
    the same test-split bodies the catalog is built from.
    """
    names: set[str] = set()
    reg_re = re.compile(r"Box::new\(\s*(?:[A-Za-z0-9_]+::)*([A-Za-z0-9_]+)::new\b")
    for path in (
        SRC / "analyzers" / "mod.rs",
        SRC / "analyzers" / "post_crawl_analyzers.rs",
    ):
        src = path.read_text()
        clean = sanitize(src)
        body = blank_spans(clean, test_item_spans(clean))
        names.update(reg_re.findall(body))
    return names


def collect() -> tuple[dict[str, dict[str, list[tuple[str, str]]]], int]:
    """Return ({code: {analyzer: [(file, title), …]}}, analyzer_impl_count)."""
    catalog: dict[str, dict[str, list[tuple[str, str]]]] = {}
    impl_count = 0
    for path in sorted(SRC.rglob("*.rs")):
        src = path.read_text()
        clean = sanitize(src)
        # Blank test items in both views at identical offsets: emit sites are
        # read from the raw text (string literals survive there) while impl
        # spans are matched on the sanitized text. Blank -- not truncate --
        # because production code lives before, between, and after test
        # modules in most analyzer files.
        test_spans = test_item_spans(clean)
        body = blank_spans(src, test_spans)
        clean = blank_spans(clean, test_spans)
        spans = impl_spans(clean)
        impl_count += len(spans)
        rel = path.relative_to(ROOT).as_posix()
        # Emit sites (string literals) live in the raw body; spans index
        # into the same offsets because sanitize() preserves them.
        for cm in CODE_RE.finditer(body):
            for s, e, name in spans:
                if s < cm.start() < e:
                    tm = TITLE_RE.search(body, cm.end(), cm.end() + 400)
                    entry = (rel, tm.group(1) if tm else "(no title literal)")
                    catalog.setdefault(cm.group(1), {}).setdefault(name, []).append(entry)
                    break
    return catalog, impl_count


def render(
    catalog: dict[str, dict[str, list[tuple[str, str]]]],
    impl_count: int,
    registered: set[str],
) -> str:
    """Render the catalog, splitting codes by whether they can fire.

    A code whose every emitter is unregistered cannot appear in any
    audit. Documenting it alongside live codes told users crawlkit
    checks something it does not check -- the same fiction as shipping
    an unregistered analyzer. Until the orphan tripwire existed, the
    catalog carried dozens of such codes (both spellings of every
    deliberately-excluded ladder rung); they are still listed, in their
    own section, so the knowledge that the spelling exists survives.
    """
    live: dict[str, dict[str, list[tuple[str, str]]]] = {}
    never_fire: dict[str, dict[str, list[tuple[str, str]]]] = {}
    for code, owners in catalog.items():
        target = live if any(name in registered for name in owners) else never_fire
        target[code] = owners

    shared = {c: owners for c, owners in live.items() if len(owners) > 1}
    unrecorded = sorted(c for c in shared if c not in KNOWN_SHARED)
    lines: list[str] = [
        "# Finding-Code Catalog",
        "",
        "**GENERATED — do not edit by hand.** Regenerate with:",
        "",
        "```console",
        "$ python3 scripts/generate_finding_catalog.py",
        "```",
        "",
        "The CI gate (`--check`) fails when this file drifts from source or",
        "when a finding code becomes shared by a second analyzer without",
        "being recorded in `KNOWN_SHARED` in",
        "`scripts/generate_finding_catalog.py` (remediation: namespace the",
        "new site per the docs/ANALYZER_AUDIT.md Phase 4 convention, or",
        "consolidate the analyzers).",
        "",
        "| Metric | Value |",
        "|---|---|",
        f"| Distinct finding codes | {len(live)} |",
        f"| `impl Analyzer for` blocks scanned | {impl_count} |",
        f"| Registered analyzers | {len(registered)} |",
        f"| Codes shared by 2+ analyzers | {len(shared)} |",
        f"| Codes with no registered emitter | {len(never_fire)} |",
        "",
        "Only codes with at least one registered emitter are documented",
        "below; a code whose every emitter is unregistered cannot appear",
        "in any audit and is listed separately at the bottom.",
        "",
    ]
    lines += ["## Shared codes (recorded ownership)", ""]
    if shared:
        lines += [
            "| Code | Analyzers | Recorded reason |",
            "|---|---|---|",
        ]
        for code in sorted(shared):
            owners = ", ".join(f"`{a}`" for a in sorted(shared[code]))
            reason = KNOWN_SHARED.get(code, "**UNRECORDED — gate failure**")
            lines.append(f"| `{code}` | {owners} | {reason} |")
    else:
        lines += ["None."]
    lines += ["", "## All codes", "", "| Code | Owner(s) | Source | First title |", "|---|---|---|---|"]
    for code in sorted(live):
        owners = live[code]
        names = sorted(owners)
        owner_cell = ", ".join(f"`{n}`" for n in names)
        files = sorted({f for n in names for f, _ in owners[n]})
        file_cell = ", ".join(f"`{f}`" for f in files)
        title = owners[names[0]][0][1]
        lines.append(f"| `{code}` | {owner_cell} | {file_cell} | {title} |")
    lines += [
        "",
        "## Codes with no registered emitter",
        "",
        "Every analyzer emitting these is deliberately unregistered",
        "(`tests/no_orphan_analyzers.rs` holds the reasons). Each is a",
        "second spelling of a defect a registered analyzer already",
        "reports; none of these codes can appear in an audit.",
        "",
        "| Code | Unregistered owner(s) | Source | First title |",
        "|---|---|---|---|",
    ]
    for code in sorted(never_fire):
        owners = never_fire[code]
        names = sorted(owners)
        owner_cell = ", ".join(f"`{n}`" for n in names)
        files = sorted({f for n in names for f, _ in owners[n]})
        file_cell = ", ".join(f"`{f}`" for f in files)
        title = owners[names[0]][0][1]
        lines.append(f"| `{code}` | {owner_cell} | {file_cell} | {title} |")
    lines.append("")
    return "\n".join(lines)


def self_test() -> int:
    """Verify the scanner against the mis-scan classes that broke attribution.

    Each case is a synthetic source exercising one scoping hazard; the
    assertion is that `impl Analyzer for` spans still close where the
    blocks end and that emit sites land in the right owner. These pin
    the 2026-09-27 hardening (unbalanced brace in a comment swallowed
    every following impl; char literals with embedded quotes corrupted
    string handling); the gate script runs this before --check.
    """
    failures: list[str] = []

    def owner_of(body: str, code: str) -> str | None:
        clean = sanitize(body)
        spans = impl_spans(clean)
        cm = CODE_RE.search(body, 0, len(body))
        while cm is not None and cm.group(1) != code:
            cm = CODE_RE.search(body, cm.end())
        if cm is None:
            return None
        for s, e, name in spans:
            if s < cm.start() < e:
                return name
        return None

    # 1. Unbalanced `{` in a line comment must not swallow the next impl.
    comment_brace = '''
impl Analyzer for A {
    fn f(&self) -> u32 {
        // a { color: #fff  <- unbalanced brace in a comment
        1
    }
}
impl Analyzer for B {
    fn analyze(&self, _ctx: &AnalysisContext) -> Vec<Finding> {
        vec![Finding { code: "X001".to_string(), title: "x" }]
    }
}
'''
    spans = impl_spans(sanitize(comment_brace))
    if [n for _, _, n in spans] != ["A", "B"]:
        failures.append(f"comment-brace: spans {spans}")
    if owner_of(comment_brace, "X001") != "B":
        failures.append("comment-brace: emit site mis-attributed")

    # 2. Char literal with an embedded quote (tuple-array element) must
    #    not corrupt string handling.
    char_quote = '''
impl Analyzer for C {
    fn f(&self) {
        let x = [("src=\\"http://", '"')];
        if x.len() == 1 {
            let _ = 1;
        }
    }
}
impl Analyzer for D {
    fn analyze(&self, _ctx: &AnalysisContext) -> Vec<Finding> {
        vec![Finding { code: "X002".to_string(), title: "x" }]
    }
}
'''
    spans = impl_spans(sanitize(char_quote))
    if [n for _, _, n in spans] != ["C", "D"]:
        failures.append(f"char-quote: spans {spans}")
    if owner_of(char_quote, "X002") != "D":
        failures.append("char-quote: emit site mis-attributed")

    # 3. Raw string with braces + nested block comment must be inert.
    raw_nested = '''
impl Analyzer for E {
    fn f(&self) {
        let s = r#"body { color: red } "#;
        /* /* nested } */ still comment } */
        let _ = 1;
    }
}
impl Analyzer for F {
    fn g(&self) {
        let _ = 2;
    }
}
'''
    spans = impl_spans(sanitize(raw_nested))
    if [n for _, _, n in spans] != ["E", "F"]:
        failures.append(f"raw-nested: spans {spans}")

    # 4. Byte strings and escaped quotes inside ordinary strings.
    byte_escape = '''
impl Analyzer for G {
    fn f(&self) {
        let b = b"}";
        let s = "a\\"}b";
        if s.len() > 0 {
            let _ = b;
        }
    }
}
impl Analyzer for H {
    fn analyze(&self, _ctx: &AnalysisContext) -> Vec<Finding> {
        vec![Finding { code: "X003".to_string(), title: "x" }]
    }
}
'''
    spans = impl_spans(sanitize(byte_escape))
    if [n for _, _, n in spans] != ["G", "H"]:
        failures.append(f"byte-escape: spans {spans}")
    if owner_of(byte_escape, "X003") != "H":
        failures.append("byte-escape: emit site mis-attributed")

    # 5. Doc-comment examples must not create phantom impl blocks (the
       # `MyAnalyzer` incident) nor contribute emit sites.
    doc_phantom = '''
/// Example:
/// impl Analyzer for Ghost {
///     fn analyze(&self, _ctx: &AnalysisContext) -> Vec<Finding> { vec![] }
/// }
pub struct Real;
impl Analyzer for Real {
    fn analyze(&self, _ctx: &AnalysisContext) -> Vec<Finding> {
        vec![]
    }
}
'''
    spans = impl_spans(sanitize(doc_phantom))
    if [n for _, _, n in spans] != ["Real"]:
        failures.append(f"doc-phantom: spans {spans}")

    if failures:
        for f in failures:
            print(f"FAIL: {f}", file=sys.stderr)
        return 1
    print("self-test OK: comment/char/raw/byte/doc scoping (5 cases)")
    return 0


def main() -> int:
    if "--self-test" in sys.argv[1:]:
        return self_test()
    check = "--check" in sys.argv[1:]
    catalog, impl_count = collect()
    registered = registered_analyzers()
    live = {
        c: owners for c, owners in catalog.items() if any(n in registered for n in owners)
    }
    shared = {c: owners for c, owners in live.items() if len(owners) > 1}
    unrecorded = sorted(c for c in shared if c not in KNOWN_SHARED)
    stale_known = sorted(c for c in KNOWN_SHARED if c not in shared)

    failures: list[str] = []
    if unrecorded:
        failures.append(
            "new shared finding code(s) without recorded ownership: "
            + ", ".join(unrecorded)
            + " — namespace the new site per docs/ANALYZER_AUDIT.md Phase 4 "
            "(e.g. -DEEP / -DEEP-DEEP / -VALIDATOR / -SCHEMA suffix) or "
            "consolidate the analyzers, then record it in KNOWN_SHARED."
        )
    if stale_known:
        failures.append(
            "KNOWN_SHARED entries no longer shared (consolidation happened — "
            "remove them): " + ", ".join(stale_known)
        )

    content = render(catalog, impl_count, registered)
    if check:
        if DOC.exists():
            current = DOC.read_text()
            if current != content:
                failures.append(
                    "docs/FINDING_CODES.md is stale — regenerate with "
                    "`python3 scripts/generate_finding_catalog.py` and commit."
                )
        else:
            failures.append("docs/FINDING_CODES.md is missing — generate and commit it.")
    else:
        DOC.write_text(content)
        never_fire = len(catalog) - len(live)
        print(
            f"wrote {DOC.relative_to(ROOT)}: {len(live)} live codes, "
            f"{never_fire} never-fire codes listed separately, "
            f"{len(shared)} shared, {impl_count} impl blocks"
        )

    for f in failures:
        print(f"FAIL: {f}", file=sys.stderr)
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
