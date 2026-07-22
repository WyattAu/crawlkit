# Contributing to crawlkit

Thank you for your interest in contributing to crawlkit! This document provides guidelines and information for contributors.

## Getting Started

### Prerequisites

- Rust 1.70+ (latest stable recommended)
- Git
- cargo-edit (optional, for dependency management)

### Building

```bash
# Clone the repository
git clone https://github.com/WyattAu/crawlkit.git
cd crawlkit

# Build the project
cargo build

# Run tests
cargo test

# Build release binary
cargo build --release
```

### Development Setup

1. Fork the repository on GitHub
2. Clone your fork locally
3. Create a feature branch: `git checkout -b feature/my-feature`
4. Make your changes
5. Run tests: `cargo test`
6. Run clippy: `cargo clippy -- -D warnings`
7. Format code: `cargo fmt`
8. Commit your changes
9. Push to your fork
10. Create a Pull Request

## Code Style

- Follow Rust standard style (enforced by `rustfmt`)
- Use `clippy` lints (enforced by CI)
- Write doc comments for all public items
- Write tests for new functionality

## Testing

- All new code must have unit tests
- Tests should cover normal, edge, and error cases
- Run `cargo test` before submitting PR
- Target: >90% test coverage

## Commit Messages

Use [Conventional Commits](https://www.conventionalcommits.org/):

- `feat: add new feature`
- `fix: bug fix`
- `docs: documentation changes`
- `refactor: code refactoring`
- `test: add tests`
- `chore: maintenance tasks`

## Pull Request Process

1. Update documentation if needed
2. Add tests for new functionality
3. Ensure all CI checks pass
4. Request review from maintainer
5. Address review feedback
6. Merge when approved

## Reporting Issues

- Use GitHub Issues for bug reports
- Include minimal reproduction steps
- Include Rust version and OS information
- Check existing issues before creating new ones

## License

By contributing, you agree that your contributions will be licensed under the Apache License 2.0.
