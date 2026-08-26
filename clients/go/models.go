package crawlkit

import "time"

type CrawlRequest struct {
	StartURL       string `json:"start_url"`
	MaxPages       int    `json:"max_pages,omitempty"`
	RequestDelayMs int    `json:"request_delay_ms,omitempty"`
	Concurrency    int    `json:"concurrency,omitempty"`
	TenantID       string `json:"tenant_id,omitempty"`
}

type CrawlResponse struct {
	CrawlID string `json:"crawl_id"`
	Status  string `json:"status"`
	Message string `json:"message"`
}

type CrawlResult struct {
	CrawlID      string     `json:"crawl_id"`
	StartURL     string     `json:"start_url"`
	Status       string     `json:"status"`
	PagesCrawled int        `json:"pages_crawled"`
	IssuesFound  int        `json:"issues_found"`
	CreatedAt    time.Time  `json:"created_at"`
	CompletedAt  *time.Time `json:"completed_at,omitempty"`
}

type CrawlStats struct {
	CrawlID            string             `json:"crawl_id"`
	TotalPages         int                `json:"total_pages"`
	TotalIssues        int                `json:"total_issues"`
	IssuesBySeverity   map[string]int     `json:"issues_by_severity"`
	IssuesByCategory   map[string]int     `json:"issues_by_category"`
	AvgResponseTimeMs  *float64           `json:"avg_response_time_ms,omitempty"`
}

type Finding struct {
	ID             string  `json:"id"`
	PageID         string  `json:"page_id"`
	Category       string  `json:"category"`
	Severity       string  `json:"severity"`
	Code           string  `json:"code"`
	Title          string  `json:"title"`
	Description    string  `json:"description"`
	Element        *string `json:"element,omitempty"`
	Recommendation string  `json:"recommendation"`
}

type BacklinksResponse struct {
	CrawlID               string            `json:"crawl_id"`
	TotalInternalLinks    int               `json:"total_internal_links"`
	TotalExternalLinks    int               `json:"total_external_links"`
	TotalReferringDomains int               `json:"total_referring_domains"`
	OrphanPages           []string          `json:"orphan_pages"`
	TopPagesByPageRank    []PageRankEntry   `json:"top_pages_by_pagerank"`
}

type PageRankEntry struct {
	URL              string  `json:"url"`
	PageRank         float64 `json:"pagerank"`
	InboundLinks     int     `json:"inbound_links"`
	OutboundLinks    int     `json:"outbound_links"`
	ReferringDomains int     `json:"referring_domains"`
}

type User struct {
	ID       string   `json:"id"`
	Email    string   `json:"email"`
	Name     string   `json:"name"`
	TenantID string   `json:"tenant_id"`
	Roles    []string `json:"roles"`
	Enabled  bool     `json:"enabled"`
}

type CreateUserRequest struct {
	Email    string   `json:"email"`
	Name     string   `json:"name"`
	Password string   `json:"password"`
	TenantID string   `json:"tenant_id,omitempty"`
	Roles    []string `json:"roles,omitempty"`
}

type Tenant struct {
	ID        string    `json:"id"`
	Name      string    `json:"name"`
	CreatedAt time.Time `json:"created_at"`
}

type CreateTenantRequest struct {
	ID   string `json:"id"`
	Name string `json:"name"`
}

type ApiKeyInfo struct {
	Key               string `json:"key"`
	Name              string `json:"name"`
	RequestsPerMinute uint32 `json:"requests_per_minute"`
}

type CreateApiKeyRequest struct {
	Name              string `json:"name"`
	RequestsPerMinute uint32 `json:"requests_per_minute,omitempty"`
}

type WebhookConfig struct {
	ID        string    `json:"id"`
	URL       string    `json:"url"`
	Events    []string  `json:"events"`
	CreatedAt time.Time `json:"created_at"`
}

type CreateWebhookRequest struct {
	URL    string   `json:"url"`
	Events []string `json:"events,omitempty"`
}

type ScheduleResponse struct {
	ID           string    `json:"id"`
	StartURL     string    `json:"start_url"`
	IntervalSecs uint64    `json:"interval_secs"`
	Enabled      bool      `json:"enabled"`
	NextRun      time.Time `json:"next_run"`
	CreatedAt    time.Time `json:"created_at"`
}

type CreateScheduleRequest struct {
	StartURL       string `json:"start_url"`
	MaxPages       int    `json:"max_pages,omitempty"`
	RequestDelayMs int    `json:"request_delay_ms,omitempty"`
	Concurrency    int    `json:"concurrency,omitempty"`
	IntervalSecs   uint64 `json:"interval_secs,omitempty"`
}

type AuditEvent struct {
	ID        string    `json:"id"`
	Action    string    `json:"action"`
	Resource  string    `json:"resource"`
	UserID    string    `json:"user_id"`
	Details   string    `json:"details,omitempty"`
	CreatedAt time.Time `json:"created_at"`
}

type HealthResponse struct {
	Status  string `json:"status"`
	Version string `json:"version"`
}

type LoginResponse struct {
	Token string `json:"token"`
	User  User   `json:"user"`
}

type MarketplacePlugin struct {
	Name        string   `json:"name"`
	Version     string   `json:"version"`
	Author      string   `json:"author"`
	Description string   `json:"description"`
	License     string   `json:"license"`
	Categories  []string `json:"categories"`
	Tags        []string `json:"tags"`
	Downloads   uint64   `json:"downloads"`
	Rating      float64  `json:"rating"`
	CreatedAt   string   `json:"created_at"`
	UpdatedAt   string   `json:"updated_at"`
}

type SubmitPluginRequest struct {
	Name        string   `json:"name"`
	Version     string   `json:"version"`
	Author      string   `json:"author"`
	Description string   `json:"description"`
	License     string   `json:"license"`
	Categories  []string `json:"categories"`
	Tags        []string `json:"tags"`
	Repository  *string  `json:"repository,omitempty"`
	Homepage    *string  `json:"homepage,omitempty"`
}

type UpdateScheduleRequest struct {
	StartURL     *string `json:"start_url,omitempty"`
	MaxPages     *int    `json:"max_pages,omitempty"`
	IntervalSecs *uint64 `json:"interval_secs,omitempty"`
	Enabled      *bool   `json:"enabled,omitempty"`
}

type Session struct {
	ID           string    `json:"id"`
	UserID       string    `json:"user_id"`
	TenantID     string    `json:"tenant_id"`
	IPAddress    string    `json:"ip_address"`
	UserAgent    string    `json:"user_agent"`
	CreatedAt    time.Time `json:"created_at"`
	LastActiveAt time.Time `json:"last_active_at"`
	ExpiresAt    time.Time `json:"expires_at"`
}

type PluginTestResult struct {
	Success bool   `json:"success"`
	Message string `json:"message"`
}
