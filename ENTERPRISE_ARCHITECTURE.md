# crawlkit Enterprise Architecture

Status: Proposed. Requires stakeholder approval before execution.

---

## Overview

Enterprise features enable multi-tenant, role-based access control with SSO
integration for team deployments. Designed for compliance with SOC 2, ISO 27001,
and GDPR requirements.

## Design Principles

1. **Zero trust** -- Every request authenticated and authorized
2. **Least privilege** -- Users get minimum required permissions
3. **Audit everything** -- All actions logged with who/what/when
4. **Tenant isolation** -- Data separated by tenant
5. **SSO-first** -- Delegate authentication to enterprise IdPs

## Authentication Architecture

### Hybrid Mode (Recommended)

```
┌─────────────────────────────────────────────────────────┐
│                    Authentication                        │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐ │
│  │   Local     │    │    OIDC     │    │    SAML     │ │
│  │   Users     │    │   (OAuth2)  │    │  (Legacy)   │ │
│  └──────┬──────┘    └──────┬──────┘    └──────┬──────┘ │
│         │                  │                  │         │
│         └──────────────────┼──────────────────┘         │
│                            │                            │
│                    ┌───────▼───────┐                    │
│                    │  Auth Router  │                    │
│                    └───────┬───────┘                    │
│                            │                            │
│                    ┌───────▼───────┐                    │
│                    │  JWT Issuer   │                    │
│                    └───────┬───────┘                    │
│                            │                            │
│                    ┌───────▼───────┐                    │
│                    │  Token Store  │                    │
│                    │  (Redis/SQLite)│                    │
│                    └───────────────┘                    │
└─────────────────────────────────────────────────────────┘
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

1. **Login** → Issue access token (1h) + refresh token (7d)
2. **API Request** → Validate access token, check permissions
3. **Token Refresh** → Issue new access token, rotate refresh token
4. **Logout** → Revoke refresh token, blacklist access token
5. **Expiry** → Token invalid, require re-authentication

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
| Crawl:Read | Yes | Yes | Yes |
| Crawl:Write | No | Yes | Yes |
| Crawl:Delete | No | No | Yes |
| Report:Read | Yes | Yes | Yes |
| Report:Write | No | Yes | Yes |
| Plugin:Read | Yes | Yes | Yes |
| Plugin:Write | No | Yes | Yes |
| User:Read | No | No | Yes |
| User:Write | No | No | Yes |
| Role:Read | No | No | Yes |
| Role:Write | No | No | Yes |
| Tenant:Read | No | No | Yes |
| Tenant:Write | No | No | Yes |
| APIKey:Read | No | Yes | Yes |
| APIKey:Write | No | No | Yes |
| Webhook:Read | Yes | Yes | Yes |
| Webhook:Write | No | Yes | Yes |
| Schedule:Read | Yes | Yes | Yes |
| Schedule:Write | No | Yes | Yes |
| Audit:Read | No | No | Yes |
| Settings:Read | No | No | Yes |
| Settings:Write | No | No | Yes |

### Custom Roles

Admins can create custom roles with specific permission sets:

```json
{
  "name": "crawl-only",
  "permissions": ["crawl:read", "crawl:write", "report:read"]
}
```

## Multi-Tenancy

### Tenant Isolation Model

```
┌─────────────────────────────────────────────────────────┐
│                    Tenant Isolation                      │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐    │
│  │  Tenant A   │  │  Tenant B   │  │  Tenant C   │    │
│  │             │  │             │  │             │    │
│  │  ┌───────┐  │  │  ┌───────┐  │  │  ┌───────┐  │    │
│  │  │Users  │  │  │  │Users  │  │  │  │Users  │  │    │
│  │  └───────┘  │  │  └───────┘  │  │  └───────┘  │    │
│  │  ┌───────┐  │  │  ┌───────┐  │  │  ┌───────┐  │    │
│  │  │Crawls │  │  │  │Crawls │  │  │  │Crawls │  │    │
│  │  └───────┘  │  │  └───────┘  │  │  └───────┘  │    │
│  │  ┌───────┐  │  │  ┌───────┐  │  │  ┌───────┐  │    │
│  │  │Reports│  │  │  │Reports│  │  │  │Reports│  │    │
│  │  └───────┘  │  │  └───────┘  │  │  └───────┘  │    │
│  └─────────────┘  └─────────────┘  └─────────────┘    │
│                                                         │
│  ┌─────────────────────────────────────────────────┐   │
│  │              Shared Database                     │   │
│  │  (tenant_id column for isolation)                │   │
│  └─────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
```

### Storage Isolation

- **Option A: Shared database with tenant_id** (Recommended)
  - Simpler implementation
  - Tenant_id column in all tables
  - Row-level security via WHERE clauses
  - Backup/restore per tenant

- **Option B: Separate databases**
  - Stronger isolation
  - More operational complexity
  - Better for compliance requirements

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

| Provider | Protocol | Notes |
|----------|----------|-------|
| Google | OIDC | `accounts.google.com` |
| GitHub | OAuth2 | No OIDC, use OAuth2 flow |
| Azure AD | OIDC | `login.microsoftonline.com` |
| Okta | OIDC | Custom domain |
| Keycloak | OIDC | Self-hosted |
| Auth0 | OIDC | Custom domain |

### OIDC Flow

```
1. User clicks "Login with SSO"
2. crawlkit redirects to IdP authorize endpoint
3. User authenticates with IdP
4. IdP redirects back with authorization code
5. crawlkit exchanges code for tokens
6. crawlkit validates ID token
7. crawlkit creates/finds local user
8. crawlkit issues JWT access token
```

### Configuration

```toml
[auth]
# Authentication mode: "local" | "oidc" | "hybrid"
mode = "hybrid"

[auth.local]
# Password hashing algorithm
hash_algorithm = "argon2id"
# Password minimum length
min_password_length = 12

[auth.oidc]
# OIDC provider name
provider = "google"
# Client ID from IdP
client_id = "123456789.apps.googleusercontent.com"
# Client secret (from environment variable)
client_secret_env = "OIDC_CLIENT_SECRET"
# OIDC discovery URL
discovery_url = "https://accounts.google.com/.well-known/openid-configuration"
# Scopes to request
scopes = ["openid", "email", "profile"]
# Redirect URI after authentication
redirect_uri = "https://crawlkit.example.com/api/v1/auth/callback"
```

## SLA Monitoring

### SLA Targets

```json
{
  "tenant_id": "tenant-789",
  "targets": {
    "availability": 99.9,
    "response_time_ms": 500,
    "error_rate_percent": 1.0,
    "uptime_percent": 99.95
  }
}
```

### SLA Monitoring

- Track availability (successful requests / total requests)
- Track response time (p50, p95, p99)
- Track error rate (4xx + 5xx / total)
- Alert when SLA targets are missed
- Generate SLA reports (monthly/quarterly)

## Audit Logging

### Audit Event Structure

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
  "user_agent": "crawlkit-cli/0.6.0"
}
```

### Audit Event Categories

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

### Audit Log Retention

- Default: 90 days
- Configurable per tenant
- Export to external SIEM (future)

## Implementation Phases

### Phase 1: Auth Foundation (16h)
- JWT token generation and validation
- Authorization middleware
- User management (CRUD)
- Login/refresh/logout endpoints
- Password hashing (argon2id)

### Phase 2: RBAC (8h)
- Role hierarchy
- Permission checks
- Role management endpoints
- Custom roles

### Phase 3: Multi-Tenancy (8h)
- Tenant management
- Tenant isolation
- Quota enforcement
- Tenant-scoped queries

### Phase 4: OIDC Integration (16h)
- OIDC discovery
- Authorization code flow
- Token exchange
- User provisioning
- Provider configuration

### Phase 5: SLA Monitoring (4h)
- SLA tracking
- Alert generation
- SLA reports

### Phase 6: Audit Logging (4h)
- Event recording
- Log storage
- Query API
- Export

### Phase 7: Team Management UI (16h)
- User management interface
- Role assignment
- Tenant switching
- SSO configuration

### Phase 8: Billing Integration (16h)
- Stripe integration
- Usage tracking
- Invoice generation
- Usage dashboard

**Total: ~88h**

## Security Considerations

- All passwords hashed with argon2id (memory-hard)
- JWT signed with RS256 (asymmetric)
- Refresh tokens rotatable on use
- Rate limiting per user and per tenant
- IP allowlisting (optional)
- MFA support via OIDC provider
- Session management (concurrent session limits)
- Account lockout after failed attempts

## Compliance

- SOC 2: Audit logging, access controls, encryption
- ISO 27001: Risk management, incident response
- GDPR: Data subject rights, data minimization
- CCPA: Opt-out, data deletion
