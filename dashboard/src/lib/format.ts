/**
 * Formatting helpers shared across marketplace components.
 * Extracted from PluginCard so component files only export components
 * (react-refresh/only-export-components).
 */

export function formatDownloads(count: number): string {
  if (count >= 1_000_000) return `${(count / 1_000_000).toFixed(1)}M`;
  if (count >= 1_000) return `${(count / 1_000).toFixed(1)}k`;
  return String(count);
}
