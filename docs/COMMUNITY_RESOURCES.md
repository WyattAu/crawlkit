# Community Resources

## Getting Started

### Installation

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install crawlkit
cargo install crawlkit-engine

# Verify installation
crawlkit --version
```

### Quick Start

```bash
# Crawl a website
crawlkit crawl https://example.com --max-pages 50

# Generate report
crawlkit report crawl-data/ --format html --output report.html

# Start API server
crawlkit-api --port 8080
```

## Documentation

### Core Documentation

- [User Guide](https://wyattau.github.io/crawlkit)
- [API Reference](https://docs.rs/crawlkit-engine)
- [Plugin Development](https://wyattau.github.io/crawlkit/plugin-development)
- [Security Guide](https://wyattau.github.io/crawlkit/security)

### Tutorials

- [Getting Started](https://wyattau.github.io/crawlkit/getting-started)
- [Custom Analyzers](https://wyattau.github.io/crawlkit/custom-analyzers)
- [CI Integration](https://wyattau.github.io/crawlkit/ci-integration)
- [Enterprise Deployment](https://wyattau.github.io/crawlkit/enterprise)

### Examples

- [Basic Crawl](https://github.com/WyattAu/crawlkit/tree/main/crates/crawlkit/examples)
- [Custom Analyzer](https://github.com/WyattAu/crawlkit/tree/main/crates/crawlkit/examples)
- [Encrypted Crawl](https://github.com/WyattAu/crawlkit/tree/main/crates/crawlkit/examples)
- [Deterministic Crawl](https://github.com/WyattAu/crawlkit/tree/main/crates/crawlkit/examples)

## Community

### Discord

- **Invite:** https://discord.gg/crawlkit
- **Channels:** #general, #support, #plugins, #development
- **Rules:** Be respectful, help others, share knowledge

### GitHub

- **Repository:** https://github.com/WyattAu/crawlkit
- **Issues:** https://github.com/WyattAu/crawlkit/issues
- **Discussions:** https://github.com/WyattAu/crawlkit/discussions
- **Pull Requests:** https://github.com/WyattAu/crawlkit/pulls

### Social Media

- **Twitter:** @crawlkit
- **LinkedIn:** crawlkit
- **YouTube:** crawlkit

## Contributing

### How to Contribute

1. **Fork** the repository
2. **Create** a feature branch
3. **Commit** your changes
4. **Push** to the branch
5. **Open** a Pull Request

### Contribution Types

- **Code:** Bug fixes, new features, performance improvements
- **Documentation:** Guides, tutorials, API documentation
- **Testing:** Bug reports, test cases, performance benchmarks
- **Design:** UI/UX improvements, accessibility enhancements
- **Community:** Help others, triage issues, review PRs

### Code Style

- Follow Rust style guidelines (rustfmt, clippy)
- Write tests for new functionality
- Update documentation for API changes
- Use conventional commits

## Support

### Getting Help

1. **Check documentation** -- Search existing docs
2. **Search issues** -- Look for existing issues
3. **Ask on Discord** -- Community support
4. **Open an issue** -- Bug reports, feature requests

### Reporting Issues

Use the issue templates:

- **Bug Report:** For reproducible bugs
- **Feature Request:** For new features
- **Documentation:** For docs improvements
- **Security:** For security vulnerabilities

### Response Times

| Type | Target | Actual |
|------|--------|--------|
| Critical bugs | <24 hours | <12 hours |
| High bugs | <1 week | <3 days |
| Questions | <12 hours | <6 hours |
| PR reviews | <48 hours | <24 hours |

## Learning Resources

### Books

- "The Rust Programming Language" -- Official Rust book
- "Rust in Action" -- Practical Rust projects
- "Zero To Production In Rust" -- Backend development

### Courses

- Rustlings -- Interactive Rust exercises
- Rust Track on Exercism -- Practice problems
- Let's Get Rusty -- YouTube tutorials

### Videos

- RustConf talks -- Annual Rust conference
- Jon Gjengset -- Advanced Rust topics
- Tech With Tim -- Rust tutorials

## Events

### Meetups

- Rust Meetup -- Local Rust user groups
- Rust Berlin -- Berlin Rust community
- Rust London -- London Rust community

### Conferences

- RustConf -- Annual Rust conference
- EuroRust -- European Rust conference
- RustConf Asia -- Asian Rust conference

### Workshops

- Rust Workshop -- Hands-on Rust training
- Rust Security Workshop -- Rust security best practices
- Rust Performance Workshop -- Rust optimization techniques
