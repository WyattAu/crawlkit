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

  async health() {
    return this.request<Record<string, unknown>>('GET', '/health');
  }

  async startCrawl(body: Record<string, unknown>) {
    return this.request<Record<string, unknown>>('POST', '/api/v1/crawls', body);
  }

  async getCrawl(crawlId: string) {
    return this.request<Record<string, unknown>>('GET', `/api/v1/crawls/${crawlId}`);
  }

  async getCrawlStats(crawlId: string) {
    return this.request<Record<string, unknown>>('GET', `/api/v1/crawls/${crawlId}/stats`);
  }

  async listCrawls() {
    return this.request<Record<string, unknown>[]>('GET', '/api/v1/crawls');
  }

  async listUsers() {
    return this.request<Record<string, unknown>[]>('GET', '/api/v1/users');
  }

  async createUser(body: Record<string, unknown>) {
    return this.request<Record<string, unknown>>('POST', '/api/v1/users', body);
  }

  async deleteUser(userId: string) {
    return this.request<void>('DELETE', `/api/v1/users/${userId}`);
  }

  async listTenants() {
    return this.request<Record<string, unknown>[]>('GET', '/api/v1/tenants');
  }

  async createTenant(body: Record<string, unknown>) {
    return this.request<Record<string, unknown>>('POST', '/api/v1/tenants', body);
  }

  async login(email: string, password: string) {
    return this.request<Record<string, unknown>>('POST', '/api/v1/auth/login', { email, password });
  }

  async getMetrics() {
    const response = await fetch(`${BASE_URL}/metrics`, {
      headers: this.token ? { Authorization: `Bearer ${this.token}` } : {},
    });
    return response.text();
  }
}

export const apiClient = new ApiClient();
