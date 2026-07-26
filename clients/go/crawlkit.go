package crawlkit

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strconv"
	"time"
)

type Client struct {
	BaseURL    string
	APIKey     string
	JWTToken   string
	HTTPClient *http.Client
}

func NewClient(baseURL, apiKey string) *Client {
	return &Client{
		BaseURL:    baseURL,
		APIKey:     apiKey,
		HTTPClient: &http.Client{Timeout: 30 * time.Second},
	}
}

func NewClientWithJWT(baseURL, jwtToken string) *Client {
	return &Client{
		BaseURL:    baseURL,
		JWTToken:   jwtToken,
		HTTPClient: &http.Client{Timeout: 30 * time.Second},
	}
}

func (c *Client) SetTimeout(d time.Duration) {
	c.HTTPClient.Timeout = d
}

func (c *Client) Health(ctx context.Context) (*HealthResponse, error) {
	var result HealthResponse
	err := c.get(ctx, "/health", &result)
	return &result, err
}

func (c *Client) GetMetrics(ctx context.Context) (string, error) {
	req, err := http.NewRequestWithContext(ctx, "GET", c.BaseURL+"/metrics", nil)
	if err != nil {
		return "", err
	}
	c.addHeaders(req)

	resp, err := c.HTTPClient.Do(req)
	if err != nil {
		return "", err
	}
	defer resp.Body.Close()

	bodyBytes, err := io.ReadAll(resp.Body)
	if err != nil {
		return "", err
	}

	if resp.StatusCode != http.StatusOK {
		return "", &APIError{
			StatusCode: resp.StatusCode,
			Message:    string(bodyBytes),
		}
	}

	return string(bodyBytes), nil
}

func (c *Client) Login(ctx context.Context, email, password string) (*LoginResponse, error) {
	req := map[string]string{
		"email":    email,
		"password": password,
	}
	var result LoginResponse
	err := c.post(ctx, "/api/v1/auth/login", req, &result)
	return &result, err
}

func (c *Client) RefreshToken(ctx context.Context) (*LoginResponse, error) {
	var result LoginResponse
	err := c.post(ctx, "/api/v1/auth/refresh", nil, &result)
	return &result, err
}

func (c *Client) GetCurrentUser(ctx context.Context) (*User, error) {
	var result User
	err := c.get(ctx, "/api/v1/auth/me", &result)
	return &result, err
}

func (c *Client) StartCrawl(ctx context.Context, req CrawlRequest) (*CrawlResponse, error) {
	var result CrawlResponse
	err := c.post(ctx, "/api/v1/crawls", req, &result)
	return &result, err
}

func (c *Client) GetCrawl(ctx context.Context, crawlID string) (*CrawlResult, error) {
	var result CrawlResult
	err := c.get(ctx, fmt.Sprintf("/api/v1/crawls/%s", crawlID), &result)
	return &result, err
}

func (c *Client) GetCrawlStats(ctx context.Context, crawlID string) (*CrawlStats, error) {
	var result CrawlStats
	err := c.get(ctx, fmt.Sprintf("/api/v1/crawls/%s/stats", crawlID), &result)
	return &result, err
}

func (c *Client) GetCrawlFindings(ctx context.Context, crawlID string) ([]Finding, error) {
	var result []Finding
	err := c.get(ctx, fmt.Sprintf("/api/v1/crawls/%s/findings", crawlID), &result)
	return result, err
}

func (c *Client) GetCrawlBacklinks(ctx context.Context, crawlID string) (*BacklinksResponse, error) {
	var result BacklinksResponse
	err := c.get(ctx, fmt.Sprintf("/api/v1/crawls/%s/backlinks", crawlID), &result)
	return &result, err
}

func (c *Client) ListCrawls(ctx context.Context) ([]CrawlResult, error) {
	var result []CrawlResult
	err := c.get(ctx, "/api/v1/crawls", &result)
	return result, err
}

func (c *Client) ListUsers(ctx context.Context) ([]User, error) {
	var result []User
	err := c.get(ctx, "/api/v1/users", &result)
	return result, err
}

func (c *Client) CreateUser(ctx context.Context, req CreateUserRequest) (*User, error) {
	var result User
	err := c.post(ctx, "/api/v1/users", req, &result)
	return &result, err
}

func (c *Client) DeleteUser(ctx context.Context, userID string) error {
	return c.delete(ctx, fmt.Sprintf("/api/v1/users/%s", userID))
}

func (c *Client) ListTenants(ctx context.Context) ([]Tenant, error) {
	var result []Tenant
	err := c.get(ctx, "/api/v1/tenants", &result)
	return result, err
}

func (c *Client) CreateTenant(ctx context.Context, req CreateTenantRequest) (*Tenant, error) {
	var result Tenant
	err := c.post(ctx, "/api/v1/tenants", req, &result)
	return &result, err
}

func (c *Client) GetTenant(ctx context.Context, tenantID string) (*Tenant, error) {
	var result Tenant
	err := c.get(ctx, fmt.Sprintf("/api/v1/tenants/%s", tenantID), &result)
	return &result, err
}

func (c *Client) DeleteTenant(ctx context.Context, tenantID string) error {
	return c.delete(ctx, fmt.Sprintf("/api/v1/tenants/%s", tenantID))
}

func (c *Client) CreateApiKey(ctx context.Context, req CreateApiKeyRequest) (*ApiKeyInfo, error) {
	var result ApiKeyInfo
	err := c.post(ctx, "/api/v1/keys", req, &result)
	return &result, err
}

func (c *Client) ListApiKeys(ctx context.Context) ([]ApiKeyInfo, error) {
	var result []ApiKeyInfo
	err := c.get(ctx, "/api/v1/keys", &result)
	return result, err
}

func (c *Client) DeleteApiKey(ctx context.Context, key string) error {
	return c.delete(ctx, fmt.Sprintf("/api/v1/keys/%s", key))
}

func (c *Client) CreateWebhook(ctx context.Context, req CreateWebhookRequest) (*WebhookConfig, error) {
	var result WebhookConfig
	err := c.post(ctx, "/api/v1/webhooks", req, &result)
	return &result, err
}

func (c *Client) ListWebhooks(ctx context.Context) ([]WebhookConfig, error) {
	var result []WebhookConfig
	err := c.get(ctx, "/api/v1/webhooks", &result)
	return result, err
}

func (c *Client) DeleteWebhook(ctx context.Context, webhookID string) error {
	return c.delete(ctx, fmt.Sprintf("/api/v1/webhooks/%s", webhookID))
}

func (c *Client) CreateSchedule(ctx context.Context, req CreateScheduleRequest) (*ScheduleResponse, error) {
	var result ScheduleResponse
	err := c.post(ctx, "/api/v1/schedules", req, &result)
	return &result, err
}

func (c *Client) ListSchedules(ctx context.Context) ([]ScheduleResponse, error) {
	var result []ScheduleResponse
	err := c.get(ctx, "/api/v1/schedules", &result)
	return result, err
}

func (c *Client) DeleteSchedule(ctx context.Context, scheduleID string) error {
	return c.delete(ctx, fmt.Sprintf("/api/v1/schedules/%s", scheduleID))
}

func (c *Client) ListAuditEvents(ctx context.Context) ([]AuditEvent, error) {
	var result []AuditEvent
	err := c.get(ctx, "/api/v1/audit", &result)
	return result, err
}

func (c *Client) ListMarketplacePlugins(ctx context.Context) ([]MarketplacePlugin, error) {
	var result []MarketplacePlugin
	err := c.get(ctx, "/api/v1/marketplace/plugins", &result)
	return result, err
}

func (c *Client) GetMarketplacePlugin(ctx context.Context, name string) (*MarketplacePlugin, error) {
	var result MarketplacePlugin
	err := c.get(ctx, fmt.Sprintf("/api/v1/marketplace/plugins/%s", name), &result)
	return &result, err
}

func (c *Client) SubmitPlugin(ctx context.Context, req SubmitPluginRequest) (*MarketplacePlugin, error) {
	var result MarketplacePlugin
	err := c.post(ctx, "/api/v1/marketplace/plugins", req, &result)
	return &result, err
}

func (c *Client) DeleteMarketplacePlugin(ctx context.Context, name string) error {
	return c.delete(ctx, fmt.Sprintf("/api/v1/marketplace/plugins/%s", name))
}

func (c *Client) get(ctx context.Context, path string, result interface{}) error {
	req, err := http.NewRequestWithContext(ctx, "GET", c.BaseURL+path, nil)
	if err != nil {
		return err
	}
	c.addHeaders(req)

	resp, err := c.HTTPClient.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return parseErrorResponse(resp)
	}

	return json.NewDecoder(resp.Body).Decode(result)
}

func (c *Client) post(ctx context.Context, path string, body interface{}, result interface{}) error {
	var reqBody io.Reader
	if body != nil {
		jsonBody, err := json.Marshal(body)
		if err != nil {
			return err
		}
		reqBody = bytes.NewBuffer(jsonBody)
	}

	req, err := http.NewRequestWithContext(ctx, "POST", c.BaseURL+path, reqBody)
	if err != nil {
		return err
	}
	if body != nil {
		req.Header.Set("Content-Type", "application/json")
	}
	c.addHeaders(req)

	resp, err := c.HTTPClient.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK && resp.StatusCode != http.StatusCreated {
		return parseErrorResponse(resp)
	}

	if result == nil {
		return nil
	}
	return json.NewDecoder(resp.Body).Decode(result)
}

func (c *Client) delete(ctx context.Context, path string) error {
	req, err := http.NewRequestWithContext(ctx, "DELETE", c.BaseURL+path, nil)
	if err != nil {
		return err
	}
	c.addHeaders(req)

	resp, err := c.HTTPClient.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK && resp.StatusCode != http.StatusNoContent {
		return parseErrorResponse(resp)
	}

	return nil
}

func (c *Client) addHeaders(req *http.Request) {
	if c.APIKey != "" {
		req.Header.Set("X-API-Key", c.APIKey)
	}
	if c.JWTToken != "" {
		req.Header.Set("Authorization", "Bearer "+c.JWTToken)
	}
}

func parseErrorResponse(resp *http.Response) error {
	bodyBytes, _ := io.ReadAll(resp.Body)

	var errResp struct {
		Error  string `json:"error"`
		Status int    `json:"status"`
	}
	if err := json.Unmarshal(bodyBytes, &errResp); err == nil && errResp.Error != "" {
		return &APIError{
			StatusCode: resp.StatusCode,
			Message:    errResp.Error,
		}
	}

	return &APIError{
		StatusCode: resp.StatusCode,
		Message:    strconv.Itoa(resp.StatusCode),
	}
}
