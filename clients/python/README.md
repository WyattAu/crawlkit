# crawlkit Python Client

Python client for the crawlkit REST API.

## Installation

```bash
pip install crawlkit-python
```

## Quick Start

```python
from crawlkit import CrawlkitClient, CrawlRequest

client = CrawlkitClient(
    base_url="http://localhost:4000",
    api_key="your-api-key",
)

# Start a crawl
request = CrawlRequest(
    start_url="https://example.com",
    max_pages=50,
)
response = client.start_crawl(request)
print(f"Crawl started: {response.crawl_id}")

# Check status
status = client.get_crawl(response.crawl_id)
print(f"Status: {status['status']}")

client.close()
```

## Authentication

### API Key

```python
client = CrawlkitClient(
    base_url="http://localhost:4000",
    api_key="your-api-key",
)
```

### JWT Token

```python
# Login
client = CrawlkitClient(base_url="http://localhost:4000")
token = client.login("admin@crawlkit.local", "admin123")

# Use JWT
client = CrawlkitClient(
    base_url="http://localhost:4000",
    jwt_token=token,
)
```

## API Reference

### CrawlkitClient

- `health()` - Check API health
- `start_crawl(request)` - Start a new crawl
- `get_crawl(crawl_id)` - Get crawl status
- `get_crawl_stats(crawl_id)` - Get crawl statistics
- `list_crawls()` - List all crawls
- `list_users()` - List all users
- `create_user(...)` - Create a new user
- `delete_user(user_id)` - Delete a user
- `list_tenants()` - List all tenants
- `create_tenant(name, plan)` - Create a new tenant
- `get_tenant(tenant_id)` - Get tenant details
- `delete_tenant(tenant_id)` - Delete a tenant
- `login(email, password)` - Login and get JWT
- `get_current_user()` - Get current user
- `get_metrics()` - Get Prometheus metrics

## Documentation

- [GitHub Repository](https://github.com/WyattAu/crawlkit)
- [API Reference](https://wyattau.github.io/crawlkit/api-reference/)
