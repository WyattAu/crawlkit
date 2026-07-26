import {
  CrawlRequest,
  CrawlResponse,
  CrawlResult,
  CrawlStats,
  Finding,
  BacklinksResponse,
  User,
  CreateUserRequest,
  Tenant,
  CreateTenantRequest,
  ApiKeyInfo,
  CreateApiKeyRequest,
  WebhookConfig,
  CreateWebhookRequest,
  ScheduleResponse,
  CreateScheduleRequest,
  AuditEvent,
  HealthResponse,
  LoginResponse,
  MarketplacePlugin,
  SubmitPluginRequest,
} from "./models";
import {
  CrawlkitError,
  AuthenticationError,
  NotFoundError,
  RateLimitError,
} from "./errors";

export class CrawlkitClient {
  private baseURL: string;
  private apiKey: string;
  private jwtToken: string;

  constructor(baseURL: string, apiKey: string = "", jwtToken: string = "") {
    this.baseURL = baseURL.replace(/\/$/, "");
    this.apiKey = apiKey;
    this.jwtToken = jwtToken;
  }

  async health(): Promise<HealthResponse> {
    return this.get("/health");
  }

  async getMetrics(): Promise<string> {
    const headers: Record<string, string> = {};
    if (this.apiKey) headers["X-API-Key"] = this.apiKey;
    if (this.jwtToken) headers["Authorization"] = `Bearer ${this.jwtToken}`;

    const response = await fetch(`${this.baseURL}/metrics`, { headers });
    if (!response.ok) {
      throw await this.parseError(response);
    }
    return response.text();
  }

  async login(email: string, password: string): Promise<LoginResponse> {
    return this.post("/api/v1/auth/login", { email, password });
  }

  async refreshToken(): Promise<LoginResponse> {
    return this.post("/api/v1/auth/refresh", {});
  }

  async getCurrentUser(): Promise<User> {
    return this.get("/api/v1/auth/me");
  }

  async startCrawl(req: CrawlRequest): Promise<CrawlResponse> {
    return this.post("/api/v1/crawls", req);
  }

  async getCrawl(crawlId: string): Promise<CrawlResult> {
    return this.get(`/api/v1/crawls/${crawlId}`);
  }

  async getCrawlStats(crawlId: string): Promise<CrawlStats> {
    return this.get(`/api/v1/crawls/${crawlId}/stats`);
  }

  async getCrawlFindings(crawlId: string): Promise<Finding[]> {
    return this.get(`/api/v1/crawls/${crawlId}/findings`);
  }

  async getCrawlBacklinks(crawlId: string): Promise<BacklinksResponse> {
    return this.get(`/api/v1/crawls/${crawlId}/backlinks`);
  }

  async listCrawls(): Promise<CrawlResult[]> {
    return this.get("/api/v1/crawls");
  }

  async listUsers(): Promise<User[]> {
    return this.get("/api/v1/users");
  }

  async createUser(req: CreateUserRequest): Promise<User> {
    return this.post("/api/v1/users", req);
  }

  async deleteUser(userId: string): Promise<void> {
    await this.delete(`/api/v1/users/${userId}`);
  }

  async listTenants(): Promise<Tenant[]> {
    return this.get("/api/v1/tenants");
  }

  async createTenant(req: CreateTenantRequest): Promise<Tenant> {
    return this.post("/api/v1/tenants", req);
  }

  async getTenant(tenantId: string): Promise<Tenant> {
    return this.get(`/api/v1/tenants/${tenantId}`);
  }

  async deleteTenant(tenantId: string): Promise<void> {
    await this.delete(`/api/v1/tenants/${tenantId}`);
  }

  async listApiKeys(): Promise<ApiKeyInfo[]> {
    return this.get("/api/v1/keys");
  }

  async createApiKey(req: CreateApiKeyRequest): Promise<ApiKeyInfo> {
    return this.post("/api/v1/keys", req);
  }

  async deleteApiKey(key: string): Promise<void> {
    await this.delete(`/api/v1/keys/${key}`);
  }

  async listWebhooks(): Promise<WebhookConfig[]> {
    return this.get("/api/v1/webhooks");
  }

  async createWebhook(req: CreateWebhookRequest): Promise<WebhookConfig> {
    return this.post("/api/v1/webhooks", req);
  }

  async deleteWebhook(webhookId: string): Promise<void> {
    await this.delete(`/api/v1/webhooks/${webhookId}`);
  }

  async listSchedules(): Promise<ScheduleResponse[]> {
    return this.get("/api/v1/schedules");
  }

  async createSchedule(req: CreateScheduleRequest): Promise<ScheduleResponse> {
    return this.post("/api/v1/schedules", req);
  }

  async deleteSchedule(scheduleId: string): Promise<void> {
    await this.delete(`/api/v1/schedules/${scheduleId}`);
  }

  async listAuditEvents(): Promise<AuditEvent[]> {
    return this.get("/api/v1/audit");
  }

  async listMarketplacePlugins(): Promise<MarketplacePlugin[]> {
    return this.get("/api/v1/marketplace/plugins");
  }

  async getMarketplacePlugin(name: string): Promise<MarketplacePlugin> {
    return this.get(`/api/v1/marketplace/plugins/${name}`);
  }

  async submitPlugin(req: SubmitPluginRequest): Promise<MarketplacePlugin> {
    return this.post("/api/v1/marketplace/plugins", req);
  }

  async deleteMarketplacePlugin(name: string): Promise<void> {
    await this.delete(`/api/v1/marketplace/plugins/${name}`);
  }

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  private async get<T>(path: string): Promise<T> {
    const headers: Record<string, string> = {};
    if (this.apiKey) headers["X-API-Key"] = this.apiKey;
    if (this.jwtToken) headers["Authorization"] = `Bearer ${this.jwtToken}`;

    const response = await fetch(`${this.baseURL}${path}`, { headers });
    if (!response.ok) {
      throw await this.parseError(response);
    }
    return (await response.json()) as T;
  }

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  private async post<T>(path: string, body: unknown): Promise<T> {
    const headers: Record<string, string> = {
      "Content-Type": "application/json",
    };
    if (this.apiKey) headers["X-API-Key"] = this.apiKey;
    if (this.jwtToken) headers["Authorization"] = `Bearer ${this.jwtToken}`;

    const response = await fetch(`${this.baseURL}${path}`, {
      method: "POST",
      headers,
      body: JSON.stringify(body as Record<string, unknown>),
    });
    if (!response.ok) {
      throw await this.parseError(response);
    }
    return (await response.json()) as T;
  }

  private async delete(path: string): Promise<void> {
    const headers: Record<string, string> = {};
    if (this.apiKey) headers["X-API-Key"] = this.apiKey;
    if (this.jwtToken) headers["Authorization"] = `Bearer ${this.jwtToken}`;

    const response = await fetch(`${this.baseURL}${path}`, {
      method: "DELETE",
      headers,
    });
    if (!response.ok) {
      throw await this.parseError(response);
    }
  }

  private async parseError(response: Response): Promise<CrawlkitError> {
    let message: string;
    try {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const body: any = await response.json();
      message = body?.error || response.statusText;
    } catch {
      message = response.statusText;
    }

    switch (response.status) {
      case 401:
        return new AuthenticationError(message);
      case 404:
        return new NotFoundError(message);
      case 429:
        return new RateLimitError(message);
      default:
        return new CrawlkitError(message, response.status);
    }
  }
}
