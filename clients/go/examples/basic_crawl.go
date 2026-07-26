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

	// Start a crawl
	resp, err := client.StartCrawl(ctx, crawlkit.CrawlRequest{
		StartURL:    "https://example.com",
		MaxPages:    50,
		Concurrency: 4,
	})
	if err != nil {
		log.Fatal(err)
	}
	fmt.Printf("Crawl started: %s\n", resp.CrawlID)

	// Get crawl status
	status, err := client.GetCrawl(ctx, resp.CrawlID)
	if err != nil {
		log.Fatal(err)
	}
	fmt.Printf("Status: %s\n", status.Status)

	// Get stats
	stats, err := client.GetCrawlStats(ctx, resp.CrawlID)
	if err != nil {
		log.Fatal(err)
	}
	fmt.Printf("Pages crawled: %d\n", stats.TotalPages)
	fmt.Printf("Issues found: %d\n", stats.TotalIssues)

	// List users
	users, err := client.ListUsers(ctx)
	if err != nil {
		log.Fatal(err)
	}
	fmt.Printf("Users: %d\n", len(users))
}
