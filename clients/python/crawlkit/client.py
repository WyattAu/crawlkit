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
    Backlink,
    ApiKey,
    Webhook,
    Schedule,
    AuditEvent,
    MarketplacePlugin,
    Session,
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
        transport: Optional[httpx.BaseTransport] = None,
    ):
        """Initialize the client.

        Args:
            base_url: Base URL of the crawlkit API.
            api_key: API key for authentication.
            jwt_token: JWT token for authentication.
            timeout: Request timeout in seconds.
            transport: Optional httpx transport override (e.g. for testing).
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
            transport=transport,
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

    # Crawl Findings & Backlinks

    def get_crawl_findings(self, crawl_id: str) -> List[Finding]:
        """Get findings for a crawl."""
        result = self._request("GET", f"/api/v1/crawls/{crawl_id}/findings")
        return [Finding(**f) for f in result]

    def get_crawl_backlinks(self, crawl_id: str) -> List[Backlink]:
        """Get backlinks for a crawl."""
        result = self._request("GET", f"/api/v1/crawls/{crawl_id}/backlinks")
        return [Backlink(**b) for b in result]

    # Auth

    def refresh_token(self) -> str:
        """Refresh the current JWT token."""
        result = self._request("POST", "/api/v1/auth/refresh")
        return result["token"]

    # API Keys

    def create_api_key(self, name: str) -> ApiKey:
        """Create a new API key."""
        result = self._request("POST", "/api/v1/keys", json={"name": name})
        return ApiKey(**result)

    def list_api_keys(self) -> List[ApiKey]:
        """List all API keys."""
        result = self._request("GET", "/api/v1/keys")
        return [ApiKey(**k) for k in result]

    def delete_api_key(self, key: str) -> None:
        """Delete an API key."""
        self._request("DELETE", f"/api/v1/keys/{key}")

    # Webhooks

    def create_webhook(self, url: str, events: List[str]) -> Webhook:
        """Create a new webhook subscription."""
        result = self._request(
            "POST", "/api/v1/webhooks", json={"url": url, "events": events}
        )
        return Webhook(**result)

    def list_webhooks(self) -> List[Webhook]:
        """List all webhook subscriptions."""
        result = self._request("GET", "/api/v1/webhooks")
        return [Webhook(**w) for w in result]

    def delete_webhook(self, webhook_id: str) -> None:
        """Delete a webhook subscription."""
        self._request("DELETE", f"/api/v1/webhooks/{webhook_id}")

    # Schedules

    def create_schedule(self, crawl_id: str, cron_expression: str) -> Schedule:
        """Create a new scheduled crawl."""
        result = self._request(
            "POST",
            "/api/v1/schedules",
            json={"crawl_id": crawl_id, "cron_expression": cron_expression},
        )
        return Schedule(**result)

    def list_schedules(self) -> List[Schedule]:
        """List all schedules."""
        result = self._request("GET", "/api/v1/schedules")
        return [Schedule(**s) for s in result]

    def delete_schedule(self, schedule_id: str) -> None:
        """Delete a schedule."""
        self._request("DELETE", f"/api/v1/schedules/{schedule_id}")

    def update_schedule(
        self,
        schedule_id: str,
        cron_expression: Optional[str] = None,
        enabled: Optional[bool] = None,
    ) -> Schedule:
        """Update a schedule."""
        data: Dict[str, Any] = {}
        if cron_expression is not None:
            data["cron_expression"] = cron_expression
        if enabled is not None:
            data["enabled"] = enabled
        result = self._request("PATCH", f"/api/v1/schedules/{schedule_id}", json=data)
        return Schedule(**result)

    # Audit

    def list_audit_events(self) -> List[AuditEvent]:
        """List audit log events."""
        result = self._request("GET", "/api/v1/audit")
        return [AuditEvent(**e) for e in result]

    # Marketplace

    def submit_plugin(
        self, name: str, description: str, version: str
    ) -> MarketplacePlugin:
        """Submit a plugin to the marketplace."""
        result = self._request(
            "POST",
            "/api/v1/marketplace/plugins",
            json={"name": name, "description": description, "version": version},
        )
        return MarketplacePlugin(**result)

    def list_marketplace_plugins(self) -> List[MarketplacePlugin]:
        """List marketplace plugins."""
        result = self._request("GET", "/api/v1/marketplace/plugins")
        return [MarketplacePlugin(**p) for p in result]

    def get_marketplace_plugin(self, name: str) -> MarketplacePlugin:
        """Get a marketplace plugin by name."""
        result = self._request("GET", f"/api/v1/marketplace/plugins/{name}")
        return MarketplacePlugin(**result)

    def delete_marketplace_plugin(self, name: str) -> None:
        """Delete a marketplace plugin."""
        self._request("DELETE", f"/api/v1/marketplace/plugins/{name}")

    # Sessions

    def list_sessions(self) -> List[Session]:
        """List all active sessions."""
        result = self._request("GET", "/api/v1/sessions")
        return [Session(**s) for s in result]

    def revoke_session(self, session_id: str) -> None:
        """Revoke a session."""
        self._request("POST", f"/api/v1/sessions/revoke", json={"session_id": session_id})

    # Metrics

    def get_metrics(self) -> str:
        """Get Prometheus metrics."""
        response = self.client.get("/metrics")
        response.raise_for_status()
        return response.text
