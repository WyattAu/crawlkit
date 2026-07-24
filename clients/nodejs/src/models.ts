export interface CrawlRequest {
  start_url: string;
  max_pages?: number;
  request_delay_ms?: number;
  concurrency?: number;
  tenant_id?: string;
}

export interface CrawlResponse {
  crawl_id: string;
  status: string;
  message: string;
}

export interface CrawlStats {
  total_pages: number;
  total_issues: number;
  avg_response_time_ms: number;
  total_body_size: number;
}

export interface User {
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
  roles?: string[];
}

export interface Tenant {
  id: string;
  name: string;
  plan: string;
  max_users: number;
  max_crawls_per_month: number;
}
