// Package crawlkit provides a Go client for the crawlkit REST API.
package crawlkit

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"time"
)

// Client represents a crawlkit API client.
type Client struct {
	BaseURL    string
	APIKey     string
	JWTToken   string
	HTTPClient *http.Client
}

// NewClient creates a new crawlkit client.
func NewClient(baseURL, apiKey string) *Client {
	return &Client{
		BaseURL:    baseURL,
		APIKey:     apiKey,
		HTTPClient: &http.Client{Timeout: 30 * time.Second},
	}
}

// NewClientWithJWT creates a new crawlkit client with JWT authentication.
func NewClientWithJWT(baseURL, jwtToken string) *Client {
	return &Client{
		BaseURL:    baseURL,
		JWTToken:   jwtToken,
		HTTPClient: &http.Client{Timeout: 30 * time.Second},
	}
}

// Health checks API health.
func (c *Client) Health() (map[string]interface{}, error) {
	var result map[string]interface{}
	err := c.get("/health", &result)
	return result, err
}

// StartCrawl starts a new crawl.
func (c *Client) StartCrawl(req CrawlRequest) (*CrawlResponse, error) {
	var result CrawlResponse
	err := c.post("/api/v1/crawls", req, &result)
	return &result, err
}

// GetCrawl gets crawl status.
func (c *Client) GetCrawl(crawlID string) (map[string]interface{}, error) {
	var result map[string]interface{}
	err := c.get(fmt.Sprintf("/api/v1/crawls/%s", crawlID), &result)
	return result, err
}

// GetCrawlStats gets crawl statistics.
func (c *Client) GetCrawlStats(crawlID string) (*CrawlStats, error) {
	var result CrawlStats
	err := c.get(fmt.Sprintf("/api/v1/crawls/%s/stats", crawlID), &result)
	return &result, err
}

// ListUsers lists all users.
func (c *Client) ListUsers() ([]User, error) {
	var result []User
	err := c.get("/api/v1/users", &result)
	return result, err
}

// CreateUser creates a new user.
func (c *Client) CreateUser(req CreateUserRequest) (*User, error) {
	var result User
	err := c.post("/api/v1/users", req, &result)
	return &result, err
}

// Login authenticates and returns JWT token.
func (c *Client) Login(email, password string) (string, error) {
	req := map[string]string{
		"email":    email,
		"password": password,
	}
	var result map[string]string
	err := c.post("/api/v1/auth/login", req, &result)
	if err != nil {
		return "", err
	}
	return result["token"], nil
}

func (c *Client) get(path string, result interface{}) error {
	req, err := http.NewRequest("GET", c.BaseURL+path, nil)
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
		return fmt.Errorf("HTTP %d", resp.StatusCode)
	}

	return json.NewDecoder(resp.Body).Decode(result)
}

func (c *Client) post(path string, body interface{}, result interface{}) error {
	jsonBody, err := json.Marshal(body)
	if err != nil {
		return err
	}

	req, err := http.NewRequest("POST", c.BaseURL+path, bytes.NewBuffer(jsonBody))
	if err != nil {
		return err
	}
	req.Header.Set("Content-Type", "application/json")
	c.addHeaders(req)

	resp, err := c.HTTPClient.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK && resp.StatusCode != http.StatusCreated {
		bodyBytes, _ := io.ReadAll(resp.Body)
		return fmt.Errorf("HTTP %d: %s", resp.StatusCode, string(bodyBytes))
	}

	return json.NewDecoder(resp.Body).Decode(result)
}

func (c *Client) addHeaders(req *http.Request) {
	if c.APIKey != "" {
		req.Header.Set("X-API-Key", c.APIKey)
	}
	if c.JWTToken != "" {
		req.Header.Set("Authorization", "Bearer "+c.JWTToken)
	}
}
