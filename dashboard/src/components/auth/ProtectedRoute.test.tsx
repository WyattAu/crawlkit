/**
 * @vitest-environment jsdom
 */
import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

const isAuthenticatedMock = vi.hoisted(() => vi.fn(() => false));
vi.mock('../../hooks/use_auth', () => ({
  useAuth: () => ({ isAuthenticated: isAuthenticatedMock() }),
}));

let capturedNavigate: ((to: string) => void) | null = null;

vi.mock('react-router-dom', async (importOriginal) => {
  const actual = await importOriginal<typeof import('react-router-dom')>();
  return {
    ...actual,
    Navigate: ({ to }: { to: string }) => {
      if (capturedNavigate) capturedNavigate(to);
      return null;
    },
    useLocation: () => ({ pathname: '/protected', search: '', hash: '', state: null, key: 'default' }),
  };
});

import ProtectedRoute from './ProtectedRoute';

describe('ProtectedRoute', () => {
  it('renders children when authenticated', () => {
    isAuthenticatedMock.mockReturnValue(true);
    render(
      <div>
        <ProtectedRoute>
          <div>Secret content</div>
        </ProtectedRoute>
      </div>,
    );
    expect(screen.getByText('Secret content')).toBeDefined();
  });

  it('redirects to /login when not authenticated', () => {
    isAuthenticatedMock.mockReturnValue(false);
    const navigateTo = vi.fn();
    capturedNavigate = navigateTo;
    render(
      <div>
        <ProtectedRoute>
          <div>Secret content</div>
        </ProtectedRoute>
      </div>,
    );
    expect(screen.queryByText('Secret content')).toBeNull();
    expect(navigateTo).toHaveBeenCalledWith('/login');
    capturedNavigate = null;
  });
});
