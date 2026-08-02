import { describe, expect, it } from 'vite-plus/test';
import {
  AccountAuthenticationError,
  AccountClient,
  AccountDtoValidationError,
  AccountHttpError,
} from './index';

type RecordedFetch = {
  input: RequestInfo | URL;
  init: RequestInit | undefined;
};

describe('AccountClient OAuth transport', () => {
  it('sends the bearer token and explicitly omits browser credentials', async () => {
    const recorder = fetchRecorder(profileEnvelope());
    const client = new AccountClient({
      baseUrl: 'https://account.example.test',
      fetchImpl: recorder.fetchImpl,
      getAccessToken: async () => 'account-access-token',
    });

    await expect(client.getProfile()).resolves.toMatchObject({
      display_name: 'Account Person',
      id: 'principal-1',
    });

    const request = onlyRequest(recorder.calls);
    expect(request.url.toString()).toBe('https://account.example.test/api/v1/profile');
    expect(request.headers.get('Authorization')).toBe('Bearer account-access-token');
    expect(request.headers.has('Cookie')).toBe(false);
    expect(request.headers.has('X-Auth-User')).toBe(false);
    expect(request.headers.has('X-CSRF-Token')).toBe(false);
    expect(request.init.credentials).toBe('omit');
  });

  it('fails before the network when no access token is available', async () => {
    const recorder = fetchRecorder(profileEnvelope());
    const client = new AccountClient({
      fetchImpl: recorder.fetchImpl,
      getAccessToken: () => null,
    });

    await expect(client.getProfile()).rejects.toBeInstanceOf(AccountAuthenticationError);
    expect(recorder.calls).toHaveLength(0);
  });

  it('resolves a fresh token for every request', async () => {
    const recorder = fetchRecorder({ language: 'fr', theme: 'dark' });
    const tokens = ['first-token', 'second-token'];
    const client = new AccountClient({
      fetchImpl: recorder.fetchImpl,
      getAccessToken: () => tokens.shift() ?? null,
    });

    await client.getPreferences();
    await client.getPreferences();

    expect(
      recorder.calls.map((call) => new Headers(call.init?.headers).get('Authorization')),
    ).toEqual(['Bearer first-token', 'Bearer second-token']);
    expect(recorder.calls.every((call) => call.init?.credentials === 'omit')).toBe(true);
  });

  it('keeps bearer-only transport for binary downloads', async () => {
    const calls: RecordedFetch[] = [];
    const client = new AccountClient({
      fetchImpl: async (input, init) => {
        calls.push({ input, init });
        return new Response('avatar', { status: 200 });
      },
      getAccessToken: () => 'download-token',
    });

    await expect(client.downloadAvatar()).resolves.toBeInstanceOf(Blob);

    const request = onlyRequest(calls);
    expect(request.headers.get('Authorization')).toBe('Bearer download-token');
    expect(request.headers.has('Cookie')).toBe(false);
    expect(request.init.credentials).toBe('omit');
  });

  it('encodes consent cursors and sends Account-only preference fields', async () => {
    const responses = [
      {
        consents: [],
        has_more: false,
        next_cursor: null,
      },
      { language: 'en', theme: 'system' },
    ];
    const recorder = fetchRecorder(() => responses.shift());
    const client = new AccountClient({
      baseUrl: 'https://account.example.test/gateway',
      fetchImpl: recorder.fetchImpl,
      getAccessToken: () => 'token',
    });

    await client.listConsents({ cursor: 'next/page?scope=other', limit: 25 });
    await client.updatePreferences({ language: 'en', theme: 'system' });

    const first = requestAt(recorder.calls, 0);
    expect(first.url.toString()).toBe(
      'https://account.example.test/gateway/api/v1/consents?limit=25&cursor=next%2Fpage%3Fscope%3Dother',
    );
    const second = requestAt(recorder.calls, 1);
    if (typeof second.init.body !== 'string') {
      throw new Error('Expected a JSON request body');
    }
    expect(JSON.parse(second.init.body) as unknown).toEqual({
      language: 'en',
      theme: 'system',
    });
  });

  it('lists and revokes sessions through the Account security API', async () => {
    const responses = [{ has_more: false, next_cursor: null, sessions: [] }, { success: true }];
    const recorder = fetchRecorder(() => responses.shift());
    const client = new AccountClient({
      baseUrl: 'https://account.example.test',
      fetchImpl: recorder.fetchImpl,
      getAccessToken: () => 'session-token',
    });

    await client.listSessions({ cursor: 'next/session', limit: 20 });
    await client.revokeSession('session/with spaces');

    const listRequest = requestAt(recorder.calls, 0);
    expect(listRequest.url.toString()).toBe(
      'https://account.example.test/api/v1/security/sessions?limit=20&cursor=next%2Fsession',
    );
    expect(listRequest.init.method).toBe('GET');

    const revokeRequest = requestAt(recorder.calls, 1);
    expect(revokeRequest.url.toString()).toBe(
      'https://account.example.test/api/v1/security/sessions/session%2Fwith%20spaces',
    );
    expect(revokeRequest.init.method).toBe('DELETE');
    expect(revokeRequest.headers.get('Authorization')).toBe('Bearer session-token');
    expect(revokeRequest.init.credentials).toBe('omit');
  });

  it('returns the durable closure saga accepted by Account', async () => {
    const recorder = fetchRecorder({
      requested_at: '2026-08-02T12:00:00Z',
      saga_id: '7f8c519f-4ca9-4242-a49e-b0d174e4c14b',
      status: 'pending',
    });
    const client = new AccountClient({
      baseUrl: 'https://account.example.test',
      fetchImpl: recorder.fetchImpl,
      getAccessToken: () => 'closure-token',
    });

    await expect(client.closeAccount()).resolves.toMatchObject({ status: 'pending' });
    const request = onlyRequest(recorder.calls);
    expect(request.url.toString()).toBe('https://account.example.test/api/v1/closure');
    expect(request.init.method).toBe('POST');
  });

  it('reads participant checkpoints for an accepted closure', async () => {
    const recorder = fetchRecorder({
      completed_at: null,
      last_error: null,
      participants: [
        {
          attempts: 1,
          completed_at: '2026-08-02T12:00:01Z',
          last_error: null,
          participant: 'cloud',
          status: 'completed',
        },
      ],
      requested_at: '2026-08-02T12:00:00Z',
      saga_id: '7f8c519f-4ca9-4242-a49e-b0d174e4c14b',
      status: 'dispatching',
      updated_at: '2026-08-02T12:00:01Z',
    });
    const client = new AccountClient({
      baseUrl: 'https://account.example.test',
      fetchImpl: recorder.fetchImpl,
      getAccessToken: () => 'closure-token',
    });

    await expect(client.getAccountClosure()).resolves.toMatchObject({
      participants: [{ participant: 'cloud', status: 'completed' }],
    });
    expect(onlyRequest(recorder.calls).init.method).toBe('GET');
  });

  it('exposes typed HTTP and DTO failures', async () => {
    const failed = fetchRecorder(
      { error: { message: 'Scope refused', request_id: 'request-1' } },
      { status: 403 },
    );
    const malformed = fetchRecorder({ unexpected: true });
    const failedClient = new AccountClient({
      fetchImpl: failed.fetchImpl,
      getAccessToken: () => 'token',
    });
    const malformedClient = new AccountClient({
      fetchImpl: malformed.fetchImpl,
      getAccessToken: () => 'token',
    });

    await expect(failedClient.getProfile()).rejects.toMatchObject({
      message: 'Scope refused',
      requestId: 'request-1',
      status: 403,
    });
    await expect(failedClient.getProfile()).rejects.toBeInstanceOf(AccountHttpError);
    await expect(malformedClient.getProfile()).rejects.toBeInstanceOf(AccountDtoValidationError);
  });
});

function profileEnvelope(): unknown {
  return {
    current_organization_id: null,
    current_tenant_id: null,
    current_workspace_id: null,
    current_workspace_region: null,
    user: {
      birthdate: null,
      created_at: '2026-07-30T12:00:00Z',
      display_name: 'Account Person',
      email: 'identity-owned@example.test',
      email_verified: true,
      firstname: null,
      id: 'principal-1',
      lastname: null,
      mfa_enabled: true,
      region: null,
      username: null,
    },
  };
}

function fetchRecorder(
  body: unknown,
  init: { status?: number } = {},
): {
  calls: RecordedFetch[];
  fetchImpl: typeof fetch;
} {
  const calls: RecordedFetch[] = [];
  const fetchImpl: typeof fetch = async (input, requestInit) => {
    calls.push({ input, init: requestInit });
    const responseBody = typeof body === 'function' ? (body as () => unknown)() : body;
    return new Response(JSON.stringify(responseBody), {
      headers: { 'Content-Type': 'application/json' },
      status: init.status ?? 200,
    });
  };
  return { calls, fetchImpl };
}

function onlyRequest(calls: RecordedFetch[]): ReturnType<typeof requestAt> {
  expect(calls).toHaveLength(1);
  return requestAt(calls, 0);
}

function requestAt(
  calls: RecordedFetch[],
  index: number,
): {
  headers: Headers;
  init: RequestInit;
  url: URL;
} {
  const call = calls[index];
  if (!call) {
    throw new Error(`Expected fetch call at index ${index}`);
  }
  const rawUrl =
    typeof call.input === 'string'
      ? call.input
      : call.input instanceof URL
        ? call.input.toString()
        : call.input.url;
  return {
    headers: new Headers(call.init?.headers),
    init: call.init ?? {},
    url: new URL(rawUrl),
  };
}
