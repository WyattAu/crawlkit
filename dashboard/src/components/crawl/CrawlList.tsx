import { useState } from 'react';
import CrawlCard from './CrawlCard';
import { Loader, ChevronLeft, ChevronRight } from 'lucide-react';
import type { CrawlResult } from '../../models/types';

const PAGE_SIZE = 20;

interface CrawlListProps {
  crawls: CrawlResult[];
  loading: boolean;
}

export default function CrawlList({ crawls, loading }: CrawlListProps) {
  const [page, setPage] = useState(0);
  const totalPages = Math.ceil(crawls.length / PAGE_SIZE);
  const start = page * PAGE_SIZE;
  const visible = crawls.slice(start, start + PAGE_SIZE);

  if (loading) {
    return (
      <div role="status" className="flex items-center justify-center py-12">
        <Loader className="w-8 h-8 text-blue-500 animate-spin" />
        <span className="sr-only">Loading...</span>
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
    <div>
      <div className="grid gap-4">
        {visible.map((crawl) => (
          <CrawlCard key={crawl.crawl_id} {...crawl} />
        ))}
      </div>
      {totalPages > 1 && (
        <nav className="flex items-center justify-between mt-6 text-sm text-gray-600 dark:text-gray-400" aria-label="Pagination">
          <button
            onClick={() => setPage((p) => Math.max(0, p - 1))}
            disabled={page === 0}
            className="flex items-center gap-1 px-3 py-1.5 rounded-lg border border-gray-200 dark:border-gray-700 disabled:opacity-40 disabled:cursor-not-allowed hover:bg-gray-50 dark:hover:bg-gray-800"
          >
            <ChevronLeft className="w-4 h-4" />
            Previous
          </button>
          <span>
            Page {page + 1} of {totalPages}
          </span>
          <button
            onClick={() => setPage((p) => Math.min(totalPages - 1, p + 1))}
            disabled={page >= totalPages - 1}
            className="flex items-center gap-1 px-3 py-1.5 rounded-lg border border-gray-200 dark:border-gray-700 disabled:opacity-40 disabled:cursor-not-allowed hover:bg-gray-50 dark:hover:bg-gray-800"
          >
            Next
            <ChevronRight className="w-4 h-4" />
          </button>
        </nav>
      )}
    </div>
  );
}
