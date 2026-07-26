import type {
  CrawlResult,
  CrawlResponse,
  CrawlStats,
  UserResponse,
  CreateUserRequest,
  Tenant,
  CreateTenantRequest,
  HealthResponse,
  LoginResponse,
  LoginRequest,
  Finding,
  CreateCrawlRequest,
  ApiKeyResponse,
  ApiKeyCreateRequest,
  WebhookConfig,
  CreateWebhookRequest,
  ScheduleResponse,
  CreateScheduleRequest,
  BacklinksResponse,
  AuditEvent,
} from '../models/types';

const BASE_URL = import.meta.env.VITE_API_URL || 'http://localhost:4000';

class ApiClient {
  private token: string | null = null;

  setToken(token: string): void {
    this.token = token;
  }

  private async request<T>(
    method: string,
    path: string,
    body?: unknown
  ): Promise<T> {
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
    };
    if (this.token) {
      headers['Authorization'] = `Bearer ${this.token}`;
    }

    const response = await fetch(`${BASE_URL}${path}`, {
      method,
      headers,
      body: body ? JSON.stringify(body) : undefined,
    });

    if (!response.ok) {
      let message = `HTTP ${response.status}`;
      try {
        const errorBody: { message?: string } = await response.json();
        if (errorBody.message) message = errorBody.message;
      } catch {
        // response body wasn't JSON
      }
      throw new Error(message);
    }

    return response.json();
  }

  // Health

  async health(): Promise<HealthResponse> {
    return this.request<HealthResponse>('GET', '/health');
  }

  // Crawls

  async startCrawl(body: CreateCrawlRequest): Promise<CrawlResponse> {
    return this.request<CrawlResponse>('POST', '/api/v1/crawls', body);
  }

  async getCrawl(crawlId: string): Promise<CrawlResult> {
    return this.request<CrawlResult>('GET', `/api/v1/crawls/${crawlId}`);
  }

  async getCrawlStats(crawlId: string): Promise<CrawlStats> {
    return this.request<CrawlStats>('GET', `/api/v1/crawls/${crawlId}/stats`);
  }

  async getFindings(crawlId: string): Promise<Finding[]> {
    return this.request<Finding[]>('GET', `/api/v1/crawls/${crawlId}/findings`);
  }

  async getBacklinks(crawlId: string): Promise<BacklinksResponse> {
    return this.request<BacklinksResponse>('GET', `/api/v1/crawls/${crawlId}/backlinks`);
  }

  async listCrawls(): Promise<CrawlResult[]> {
    return this.request<CrawlResult[]>('GET', '/api/v1/crawls');
  }

  // Users

  async listUsers(): Promise<UserResponse[]> {
    return this.request<UserResponse[]>('GET', '/api/v1/users');
  }

  async createUser(body: CreateUserRequest): Promise<UserResponse> {
    return this.request<UserResponse>('POST', '/api/v1/users', body);
  }

  async deleteUser(userId: string): Promise<void> {
    return this.request<void>('DELETE', `/api/v1/users/${userId}`);
  }

  // Tenants

  async listTenants(): Promise<Tenant[]> {
    return this.request<Tenant[]>('GET', '/api/v1/tenants');
  }

  async createTenant(body: CreateTenantRequest): Promise<Tenant> {
    return this.request<Tenant>('POST', '/api/v1/tenants', body);
  }

  // Auth

  async login(email: string, password: string): Promise<LoginResponse> {
    const body: LoginRequest = { email, password };
    return this.request<LoginResponse>('POST', '/api/v1/auth/login', body);
  }

  async getMe(): Promise<UserResponse> {
    return this.request<UserResponse>('GET', '/api/v1/auth/me');
  }

  async refreshToken(): Promise<LoginResponse> {
    return this.request<LoginResponse>('POST', '/api/v1/auth/refresh');
  }

  // API Keys

  async createApiKey(body: ApiKeyCreateRequest): Promise<ApiKeyResponse> {
    return this.request<ApiKeyResponse>('POST', '/api/v1/keys', body);
  }

  async listApiKeys(): Promise<ApiKeyResponse[]> {
    return this.request<ApiKeyResponse[]>('GET', '/api/v1/keys');
  }

  async deleteApiKey(key: string): Promise<void> {
    return this.request<void>('DELETE', `/api/v1/keys/${key}`);
  }

  // Webhooks

  async createWebhook(body: CreateWebhookRequest): Promise<WebhookConfig> {
    return this.request<WebhookConfig>('POST', '/api/v1/webhooks', body);
  }

  async listWebhooks(): Promise<WebhookConfig[]> {
    return this.request<WebhookConfig[]>('GET', '/api/v1/webhooks');
  }

  async deleteWebhook(id: string): Promise<void> {
    return this.request<void>('DELETE', `/api/v1/webhooks/${id}`);
  }

  // Schedules

  async createSchedule(body: CreateScheduleRequest): Promise<ScheduleResponse> {
    return this.request<ScheduleResponse>('POST', '/api/v1/schedules', body);
  }

  async listSchedules(): Promise<ScheduleResponse[]> {
    return this.request<ScheduleResponse[]>('GET', '/api/v1/schedules');
  }

  async deleteSchedule(id: string): Promise<void> {
    return this.request<void>('DELETE', `/api/v1/schedules/${id}`);
  }

  // Audit

  async getAuditEvents(): Promise<AuditEvent[]> {
    return this.request<AuditEvent[]>('GET', '/api/v1/audit');
  }

  // Metrics (Prometheus text format)

  async getMetrics(): Promise<string> {
    const headers: Record<string, string> = {};
    if (this.token) {
      headers['Authorization'] = `Bearer ${this.token}`;
    }
    const response = await fetch(`${BASE_URL}/metrics`, { headers });
    return response.text();
  }
}

export const apiClient = new ApiClient();
