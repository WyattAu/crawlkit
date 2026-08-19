"""Tests for the crawlkit Python client using httpx.MockTransport."""

import json
import unittest

import httpx

from crawlkit import CrawlkitClient, CrawlRequest, CrawlStatus
from crawlkit.client import CrawlkitClient as CrawlkitClientDirect
from crawlkit.exceptions import (
    AuthenticationError,
    CrawlkitError,
    NotFoundError,
    RateLimitError,
)


class RequestRecorder:
    """Captures the outgoing httpx request for assertions."""

    def __init__(self):
        self.requests = []

    def __call__(self, request):
        self.requests.append(request)


def json_response(status_code, payload):
    return httpx.Response(
        status_code,
        json=payload,
        headers={"Content-Type": "application/json"},
    )


def make_client(handler, **kwargs):
    transport = httpx.MockTransport(handler)
    return CrawlkitClient(
        base_url="http://api.test", api_key="test-key", transport=transport, **kwargs
    )


class TestAuthHeaders(unittest.TestCase):
    def test_api_key_header_on_every_request(self):
        recorder = RequestRecorder()

        def handler(request):
            recorder(request)
            return json_response(200, {"status": "ok", "version": "1.0.0"})

        with make_client(handler) as client:
            client.health()

        self.assertEqual(len(recorder.requests), 1)
        request = recorder.requests[0]
        self.assertEqual(request.headers["X-API-Key"], "test-key")
        self.assertNotIn("Authorization", request.headers)
        self.assertEqual(request.method, "GET")
        self.assertEqual(request.url.path, "/health")

    def test_jwt_header(self):
        recorder = RequestRecorder()

        def handler(request):
            recorder(request)
            return json_response(200, {"crawl_id": "c1", "status": "completed"})

        client = CrawlkitClient(
            base_url="http://api.test",
            jwt_token="jwt-token",
            transport=httpx.MockTransport(handler),
        )
        client.get_crawl("c1")

        request = recorder.requests[0]
        self.assertEqual(request.headers["Authorization"], "Bearer jwt-token")
        self.assertNotIn("X-API-Key", request.headers)


class TestStartCrawl(unittest.TestCase):
    def test_start_crawl_happy_path(self):
        recorder = RequestRecorder()

        def handler(request):
            recorder(request)
            return json_response(
                202,
                {
                    "crawl_id": "c1",
                    "status": "running",
                    "message": "Crawl started successfully",
                },
            )

        with make_client(handler) as client:
            result = client.start_crawl(
                CrawlRequest(
                    start_url="https://example.com",
                    max_pages=50,
                    request_delay_ms=100,
                    concurrency=4,
                )
            )

        request = recorder.requests[0]
        self.assertEqual(request.method, "POST")
        self.assertEqual(request.url.path, "/api/v1/crawls")
        self.assertEqual(request.headers["X-API-Key"], "test-key")
        self.assertEqual(request.headers["Content-Type"], "application/json")

        body = json.loads(request.content.decode("utf-8"))
        self.assertEqual(
            body,
            {
                "start_url": "https://example.com",
                "max_pages": 50,
                "request_delay_ms": 100,
                "concurrency": 4,
            },
        )

        self.assertEqual(result.crawl_id, "c1")
        self.assertEqual(result.status, CrawlStatus.RUNNING)
        self.assertEqual(result.message, "Crawl started successfully")

    def test_start_crawl_includes_tenant_id_when_set(self):
        recorder = RequestRecorder()

        def handler(request):
            recorder(request)
            return json_response(
                202, {"crawl_id": "c1", "status": "running", "message": "ok"}
            )

        with make_client(handler) as client:
            client.start_crawl(
                CrawlRequest(start_url="https://example.com", tenant_id="tenant-1")
            )

        body = json.loads(recorder.requests[0].content.decode("utf-8"))
        self.assertEqual(body["tenant_id"], "tenant-1")


class TestGetCrawl(unittest.TestCase):
    def test_get_crawl_happy_path(self):
        recorder = RequestRecorder()
        payload = {
            "crawl_id": "c1",
            "start_url": "https://example.com",
            "status": "completed",
            "pages_crawled": 42,
            "issues_found": 7,
            "created_at": "2024-01-15T10:30:00Z",
            "completed_at": "2024-01-15T11:00:00Z",
        }

        def handler(request):
            recorder(request)
            return json_response(200, payload)

        with make_client(handler) as client:
            result = client.get_crawl("c1")

        request = recorder.requests[0]
        self.assertEqual(request.method, "GET")
        self.assertEqual(request.url.path, "/api/v1/crawls/c1")
        self.assertEqual(request.headers["X-API-Key"], "test-key")
        self.assertEqual(result, payload)


class TestGetCrawlStats(unittest.TestCase):
    def test_get_crawl_stats_happy_path(self):
        recorder = RequestRecorder()

        def handler(request):
            recorder(request)
            return json_response(
                200,
                {
                    "total_pages": 42,
                    "total_issues": 7,
                    "avg_response_time_ms": 123.5,
                    "total_body_size": 4096,
                },
            )

        with make_client(handler) as client:
            stats = client.get_crawl_stats("c1")

        request = recorder.requests[0]
        self.assertEqual(request.method, "GET")
        self.assertEqual(request.url.path, "/api/v1/crawls/c1/stats")

        self.assertEqual(stats.total_pages, 42)
        self.assertEqual(stats.total_issues, 7)
        self.assertEqual(stats.avg_response_time_ms, 123.5)
        self.assertEqual(stats.total_body_size, 4096)


class TestErrorMapping(unittest.TestCase):
    def assert_raises_with_type(self, exc_type, status_code):
        def handler(request):
            return json_response(status_code, {"error": "boom"})

        with make_client(handler) as client:
            with self.assertRaises(exc_type):
                client.get_crawl("c1")

    def test_401_raises_authentication_error(self):
        self.assert_raises_with_type(AuthenticationError, 401)

    def test_404_raises_not_found_error(self):
        self.assert_raises_with_type(NotFoundError, 404)

    def test_429_raises_rate_limit_error(self):
        self.assert_raises_with_type(RateLimitError, 429)

    def test_500_raises_crawlkit_error(self):
        self.assert_raises_with_type(CrawlkitError, 500)

    def test_all_exceptions_subclass_crawlkit_error(self):
        for exc_type in (AuthenticationError, NotFoundError, RateLimitError):
            self.assertTrue(issubclass(exc_type, CrawlkitError))


class TestContextManager(unittest.TestCase):
    def test_context_manager_closes_client(self):
        def handler(request):
            return json_response(200, {"status": "ok", "version": "1.0.0"})

        with make_client(handler) as client:
            self.assertIsInstance(client, CrawlkitClientDirect)
            self.assertFalse(client.client.is_closed)
            result = client.health()
            self.assertEqual(result, {"status": "ok", "version": "1.0.0"})
        self.assertTrue(client.client.is_closed)


if __name__ == "__main__":
    unittest.main()
