import { useEffect, useState } from 'react';
import { apiClient } from '../services/api_client';
import { useAuth } from '../hooks/use_auth';
import Button from '../components/ui/Button';
import Input from '../components/ui/Input';
import Modal from '../components/ui/Modal';
import { Plus, Loader, Server } from 'lucide-react';
import type { Tenant } from '../models/types';

export default function SettingsPage() {
  const { token } = useAuth();
  const [tenants, setTenants] = useState<Tenant[]>([]);
  const [loading, setLoading] = useState(true);
  const [showForm, setShowForm] = useState(false);
  const [formLoading, setFormLoading] = useState(false);
  const [tenantName, setTenantName] = useState('');
  const [healthStatus, setHealthStatus] = useState<'ok' | 'error' | 'loading'>('loading');
  const [formError, setFormError] = useState('');

  useEffect(() => {
    if (token) {
      apiClient.setToken(token);
      loadData();
    }
  }, [token]);

  async function loadData() {
    try {
      setLoading(true);
      const [tenantsData] = await Promise.all([
        apiClient.listTenants().catch(() => []),
        apiClient.health().then(() => setHealthStatus('ok')).catch(() => setHealthStatus('error')),
      ]);
      setTenants(tenantsData);
    } catch (error) {
      console.error('Failed to load settings:', error);
    } finally {
      setLoading(false);
    }
  }

  async function handleCreate(e: React.FormEvent) {
    e.preventDefault();
    setFormLoading(true);
    setFormError('');
    try {
      await apiClient.createTenant({ name: tenantName });
      setShowForm(false);
      setTenantName('');
      loadData();
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to create tenant';
      setFormError(message);
      console.error('Failed to create tenant:', err);
    } finally {
      setFormLoading(false);
    }
  }

  return (
    <div>
      <h1 className="text-2xl font-bold mb-6 text-gray-900 dark:text-white">Settings</h1>

      <div className="space-y-6">
        <div className="bg-white dark:bg-gray-800 rounded-xl p-6 border border-gray-200 dark:border-gray-700">
          <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">System Status</h2>
          <div className="flex items-center gap-3">
            <Server className="w-5 h-5 text-gray-500" />
            <span className="text-sm text-gray-600 dark:text-gray-400">API Server:</span>
            <span
              className={`px-2 py-0.5 text-xs rounded-full ${
                healthStatus === 'ok'
                  ? 'bg-green-100 text-green-700 dark:bg-green-900/20 dark:text-green-400'
                  : healthStatus === 'error'
                  ? 'bg-red-100 text-red-700 dark:bg-red-900/20 dark:text-red-400'
                  : 'bg-gray-100 text-gray-700 dark:bg-gray-700 dark:text-gray-400'
              }`}
            >
              {healthStatus === 'ok' ? 'Connected' : healthStatus === 'error' ? 'Disconnected' : 'Checking...'}
            </span>
          </div>
        </div>

        <div className="bg-white dark:bg-gray-800 rounded-xl p-6 border border-gray-200 dark:border-gray-700">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-lg font-semibold text-gray-900 dark:text-white">Tenants</h2>
            <Button size="sm" onClick={() => setShowForm(true)}>
              <Plus className="w-4 h-4 mr-1" />
              Add Tenant
            </Button>
          </div>

          {loading ? (
            <div role="status" className="flex items-center justify-center py-8">
              <Loader className="w-6 h-6 text-blue-500 animate-spin" />
              <span className="sr-only">Loading...</span>
            </div>
          ) : tenants.length === 0 ? (
            <p className="text-sm text-gray-500 dark:text-gray-400">No tenants configured.</p>
          ) : (
            <div className="space-y-2">
              {tenants.map((tenant) => (
                <div
                  key={tenant.id}
                  className="flex items-center justify-between p-3 bg-gray-50 dark:bg-gray-900 rounded-lg"
                >
                  <div>
                    <p className="font-medium text-gray-900 dark:text-white">{tenant.name}</p>
                    <p className="text-xs text-gray-500 dark:text-gray-400">{tenant.id}</p>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>

      <Modal open={showForm} onOpenChange={setShowForm} title="Add Tenant">
        <form onSubmit={handleCreate} className="space-y-4">
          {formError && (
            <div role="alert" className="p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg text-sm text-red-600 dark:text-red-400">
              {formError}
            </div>
          )}
          <Input
            label="Name"
            value={tenantName}
            onChange={(e) => setTenantName(e.target.value)}
            placeholder="My Organization"
            required
          />
          <div className="flex justify-end gap-3 pt-4">
            <Button type="submit" disabled={formLoading}>
              {formLoading ? 'Creating...' : 'Create Tenant'}
            </Button>
          </div>
        </form>
      </Modal>
    </div>
  );
}
