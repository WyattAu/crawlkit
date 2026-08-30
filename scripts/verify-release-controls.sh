#!/usr/bin/env bash
set -euo pipefail

scripts/verify-roadmap-baseline.sh
scripts/verify-unsafe-inventory.sh
scripts/verify-contracts.sh
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
