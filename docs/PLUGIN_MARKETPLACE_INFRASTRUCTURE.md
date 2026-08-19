# Plugin Marketplace Infrastructure

This document outlines the infrastructure for the plugin marketplace.

## Architecture

### Components

```
+-----------------------------------------------------------+
|                  Plugin Marketplace                        |
+-----------------------------------------------------------+
|                                                           |
|  +-------------+  +-------------+  +-------------+       |
|  |   Registry  |  |    CDN      |  |   Gateway   |       |
|  | (PostgreSQL)|  | (Cloudflare)|  | (Cloudflare)|       |
|  +------+------+  +------+------+  +------+------+       |
|         |                |                |               |
|         +----------------+----------------+               |
|                          |                               |
|                    +-----v-----+                         |
|                    |  Plugin   |                         |
|                    |  Scanner  |                         |
|                    |  (Rust)   |                         |
|                    +-----------+                         |
|                          |                               |
|                    +-----v-----+                         |
|                    |  Signing  |                         |
|                    |  Service  |                         |
|                    +-----------+                         |
+-----------------------------------------------------------+
```

### Technology Stack

| Component | Technology | Purpose |
|-----------|------------|---------|
| Registry | PostgreSQL | Plugin metadata storage |
| CDN | Cloudflare | WASM file distribution |
| Gateway | Cloudflare Workers | API routing and rate limiting |
| Scanner | Rust | WASM security scanning |
| Signing | SHIPPED: Rust + ed25519-dalek | Plugin signature verification via the built-in trust store |

### Database Schema

```sql
-- Plugins table
CREATE TABLE plugins (
    id UUID PRIMARY KEY,
    name VARCHAR(255) UNIQUE NOT NULL,
    version VARCHAR(50) NOT NULL,
    author_id UUID NOT NULL,
    description TEXT,
    license VARCHAR(50),
    repository_url TEXT,
    homepage_url TEXT,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    downloads INTEGER DEFAULT 0,
    rating DECIMAL(3,2) DEFAULT 0.0,
    status VARCHAR(20) DEFAULT 'pending'
);

-- Plugin versions table
CREATE TABLE plugin_versions (
    id UUID PRIMARY KEY,
    plugin_id UUID REFERENCES plugins(id),
    version VARCHAR(50) NOT NULL,
    wasm_url TEXT NOT NULL,
    wasm_hash VARCHAR(64) NOT NULL,
    signature TEXT,
    created_at TIMESTAMP DEFAULT NOW(),
    status VARCHAR(20) DEFAULT 'pending'
);

-- Plugin categories table
CREATE TABLE plugin_categories (
    id UUID PRIMARY KEY,
    plugin_id UUID REFERENCES plugins(id),
    category VARCHAR(100) NOT NULL
);

-- Plugin tags table
CREATE TABLE plugin_tags (
    id UUID PRIMARY KEY,
    plugin_id UUID REFERENCES plugins(id),
    tag VARCHAR(100) NOT NULL
);

-- Authors table
CREATE TABLE authors (
    id UUID PRIMARY KEY,
    username VARCHAR(255) UNIQUE NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    created_at TIMESTAMP DEFAULT NOW()
);

-- Downloads table
CREATE TABLE downloads (
    id UUID PRIMARY KEY,
    plugin_id UUID REFERENCES plugins(id),
    version_id UUID REFERENCES plugin_versions(id),
    user_ip INET,
    user_agent TEXT,
    downloaded_at TIMESTAMP DEFAULT NOW()
);
```

### API Endpoints

```
# Plugin Management
GET    /api/v1/plugins                    # List plugins
POST   /api/v1/plugins                    # Submit plugin
GET    /api/v1/plugins/{name}             # Get plugin details
PUT    /api/v1/plugins/{name}             # Update plugin
DELETE /api/v1/plugins/{name}             # Delete plugin

# Version Management
GET    /api/v1/plugins/{name}/versions    # List versions
POST   /api/v1/plugins/{name}/versions    # Submit version
GET    /api/v1/plugins/{name}/versions/{version}  # Get version
DELETE /api/v1/plugins/{name}/versions/{version}  # Delete version

# Download
GET    /api/v1/plugins/{name}/download    # Download latest
GET    /api/v1/plugins/{name}/versions/{version}/download  # Download specific

# Search
GET    /api/v1/plugins/search?q={query}  # Search plugins
GET    /api/v1/plugins/category/{category}  # List by category

# Testing
POST   /api/v1/plugins/{name}/test        # Test plugin
GET    /api/v1/plugins/{name}/test/{test_id}  # Get test results
```

### Security Features

**Plugin Signing (SHIPPED in v3.0):**
- Shipped: plugins are signed with ed25519 (`crawlkit plugin keygen/sign/verify`)
  and verified by the engine against a built-in trust store
  (`TRUSTED_PLUGIN_KEYS`) *before* the WASM is compiled — fail-closed by
  default, with an opt-in `AllowUnsigned` policy for local development
- Each plugin version records `wasm_hash` (sha256), `signature` (ed25519
  over the raw digest), and `signed_by` (key id) in its manifest
- Registry-side signature storage is used for marketplace listings
- Future: KMS-backed key custody and automated key rotation for the
  release signing service

**Scanning:**
- WASM binary analyzed for vulnerabilities
- Memory access patterns checked
- Network access patterns checked
- File system access patterns checked

**Rate Limiting:**
- API rate limiting per user
- Download rate limiting per IP
- Submission rate limiting per author

**Access Control:**
- API key authentication
- JWT token authentication
- Role-based access control

### Deployment

**Infrastructure:**
- PostgreSQL on AWS RDS
- Cloudflare for CDN and Gateway
- AWS Lambda for scanner
- AWS KMS for signing keys (future — current signing uses the shipped
  built-in ed25519 trust store; KMS custody is planned for the release
  signing service)

**Monitoring:**
- Prometheus metrics
- Grafana dashboards
- Alerting rules
- Log aggregation

**Scaling:**
- Horizontal scaling for Gateway
- Vertical scaling for Database
- CDN caching for WASM files
- Database read replicas

### Cost Estimation

| Component | Monthly Cost |
|-----------|--------------|
| PostgreSQL (RDS) | $100 |
| Cloudflare (Pro) | $20 |
| AWS Lambda | $50 |
| AWS KMS | $10 |
| Monitoring | $20 |
| **Total** | **$200/month** |

## Implementation Roadmap

### Phase 1: Foundation (4 weeks)
- Database schema
- API gateway
- Basic plugin management

### Phase 2: Security (4 weeks)
- Plugin signing — SHIPPED in v3.0 (built-in ed25519 trust store)
- WASM scanning
- Access control

### Phase 3: CDN (2 weeks)
- Cloudflare setup
- WASM distribution
- Caching configuration

### Phase 4: Testing (2 weeks)
- Plugin testing
- Security testing
- Performance testing

### Phase 5: Launch (2 weeks)
- Documentation
- Monitoring setup
- Launch preparation

**Total: 14 weeks**
