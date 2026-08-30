#!/usr/bin/env bash
set -euo pipefail

threads="${CRAWLKIT_TEST_THREADS:-1}"
cargo test -p crawlkit-engine --lib -- --test-threads="$threads"
cargo test -p crawlkit-api --lib -- --test-threads="$threads"
cargo test -p crawlkit --test cli_tests -- --test-threads="$threads"
cargo test -p crawlkit-engine --test integration_tests -- --test-threads="$threads"
cargo test -p crawlkit-engine --test parallel_pipeline_tests -- --test-threads="$threads"
cargo test -p crawlkit-engine --test property_tests -- --test-threads="$threads"
cargo test -p crawlkit-engine --test determinism_tests -- --test-threads="$threads"
cargo test -p crawlkit-engine --test ssrf_boundary_tests -- --test-threads="$threads"
cargo test -p crawlkit --test cli_tests -- --test-threads="$threads"
