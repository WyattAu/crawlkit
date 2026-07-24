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
	"fmt"
	"log"

	crawlkit "github.com/WyattAu/crawlkit/clients/go"
)

func main() {
	client := crawlkit.NewClient("http://localhost:4000", "your-api-key")

	resp, err := client.StartCrawl(crawlkit.CrawlRequest{
		StartURL: "https://example.com",
		MaxPages: 50,
	})
	if err != nil {
		log.Fatal(err)
	}
	fmt.Printf("Crawl started: %s\n", resp.CrawlID)
}
```

## API Reference

### Client

- `NewClient(baseURL, apiKey)` -- Create client with API key
- `NewClientWithJWT(baseURL, jwtToken)` -- Create client with JWT
- `Health()` -- Check API health
- `StartCrawl(req)` -- Start a crawl
- `GetCrawl(crawlID)` -- Get crawl status
- `GetCrawlStats(crawlID)` -- Get crawl statistics
- `ListUsers()` -- List users
- `CreateUser(req)` -- Create user
- `Login(email, password)` -- Login and get JWT

## Documentation

- [GitHub Repository](https://github.com/WyattAu/crawlkit)
- [API Reference](https://wyattau.github.io/crawlkit/api-reference/)
