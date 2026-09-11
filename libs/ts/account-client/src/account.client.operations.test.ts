import { describe, expect, it } from 'vite-plus/test';
import { createAccountClient } from './account.client';

type RecordedRequest = { input: RequestInfo | URL; init?: RequestInit };

describe('AccountClient remaining V1 operations', () => {
  it('maps profile, preferences and notification updates to their runtime contracts', async () => {
    const profile = profileEnvelope();
    const profileUpdate = {
      birthdate: null,
      firstname: 'Ada',
      lastname: null,
      region: null,
      username: null,
    };
    const preferences = { language: 'fr', theme: 'dark' } as const;
    const notifications = { email: true, in_app: true, marketing_email: false, push: false };
    const recorder = responseQueue([
      profile,
      preferences,
      preferences,
      notifications,
      notifications,
    ]);
    const signal = AbortSignal.abort();
    const client = createAccountClient({
      baseUrl: 'https://account.example.test/root/',
      fetchImpl: recorder.fetchImpl,
      getAccessToken: async () => ' account-access-token ',
    });

    await expect(client.updateProfile(profileUpdate, { signal })).resolves.toEqual(profile.user);
    await expect(client.getPreferences({ signal })).resolves.toEqual(preferences);
    await expect(client.updatePreferences(preferences, { signal })).resolves.toEqual(preferences);
    await expect(client.getNotifications({ signal })).resolves.toEqual(notifications);
    await expect(client.updateNotifications(notifications, { signal })).resolves.toEqual(
      notifications,
    );

    expectRequest(recorder.calls[0], '/root/api/v1/profile', 'PUT', profileUpdate, signal);
    expectRequest(recorder.calls[1], '/root/api/v1/preferences', 'GET', undefined, signal);
    expectRequest(recorder.calls[2], '/root/api/v1/preferences', 'PUT', preferences, signal);
    expectRequest(recorder.calls[3], '/root/api/v1/notifications', 'GET', undefined, signal);
    expectRequest(recorder.calls[4], '/root/api/v1/notifications', 'PUT', notifications, signal);
  });

  it('reads export and closure status and downloads an encoded export resource', async () => {
    const exportStatus = {
      completed_at: null,
      expires_at: null,
      export_id: '7f8c519f-4ca9-4242-a49e-b0d174e4c14b',
      last_error: null,
      participants: [],
      requested_at: '2026-09-03T12:00:00Z',
      status: 'processing',
      updated_at: '2026-09-03T12:01:00Z',
    };
    const closureStatus = {
      completed_at: null,
      last_error: null,
      participants: [],
      requested_at: '2026-09-03T12:00:00Z',
      saga_id: '7f8c519f-4ca9-4242-a49e-b0d174e4c14b',
      status: 'processing',
      updated_at: '2026-09-03T12:01:00Z',
    };
    const recorder = responseQueue([exportStatus, new Blob(['archive']), closureStatus]);
    const client = createAccountClient({
      baseUrl: 'https://account.example.test',
      fetchImpl: recorder.fetchImpl,
      getAccessToken: () => 'token',
    });

    await expect(client.getLatestDataExport()).resolves.toEqual(exportStatus);
    await expect(client.downloadDataExport('export / 1')).resolves.toBeInstanceOf(Blob);
    await expect(client.getAccountClosure()).resolves.toEqual(closureStatus);

    expect(request(recorder.calls[0]).url.pathname).toBe('/api/v1/privacy/exports/latest');
    expect(request(recorder.calls[1]).url.pathname).toBe(
      '/api/v1/privacy/exports/export%20%2F%201/document',
    );
    expect(request(recorder.calls[1]).init.credentials).toBe('omit');
    expect(new Headers(request(recorder.calls[1]).init.headers).get('Accept')).toBe(
      'application/json',
    );
    expect(new Headers(request(recorder.calls[1]).init.headers).has('Content-Type')).toBe(false);
    expect(request(recorder.calls[2]).url.pathname).toBe('/api/v1/closure');
    expect(recorder.calls.map((call) => call.init?.method)).toEqual(['GET', 'GET', 'GET']);
  });
});

function profileEnvelope() {
  return {
    user: {
      birthdate: null,
      created_at: '2026-09-03T12:00:00Z',
      display_name: 'Ada',
      firstname: 'Ada',
      id: '7f8c519f-4ca9-4242-a49e-b0d174e4c14b',
      lastname: null,
      region: null,
      username: null,
    },
  };
}

function responseQueue(values: unknown[]): { calls: RecordedRequest[]; fetchImpl: typeof fetch } {
  const calls: RecordedRequest[] = [];
  const fetchImpl: typeof fetch = async (input, init) => {
    calls.push({ input, init });
    const value = values.shift();
    if (value instanceof Blob) return new Response(value, { status: 200 });
    return new Response(JSON.stringify(value), {
      headers: { 'content-type': 'application/json' },
      status: 200,
    });
  };
  return { calls, fetchImpl };
}

function expectRequest(
  recorded: RecordedRequest | undefined,
  pathname: string,
  method: string,
  body: unknown,
  signal: AbortSignal,
) {
  const actual = request(recorded);
  expect(actual.url.pathname).toBe(pathname);
  expect(actual.init.method).toBe(method);
  expect(actual.init.signal).toBe(signal);
  expect(actual.init.body).toBe(body === undefined ? undefined : JSON.stringify(body));
  expect(new Headers(actual.init.headers).get('Authorization')).toBe('Bearer account-access-token');
  expect(new Headers(actual.init.headers).get('Accept')).toBe('application/json');
  expect(new Headers(actual.init.headers).has('Content-Type')).toBe(body !== undefined);
  if (body !== undefined) {
    expect(new Headers(actual.init.headers).get('Content-Type')).toBe('application/json');
  }
}

function request(recorded: RecordedRequest | undefined) {
  if (!recorded) throw new Error('Expected a recorded request');
  const rawUrl =
    typeof recorded.input === 'string'
      ? recorded.input
      : recorded.input instanceof URL
        ? recorded.input.toString()
        : recorded.input.url;
  return { init: recorded.init ?? {}, url: new URL(rawUrl) };
}
