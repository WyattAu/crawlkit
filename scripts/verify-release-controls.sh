#!/usr/bin/env bash
set -euo pipefail

scripts/verify-roadmap-baseline.sh
scripts/verify-unsafe-inventory.sh
scripts/verify-contracts.sh
# Finding-code drift gate (ANALYZER_AUDIT Phase 4 follow-up): fails on a
# new cross-analyzer code collision or a stale docs/FINDING_CODES.md.
# --self-test first pins the scanner's literal/comment scoping so a
# scoping regression cannot silently mis-attribute codes again.
python3 scripts/generate_finding_catalog.py --self-test
python3 scripts/generate_finding_catalog.py --check
# Capacity-report drift gate (CAPACITY_EVIDENCE_PLAN §5.7): every
# docs/capacity/ REPORT.md's generated-numbers section must match a fresh
# render of the committed run records.
python3 scripts/render_capacity_report.py --check
if git diff --check; then
  :
else
  echo "release control: whitespace errors detected in the working tree" >&2
  exit 1
fi
cargo fmt --all -- --check
CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}" cargo clippy --workspace --all-targets -- -D warnings
CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}" cargo test -p crawlkit-engine --lib -- --test-threads="${CRAWLKIT_TEST_THREADS:-1}"
CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}" cargo test -p crawlkit-api --lib -- --test-threads="${CRAWLKIT_TEST_THREADS:-1}"
CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}" cargo test --doc --workspace -- --test-threads="${CRAWLKIT_TEST_THREADS:-1}"
