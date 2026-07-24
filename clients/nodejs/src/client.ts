import {
  CrawlRequest,
  CrawlResponse,
  CrawlStats,
  User,
  CreateUserRequest,
  Tenant,
} from "./models";

export class CrawlkitClient {
  private baseURL: string;
  private apiKey: string;
  private jwtToken: string;

  constructor(baseURL: string, apiKey: string = "", jwtToken: string = "") {
    this.baseURL = baseURL.replace(/\/$/, "");
    this.apiKey = apiKey;
    this.jwtToken = jwtToken;
  }

  async health(): Promise<Record<string, unknown>> {
    return this.get("/health");
  }

  async startCrawl(req: CrawlRequest): Promise<CrawlResponse> {
    return this.post("/api/v1/crawls", req);
  }

  async getCrawl(crawlId: string): Promise<Record<string, unknown>> {
    return this.get(`/api/v1/crawls/${crawlId}`);
  }

  async getCrawlStats(crawlId: string): Promise<CrawlStats> {
    return this.get(`/api/v1/crawls/${crawlId}/stats`);
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

  async createTenant(
    name: string,
    plan: string = "free",
  ): Promise<Tenant> {
    return this.post("/api/v1/tenants", { name, plan });
  }

  async login(email: string, password: string): Promise<string> {
    const result: Record<string, string> = await this.post(
      "/api/v1/auth/login",
      { email, password },
    );
    return result.token;
  }

  private async get(path: string): Promise<any> {
    const headers: Record<string, string> = {};
    if (this.apiKey) headers["X-API-Key"] = this.apiKey;
    if (this.jwtToken) headers["Authorization"] = `Bearer ${this.jwtToken}`;

    const response = await fetch(`${this.baseURL}${path}`, { headers });
    if (!response.ok) {
      throw new Error(
        `HTTP ${response.status}: ${await response.text()}`,
      );
    }
    return response.json();
  }

  private async post(path: string, body: unknown): Promise<any> {
    const headers: Record<string, string> = {
      "Content-Type": "application/json",
    };
    if (this.apiKey) headers["X-API-Key"] = this.apiKey;
    if (this.jwtToken) headers["Authorization"] = `Bearer ${this.jwtToken}`;

    const response = await fetch(`${this.baseURL}${path}`, {
      method: "POST",
      headers,
      body: JSON.stringify(body),
    });
    if (!response.ok) {
      throw new Error(
        `HTTP ${response.status}: ${await response.text()}`,
      );
    }
    return response.json();
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
      throw new Error(
        `HTTP ${response.status}: ${await response.text()}`,
      );
    }
  }
}
