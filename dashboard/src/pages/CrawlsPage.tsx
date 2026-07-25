import { useState } from 'react';
import { useCrawls } from '../hooks/use_crawls';
import CrawlList from '../components/crawl/CrawlList';
import CrawlForm from '../components/crawl/CrawlForm';
import Button from '../components/ui/Button';
import Modal from '../components/ui/Modal';
import { Plus } from 'lucide-react';

export default function CrawlsPage() {
  const { crawls, loading, startCrawl } = useCrawls();
  const [showForm, setShowForm] = useState(false);
  const [starting, setStarting] = useState(false);
  const [error, setError] = useState('');

  const handleStartCrawl = async (config: Record<string, unknown>) => {
    setStarting(true);
    setError('');
    try {
      await startCrawl(config);
      setShowForm(false);
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to start crawl';
      setError(message);
      console.error('Failed to start crawl:', err);
    } finally {
      setStarting(false);
    }
  };

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold text-gray-900 dark:text-white">Crawls</h1>
        <Button onClick={() => setShowForm(true)}>
          <Plus className="w-4 h-4 mr-2" />
          New Crawl
        </Button>
      </div>

      <CrawlList crawls={crawls} loading={loading} />

      {error && (
        <div role="alert" className="mt-4 p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg text-sm text-red-600 dark:text-red-400">
          {error}
        </div>
      )}

      <Modal open={showForm} onOpenChange={setShowForm} title="Start New Crawl">
        <CrawlForm onSubmit={handleStartCrawl} loading={starting} />
      </Modal>
    </div>
  );
}
