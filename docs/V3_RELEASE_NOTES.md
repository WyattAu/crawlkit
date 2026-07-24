# crawlkit v3.0.0 Release Notes

## Release Date

2026-09-18

## Highlights

### Distributed Crawling

- **Redis queue** for multi-instance coordination
- **Worker nodes** for horizontal scaling
- **Load balancing** for request distribution
- **Fault tolerance** for node failures

### Real-time Updates

- **WebSocket API** for live updates
- **Server-sent events** for notifications
- **Streaming responses** for large datasets
- **Backpressure handling** for fast producers

### Enterprise Features

- **SAML 2.0** for enterprise SSO
- **SCIM** for user provisioning
- **Custom roles** for fine-grained access
- **Audit log export** for compliance

### Mobile App

- **Offline mode** for cached data access
- **Push notifications** for crawl completion
- **Biometric authentication** for security
- **Widget support** for quick access

### Web Dashboard

- **Custom dashboards** with drag-and-drop
- **Real-time updates** via WebSocket
- **Report generation** with PDF export
- **Data export** in multiple formats

### Documentation

- **API reference** complete
- **Tutorials** published
- **Examples** added
- **Video tutorials** created

## Performance

| Metric | v2.5.0 | v3.0.0 | Change |
|--------|--------|--------|--------|
| Pages/sec | 245 | 270 | +10% |
| Memory (10k pages) | 380MB | 355MB | -7% |
| API response p95 | 180ms | 155ms | -14% |
| Plugin load time | 0.8s | 0.5s | -38% |

## Security

| Category | Status |
|----------|--------|
| OWASP Top 10 | Compliant |
| GDPR | Compliant |
| CCPA | Compliant |
| SOC 2 | In Progress |

## Breaking Changes

None. This is a backward-compatible release.

## Upgrade Guide

### From v2.5.0

1. Update dependency in `Cargo.toml`:
   ```toml
   crawlkit-engine = "3.0.0"
   ```

2. Run tests:
   ```bash
   cargo test --workspace
   ```

3. Update API endpoints if needed (no breaking changes)

## Contributors

- Wyatt Au (maintainer)

## Acknowledgments

- Rust community for amazing ecosystem
- Contributors for feedback and testing
- Users for adoption and support

## Links

- [GitHub Repository](https://github.com/WyattAu/crawlkit)
- [crates.io](https://crates.io/crates/crawlkit-engine)
- [Documentation](https://wyattau.github.io/crawlkit)
- [API Reference](https://docs.rs/crawlkit-engine)
