import { afterEach, expect, it, vi } from 'vite-plus/test';
import { accountTransport } from './account.transport';

afterEach(() => vi.unstubAllGlobals());

it('refuses other origins before sending any request', async () => {
  const send = vi.fn();
  vi.stubGlobal('fetch', send);
  const transport = accountTransport(['https://identity.example'], new AbortController().signal);
  for (const url of [
    'https://evil.example/oauth/token',
    'https://identity.example.evil/oauth/token',
    'https://user@identity.example/oauth/token',
    'https://identity.example/oauth/token#secret',
  ])
    await expect(transport(url)).rejects.toThrow();
  expect(send).not.toHaveBeenCalled();
});

it('keeps protocol headers, forbids redirects and omits cookies', async () => {
  const send = vi.fn(async () => new Response('{}'));
  vi.stubGlobal('fetch', send);
  const signal = new AbortController().signal;
  const transport = accountTransport(['https://identity.example'], signal);
  await transport('https://identity.example/oauth/token', {
    method: 'POST',
    headers: { DPoP: 'proof' },
    credentials: 'include',
    redirect: 'follow',
    cache: 'force-cache',
  });
  expect(send).toHaveBeenCalledWith(
    'https://identity.example/oauth/token',
    expect.objectContaining({
      method: 'POST',
      headers: { DPoP: 'proof' },
      credentials: 'omit',
      redirect: 'error',
      cache: 'no-store',
    }),
  );
});
