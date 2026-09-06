#!/usr/bin/env bash
# verify-release-readiness.sh — read-only pre-tag release gate.
#
# Confirms the release-critical state is coherent before a tag is pushed:
#   1. Version alignment across workspace + satellite manifests + docs
#   1b. The target version is not already tagged at a different commit
#   2. CHANGELOG has a dated section for the target version
#   3. Release workflow YAML parses
#   4. Workspace compiles and the full quality gate passes
#
# Usage: scripts/verify-release-readiness.sh [version]   (default: workspace version)
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

fail() { echo "READY-CHECK FAIL: $*" >&2; exit 1; }
ok()   { echo "  ok: $*"; }

WORKSPACE_VERSION="$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -1)"
VERSION="${1:-$WORKSPACE_VERSION}"

echo "== Release readiness for v${VERSION} =="

# --- 1. Version alignment -----------------------------------------------
[ "$WORKSPACE_VERSION" = "$VERSION" ] || fail "workspace Cargo.toml is ${WORKSPACE_VERSION}, expected ${VERSION}"
ok "workspace Cargo.toml = ${VERSION}"

for crate in crawlkit crawlkit-engine crawlkit-api crawlkit-plugin-sdk crawlkit-types; do
    grep -q 'version.workspace = true' "crates/${crate}/Cargo.toml" \
        || fail "crates/${crate} no longer inherits workspace version — update this check"
done
ok "all workspace crates inherit version"

grep -q "\"version\": \"${VERSION}\"" dashboard/package.json \
    || fail "dashboard/package.json is not ${VERSION}"
ok "dashboard/package.json = ${VERSION}"

grep -q "^\*\*Version:\*\* ${VERSION}$" VERSION.md \
    || fail "VERSION.md Version field is not ${VERSION}"
ok "VERSION.md = ${VERSION}"

if grep -qE '^\*\*Current Phase:.*v[0-9]' VERSION.md; then
    grep -qE "^\*\*Current Phase:.*v${VERSION}" VERSION.md \
        || fail "VERSION.md phase header does not reference v${VERSION}"
else
    ok "VERSION.md uses a non-versioned roadmap phase header"
fi
ok "VERSION.md phase header consistent"

# --- 1b. Target version not already tagged --------------------------------
# Prevents re-tagging an already-released version (the v4.4.1 mislabel): if a
# tag v$VERSION exists anywhere but at HEAD, the release was already shipped
# and the workspace version must be bumped before preparing another one.
if git rev-parse -q --verify "refs/tags/v${VERSION}^{commit}" >/dev/null 2>&1; then
    TAG_COMMIT="$(git rev-parse "refs/tags/v${VERSION}^{commit}")"
    HEAD_COMMIT="$(git rev-parse HEAD)"
    [ "$TAG_COMMIT" = "$HEAD_COMMIT" ] \
        || fail "v${VERSION} is already tagged at ${TAG_COMMIT}; bump the workspace version before preparing this release"
    ok "v${VERSION} tag matches HEAD"
else
    ok "v${VERSION} not yet tagged"
fi

# --- 2. CHANGELOG has a dated section ------------------------------------
grep -qE "^## \[${VERSION}\] - [0-9]{4}-[0-9]{2}-[0-9]{2}" CHANGELOG.md \
    || fail "CHANGELOG.md has no dated [${VERSION}] section"
ok "CHANGELOG.md [${VERSION}] section dated"

if grep -q '^## \[Unreleased\]' CHANGELOG.md; then
    UNRELEASED_LINES="$(sed -n '/^## \[Unreleased\]/,/^## \[/p' CHANGELOG.md | grep -c '^- ' || true)"
    [ "$UNRELEASED_LINES" -eq 0 ] \
        || fail "CHANGELOG [Unreleased] section is non-empty (${UNRELEASED_LINES} items) — fold into [${VERSION}] or defer"
    ok "CHANGELOG [Unreleased] is empty"
fi

# --- 3. Release workflow parses ------------------------------------------
if command -v python3 >/dev/null 2>&1; then
    python3 -c "import yaml,sys; yaml.safe_load(open('.github/workflows/release.yml'))" \
        || fail "release.yml is not valid YAML"
    ok "release.yml parses"
else
    echo "  warn: python3 not available; skipped YAML validation"
fi

grep -q 'Sign checksums' .github/workflows/release.yml \
    || fail "release.yml lost the checksums signing step"
ok "release.yml signs checksums.txt"
grep -q 'Add SBOM checksums' .github/workflows/release.yml \
    || fail "release.yml does not cover SBOM files in checksums.txt"
ok "release.yml covers SBOM checksums"
grep -q 'test -s checksums.txt' .github/workflows/release.yml \
    || fail "release.yml does not reject an empty checksum manifest"
ok "release.yml rejects empty checksum manifests"
grep -q 'sha256sum -c checksums.txt' .github/workflows/release.yml \
    || fail "release.yml does not verify aggregated checksums"
ok "release.yml verifies aggregated checksums"
grep -q 'Smoke test artifact' .github/workflows/release.yml \
    || fail "release.yml has no packaged-artifact smoke test"
ok "release.yml smoke-tests packaged artifacts"

# --- 4. Quality gate -------------------------------------------------------
echo "-- running quality gate (claims, security, fmt, clippy, tests) --"
scripts/verify-roadmap-baseline.sh
ok "roadmap and capability claims"

scripts/verify-unsafe-inventory.sh
ok "unsafe-code inventory"

cargo fmt --all -- --check
ok "cargo fmt --check"

cargo clippy --workspace --all-targets -- -D warnings
ok "cargo clippy -D warnings"

CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}" \
CRAWLKIT_TEST_THREADS="${CRAWLKIT_TEST_THREADS:-1}" bash scripts/verify-contracts.sh
ok "contract and bounded integration tests"

CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}" cargo test --doc --workspace -- --test-threads="${CRAWLKIT_TEST_THREADS:-1}"
ok "cargo test (documentation)"

# --- 5. Dogfood gate (ADR-009) -------------------------------------------
# Network-gated: skipped unless DOGFOOD=1. Runs the production-site crawl
# and prints the triage summary; a human eyeballs severity codes before
# tagging (findings are triaged, not pass/fail — see ADR-009).
if [ "${DOGFOOD:-0}" = "1" ]; then
    echo "-- dogfood crawl (ADR-009) --"
    bash scripts/dogfood.sh || fail "dogfood crawl failed"
    echo "  -> review the findings summary above: warnings are site-side (or new"
    echo "     engine bugs); unexpected new codes block the release."
else
    echo "  skip: dogfood crawl not run (set DOGFOOD=1 to run it)"
fi

echo
echo "READY: v${VERSION} is consistent and the quality gate passes."
echo "Manual steps remaining before tagging (see docs/RELEASE_ASSURANCE.md):"
echo "  1. Complete the release-assurance checklist: GPG key + secrets + 'release' environment, dogfood/service-backed checks, artifact and SBOM review"
echo "  2. git tag v${VERSION} && git push origin v${VERSION}"
echo "  3. Approve the release environment when CI pauses"
