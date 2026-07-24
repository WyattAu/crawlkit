export interface Tenant {
  id: string;
  name: string;
  slug: string;
  created_at: string;
  user_count?: number;
  crawl_count?: number;
}

export interface CreateTenantRequest {
  name: string;
  slug: string;
}
