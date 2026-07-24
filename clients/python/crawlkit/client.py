"""crawlkit API client."""

from typing import Optional, List, Dict, Any
import httpx

from .models import (
    CrawlRequest,
    CrawlResponse,
    CrawlStatus,
    CrawlStats,
    Finding,
    User,
    Tenant,
)
from .exceptions import (
    CrawlkitError,
    AuthenticationError,
    NotFoundError,
    RateLimitError,
)


class CrawlkitClient:
    """Client for the crawlkit REST API."""

    def __init__(
        self,
        base_url: str = "http://localhost:4000",
        api_key: Optional[str] = None,
        jwt_token: Optional[str] = None,
        timeout: float = 30.0,
    ):
        """Initialize the client.

        Args:
            base_url: Base URL of the crawlkit API.
            api_key: API key for authentication.
            jwt_token: JWT token for authentication.
            timeout: Request timeout in seconds.
        """
        self.base_url = base_url.rstrip("/")
        self.timeout = timeout

        headers = {}
        if api_key:
            headers["X-API-Key"] = api_key
        elif jwt_token:
            headers["Authorization"] = f"Bearer {jwt_token}"

        self.client = httpx.Client(
            base_url=self.base_url,
            headers=headers,
            timeout=timeout,
        )

    def close(self):
        """Close the client."""
        self.client.close()

    def __enter__(self):
        return self

    def __exit__(self, *args):
        self.close()

    def _request(
        self,
        method: str,
        path: str,
        **kwargs,
    ) -> Dict[str, Any]:
        """Make a request to the API."""
        try:
            response = self.client.request(method, path, **kwargs)
            response.raise_for_status()
            return response.json()
        except httpx.HTTPStatusError as e:
            if e.response.status_code == 401:
                raise AuthenticationError("Authentication failed")
            elif e.response.status_code == 404:
                raise NotFoundError(f"Resource not found: {path}")
            elif e.response.status_code == 429:
                raise RateLimitError("Rate limit exceeded")
            else:
                raise CrawlkitError(f"HTTP {e.response.status_code}: {e.response.text}")

    # Health

    def health(self) -> Dict[str, Any]:
        """Check API health."""
        return self._request("GET", "/health")

    # Crawls

    def start_crawl(self, request: CrawlRequest) -> CrawlResponse:
        """Start a new crawl."""
        data = {
            "start_url": request.start_url,
            "max_pages": request.max_pages,
            "request_delay_ms": request.request_delay_ms,
            "concurrency": request.concurrency,
        }
        if request.tenant_id:
            data["tenant_id"] = request.tenant_id

        result = self._request("POST", "/api/v1/crawls", json=data)
        return CrawlResponse(
            crawl_id=result["crawl_id"],
            status=CrawlStatus(result["status"]),
            message=result["message"],
        )

    def get_crawl(self, crawl_id: str) -> Dict[str, Any]:
        """Get crawl status."""
        return self._request("GET", f"/api/v1/crawls/{crawl_id}")

    def get_crawl_stats(self, crawl_id: str) -> CrawlStats:
        """Get crawl statistics."""
        result = self._request("GET", f"/api/v1/crawls/{crawl_id}/stats")
        return CrawlStats(
            total_pages=result.get("total_pages", 0),
            total_issues=result.get("total_issues", 0),
            avg_response_time_ms=result.get("avg_response_time_ms", 0.0),
            total_body_size=result.get("total_body_size", 0),
        )

    def list_crawls(self) -> List[Dict[str, Any]]:
        """List all crawls."""
        return self._request("GET", "/api/v1/crawls")

    # Users

    def list_users(self) -> List[User]:
        """List all users."""
        result = self._request("GET", "/api/v1/users")
        return [User(**u) for u in result]

    def create_user(
        self,
        email: str,
        name: str,
        password: str,
        tenant_id: str = "default",
        roles: Optional[List[str]] = None,
    ) -> User:
        """Create a new user."""
        data = {
            "email": email,
            "name": name,
            "password": password,
            "tenant_id": tenant_id,
            "roles": roles or ["viewer"],
        }
        result = self._request("POST", "/api/v1/users", json=data)
        return User(**result)

    def delete_user(self, user_id: str) -> None:
        """Delete a user."""
        self._request("DELETE", f"/api/v1/users/{user_id}")

    # Tenants

    def list_tenants(self) -> List[Tenant]:
        """List all tenants."""
        result = self._request("GET", "/api/v1/tenants")
        return [Tenant(**t) for t in result]

    def create_tenant(self, name: str, plan: str = "free") -> Tenant:
        """Create a new tenant."""
        data = {"name": name, "plan": plan}
        result = self._request("POST", "/api/v1/tenants", json=data)
        return Tenant(**result)

    def get_tenant(self, tenant_id: str) -> Tenant:
        """Get tenant details."""
        result = self._request("GET", f"/api/v1/tenants/{tenant_id}")
        return Tenant(**result)

    def delete_tenant(self, tenant_id: str) -> None:
        """Delete a tenant."""
        self._request("DELETE", f"/api/v1/tenants/{tenant_id}")

    # Auth

    def login(self, email: str, password: str) -> str:
        """Login and get JWT token."""
        data = {"email": email, "password": password}
        result = self._request("POST", "/api/v1/auth/login", json=data)
        return result["token"]

    def get_current_user(self) -> User:
        """Get current user info."""
        result = self._request("GET", "/api/v1/auth/me")
        return User(**result)

    # Metrics

    def get_metrics(self) -> str:
        """Get Prometheus metrics."""
        response = self.client.get("/metrics")
        response.raise_for_status()
        return response.text
