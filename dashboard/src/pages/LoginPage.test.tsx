/**
 * @vitest-environment jsdom
 */
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi, beforeEach } from 'vitest';
import { MemoryRouter } from 'react-router-dom';

const navigateMock = vi.fn();
vi.mock('react-router-dom', async (importOriginal) => {
  const actual = await importOriginal<typeof import('react-router-dom')>();
  return { ...actual, useNavigate: () => navigateMock };
});

const loginMock = vi.fn();
vi.mock('../hooks/use_auth', () => ({
  useAuth: () => ({ login: loginMock }),
}));

import LoginPage from './LoginPage';

const renderPage = () =>
  render(
    <MemoryRouter>
      <LoginPage />
    </MemoryRouter>,
  );

describe('LoginPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders the heading and sign-in form', () => {
    renderPage();

    expect(screen.getByRole('heading', { name: /crawlkit/i })).toBeDefined();
    expect(screen.getByText('Sign in to your account')).toBeDefined();
    expect(screen.getByLabelText(/email/i)).toBeDefined();
    expect(screen.getByLabelText(/password/i)).toBeDefined();
    expect(screen.getByRole('button', { name: /sign in/i })).toBeDefined();
  });

  it('shows error message when login fails', async () => {
    loginMock.mockResolvedValue(false);
    renderPage();

    await userEvent.type(screen.getByLabelText(/email/i), 'user@example.com');
    await userEvent.type(screen.getByLabelText(/password/i), 'wrong');
    await userEvent.click(screen.getByRole('button', { name: /sign in/i }));

    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeDefined();
    });
    expect(screen.getByRole('alert').textContent).toBe('Invalid credentials');
    expect(navigateMock).not.toHaveBeenCalled();
  });

  it('navigates to /dashboard on successful login', async () => {
    loginMock.mockResolvedValue(true);
    renderPage();

    await userEvent.type(screen.getByLabelText(/email/i), 'user@example.com');
    await userEvent.type(screen.getByLabelText(/password/i), 'secret');
    await userEvent.click(screen.getByRole('button', { name: /sign in/i }));

    await waitFor(() => {
      expect(navigateMock).toHaveBeenCalledWith('/dashboard');
    });
  });

  it('disables the button and shows loading text while submitting', async () => {
    let resolveLogin: (value: boolean) => void;
    loginMock.mockImplementation(() => new Promise<boolean>((r) => { resolveLogin = r; }));
    renderPage();

    await userEvent.type(screen.getByLabelText(/email/i), 'u@e.com');
    await userEvent.type(screen.getByLabelText(/password/i), 'p');
    await userEvent.click(screen.getByRole('button', { name: /sign in/i }));

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /signing in/i })).toBeDefined();
    });
    expect((screen.getByRole('button', { name: /signing in/i }) as HTMLButtonElement).disabled).toBe(true);

    resolveLogin!(true);
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /^sign in$/i })).toBeDefined();
    });
  });
});
