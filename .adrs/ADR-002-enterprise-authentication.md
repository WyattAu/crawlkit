# ADR-002: Enterprise Authentication

## Status
Accepted

## Context
crawlkit needs authentication and authorization for team deployments.

## Decision
Implement JWT-based authentication with RBAC and OIDC delegation.

## Consequences
### Positive
- Stateless JWT tokens (no session storage needed)
- Standard OIDC integration for enterprise SSO
- Role-based access control with hierarchical permissions
- Argon2id password hashing (memory-hard)

### Negative
- JWT tokens cannot be revoked until expiry (need blacklist for immediate revocation)
- OIDC requires external IdP configuration
- More complex than simple API key authentication

### Risks
- JWT secret management (rotation, storage)
- OIDC provider compatibility
- Password brute-force attacks (mitigated by rate limiting)

## Alternatives Considered
- **Session-based auth:** Stateful, simpler but less scalable
- **OAuth2 only:** Requires external IdP, no local auth
- **API keys only:** Simple but no user management

## References
- JWT specification: https://tools.ietf.org/html/rfc7519
- OIDC specification: https://openid.net/specs/openid-connect-core-1_0.html
- Argon2 specification: https://password-hashing.net/argon2-specs.pdf
