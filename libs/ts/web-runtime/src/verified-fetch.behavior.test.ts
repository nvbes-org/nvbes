import { afterEach, describe, expect, it, vi } from 'vite-plus/test';
import { z } from 'zod';
import { createVerifiedFetch, verifiedFetch, verifiedFetchJson } from './verified-fetch';

const sameOrigin = 'https://account.nvbes.test';
afterEach(() => vi.unstubAllGlobals());

describe('verified requests', () => {
  it('merges Request headers, preserves credentials and method, and removes private options', async () => {
    const fetchImpl = vi.fn<typeof fetch>().mockResolvedValue(new Response());
    const request = new Request(`${sameOrigin}/profile`, {
      method: 'PATCH',
      credentials: 'same-origin',
      headers: { 'X-Existing': 'keep', 'X-Override': 'old', 'X-CSRF-Token': 'explicit' },
    });
    await verifiedFetch(request, { sameOrigin, fetchImpl, headers: { 'X-Override': 'new' } });
    expect(fetchImpl).toHaveBeenCalledExactlyOnceWith(request, {
      method: 'PATCH',
      credentials: 'same-origin',
      headers: new Headers({
        'X-Existing': 'keep',
        'X-Override': 'new',
        'X-CSRF-Token': 'explicit',
        'Nvbes-Verified-Fetch': '1',
        'X-Requested-With': 'XMLHttpRequest',
      }),
    });
  });

  it.each(['POST', 'put', 'PATCH', 'DELETE'])('attaches decoded CSRF for %s', async (method) => {
    vi.stubGlobal('document', { cookie: 'csrf_token=synthetic%20token' });
    const fetchImpl = vi.fn<typeof fetch>().mockResolvedValue(new Response());
    await verifiedFetch('/profile', { sameOrigin, fetchImpl, method });
    const init = fetchImpl.mock.calls[0]?.[1];
    expect(init?.credentials).toBe('include');
    expect(new Headers(init?.headers).get('X-CSRF-Token')).toBe('synthetic token');
    expect(new Headers(init?.headers).get('X-Requested-With')).toBe('XMLHttpRequest');
  });

  it.each([
    { method: 'GET' },
    { method: 'HEAD' },
    { method: 'OPTIONS' },
    { method: 'POST', credentials: 'omit' as const },
    { method: 'POST', skipCsrf: true },
  ])('omits CSRF under $method / $credentials / $skipCsrf', async (options) => {
    vi.stubGlobal('document', { cookie: 'csrf_token=synthetic' });
    const fetchImpl = vi.fn<typeof fetch>().mockResolvedValue(new Response());
    await verifiedFetch('/profile', { sameOrigin, fetchImpl, ...options });
    const headers = new Headers(fetchImpl.mock.calls[0]?.[1]?.headers);
    expect(headers.has('X-CSRF-Token')).toBe(false);
    expect(headers.has('X-Requested-With')).toBe(options.method === 'POST');
  });

  it('honors factory policy, custom headers, cookies and per-call credentials', async () => {
    vi.stubGlobal('document', { cookie: 'custom=custom%20token' });
    const fetchImpl = vi.fn<typeof fetch>().mockResolvedValue(new Response());
    const client = createVerifiedFetch({
      sameOrigin,
      fetchImpl,
      credentials: 'omit',
      headerName: 'X-Verified',
      headerValue: 'test',
      csrfCookieName: 'custom',
      csrfHeaderName: 'X-Custom-CSRF',
    });
    await client(new URL(`${sameOrigin}/profile`), {
      method: 'POST',
      credentials: 'include',
      headers: { 'X-Requested-With': 'explicit' },
    });
    const init = fetchImpl.mock.calls[0]?.[1];
    expect(init?.credentials).toBe('include');
    expect(new Headers(init?.headers)).toEqual(
      new Headers({
        'X-Verified': 'test',
        'X-Custom-CSRF': 'custom token',
        'X-Requested-With': 'explicit',
      }),
    );
    await client('/profile');
    expect(fetchImpl.mock.calls[1]?.[1]?.credentials).toBe('omit');
  });

  it('uses browser origin and global fetch without requiring a Request constructor', async () => {
    vi.stubGlobal('location', { origin: sameOrigin });
    vi.stubGlobal('Request', undefined);
    const fetchImpl = vi.fn<typeof fetch>().mockResolvedValue(new Response());
    vi.stubGlobal('fetch', fetchImpl);
    await verifiedFetch('/profile');
    expect(fetchImpl.mock.calls[0]?.[1]?.method).toBe('GET');
    await expect(verifiedFetch('https://evil.test/profile')).rejects.toMatchObject({
      kind: 'cross-origin',
    });
    expect(fetchImpl).toHaveBeenCalledTimes(1);
  });

  it('checks exact origins including ports and allows explicit API origins', async () => {
    const fetchImpl = vi.fn<typeof fetch>().mockResolvedValue(new Response());
    for (const url of [
      'https://account.nvbes.test:444/profile',
      'https://account.nvbes.test.evil.test/profile',
    ]) {
      await expect(verifiedFetch(url, { sameOrigin, fetchImpl })).rejects.toMatchObject({
        kind: 'cross-origin',
        url,
      });
    }
    expect(fetchImpl).not.toHaveBeenCalled();
    await verifiedFetch('https://api.nvbes.test/profile', {
      sameOrigin,
      fetchImpl,
      allowedOrigins: ['https://api.nvbes.test/'],
    });
    expect(fetchImpl).toHaveBeenCalledTimes(1);
  });

  it('does not retry network failures', async () => {
    const failure = new TypeError('offline');
    const fetchImpl = vi.fn<typeof fetch>().mockRejectedValue(failure);
    await expect(verifiedFetch('/profile', { sameOrigin, fetchImpl, method: 'POST' })).rejects.toBe(
      failure,
    );
    expect(fetchImpl).toHaveBeenCalledTimes(1);
  });
});

describe('verified response validation', () => {
  it.each([
    null,
    false,
    3,
    'failure',
    {},
    { error: null },
    { error: 'bad' },
    { error: {} },
    { error: { message: null } },
    { error: { message: 42 } },
    { error: { message: { toString: 'bad' } } },
  ])('keeps a typed HTTP error for malformed error envelope %j', async (body) => {
    const response = new Response(JSON.stringify(body), { status: 403, statusText: 'Forbidden' });
    await expect(
      verifiedFetchJson('/profile', z.unknown(), { sameOrigin, fetchImpl: async () => response }),
    ).rejects.toMatchObject({
      name: 'VerifiedFetchError',
      kind: 'http',
      status: 403,
      message: '403 Forbidden',
      body,
    });
  });

  it('preserves a textual API error instead of validating it as a success DTO', async () => {
    const body = { error: { message: 'Denied' } };
    await expect(
      verifiedFetchJson('/profile', z.never(), {
        sameOrigin,
        fetchImpl: async () => new Response(JSON.stringify(body), { status: 409 }),
      }),
    ).rejects.toMatchObject({ kind: 'http', status: 409, message: 'Denied', body });
  });

  it.each([204, 200])('accepts an empty %i response only through its schema', async (status) => {
    const options = { sameOrigin, fetchImpl: async () => new Response(null, { status }) };
    await expect(verifiedFetchJson('/profile', z.undefined(), options)).resolves.toBeUndefined();
    await expect(
      verifiedFetchJson('/profile', z.object({ id: z.string() }), options),
    ).rejects.toMatchObject({ kind: 'dto', cause: expect.any(z.ZodError) });
  });

  it('validates non-JSON text and rejects malformed DTOs without discarding the body', async () => {
    await expect(
      verifiedFetchJson('/profile', z.string(), {
        sameOrigin,
        fetchImpl: async () => new Response('ready'),
      }),
    ).resolves.toBe('ready');
    await expect(
      verifiedFetchJson('/profile', z.object({ id: z.string() }), {
        sameOrigin,
        fetchImpl: async () => new Response('{"id":7}'),
      }),
    ).rejects.toMatchObject({ kind: 'dto', body: { id: 7 }, cause: expect.any(z.ZodError) });
  });
});
