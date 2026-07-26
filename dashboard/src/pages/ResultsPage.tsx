import { useEffect, useState } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { apiClient } from '../services/api_client';
import { useAuth } from '../hooks/use_auth';
import FindingList from '../components/findings/FindingList';
import Button from '../components/ui/Button';
import { ArrowLeft, Loader } from 'lucide-react';
import type { CrawlResult, CrawlStats, Finding } from '../models/types';

interface CrawlDetail extends CrawlResult {
  stats?: CrawlStats;
}

export default function ResultsPage() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { token } = useAuth();
  const [crawl, setCrawl] = useState<CrawlDetail | null>(null);
  const [findings, setFindings] = useState<Finding[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    if (!token || !id) return;
    apiClient.setToken(token);

    async function load() {
      try {
        const [crawlData, statsData, findingsData] = await Promise.all([
          apiClient.getCrawl(id!),
          apiClient.getCrawlStats(id!).catch(() => null),
          apiClient.getFindings(id!).catch(() => []),
        ]);
        setCrawl({ ...crawlData, stats: statsData ?? undefined });
        setFindings(findingsData);
      } catch (error) {
        console.error('Failed to load crawl:', error);
      } finally {
        setLoading(false);
      }
    }

    load();
  }, [id, token]);

  if (loading) {
    return (
      <div role="status" className="flex items-center justify-center py-12">
        <Loader className="w-8 h-8 text-blue-500 animate-spin" />
        <span className="sr-only">Loading...</span>
      </div>
    );
  }

  if (!crawl) {
    return (
      <div className="text-center py-12 text-gray-500 dark:text-gray-400">
        Crawl not found.
      </div>
    );
  }

  return (
    <div>
      <div className="flex items-center gap-4 mb-6">
        <Button variant="ghost" onClick={() => navigate('/crawls')}>
          <ArrowLeft className="w-4 h-4 mr-2" />
          Back
        </Button>
        <div>
          <h1 className="text-2xl font-bold text-gray-900 dark:text-white">{crawl.crawl_id}</h1>
          <p className="text-sm text-gray-500 dark:text-gray-400">{crawl.start_url}</p>
        </div>
      </div>

      {crawl.stats && (
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-8">
          <div className="bg-white dark:bg-gray-800 rounded-xl p-4 border border-gray-200 dark:border-gray-700">
            <p className="text-sm text-gray-500 dark:text-gray-400">Total Pages</p>
            <p className="text-2xl font-bold text-gray-900 dark:text-white">{crawl.stats.total_pages}</p>
          </div>
          <div className="bg-white dark:bg-gray-800 rounded-xl p-4 border border-gray-200 dark:border-gray-700">
            <p className="text-sm text-gray-500 dark:text-gray-400">Total Issues</p>
            <p className="text-2xl font-bold text-gray-900 dark:text-white">{crawl.stats.total_issues}</p>
          </div>
          <div className="bg-white dark:bg-gray-800 rounded-xl p-4 border border-gray-200 dark:border-gray-700">
            <p className="text-sm text-gray-500 dark:text-gray-400">Pages Crawled</p>
            <p className="text-2xl font-bold text-gray-900 dark:text-white">{crawl.pages_crawled}</p>
          </div>
          <div className="bg-white dark:bg-gray-800 rounded-xl p-4 border border-gray-200 dark:border-gray-700">
            <p className="text-sm text-gray-500 dark:text-gray-400">Issues Found</p>
            <p className="text-2xl font-bold text-gray-900 dark:text-white">{crawl.issues_found}</p>
          </div>
        </div>
      )}

      <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">Findings</h2>
      <FindingList findings={findings} loading={false} />
    </div>
  );
}
