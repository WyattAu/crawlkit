# crawlkit Security Hardening Guide

## Overview

This guide covers security best practices for deploying crawlkit in production.

## Authentication

### JWT Configuration

```bash
# Generate a strong JWT secret (256-bit minimum)
export JWT_SECRET=$(openssl rand -hex 32)

# Use environment variable, never hardcode
crawlkit-api --port 8080
```

### Password Policy

- Minimum 12 characters
- Require uppercase, lowercase, numbers, symbols
- Use argon2id for hashing (default)
- Implement account lockout after 5 failed attempts

### API Key Management

```bash
# Create API key with limited permissions
curl -X POST http://localhost:4000/api/v1/keys \
  -H "Authorization: Bearer <jwt>" \
  -H "Content-Type: application/json" \
  -d '{"name": "ci-runner", "requests_per_minute": 60}'

# Rotate keys periodically
curl -X DELETE http://localhost:4000/api/v1/keys/<key-id> \
  -H "Authorization: Bearer <jwt>"
```

## Encryption at Rest

### Enable Encryption

```bash
# Generate encryption key (256-bit for AES-256-GCM)
export CRAWLKIT_ENCRYPTION_KEY=$(openssl rand -hex 32)

# Crawl with encryption
crawlkit crawl https://example.com --encrypt
```

### Key Management

- Store keys in environment variables or secret managers
- Never commit keys to version control
- Rotate keys periodically
- Use different keys per environment (dev/staging/prod)

## Network Security

### TLS Configuration

```bash
# Always use HTTPS for API endpoints
crawlkit-api --port 443 --tls-cert cert.pem --tls-key key.pem
```

### CORS Configuration

```rust
// Restrict CORS to known origins
let cors = CorsLayer::new()
    .allow_origin(["https://crawlkit.example.com".parse().unwrap()])
    .allow_methods([Method::GET, Method::POST])
    .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE]);
```

### Rate Limiting

```toml
# Configure per-IP rate limiting
[api]
rate_limit_per_minute = 60
rate_limit_burst = 10
```

## Input Validation

### URL Validation

```rust
// Validate URLs before crawling
let url = Url::parse(&input)
    .map_err(|_| "Invalid URL")?;

// Check against allowlist
if !is_allowed_domain(&url, &allowed_domains) {
    return Err("Domain not allowed");
}
```

### HTML Sanitization

```rust
// Sanitize HTML before analysis
let sanitized = ammonia::clean(&html);
```

## Audit Logging

### Enable Audit Trail

```toml
[features]
audit_trail = true
```

### Log Retention

```toml
[audit]
retention_days = 90
export_format = "json"
```

## Vulnerability Management

### Dependency Scanning

```bash
# Run security audit
cargo audit

# Check for known vulnerabilities
cargo deny check
```

### Regular Updates

```bash
# Update dependencies
cargo update

# Check for security advisories
cargo audit
```

## Deployment Security

### Container Security

```dockerfile
# Use non-root user
RUN adduser --disabled-password --gecos "" appuser
USER appuser

# Read-only filesystem
RUN chmod -R 555 /app

# No new privileges
--security-opt=no-new-privileges
```

### Kubernetes Security

```yaml
securityContext:
  runAsNonRoot: true
  runAsUser: 1000
  readOnlyRootFilesystem: true
  allowPrivilegeEscalation: false
  capabilities:
    drop:
      - ALL
```

## Monitoring

### Security Metrics

```bash
# Monitor failed authentication attempts
curl http://localhost:4000/metrics | grep auth_failures

# Monitor rate limit hits
curl http://localhost:4000/metrics | grep rate_limit_hits
```

### Alerting

```yaml
# Alert on security events
- alert: HighAuthFailures
  expr: rate(auth_failures_total[5m]) > 10
  for: 5m
  labels:
    severity: critical
```

## Compliance

### SOC 2

- Enable audit logging
- Implement role-based access control
- Encrypt data at rest and in transit
- Regular security assessments

### ISO 27001

- Document security policies
- Implement access controls
- Monitor and log access
- Regular vulnerability assessments

### GDPR

- Implement data minimization
- Provide data subject rights
- Document data processing
- Appoint data protection officer

## Incident Response

### Security Incident Checklist

1. **Containment:** Isolate affected systems
2. **Eradication:** Remove threat
3. **Recovery:** Restore from backups
4. **Lessons Learned:** Document and improve
