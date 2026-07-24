# Migration Guide

This guide helps you migrate from other SEO tools to crawlkit.

## From Screaming Frog

### Export Data

1. Export crawl data as CSV from Screaming Frog
2. Import into crawlkit:

```bash
# Convert CSV to crawlkit format
crawlkit import screaming-frog-export.csv --format screaming-frog

# Run analysis
crawlkit analyze imported-crawl/ --output results/
```

### Key Differences

| Feature | Screaming Frog | crawlkit |
|---------|---------------|----------|
| Platform | Desktop app | CLI + API |
| Pricing | License per user | Open source + SaaS |
| Analyzers | 100+ | 28 (extensible) |
| Plugins | Java-based | WASM |
| API | Limited | Full REST API |
| Multi-tenant | No | Yes |

## From Ahrefs

### Export Data

1. Export backlink data from Ahrefs
2. Import into crawlkit:

```bash
crawlkit import ahrefs-export.csv --format ahrefs
```

### API Integration

```python
from crawlkit import CrawlkitClient

client = CrawlkitClient(base_url="http://localhost:4000")

# Fetch backlinks from Ahrefs
backlinks = client.fetch_backlinks("ahrefs", "example.com")
```

## From SEMrush

### Export Data

1. Export site audit data from SEMrush
2. Import into crawlkit:

```bash
crawlkit import semrush-export.csv --format semrush
```

## From Custom Scripts

### Python Scripts

```python
# Old way
import requests
response = requests.get("https://example.com")
html = response.text

# New way
from crawlkit import CrawlkitClient, CrawlRequest

client = CrawlkitClient(base_url="http://localhost:4000")
crawl = client.start_crawl(CrawlRequest(start_url="https://example.com"))
```

### Shell Scripts

```bash
# Old way
curl -s https://example.com | grep -o '<title>[^<]*</title>'

# New way
crawlkit inspect https://example.com --format json
```

## Data Format Changes

### crawlkit JSON Format

```json
{
  "crawl_id": "uuid",
  "target_url": "https://example.com",
  "pages_crawled": 50,
  "total_issues": 12,
  "status": "completed"
}
```

### Finding Format

```json
{
  "severity": "warning",
  "category": "seo",
  "code": "META001",
  "title": "Missing meta description",
  "description": "Page lacks meta description",
  "recommendation": "Add meta description"
}
```
