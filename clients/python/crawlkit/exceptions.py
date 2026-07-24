"""Exceptions for the crawlkit API client."""


class CrawlkitError(Exception):
    """Base exception for crawlkit errors."""
    pass


class AuthenticationError(CrawlkitError):
    """Authentication failed."""
    pass


class NotFoundError(CrawlkitError):
    """Resource not found."""
    pass


class RateLimitError(CrawlkitError):
    """Rate limit exceeded."""
    pass


class ValidationError(CrawlkitError):
    """Validation error."""
    pass
