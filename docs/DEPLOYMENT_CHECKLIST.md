# Deployment Checklist

## Pre-Deployment

### Code Quality

- [ ] All tests passing (470+ Rust, 28 Flutter, 38 React)
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

### 1. Create Release Branch

```bash
git checkout -b release/v2.5.0
```

### 2. Update Version

```bash
# Update Cargo.toml
sed -i 's/version = "2.4.0"/version = "2.5.0"/' Cargo.toml

# Update package.json
sed -i 's/"version": "2.4.0"/"version": "2.5.0"/' dashboard/package.json
```

### 3. Run Final Tests

```bash
# Rust tests
cargo test --workspace

# Flutter tests
cd mobile && flutter test

# React tests
cd dashboard && npm test
```

### 4. Build Release

```bash
# Rust release build
cargo build --release

# Flutter release build
cd mobile && flutter build apk --release

# React production build
cd dashboard && npm run build
```

### 5. Create Git Tag

```bash
git tag -a v2.5.0 -m "Release v2.5.0"
git push origin v2.5.0
```

### 6. Publish to crates.io

```bash
cargo publish -p crawlkit-engine
cargo publish -p crawlkit-plugin-sdk
```

### 7. Deploy Documentation

```bash
cd web && npm run build
# Deploy to GitHub Pages
```

### 8. Deploy API Server

```bash
# Deploy to production server
ssh production-server
cd /opt/crawlkit
git pull
cargo build --release
sudo systemctl restart crawlkit-api
```

### 9. Verify Deployment

```bash
# Health check
curl https://api.crawlkit.io/health

# API test
curl -H "X-API-Key: <key>" https://api.crawlkit.io/api/v1/crawls
```

### 10. Monitor

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

```bash
# Revert to previous version
git checkout v2.4.0
cargo build --release
sudo systemctl restart crawlkit-api
```

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
