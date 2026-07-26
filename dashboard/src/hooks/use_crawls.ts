import { useState, useEffect, useCallback } from 'react';
import { apiClient } from '../services/api_client';
import { useAuth } from './use_auth';
import type { CrawlResult } from '../models/types';

export function useCrawls() {
  const { token } = useAuth();
  const [crawls, setCrawls] = useState<CrawlResult[]>([]);
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
    async (config: { name: string; url: string; max_pages?: number; max_depth?: number; respect_robots?: boolean }) => {
      if (!token) return;
      apiClient.setToken(token);
      const response = await apiClient.startCrawl({
        start_url: config.url,
        max_pages: config.max_pages,
      });
      const pending: CrawlResult = {
        crawl_id: response.crawl_id,
        start_url: config.url,
        status: response.status,
        pages_crawled: 0,
        issues_found: 0,
        created_at: new Date().toISOString(),
        completed_at: null,
      };
      setCrawls((prev) => [pending, ...prev]);
      return pending;
    },
    [token]
  );

  return { crawls, loading, error, loadCrawls, startCrawl };
}
