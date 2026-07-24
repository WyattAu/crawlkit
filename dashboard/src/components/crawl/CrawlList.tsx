import CrawlCard from './CrawlCard';
import { Loader } from 'lucide-react';

interface Crawl {
  id: string;
  name: string;
  url: string;
  status: string;
  created_at: string;
}

interface CrawlListProps {
  crawls: Crawl[];
  loading: boolean;
}

export default function CrawlList({ crawls, loading }: CrawlListProps) {
  if (loading) {
    return (
      <div className="flex items-center justify-center py-12">
        <Loader className="w-8 h-8 text-blue-500 animate-spin" />
      </div>
    );
  }

  if (crawls.length === 0) {
    return (
      <div className="text-center py-12 text-gray-500 dark:text-gray-400">
        No crawls found. Start a new crawl to begin.
      </div>
    );
  }

  return (
    <div className="grid gap-4">
      {crawls.map((crawl) => (
        <CrawlCard key={crawl.id} {...crawl} />
      ))}
    </div>
  );
}
