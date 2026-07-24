package crawlkit

// CrawlRequest represents a request to start a crawl.
type CrawlRequest struct {
	StartURL       string `json:"start_url"`
	MaxPages       int    `json:"max_pages,omitempty"`
	RequestDelayMs int    `json:"request_delay_ms,omitempty"`
	Concurrency    int    `json:"concurrency,omitempty"`
	TenantID       string `json:"tenant_id,omitempty"`
}

// CrawlResponse represents a response from starting a crawl.
type CrawlResponse struct {
	CrawlID  string `json:"crawl_id"`
	Status   string `json:"status"`
	Message  string `json:"message"`
}

// CrawlStats represents crawl statistics.
type CrawlStats struct {
	TotalPages        int     `json:"total_pages"`
	TotalIssues       int     `json:"total_issues"`
	AvgResponseTimeMs float64 `json:"avg_response_time_ms"`
	TotalBodySize     int     `json:"total_body_size"`
}

// User represents a user in the system.
type User struct {
	ID       string   `json:"id"`
	Email    string   `json:"email"`
	Name     string   `json:"name"`
	TenantID string   `json:"tenant_id"`
	Roles    []string `json:"roles"`
	Enabled  bool     `json:"enabled"`
}

// CreateUserRequest represents a request to create a user.
type CreateUserRequest struct {
	Email    string   `json:"email"`
	Name     string   `json:"name"`
	Password string   `json:"password"`
	TenantID string   `json:"tenant_id"`
	Roles    []string `json:"roles"`
}

// Tenant represents a tenant in the system.
type Tenant struct {
	ID                string `json:"id"`
	Name              string `json:"name"`
	Plan              string `json:"plan"`
	MaxUsers          int    `json:"max_users"`
	MaxCrawlsPerMonth int    `json:"max_crawls_per_month"`
}
