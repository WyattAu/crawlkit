/**
 * @vitest-environment jsdom
 */
import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import FindingList from './FindingList';
import type { Finding } from '../../models/types';

const makeFinding = (id: string): Finding => ({
  id,
  page_id: 'p1',
  category: 'accessibility',
  severity: 'high',
  code: 'WCAG-1.1.1',
  title: `Finding ${id}`,
  description: `Description for ${id}`,
  element: '<img />',
  recommendation: 'Add alt text',
});

const renderList = (findings: Finding[] = [], loading = false) =>
  render(<FindingList findings={findings} loading={loading} />);

describe('FindingList', () => {
  it('shows loading spinner when loading', () => {
    renderList([], true);
    expect(screen.getByRole('status')).toBeDefined();
    expect(screen.getByText('Loading...')).toBeDefined();
  });

  it('shows empty state when no findings', () => {
    renderList([], false);
    expect(screen.getByText(/no findings yet/i)).toBeDefined();
  });

  it('renders finding cards when data is present', () => {
    const findings = [makeFinding('f1'), makeFinding('f2')];
    renderList(findings);
    expect(screen.getByText('Finding f1')).toBeDefined();
    expect(screen.getByText('Finding f2')).toBeDefined();
  });
});
