import { describe, expect, it } from 'vite-plus/test';
import {
  AccountAuthenticationError,
  AccountClient,
  AccountDtoValidationError,
  AccountHttpError,
} from './index';

type RecordedFetch = { input: RequestInfo | URL; init: RequestInit | undefined };

describe('AccountClient V1 contract', () => {
  it('uses bearer-only transport and validates the Account profile envelope', async () => {
    const recorder = fetchRecorder(profileEnvelope());
    const client = clientFor(recorder);

    await expect(client.getProfile()).resolves.toMatchObject({ display_name: 'Account Person' });

    const request = onlyRequest(recorder.calls);
    expect(request.url.pathname).toBe('/api/v1/profile');
    expect(request.headers.get('Authorization')).toBe('Bearer account-access-token');
    expect(request.init.credentials).toBe('omit');
    expect(request.headers.has('Cookie')).toBe(false);
  });

  it('models team creation, listing and join with the runtime routes', async () => {
    const team = {
      created_at: '2026-09-03T12:00:00Z',
      id: '7f8c519f-4ca9-4242-a49e-b0d174e4c14b',
      name: 'Invited team',
      role: 'owner',
    };
    const responses = [
      { ...team, join_code: 'team_123' },
      { teams: [team] },
      { ...team, role: 'member' },
    ];
    const recorder = fetchRecorder(() => responses.shift());
    const client = clientFor(recorder);

    await client.createTeam({ name: 'Invited team' });
    await client.listTeams();
    await client.joinTeam({ join_code: 'team_123' });

    expect(requestAt(recorder.calls, 0).url.pathname).toBe('/api/v1/teams');
    expect(requestAt(recorder.calls, 0).init.method).toBe('POST');
    expect(requestAt(recorder.calls, 1).init.method).toBe('GET');
    expect(requestAt(recorder.calls, 2).url.pathname).toBe('/api/v1/teams/join');
  });

  it('uses durable export and cancellable closure resources', async () => {
    const id = '7f8c519f-4ca9-4242-a49e-b0d174e4c14b';
    const responses = [
      { export_id: id, requested_at: '2026-09-03T12:00:00Z', status: 'pending' },
      { requested_at: '2026-09-03T12:00:00Z', saga_id: id, status: 'pending' },
      undefined,
    ];
    const recorder = fetchRecorder(
      () => responses.shift(),
      () => (responses.length === 0 ? 204 : 202),
    );
    const client = clientFor(recorder);

    await client.requestDataExport();
    await client.closeAccount();
    await client.cancelAccountClosure();

    expect(requestAt(recorder.calls, 0).url.pathname).toBe('/api/v1/privacy/exports');
    expect(requestAt(recorder.calls, 1).url.pathname).toBe('/api/v1/closure');
    expect(requestAt(recorder.calls, 2).url.pathname).toBe('/api/v1/closure/cancel');
  });

  it('fails locally without a token and exposes typed remote failures', async () => {
    const recorder = fetchRecorder(profileEnvelope());
    const unauthenticated = new AccountClient({
      fetchImpl: recorder.fetchImpl,
      getAccessToken: () => null,
    });
    await expect(unauthenticated.getProfile()).rejects.toBeInstanceOf(AccountAuthenticationError);
    expect(recorder.calls).toHaveLength(0);

    const forbidden = fetchRecorder(
      { error: { message: 'Step-up required', request_id: 'request-1' } },
      () => 403,
    );
    await expect(clientFor(forbidden).closeAccount()).rejects.toBeInstanceOf(AccountHttpError);

    const malformed = fetchRecorder({ unexpected: true });
    await expect(clientFor(malformed).getProfile()).rejects.toBeInstanceOf(
      AccountDtoValidationError,
    );
  });
});

function profileEnvelope(): unknown {
  return {
    user: {
      birthdate: null,
      created_at: '2026-09-03T12:00:00Z',
      display_name: 'Account Person',
      firstname: null,
      id: '7f8c519f-4ca9-4242-a49e-b0d174e4c14b',
      lastname: null,
      region: null,
      username: null,
    },
  };
}

function clientFor(recorder: ReturnType<typeof fetchRecorder>): AccountClient {
  return new AccountClient({
    baseUrl: 'https://account.example.test',
    fetchImpl: recorder.fetchImpl,
    getAccessToken: () => 'account-access-token',
  });
}

function fetchRecorder(
  body: unknown,
  status: () => number = () => 200,
): { calls: RecordedFetch[]; fetchImpl: typeof fetch } {
  const calls: RecordedFetch[] = [];
  const fetchImpl: typeof fetch = async (input, init) => {
    calls.push({ input, init });
    const value = typeof body === 'function' ? (body as () => unknown)() : body;
    return new Response(value === undefined ? null : JSON.stringify(value), {
      headers: value === undefined ? undefined : { 'Content-Type': 'application/json' },
      status: status(),
    });
  };
  return { calls, fetchImpl };
}

function onlyRequest(calls: RecordedFetch[]): ReturnType<typeof requestAt> {
  expect(calls).toHaveLength(1);
  return requestAt(calls, 0);
}

function requestAt(calls: RecordedFetch[], index: number) {
  const call = calls[index];
  if (!call) throw new Error(`Expected fetch call at index ${index}`);
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
