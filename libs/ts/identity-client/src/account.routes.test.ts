import { createHttpClient } from '@nvbes/http-client';
import { afterEach, expect, it, vi } from 'vite-plus/test';
import { accountClient, listOAuthClients, revokeOAuthClient } from './account.client';
import { AccountIdentityClient } from './identity.account-client';

const user = {
  id: 'user-1',
  email: 'person@example.test',
  display_name: 'Person',
  email_verified: true,
  mfa_enabled: false,
  created_at: '2026-09-11',
};
const email = {
  id: 'email-1',
  email: 'secondary@example.test',
  is_primary: false,
  verified: false,
  verified_at: null,
  created_at: '2026-09-11',
};
const workspace = { id: 'team-1', name: 'Team', workspace_type: 'team', role: 'owner' };
const consent = {
  id: 'consent-1',
  principal_id: 'user-1',
  consent_type: 'analytics',
  document_version: 'v1',
  granted_at: '2026-09-11',
};
const session = {
  id: 'session-1',
  tenant_id: null,
  organization_id: null,
  workspace_id: null,
  workspace_region: null,
  created_at: '2026-09-11',
  last_seen_at: '2026-09-11',
  expires_at: '2026-09-12',
  revoked_at: null,
  ip: null,
  user_agent: null,
  client: null,
  device_id: null,
  device_trust_level: null,
  device_trust_score: null,
  risk_score: null,
  risk_decision: null,
  risk_confirmed_at: null,
  current: true,
};
const account = { authuser: '0', user, session };
const me = {
  user,
  current_tenant_id: null,
  current_organization_id: null,
  current_workspace_id: null,
  current_workspace_region: null,
};
const device = { success: true, device_id: 'device-1', trust_level: 'trusted', trust_score: 90 };
const oauth = {
  id: 'app-1',
  client_id: 'client-1',
  name: 'Test client',
  redirect_uris: [],
  created_at: '2026-09-11',
  owner_scope_type: 'user',
  owner_scope_id: 'user-1',
  client_type: 'public',
};
const emails = { emails: [email], primary_min_age_hours: 24, next_cursor: null, has_more: false };
const sessions = { sessions: [session], next_cursor: null, has_more: false };
const consents = { consents: [consent], next_cursor: null, has_more: false };
const success = { success: true };
const signal = new AbortController().signal;

type Route = {
  name: string;
  invoke: (client: AccountIdentityClient) => Promise<unknown>;
  method: string;
  path: string;
  response: unknown;
  expected: unknown;
  body?: unknown;
};
const routes: Route[] = [
  {
    name: 'profile',
    invoke: (c) => c.getMe({ signal }),
    method: 'GET',
    path: '/auth/me',
    response: me,
    expected: me,
  },
  {
    name: 'emails',
    invoke: (c) => c.listEmails({ signal }),
    method: 'GET',
    path: '/auth/me/emails',
    response: emails,
    expected: emails,
  },
  {
    name: 'add email',
    invoke: (c) => c.addSecondaryEmail(email.email),
    method: 'POST',
    path: '/auth/me/emails',
    body: { email: email.email },
    response: { email, verification_resend_available_at: 'later' },
    expected: { email, verification_resend_available_at: 'later' },
  },
  {
    name: 'promote email',
    invoke: (c) => c.promoteSecondaryEmail('a/b?'),
    method: 'POST',
    path: '/auth/me/emails/a%2Fb%3F/promote',
    body: {},
    response: { email, user },
    expected: { email, user },
  },
  {
    name: 'resend email',
    invoke: (c) => c.resendSecondaryEmailVerification('a/b?'),
    method: 'POST',
    path: '/auth/me/emails/a%2Fb%3F/resend-verification',
    body: {},
    response: { email, verification_resend_available_at: 'later' },
    expected: { email, verification_resend_available_at: 'later' },
  },
  {
    name: 'delete email',
    invoke: (c) => c.deleteSecondaryEmail('a/b?'),
    method: 'DELETE',
    path: '/auth/me/emails/a%2Fb%3F',
    response: success,
    expected: undefined,
  },
  {
    name: 'workspaces',
    invoke: (c) => c.listWorkspaces({ signal }),
    method: 'GET',
    path: '/workspaces',
    response: { workspaces: [workspace] },
    expected: [workspace],
  },
  {
    name: 'accounts',
    invoke: (c) => c.listAccounts({ signal }),
    method: 'GET',
    path: '/auth/accounts',
    response: { accounts: [account] },
    expected: [account],
  },
  {
    name: 'sessions',
    invoke: (c) => c.listSessions({ signal }),
    method: 'GET',
    path: '/auth/sessions',
    response: sessions,
    expected: [session],
  },
  {
    name: 'revoke session',
    invoke: (c) => c.revokeSession('a/b?'),
    method: 'DELETE',
    path: '/auth/sessions/a%2Fb%3F',
    response: success,
    expected: undefined,
  },
  {
    name: 'confirm session',
    invoke: (c) => c.confirmHighRiskSession('a/b?'),
    method: 'POST',
    path: '/auth/sessions/a%2Fb%3F/confirm',
    body: {},
    response: session,
    expected: session,
  },
  {
    name: 'forget account',
    invoke: (c) => c.forgetAccount('a/b?'),
    method: 'DELETE',
    path: '/auth/accounts/a%2Fb%3F',
    response: success,
    expected: undefined,
  },
  {
    name: 'revoke others',
    invoke: (c) => c.revokeOtherSessions(),
    method: 'POST',
    path: '/auth/sessions/revoke-others',
    body: {},
    response: success,
    expected: undefined,
  },
  {
    name: 'revoke all',
    invoke: (c) => c.revokeAllSessions(),
    method: 'POST',
    path: '/auth/sessions/revoke-all',
    body: {},
    response: success,
    expected: undefined,
  },
  {
    name: 'trust device',
    invoke: (c) => c.trustDevice('a/b?'),
    method: 'POST',
    path: '/auth/devices/a%2Fb%3F/trust',
    body: {},
    response: device,
    expected: device,
  },
  {
    name: 'revoke device',
    invoke: (c) => c.revokeDevice('a/b?'),
    method: 'DELETE',
    path: '/auth/devices/a%2Fb%3F',
    response: device,
    expected: device,
  },
  {
    name: 'logout',
    invoke: (c) => c.logout(),
    method: 'POST',
    path: '/auth/logout',
    body: {},
    response: success,
    expected: undefined,
  },
  {
    name: 'consents',
    invoke: (c) => c.listConsents({ signal }),
    method: 'GET',
    path: '/legal/consents',
    response: consents,
    expected: [consent],
  },
  {
    name: 'grant consent',
    invoke: (c) => c.grantConsent('analytics', 'v1'),
    method: 'POST',
    path: '/legal/consent',
    body: { consent_type: 'analytics', document_version: 'v1' },
    response: consent,
    expected: consent,
  },
  {
    name: 'GPC',
    invoke: (c) => c.gpcStatus({ signal }),
    method: 'GET',
    path: '/legal/gpc',
    response: { gpc_enabled: true, gpc_opt_out_active: true },
    expected: { gpc_enabled: true, gpc_opt_out_active: true },
  },
  {
    name: 'revoke consent',
    invoke: (c) => c.revokeConsent('analytics', 'v1'),
    method: 'POST',
    path: '/legal/consent/revoke',
    body: { consent_type: 'analytics', document_version: 'v1' },
    response: undefined,
    expected: undefined,
  },
  {
    name: 'OAuth clients',
    invoke: (c) => c.listOAuthClients({ signal }),
    method: 'GET',
    path: '/oauth/clients',
    response: { clients: [oauth] },
    expected: [oauth],
  },
  {
    name: 'revoke OAuth client',
    invoke: (c) => c.revokeOAuthClient('a/b?'),
    method: 'DELETE',
    path: '/oauth/clients/a%2Fb%3F',
    response: success,
    expected: undefined,
  },
  {
    name: 'create workspace',
    invoke: (c) => c.createWorkspace({ name: 'Team', workspace_type: 'team' }),
    method: 'POST',
    path: '/workspaces',
    body: { name: 'Team', workspace_type: 'team' },
    response: { workspace },
    expected: workspace,
  },
  {
    name: 'forgot password',
    invoke: (c) => c.forgotPassword(user.email),
    method: 'POST',
    path: '/auth/password/forgot',
    body: { email: user.email },
    response: success,
    expected: success,
  },
  {
    name: 'reset password',
    invoke: (c) => c.resetPassword('token', 'new secret'),
    method: 'POST',
    path: '/auth/password/reset',
    body: { token: 'token', new_password: 'new secret' },
    response: success,
    expected: success,
  },
];

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

it.each(routes)(
  '$name uses its HTTP contract and returns validated business data',
  async (route) => {
    const fetchImpl = vi
      .fn<typeof fetch>()
      .mockResolvedValue(
        route.response === undefined
          ? new Response(null, { status: 204 })
          : Response.json(route.response),
      );
    const client = new AccountIdentityClient({
      http: createHttpClient({
        baseUrl: 'https://identity.example.test',
        credentials: 'include',
        fetchImpl,
      }),
    });
    await expect(route.invoke(client)).resolves.toEqual(route.expected);
    expect(fetchImpl).toHaveBeenCalledTimes(1);
    const [url, init] = fetchImpl.mock.calls[0] ?? [];
    expect(url).toBe(`https://identity.example.test${route.path}`);
    expect(init?.method).toBe(route.method);
    expect(init?.credentials).toBe('include');
    expect(init?.body).toBe(route.body === undefined ? undefined : JSON.stringify(route.body));
    if (route.method === 'GET') expect(init?.signal).toBe(signal);
  },
);

it.each(routes)('$name rejects a broken response instead of reporting success', async (route) => {
  const fetchImpl = vi.fn<typeof fetch>().mockResolvedValue(Response.json(null));
  const client = new AccountIdentityClient({ http: createHttpClient({ fetchImpl }) });
  await expect(route.invoke(client)).rejects.toThrow();
});

for (const [name, response, path] of [
  ['listEmailsPage', emails, '/auth/me/emails'],
  ['listSessionsPage', sessions, '/auth/sessions'],
  ['listConsentsPage', consents, '/legal/consents'],
] as const) {
  it.each([
    [undefined, ''],
    [{}, ''],
    [{ cursor: '' }, ''],
    [{ limit: 0 }, '?limit=0'],
    [{ limit: 25 }, '?limit=25'],
    [{ cursor: 'a/b?x=1' }, '?cursor=a%2Fb%3Fx%3D1'],
    [{ limit: 5, cursor: 'a b' }, '?limit=5&cursor=a+b'],
  ])(`${name} encodes pagination %j`, async (options, query) => {
    const fetchImpl = vi.fn<typeof fetch>().mockImplementation(async () => Response.json(response));
    const client = new AccountIdentityClient({
      http: createHttpClient({ baseUrl: 'https://identity.example.test', fetchImpl }),
    });
    await expect(client[name](options)).resolves.toEqual(response);
    expect(fetchImpl.mock.calls[0]?.[0]).toBe(`https://identity.example.test${path}${query}`);
    await client[name]({ ...options, signal });
    expect(fetchImpl.mock.calls[1]?.[1]?.signal).toBe(signal);
  });
}

it('constructs the default cookie-authenticated transport with the configured origin', async () => {
  const fetchImpl = vi.fn<typeof fetch>().mockResolvedValue(Response.json(me));
  vi.stubGlobal('fetch', fetchImpl);
  await expect(
    new AccountIdentityClient({ baseUrl: 'https://custom.example.test' }).getMe(),
  ).resolves.toEqual(me);
  expect(fetchImpl).toHaveBeenCalledWith(
    'https://custom.example.test/auth/me',
    expect.objectContaining({ credentials: 'include' }),
  );
});

it('convenience functions forward OAuth results, identifiers and cancellation', async () => {
  const list = vi.spyOn(accountClient, 'listOAuthClients').mockResolvedValue([oauth]);
  const revoke = vi.spyOn(accountClient, 'revokeOAuthClient').mockResolvedValue();
  await expect(listOAuthClients({ signal })).resolves.toEqual([oauth]);
  expect(list).toHaveBeenCalledExactlyOnceWith({ signal });
  await expect(revokeOAuthClient('client/a')).resolves.toBeUndefined();
  expect(revoke).toHaveBeenCalledExactlyOnceWith('client/a');
});
