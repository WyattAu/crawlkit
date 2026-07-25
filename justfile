# crawlkit build automation
# Requires: just (https://github.com/casey/just)

# List available commands
default:
    @just --list

# Format code
fmt:
    cargo fmt --all

# Check formatting
fmt-check:
    cargo fmt --all -- --check

# Run clippy with workspace lints
clippy:
    cargo clippy --workspace --all-targets -- -D warnings

# Run all tests
test:
    cargo test --workspace

# Run unit tests only (fast feedback)
test-unit:
    cargo test --lib --workspace

# Run integration tests
test-integration:
    cargo test --workspace --test integration_tests --test backlink_integration_tests --test playwright_integration_tests --test rum_integration_tests

# Run benchmarks
bench:
    cargo bench --workspace

# Build release binary
build:
    cargo build --release --workspace

# Security audit
audit:
    cargo audit

# License and advisory audit via cargo-deny
deny:
    cargo deny check

# Generate code coverage
coverage:
    cargo tarpaulin --workspace --out Xml --skip-clean

# Run all pre-commit checks locally
pre-commit:
    @echo "Running formatting check..."
    cargo fmt --all -- --check
    @echo "Running clippy..."
    cargo clippy --workspace --all-targets -- -D warnings
    @echo "Running build check..."
    cargo check --workspace
    @echo "Running unit tests..."
    cargo test --lib --workspace
    @echo "All pre-commit checks passed."

# Full quality gate (all checks)
qa: fmt-check clippy test test-integration deny audit
    @cargo test --doc --workspace
    @cargo +1.82.0 check --workspace
    @bash scripts/pre-commit.sh
    @echo "All quality gates passed."

# Install pre-commit hook
install-hooks:
    #!/bin/bash
    set -euo pipefail
    cp scripts/pre-commit.sh .git/hooks/pre-commit
    chmod +x .git/hooks/pre-commit
    echo "Pre-commit hook installed."

# Clean build artifacts
clean:
    cargo clean
    rm -rf web/dist web/.astro

# Build documentation site
docs-build:
    cd web && npm ci && npm run build

# Development server for docs
docs-dev:
    cd web && npm run dev

# Update lockfile
update:
    cargo update

# Check MSRV (Minimum Supported Rust Version)
msrv:
    cargo +1.82.0 check --workspace
