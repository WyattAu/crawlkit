"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.ValidationError = exports.RateLimitError = exports.NotFoundError = exports.AuthenticationError = exports.CrawlkitError = void 0;
class CrawlkitError extends Error {
    constructor(message, statusCode) {
        super(message);
        this.name = "CrawlkitError";
        this.statusCode = statusCode;
    }
}
exports.CrawlkitError = CrawlkitError;
class AuthenticationError extends CrawlkitError {
    constructor(message = "Authentication failed") {
        super(message, 401);
        this.name = "AuthenticationError";
    }
}
exports.AuthenticationError = AuthenticationError;
class NotFoundError extends CrawlkitError {
    constructor(message = "Resource not found") {
        super(message, 404);
        this.name = "NotFoundError";
    }
}
exports.NotFoundError = NotFoundError;
class RateLimitError extends CrawlkitError {
    constructor(message = "Rate limit exceeded") {
        super(message, 429);
        this.name = "RateLimitError";
    }
}
exports.RateLimitError = RateLimitError;
class ValidationError extends CrawlkitError {
    constructor(message) {
        super(message, 400);
        this.name = "ValidationError";
    }
}
exports.ValidationError = ValidationError;
//# sourceMappingURL=errors.js.map