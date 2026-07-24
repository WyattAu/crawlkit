import { useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { useAuth } from '../hooks/use_auth';
import { useMetrics } from '../hooks/use_metrics';
import MetricCard from '../components/ui/Card';
import LineChart from '../components/charts/LineChart';
import BarChart from '../components/charts/BarChart';
import { Loader } from 'lucide-react';

export default function DashboardPage() {
  const { isAuthenticated } = useAuth();
  const navigate = useNavigate();
  const { metrics, loading } = useMetrics();

  useEffect(() => {
    if (!isAuthenticated) {
      navigate('/login');
    }
  }, [isAuthenticated, navigate]);

  if (loading) {
    return (
      <div className="flex items-center justify-center py-12">
        <Loader className="w-8 h-8 text-blue-500 animate-spin" />
      </div>
    );
  }

  const crawlStatusData = [
    { name: 'Active', value: metrics.activeCrawls },
    { name: 'Completed', value: metrics.completedCrawls },
    { name: 'Failed', value: metrics.failedCrawls },
  ];

  const overviewData = [
    { name: 'Crawls', value: metrics.totalCrawls },
    { name: 'Users', value: metrics.totalUsers },
    { name: 'Tenants', value: metrics.totalTenants },
  ];

  return (
    <div>
      <h1 className="text-2xl font-bold mb-6 text-gray-900 dark:text-white">Dashboard</h1>

      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mb-8">
        <MetricCard title="Total Crawls" value={metrics.totalCrawls} icon="spider" />
        <MetricCard title="Active" value={metrics.activeCrawls} icon="play" color="blue" />
        <MetricCard title="Completed" value={metrics.completedCrawls} icon="check" color="green" />
        <MetricCard title="Failed" value={metrics.failedCrawls} icon="x" color="red" />
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <BarChart data={crawlStatusData} title="Crawl Status Distribution" color="#3b82f6" />
        <LineChart data={overviewData} title="Overview Metrics" color="#22c55e" />
      </div>
    </div>
  );
}
