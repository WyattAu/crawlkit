package crawlkit

import (
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"reflect"
	"strconv"
	"testing"
	"time"
)

func newTestServer(t *testing.T, handler http.HandlerFunc) *httptest.Server {
	t.Helper()
	ts := httptest.NewServer(handler)
	t.Cleanup(ts.Close)
	return ts
}

// wantRequest is a helper that records details about the incoming request so
// tests can assert method, path, and headers.
type recordedRequest struct {
	Method      string
	Path        string
	APIKey      string
	ContentType string
	Body        []byte
}

func recorder(rec *recordedRequest, next http.HandlerFunc) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if rec != nil {
			rec.Method = r.Method
			rec.Path = r.URL.Path
			rec.APIKey = r.Header.Get("X-API-Key")
			rec.ContentType = r.Header.Get("Content-Type")
			rec.Body, _ = readAll(r)
		}
		if next != nil {
			next(w, r)
		}
	}
}

func readAll(r *http.Request) ([]byte, error) {
	if r.Body == nil {
		return nil, nil
	}
	defer r.Body.Close()
	buf := make([]byte, 0, 1024)
	tmp := make([]byte, 512)
	for {
		n, err := r.Body.Read(tmp)
		buf = append(buf, tmp[:n]...)
		if err != nil {
			break
		}
	}
	return buf, nil
}

func writeJSON(t *testing.T, w http.ResponseWriter, status int, body string) {
	t.Helper()
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	if _, err := w.Write([]byte(body)); err != nil {
		t.Errorf("failed to write response body: %v", err)
	}
}

// TestStartCrawl covers a successful crawl start against the documented API
// contract (202 Accepted with a JSON body) and other accepted 2xx codes.
func TestStartCrawl(t *testing.T) {
	tests := []struct {
		name       string
		statusCode int
	}{
		{"accepted_202", http.StatusAccepted},
		{"created_201", http.StatusCreated},
		{"ok_200", http.StatusOK},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			var rec recordedRequest
			ts := newTestServer(t, recorder(&rec, func(w http.ResponseWriter, r *http.Request) {
				writeJSON(t, w, tt.statusCode, `{"crawl_id":"c1","status":"running","message":"ok"}`)
			}))

			client := NewClient(ts.URL, "test-key")
			got, err := client.StartCrawl(context.Background(), CrawlRequest{
				StartURL:       "https://example.com",
				MaxPages:       50,
				RequestDelayMs: 100,
				Concurrency:    4,
				TenantID:       "tenant-1",
			})
			if err != nil {
				t.Fatalf("StartCrawl() error = %v, want nil", err)
			}

			// Verify the outgoing request.
			if rec.Method != http.MethodPost {
				t.Errorf("request method = %q, want POST", rec.Method)
			}
			if rec.Path != "/api/v1/crawls" {
				t.Errorf("request path = %q, want /api/v1/crawls", rec.Path)
			}
			if rec.APIKey != "test-key" {
				t.Errorf("X-API-Key header = %q, want %q", rec.APIKey, "test-key")
			}
			if rec.ContentType != "application/json" {
				t.Errorf("Content-Type header = %q, want application/json", rec.ContentType)
			}

			var sent CrawlRequest
			if err := json.Unmarshal(rec.Body, &sent); err != nil {
				t.Fatalf("request body is not valid JSON: %v", err)
			}
			wantReq := CrawlRequest{
				StartURL:       "https://example.com",
				MaxPages:       50,
				RequestDelayMs: 100,
				Concurrency:    4,
				TenantID:       "tenant-1",
			}
			if !reflect.DeepEqual(sent, wantReq) {
				t.Errorf("request body = %+v, want %+v", sent, wantReq)
			}

			// Verify the parsed response.
			want := &CrawlResponse{CrawlID: "c1", Status: "running", Message: "ok"}
			if !reflect.DeepEqual(got, want) {
				t.Errorf("StartCrawl() = %+v, want %+v", got, want)
			}
		})
	}
}

// TestStartCrawlJWT verifies JWT auth uses the Authorization header.
func TestStartCrawlJWT(t *testing.T) {
	var authHeader string
	ts := newTestServer(t, func(w http.ResponseWriter, r *http.Request) {
		authHeader = r.Header.Get("Authorization")
		writeJSON(t, w, http.StatusAccepted, `{"crawl_id":"c1","status":"running","message":"ok"}`)
	})

	client := NewClientWithJWT(ts.URL, "jwt-token")
	if _, err := client.StartCrawl(context.Background(), CrawlRequest{StartURL: "https://example.com"}); err != nil {
		t.Fatalf("StartCrawl() error = %v, want nil", err)
	}
	if authHeader != "Bearer jwt-token" {
		t.Errorf("Authorization header = %q, want %q", authHeader, "Bearer jwt-token")
	}
}

func TestGetEndpoints(t *testing.T) {
	created := time.Date(2024, 1, 15, 10, 30, 0, 0, time.UTC)

	tests := []struct {
		name    string
		handler http.HandlerFunc
		call    func(c *Client) (interface{}, error)
		want    interface{}
	}{
		{
			name: "get_crawl",
			handler: func(w http.ResponseWriter, r *http.Request) {
				writeJSON(t, w, http.StatusOK, `{
					"crawl_id":"c1",
					"start_url":"https://example.com",
					"status":"completed",
					"pages_crawled":42,
					"issues_found":7,
					"created_at":"2024-01-15T10:30:00Z",
					"completed_at":"2024-01-15T11:00:00Z"
				}`)
			},
			call: func(c *Client) (interface{}, error) { return c.GetCrawl(context.Background(), "c1") },
			want: &CrawlResult{
				CrawlID:      "c1",
				StartURL:     "https://example.com",
				Status:       "completed",
				PagesCrawled: 42,
				IssuesFound:  7,
				CreatedAt:    created,
				CompletedAt:  ptrTime(time.Date(2024, 1, 15, 11, 0, 0, 0, time.UTC)),
			},
		},
		{
			name: "get_crawl_stats",
			handler: func(w http.ResponseWriter, r *http.Request) {
				writeJSON(t, w, http.StatusOK, `{
					"crawl_id":"c1",
					"total_pages":42,
					"total_issues":7,
					"issues_by_severity":{"critical":1,"warning":6},
					"issues_by_category":{"meta":3,"links":4},
					"avg_response_time_ms":123.5
				}`)
			},
			call: func(c *Client) (interface{}, error) { return c.GetCrawlStats(context.Background(), "c1") },
			want: &CrawlStats{
				CrawlID:           "c1",
				TotalPages:        42,
				TotalIssues:       7,
				IssuesBySeverity:  map[string]int{"critical": 1, "warning": 6},
				IssuesByCategory:  map[string]int{"meta": 3, "links": 4},
				AvgResponseTimeMs: ptrFloat(123.5),
			},
		},
		{
			name: "get_crawl_findings",
			handler: func(w http.ResponseWriter, r *http.Request) {
				writeJSON(t, w, http.StatusOK, `[
					{
						"id":"f1",
						"page_id":"p1",
						"category":"meta",
						"severity":"critical",
						"code":"missing_title",
						"title":"Missing title",
						"description":"Page has no title",
						"element":"<head>",
						"recommendation":"Add a title"
					},
					{
						"id":"f2",
						"page_id":"p2",
						"category":"links",
						"severity":"warning",
						"code":"broken_link",
						"title":"Broken link",
						"description":"Link returns 404",
						"recommendation":"Fix or remove the link"
					}
				]`)
			},
			call: func(c *Client) (interface{}, error) { return c.GetCrawlFindings(context.Background(), "c1") },
			want: []Finding{
				{
					ID: "f1", PageID: "p1", Category: "meta", Severity: "critical",
					Code: "missing_title", Title: "Missing title",
					Description: "Page has no title", Element: ptrString("<head>"),
					Recommendation: "Add a title",
				},
				{
					ID: "f2", PageID: "p2", Category: "links", Severity: "warning",
					Code: "broken_link", Title: "Broken link",
					Description:    "Link returns 404",
					Recommendation: "Fix or remove the link",
				},
			},
		},
		{
			name: "list_crawls",
			handler: func(w http.ResponseWriter, r *http.Request) {
				writeJSON(t, w, http.StatusOK, `[
					{"crawl_id":"c1","start_url":"https://example.com","status":"completed","pages_crawled":42,"issues_found":7,"created_at":"2024-01-15T10:30:00Z"}
				]`)
			},
			call: func(c *Client) (interface{}, error) { return c.ListCrawls(context.Background()) },
			want: []CrawlResult{
				{
					CrawlID: "c1", StartURL: "https://example.com", Status: "completed",
					PagesCrawled: 42, IssuesFound: 7, CreatedAt: created,
				},
			},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			var rec recordedRequest
			ts := newTestServer(t, recorder(&rec, tt.handler))
			client := NewClient(ts.URL, "test-key")

			got, err := tt.call(client)
			if err != nil {
				t.Fatalf("%s() error = %v, want nil", tt.name, err)
			}
			if !reflect.DeepEqual(got, tt.want) {
				t.Errorf("%s() = %+v, want %+v", tt.name, got, tt.want)
			}

			if rec.Method != http.MethodGet {
				t.Errorf("request method = %q, want GET", rec.Method)
			}
			if rec.APIKey != "test-key" {
				t.Errorf("X-API-Key header = %q, want %q", rec.APIKey, "test-key")
			}
		})
	}
}

func TestGetCrawlRequestPath(t *testing.T) {
	paths := []struct {
		crawlID string
		want    string
	}{
		{"c1", "/api/v1/crawls/c1"},
		{"abc-123", "/api/v1/crawls/abc-123"},
	}
	for _, tt := range paths {
		t.Run(tt.crawlID, func(t *testing.T) {
			var rec recordedRequest
			ts := newTestServer(t, recorder(&rec, func(w http.ResponseWriter, r *http.Request) {
				writeJSON(t, w, http.StatusOK, `{"crawl_id":"x","start_url":"y","status":"running","pages_crawled":0,"issues_found":0,"created_at":"2024-01-15T10:30:00Z"}`)
			}))
			client := NewClient(ts.URL, "test-key")
			if _, err := client.GetCrawl(context.Background(), tt.crawlID); err != nil {
				t.Fatalf("GetCrawl() error = %v, want nil", err)
			}
			if rec.Path != tt.want {
				t.Errorf("request path = %q, want %q", rec.Path, tt.want)
			}
		})
	}
}

func TestErrorMapping(t *testing.T) {
	tests := []struct {
		name         string
		statusCode   int
		wantMessage  string
		isAuth       bool
		isNotFound   bool
		isRateLimit  bool
		isValidation bool
	}{
		{name: "unauthorized_401", statusCode: 401, wantMessage: "invalid api key", isAuth: true},
		{name: "not_found_404", statusCode: 404, wantMessage: "crawl not found", isNotFound: true},
		{name: "rate_limited_429", statusCode: 429, wantMessage: "too many requests", isRateLimit: true},
		{name: "server_error_500", statusCode: 500, wantMessage: "internal error"},
	}

	for _, tt := range tests {
		t.Run(tt.name+"_get", func(t *testing.T) {
			ts := newTestServer(t, func(w http.ResponseWriter, r *http.Request) {
				writeJSON(t, w, tt.statusCode, `{"error":"`+tt.wantMessage+`","status":`+strconv.Itoa(tt.statusCode)+`}`)
			})
			client := NewClient(ts.URL, "test-key")

			_, err := client.GetCrawl(context.Background(), "c1")
			if err == nil {
				t.Fatal("GetCrawl() error = nil, want *APIError")
			}

			apiErr, ok := err.(*APIError)
			if !ok {
				t.Fatalf("error type = %T, want *APIError", err)
			}
			if apiErr.StatusCode != tt.statusCode {
				t.Errorf("APIError.StatusCode = %d, want %d", apiErr.StatusCode, tt.statusCode)
			}
			if apiErr.Message != tt.wantMessage {
				t.Errorf("APIError.Message = %q, want %q", apiErr.Message, tt.wantMessage)
			}
			if got := IsAuthError(err); got != tt.isAuth {
				t.Errorf("IsAuthError() = %v, want %v", got, tt.isAuth)
			}
			if got := IsNotFoundError(err); got != tt.isNotFound {
				t.Errorf("IsNotFoundError() = %v, want %v", got, tt.isNotFound)
			}
			if got := IsRateLimitError(err); got != tt.isRateLimit {
				t.Errorf("IsRateLimitError() = %v, want %v", got, tt.isRateLimit)
			}
			if got := IsValidationError(err); got != tt.isValidation {
				t.Errorf("IsValidationError() = %v, want %v", got, tt.isValidation)
			}
		})
	}

	// The same mapping must apply to POST-based calls (e.g. StartCrawl).
	t.Run("start_crawl_401", func(t *testing.T) {
		ts := newTestServer(t, func(w http.ResponseWriter, r *http.Request) {
			writeJSON(t, w, 401, `{"error":"invalid api key","status":401}`)
		})
		client := NewClient(ts.URL, "bad-key")
		_, err := client.StartCrawl(context.Background(), CrawlRequest{StartURL: "https://example.com"})
		if !IsAuthError(err) {
			t.Fatalf("IsAuthError(StartCrawl err) = false, want true (err = %v)", err)
		}
	})

	// Non-JSON error bodies fall back to the status code as the message.
	t.Run("non_json_error_body", func(t *testing.T) {
		ts := newTestServer(t, func(w http.ResponseWriter, r *http.Request) {
			w.WriteHeader(500)
			if _, err := w.Write([]byte("oops")); err != nil {
				t.Errorf("failed to write body: %v", err)
			}
		})
		client := NewClient(ts.URL, "test-key")
		_, err := client.GetCrawl(context.Background(), "c1")
		apiErr, ok := err.(*APIError)
		if !ok {
			t.Fatalf("error type = %T, want *APIError", err)
		}
		if apiErr.StatusCode != 500 || apiErr.Message != "500" {
			t.Errorf("APIError = {StatusCode:%d, Message:%q}, want {500, \"500\"}", apiErr.StatusCode, apiErr.Message)
		}
	})
}

func TestInvalidBaseURL(t *testing.T) {
	requests := 0
	ts := newTestServer(t, func(w http.ResponseWriter, r *http.Request) {
		requests++
	})
	_ = ts

	client := NewClient("://invalid-url", "test-key")
	ctx := context.Background()

	if _, err := client.GetCrawl(ctx, "c1"); err == nil {
		t.Error("GetCrawl() with invalid base URL: error = nil, want error")
	}
	if _, err := client.StartCrawl(ctx, CrawlRequest{StartURL: "https://example.com"}); err == nil {
		t.Error("StartCrawl() with invalid base URL: error = nil, want error")
	}
	if requests != 0 {
		t.Errorf("server received %d requests, want 0 (request must not be sent)", requests)
	}
}

func ptrString(s string) *string { return &s }

func ptrTime(t time.Time) *time.Time { return &t }

func ptrFloat(f float64) *float64 { return &f }
