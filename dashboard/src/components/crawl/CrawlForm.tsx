import { useState } from 'react';
import Input from '../ui/Input';
import Button from '../ui/Button';

interface CrawlFormProps {
  onSubmit: (config: CrawlConfig) => void;
  loading?: boolean;
}

interface CrawlConfig {
  name: string;
  url: string;
  max_depth?: number;
  max_pages?: number;
  respect_robots?: boolean;
}

export default function CrawlForm({ onSubmit, loading }: CrawlFormProps) {
  const [name, setName] = useState('');
  const [url, setUrl] = useState('');
  const [maxDepth, setMaxDepth] = useState('10');
  const [maxPages, setMaxPages] = useState('1000');

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onSubmit({
      name,
      url,
      max_depth: parseInt(maxDepth, 10) || 10,
      max_pages: parseInt(maxPages, 10) || 1000,
      respect_robots: true,
    });
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-4">
      <Input
        label="Crawl Name"
        value={name}
        onChange={(e) => setName(e.target.value)}
        placeholder="My Crawl"
        required
      />
      <Input
        label="Target URL"
        value={url}
        onChange={(e) => setUrl(e.target.value)}
        placeholder="https://example.com"
        type="url"
        required
      />
      <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
        <Input
          label="Max Depth"
          value={maxDepth}
          onChange={(e) => setMaxDepth(e.target.value)}
          type="number"
          min="1"
          max="100"
        />
        <Input
          label="Max Pages"
          value={maxPages}
          onChange={(e) => setMaxPages(e.target.value)}
          type="number"
          min="1"
          max="100000"
        />
      </div>
      <div className="flex justify-end gap-3 pt-4">
        <Button type="submit" disabled={loading}>
          {loading ? 'Starting...' : 'Start Crawl'}
        </Button>
      </div>
    </form>
  );
}
