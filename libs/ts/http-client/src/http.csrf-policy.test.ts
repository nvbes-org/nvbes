import { afterEach, describe, expect, it, vi } from 'vite-plus/test';
import { z } from 'zod';
import { createHttpClient } from './index';

afterEach(() => vi.unstubAllGlobals());
const schema = z.object({ ok: z.literal(true) });
const modes: (RequestCredentials | undefined)[] = [undefined, 'same-origin', 'include', 'omit'];

describe.each(modes)('automatic CSRF with credentials=%s', (credentials) => {
  it.each(['GET', 'HEAD', 'OPTIONS', 'POST', 'put', 'PATCH', 'DELETE'])(
    'matches cookie-bearing fetch semantics for %s',
    async (method) => {
      vi.stubGlobal('document', { cookie: 'csrf_token=synthetic' });
      const fetchImpl = vi.fn<typeof fetch>().mockResolvedValue(Response.json({ ok: true }));
      const client = createHttpClient({ baseUrl: 'https://account.test', credentials, fetchImpl });
      await client.request('/profile', schema, { method });
      const headers = new Headers(fetchImpl.mock.calls[0]?.[1]?.headers);
      const mutates = ['POST', 'put', 'PATCH', 'DELETE'].includes(method);
      expect(headers.get('X-CSRF-Token')).toBe(
        mutates && credentials !== 'omit' ? 'synthetic' : null,
      );
      expect(fetchImpl).toHaveBeenCalledTimes(1);
    },
  );
});

it('honors per-call credential overrides instead of the client default', async () => {
  vi.stubGlobal('document', { cookie: 'csrf_token=synthetic' });
  const fetchImpl = vi
    .fn<typeof fetch>()
    .mockImplementation(async () => Response.json({ ok: true }));
  const client = createHttpClient({
    baseUrl: 'https://account.test',
    credentials: 'include',
    fetchImpl,
  });
  await client.post('/profile', schema, {}, { credentials: 'omit' });
  await client.post('/profile', schema, {}, { credentials: 'same-origin' });
  expect(new Headers(fetchImpl.mock.calls[0]?.[1]?.headers).has('X-CSRF-Token')).toBe(false);
  expect(new Headers(fetchImpl.mock.calls[1]?.[1]?.headers).get('X-CSRF-Token')).toBe('synthetic');
});

it('does not read cookies when credential transmission is disabled', async () => {
  const readCookie = vi.fn(() => {
    throw new Error('cookie access denied');
  });
  vi.stubGlobal('document', {
    get cookie() {
      return readCookie();
    },
  });
  const fetchImpl = vi.fn<typeof fetch>().mockResolvedValue(Response.json({ ok: true }));
  const client = createHttpClient({
    baseUrl: 'https://account.test',
    credentials: 'omit',
    fetchImpl,
  });
  await expect(client.post('/profile', schema, {})).resolves.toEqual({ ok: true });
  expect(readCookie).not.toHaveBeenCalled();
});
