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
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / "crates" / "crawlkit-engine" / "src"
DOC = ROOT / "docs" / "FINDING_CODES.md"

IMPL_RE = re.compile(r"impl\s+Analyzer\s+for\s+(\w+)\s*\{")
CODE_RE = re.compile(r'code:\s*"([A-Z][A-Z0-9-]+)"')
TITLE_RE = re.compile(r'title:\s*"([^"]*)"')
TEST_SPLIT = re.compile(r"#\[cfg\(test\)\]|mod\s+tests")

# Codes deliberately shared by two registered analyzers, with the owning
# convention (ANALYZER_AUDIT Phase 4: generation suffixes identify
# ownership). Anything NOT in this list that becomes shared is a gate
# failure — namespace it (e.g. -DEEP, -DEEP-DEEP, -VALIDATOR, -SCHEMA)
# or consolidate the analyzers.
KNOWN_SHARED: dict[str, str] = {
    "COOKIEHTTP001": "base CookieHttpOnlyFlagValidator owns the code; the exact-duplicate deep-deep emitter was unregistered 2026-09-27 (impl remains exported)",
    "COOKIESEC001": "base CookieSecurityFlagAnalyzer owns the code; the duplicate deep-deep emitter was unregistered 2026-09-27 (impl remains exported)",
    "EXTLINKAUTH-V2001": "external link authority deep vs deep-deep validator",
    "FORMLAB-V2001": "form label V2 analyzer vs deep validator",
    "HSTSPR-V2001": "HSTS preload V2 analyzer vs deep-deep validator",
    "HSTSPR001": "HSTS preload analyzer vs deep validator",
    "INTLINKQ-V2001": "internal link quality V2 vs deep validator (branch A)",
    "INTLINKQ-V2002": "internal link quality V2 vs deep validator (branch B)",
    "SITEMAPDEEP-V2001": "sitemap coverage deep vs deep-deep validator",
    "TBLCAP-V2001": "table caption V2 analyzer vs deep validator",
    "TBLSCOP-V2001": "table header scope V2 analyzer vs deep validator",
    "XFODEEP-V2001": "X-Frame-Options deep vs deep-deep validator",
}


def impl_spans(body: str) -> list[tuple[int, int, str]]:
    """Return (start, end, type_name) for every `impl Analyzer for X { … }`."""
    spans: list[tuple[int, int, str]] = []
    for im in IMPL_RE.finditer(body):
        depth, i = 0, im.end() - 1
        while i < len(body):
            if body[i] == "{":
                depth += 1
            elif body[i] == "}":
                depth -= 1
                if depth == 0:
                    break
            i += 1
        spans.append((im.start(), i + 1, im.group(1)))
    return spans


def collect() -> tuple[dict[str, dict[str, list[tuple[str, str]]]], int]:
    """Return ({code: {analyzer: [(file, title), …]}}, analyzer_impl_count)."""
    catalog: dict[str, dict[str, list[tuple[str, str]]]] = {}
    impl_count = 0
    for path in sorted(SRC.rglob("*.rs")):
        src = path.read_text()
        m = TEST_SPLIT.search(src)
        body = src[: m.start()] if m else src
        spans = impl_spans(body)
        impl_count += len(spans)
        rel = path.relative_to(ROOT).as_posix()
        for cm in CODE_RE.finditer(body):
            for s, e, name in spans:
                if s < cm.start() < e:
                    tm = TITLE_RE.search(body, cm.end(), cm.end() + 400)
                    entry = (rel, tm.group(1) if tm else "(no title literal)")
                    catalog.setdefault(cm.group(1), {}).setdefault(name, []).append(entry)
                    break
    return catalog, impl_count


def render(catalog: dict[str, dict[str, list[tuple[str, str]]]], impl_count: int) -> str:
    shared = {c: owners for c, owners in catalog.items() if len(owners) > 1}
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
        f"| Distinct finding codes | {len(catalog)} |",
        f"| `impl Analyzer for` blocks scanned | {impl_count} |",
        f"| Codes shared by 2+ analyzers | {len(shared)} |",
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
    for code in sorted(catalog):
        owners = catalog[code]
        names = sorted(owners)
        owner_cell = ", ".join(f"`{n}`" for n in names)
        files = sorted({f for n in names for f, _ in owners[n]})
        file_cell = ", ".join(f"`{f}`" for f in files)
        title = owners[names[0]][0][1]
        lines.append(f"| `{code}` | {owner_cell} | {file_cell} | {title} |")
    lines.append("")
    return "\n".join(lines)


def main() -> int:
    check = "--check" in sys.argv[1:]
    catalog, impl_count = collect()
    shared = {c: owners for c, owners in catalog.items() if len(owners) > 1}
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

    content = render(catalog, impl_count)
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
        print(
            f"wrote {DOC.relative_to(ROOT)}: {len(catalog)} codes, "
            f"{len(shared)} shared, {impl_count} impl blocks"
        )

    for f in failures:
        print(f"FAIL: {f}", file=sys.stderr)
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
