/**
 * @vitest-environment jsdom
 */
import { render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi, beforeEach } from 'vitest';
import { MemoryRouter, Route, Routes } from 'react-router-dom';

const tokenMock = vi.hoisted(() => vi.fn(() => 'test-token'));
vi.mock('../hooks/use_auth', () => ({
  useAuth: () => ({ token: tokenMock() }),
}));

const getCrawlMock = vi.hoisted(() => vi.fn());
const getCrawlStatsMock = vi.hoisted(() => vi.fn());
const getFindingsMock = vi.hoisted(() => vi.fn());
vi.mock('../services/api_client', () => ({
  apiClient: {
    setToken: vi.fn(),
    getCrawl: (...args: unknown[]) => getCrawlMock(...args),
    getCrawlStats: (...args: unknown[]) => getCrawlStatsMock(...args).catch(() => null),
    getFindings: (...args: unknown[]) => getFindingsMock(...args).catch(() => []),
  },
}));

import ResultsPage from './ResultsPage';

const renderPage = (id = 'crawl-1') =>
  render(
    <MemoryRouter initialEntries={[`/crawls/${id}`]}>
      <Routes>
        <Route path="/crawls/:id" element={<ResultsPage />} />
      </Routes>
    </MemoryRouter>,
  );

describe('ResultsPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('shows loading state initially', () => {
    getCrawlMock.mockReturnValue(new Promise(() => {}));
    getCrawlStatsMock.mockReturnValue(new Promise(() => {}));
    getFindingsMock.mockReturnValue(new Promise(() => {}));
    renderPage();
    expect(screen.getByRole('status')).toBeDefined();
    expect(screen.getByText('Loading...')).toBeDefined();
  });

  it('shows "Crawl not found" when the crawl request fails', async () => {
    getCrawlMock.mockRejectedValue(new Error('not found'));
    getCrawlStatsMock.mockRejectedValue(new Error('no stats'));
    getFindingsMock.mockRejectedValue(new Error('no findings'));
    renderPage();
    await waitFor(() => {
      expect(screen.getByText('Crawl not found.')).toBeDefined();
    });
  });

  it('renders crawl details after successful load', async () => {
    getCrawlMock.mockResolvedValue({
      crawl_id: 'crawl-1',
      start_url: 'https://example.com',
      status: 'completed',
      pages_crawled: 5,
      issues_found: 2,
    });
    getCrawlStatsMock.mockResolvedValue({
      total_pages: 5,
      total_issues: 2,
    });
    getFindingsMock.mockResolvedValue([]);

    renderPage();

    await waitFor(() => {
      expect(screen.getByText('crawl-1')).toBeDefined();
    });
    expect(screen.getByText('https://example.com')).toBeDefined();
    expect(screen.getAllByText('5').length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText('2').length).toBeGreaterThanOrEqual(1);
  });

  it('shows empty findings message when no findings exist', async () => {
    getCrawlMock.mockResolvedValue({
      crawl_id: 'crawl-2',
      start_url: 'https://test.com',
      status: 'completed',
      pages_crawled: 1,
      issues_found: 0,
    });
    getCrawlStatsMock.mockResolvedValue({
      total_pages: 1,
      total_issues: 0,
    });
    getFindingsMock.mockResolvedValue([]);

    renderPage('crawl-2');

    await waitFor(() => {
      expect(screen.getByText('crawl-2')).toBeDefined();
    });
    expect(screen.getByText(/no findings yet/i)).toBeDefined();
  });
});
