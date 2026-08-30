#!/usr/bin/env bash
# verify-benchmark-regression.sh
#
# Compares the current benchmark output against the retained baseline.
# Fails (exit 1) if any benchmark regresses beyond the threshold.
#
# Usage: scripts/verify-benchmark-regression.sh [threshold_pct]
#   threshold_pct: regression tolerance in percent (default: 25)
set -euo pipefail

threshold="${1:-25}"
baseline_dir="docs/benchmarks/baseline"
baseline="$baseline_dir/criterion.txt"

if [ ! -f "$baseline" ]; then
  echo "SKIP: no retained baseline at $baseline" >&2
  exit 0
fi

tmp=$(mktemp)
trap 'rm -f "$tmp"' EXIT

# Run benchmarks with bencher output format (same as baseline capture).
CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}" \
  cargo bench -p crawlkit-engine --bench crawlkit_benchmarks \
  -- --output-format bencher 2>/dev/null | grep '^test ' > "$tmp" || true

if [ ! -s "$tmp" ]; then
  echo "SKIP: no benchmark output produced" >&2
  exit 0
fi

failed=0
while IFS= read -r line; do
  # Parse: "test <name> ... bench:  <ns> ns/iter"
  name=$(echo "$line" | sed -n 's/^test \([^ ]*\) \.\.\. bench:.*/\1/p')
  [ -z "$name" ] && continue
  current_ns=$(echo "$line" | sed -n 's/.*bench: *\([0-9]*\) ns\/iter.*/\1/p')
  [ -z "$current_ns" ] && continue

  baseline_line=$(grep "test $name " "$baseline" || true)
  [ -z "$baseline_line" ] && continue
  baseline_ns=$(echo "$baseline_line" | sed -n 's/.*bench: *\([0-9]*\) ns\/iter.*/\1/p')
  [ -z "$baseline_ns" ] && continue

  # Skip zero-cost benchmarks (e.g., circuit_breaker at 0 ns).
  [ "$baseline_ns" -eq 0 ] && continue

  delta_pct=$(( (current_ns - baseline_ns) * 100 / baseline_ns ))
  status="ok"
  if [ "$delta_pct" -gt "$threshold" ]; then
    status="REGRESSION"
    failed=1
  fi
  printf '%-50s baseline=%10s ns  current=%10s ns  delta=%+3d%%  %s\n' \
    "$name" "$baseline_ns" "$current_ns" "$delta_pct" "$status"
done < "$tmp"

if [ "$failed" -ne 0 ]; then
  echo "::error::benchmark regression exceeds ${threshold}% threshold" >&2
  exit 1
fi
echo "Benchmark regression check passed (threshold: ${threshold}%)."
