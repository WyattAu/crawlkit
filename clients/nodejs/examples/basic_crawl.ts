import { CrawlkitClient, CrawlRequest } from "../src";

async function main() {
  const client = new CrawlkitClient("http://localhost:4000", "your-api-key");

  const req: CrawlRequest = {
    start_url: "https://example.com",
    max_pages: 50,
    concurrency: 4,
  };

  const resp = await client.startCrawl(req);
  console.log(`Crawl started: ${resp.crawl_id}`);

  const stats = await client.getCrawlStats(resp.crawl_id);
  console.log(`Pages crawled: ${stats.total_pages}`);
  console.log(`Issues found: ${stats.total_issues}`);
}

main().catch(console.error);
