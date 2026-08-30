# Contributing to crawlkit

*Version: 4.4.1 | Last reviewed: 2026-08-30*

## Prerequisites

- Rust 1.94.0 or newer (the repository MSRV)
- Git
- `cargo-audit` (optional, for dependency auditing)

## Development Setup

```bash
git clone https://github.com/WyattAu/crawlkit.git
cd crawlkit
cargo build
cargo test
```

### Quality gate

Run the repository gate before submitting changes:

```bash
just gate
```

The exact CI workflow is authoritative; local hooks are convenience tooling only.

### Pre-commit Hook

Install the pre-commit hook to enforce quality gates before each commit:

```bash
cp .git/hooks/pre-commit .git/hooks/pre-commit
chmod +x .git/hooks/pre-commit
```

The hook may execute the following checks sequentially, aborting on first failure. Keep it aligned with CI; do not treat a local hook as a substitute for CI:

| Check | Command | Purpose |
|-------|---------|---------|
| Format | `cargo fmt --check` | Enforce rustfmt style |
| Lint | `cargo clippy --all-targets -- -D warnings` | Zero clippy warnings |
| Test | `cargo test --lib` | Unit test pass |
| Audit | `cargo audit` | Dependency vulnerability scan |

## Code Quality Standards

### Metrics and Thresholds

| Metric | Threshold | Enforcement |
|--------|-----------|-------------|
| Function length | <= 30 lines | Manual review |
| Cyclomatic complexity | <= 10 | `cargo clippy` |
| Cognitive complexity | <= 15 | `cargo clippy` |
| Test coverage | CI-enforced floor; see workflow output | `cargo llvm-cov` |
| Critical-path behavior | Required unit, integration and negative-path tests | Code review + CI |
| Documentation | 100% public items | `cargo doc` |

### Naming Conventions

| Element | Convention | Example |
|---------|------------|---------|
| Types | PascalCase | `HttpClient` |
| Functions | snake_case | `fetch_page()` |
| Constants | SCREAMING_SNAKE | `MAX_RETRIES` |
| Modules | snake_case | `http_client.rs` |

### Error Handling

- Use `thiserror` for library error types
- Use `?` operator for error propagation
- No `unwrap()` in production code paths
- No `expect()` without a descriptive message

### Testing

- Follow AAA pattern (Arrange-Act-Assert)
- Naming convention: `test_<function>_<scenario>_<expected>`
- All new code requires unit tests
- Edge cases and error paths must be covered

## Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add AI crawler accessibility analyzer
fix: resolve deadlock in audit trail
docs: update API reference
refactor: extract circuit breaker module
test: add token bucket edge cases
chore: update dependencies
```

## Pull Request Workflow

Major changes also require an ADR when they alter public APIs/ABIs, storage schemas, trust boundaries, unsafe code, queue semantics, external services or formal capability claims.

## Hard Rules (enforced by CI and reviewers)

**Central pre-push gate:** Before any `git push`, ALL of the following must pass:
1. `cargo clippy --workspace --all-targets -- -D warnings` — zero clippy errors
2. `cargo test --workspace --lib --bins` — all tests pass
3. README badge numbers must match actual test count and analyzer count (verify: `grep "badge/tests" README.md` and `grep "badge/analyzers" README.md` vs actual counts)

This gate exists because past releases pushed with broken clippy / stale badges. `just gate` runs steps 1-2 automatically.
2. Implement changes with tests
3. Ensure all CI checks pass (fmt, clippy, test, audit)
4. Request review (minimum 1 approval)
5. Address feedback
6. Squash merge into `main`

## Architecture Decision Records

Major changes require an ADR in `.adrs/`:

```markdown
# ADR-XXX: Title

| Field | Value |
|-------|-------|
| Status | Proposed / Accepted / Rejected |
| Date | YYYY-MM-DD |

## Context

## Decision

## Consequences
```

## License

Apache License 2.0. By contributing, you agree to these terms.
