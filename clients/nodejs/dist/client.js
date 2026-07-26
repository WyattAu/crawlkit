"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.CrawlkitClient = void 0;
const errors_1 = require("./errors");
class CrawlkitClient {
    constructor(baseURL, apiKey = "", jwtToken = "") {
        this.baseURL = baseURL.replace(/\/$/, "");
        this.apiKey = apiKey;
        this.jwtToken = jwtToken;
    }
    async health() {
        return this.get("/health");
    }
    async getMetrics() {
        const headers = {};
        if (this.apiKey)
            headers["X-API-Key"] = this.apiKey;
        if (this.jwtToken)
            headers["Authorization"] = `Bearer ${this.jwtToken}`;
        const response = await fetch(`${this.baseURL}/metrics`, { headers });
        if (!response.ok) {
            throw await this.parseError(response);
        }
        return response.text();
    }
    async login(email, password) {
        return this.post("/api/v1/auth/login", { email, password });
    }
    async refreshToken() {
        return this.post("/api/v1/auth/refresh", {});
    }
    async getCurrentUser() {
        return this.get("/api/v1/auth/me");
    }
    async startCrawl(req) {
        return this.post("/api/v1/crawls", req);
    }
    async getCrawl(crawlId) {
        return this.get(`/api/v1/crawls/${crawlId}`);
    }
    async getCrawlStats(crawlId) {
        return this.get(`/api/v1/crawls/${crawlId}/stats`);
    }
    async getCrawlFindings(crawlId) {
        return this.get(`/api/v1/crawls/${crawlId}/findings`);
    }
    async getCrawlBacklinks(crawlId) {
        return this.get(`/api/v1/crawls/${crawlId}/backlinks`);
    }
    async listCrawls() {
        return this.get("/api/v1/crawls");
    }
    async listUsers() {
        return this.get("/api/v1/users");
    }
    async createUser(req) {
        return this.post("/api/v1/users", req);
    }
    async deleteUser(userId) {
        await this.delete(`/api/v1/users/${userId}`);
    }
    async listTenants() {
        return this.get("/api/v1/tenants");
    }
    async createTenant(req) {
        return this.post("/api/v1/tenants", req);
    }
    async getTenant(tenantId) {
        return this.get(`/api/v1/tenants/${tenantId}`);
    }
    async deleteTenant(tenantId) {
        await this.delete(`/api/v1/tenants/${tenantId}`);
    }
    async listApiKeys() {
        return this.get("/api/v1/keys");
    }
    async createApiKey(req) {
        return this.post("/api/v1/keys", req);
    }
    async deleteApiKey(key) {
        await this.delete(`/api/v1/keys/${key}`);
    }
    async listWebhooks() {
        return this.get("/api/v1/webhooks");
    }
    async createWebhook(req) {
        return this.post("/api/v1/webhooks", req);
    }
    async deleteWebhook(webhookId) {
        await this.delete(`/api/v1/webhooks/${webhookId}`);
    }
    async listSchedules() {
        return this.get("/api/v1/schedules");
    }
    async createSchedule(req) {
        return this.post("/api/v1/schedules", req);
    }
    async deleteSchedule(scheduleId) {
        await this.delete(`/api/v1/schedules/${scheduleId}`);
    }
    async listAuditEvents() {
        return this.get("/api/v1/audit");
    }
    async listMarketplacePlugins() {
        return this.get("/api/v1/marketplace/plugins");
    }
    async getMarketplacePlugin(name) {
        return this.get(`/api/v1/marketplace/plugins/${name}`);
    }
    async submitPlugin(req) {
        return this.post("/api/v1/marketplace/plugins", req);
    }
    async deleteMarketplacePlugin(name) {
        await this.delete(`/api/v1/marketplace/plugins/${name}`);
    }
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    async get(path) {
        const headers = {};
        if (this.apiKey)
            headers["X-API-Key"] = this.apiKey;
        if (this.jwtToken)
            headers["Authorization"] = `Bearer ${this.jwtToken}`;
        const response = await fetch(`${this.baseURL}${path}`, { headers });
        if (!response.ok) {
            throw await this.parseError(response);
        }
        return (await response.json());
    }
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    async post(path, body) {
        const headers = {
            "Content-Type": "application/json",
        };
        if (this.apiKey)
            headers["X-API-Key"] = this.apiKey;
        if (this.jwtToken)
            headers["Authorization"] = `Bearer ${this.jwtToken}`;
        const response = await fetch(`${this.baseURL}${path}`, {
            method: "POST",
            headers,
            body: JSON.stringify(body),
        });
        if (!response.ok) {
            throw await this.parseError(response);
        }
        return (await response.json());
    }
    async delete(path) {
        const headers = {};
        if (this.apiKey)
            headers["X-API-Key"] = this.apiKey;
        if (this.jwtToken)
            headers["Authorization"] = `Bearer ${this.jwtToken}`;
        const response = await fetch(`${this.baseURL}${path}`, {
            method: "DELETE",
            headers,
        });
        if (!response.ok) {
            throw await this.parseError(response);
        }
    }
    async parseError(response) {
        let message;
        try {
            // eslint-disable-next-line @typescript-eslint/no-explicit-any
            const body = await response.json();
            message = body?.error || response.statusText;
        }
        catch {
            message = response.statusText;
        }
        switch (response.status) {
            case 401:
                return new errors_1.AuthenticationError(message);
            case 404:
                return new errors_1.NotFoundError(message);
            case 429:
                return new errors_1.RateLimitError(message);
            default:
                return new errors_1.CrawlkitError(message, response.status);
        }
    }
}
exports.CrawlkitClient = CrawlkitClient;
//# sourceMappingURL=client.js.map