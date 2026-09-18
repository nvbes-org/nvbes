import type { HttpClient, HttpRequestOptions } from '@nvbes/http-client';
import { describe, expect, it } from 'vite-plus/test';
import type { z } from 'zod';
import { AccountIdentityClient } from './identity.account-client';

describe('AccountIdentityClient contract', () => {
  it('rejects malformed account payloads at the transport boundary', async () => {
    const { client } = clientWith({
      current_organization_id: null,
      current_tenant_id: null,
      current_workspace_id: null,
      current_workspace_region: null,
      user: {
        ...principal(),
        email: 'not-an-email',
      },
    });

    await expect(client.getMe()).rejects.toThrow();
  });

  it('returns stable empty arrays for optional collection envelopes', async () => {
    const { client, transport } = clientWith({});

    await expect(client.listWorkspaces()).resolves.toEqual([]);
    await expect(client.listAccounts()).resolves.toEqual([]);
    expect(transport.calls.map(({ path }) => path)).toEqual(['/workspaces', '/auth/accounts']);
  });

  it('encodes pagination cursors and forwards cancellation signals', async () => {
    const signal = new AbortController().signal;
    const { client, transport } = clientWith({
      emails: [],
      has_more: false,
      next_cursor: null,
      primary_min_age_hours: 24,
    });

    await client.listEmailsPage({ cursor: 'next/page?tenant=other', limit: 25, signal });

    expect(transport.calls[0]).toMatchObject({
      method: 'GET',
      path: '/auth/me/emails?limit=25&cursor=next%2Fpage%3Ftenant%3Dother',
      signal,
    });
  });

  it('encodes every externally supplied identifier used as a path segment', async () => {
    const { client, transport } = clientWith({ success: true });
    const hostileSegment = 'tenant/other?admin=true';

    await client.deleteSecondaryEmail(hostileSegment);
    await client.revokeSession(hostileSegment);
    await client.forgetAccount(hostileSegment);
    await client.revokeOAuthClient(hostileSegment);

    expect(transport.calls.map(({ path }) => path)).toEqual([
      '/auth/me/emails/tenant%2Fother%3Fadmin%3Dtrue',
      '/auth/sessions/tenant%2Fother%3Fadmin%3Dtrue',
      '/auth/accounts/tenant%2Fother%3Fadmin%3Dtrue',
      '/oauth/clients/tenant%2Fother%3Fadmin%3Dtrue',
    ]);
  });

  it('encodes the session identifier used by high-risk confirmation', async () => {
    const { client, transport } = clientWith(accountSession());

    await client.confirmHighRiskSession('session/other?confirm=false');

    expect(transport.calls[0]?.path).toBe(
      '/auth/sessions/session%2Fother%3Fconfirm%3Dfalse/confirm',
    );
  });

  it('keeps password reset field names aligned with the API contract', async () => {
    const { client, transport } = clientWith({ success: true });

    await client.forgotPassword('person@example.test');
    await client.resetPassword('one-time-token', 'new secret');

    expect(transport.calls).toEqual([
      {
        body: { email: 'person@example.test' },
        method: 'POST',
        path: '/auth/password/forgot',
        signal: undefined,
      },
      {
        body: { new_password: 'new secret', token: 'one-time-token' },
        method: 'POST',
        path: '/auth/password/reset',
        signal: undefined,
      },
    ]);
  });
});

type RecordedCall = {
  body?: unknown;
  method: 'DELETE' | 'GET' | 'POST';
  path: string;
  signal: AbortSignal | null | undefined;
};

class RecordingTransport {
  readonly calls: RecordedCall[] = [];

  constructor(private readonly response: unknown) {}

  async get<T>(path: string, schema: z.ZodType<T>, options?: HttpRequestOptions): Promise<T> {
    return this.record('GET', path, schema, undefined, options);
  }

  async post<T>(
    path: string,
    schema: z.ZodType<T>,
    body?: unknown,
    options?: HttpRequestOptions,
  ): Promise<T> {
    return this.record('POST', path, schema, body, options);
  }

  async delete<T>(path: string, schema: z.ZodType<T>, options?: HttpRequestOptions): Promise<T> {
    return this.record('DELETE', path, schema, undefined, options);
  }

  private async record<T>(
    method: RecordedCall['method'],
    path: string,
    schema: z.ZodType<T>,
    body: unknown,
    options?: HttpRequestOptions,
  ): Promise<T> {
    this.calls.push({ body, method, path, signal: options?.signal });
    return schema.parse(this.response);
  }
}

function clientWith(response: unknown): {
  client: AccountIdentityClient;
  transport: RecordingTransport;
} {
  const transport = new RecordingTransport(response);
  return {
    client: new AccountIdentityClient({
      http: transport as unknown as HttpClient,
    }),
    transport,
  };
}

function principal() {
  return {
    created_at: '2026-07-29T12:00:00Z',
    display_name: 'Test Person',
    email: 'person@example.test',
    email_verified: true,
    id: 'principal-1',
    mfa_enabled: true,
  };
}

function accountSession() {
  return {
    client: null,
    created_at: '2026-07-29T12:00:00Z',
    current: true,
    device_id: null,
    device_trust_level: null,
    device_trust_score: null,
    expires_at: '2026-07-30T12:00:00Z',
    id: 'session-1',
    ip: null,
    last_seen_at: '2026-07-29T12:00:00Z',
    organization_id: null,
    revoked_at: null,
    risk_confirmed_at: null,
    risk_decision: null,
    risk_score: null,
    tenant_id: null,
    user_agent: null,
    workspace_id: null,
    workspace_region: null,
  };
}
