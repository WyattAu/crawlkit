import { format } from 'date-fns';
import { AlertTriangle, Info, AlertCircle, Shield, ShieldAlert } from 'lucide-react';
import type { Finding } from '../../models/types';

const severityConfig: Record<string, { icon: typeof Info; color: string; bg: string }> = {
  info: { icon: Info, color: 'text-blue-700', bg: 'bg-blue-50 dark:bg-blue-900/20' },
  low: { icon: Info, color: 'text-green-700', bg: 'bg-green-50 dark:bg-green-900/20' },
  medium: { icon: AlertTriangle, color: 'text-amber-700', bg: 'bg-yellow-50 dark:bg-yellow-900/20' },
  high: { icon: AlertCircle, color: 'text-orange-500', bg: 'bg-orange-50 dark:bg-orange-900/20' },
  critical: { icon: ShieldAlert, color: 'text-red-700', bg: 'bg-red-50 dark:bg-red-900/20' },
};

export default function FindingCard({
  type,
  severity,
  title,
  description,
  url,
  evidence,
  created_at,
}: Finding) {
  const config = severityConfig[severity] || severityConfig.info;
  const SeverityIcon = config.icon;

  return (
    <div className="bg-white dark:bg-gray-800 rounded-xl p-4 shadow-sm border border-gray-200 dark:border-gray-700">
      <div className="flex items-start gap-3">
        <div className={`p-2 rounded-lg ${config.bg}`}>
          <Shield className={`w-5 h-5 ${config.color}`} />
        </div>
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            <h3 className="font-medium text-gray-900 dark:text-white">{title}</h3>
            <span className={`inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium ${config.bg} ${config.color}`}>
              <SeverityIcon className="w-3 h-3" />
              {severity}
            </span>
          </div>
          <p className="text-sm text-gray-600 dark:text-gray-400 mt-1">{description}</p>
          {evidence && (
            <pre className="mt-2 p-2 bg-gray-50 dark:bg-gray-900 rounded text-xs text-gray-700 dark:text-gray-300 overflow-x-auto">
              {evidence}
            </pre>
          )}
          <div className="mt-2 flex items-center gap-4 text-xs text-gray-400 dark:text-gray-500">
            <span>{type}</span>
            <span>{url}</span>
            <span>{format(new Date(created_at), 'MMM d, yyyy HH:mm')}</span>
          </div>
        </div>
      </div>
    </div>
  );
}
