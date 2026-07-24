import { useState, useEffect, useCallback } from 'react';
import { apiClient } from '../services/api_client';
import { useAuth } from './use_auth';

export interface Metrics {
  totalCrawls: number;
  activeCrawls: number;
  completedCrawls: number;
  failedCrawls: number;
  totalUsers: number;
  totalTenants: number;
}

export function useMetrics() {
  const { token } = useAuth();
  const [metrics, setMetrics] = useState<Metrics>({
    totalCrawls: 0,
    activeCrawls: 0,
    completedCrawls: 0,
    failedCrawls: 0,
    totalUsers: 0,
    totalTenants: 0,
  });
  const [loading, setLoading] = useState(true);

  const loadMetrics = useCallback(async () => {
    if (!token) return;
    apiClient.setToken(token);
    try {
      setLoading(true);
      const [crawls, users, tenants] = await Promise.all([
        apiClient.listCrawls(),
        apiClient.listUsers().catch(() => []),
        apiClient.listTenants().catch(() => []),
      ]);

      setMetrics({
        totalCrawls: crawls.length,
        activeCrawls: crawls.filter((c: Record<string, unknown>) => c.status === 'running').length,
        completedCrawls: crawls.filter((c: Record<string, unknown>) => c.status === 'completed').length,
        failedCrawls: crawls.filter((c: Record<string, unknown>) => c.status === 'failed').length,
        totalUsers: users.length,
        totalTenants: tenants.length,
      });
    } catch {
      // silently fail on metrics
    } finally {
      setLoading(false);
    }
  }, [token]);

  useEffect(() => {
    loadMetrics();
  }, [loadMetrics]);

  return { metrics, loading, refresh: loadMetrics };
}
