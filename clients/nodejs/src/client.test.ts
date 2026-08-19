import { CrawlkitClient } from "./client";
import {
  AuthenticationError,
  CrawlkitError,
  NotFoundError,
  RateLimitError,
} from "./errors";
import { CrawlRequest } from "./models";

const originalFetch = globalThis.fetch;

function jsonResponse(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json" },
  });
}

beforeEach(() => {
  (globalThis.fetch as unknown) = jest.fn();
});

afterAll(() => {
  (globalThis.fetch as unknown) = originalFetch;
});

function fetchMock(): jest.Mock {
  return globalThis.fetch as unknown as jest.Mock;
}

describe("CrawlkitClient", () => {
  describe("auth headers", () => {
    it("sends the X-API-Key header on GET requests", async () => {
      fetchMock().mockResolvedValueOnce(
        jsonResponse(200, {
          crawl_id: "c1",
          start_url: "https://example.com",
          status: "completed",
          pages_crawled: 42,
          issues_found: 7,
          created_at: "2024-01-15T10:30:00Z",
        })
      );

      const client = new CrawlkitClient("http://api.test", "test-key");
      await client.getCrawl("c1");

      expect(fetchMock()).toHaveBeenCalledTimes(1);
      const [url, init] = fetchMock().mock.calls[0];
      expect(url).toBe("http://api.test/api/v1/crawls/c1");
      expect(init.method ?? "GET").toBe("GET");
      expect(init.headers["X-API-Key"]).toBe("test-key");
    });

    it("sends the Authorization header when constructed with a JWT", async () => {
      fetchMock().mockResolvedValueOnce(
        jsonResponse(200, { status: "ok", version: "1.0.0" })
      );

      const client = new CrawlkitClient("http://api.test", "", "jwt-token");
      await client.health();

      const [, init] = fetchMock().mock.calls[0];
      expect(init.headers["Authorization"]).toBe("Bearer jwt-token");
      expect(init.headers["X-API-Key"]).toBeUndefined();
    });
  });

  describe("startCrawl", () => {
    it("POSTs to /api/v1/crawls and returns the parsed response", async () => {
      fetchMock().mockResolvedValueOnce(
        jsonResponse(202, {
          crawl_id: "c1",
          status: "running",
          message: "Crawl started successfully",
        })
      );

      const client = new CrawlkitClient("http://api.test/", "test-key");
      const request: CrawlRequest = {
        start_url: "https://example.com",
        max_pages: 50,
        request_delay_ms: 100,
        concurrency: 4,
      };
      const result = await client.startCrawl(request);

      expect(fetchMock()).toHaveBeenCalledTimes(1);
      const [url, init] = fetchMock().mock.calls[0];
      expect(url).toBe("http://api.test/api/v1/crawls");
      expect(init.method).toBe("POST");
      expect(init.headers["X-API-Key"]).toBe("test-key");
      expect(init.headers["Content-Type"]).toBe("application/json");
      expect(JSON.parse(init.body)).toEqual(request);

      expect(result).toEqual({
        crawl_id: "c1",
        status: "running",
        message: "Crawl started successfully",
      });
    });
  });

  describe("getCrawl", () => {
    it("GETs /api/v1/crawls/:id and returns the parsed result", async () => {
      const payload = {
        crawl_id: "c1",
        start_url: "https://example.com",
        status: "completed",
        pages_crawled: 42,
        issues_found: 7,
        created_at: "2024-01-15T10:30:00Z",
        completed_at: "2024-01-15T11:00:00Z",
      };
      fetchMock().mockResolvedValueOnce(jsonResponse(200, payload));

      const client = new CrawlkitClient("http://api.test", "test-key");
      const result = await client.getCrawl("c1");

      const [url, init] = fetchMock().mock.calls[0];
      expect(url).toBe("http://api.test/api/v1/crawls/c1");
      expect(init.method ?? "GET").toBe("GET");
      expect(result).toEqual(payload);
    });
  });

  describe("getCrawlStats", () => {
    it("GETs /api/v1/crawls/:id/stats and returns the parsed stats", async () => {
      const payload = {
        crawl_id: "c1",
        total_pages: 42,
        total_issues: 7,
        issues_by_severity: { critical: 1, warning: 6 },
        issues_by_category: { meta: 3, links: 4 },
        avg_response_time_ms: 123.5,
      };
      fetchMock().mockResolvedValueOnce(jsonResponse(200, payload));

      const client = new CrawlkitClient("http://api.test", "test-key");
      const result = await client.getCrawlStats("c1");

      const [url] = fetchMock().mock.calls[0];
      expect(url).toBe("http://api.test/api/v1/crawls/c1/stats");
      expect(result).toEqual(payload);
    });
  });

  describe("error mapping", () => {
    it.each([
      [401, AuthenticationError, "invalid api key"],
      [404, NotFoundError, "crawl not found"],
      [429, RateLimitError, "too many requests"],
    ])(
      "maps HTTP %i to %p",
      async (status, errorClass, message) => {
        fetchMock().mockResolvedValueOnce(
          jsonResponse(status, { error: message })
        );

        const client = new CrawlkitClient("http://api.test", "test-key");
        const promise = client.getCrawl("c1");

        await expect(promise).rejects.toBeInstanceOf(errorClass);
        await expect(promise).rejects.toBeInstanceOf(CrawlkitError);
        await expect(promise).rejects.toMatchObject({
          statusCode: status,
          message,
        });
      }
    );

    it("maps other non-2xx statuses to CrawlkitError", async () => {
      fetchMock().mockResolvedValueOnce(
        jsonResponse(500, { error: "internal error" })
      );

      const client = new CrawlkitClient("http://api.test", "test-key");
      const err = await client.getCrawl("c1").catch((e) => e);

      expect(err).toBeInstanceOf(CrawlkitError);
      expect(err).not.toBeInstanceOf(AuthenticationError);
      expect(err.statusCode).toBe(500);
      expect(err.message).toBe("internal error");
    });

    it("falls back to a generic error for 401 on POST", async () => {
      fetchMock().mockResolvedValueOnce(
        jsonResponse(401, { error: "invalid api key" })
      );

      const client = new CrawlkitClient("http://api.test", "bad-key");
      const err = await client
        .startCrawl({ start_url: "https://example.com" })
        .catch((e) => e);

      expect(err).toBeInstanceOf(AuthenticationError);
      expect(err.statusCode).toBe(401);
    });
  });
});
