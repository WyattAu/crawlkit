# Contributing to crawlkit

## Prerequisites

- Rust 1.75+ (latest stable)
- Git
- Pre-commit hook installed (see below)

## Development Setup

```bash
git clone https://github.com/WyattAu/crawlkit.git
cd crawlkit

# Install the pre-commit hook (runs fmt, clippy -D warnings, tests, audit)
cp .git/hooks/pre-commit .git/hooks/pre-commit
chmod +x .git/hooks/pre-commit

cargo build
cargo test
```

## Code Standards

### Mandatory Checks (Pre-commit)

Every commit must pass:
1. `cargo fmt --check`
2. `cargo clippy --all-targets -- -D warnings`
3. `cargo test --lib`
4. `cargo audit` (if installed)

### Coding Requirements

| Requirement | Threshold | Enforcement |
|-------------|-----------|-------------|
| Function length | <= 30 lines | Manual review |
| Cyclomatic complexity | <= 10 | `cargo clippy` |
| Cognitive complexity | <= 15 | `cargo clippy` |
| Test coverage (critical) | >= 95% | `cargo tarpaulin` |
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

- Use `thiserror` for library errors
- Use `?` operator for propagation
- No `unwrap()` in production code
- No `expect()` without descriptive message

### Testing

- AAA pattern (Arrange-Act-Assert)
- Test naming: `test_<function>_<scenario>_<expected>`
- All new code requires unit tests
- Edge cases and error paths must be covered

## Commit Messages

Use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add AI crawler accessibility analyzer
fix: resolve deadlock in audit trail
docs: update API reference
refactor: extract circuit breaker module
test: add token bucket edge cases
chore: update dependencies
```

## Pull Request Process

1. Create feature branch from `main`
2. Implement changes with tests
3. Ensure all CI checks pass
4. Request review (1 approval minimum)
5. Address feedback
6. Squash merge

## Architecture Decision Records

Major changes require an ADR in `.adrs/`:

```markdown
# ADR-XXX: Title

| Field | Value |
|-------|-------|
| Status | Proposed/Accepted/Rejected |
| Date | YYYY-MM-DD |

## Context
## Decision
## Consequences
```

## License

Apache License 2.0. By contributing, you agree to these terms.
