import { beforeEach, describe, expect, it, vi } from 'vitest';

const apiClientMock = vi.hoisted(() => ({
  login: vi.fn(),
  refreshToken: vi.fn(),
  setToken: vi.fn(),
}));

vi.mock('../services/api_client', () => ({
  apiClient: apiClientMock,
}));

vi.stubGlobal(
  'localStorage',
  (() => {
    const items = new Map<string, string>();
    return {
      getItem: (key: string) => items.get(key) ?? null,
      setItem: (key: string, value: string) => void items.set(key, value),
      removeItem: (key: string) => void items.delete(key),
      clear: () => items.clear(),
    };
  })(),
);

const { useAuth } = await import('./use_auth');

const resetState = () =>
  useAuth.setState({ token: null, user: null, isAuthenticated: false });

const storedUser = { id: 'u1', email: 'user@example.com', name: 'User', roles: ['viewer'] };

describe('useAuth store', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    resetState();
  });

  it('starts unauthenticated', () => {
    const state = useAuth.getState();

    expect(state.token).toBeNull();
    expect(state.user).toBeNull();
    expect(state.isAuthenticated).toBe(false);
  });

  it('login stores the token and authenticates on success', async () => {
    apiClientMock.login.mockResolvedValue({ token: 'jwt-1', user: storedUser });

    const result = await useAuth.getState().login('user@example.com', 'password');

    expect(result).toBe(true);
    expect(apiClientMock.login).toHaveBeenCalledWith('user@example.com', 'password');
    expect(apiClientMock.setToken).toHaveBeenCalledWith('jwt-1');
    expect(useAuth.getState().token).toBe('jwt-1');
    expect(useAuth.getState().isAuthenticated).toBe(true);
  });

  it('login returns false and leaves state untouched on failure', async () => {
    apiClientMock.login.mockRejectedValue(new Error('invalid credentials'));

    const result = await useAuth.getState().login('user@example.com', 'wrong');

    expect(result).toBe(false);
    expect(apiClientMock.setToken).not.toHaveBeenCalled();
    expect(useAuth.getState().token).toBeNull();
    expect(useAuth.getState().isAuthenticated).toBe(false);
  });

  it('logout clears the session', () => {
    useAuth.setState({ token: 'jwt-1', user: storedUser, isAuthenticated: true });

    useAuth.getState().logout();

    expect(useAuth.getState().token).toBeNull();
    expect(useAuth.getState().user).toBeNull();
    expect(useAuth.getState().isAuthenticated).toBe(false);
  });

  it('setToken authenticates and syncs the api client', () => {
    useAuth.getState().setToken('jwt-2');

    expect(apiClientMock.setToken).toHaveBeenCalledWith('jwt-2');
    expect(useAuth.getState().token).toBe('jwt-2');
    expect(useAuth.getState().isAuthenticated).toBe(true);
  });

  it('refreshAuth short-circuits when no token is present', async () => {
    const result = await useAuth.getState().refreshAuth();

    expect(result).toBe(false);
    expect(apiClientMock.refreshToken).not.toHaveBeenCalled();
  });

  it('refreshAuth rotates the token on success', async () => {
    useAuth.setState({ token: 'stale-jwt', isAuthenticated: true });
    apiClientMock.refreshToken.mockResolvedValue({ token: 'fresh-jwt', user: storedUser });

    const result = await useAuth.getState().refreshAuth();

    expect(result).toBe(true);
    expect(apiClientMock.setToken).toHaveBeenCalledWith('stale-jwt');
    expect(apiClientMock.setToken).toHaveBeenLastCalledWith('fresh-jwt');
    expect(useAuth.getState().token).toBe('fresh-jwt');
    expect(useAuth.getState().isAuthenticated).toBe(true);
  });

  it('refreshAuth clears the session on failure', async () => {
    useAuth.setState({ token: 'stale-jwt', user: storedUser, isAuthenticated: true });
    apiClientMock.refreshToken.mockRejectedValue(new Error('HTTP 401'));

    const result = await useAuth.getState().refreshAuth();

    expect(result).toBe(false);
    expect(useAuth.getState().token).toBeNull();
    expect(useAuth.getState().user).toBeNull();
    expect(useAuth.getState().isAuthenticated).toBe(false);
  });
});
