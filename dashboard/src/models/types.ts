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

export interface CreateCrawlRequest {
  start_url: string;
  max_pages?: number;
  request_delay_ms?: number;
  concurrency?: number;
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

export interface LoginRequest {
  email: string;
  password: string;
}

export interface LoginResponse {
  token: string;
  user: UserResponse;
}

export interface Finding {
  id: string;
  page_id: string;
  category: string;
  severity: string;
  code: string;
  title: string;
  description: string;
  element: string | null;
  recommendation: string;
}

export interface ApiKeyResponse {
  key: string;
  name: string;
  requests_per_minute: number;
}

export interface ApiKeyCreateRequest {
  name: string;
  requests_per_minute?: number;
}

export interface WebhookConfig {
  id: string;
  url: string;
  events: string[];
  created_at: string;
}

export interface CreateWebhookRequest {
  url: string;
  events?: string[];
}

export interface ScheduleResponse {
  id: string;
  start_url: string;
  interval_secs: number;
  enabled: boolean;
  next_run: string;
  created_at: string;
}

export interface CreateScheduleRequest {
  start_url: string;
  max_pages?: number;
  request_delay_ms?: number;
  concurrency?: number;
  interval_secs?: number;
}

export interface BacklinksResponse {
  crawl_id: string;
  total_internal_links: number;
  total_external_links: number;
  total_referring_domains: number;
  orphan_pages: string[];
  top_pages_by_pagerank: {
    url: string;
    pagerank: number;
    inbound_links: number;
    outbound_links: number;
    referring_domains: number;
  }[];
}

export interface AuditEvent {
  timestamp: string;
  event: string;
  details: string;
}
