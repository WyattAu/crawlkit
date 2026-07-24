# Plugin Marketplace

The crawlkit plugin marketplace is a central registry for discovering, installing,
and sharing WASM plugins.

## Overview

The marketplace enables:
- **Discovery** -- Find plugins by category, rating, or keyword
- **Installation** -- One-command plugin installation
- **Distribution** -- Share your plugins with the community
- **Quality** -- Automated testing and security scanning

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                  Plugin Marketplace                      │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐    │
│  │   Registry  │  │    CDN      │  │   Gateway   │    │
│  │  (SQLite)   │  │  (S3/R2)   │  │   (API)     │    │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘    │
│         │                │                │            │
│         └────────────────┼────────────────┘            │
│                          │                             │
│                    ┌─────▼─────┐                       │
│                    │  Plugin   │                       │
│                    │  Scanner  │                       │
│                    └───────────┘                       │
└─────────────────────────────────────────────────────────┘
```

## Plugin Submission

### 1. Create Plugin

Follow the [Plugin Development Guide](PLUGIN_DEVELOPMENT.md) to create your plugin.

### 2. Test Plugin

```bash
# Test locally
crawlkit plugin test ./my-plugin/ --html "<html>...</html>"

# Run automated tests
crawlkit plugin test ./my-plugin/ --test-suite full
```

### 3. Submit to Marketplace

```bash
# Submit plugin
crawlkit plugin submit ./my-plugin/ \
  --name "my-seo-checker" \
  --description "Custom SEO analyzer" \
  --author "Your Name" \
  --license "Apache-2.0"
```

### 4. Review Process

1. **Automated Testing** -- Plugin is tested against test suite
2. **Security Scan** -- WASM binary is analyzed for vulnerabilities
3. **Quality Check** -- Code quality and documentation reviewed
4. **Approval** -- Plugin is published to marketplace

## Plugin Discovery

### Browse Plugins

```bash
# List all plugins
crawlkit plugin search

# Search by keyword
crawlkit plugin search "seo"

# Search by category
crawlkit plugin search --category security

# Search by rating
crawlkit plugin search --min-rating 4
```

### Plugin Categories

| Category | Description |
|----------|-------------|
| seo | SEO analysis plugins |
| security | Security analysis plugins |
| performance | Performance analysis plugins |
| accessibility | Accessibility analysis plugins |
| content | Content analysis plugins |
| custom | Custom analysis plugins |

## Plugin Installation

### From Marketplace

```bash
# Install from marketplace
crawlkit plugin install marketplace://author/plugin-name

# Install specific version
crawlkit plugin install marketplace://author/plugin-name@1.0.0
```

### From Local Directory

```bash
# Install from local directory
crawlkit plugin install ./my-plugin/

# Install from .wasm file
crawlkit plugin install ./my-plugin.wasm
```

## Plugin Management

### List Installed Plugins

```bash
crawlkit plugin list
```

### Update Plugins

```bash
# Update all plugins
crawlkit plugin update

# Update specific plugin
crawlkit plugin update my-seo-checker
```

### Remove Plugins

```bash
crawlkit plugin remove my-seo-checker
```

### Plugin Information

```bash
crawlkit plugin info my-seo-checker
```

## API Endpoints

```
GET    /api/v1/marketplace/plugins           # List marketplace plugins
GET    /api/v1/marketplace/plugins/{name}    # Get plugin details
POST   /api/v1/marketplace/plugins           # Submit plugin
DELETE /api/v1/marketplace/plugins/{name}    # Remove plugin
GET    /api/v1/marketplace/plugins/{name}/download  # Download plugin
POST   /api/v1/marketplace/plugins/{name}/test      # Test plugin
```

## Plugin Metadata

```toml
[plugin]
name = "my-seo-checker"
version = "1.0.0"
api_version = "1.0"
author = "Your Name"
description = "Custom SEO analyzer"
license = "Apache-2.0"
repository = "https://github.com/yourname/my-plugin"
homepage = "https://example.com/my-plugin"
categories = ["seo", "custom"]
tags = ["seo", "meta-tags", "analysis"]
min_crawlkit_version = "2.0.0"
```

## Quality Standards

### Automated Testing

- All plugins must pass automated test suite
- Tests include: HTML parsing, finding generation, error handling
- Plugins must handle edge cases (empty HTML, malformed input)

### Security Scanning

- WASM binary analyzed for vulnerabilities
- No network access unless explicitly granted
- No file system access unless explicitly granted
- Memory and CPU limits enforced

### Code Quality

- Rust code follows rustfmt and clippy standards
- Documentation includes usage examples
- CHANGELOG maintained for version history

## Community

### Contributing

1. Fork the plugin repository
2. Create your plugin
3. Submit pull request
4. Review process

### Support

- GitHub Issues: https://github.com/WyattAu/crawlkit/issues
- Discord: https://discord.gg/crawlkit
- Documentation: https://wyattau.github.io/crawlkit
