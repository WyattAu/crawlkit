# Security Audit Checklist

This checklist covers security requirements for external audits.

## Authentication and Authorization

- [ ] JWT tokens signed with RS256
- [ ] Token expiry enforced (1 hour default)
- [ ] Refresh token rotation enabled
- [ ] Password hashing with argon2id
- [ ] Rate limiting per user and per IP
- [ ] Account lockout after failed attempts
- [ ] MFA support via OIDC provider
- [ ] Session management (concurrent session limits)

## Data Protection

- [ ] Encryption at rest with AES-256-GCM
- [ ] Encryption key management (env vars, keyring)
- [ ] Data minimization (collect only necessary data)
- [ ] Data retention policies configured
- [ ] Data subject rights (export, deletion)
- [ ] GDPR compliance documented
- [ ] CCPA compliance documented

## Access Control

- [ ] Role-based access control (RBAC)
- [ ] Least privilege principle enforced
- [ ] Tenant data isolation verified
- [ ] API key scoping and rotation
- [ ] Audit logging for all actions
- [ ] Privilege escalation prevention

## Input Validation

- [ ] URL validation before crawling
- [ ] HTML sanitization for analysis
- [ ] SQL injection prevention (parameterized queries)
- [ ] XSS prevention in reports
- [ ] CSRF protection on state-changing operations
- [ ] File upload validation (plugins)

## Network Security

- [ ] TLS 1.2+ for all connections
- [ ] Certificate validation enabled
- [ ] HSTS headers configured
- [ ] CORS properly restricted
- [ ] Content Security Policy configured
- [ ] Rate limiting on all endpoints

## Dependency Management

- [ ] Dependencies audited (cargo audit)
- [ ] Known vulnerabilities addressed
- [ ] SBOM generated (SPDX format)
- [ ] Dependency pinning (Cargo.lock committed)
- [ ] Automated dependency updates
- [ ] Supply chain security (cargo-deny)

## Plugin Security

- [ ] WASM sandboxing enforced
- [ ] Plugin permissions checked
- [ ] Plugin signatures verified
- [ ] Memory limits enforced (64MB)
- [ ] CPU limits enforced (100ms)
- [ ] Network access restricted

## Monitoring and Logging

- [ ] Security event logging
- [ ] Anomaly detection configured
- [ ] Alert thresholds set
- [ ] Log retention policies
- [ ] Log integrity verification
- [ ] External SIEM integration

## Incident Response

- [ ] Incident response plan documented
- [ ] Escalation procedures defined
- [ ] Communication templates ready
- [ ] Recovery procedures tested
- [ ] Post-incident review process
- [ ] Lessons learned documented

## Compliance

- [ ] SOC 2 Type II audit scheduled
- [ ] ISO 27001 certification planned
- [ ] GDPR compliance verified
- [ ] CCPA compliance verified
- [ ] Industry-specific regulations addressed
- [ ] Regular compliance reviews scheduled
