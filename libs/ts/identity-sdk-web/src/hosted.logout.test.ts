import { afterEach, expect, it, vi } from 'vite-plus/test';
import { loadHostedLogoutContext } from './hosted.logout';

afterEach(() => vi.unstubAllGlobals());
it('loads a context only on the Identity origin with the required header', async () => {
  vi.stubGlobal('location', { origin: 'https://identity.example' });
  const proof = 'x'.repeat(43);
  const send = vi.fn<typeof fetch>(
    async () =>
      new Response(JSON.stringify({ session_csrf_token: proof }), {
        headers: { 'Content-Type': 'application/json' },
      }),
  );
  expect(
    await loadHostedLogoutContext({ baseUrl: 'https://identity.example', fetchImpl: send }),
  ).toBe(proof);
  const init = send.mock.calls[0]?.[1] as RequestInit | undefined;
  expect(init?.method).toBe('GET');
  expect(new Headers(init?.headers).get('X-Nvbes-Session-Context')).toBe('1');
  expect(init?.credentials).toBe('same-origin');
  expect(init?.cache).toBe('no-store');
  await expect(
    loadHostedLogoutContext({ baseUrl: 'https://attacker.example', fetchImpl: send }),
  ).rejects.toThrow();
  expect(send).toHaveBeenCalledTimes(1);
});
it.each([{}, { session_csrf_token: 'short' }, { session_csrf_token: 1 }])(
  'rejects malformed context %j',
  async (value) => {
    vi.stubGlobal('location', { origin: 'https://identity.example' });
    const fetchImpl: typeof fetch = async () =>
      new Response(JSON.stringify(value), { headers: { 'Content-Type': 'application/json' } });
    await expect(
      loadHostedLogoutContext({ baseUrl: 'https://identity.example', fetchImpl }),
    ).rejects.toThrow();
  },
);
