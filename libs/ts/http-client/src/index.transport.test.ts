import { afterEach, expect, it, vi } from 'vite-plus/test';
import { z } from 'zod';
import { configureHttpRequestContextHeaders, createHttpClient } from './index';

afterEach(() => {
  vi.unstubAllGlobals();
  configureHttpRequestContextHeaders();
});

function transport(credentials?: RequestCredentials) {
  const calls: RequestInit[] = [];
  const fetchImpl: typeof fetch = async (_input, init = {}) => {
    calls.push(init);
    return new Response('{"ok":true}');
  };
  return { calls, client: createHttpClient({ baseUrl: 'https://a.test', credentials, fetchImpl }) };
}
const schema = z.object({ ok: z.literal(true) });

it('forwards GET and DELETE methods, cancellation and standard fetch options', async () => {
  const { calls, client } = transport();
  const signal = new AbortController().signal;
  await client.get('/item', schema, { signal, cache: 'no-store' });
  await client.delete('/item', schema, { redirect: 'error' });
  expect(calls[0]).toMatchObject({ method: 'GET', signal, cache: 'no-store' });
  expect(calls[1]).toMatchObject({ method: 'DELETE', redirect: 'error' });
  expect(new Headers(calls[0]?.headers).has('Content-Type')).toBe(false);
  expect(new Headers(calls[1]?.headers).get('X-Requested-With')).toBe('XMLHttpRequest');
});

it('request headers override provider headers without losing provider metadata', async () => {
  configureHttpRequestContextHeaders(() => ({ 'X-Provider': 'context', Accept: 'text/plain' }));
  const { calls, client } = transport();
  await client.get('/item', schema, { headers: { Accept: 'application/json' } });
  const headers = new Headers(calls[0]?.headers);
  expect(headers.get('X-Provider')).toBe('context');
  expect(headers.get('Accept')).toBe('application/json');
  expect(headers.has('X-Auth-User')).toBe(false);
  expect(headers.has('X-Requested-With')).toBe(false);
});

it('does not invoke the context provider for an external origin', async () => {
  const provider = vi.fn(() => ({ Authorization: 'synthetic-token' }));
  configureHttpRequestContextHeaders(provider);
  const { client, calls } = transport('include');
  await client.get('https://b.test/item', schema);
  expect(provider).not.toHaveBeenCalled();
  expect(calls[0]?.credentials).toBe('omit');
  expect(new Headers(calls[0]?.headers).has('Authorization')).toBe(false);
});

it('uses CSRF only for credential-bearing mutations and preserves explicit tokens', async () => {
  vi.stubGlobal('document', { cookie: 'csrf_token=cookie-token' });
  const { client, calls } = transport('include');
  await client.get('/item', schema);
  await client.post('/item', schema, {}, { headers: { 'X-CSRF-Token': 'explicit' } });
  expect(new Headers(calls[0]?.headers).has('X-CSRF-Token')).toBe(false);
  expect(new Headers(calls[1]?.headers).get('X-CSRF-Token')).toBe('explicit');
  const anonymous = transport();
  await anonymous.client.post('/item', schema, {});
  expect(new Headers(anonymous.calls[0]?.headers).has('X-CSRF-Token')).toBe(false);
  vi.stubGlobal('document', { cookie: '' });
  await client.post('/item', schema, {});
  expect(new Headers(calls[2]?.headers).has('X-CSRF-Token')).toBe(false);
});

it('preserves string payloads and removes SDK-only options from fetch init', async () => {
  const { client, calls } = transport();
  await client.post('/item', schema, 'raw text', {
    idempotencyKey: 'caller-key',
    requestE2ee: false,
  });
  expect(calls[0]?.body).toBe('raw text');
  expect(calls[0]).not.toHaveProperty('idempotencyKey');
  expect(calls[0]).not.toHaveProperty('requestE2ee');
  expect(new Headers(calls[0]?.headers).has('Content-Type')).toBe(false);
  expect(new Headers(calls[0]?.headers).get('Idempotency-Key')).toBe('caller-key');
});

it('passes binary bodies through when optional string-body encryption is configured', async () => {
  const { client, calls } = transport();
  const form = new FormData();
  form.set('name', 'synthetic');
  await client.post('/item', schema, form, {
    requestE2ee: { keyId: 'synthetic', secret: 'a'.repeat(32) },
  });
  expect(calls[0]?.body).toBe(form);
  expect(new Headers(calls[0]?.headers).has('X-Nvbes-E2ee')).toBe(false);
});
