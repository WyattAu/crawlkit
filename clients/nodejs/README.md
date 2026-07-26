# crawlkit Node.js Client

Node.js client for the crawlkit REST API.

## Installation

```bash
npm install crawlkit-nodejs
```

## Quick Start

```typescript
import { CrawlkitClient, CrawlRequest } from "crawlkit-nodejs";

const client = new CrawlkitClient("http://localhost:4000", "your-api-key");

const req: CrawlRequest = {
  start_url: "https://example.com",
  max_pages: 50,
};

const resp = await client.startCrawl(req);
console.log(`Crawl started: ${resp.crawl_id}`);
```

## Authentication

### API Key

```typescript
const client = new CrawlkitClient("http://localhost:4000", "your-api-key");
```

### JWT Token

```typescript
const client = new CrawlkitClient("http://localhost:4000");
const loginResp = await client.login("admin@crawlkit.local", "admin123");

const authClient = new CrawlkitClient(
  "http://localhost:4000",
  "",
  loginResp.token,
);
```

## Error Handling

```typescript
import {
  CrawlkitError,
  AuthenticationError,
  NotFoundError,
  RateLimitError,
} from "crawlkit-nodejs";

try {
  await client.getCrawl("nonexistent-id");
} catch (err) {
  if (err instanceof AuthenticationError) {
    console.error("Authentication failed");
  } else if (err instanceof NotFoundError) {
    console.error("Resource not found");
  } else if (err instanceof RateLimitError) {
    console.error("Rate limit exceeded");
  } else if (err instanceof CrawlkitError) {
    console.error(`API error ${err.statusCode}: ${err.message}`);
  }
}
```

## API Reference

### Health & Metrics

- `health()` - Check API health
- `getMetrics()` - Get Prometheus metrics

### Authentication

- `login(email, password)` - Login and get JWT
- `refreshToken()` - Refresh JWT token
- `getCurrentUser()` - Get current user info

### Crawls

- `startCrawl(req)` - Start a new crawl
- `getCrawl(crawlId)` - Get crawl status
- `getCrawlStats(crawlId)` - Get crawl statistics
- `getCrawlFindings(crawlId)` - Get crawl findings
- `getCrawlBacklinks(crawlId)` - Get crawl backlinks
- `listCrawls()` - List all crawls

### Users

- `listUsers()` - List all users
- `createUser(req)` - Create a new user
- `deleteUser(userId)` - Delete a user

### Tenants

- `listTenants()` - List all tenants
- `createTenant(req)` - Create a new tenant
- `getTenant(tenantId)` - Get tenant details
- `deleteTenant(tenantId)` - Delete a tenant

### API Keys

- `listApiKeys()` - List all API keys
- `createApiKey(req)` - Create a new API key
- `deleteApiKey(key)` - Delete an API key

### Webhooks

- `listWebhooks()` - List all webhooks
- `createWebhook(req)` - Create a new webhook
- `deleteWebhook(webhookId)` - Delete a webhook

### Schedules

- `listSchedules()` - List all schedules
- `createSchedule(req)` - Create a new schedule
- `deleteSchedule(scheduleId)` - Delete a schedule

### Audit

- `listAuditEvents()` - List audit events

### Plugin Marketplace

- `listMarketplacePlugins()` - List all plugins
- `getMarketplacePlugin(name)` - Get a plugin
- `submitPlugin(req)` - Submit a plugin
- `deleteMarketplacePlugin(name)` - Delete a plugin

## Documentation

- [GitHub Repository](https://github.com/WyattAu/crawlkit)
- [API Reference](https://wyattau.github.io/crawlkit/api-reference/)
