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
  Finding,
} from '../models/types';

const BASE_URL = import.meta.env.VITE_API_URL || 'http://localhost:4000';

class ApiClient {
  private token: string | null = null;

  setToken(token: string) {
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
        const errorBody = await response.json();
        if (errorBody.message) message = errorBody.message;
      } catch {
        // response body wasn't JSON
      }
      throw new Error(message);
    }

    return response.json();
  }

  async health(): Promise<HealthResponse> {
    return this.request<HealthResponse>('GET', '/health');
  }

  async startCrawl(body: {
    start_url: string;
    max_pages?: number;
    request_delay_ms?: number;
    concurrency?: number;
  }): Promise<CrawlResponse> {
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

  async listCrawls(): Promise<CrawlResult[]> {
    return this.request<CrawlResult[]>('GET', '/api/v1/crawls');
  }

  async listUsers(): Promise<UserResponse[]> {
    return this.request<UserResponse[]>('GET', '/api/v1/users');
  }

  async createUser(body: CreateUserRequest): Promise<UserResponse> {
    return this.request<UserResponse>('POST', '/api/v1/users', body);
  }

  async deleteUser(userId: string): Promise<void> {
    return this.request<void>('DELETE', `/api/v1/users/${userId}`);
  }

  async listTenants(): Promise<Tenant[]> {
    return this.request<Tenant[]>('GET', '/api/v1/tenants');
  }

  async createTenant(body: CreateTenantRequest): Promise<Tenant> {
    return this.request<Tenant>('POST', '/api/v1/tenants', body);
  }

  async login(email: string, password: string): Promise<LoginResponse> {
    return this.request<LoginResponse>('POST', '/api/v1/auth/login', { email, password });
  }

  async getMetrics(): Promise<string> {
    const response = await fetch(`${BASE_URL}/metrics`, {
      headers: this.token ? { Authorization: `Bearer ${this.token}` } : {},
    });
    return response.text();
  }
}

export const apiClient = new ApiClient();
