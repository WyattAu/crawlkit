import { useState } from 'react';
import { format } from 'date-fns';
import { CheckCircle2, Download, Loader, ShieldCheck } from 'lucide-react';
import Modal from '../ui/Modal';
import Button from '../ui/Button';
import { apiClient } from '../../services/api_client';
import type { MarketplacePlugin } from '../../models/types';
import { StarRating } from './PluginCard';
import { formatDownloads } from '../../lib/format';

interface PluginDetailProps {
  plugin: MarketplacePlugin | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  isAdmin: boolean;
  onUpdate: (plugin: MarketplacePlugin) => void;
}

export default function PluginDetail({
  plugin,
  open,
  onOpenChange,
  isAdmin,
  onUpdate,
}: PluginDetailProps) {
  const [installing, setInstalling] = useState(false);
  const [installed, setInstalled] = useState(false);
  const [verifying, setVerifying] = useState(false);
  const [actionError, setActionError] = useState('');

  if (!plugin) return null;

  async function handleInstall() {
    if (!plugin) return;
    setInstalling(true);
    setActionError('');
    try {
      const result = await apiClient.trackPluginDownload(plugin.name);
      onUpdate({ ...plugin, downloads: result.downloads });
      setInstalled(true);
    } catch (err) {
      setActionError(err instanceof Error ? err.message : 'Failed to install plugin');
    } finally {
      setInstalling(false);
    }
  }

  async function handleVerify() {
    if (!plugin) return;
    setVerifying(true);
    setActionError('');
    try {
      const updated = await apiClient.verifyMarketplacePlugin(plugin.name);
      onUpdate(updated);
    } catch (err) {
      setActionError(err instanceof Error ? err.message : 'Failed to verify plugin');
    } finally {
      setVerifying(false);
    }
  }

  const metadata = [
    { label: 'Author', value: plugin.author },
    { label: 'Version', value: `v${plugin.version}` },
    { label: 'License', value: plugin.license },
    { label: 'Downloads', value: formatDownloads(plugin.downloads) },
    { label: 'Updated', value: formatDate(plugin.updated_at) },
  ];

  return (
    <Modal
      open={open}
      onOpenChange={(next) => {
        if (!next) {
          setInstalled(false);
          setActionError('');
        }
        onOpenChange(next);
      }}
      title={plugin.name}
      className="max-w-2xl max-h-[85vh] overflow-y-auto"
    >
      <div className="flex items-center gap-2 mb-4">
        <span className="px-2 py-0.5 text-xs rounded-full bg-blue-50 dark:bg-blue-900/20 text-blue-600 dark:text-blue-400">
          v{plugin.version}
        </span>
        {plugin.verified && (
          <span className="flex items-center gap-1 px-2 py-0.5 text-xs rounded-full bg-green-50 dark:bg-green-900/20 text-green-600 dark:text-green-400">
            <CheckCircle2 className="w-3.5 h-3.5" aria-hidden="true" />
            Verified
          </span>
        )}
        <span className="text-xs text-gray-500 dark:text-gray-400">
          by {plugin.author}
        </span>
      </div>

      <p className="text-sm text-gray-600 dark:text-gray-300 mb-6 whitespace-pre-line">
        {plugin.description}
      </p>

      <dl className="grid grid-cols-2 sm:grid-cols-3 gap-4 mb-6">
        {metadata.map((item) => (
          <div key={item.label}>
            <dt className="text-xs font-medium text-gray-500 dark:text-gray-400">
              {item.label}
            </dt>
            <dd className="text-sm text-gray-900 dark:text-white mt-0.5">
              {item.value}
            </dd>
          </div>
        ))}
      </dl>

      <div className="mb-6">
        <h4 className="text-sm font-medium text-gray-900 dark:text-white mb-2">
          Categories
        </h4>
        <div className="flex flex-wrap gap-1">
          {plugin.categories.map((category) => (
            <span
              key={category}
              className="px-2 py-0.5 text-xs rounded-full bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-400"
            >
              {category}
            </span>
          ))}
        </div>
      </div>

      <div className="mb-6">
        <h4 className="text-sm font-medium text-gray-900 dark:text-white mb-2">
          Rating breakdown
        </h4>
        <div className="flex items-center gap-4">
          <span className="text-3xl font-bold text-gray-900 dark:text-white">
            {plugin.rating.toFixed(1)}
          </span>
          <div>
            <StarRating rating={plugin.rating} />
            <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
              {plugin.rating_count}{' '}
              {plugin.rating_count === 1 ? 'rating' : 'ratings'}
            </p>
          </div>
        </div>
        <div
          className="mt-3 h-2 rounded-full bg-gray-100 dark:bg-gray-700 overflow-hidden"
          role="img"
          aria-label={`Average rating ${plugin.rating.toFixed(1)} out of 5 from ${plugin.rating_count} ratings`}
        >
          <div
            className="h-full bg-amber-400"
            style={{ width: `${Math.min(100, (plugin.rating / 5) * 100)}%` }}
          />
        </div>
      </div>

      {plugin.changelog && plugin.changelog.length > 0 && (
        <div className="mb-6">
          <h4 className="text-sm font-medium text-gray-900 dark:text-white mb-2">
            Changelog
          </h4>
          <ul className="space-y-1 list-disc list-inside">
            {plugin.changelog.map((entry, index) => (
              <li
                key={index}
                className="text-sm text-gray-600 dark:text-gray-300"
              >
                {entry}
              </li>
            ))}
          </ul>
        </div>
      )}

      {actionError && (
        <div
          role="alert"
          className="mb-4 p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg text-sm text-red-600 dark:text-red-400"
        >
          {actionError}
        </div>
      )}

      <div className="flex justify-end gap-3 pt-2">
        {isAdmin && !plugin.verified && (
          <Button variant="secondary" onClick={handleVerify} disabled={verifying}>
            {verifying ? (
              <Loader className="w-4 h-4 animate-spin" aria-hidden="true" />
            ) : (
              <ShieldCheck className="w-4 h-4 mr-2" aria-hidden="true" />
            )}
            {verifying ? 'Verifying...' : 'Verify'}
          </Button>
        )}
        <Button onClick={handleInstall} disabled={installing || installed}>
          {installing ? (
            <Loader className="w-4 h-4 animate-spin" aria-hidden="true" />
          ) : (
            <Download className="w-4 h-4 mr-2" aria-hidden="true" />
          )}
          {installed ? 'Installed' : installing ? 'Installing...' : 'Install'}
        </Button>
      </div>
    </Modal>
  );
}

function formatDate(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return format(date, 'MMM d, yyyy');
}
