# Team Management Guide

This guide covers managing users, roles, and tenants in crawlkit.

## User Management

### Creating Users

```bash
# Via API
curl -X POST http://localhost:4000/api/v1/users \
  -H "Authorization: Bearer <jwt>" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "name": "John Doe",
    "password": "secure-password-123",
    "tenant_id": "tenant-123",
    "roles": ["editor"]
  }'
```

### Listing Users

```bash
curl http://localhost:4000/api/v1/users \
  -H "Authorization: Bearer <jwt>"
```

### Deleting Users

```bash
curl -X DELETE http://localhost:4000/api/v1/users/user-123 \
  -H "Authorization: Bearer <jwt>"
```

## Role Management

### Default Roles

| Role | Permissions |
|------|-------------|
| Admin | Full access to all resources |
| Editor | Create/update crawls, reports |
| Viewer | Read-only access |

### Creating Custom Roles

```bash
curl -X POST http://localhost:4000/api/v1/roles \
  -H "Authorization: Bearer <jwt>" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "crawl-only",
    "permissions": ["crawl:read", "crawl:write", "report:read"]
  }'
```

### Permission Matrix

| Resource | Viewer | Editor | Admin |
|----------|--------|--------|-------|
| Crawl:Read | Yes | Yes | Yes |
| Crawl:Write | No | Yes | Yes |
| Crawl:Delete | No | No | Yes |
| Report:Read | Yes | Yes | Yes |
| Report:Write | No | Yes | Yes |
| User:Read | No | No | Yes |
| User:Write | No | No | Yes |
| Tenant:Read | No | No | Yes |
| Tenant:Write | No | No | Yes |

## Tenant Management

### Creating Tenants

```bash
curl -X POST http://localhost:4000/api/v1/tenants \
  -H "Authorization: Bearer <jwt>" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Acme Corp",
    "plan": "professional"
  }'
```

### Tenant Quotas

| Plan | Users | Crawls/Month | Pages/Crawl | Storage |
|------|-------|--------------|-------------|---------|
| Free | 1 | 10 | 100 | 100MB |
| Starter | 5 | 100 | 1,000 | 1GB |
| Professional | 20 | 1,000 | 10,000 | 10GB |
| Enterprise | Unlimited | Unlimited | Unlimited | Unlimited |

### Tenant Isolation

All data is isolated by tenant. Users can only access their own tenant's data.

```sql
-- Tenant-scoped query
SELECT * FROM pages
WHERE tenant_id = 'tenant-123'
AND crawl_id = 'crawl-456';
```

## SSO Configuration

### OIDC Setup

```bash
# Set environment variables
export OIDC_PROVIDER=google
export OIDC_CLIENT_ID=your-client-id
export OIDC_CLIENT_SECRET=your-client-secret
export OIDC_DISCOVERY_URL=https://accounts.google.com
export OIDC_REDIRECT_URI=https://your-domain.com/api/v1/auth/oidc/callback

# Start API server
crawlkit-api --port 8080
```

### Supported Providers

| Provider | Protocol | Discovery URL |
|----------|----------|---------------|
| Google | OIDC | accounts.google.com |
| GitHub | OAuth2 | N/A (use OAuth2 flow) |
| Azure AD | OIDC | login.microsoftonline.com/{tenant} |
| Okta | OIDC | {domain}/.well-known/openid-configuration |
| Keycloak | OIDC | {domain}/.well-known/openid-configuration |

## Audit Logging

### Viewing Audit Logs

```bash
curl http://localhost:4000/api/v1/audit \
  -H "Authorization: Bearer <jwt>"
```

### Audit Event Types

| Event | Description |
|-------|-------------|
| auth.login | User login |
| auth.logout | User logout |
| crawl.create | Crawl started |
| crawl.complete | Crawl completed |
| user.create | User created |
| user.delete | User deleted |
| tenant.create | Tenant created |
| tenant.delete | Tenant deleted |
```
