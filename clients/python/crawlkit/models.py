"""Data models for the crawlkit API."""

from dataclasses import dataclass, field
from typing import Optional, List, Dict, Any
from enum import Enum


class Severity(str, Enum):
    """Issue severity."""
    CRITICAL = "critical"
    ERROR = "error"
    WARNING = "warning"
    INFO = "info"


class CrawlStatus(str, Enum):
    """Crawl status."""
    PENDING = "pending"
    RUNNING = "running"
    COMPLETED = "completed"
    FAILED = "failed"


@dataclass
class CrawlRequest:
    """Request to start a crawl."""
    start_url: str
    max_pages: int = 100
    request_delay_ms: int = 500
    concurrency: int = 4
    tenant_id: Optional[str] = None


@dataclass
class CrawlResponse:
    """Response from starting a crawl."""
    crawl_id: str
    status: CrawlStatus
    message: str


@dataclass
class CrawlStats:
    """Crawl statistics."""
    total_pages: int = 0
    total_issues: int = 0
    avg_response_time_ms: float = 0.0
    total_body_size: int = 0


@dataclass
class Finding:
    """An SEO finding."""
    severity: Severity
    category: str
    code: str
    title: str
    description: str
    recommendation: str


@dataclass
class User:
    """User in the system."""
    id: str
    email: str
    name: str
    tenant_id: str
    roles: List[str] = field(default_factory=list)
    enabled: bool = True


@dataclass
class Tenant:
    """Tenant in the system."""
    id: str
    name: str
    plan: str = "free"
    max_users: int = 10
    max_crawls_per_month: int = 100


@dataclass
class Backlink:
    """A backlink entry."""
    source_url: str
    target_url: str
    anchor_text: str = ""
    rel: str = ""


@dataclass
class ApiKey:
    """An API key."""
    key: str
    name: str
    created_at: str = ""


@dataclass
class Webhook:
    """A webhook subscription."""
    id: str
    url: str
    events: List[str] = field(default_factory=list)
    secret: str = ""
    enabled: bool = True


@dataclass
class Schedule:
    """A scheduled crawl."""
    id: str
    crawl_id: str
    cron_expression: str
    enabled: bool = True
    last_run: Optional[str] = None
    next_run: Optional[str] = None


@dataclass
class AuditEvent:
    """An audit log event."""
    id: str
    action: str
    user_id: str
    resource_type: str
    resource_id: str = ""
    details: Optional[Dict[str, Any]] = None
    timestamp: str = ""


@dataclass
class MarketplacePlugin:
    """A marketplace plugin (full API projection)."""
    name: str
    description: str = ""
    version: str = ""
    author: str = ""
    license: str = ""
    categories: List[str] = field(default_factory=list)
    tags: List[str] = field(default_factory=list)
    downloads: int = 0
    rating: float = 0.0
    rating_count: int = 0
    verified: bool = False
    created_at: str = ""
    updated_at: str = ""


@dataclass
class PluginTestResult:
    """Result of running a plugin's self-test (plugin-defined payload)."""
    status: str = ""
    findings: int = 0
    execution_time_ms: int = 0
    raw: Optional[Dict[str, Any]] = None


@dataclass
class PluginRatingResult:
    """Response from submitting a plugin rating."""
    name: str
    rating: float
    rating_count: int
    average_rating: float


@dataclass
class PluginDownloadResult:
    """Response from recording a plugin download."""
    name: str
    downloads: int


@dataclass
class Session:
    """An active session."""
    id: str
    user_id: str
    created_at: str = ""
    expires_at: str = ""
    ip_address: str = ""
