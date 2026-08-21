/**
 * @vitest-environment jsdom
 */
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi, beforeEach } from 'vitest';
import { MemoryRouter } from 'react-router-dom';

vi.mock('react-router-dom', async (importOriginal) => {
  const actual = await importOriginal<typeof import('react-router-dom')>();
  return { ...actual, useNavigate: () => vi.fn() };
});

const startCrawlMock = vi.fn();
vi.mock('../hooks/use_crawls', () => ({
  useCrawls: () => ({
    crawls: [],
    loading: false,
    startCrawl: startCrawlMock,
  }),
}));

import CrawlsPage from './CrawlsPage';

const renderPage = () =>
  render(
    <MemoryRouter>
      <CrawlsPage />
    </MemoryRouter>,
  );

describe('CrawlsPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders the Crawls heading', () => {
    renderPage();
    expect(screen.getByRole('heading', { name: /crawls/i })).toBeDefined();
  });

  it('shows empty state when no crawls exist', () => {
    renderPage();
    expect(screen.getByText(/no crawls found/i)).toBeDefined();
  });

  it('renders the New Crawl button', () => {
    renderPage();
    expect(screen.getByRole('button', { name: /new crawl/i })).toBeDefined();
  });

  it('opens the modal when New Crawl is clicked', async () => {
    renderPage();
    await userEvent.click(screen.getByRole('button', { name: /new crawl/i }));
    expect(screen.getByRole('heading', { name: /start new crawl/i })).toBeDefined();
  });
});
