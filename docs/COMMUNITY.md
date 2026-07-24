# Community Guidelines

Welcome to the crawlkit community! This guide covers how to contribute, get help, and participate.

## Getting Help

### Documentation

- [User Guide](https://wyattau.github.io/crawlkit)
- [API Reference](https://docs.rs/crawlkit-engine)
- [Plugin Development](PLUGIN_DEVELOPMENT.md)
- [Security Guide](SECURITY.md)

### Support Channels

- **GitHub Issues**: https://github.com/WyattAu/crawlkit/issues
- **GitHub Discussions**: https://github.com/WyattAu/crawlkit/discussions
- **Discord**: https://discord.gg/crawlkit (coming soon)

### Asking Questions

When asking questions, please include:

1. **Environment**: OS, Rust version, crawlkit version
2. **Problem**: Clear description of the issue
3. **Steps to reproduce**: Minimal example that reproduces the issue
4. **Expected behavior**: What you expected to happen
5. **Actual behavior**: What actually happened

## Contributing

### Code Contributions

1. **Fork** the repository
2. **Create** a feature branch (`git checkout -b feature/my-feature`)
3. **Commit** your changes (`git commit -m 'feat: add my feature'`)
4. **Push** to the branch (`git push origin feature/my-feature`)
5. **Open** a Pull Request

### Code Style

- Follow Rust style guidelines (rustfmt, clippy)
- Write tests for new functionality
- Update documentation for API changes
- Use conventional commits

### Pull Request Process

1. Fill out the PR template
2. Ensure all CI checks pass
3. Request review from maintainers
4. Address review feedback
5. Merge after approval

## Reporting Issues

### Bug Reports

Use the bug report template:

```markdown
**Describe the bug**
A clear description of the bug.

**To reproduce**
Steps to reproduce the behavior.

**Expected behavior**
What you expected to happen.

**Screenshots**
If applicable, add screenshots.

**Environment**
- OS: [e.g., Ubuntu 22.04]
- Rust version: [e.g., 1.75.0]
- crawlkit version: [e.g., 2.0.0]
```

### Feature Requests

Use the feature request template:

```markdown
**Is your feature request related to a problem?**
A clear description of the problem.

**Describe the solution you'd like**
A clear description of what you want to happen.

**Describe alternatives you've considered**
Alternative solutions or features.

**Additional context**
Add any other context about the feature request.
```

## Code of Conduct

### Our Pledge

We pledge to make participation in our project a harassment-free experience for everyone.

### Our Standards

Examples of behavior that contributes to creating a positive environment:

- Using welcoming and inclusive language
- Being respectful of differing viewpoints and experiences
- Gracefully accepting constructive criticism
- Focusing on what is best for the community

### Enforcement

Instances of abusive, harassing, or otherwise unacceptable behavior may be reported to the project maintainers.

## Recognition

### Contributors

All contributors are recognized in the project's README and release notes.

### Types of Contributions

- **Code**: Bug fixes, new features, performance improvements
- **Documentation**: Guides, tutorials, API documentation
- **Testing**: Bug reports, test cases, performance benchmarks
- **Design**: UI/UX improvements, accessibility enhancements
- **Community**: Help others, triage issues, review PRs

## Release Process

### Versioning

crawlkit follows Semantic Versioning:

- **Major**: Breaking changes
- **New features**: Backward compatible
- **Patch**: Bug fixes

### Release Cycle

- **Major releases**: Every 6-12 months
- **Minor releases**: Every 1-2 months
- **Patch releases**: As needed

### Changelog

All changes are documented in [CHANGELOG.md](../CHANGELOG.md).
