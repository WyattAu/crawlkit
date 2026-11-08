# Agent-Readiness Audit Catalog — analyzer backlog (from isitagentready.com scan of wyattau.com)

Crawlkit must audit sites for ALL of the following checks (fetch each path, validate shape/status/code, emit one finding per check). Implementation: new `agent-readiness` analyzer group (or extend the existing AI-readiness analyzer), fixture-tested. External validation reference: POST https://isitagentready.com/api/scan {"url":...} → checks.<group>.<check>.status == "pass".

## DISCOVERABILITY / AI-READINESS

1. **robots.txt** (RFC 9309): /robots.txt served text/plain HTTP 200; User-agent + Allow/Disallow directives; `Sitemap:` reference when sitemap exists.
2. **sitemap.xml**: /sitemap.xml valid XML HTTP 200; canonical `<url><loc>` entries; referenced from robots.txt.
3. **Content Signals** (contentsignals.org): robots.txt contains `Content-Signal:` directives, e.g. `Content-Signal: ai-train=no, search=yes, ai-input=no`.
4. **AI crawler rules**: explicit User-agent entries for AI bots: GPTBot, OAI-SearchBot, Claude-Web/ClaudeBot, Google-Extended, CCBot, anthropic-ai, Bytespider, Cohere-AI, Applebot-Extended, Amazonbot, Meta-ExternalAgent, Diffbot — with allow/disallow rules matching policy.
5. **API catalog** (RFC 9727): /.well-known/api-catalog, Content-Type application/linkset+json, 200; `linkset` array; each entry `anchor` + relations service-desc/service-doc (+optional status).
6. **Link headers** (RFC 8288 + RFC 9727 §3): homepage returns `Link:` headers with rel api-catalog / service-desc / service-doc / describedby. Multiple headers or comma-separated both valid.
7. **auth.md**: /auth.md markdown with H1 containing "auth.md"; self-contained (agent audience, registration endpoints, credential use) or references OAuth PRM.
8. **OAuth/OIDC discovery** (OIDC discovery / RFC 8414): /.well-known/openid-configuration OR /.well-known/oauth-authorization-server with issuer, authorization_endpoint, token_endpoint, jwks_uri, grant_types_supported, response_types_supported.
9. **OAuth PRM** (RFC 9728): /.well-known/oauth-protected-resource with resource, authorization_servers[]; optional scopes_supported; optional WWW-Authenticate resource_metadata on 401s.
10. **A2A Agent Card**: /.well-known/agent-card.json with name, version, description, supportedInterfaces (service URL + transport), capabilities, skills[] (id/name/description).
11. **Agent Skills index**: /.well-known/agent-skills/index.json with $schema = https://schemas.agentskills.io/discovery/0.2.0/schema.json; skills[] each with name (lowercase+hyphens), type (skill-md|archive), description, url, digest (sha256:{hex}).
12. **MCP Server Card** (SEP-1649): /.well-known/mcp/server-card.json with serverInfo{name,version}, transport endpoint URL, capabilities (tools/resources/prompts).
13. **DNS-AID**: `_agents` SVCB/HTTPS DNS records via DoH (default https://cloudflare-dns.com/dns-query, fallback https://dns.google/resolve), e.g. `_a2a._agents.example.com SVCB 1 agent.example.com. alpn="a2a" port=443`; DNSSEC-signed zones; numeric keyNNNNN for experimental params.
14. **Markdown for Agents**: content negotiation — Accept: text/markdown returns markdown; HTML default for browsers.
15. **WebMCP** (webmachinelearning/webmcp): page JS calls navigator.modelContext.registerTool() with name/description/inputSchema/execute; AbortController cleanup; detected by browser render.
16. **Web Bot Auth** (IETF WebBotAuth WG): /.well-known/http-message-signatures-directory JWKS (≥1 public key); bot requests signed with Signature-Agent + Signature-Input headers.

## COMMERCE

17. **ACP** (agenticcommerce.dev): /.well-known/acp.json — protocol.name="acp", protocol.version, api_base_url (absolute HTTP(S)), transports[] non-empty, capabilities.services[] non-empty.
18. **AP2** (ap2-protocol.org): A2A Agent Card extension entry uri=https://github.com/google-agentic-commerce/AP2/tree/v0.1.0 with params.roles (merchant|shopper|credentials-provider|payment-processor); merchant agents mark required.
19. **MPP** (mpp.dev / paymentauth.org draft): /openapi.json with x-payment-info per payable operation (intent charge|session, method tempo|stripe|lightning|card, amount; optional currency/description) + top-level x-service-info.
20. **UCP** (ucp.dev): /.well-known/ucp with protocol_version, services, capabilities, endpoints; referenced spec URLs reachable.
21. **x402** (x402.org): HTTP 402 responses with payment requirements on protected API routes (middleware-based).

## PRIORITY
P0: 1, 2, 4, 3 (robots/sitemap/AI-rules/content-signals — the crawl-blocking basics)
P1: 11, 12, 5, 6, 10, 14 (agent discovery surface)
P2: 7, 8, 9, 15, 16 (auth + browser/identity)
P3: 13, 17-21 (DNS + commerce — only relevant for commerce/API sites)
