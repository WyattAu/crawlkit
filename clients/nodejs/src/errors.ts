export class CrawlkitError extends Error {
  public statusCode: number;

  constructor(message: string, statusCode: number) {
    super(message);
    this.name = "CrawlkitError";
    this.statusCode = statusCode;
  }
}

export class AuthenticationError extends CrawlkitError {
  constructor(message = "Authentication failed") {
    super(message, 401);
    this.name = "AuthenticationError";
  }
}

export class NotFoundError extends CrawlkitError {
  constructor(message = "Resource not found") {
    super(message, 404);
    this.name = "NotFoundError";
  }
}

export class RateLimitError extends CrawlkitError {
  constructor(message = "Rate limit exceeded") {
    super(message, 429);
    this.name = "RateLimitError";
  }
}

export class ValidationError extends CrawlkitError {
  constructor(message: string) {
    super(message, 400);
    this.name = "ValidationError";
  }
}
