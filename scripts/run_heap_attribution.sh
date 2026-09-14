#!/usr/bin/env bash
# Runner for the heap-attribution harness (profiling feature; see the test
# module doc for method and honest scope). Writes the attribution record to
# CAPACITY_RECORD_DIR (default target/capacity).
set -euo pipefail
cd "$(dirname "$0")/.."
BIN=$(ls -t target/release/deps/heap_attribution-* | grep -v '\.d$' | head -1)
mkdir -p "${CAPACITY_RECORD_DIR:-target/capacity}"
exec env CAPACITY_RECORD_DIR="${CAPACITY_RECORD_DIR:-target/capacity}" \
  "$BIN" --ignored --nocapture --test-threads=1 "$@"
