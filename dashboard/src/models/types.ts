export interface CrawlResult {
  crawl_id: string;
  start_url: string;
  status: string;
  pages_crawled: number;
  issues_found: number;
  created_at: string;
  completed_at: string | null;
}

export interface CrawlResponse {
  crawl_id: string;
  status: string;
  message: string;
}

export interface CrawlStats {
  crawl_id: string;
  total_pages: number;
  total_issues: number;
  issues_by_severity: Record<string, number>;
  issues_by_category: Record<string, number>;
  avg_response_time_ms: number | null;
}

export interface UserResponse {
  id: string;
  email: string;
  name: string;
  tenant_id: string;
  roles: string[];
  enabled: boolean;
}

export interface CreateUserRequest {
  email: string;
  name: string;
  password: string;
  tenant_id: string;
  roles: string[];
}

export interface Tenant {
  id: string;
  name: string;
  created_at: string;
}

export interface CreateTenantRequest {
  name: string;
}

export interface HealthResponse {
  status: string;
  version: string;
}

export interface LoginResponse {
  token: string;
  user: UserResponse;
}

export interface Finding {
  id: string;
  type: string;
  severity: string;
  title: string;
  description: string;
  url: string;
  evidence?: string;
  created_at: string;
}
