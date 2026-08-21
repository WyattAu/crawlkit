/**
 * @vitest-environment jsdom
 */
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { MemoryRouter } from 'react-router-dom';
import CrawlList from './CrawlList';
import type { CrawlResult } from '../../models/types';

const makeCrawl = (id: string): CrawlResult => ({
  crawl_id: id,
  start_url: `https://${id}.example.com`,
  status: 'completed',
  pages_crawled: 10,
  issues_found: 1,
  created_at: '2024-01-01T00:00:00Z',
  completed_at: '2024-01-01T01:00:00Z',
});

const renderList = (crawls: CrawlResult[] = [], loading = false) =>
  render(
    <MemoryRouter>
      <CrawlList crawls={crawls} loading={loading} />
    </MemoryRouter>,
  );

describe('CrawlList', () => {
  it('shows loading spinner when loading', () => {
    renderList([], true);
    expect(screen.getByRole('status')).toBeDefined();
    expect(screen.getByText('Loading...')).toBeDefined();
  });

  it('shows empty state when no crawls', () => {
    renderList([], false);
    expect(screen.getByText(/no crawls found/i)).toBeDefined();
  });

  it('renders crawl cards when data is present', () => {
    const crawls = [makeCrawl('c1'), makeCrawl('c2')];
    renderList(crawls);
    expect(screen.getByText('c1')).toBeDefined();
    expect(screen.getByText('c2')).toBeDefined();
  });

  it('does not show pagination when there are fewer than 20 items', () => {
    renderList([makeCrawl('c1')]);
    expect(screen.queryByLabelText(/pagination/i)).toBeNull();
  });

  it('shows pagination when there are more than 20 items', () => {
    const crawls = Array.from({ length: 25 }, (_, i) => makeCrawl(`c${i}`));
    renderList(crawls);
    expect(screen.getByLabelText('Pagination')).toBeDefined();
    expect(screen.getByText('Page 1 of 2')).toBeDefined();
  });

  it('navigates to next page when Next is clicked', async () => {
    const crawls = Array.from({ length: 25 }, (_, i) => makeCrawl(`c${i}`));
    renderList(crawls);
    await userEvent.click(screen.getByRole('button', { name: /next/i }));
    expect(screen.getByText('Page 2 of 2')).toBeDefined();
  });
});
