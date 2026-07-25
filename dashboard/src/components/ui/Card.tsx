import { LucideIcon, Bug, Play, Check, X } from 'lucide-react';

interface MetricCardProps {
  title: string;
  value: number;
  icon: string;
  color?: string;
}

const iconMap: Record<string, LucideIcon> = {
  spider: Bug,
  play: Play,
  check: Check,
  x: X,
};

export default function MetricCard({ title, value, icon, color }: MetricCardProps) {
  const Icon = iconMap[icon] || Bug;
  const colorClasses: Record<string, string> = {
    blue: 'text-blue-500',
    green: 'text-green-500',
    red: 'text-red-500',
  };

  return (
    <div className="bg-white dark:bg-gray-800 rounded-xl p-6 shadow-sm border border-gray-200 dark:border-gray-700">
      <div className="flex items-center justify-between">
        <div>
          <p className="text-sm text-gray-500 dark:text-gray-400">{title}</p>
          <p className="text-3xl font-bold text-gray-900 dark:text-white mt-1">
            {value}
          </p>
        </div>
        <div className={`p-3 rounded-lg bg-gray-100 dark:bg-gray-700 ${colorClasses[color || ''] || 'text-gray-500'}`}>
          <Icon className="w-6 h-6" />
        </div>
      </div>
    </div>
  );
}
