import { useEffect, useRef, useState } from 'react';
import { Loader, Search } from 'lucide-react';
import { apiClient } from '../services/api_client';
import { useAuth } from '../hooks/use_auth';
import PluginCard from '../components/marketplace/PluginCard';
import PluginDetail from '../components/marketplace/PluginDetail';
import type { MarketplacePlugin } from '../models/types';

const SEARCH_DEBOUNCE_MS = 300;

export default function MarketplacePage() {
  const { token } = useAuth();
  const [plugins, setPlugins] = useState<MarketplacePlugin[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [query, setQuery] = useState('');
  const [category, setCategory] = useState('');
  const [categories, setCategories] = useState<string[]>([]);
  const [selected, setSelected] = useState<MarketplacePlugin | null>(null);
  const [isAdmin, setIsAdmin] = useState(false);
  const searchSeq = useRef(0);

  useEffect(() => {
    if (!token) return;
    apiClient.setToken(token);
    apiClient
      .getMe()
      .then((user) => setIsAdmin(user.roles.includes('admin')))
      .catch(() => setIsAdmin(false));
  }, [token]);

  useEffect(() => {
    let cancelled = false;
    const timer = setTimeout(async () => {
      const seq = ++searchSeq.current;
      try {
        const q = query.trim();
        const data =
          q || category
            ? await apiClient.searchMarketplacePlugins(q || undefined, category || undefined)
            : await apiClient.listMarketplacePlugins();
        if (cancelled || seq !== searchSeq.current) return;
        setPlugins(data);
        setError('');
        if (!q && !category) {
          setCategories(
            Array.from(new Set(data.flatMap((plugin) => plugin.categories))).sort()
          );
        }
      } catch (err) {
        if (cancelled || seq !== searchSeq.current) return;
        setError(err instanceof Error ? err.message : 'Failed to load plugins');
      } finally {
        if (!cancelled && seq === searchSeq.current) setLoading(false);
      }
    }, SEARCH_DEBOUNCE_MS);

    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [query, category]);

  function handleUpdate(updated: MarketplacePlugin) {
    setPlugins((prev) =>
      prev.map((plugin) => (plugin.name === updated.name ? updated : plugin))
    );
    setSelected((prev) => (prev && prev.name === updated.name ? updated : prev));
  }

  return (
    <div>
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 mb-6">
        <h1 className="text-2xl font-bold text-gray-900 dark:text-white">
          Marketplace
        </h1>
        <div className="flex flex-col sm:flex-row gap-3">
          <div className="relative">
            <Search
              className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-400 pointer-events-none"
              aria-hidden="true"
            />
            <input
              type="search"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder="Search plugins..."
              aria-label="Search plugins"
              className="block w-full sm:w-64 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 pl-9 pr-3 py-2 text-sm text-gray-900 dark:text-gray-100 placeholder-gray-400 dark:placeholder-gray-500 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
            />
          </div>
          <select
            value={category}
            onChange={(e) => setCategory(e.target.value)}
            aria-label="Filter by category"
            className="block w-full sm:w-48 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 px-3 py-2 text-sm text-gray-900 dark:text-gray-100 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
          >
            <option value="">All categories</option>
            {categories.map((c) => (
              <option key={c} value={c}>
                {c}
              </option>
            ))}
          </select>
        </div>
      </div>

      {error && (
        <div
          role="alert"
          className="mb-4 p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg text-sm text-red-600 dark:text-red-400"
        >
          {error}
        </div>
      )}

      {loading ? (
        <div role="status" className="flex items-center justify-center py-12">
          <Loader className="w-8 h-8 text-blue-500 animate-spin" />
          <span className="sr-only">Loading plugins...</span>
        </div>
      ) : plugins.length === 0 ? (
        <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-12 text-center">
          <p className="text-sm text-gray-500 dark:text-gray-400">
            No plugins found matching your criteria.
          </p>
        </div>
      ) : (
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
          {plugins.map((plugin) => (
            <PluginCard
              key={plugin.name}
              plugin={plugin}
              onClick={setSelected}
            />
          ))}
        </div>
      )}

      <PluginDetail
        plugin={selected}
        open={selected !== null}
        onOpenChange={(open) => {
          if (!open) setSelected(null);
        }}
        isAdmin={isAdmin}
        onUpdate={handleUpdate}
      />
    </div>
  );
}
