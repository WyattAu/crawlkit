export declare class CrawlkitError extends Error {
    statusCode: number;
    constructor(message: string, statusCode: number);
}
export declare class AuthenticationError extends CrawlkitError {
    constructor(message?: string);
}
export declare class NotFoundError extends CrawlkitError {
    constructor(message?: string);
}
export declare class RateLimitError extends CrawlkitError {
    constructor(message?: string);
}
export declare class ValidationError extends CrawlkitError {
    constructor(message: string);
}
//# sourceMappingURL=errors.d.ts.map