package main

import (
	"fmt"
	"log"

	crawlkit "github.com/WyattAu/crawlkit/clients/go"
)

func main() {
	client := crawlkit.NewClient("http://localhost:4000", "your-api-key")

	req := crawlkit.CrawlRequest{
		StartURL:    "https://example.com",
		MaxPages:    50,
		Concurrency: 4,
	}
	resp, err := client.StartCrawl(req)
	if err != nil {
		log.Fatal(err)
	}
	fmt.Printf("Crawl started: %s\n", resp.CrawlID)

	stats, err := client.GetCrawlStats(resp.CrawlID)
	if err != nil {
		log.Fatal(err)
	}
	fmt.Printf("Pages crawled: %d\n", stats.TotalPages)
	fmt.Printf("Issues found: %d\n", stats.TotalIssues)
}
