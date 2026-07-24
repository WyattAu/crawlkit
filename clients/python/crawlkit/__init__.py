"""crawlkit Python client for the REST API."""

from .client import CrawlkitClient
from .models import (
    CrawlRequest,
    CrawlResponse,
    CrawlStatus,
    User,
    Tenant,
    Finding,
)
from .exceptions import CrawlkitError, AuthenticationError, NotFoundError

__version__ = "1.0.0"
__all__ = [
    "CrawlkitClient",
    "CrawlRequest",
    "CrawlResponse",
    "CrawlStatus",
    "User",
    "Tenant",
    "Finding",
    "CrawlkitError",
    "AuthenticationError",
    "NotFoundError",
]
