"""Example: Basic crawl using the Python client."""

from crawlkit import CrawlkitClient, CrawlRequest

# Create client
client = CrawlkitClient(
    base_url="http://localhost:4000",
    api_key="your-api-key",
)

# Start a crawl
request = CrawlRequest(
    start_url="https://example.com",
    max_pages=50,
    concurrency=4,
)
response = client.start_crawl(request)
print(f"Crawl started: {response.crawl_id}")

# Check status
status = client.get_crawl(response.crawl_id)
print(f"Status: {status['status']}")

# Get stats
stats = client.get_crawl_stats(response.crawl_id)
print(f"Pages crawled: {stats.total_pages}")
print(f"Issues found: {stats.total_issues}")

client.close()
