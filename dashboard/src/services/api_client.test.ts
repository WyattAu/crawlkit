import { beforeEach, describe, expect, it, vi } from 'vitest';
import { apiClient } from './api_client';

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
});
