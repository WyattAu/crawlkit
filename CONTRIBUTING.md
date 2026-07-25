# Contributing to crawlkit

*Version: 2.0.0 | Last updated: 2026-07-25*

## Prerequisites

- Rust 1.75+ (latest stable)
- Git
- `cargo-audit` (optional, for dependency auditing)

## Development Setup

```bash
git clone https://github.com/WyattAu/crawlkit.git
cd crawlkit
cargo build
cargo test
```

### Pre-commit Hook

Install the pre-commit hook to enforce quality gates before each commit:

```bash
cp .git/hooks/pre-commit .git/hooks/pre-commit
chmod +x .git/hooks/pre-commit
```

The hook executes the following checks sequentially, aborting on first failure:

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
| Test coverage (critical paths) | >= 95% | `cargo tarpaulin` |
| Test coverage (overall) | >= 90% | `cargo tarpaulin` |
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

1. Create feature branch from `main`
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
