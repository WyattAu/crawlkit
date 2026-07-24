import { useState, useEffect, useCallback } from 'react';
import { apiClient } from '../services/api_client';
import { useAuth } from './use_auth';

export interface Crawl {
  id: string;
  tenant_id: string;
  name: string;
  url: string;
  status: string;
  created_at: string;
  updated_at: string;
  config: Record<string, unknown>;
}

export function useCrawls() {
  const { token } = useAuth();
  const [crawls, setCrawls] = useState<Crawl[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadCrawls = useCallback(async () => {
    if (!token) return;
    apiClient.setToken(token);
    try {
      setLoading(true);
      const data = await apiClient.listCrawls();
      setCrawls(data);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load crawls');
    } finally {
      setLoading(false);
    }
  }, [token]);

  useEffect(() => {
    loadCrawls();
  }, [loadCrawls]);

  const startCrawl = useCallback(
    async (config: Record<string, unknown>) => {
      if (!token) return;
      apiClient.setToken(token);
      const crawl = await apiClient.startCrawl(config);
      setCrawls((prev) => [crawl, ...prev]);
      return crawl;
    },
    [token]
  );

  return { crawls, loading, error, loadCrawls, startCrawl };
}
