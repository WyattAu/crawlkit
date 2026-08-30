# Deployment Checklist

## Pre-Deployment

### Code Quality

- [ ] Required test suites pass; use the current CI artifacts rather than a manually maintained test count
- [ ] Zero clippy warnings
- [ ] Code formatted with rustfmt
- [ ] Documentation complete
- [ ] Examples working

### Security

- [ ] Security audit completed
- [ ] No critical vulnerabilities
- [ ] No high vulnerabilities
- [ ] Dependencies audited
- [ ] Secrets rotated

### Performance

- [ ] Benchmarks meet targets
- [ ] Memory usage within limits
- [ ] API response times acceptable
- [ ] No performance regressions

### Documentation

- [ ] README updated
- [ ] API documentation complete
- [ ] Deployment guide complete
- [ ] Changelog updated

## Deployment Steps

### 1. Run release readiness

```bash
scripts/verify-release-readiness.sh
```

Version changes must be made consistently through the workspace release
process; do not use the historical version-edit commands below.

### 2. Run Final Tests

```bash
# Rust tests and release controls
CARGO_BUILD_JOBS=1 CRAWLKIT_TEST_THREADS=1 bash scripts/verify-release-controls.sh

# Dashboard/docs tests, when applicable
cd dashboard && pnpm test
```

### 3. Build Release

```bash
# Rust release build
cargo build --release

# React production build
cd dashboard && npm run build
```

### 4. Create Git Tag

Use the reviewed target version and follow the repository's protected release
process. Do not tag until `verify-release-readiness.sh` passes.

### 5. Publish to crates.io

```bash
cargo publish -p crawlkit-engine
cargo publish -p crawlkit-plugin-sdk
```

### 6. Deploy Documentation

```bash
cd web && npm run build
# Deploy to GitHub Pages
```

### 7. Deploy API Server

```bash
# Deploy to production server
ssh production-server
cd /opt/crawlkit
git pull
cargo build --release
sudo systemctl restart crawlkit-api
```

### 8. Verify Deployment

```bash
# Health check
curl https://api.crawlkit.io/health

# API test
curl -H "X-API-Key: <key>" https://api.crawlkit.io/api/v1/crawls
```

### 9. Monitor

```bash
# Check metrics
curl https://api.crawlkit.io/metrics

# Check logs
ssh production-server
tail -f /var/log/crawlkit/api.log
```

## Post-Deployment

### Verification

- [ ] Health endpoint responding
- [ ] API endpoints working
- [ ] Authentication working
- [ ] Crawl functionality working
- [ ] Plugin loading working

### Monitoring

- [ ] Metrics collection active
- [ ] Alerting configured
- [ ] Log aggregation working
- [ ] Performance monitoring active

### Communication

- [ ] Release notes published
- [ ] Email newsletter sent
- [ ] Social media announced
- [ ] Community forums updated

## Rollback Plan

### Immediate Rollback

Use the previously verified release artifact and its checksum rather than
rebuilding arbitrary source from a mutable checkout. Follow the deployment
platform's approved rollback procedure, then run the health and API smoke tests.

### Database Rollback

```bash
# If database schema changed
sqlite3 crawlkit.db < rollback.sql
```

### Verification After Rollback

```bash
# Verify rollback successful
curl https://api.crawlkit.io/health
curl -H "X-API-Key: <key>" https://api.crawlkit.io/api/v1/crawls
```

## Emergency Contacts

| Role | Name | Contact |
|------|------|---------|
| Release Manager | [Name] | [Email] |
| Security Lead | [Name] | [Email] |
| DevOps Lead | [Name] | [Email] |
| QA Lead | [Name] | [Email] |
