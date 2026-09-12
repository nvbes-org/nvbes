import { afterEach, describe, expect, it } from 'vite-plus/test';
import { z } from 'zod';
import { configureHttpRequestContextHeaders, createHttpClient } from './index';

const SuccessSchema = z.object({ ok: z.literal(true) });

type RecordedFetch = {
  input: RequestInfo | URL;
  init: RequestInit | undefined;
};

function createFetchRecorder(): {
  calls: RecordedFetch[];
  fetchImpl: typeof fetch;
} {
  const calls: RecordedFetch[] = [];
  const fetchImpl: typeof fetch = async (input, init) => {
    calls.push({ input, init });
    return new Response(JSON.stringify({ ok: true }), {
      headers: { 'Content-Type': 'application/json' },
      status: 200,
    });
  };

  return { calls, fetchImpl };
}

function installBrowserContext({
  cookie = '',
  pathname,
  search = '',
}: {
  cookie?: string;
  pathname: string;
  search?: string;
}): void {
  Object.defineProperty(globalThis, 'window', {
    configurable: true,
    value: {
      location: { pathname, search },
    },
  });
  Object.defineProperty(globalThis, 'document', {
    configurable: true,
    value: { cookie },
  });
}

function readFetchCall(calls: RecordedFetch[]): {
  headers: Headers;
  init: RequestInit;
  url: URL;
} {
  expect(calls).toHaveLength(1);
  const call = calls[0];
  if (!call) {
    throw new Error('Expected one fetch call');
  }

  const rawUrl =
    typeof call.input === 'string'
      ? call.input
      : call.input instanceof URL
        ? call.input.toString()
        : call.input.url;
  const init = call.init ?? {};
  return {
    headers: new Headers(init.headers),
    init,
    url: new URL(rawUrl),
  };
}

afterEach(() => {
  configureHttpRequestContextHeaders();
  Reflect.deleteProperty(globalThis, 'window');
  Reflect.deleteProperty(globalThis, 'document');
});

describe('HttpClient origin isolation', () => {
  it('strips account secrets and credentials from an external absolute URL', async () => {
    installBrowserContext({
      cookie: 'csrf_token_7=browser-csrf',
      pathname: '/account/7/preferences',
    });
    configureHttpRequestContextHeaders(() => ({
      'X-Auth-User': 'context-account',
      'X-CSRF-Token': 'context-csrf',
      'Idempotency-Key': 'context-idempotency',
    }));
    const { calls, fetchImpl } = createFetchRecorder();
    const client = createHttpClient({
      baseUrl: 'https://account.example.test',
      credentials: 'include',
      fetchImpl,
      headers: {
        Authorization: 'Bearer client-token',
        'X-Auth-User': 'client-account',
        'X-CSRF-Token': 'client-csrf',
        'Idempotency-Key': 'client-idempotency',
      },
      idempotencyKey: 'generated-idempotency',
    });

    await client.request(
      'https://uploads.example.test/files?authuser=42&upload=avatar',
      SuccessSchema,
      {
        body: { content: 'safe' },
        credentials: 'include',
        headers: {
          Authorization: 'Bearer request-token',
          'X-Auth-User': 'request-account',
          'X-CSRF-Token': 'request-csrf',
          'Idempotency-Key': 'request-idempotency',
        },
        idempotencyKey: 'request-option-idempotency',
        method: 'POST',
      },
    );

    const request = readFetchCall(calls);
    expect(request.url.origin).toBe('https://uploads.example.test');
    expect(request.url.searchParams.get('upload')).toBe('avatar');
    expect(request.url.searchParams.has('authuser')).toBe(false);
    expect(request.headers.has('Authorization')).toBe(false);
    expect(request.headers.has('X-Auth-User')).toBe(false);
    expect(request.headers.has('X-CSRF-Token')).toBe(false);
    expect(request.headers.has('Idempotency-Key')).toBe(false);
    expect(request.headers.has('X-Requested-With')).toBe(false);
    expect(request.init.credentials).toBe('omit');
    expect(request.init).not.toHaveProperty('idempotencyKey');
  });

  it('preserves account scoping for an absolute URL on the configured origin', async () => {
    installBrowserContext({
      cookie: 'csrf_token_7=browser-csrf',
      pathname: '/account/7/preferences',
    });
    configureHttpRequestContextHeaders(() => ({ 'X-Request-Context': 'browser' }));
    const { calls, fetchImpl } = createFetchRecorder();
    const client = createHttpClient({
      baseUrl: 'https://account.example.test',
      credentials: 'include',
      fetchImpl,
      idempotencyKey: 'same-origin-idempotency',
    });

    await client.request(
      'https://account.example.test/auth/me/preferences?source=settings',
      SuccessSchema,
      {
        body: { theme: 'dark' },
        method: 'PUT',
      },
    );

    const request = readFetchCall(calls);
    expect(request.url.searchParams.get('authuser')).toBe('7');
    expect(request.url.searchParams.get('source')).toBe('settings');
    expect(request.headers.get('X-Auth-User')).toBe('7');
    expect(request.headers.get('X-CSRF-Token')).toBe('browser-csrf');
    expect(request.headers.get('Idempotency-Key')).toBe('same-origin-idempotency');
    expect(request.headers.get('X-Request-Context')).toBe('browser');
    expect(request.init.credentials).toBe('include');
  });

  it('preserves configured same-origin request context without a browser window', async () => {
    const { calls, fetchImpl } = createFetchRecorder();
    const client = createHttpClient({
      baseUrl: 'https://internal-api.example.test/v1',
      credentials: 'include',
      fetchImpl,
      headers: {
        'X-Auth-User': 'service-account',
        'X-CSRF-Token': 'server-csrf',
      },
      idempotencyKey: 'server-idempotency',
    });

    await client.request('/jobs', SuccessSchema, {
      body: { kind: 'reconcile' },
      method: 'POST',
    });

    const request = readFetchCall(calls);
    expect(request.url.toString()).toBe('https://internal-api.example.test/v1/jobs');
    expect(request.headers.get('X-Auth-User')).toBe('service-account');
    expect(request.headers.get('X-CSRF-Token')).toBe('server-csrf');
    expect(request.headers.get('Idempotency-Key')).toBe('server-idempotency');
    expect(request.init.credentials).toBe('include');
  });
});
