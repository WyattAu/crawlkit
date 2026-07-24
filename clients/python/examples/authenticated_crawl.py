"""Example: Authenticated crawl with JWT."""

from crawlkit import CrawlkitClient, CrawlRequest

# Create client
client = CrawlkitClient(base_url="http://localhost:4000")

# Login
token = client.login("admin@crawlkit.local", "admin123")
print(f"Logged in, token: {token[:20]}...")

# Recreate client with JWT
client = CrawlkitClient(
    base_url="http://localhost:4000",
    jwt_token=token,
)

# Start a crawl
request = CrawlRequest(
    start_url="https://example.com",
    max_pages=100,
)
response = client.start_crawl(request)
print(f"Crawl started: {response.crawl_id}")

# List users
users = client.list_users()
print(f"Users: {len(users)}")

# List tenants
tenants = client.list_tenants()
print(f"Tenants: {len(tenants)}")

client.close()
