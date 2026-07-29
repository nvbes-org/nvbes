import { afterEach, describe, expect, it } from 'vite-plus/test';
import { z } from 'zod';
import {
  applyAjaxRequestHeader,
  applyIdempotencyKey,
  createHttpClient,
  createIdempotencyKey,
  createRequestHeaders,
} from './index';

const SuccessSchema = z.object({ ok: z.literal(true) });

afterEach(() => {
  Reflect.deleteProperty(globalThis, 'window');
  Reflect.deleteProperty(globalThis, 'document');
});

describe('request header policy', () => {
  it('adds AJAX and idempotency headers only to eligible methods', () => {
    const getHeaders = createRequestHeaders('GET');
    expect([...getHeaders]).toEqual([]);

    const postHeaders = createRequestHeaders('post', undefined, 'stable-key');
    expect(postHeaders.get('X-Requested-With')).toBe('XMLHttpRequest');
    expect(postHeaders.get('Idempotency-Key')).toBe('stable-key');

    const deleteHeaders = createRequestHeaders('DELETE');
    expect(deleteHeaders.get('X-Requested-With')).toBe('XMLHttpRequest');
    expect(deleteHeaders.has('Idempotency-Key')).toBe(false);
  });

  it('preserves explicit headers and supports disabling automatic idempotency', () => {
    const headers = new Headers({
      'Idempotency-Key': 'caller-key',
      'X-Requested-With': 'Fetch',
    });
    applyAjaxRequestHeader(headers, 'POST');
    applyIdempotencyKey(headers, 'POST');
    expect(headers.get('X-Requested-With')).toBe('Fetch');
    expect(headers.get('Idempotency-Key')).toBe('caller-key');

    const disabled = new Headers();
    applyIdempotencyKey(disabled, 'PATCH', false);
    expect(disabled.has('Idempotency-Key')).toBe(false);
  });

  it('creates a non-empty unique default idempotency key', () => {
    const first = createIdempotencyKey();
    const second = createIdempotencyKey();
    expect(first.length).toBeGreaterThanOrEqual(32);
    expect(second).not.toBe(first);
  });

  it.each([
    ['/account/12/security', '?authuser=3', '__Host-csrf_token_12=path-csrf', '12', 'path-csrf'],
    ['/settings', '?authuser=3', 'csrf_token_3=query-csrf', '3', 'query-csrf'],
    ['/u/8/sessions', '', 'csrf_token_8=legacy-csrf', '8', 'legacy-csrf'],
  ])('binds account and CSRF context from %s', async (pathname, search, cookie, authuser, csrf) => {
    installBrowser(pathname, search, cookie);
    const recorder = fetchRecorder();
    const client = createHttpClient({
      baseUrl: 'https://account.example.test',
      credentials: 'include',
      fetchImpl: recorder.fetchImpl,
    });

    await client.post('/auth/action', SuccessSchema, { action: 'test' });

    const request = recorder.onlyCall();
    expect(request.url.searchParams.get('authuser')).toBe(authuser);
    expect(request.headers.get('X-Auth-User')).toBe(authuser);
    expect(request.headers.get('X-CSRF-Token')).toBe(csrf);
  });

  it('serializes JSON but preserves native request bodies', async () => {
    const recorder = fetchRecorder(3);
    const client = createHttpClient({
      baseUrl: 'https://account.example.test',
      fetchImpl: recorder.fetchImpl,
    });
    const form = new URLSearchParams({ grant_type: 'authorization_code' });
    const blob = new Blob(['raw-body'], { type: 'text/plain' });

    await client.post('/json', SuccessSchema, { value: 42 });
    await client.post('/form', SuccessSchema, form);
    await client.post('/blob', SuccessSchema, blob);

    const [json, formCall, blobCall] = recorder.calls;
    expect(json?.init.body).toBe(JSON.stringify({ value: 42 }));
    expect(json?.headers.get('Content-Type')).toBe('application/json');
    expect(formCall?.init.body).toBe(form);
    expect(blobCall?.init.body).toBe(blob);
  });
});

describe('request body E2EE', () => {
  it('encrypts JSON with authenticated metadata headers', async () => {
    const recorder = fetchRecorder();
    const client = createHttpClient({
      baseUrl: 'https://account.example.test',
      fetchImpl: recorder.fetchImpl,
      requestE2ee: {
        keyId: 'test-key',
        secret: '0123456789abcdef0123456789abcdef',
      },
    });

    await client.post('/auth/secret', SuccessSchema, { secret: 'private' });

    const request = recorder.onlyCall();
    expect(request.init.body).toBeInstanceOf(ArrayBuffer);
    expect(request.headers.get('Content-Type')).toBe('application/octet-stream');
    expect(request.headers.get('X-Nvbes-E2ee')).toBe('aes-256-gcm');
    expect(request.headers.get('X-Nvbes-E2ee-Key-Id')).toBe('test-key');
    expect(request.headers.get('X-Nvbes-E2ee-Nonce')).toBeTruthy();
    expect(request.headers.get('X-Nvbes-E2ee-Salt')).toBeTruthy();
  });

  it('can disable configured encryption per request and rejects weak secrets', async () => {
    const recorder = fetchRecorder();
    const client = createHttpClient({
      baseUrl: 'https://account.example.test',
      fetchImpl: recorder.fetchImpl,
      requestE2ee: {
        keyId: 'test-key',
        secret: '0123456789abcdef0123456789abcdef',
      },
    });

    await client.post('/plain', SuccessSchema, { visible: true }, { requestE2ee: false });
    expect(recorder.onlyCall().init.body).toBe(JSON.stringify({ visible: true }));

    const weakClient = createHttpClient({
      baseUrl: 'https://account.example.test',
      fetchImpl: recorder.fetchImpl,
      requestE2ee: { keyId: 'weak', secret: 'too-short' },
    });
    await expect(weakClient.post('/weak', SuccessSchema, { secret: true })).rejects.toThrow(
      'at least 32 characters',
    );
  });
});

type FetchCall = {
  headers: Headers;
  init: RequestInit;
  url: URL;
};

function fetchRecorder(expectedCalls = 1): {
  calls: FetchCall[];
  fetchImpl: typeof fetch;
  onlyCall: () => FetchCall;
} {
  const calls: FetchCall[] = [];
  const fetchImpl: typeof fetch = async (input, init = {}) => {
    const rawUrl =
      typeof input === 'string'
        ? input
        : input instanceof URL
          ? input.toString()
          : input.url;
    calls.push({
      headers: new Headers(init.headers),
      init,
      url: new URL(rawUrl),
    });
    return new Response(JSON.stringify({ ok: true }), { status: 200 });
  };
  return {
    calls,
    fetchImpl,
    onlyCall: () => {
      expect(calls).toHaveLength(expectedCalls);
      const call = calls[0];
      if (!call) throw new Error('Expected a fetch call');
      return call;
    },
  };
}

function installBrowser(pathname: string, search: string, cookie: string): void {
  Object.defineProperty(globalThis, 'window', {
    configurable: true,
    value: { location: { pathname, search } },
  });
  Object.defineProperty(globalThis, 'document', {
    configurable: true,
    value: { cookie },
  });
}
