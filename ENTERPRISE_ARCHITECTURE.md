# crawlkit Enterprise Architecture

*Status: Proposed. Requires stakeholder approval before execution.*
*Version: 2.0.0 | Last updated: 2026-07-25*

## Overview

Multi-tenant, role-based access control with SSO integration for team deployments. Designed for compliance with SOC 2, ISO 27001, and GDPR requirements.

## Design Principles

1. **Zero trust** -- Every request authenticated and authorized
2. **Least privilege** -- Users receive minimum required permissions
3. **Audit everything** -- All actions logged with who/what/when
4. **Tenant isolation** -- Data separated by tenant
5. **SSO-first** -- Delegate authentication to enterprise IdPs

## Authentication Architecture

### Hybrid Mode (Recommended)

```
Authentication
  Local Users ─┐
  OIDC (OAuth2)─┼─> Auth Router > JWT Issuer > Token Store (Redis/SQLite)
  SAML (Legacy)─┘
```

### Token Structure

```json
{
  "iss": "crawlkit",
  "sub": "user-123",
  "aud": "crawlkit-api",
  "exp": 1700000000,
  "iat": 1699996400,
  "jti": "token-456",
  "tenant": "tenant-789",
  "roles": ["editor"],
  "permissions": ["crawl:read", "crawl:write", "report:read"],
  "email": "user@example.com",
  "name": "John Doe"
}
```

### Token Lifecycle

| Phase | Action | Token Behavior |
|-------|--------|----------------|
| Login | Issue tokens | Access (1h) + Refresh (7d) |
| API Request | Validate | Check access token permissions |
| Refresh | Rotate | New access token, rotate refresh token |
| Logout | Revoke | Blacklist access, revoke refresh |
| Expiry | Invalidate | Require re-authentication |

## Role-Based Access Control

### Role Hierarchy

```
Admin
  └── Editor
        └── Viewer
```

### Permission Matrix

| Resource | Viewer | Editor | Admin |
|----------|--------|--------|-------|
| crawl:read | Y | Y | Y |
| crawl:write | - | Y | Y |
| crawl:delete | - | - | Y |
| report:read | Y | Y | Y |
| report:write | - | Y | Y |
| plugin:read | Y | Y | Y |
| plugin:write | - | Y | Y |
| user:read | - | - | Y |
| user:write | - | - | Y |
| role:read | - | - | Y |
| role:write | - | - | Y |
| tenant:read | - | - | Y |
| tenant:write | - | - | Y |
| apikey:read | - | Y | Y |
| apikey:write | - | - | Y |
| webhook:read | Y | Y | Y |
| webhook:write | - | Y | Y |
| schedule:read | Y | Y | Y |
| schedule:write | - | Y | Y |
| audit:read | - | - | Y |
| settings:read | - | - | Y |
| settings:write | - | - | Y |

### Custom Roles

Admins create custom roles with arbitrary permission subsets:

```json
{
  "name": "crawl-only",
  "permissions": ["crawl:read", "crawl:write", "report:read"]
}
```

## Multi-Tenancy

### Tenant Isolation Model

```
Tenant A ─┐
Tenant B ─┼─> Shared Database (tenant_id column for row-level isolation)
Tenant C ─┘
```

### Storage Isolation

| Option | Isolation | Complexity | Recommendation |
|--------|-----------|------------|----------------|
| Shared DB + `tenant_id` | Row-level | Low | Default |
| Separate databases | Strong | High | Compliance-critical |

### Tenant Quotas

```json
{
  "tenant_id": "tenant-789",
  "plan": "professional",
  "quotas": {
    "max_users": 10,
    "max_crawls_per_month": 1000,
    "max_pages_per_crawl": 10000,
    "max_storage_mb": 1024,
    "max_api_calls_per_minute": 100
  }
}
```

## OIDC Integration

### Supported Providers

| Provider | Protocol | Discovery URL |
|----------|----------|---------------|
| Google | OIDC | `accounts.google.com/.well-known/openid-configuration` |
| GitHub | OAuth2 | N/A (no OIDC) |
| Azure AD | OIDC | `login.microsoftonline.com/{tenant}/v2.0` |
| Okta | OIDC | `{domain}/.well-known/openid-configuration` |
| Keycloak | OIDC | `{host}/realms/{realm}/.well-known/openid-configuration` |
| Auth0 | OIDC | `{domain}/.well-known/openid-configuration` |

### OIDC Authorization Code Flow

```
1. Client redirects to IdP /authorize endpoint
2. User authenticates with IdP
3. IdP redirects back with authorization code
4. crawlkit exchanges code for tokens at /token endpoint
5. crawlkit validates ID token (signature, expiry, audience)
6. crawlkit creates or updates local user record
7. crawlkit issues JWT access token
```

### Configuration

```toml
[auth]
mode = "hybrid"  # "local" | "oidc" | "hybrid"

[auth.local]
hash_algorithm = "argon2id"
min_password_length = 12

[auth.oidc]
provider = "google"
client_id = "123456789.apps.googleusercontent.com"
client_secret_env = "OIDC_CLIENT_SECRET"
discovery_url = "https://accounts.google.com/.well-known/openid-configuration"
scopes = ["openid", "email", "profile"]
redirect_uri = "https://crawlkit.example.com/api/v1/auth/callback"
```

## Audit Logging

### Event Structure

```json
{
  "event_id": "evt-123",
  "timestamp": "2026-07-24T12:00:00Z",
  "tenant_id": "tenant-789",
  "user_id": "user-456",
  "user_email": "user@example.com",
  "action": "crawl.create",
  "resource": "crawl-789",
  "details": {
    "url": "https://example.com",
    "max_pages": 100
  },
  "result": "success",
  "ip_address": "192.168.1.100",
  "user_agent": "crawlkit-cli/2.0.0"
}
```

### Event Categories

| Category | Actions |
|----------|---------|
| auth | login, logout, token_refresh, password_change |
| user | create, update, delete, role_change |
| crawl | create, start, stop, delete |
| report | create, export, delete |
| plugin | install, uninstall, enable, disable |
| webhook | create, update, delete |
| schedule | create, update, delete |
| settings | update |
| tenant | create, update, quota_change |

### Retention

Default: 90 days. Configurable per tenant. External SIEM export planned.

## SLA Monitoring

### Targets

| Metric | Target |
|--------|--------|
| Availability | 99.9% |
| Response time (p95) | 500 ms |
| Error rate | < 1.0% |
| Uptime | 99.95% |

### Monitoring

- Availability: `successful_requests / total_requests`
- Latency: p50, p95, p99
- Error rate: `(4xx + 5xx) / total`
- Alerting on SLA breach
- Monthly/quarterly SLA reports

## Security Considerations

| Control | Implementation |
|---------|---------------|
| Password hashing | argon2id (memory-hard) |
| JWT signing | RS256 (asymmetric) |
| Refresh tokens | Rotatable on use |
| Rate limiting | Per-user and per-tenant |
| IP allowlisting | Optional |
| MFA | Via OIDC provider |
| Session management | Concurrent session limits |
| Account lockout | After N failed attempts |

## Compliance Mapping

| Standard | Controls |
|----------|----------|
| SOC 2 | Audit logging, access controls, encryption |
| ISO 27001 | Risk management, incident response |
| GDPR | Data subject rights, data minimization |
| CCPA | Opt-out, data deletion |

## Implementation Phases

| Phase | Scope | Estimate |
|-------|-------|----------|
| 1. Auth Foundation | JWT, middleware, user CRUD, login/refresh/logout | 16h |
| 2. RBAC | Role hierarchy, permission checks, custom roles | 8h |
| 3. Multi-Tenancy | Tenant mgmt, isolation, quotas, scoped queries | 8h |
| 4. OIDC Integration | Discovery, auth code flow, token exchange, provisioning | 16h |
| 5. SLA Monitoring | Tracking, alerting, reports | 4h |
| 6. Audit Logging | Event recording, storage, query API, export | 4h |
| 7. Team Management UI | User mgmt, role assignment, tenant switching, SSO config | 16h |
| 8. Billing Integration | Stripe, usage tracking, invoicing, dashboard | 16h |
| **Total** | | **88h** |
