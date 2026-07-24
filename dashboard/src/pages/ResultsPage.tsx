import { useEffect, useState } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { apiClient } from '../services/api_client';
import { useAuth } from '../hooks/use_auth';
import FindingList from '../components/findings/FindingList';
import Button from '../components/ui/Button';
import { ArrowLeft, Loader } from 'lucide-react';

interface CrawlDetail {
  id: string;
  name: string;
  url: string;
  status: string;
  created_at: string;
  stats?: {
    pages_crawled: number;
    pages_failed: number;
    links_found: number;
    duration_ms: number;
  };
}

interface Finding {
  id: string;
  type: string;
  severity: string;
  title: string;
  description: string;
  url: string;
  evidence?: string;
  created_at: string;
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
        const [crawlData, statsData] = await Promise.all([
          apiClient.getCrawl(id!),
          apiClient.getCrawlStats(id!).catch(() => null),
        ]);
        setCrawl({ ...crawlData, stats: statsData } as CrawlDetail);

        const findingsData = await apiClient
          .listCrawls()
          .then(() => []) 
          .catch(() => []);
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
      <div className="flex items-center justify-center py-12">
        <Loader className="w-8 h-8 text-blue-500 animate-spin" />
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
          <h1 className="text-2xl font-bold text-gray-900 dark:text-white">{crawl.name}</h1>
          <p className="text-sm text-gray-500 dark:text-gray-400">{crawl.url}</p>
        </div>
      </div>

      {crawl.stats && (
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-8">
          <div className="bg-white dark:bg-gray-800 rounded-xl p-4 border border-gray-200 dark:border-gray-700">
            <p className="text-sm text-gray-500 dark:text-gray-400">Pages Crawled</p>
            <p className="text-2xl font-bold text-gray-900 dark:text-white">{crawl.stats.pages_crawled}</p>
          </div>
          <div className="bg-white dark:bg-gray-800 rounded-xl p-4 border border-gray-200 dark:border-gray-700">
            <p className="text-sm text-gray-500 dark:text-gray-400">Pages Failed</p>
            <p className="text-2xl font-bold text-gray-900 dark:text-white">{crawl.stats.pages_failed}</p>
          </div>
          <div className="bg-white dark:bg-gray-800 rounded-xl p-4 border border-gray-200 dark:border-gray-700">
            <p className="text-sm text-gray-500 dark:text-gray-400">Links Found</p>
            <p className="text-2xl font-bold text-gray-900 dark:text-white">{crawl.stats.links_found}</p>
          </div>
          <div className="bg-white dark:bg-gray-800 rounded-xl p-4 border border-gray-200 dark:border-gray-700">
            <p className="text-sm text-gray-500 dark:text-gray-400">Duration</p>
            <p className="text-2xl font-bold text-gray-900 dark:text-white">
              {(crawl.stats.duration_ms / 1000).toFixed(1)}s
            </p>
          </div>
        </div>
      )}

      <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">Findings</h2>
      <FindingList findings={findings} loading={false} />
    </div>
  );
}
