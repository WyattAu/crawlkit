"""Data models for the crawlkit API."""

from dataclasses import dataclass, field
from typing import Optional, List
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
