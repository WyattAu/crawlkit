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

## API Reference

### CrawlkitClient

- `constructor(baseURL, apiKey?, jwtToken?)` -- Create client
- `health()` -- Check API health
- `startCrawl(req)` -- Start a crawl
- `getCrawl(crawlId)` -- Get crawl status
- `getCrawlStats(crawlId)` -- Get crawl statistics
- `listUsers()` -- List users
- `createUser(req)` -- Create user
- `deleteUser(userId)` -- Delete user
- `listTenants()` -- List tenants
- `createTenant(name, plan)` -- Create tenant
- `login(email, password)` -- Login and get JWT

## Documentation

- [GitHub Repository](https://github.com/WyattAu/crawlkit)
- [API Reference](https://wyattau.github.io/crawlkit/api-reference/)
