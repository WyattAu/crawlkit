#!/usr/bin/env bash
# build-plugin-index.sh — rebuild and re-sign the first-party plugin index.
#
# Builds each SDK example plugin for wasm32 (release), signs it with the
# trusted first-party key, and rewrites plugins/index/plugin-index.toml.
# Run from the workspace root; commit the result (index updates are
# deliberate release events — see ADR-010).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

# Use an absolute path: the dumping test runs with the crate dir as cwd.
FIRST_PARTY_INDEX_DIR="$REPO_ROOT/plugins/index" \
  cargo test -p crawlkit-engine --test plugin_index_tests dump_first_party_index -- --ignored

echo
echo "== Index rebuilt =="
cat plugins/index/plugin-index.toml
