import { beforeEach, describe, expect, it, vi } from 'vitest';
import { apiClient } from './api_client';

const jsonResponse = (body: unknown) =>
  new Response(JSON.stringify(body), {
    status: 200,
    headers: { 'Content-Type': 'application/json' },
  });

interface EndpointCase {
  label: string;
  invoke: () => Promise<unknown>;
  method: string;
  path: string;
  body?: unknown;
}

describe('apiClient', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    apiClient.setToken('test-token');
  });

  it('sends typed login requests with bearer authentication on subsequent calls', async () => {
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ token: 'new-token', user: { id: 'u1' } }), {
          status: 200,
          headers: { 'Content-Type': 'application/json' },
        }),
      )
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ id: 'u1', email: 'user@example.com' }), {
          status: 200,
          headers: { 'Content-Type': 'application/json' },
        }),
      );
    vi.stubGlobal('fetch', fetchMock);

    const login = await apiClient.login('user@example.com', 'password');
    apiClient.setToken(login.token);
    await apiClient.getMe();

    expect(fetchMock).toHaveBeenCalledTimes(2);
    expect(fetchMock.mock.calls[0]?.[1]).toMatchObject({
      method: 'POST',
      body: JSON.stringify({ email: 'user@example.com', password: 'password' }),
    });
    expect(fetchMock.mock.calls[1]?.[1]).toMatchObject({
      method: 'GET',
      headers: expect.objectContaining({ Authorization: 'Bearer new-token' }),
    });
  });

  it('surfaces structured API error messages', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue(
        new Response(JSON.stringify({ message: 'Forbidden for this role' }), {
          status: 403,
          headers: { 'Content-Type': 'application/json' },
        }),
      ),
    );

    await expect(apiClient.listUsers()).rejects.toThrow('Forbidden for this role');
  });

  it('falls back to the HTTP status when the error body is not JSON', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue(new Response('upstream failure', { status: 502 })),
    );

    await expect(apiClient.health()).rejects.toThrow('HTTP 502');
  });

  it('maps non-2xx responses to an Error carrying the server message', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue(
        new Response(JSON.stringify({ message: 'crawl not found' }), {
          status: 404,
          headers: { 'Content-Type': 'application/json' },
        }),
      ),
    );

    const error = await apiClient.getCrawl('c1').catch((err: unknown) => err);

    expect(error).toBeInstanceOf(Error);
    expect((error as Error).message).toBe('crawl not found');
  });

  it('propagates network failures as distinct transport errors', async () => {
    vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new TypeError('fetch failed')));

    const error = await apiClient.listCrawls().catch((err: unknown) => err);

    expect(error).toBeInstanceOf(TypeError);
    expect((error as Error).message).toBe('fetch failed');
  });

  describe('auth header injection', () => {
    it('attaches the bearer token to requests when present', async () => {
      const fetchMock = vi.fn().mockImplementation(async () => jsonResponse([]));
      vi.stubGlobal('fetch', fetchMock);

      await apiClient.listCrawls();

      expect(fetchMock.mock.calls[0]?.[1]).toMatchObject({
        headers: expect.objectContaining({ Authorization: 'Bearer test-token' }),
      });
    });

    it('omits the Authorization header when no token is set', async () => {
      apiClient.setToken('');
      const fetchMock = vi.fn().mockImplementation(async () => jsonResponse([]));
      vi.stubGlobal('fetch', fetchMock);

      await apiClient.listCrawls();

      const headers = fetchMock.mock.calls[0]?.[1]?.headers as Record<string, string>;
      expect(headers['Authorization']).toBeUndefined();
    });

    it('sends the bearer token on metrics requests without a JSON content type', async () => {
      const fetchMock = vi.fn().mockResolvedValue(
        new Response('crawl_requests_total 5\n'),
      );
      vi.stubGlobal('fetch', fetchMock);

      const metrics = await apiClient.getMetrics();

      expect(metrics).toBe('crawl_requests_total 5\n');
      expect(fetchMock).toHaveBeenCalledWith('http://localhost:4000/metrics', {
        headers: { Authorization: 'Bearer test-token' },
      });
    });
  });

  describe('endpoint mapping', () => {
    const endpoints: EndpointCase[] = [
      {
        label: 'health',
        invoke: () => apiClient.health(),
        method: 'GET',
        path: '/health',
      },
      {
        label: 'startCrawl',
        invoke: () =>
          apiClient.startCrawl({ start_url: 'https://example.com', max_pages: 5 }),
        method: 'POST',
        path: '/api/v1/crawls',
        body: { start_url: 'https://example.com', max_pages: 5 },
      },
      {
        label: 'getCrawl',
        invoke: () => apiClient.getCrawl('c1'),
        method: 'GET',
        path: '/api/v1/crawls/c1',
      },
      {
        label: 'getCrawlStats',
        invoke: () => apiClient.getCrawlStats('c1'),
        method: 'GET',
        path: '/api/v1/crawls/c1/stats',
      },
      {
        label: 'getFindings',
        invoke: () => apiClient.getFindings('c1'),
        method: 'GET',
        path: '/api/v1/crawls/c1/findings',
      },
      {
        label: 'getBacklinks',
        invoke: () => apiClient.getBacklinks('c1'),
        method: 'GET',
        path: '/api/v1/crawls/c1/backlinks',
      },
      {
        label: 'listCrawls',
        invoke: () => apiClient.listCrawls(),
        method: 'GET',
        path: '/api/v1/crawls',
      },
      {
        label: 'listUsers',
        invoke: () => apiClient.listUsers(),
        method: 'GET',
        path: '/api/v1/users',
      },
      {
        label: 'createUser',
        invoke: () =>
          apiClient.createUser({
            email: 'user@example.com',
            name: 'User',
            password: 'secret',
            tenant_id: 't1',
            roles: ['viewer'],
          }),
        method: 'POST',
        path: '/api/v1/users',
        body: {
          email: 'user@example.com',
          name: 'User',
          password: 'secret',
          tenant_id: 't1',
          roles: ['viewer'],
        },
      },
      {
        label: 'deleteUser',
        invoke: () => apiClient.deleteUser('u1'),
        method: 'DELETE',
        path: '/api/v1/users/u1',
      },
      {
        label: 'listTenants',
        invoke: () => apiClient.listTenants(),
        method: 'GET',
        path: '/api/v1/tenants',
      },
      {
        label: 'createTenant',
        invoke: () => apiClient.createTenant({ name: 'Acme' }),
        method: 'POST',
        path: '/api/v1/tenants',
        body: { name: 'Acme' },
      },
      {
        label: 'login',
        invoke: () => apiClient.login('user@example.com', 'password'),
        method: 'POST',
        path: '/api/v1/auth/login',
        body: { email: 'user@example.com', password: 'password' },
      },
      {
        label: 'getMe',
        invoke: () => apiClient.getMe(),
        method: 'GET',
        path: '/api/v1/auth/me',
      },
      {
        label: 'refreshToken',
        invoke: () => apiClient.refreshToken(),
        method: 'POST',
        path: '/api/v1/auth/refresh',
      },
      {
        label: 'createApiKey',
        invoke: () => apiClient.createApiKey({ name: 'ci-key' }),
        method: 'POST',
        path: '/api/v1/keys',
        body: { name: 'ci-key' },
      },
      {
        label: 'listApiKeys',
        invoke: () => apiClient.listApiKeys(),
        method: 'GET',
        path: '/api/v1/keys',
      },
      {
        label: 'deleteApiKey',
        invoke: () => apiClient.deleteApiKey('kk'),
        method: 'DELETE',
        path: '/api/v1/keys/kk',
      },
      {
        label: 'createWebhook',
        invoke: () =>
          apiClient.createWebhook({
            url: 'https://hooks.example.com',
            events: ['crawl.completed'],
          }),
        method: 'POST',
        path: '/api/v1/webhooks',
        body: { url: 'https://hooks.example.com', events: ['crawl.completed'] },
      },
      {
        label: 'listWebhooks',
        invoke: () => apiClient.listWebhooks(),
        method: 'GET',
        path: '/api/v1/webhooks',
      },
      {
        label: 'deleteWebhook',
        invoke: () => apiClient.deleteWebhook('w1'),
        method: 'DELETE',
        path: '/api/v1/webhooks/w1',
      },
      {
        label: 'createSchedule',
        invoke: () =>
          apiClient.createSchedule({
            start_url: 'https://example.com',
            interval_secs: 3600,
          }),
        method: 'POST',
        path: '/api/v1/schedules',
        body: { start_url: 'https://example.com', interval_secs: 3600 },
      },
      {
        label: 'listSchedules',
        invoke: () => apiClient.listSchedules(),
        method: 'GET',
        path: '/api/v1/schedules',
      },
      {
        label: 'deleteSchedule',
        invoke: () => apiClient.deleteSchedule('s1'),
        method: 'DELETE',
        path: '/api/v1/schedules/s1',
      },
      {
        label: 'getAuditEvents',
        invoke: () => apiClient.getAuditEvents(),
        method: 'GET',
        path: '/api/v1/audit',
      },
    ];

    it.each(endpoints)('$label hits $method $path', async ({ invoke, method, path, body }) => {
      const fetchMock = vi.fn().mockImplementation(async () => jsonResponse({}));
      vi.stubGlobal('fetch', fetchMock);

      await invoke();

      expect(fetchMock).toHaveBeenCalledTimes(1);
      expect(fetchMock.mock.calls[0]?.[0]).toBe(`http://localhost:4000${path}`);
      expect(fetchMock.mock.calls[0]?.[1]).toMatchObject({
        method,
        ...(body ? { body: JSON.stringify(body) } : {}),
      });
      if (!body) {
        expect(fetchMock.mock.calls[0]?.[1]?.body).toBeUndefined();
      }
    });
  });
});
