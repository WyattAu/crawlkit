import FindingCard from './FindingCard';
import { Loader } from 'lucide-react';

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

interface FindingListProps {
  findings: Finding[];
  loading: boolean;
}

export default function FindingList({ findings, loading }: FindingListProps) {
  if (loading) {
    return (
      <div role="status" className="flex items-center justify-center py-12">
        <Loader className="w-8 h-8 text-blue-500 animate-spin" />
      </div>
    );
  }

  if (findings.length === 0) {
    return (
      <div className="text-center py-12 text-gray-500 dark:text-gray-400">
        No findings yet.
      </div>
    );
  }

  return (
    <div className="space-y-3">
      {findings.map((finding) => (
        <FindingCard key={finding.id} {...finding} />
      ))}
    </div>
  );
}
