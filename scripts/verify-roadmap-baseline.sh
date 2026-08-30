#!/usr/bin/env bash
set -euo pipefail

failures=0

check_contains() {
  local file="$1"
  local text="$2"
  if ! grep -Fq "$text" "$file"; then
    printf 'FAIL: %s does not contain expected text: %s\n' "$file" "$text" >&2
    failures=$((failures + 1))
  fi
}

check_not_contains() {
  local file="$1"
  local text="$2"
  if grep -Fq "$text" "$file"; then
    printf 'FAIL: %s still contains disallowed text: %s\n' "$file" "$text" >&2
    failures=$((failures + 1))
  fi
}

check_contains Cargo.toml 'rust-version = "1.94.0"'
check_contains docs/capabilities.toml 'msrv = "1.94.0"'
check_contains README.md 'unsafe FFI exists in ABI/plugin components'
check_not_contains README.md 'unsafe_code = "forbid"'
check_not_contains README.md 'zero unsafe code'
check_not_contains CONTRIBUTING.md 'Rust 1.75+'
check_not_contains docs/ARCHITECTURE.md 'crawl  │ │ compare │ │ report  │ │ export  │ │schedule'
check_not_contains docs/ARCHITECTURE.md 'Bloom filter + hash set'
check_not_contains docs/PERFORMANCE_BENCHMARKS.md '| Rust | 1.75.0 |'

if [[ "$failures" -ne 0 ]]; then
  printf '%s roadmap baseline check(s) failed.\n' "$failures" >&2
  exit 1
fi

python3 scripts/verify-capabilities.py
printf 'Roadmap baseline checks passed.\n'
