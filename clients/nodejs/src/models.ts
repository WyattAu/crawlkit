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

export interface CrawlResult {
  crawl_id: string;
  start_url: string;
  status: string;
  pages_crawled: number;
  issues_found: number;
  created_at: string;
  completed_at?: string;
}

export interface CrawlStats {
  crawl_id: string;
  total_pages: number;
  total_issues: number;
  issues_by_severity: Record<string, number>;
  issues_by_category: Record<string, number>;
  avg_response_time_ms?: number;
}

export interface Finding {
  id: string;
  page_id: string;
  category: string;
  severity: string;
  code: string;
  title: string;
  description: string;
  element?: string;
  recommendation: string;
}

export interface BacklinksResponse {
  crawl_id: string;
  total_internal_links: number;
  total_external_links: number;
  total_referring_domains: number;
  orphan_pages: string[];
  top_pages_by_pagerank: PageRankEntry[];
}

export interface PageRankEntry {
  url: string;
  pagerank: number;
  inbound_links: number;
  outbound_links: number;
  referring_domains: number;
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
  tenant_id?: string;
  roles?: string[];
}

export interface Tenant {
  id: string;
  name: string;
  created_at: string;
}

export interface CreateTenantRequest {
  id: string;
  name: string;
}

export interface ApiKeyInfo {
  key: string;
  name: string;
  requests_per_minute: number;
}

export interface CreateApiKeyRequest {
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

export interface AuditEvent {
  id: string;
  action: string;
  resource: string;
  user_id: string;
  details?: string;
  created_at: string;
}

export interface HealthResponse {
  status: string;
  version: string;
}

export interface LoginResponse {
  token: string;
  user: User;
}

export interface MarketplacePlugin {
  name: string;
  version: string;
  author: string;
  description: string;
  license: string;
  categories: string[];
  tags: string[];
  downloads: number;
  rating: number;
  created_at: string;
  updated_at: string;
}

export interface SubmitPluginRequest {
  name: string;
  version: string;
  author: string;
  description: string;
  license: string;
  categories: string[];
  tags: string[];
  repository?: string;
  homepage?: string;
}

export interface UpdateScheduleRequest {
  start_url?: string;
  max_pages?: number;
  interval_secs?: number;
  enabled?: boolean;
}

export interface Session {
  id: string;
  user_id: string;
  tenant_id: string;
  ip_address: string;
  user_agent: string;
  created_at: string;
  last_active_at: string;
  expires_at: string;
}

export interface PluginTestResult {
  success: boolean;
  message: string;
}
