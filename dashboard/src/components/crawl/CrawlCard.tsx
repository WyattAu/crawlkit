import { memo } from 'react';
import { format } from 'date-fns';
import { ExternalLink, Clock, CheckCircle, XCircle, Loader } from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import type { CrawlResult } from '../../models/types';

const statusConfig: Record<string, { icon: typeof Clock; color: string; label: string }> = {
  pending: { icon: Clock, color: 'text-yellow-500', label: 'Pending' },
  running: { icon: Loader, color: 'text-blue-500', label: 'Running' },
  completed: { icon: CheckCircle, color: 'text-green-500', label: 'Completed' },
  failed: { icon: XCircle, color: 'text-red-500', label: 'Failed' },
};

export default memo(function CrawlCard({ crawl_id, start_url, status, created_at }: CrawlResult) {
  const navigate = useNavigate();
  const config = statusConfig[status] || statusConfig.pending;
  const StatusIcon = config.icon;

  return (
    <button
      className="w-full text-left bg-white dark:bg-gray-800 rounded-xl p-4 shadow-sm border border-gray-200 dark:border-gray-700 hover:shadow-md transition-shadow cursor-pointer"
      onClick={() => navigate(`/results/${crawl_id}`)}
    >
      <div className="flex items-start justify-between">
        <div className="flex-1 min-w-0">
          <h3 className="font-medium text-gray-900 dark:text-white truncate">{crawl_id}</h3>
          <p className="text-sm text-gray-500 dark:text-gray-400 truncate mt-1 flex items-center gap-1">
            <ExternalLink className="w-3 h-3" />
            {start_url}
          </p>
        </div>
        <div className={`flex items-center gap-1 ${config.color}`}>
          <StatusIcon className="w-4 h-4" />
          <span className="text-sm font-medium">{config.label}</span>
        </div>
      </div>
      <div className="mt-3 text-xs text-gray-400 dark:text-gray-500">
        {format(new Date(created_at), 'MMM d, yyyy HH:mm')}
      </div>
    </button>
  );
})
