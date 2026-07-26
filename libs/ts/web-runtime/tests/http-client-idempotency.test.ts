import { createHttpClient } from '@nvbes/http-client';
import { describe, expect, it } from 'vite-plus/test';
import { z } from 'zod';

describe('http-client idempotency headers', () => {
  it('preserves a base URL path prefix for absolute request paths', async () => {
    const observed = new ObservedRequest();
    const client = createHttpClient({
      baseUrl: 'http://localhost:5173/api',
      fetchImpl: buildJsonFetch(observed),
    });

    await client.get('/auth/me', z.object({ ok: z.boolean() }));

    expect(observed.url).toBe('http://localhost:5173/api/auth/me');
  });

  it('adds an Idempotency-Key to POST requests by default', async () => {
    const observed = new ObservedRequest();
    const client = createHttpClient({ fetchImpl: buildJsonFetch(observed) });

    await client.post('/mutations', z.object({ ok: z.boolean() }), {
      ok: true,
    });

    expect(observed.headers.get('Idempotency-Key')).toMatch(/^[0-9a-f-]+$/i);
  });

  it('marks mutating requests as XMLHttpRequest AJAX calls', async () => {
    const observed = new ObservedRequest();
    const client = createHttpClient({ fetchImpl: buildJsonFetch(observed) });

    await client.post('/mutations', z.object({ ok: z.boolean() }), {
      ok: true,
    });

    expect(observed.headers.get('X-Requested-With')).toBe('XMLHttpRequest');
  });

  it('does not mark safe reads as XMLHttpRequest AJAX calls', async () => {
    const observed = new ObservedRequest();
    const client = createHttpClient({ fetchImpl: buildJsonFetch(observed) });

    await client.get('/health', z.object({ ok: z.boolean() }));

    expect(observed.headers.get('X-Requested-With')).toBeNull();
  });

  it('preserves an explicit Idempotency-Key header', async () => {
    const observed = new ObservedRequest();
    const client = createHttpClient({ fetchImpl: buildJsonFetch(observed) });

    await client.request('/mutations', z.object({ ok: z.boolean() }), {
      body: { ok: true },
      headers: { 'Idempotency-Key': 'custom-key-123' },
      method: 'PATCH',
    });

    expect(observed.headers.get('Idempotency-Key')).toBe('custom-key-123');
  });

  it('adds an Idempotency-Key to PUT requests by default', async () => {
    const observed = new ObservedRequest();
    const client = createHttpClient({ fetchImpl: buildJsonFetch(observed) });

    await client.request('/mutations', z.object({ ok: z.boolean() }), {
      body: { ok: true },
      method: 'PUT',
    });

    expect(observed.headers.get('Idempotency-Key')).toMatch(/^[0-9a-f-]+$/i);
  });

  it('can disable automatic Idempotency-Key generation per request', async () => {
    const observed = new ObservedRequest();
    const client = createHttpClient({ fetchImpl: buildJsonFetch(observed) });

    await client.request('/mutations', z.object({ ok: z.boolean() }), {
      body: { ok: true },
      idempotencyKey: false,
      method: 'POST',
    });

    expect(observed.headers.get('Idempotency-Key')).toBeNull();
  });
});

class ObservedRequest {
  readonly headers = new Headers();
  url = '';
}

function buildJsonFetch(observed: ObservedRequest): typeof fetch {
  return async (input, init) => {
    observed.url =
      typeof input === 'string' ? input : input instanceof URL ? input.href : input.url;
    const requestHeaders = new Headers(init?.headers);
    requestHeaders.forEach((value, key) => {
      observed.headers.set(key, value);
    });

    return new Response(JSON.stringify({ ok: true }), {
      headers: { 'Content-Type': 'application/json' },
      status: 200,
    });
  };
}
