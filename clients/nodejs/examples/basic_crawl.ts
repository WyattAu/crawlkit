import { CrawlkitClient, CrawlRequest } from "../src";

async function main() {
  const client = new CrawlkitClient("http://localhost:4000", "your-api-key");

  // Start a crawl
  const req: CrawlRequest = {
    start_url: "https://example.com",
    max_pages: 50,
    concurrency: 4,
  };

  const resp = await client.startCrawl(req);
  console.log(`Crawl started: ${resp.crawl_id}`);

  // Get crawl status
  const status = await client.getCrawl(resp.crawl_id);
  console.log(`Status: ${status.status}`);

  // Get stats
  const stats = await client.getCrawlStats(resp.crawl_id);
  console.log(`Pages crawled: ${stats.total_pages}`);
  console.log(`Issues found: ${stats.total_issues}`);

  // List users
  const users = await client.listUsers();
  console.log(`Users: ${users.length}`);
}

main().catch(console.error);
