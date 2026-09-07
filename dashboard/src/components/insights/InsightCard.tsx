import { useState } from 'react';
import {
  ChevronDown,
  FileText,
  Globe,
  Lightbulb,
  Search,
  Shield,
  Wrench,
  Zap,
  type LucideIcon,
} from 'lucide-react';
import type {
  Insight,
  InsightCategory,
  InsightEffort,
  InsightPriority,
} from '../../models/types';

const categoryIcons: Record<InsightCategory, LucideIcon> = {
  technical: Wrench,
  content: FileText,
  seo: Search,
  security: Shield,
  performance: Zap,
};

const priorityClasses: Record<InsightPriority, string> = {
  critical: 'bg-red-100 text-red-800 dark:bg-red-900/40 dark:text-red-300',
  high: 'bg-orange-100 text-orange-800 dark:bg-orange-900/40 dark:text-orange-300',
  medium: 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900/40 dark:text-yellow-300',
  low: 'bg-blue-100 text-blue-800 dark:bg-blue-900/40 dark:text-blue-300',
};

const effortClasses: Record<InsightEffort, string> = {
  quick: 'bg-green-100 text-green-800 dark:bg-green-900/40 dark:text-green-300',
  moderate: 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900/40 dark:text-yellow-300',
  significant: 'bg-red-100 text-red-800 dark:bg-red-900/40 dark:text-red-300',
};

const impactBarClasses: Record<InsightPriority, string> = {
  critical: 'bg-red-500',
  high: 'bg-orange-500',
  medium: 'bg-yellow-500',
  low: 'bg-blue-500',
};

interface InsightCardProps {
  insight: Insight;
}

export default function InsightCard({ insight }: InsightCardProps) {
  const [expanded, setExpanded] = useState(false);
  const CategoryIcon = categoryIcons[insight.category] || Lightbulb;
  const impact = Math.max(0, Math.min(100, Math.round(insight.impact_score)));
  const panelId = `insight-panel-${insight.finding_codes.join('-').toLowerCase()}`;

  return (
    <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 shadow-sm overflow-hidden">
      <button
        type="button"
        onClick={() => setExpanded((prev) => !prev)}
        aria-expanded={expanded}
        aria-controls={panelId}
        className="w-full text-left p-4 sm:p-5 hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors"
      >
        <div className="flex items-start gap-3">
          <div className="p-2 rounded-lg bg-gray-100 dark:bg-gray-700 shrink-0">
            <CategoryIcon className="w-5 h-5 text-gray-600 dark:text-gray-300" />
          </div>

          <div className="flex-1 min-w-0">
            <div className="flex flex-wrap items-center gap-2 mb-1">
              <span
                className={`px-2 py-0.5 rounded-full text-xs font-semibold uppercase tracking-wide ${priorityClasses[insight.priority]}`}
              >
                {insight.priority}
              </span>
              <span
                className={`px-2 py-0.5 rounded-full text-xs font-medium ${effortClasses[insight.effort]}`}
              >
                {insight.effort} fix
              </span>
              <span className="inline-flex items-center gap-1 text-xs text-gray-500 dark:text-gray-400">
                <Globe className="w-3.5 h-3.5" />
                {insight.affected_pages}{' '}
                {insight.affected_pages === 1 ? 'page' : 'pages'} affected
              </span>
            </div>

            <h3 className="font-semibold text-gray-900 dark:text-white truncate">
              {insight.title}
            </h3>

            <div className="flex items-center gap-2 mt-2">
              <div className="flex-1 h-1.5 bg-gray-200 dark:bg-gray-700 rounded-full overflow-hidden">
                <div
                  className={`h-full rounded-full transition-all duration-500 ${impactBarClasses[insight.priority]}`}
                  style={{ width: `${impact}%` }}
                />
              </div>
              <span className="text-xs font-medium text-gray-500 dark:text-gray-400 tabular-nums w-14 text-right">
                Impact {impact}
              </span>
            </div>
          </div>

          <ChevronDown
            className={`w-5 h-5 text-gray-400 shrink-0 transition-transform duration-200 ${
              expanded ? 'rotate-180' : ''
            }`}
          />
        </div>
      </button>

      {expanded && (
        <div
          id={panelId}
          className="px-4 sm:px-5 pb-4 sm:pb-5 pt-1 border-t border-gray-100 dark:border-gray-700"
        >
          <p className="text-sm text-gray-600 dark:text-gray-300 mt-3">
            {insight.description}
          </p>
          <div className="mt-3 p-3 rounded-lg bg-blue-50 dark:bg-blue-900/20 border border-blue-100 dark:border-blue-800">
            <p className="text-sm font-medium text-blue-800 dark:text-blue-300 mb-1 flex items-center gap-1.5">
              <Lightbulb className="w-4 h-4" />
              Recommendation
            </p>
            <p className="text-sm text-blue-700 dark:text-blue-200">
              {insight.recommendation}
            </p>
          </div>
          <div className="flex flex-wrap gap-1.5 mt-3">
            {insight.finding_codes.map((code) => (
              <span
                key={code}
                className="px-2 py-0.5 rounded bg-gray-100 dark:bg-gray-700 text-xs font-mono text-gray-600 dark:text-gray-300"
              >
                {code}
              </span>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
