import { useEffect, useMemo, useState } from 'react';
import {
  CartesianGrid,
  Cell,
  Legend,
  Pie,
  PieChart,
  ResponsiveContainer,
  Scatter,
  ScatterChart,
  Tooltip,
  XAxis,
  YAxis,
} from 'recharts';
import { AlertCircle, Loader } from 'lucide-react';
import { apiClient } from '../services/api_client';
import { useAuth } from '../hooks/use_auth';
import { useCrawls } from '../hooks/use_crawls';
import HealthScoreGauge from '../components/insights/HealthScoreGauge';
import InsightCard from '../components/insights/InsightCard';
import type {
  Finding,
  Insight,
  InsightCategory,
  InsightPriority,
} from '../models/types';

const categoryLabels: Record<InsightCategory, string> = {
  technical: 'Technical',
  content: 'Content',
  seo: 'SEO',
  security: 'Security',
  performance: 'Performance',
};

const categoryColors: Record<InsightCategory, string> = {
  technical: '#3b82f6',
  content: '#8b5cf6',
  seo: '#22c55e',
  security: '#ef4444',
  performance: '#f59e0b',
};

const priorityColors: Record<InsightPriority, string> = {
  critical: '#ef4444',
  high: '#f97316',
  medium: '#eab308',
  low: '#3b82f6',
};

const effortOrder: Insight['effort'][] = ['quick', 'moderate', 'significant'];
const effortTicks = ['Quick', 'Moderate', 'Significant'];

function extractHealthScore(findings: Finding[]): number {
  const healthFinding = findings.find((f) => f.code === 'HEALTH001');
  if (!healthFinding) return 100;
  const match = healthFinding.title.match(/(\d+)\s*\/\s*100/);
  return match ? parseInt(match[1], 10) : 100;
}

export default function InsightsPage() {
  const { token } = useAuth();
  const { crawls, loading: crawlsLoading } = useCrawls();
  const [selectedId, setSelectedId] = useState<string>('');
  const [insights, setInsights] = useState<Insight[]>([]);
  const [healthScore, setHealthScore] = useState<number>(100);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const completedCrawls = useMemo(
    () => crawls.filter((c) => c.status === 'completed' || c.status === 'success'),
    [crawls]
  );

  useEffect(() => {
    if (!selectedId && completedCrawls.length > 0) {
      setSelectedId(completedCrawls[0].crawl_id);
    }
  }, [completedCrawls, selectedId]);

  useEffect(() => {
    if (!token || !selectedId) return;
    apiClient.setToken(token);

    let cancelled = false;

    async function load() {
      setLoading(true);
      setError(null);
      try {
        const [insightData, findingData] = await Promise.all([
          apiClient.getInsights(selectedId).catch(() => [] as Insight[]),
          apiClient.getFindings(selectedId).catch(() => [] as Finding[]),
        ]);
        if (cancelled) return;
        setInsights(insightData);
        setHealthScore(extractHealthScore(findingData));
      } catch (err) {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : 'Failed to load insights');
        }
      } finally {
        if (!cancelled) setLoading(false);
      }
    }

    load();
    return () => {
      cancelled = true;
    };
  }, [selectedId, token]);

  const sortedInsights = useMemo(
    () => [...insights].sort((a, b) => b.impact_score - a.impact_score),
    [insights]
  );

  const categoryData = useMemo(() => {
    const counts = new Map<InsightCategory, number>();
    for (const insight of insights) {
      counts.set(insight.category, (counts.get(insight.category) || 0) + 1);
    }
    return Array.from(counts.entries()).map(([category, value]) => ({
      name: categoryLabels[category],
      value,
      color: categoryColors[category],
    }));
  }, [insights]);

  const matrixData = useMemo(
    () =>
      (['critical', 'high', 'medium', 'low'] as InsightPriority[]).map(
        (priority) => ({
          priority,
          color: priorityColors[priority],
          points: sortedInsights
            .filter((i) => i.priority === priority)
            .map((i) => ({
              x: effortOrder.indexOf(i.effort),
              y: Math.round(i.impact_score),
              title: i.title,
            })),
        })
      ),
    [sortedInsights]
  );

  if (crawlsLoading) {
    return (
      <div role="status" className="flex items-center justify-center py-12">
        <Loader className="w-8 h-8 text-blue-500 animate-spin" />
        <span className="sr-only">Loading...</span>
      </div>
    );
  }

  if (completedCrawls.length === 0) {
    return (
      <div>
        <h1 className="text-2xl font-bold mb-6 text-gray-900 dark:text-white">
          Insights
        </h1>
        <div className="text-center py-12 text-gray-500 dark:text-gray-400">
          No completed crawls yet. Run a crawl to generate insights.
        </div>
      </div>
    );
  }

  return (
    <div>
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 mb-6">
        <h1 className="text-2xl font-bold text-gray-900 dark:text-white">
          Insights
        </h1>
        <label className="flex items-center gap-2 text-sm text-gray-500 dark:text-gray-400">
          Crawl
          <select
            value={selectedId}
            onChange={(e) => setSelectedId(e.target.value)}
            className="rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 px-3 py-2 text-sm text-gray-900 dark:text-white max-w-xs truncate"
          >
            {completedCrawls.map((crawl) => (
              <option key={crawl.crawl_id} value={crawl.crawl_id}>
                {crawl.crawl_id} — {crawl.start_url}
              </option>
            ))}
          </select>
        </label>
      </div>

      {error && (
        <div
          role="alert"
          className="mb-6 p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg text-sm text-red-600 dark:text-red-400 flex items-center gap-2"
        >
          <AlertCircle className="w-4 h-4 shrink-0" />
          {error}
        </div>
      )}

      {loading ? (
        <div role="status" className="flex items-center justify-center py-12">
          <Loader className="w-8 h-8 text-blue-500 animate-spin" />
          <span className="sr-only">Loading insights...</span>
        </div>
      ) : (
        <>
          <div className="grid grid-cols-1 lg:grid-cols-3 gap-6 mb-8">
            <div className="bg-white dark:bg-gray-800 rounded-xl p-6 shadow-sm border border-gray-200 dark:border-gray-700 flex flex-col items-center justify-center">
              <h2 className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-4">
                Overall Health Score
              </h2>
              <HealthScoreGauge score={healthScore} />
            </div>

            <div className="bg-white dark:bg-gray-800 rounded-xl p-6 shadow-sm border border-gray-200 dark:border-gray-700">
              <h2 className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-4">
                Insights by Category
              </h2>
              {categoryData.length > 0 ? (
                <ResponsiveContainer width="100%" height={280}>
                  <PieChart>
                    <Pie
                      data={categoryData}
                      dataKey="value"
                      nameKey="name"
                      cx="50%"
                      cy="50%"
                      innerRadius={50}
                      outerRadius={90}
                      paddingAngle={2}
                    >
                      {categoryData.map((entry) => (
                        <Cell key={entry.name} fill={entry.color} />
                      ))}
                    </Pie>
                    <Tooltip />
                    <Legend />
                  </PieChart>
                </ResponsiveContainer>
              ) : (
                <div className="h-[280px] flex items-center justify-center text-sm text-gray-500 dark:text-gray-400">
                  No insights to break down.
                </div>
              )}
            </div>

            <div className="bg-white dark:bg-gray-800 rounded-xl p-6 shadow-sm border border-gray-200 dark:border-gray-700">
              <h2 className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-4">
                Effort vs Impact
              </h2>
              {sortedInsights.length > 0 ? (
                <ResponsiveContainer width="100%" height={280}>
                  <ScatterChart margin={{ top: 8, right: 8, bottom: 8, left: -16 }}>
                    <CartesianGrid strokeDasharray="3 3" stroke="#e5e7eb" />
                    <XAxis
                      type="number"
                      dataKey="x"
                      domain={[-0.3, 2.3]}
                      ticks={[0, 1, 2]}
                      tickFormatter={(tick: number) => effortTicks[tick] ?? ''}
                      tick={{ fontSize: 12 }}
                    />
                    <YAxis
                      type="number"
                      dataKey="y"
                      domain={[0, 100]}
                      tick={{ fontSize: 12 }}
                    />
                    <Tooltip
                      formatter={(value, _name, entry) =>
                        (entry?.payload as { title?: string })?.title ?? String(value)
                      }
                    />
                    {matrixData.map((series) => (
                      <Scatter
                        key={series.priority}
                        name={series.priority}
                        data={series.points}
                        fill={series.color}
                      />
                    ))}
                  </ScatterChart>
                </ResponsiveContainer>
              ) : (
                <div className="h-[280px] flex items-center justify-center text-sm text-gray-500 dark:text-gray-400">
                  No insights to plot.
                </div>
              )}
            </div>
          </div>

          <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
            Prioritized Insights
            <span className="ml-2 text-sm font-normal text-gray-500 dark:text-gray-400">
              ({sortedInsights.length})
            </span>
          </h2>

          {sortedInsights.length === 0 ? (
            <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 text-center py-12 text-gray-500 dark:text-gray-400">
              No insights generated for this crawl.
            </div>
          ) : (
            <div className="space-y-3">
              {sortedInsights.map((insight) => (
                <InsightCard
                  key={insight.finding_codes.join('|')}
                  insight={insight}
                />
              ))}
            </div>
          )}
        </>
      )}
    </div>
  );
}
