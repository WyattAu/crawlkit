import { CheckCircle2, Download, Star } from 'lucide-react';
import type { MarketplacePlugin } from '../../models/types';

export function StarRating({ rating }: { rating: number }) {
  const rounded = Math.round(rating);
  return (
    <div
      className="flex items-center gap-0.5"
      role="img"
      aria-label={`Rated ${rating.toFixed(1)} out of 5`}
    >
      {[1, 2, 3, 4, 5].map((i) => (
        <Star
          key={i}
          aria-hidden="true"
          className={`w-4 h-4 ${
            i <= rounded
              ? 'text-amber-400 fill-amber-400'
              : 'text-gray-300 dark:text-gray-600'
          }`}
        />
      ))}
    </div>
  );
}

export function formatDownloads(count: number): string {
  if (count >= 1_000_000) return `${(count / 1_000_000).toFixed(1)}M`;
  if (count >= 1_000) return `${(count / 1_000).toFixed(1)}k`;
  return String(count);
}

interface PluginCardProps {
  plugin: MarketplacePlugin;
  onClick: (plugin: MarketplacePlugin) => void;
}

export default function PluginCard({ plugin, onClick }: PluginCardProps) {
  return (
    <button
      type="button"
      onClick={() => onClick(plugin)}
      aria-label={`View details for ${plugin.name}`}
      className="text-left w-full bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-5 hover:border-blue-500 dark:hover:border-blue-500 hover:shadow-sm transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500"
    >
      <div className="flex items-start justify-between gap-2 mb-2">
        <div className="flex items-center gap-2 min-w-0">
          <h3 className="font-semibold text-gray-900 dark:text-white truncate">
            {plugin.name}
          </h3>
          {plugin.verified && (
            <CheckCircle2
              aria-label="Verified plugin"
              className="w-4 h-4 text-green-500 shrink-0"
            />
          )}
        </div>
        <span className="px-2 py-0.5 text-xs rounded-full bg-blue-50 dark:bg-blue-900/20 text-blue-600 dark:text-blue-400 shrink-0">
          v{plugin.version}
        </span>
      </div>

      <p className="text-xs text-gray-500 dark:text-gray-400 mb-3">
        by {plugin.author}
      </p>

      <p className="text-sm text-gray-600 dark:text-gray-300 line-clamp-2 mb-4">
        {plugin.description}
      </p>

      <div className="flex items-center gap-2 mb-3">
        <StarRating rating={plugin.rating} />
        <span className="text-xs text-gray-500 dark:text-gray-400">
          {plugin.rating.toFixed(1)} ({plugin.rating_count})
        </span>
      </div>

      <div className="flex items-center justify-between gap-2">
        <div className="flex flex-wrap gap-1">
          {plugin.categories.slice(0, 3).map((category) => (
            <span
              key={category}
              className="px-2 py-0.5 text-xs rounded-full bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-400"
            >
              {category}
            </span>
          ))}
        </div>
        <span className="flex items-center gap-1 text-xs text-gray-500 dark:text-gray-400 shrink-0">
          <Download className="w-3.5 h-3.5" aria-hidden="true" />
          {formatDownloads(plugin.downloads)}
        </span>
      </div>
    </button>
  );
}
