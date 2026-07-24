export interface Crawl {
  id: string;
  tenant_id: string;
  name: string;
  url: string;
  status: 'pending' | 'running' | 'completed' | 'failed';
  config: CrawlConfig;
  created_at: string;
  updated_at: string;
}

export interface CrawlConfig {
  max_depth?: number;
  max_pages?: number;
  respect_robots?: boolean;
  include_patterns?: string[];
  exclude_patterns?: string[];
  custom_headers?: Record<string, string>;
}

export interface CrawlStats {
  crawl_id: string;
  pages_crawled: number;
  pages_failed: number;
  links_found: number;
  duration_ms: number;
  findings_count: number;
}

export interface Finding {
  id: string;
  crawl_id: string;
  type: string;
  severity: 'info' | 'low' | 'medium' | 'high' | 'critical';
  title: string;
  description: string;
  url: string;
  evidence?: string;
  created_at: string;
}
