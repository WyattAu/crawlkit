#!/usr/bin/env bash
set -euo pipefail

out_dir="${1:-target/benchmark-artifacts}"
mkdir -p "$out_dir"

{
  printf 'timestamp_utc=%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  printf 'commit=%s\n' "$(git rev-parse HEAD 2>/dev/null || printf unknown)"
  printf 'rustc=%s\n' "$(rustc --version 2>/dev/null || printf unknown)"
  printf 'cargo=%s\n' "$(cargo --version 2>/dev/null || printf unknown)"
  printf 'os=%s\n' "$(uname -a 2>/dev/null || printf unknown)"
  printf 'cpu=%s\n' "$(nproc 2>/dev/null || printf unknown)"
  printf 'features=%s\n' "${CRAWLKIT_BENCH_FEATURES:-default}"
  printf 'criterion_command=cargo bench -p crawlkit-engine --bench crawlkit_benchmarks -- --output-format bencher\n'
} > "$out_dir/metadata.txt"

cargo bench -p crawlkit-engine --bench crawlkit_benchmarks -- --output-format bencher \
  | tee "$out_dir/criterion.txt"
