import { afterEach, expect, it, vi } from 'vite-plus/test';
import { prepareHostedRpLogout, confirmHostedRpLogout } from '../hosted.logout';

afterEach(() => vi.unstubAllGlobals());
function config(value: unknown, status = 200) {
  vi.stubGlobal('location', { origin: 'https://identity.example' });
  const send = vi.fn<typeof fetch>(
    async () =>
      new Response(JSON.stringify(value), {
        status,
        headers: { 'Content-Type': 'application/json' },
      }),
  );
  return { baseUrl: 'https://identity.example', fetchImpl: send };
}
it('binds preparation to the Identity origin and CSRF without following a redirect', async () => {
  const options = config({ prepared: true });
  await prepareHostedRpLogout(options, 'proof', 'ticket');
  const [url, init] = options.fetchImpl.mock.calls[0]!;
  expect(url).toBe('https://identity.example/oauth/logout/prepare');
  expect(init?.method).toBe('POST');
  expect(init?.body).toBe(JSON.stringify({ request: 'ticket' }));
  expect(new Headers(init?.headers).get('x-csrf-token')).toBe('proof');
  expect(init?.credentials).toBe('same-origin');
  expect(init?.redirect).toBe('error');
  await expect(
    prepareHostedRpLogout({ ...options, baseUrl: 'https://attacker.example' }, 'proof', 'ticket'),
  ).rejects.toThrow();
  expect(options.fetchImpl).toHaveBeenCalledTimes(1);
});
it.each([
  { logged_out: false, redirect_uri: 'https://account.example/' },
  { logged_out: true, redirect_uri: 'javascript:alert(1)' },
  { logged_out: true, redirect_uri: 'https://user@account.example/' },
  { logged_out: true, redirect_uri: 42 },
])('never navigates on an invalid confirmation %j', async (value) => {
  await expect(confirmHostedRpLogout(config(value), 'proof', 'ticket')).rejects.toThrow();
});
it('returns only the acknowledged destination and never retries an uncertain mutation', async () => {
  expect(
    await confirmHostedRpLogout(
      config({ logged_out: true, redirect_uri: 'https://account.example/?state=123' }),
      'proof',
      'ticket',
    ),
  ).toBe('https://account.example/?state=123');
  const failed = config({ error: 'unavailable' }, 503);
  await expect(confirmHostedRpLogout(failed, 'proof', 'ticket')).rejects.toThrow();
  expect(failed.fetchImpl).toHaveBeenCalledTimes(1);
});
