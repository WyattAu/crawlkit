# crawlkit Go Client

Go client for the crawlkit REST API.

## Installation

```bash
go get github.com/WyattAu/crawlkit/clients/go
```

## Quick Start

```go
package main

import (
	"context"
	"fmt"
	"log"

	crawlkit "github.com/WyattAu/crawlkit/clients/go"
)

func main() {
	client := crawlkit.NewClient("http://localhost:4000", "your-api-key")
	ctx := context.Background()

	resp, err := client.StartCrawl(ctx, crawlkit.CrawlRequest{
		StartURL: "https://example.com",
		MaxPages: 50,
	})
	if err != nil {
		log.Fatal(err)
	}
	fmt.Printf("Crawl started: %s\n", resp.CrawlID)
}
```

## Authentication

### API Key

```go
client := crawlkit.NewClient("http://localhost:4000", "your-api-key")
```

### JWT Token

```go
ctx := context.Background()
client := crawlkit.NewClient("http://localhost:4000", "")
loginResp, err := client.Login(ctx, "admin@crawlkit.local", "admin123")
if err != nil {
	log.Fatal(err)
}

client = crawlkit.NewClientWithJWT("http://localhost:4000", loginResp.Token)
```

## Error Handling

```go
resp, err := client.GetCrawl(ctx, "nonexistent-id")
if err != nil {
	var apiErr *crawlkit.APIError
	if errors.As(err, &apiErr) {
		switch {
		case crawlkit.IsAuthError(err):
			log.Fatal("Authentication failed")
		case crawlkit.IsNotFoundError(err):
			log.Fatal("Resource not found")
		case crawlkit.IsRateLimitError(err):
			log.Fatal("Rate limit exceeded")
		default:
			log.Fatalf("API error %d: %s", apiErr.StatusCode, apiErr.Message)
		}
	}
}
```

## API Reference

### Health & Metrics

- `Health(ctx)` - Check API health
- `GetMetrics(ctx)` - Get Prometheus metrics

### Authentication

- `Login(ctx, email, password)` - Login and get JWT
- `RefreshToken(ctx)` - Refresh JWT token
- `GetCurrentUser(ctx)` - Get current user info

### Crawls

- `StartCrawl(ctx, req)` - Start a new crawl
- `GetCrawl(ctx, crawlID)` - Get crawl status
- `GetCrawlStats(ctx, crawlID)` - Get crawl statistics
- `GetCrawlFindings(ctx, crawlID)` - Get crawl findings
- `GetCrawlBacklinks(ctx, crawlID)` - Get crawl backlinks
- `ListCrawls(ctx)` - List all crawls

### Users

- `ListUsers(ctx)` - List all users
- `CreateUser(ctx, req)` - Create a new user
- `DeleteUser(ctx, userID)` - Delete a user

### Tenants

- `ListTenants(ctx)` - List all tenants
- `CreateTenant(ctx, req)` - Create a new tenant
- `GetTenant(ctx, tenantID)` - Get tenant details
- `DeleteTenant(ctx, tenantID)` - Delete a tenant

### API Keys

- `ListApiKeys(ctx)` - List all API keys
- `CreateApiKey(ctx, req)` - Create a new API key
- `DeleteApiKey(ctx, key)` - Delete an API key

### Webhooks

- `ListWebhooks(ctx)` - List all webhooks
- `CreateWebhook(ctx, req)` - Create a new webhook
- `DeleteWebhook(ctx, webhookID)` - Delete a webhook

### Schedules

- `ListSchedules(ctx)` - List all schedules
- `CreateSchedule(ctx, req)` - Create a new schedule
- `DeleteSchedule(ctx, scheduleID)` - Delete a schedule

### Audit

- `ListAuditEvents(ctx)` - List audit events

### Plugin Marketplace

- `ListMarketplacePlugins(ctx)` - List all plugins
- `GetMarketplacePlugin(ctx, name)` - Get a plugin
- `SubmitPlugin(ctx, req)` - Submit a plugin
- `DeleteMarketplacePlugin(ctx, name)` - Delete a plugin

## Documentation

- [GitHub Repository](https://github.com/WyattAu/crawlkit)
- [API Reference](https://wyattau.github.io/crawlkit/api-reference/)
