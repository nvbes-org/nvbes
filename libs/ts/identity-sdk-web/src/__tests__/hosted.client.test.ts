import { afterEach, beforeEach, describe, expect, it, vi } from 'vite-plus/test';
import {
  completeHostedConsent,
  loadHostedAuthorization,
  loginHostedPassword,
  logoutHostedSession,
  parseHostedInteraction,
  HostedIdentityError,
} from '../hosted.client';
import { NvbesIdentityWeb } from '../identity-web.client';

const baseUrl = 'https://identity.example';
const initial = {
  interaction: 'interaction',
  csrf_token: 'interaction-proof',
  session_csrf_token: null,
  needs_login: true,
  client_id: 'account-web',
  scope: 'openid account:read',
};
const json = (value: unknown, status = 200) =>
  new Response(JSON.stringify(value), {
    status,
    headers: { 'content-type': 'application/json' },
  });

beforeEach(() => vi.stubGlobal('location', { origin: baseUrl }));
afterEach(() => vi.unstubAllGlobals());

describe('hosted Identity client', () => {
  it('does not expose malformed response contents in parsing errors', async () => {
    const fetchImpl = vi.fn<typeof fetch>().mockResolvedValueOnce(
      new Response('secret-response-value', {
        headers: { 'content-type': 'application/json' },
      }),
    );
    await expect(logoutHostedSession({ baseUrl, fetchImpl }, 'proof')).rejects.toThrow(
      'Invalid hosted Identity response.',
    );
  });
  it('uses the active endpoints and rotated interaction proof with bounded same-origin requests', async () => {
    const fetchImpl = vi
      .fn<typeof fetch>()
      .mockResolvedValueOnce(json(initial))
      .mockResolvedValueOnce(
        json({
          interaction: initial.interaction,
          csrf_token: 'rotated-proof',
          session_csrf_token: 'session-proof',
        }),
      )
      .mockResolvedValueOnce(
        json({ redirect_uri: 'https://account.example/callback?code=opaque&state=bound' }),
      )
      .mockResolvedValueOnce(json({ logged_out: true }));
    const config = { baseUrl, fetchImpl };
    const interaction = await loadHostedAuthorization(
      config,
      `${baseUrl}/oauth/authorize?client_id=account-web&request_uri=opaque`,
    );
    const authenticated = await loginHostedPassword(config, interaction, {
      email: 'user@example.invalid',
      password: 'password',
    });
    expect(authenticated.needsLogin).toBe(false);
    expect(interaction.needsLogin).toBe(true);
    expect(await completeHostedConsent(config, authenticated, 'approve')).toContain(
      'https://account.example/callback',
    );
    await logoutHostedSession(config, authenticated.sessionCsrfToken!);
    const calls = fetchImpl.mock.calls;
    expect(calls.map(([url]) => new URL(url instanceof Request ? url.url : url).pathname)).toEqual([
      '/oauth/authorize',
      '/oauth/authorize/login',
      '/oauth/authorize/approve',
      '/oauth/logout',
    ]);
    for (const [, init] of calls) {
      expect(init).toMatchObject({
        credentials: 'same-origin',
        redirect: 'error',
        cache: 'no-store',
      });
      expect(init?.signal).toBeInstanceOf(AbortSignal);
      expect(new Headers(init?.headers).get('accept')).toBe('application/json');
    }
    expect(new Headers(calls[2]?.[1]?.headers).get('x-csrf-token')).toBe('rotated-proof');
    expect(new Headers(calls[3]?.[1]?.headers).get('x-csrf-token')).toBe('session-proof');
  });

  it('refuses cross-origin password and cookie requests before fetching', async () => {
    vi.stubGlobal('location', { origin: 'https://account.example' });
    const fetchImpl = vi.fn<typeof fetch>();
    const config = { baseUrl, fetchImpl };
    await expect(
      loginHostedPassword(config, parseHostedInteraction(initial), { email: 'u', password: 'p' }),
    ).rejects.toThrow('Identity browser origin');
    await expect(logoutHostedSession(config, 'proof')).rejects.toThrow('Identity browser origin');
    expect(fetchImpl).not.toHaveBeenCalled();
  });

  it('refuses malformed authorization URLs and interaction payloads', async () => {
    const fetchImpl = vi.fn<typeof fetch>();
    for (const url of [
      'https://attacker.example/oauth/authorize',
      `${baseUrl}/oauth/logout`,
      `${baseUrl}/oauth/authorize#fragment`,
    ]) {
      await expect(loadHostedAuthorization({ baseUrl, fetchImpl }, url)).rejects.toThrow();
    }
    expect(fetchImpl).not.toHaveBeenCalled();
    for (const payload of [
      null,
      [],
      { ...initial, needs_login: 'false' },
      { ...initial, csrf_token: '' },
    ])
      expect(() => parseHostedInteraction(payload)).toThrow();
  });

  it('refuses a mismatched login interaction and unsafe navigation destinations', async () => {
    const fetchImpl = vi.fn<typeof fetch>().mockResolvedValueOnce(json({ interaction: 'other' }));
    await expect(
      loginHostedPassword({ baseUrl, fetchImpl }, parseHostedInteraction(initial), {
        email: 'u',
        password: 'p',
      }),
    ).rejects.toThrow('mismatch');
    for (const redirect_uri of [
      'javascript:alert(1)',
      'https://user:pass@account.example/callback',
      'http://account.example/callback',
      'https://account.example/callback#token',
    ]) {
      fetchImpl.mockResolvedValueOnce(json({ redirect_uri }));
      await expect(
        completeHostedConsent({ baseUrl, fetchImpl }, parseHostedInteraction(initial), 'deny'),
      ).rejects.toThrow();
    }
    fetchImpl.mockResolvedValueOnce(
      json({ redirect_uri: 'https://account.example/callback?error=access_denied' }),
    );
    expect(
      await completeHostedConsent({ baseUrl, fetchImpl }, parseHostedInteraction(initial), 'deny'),
    ).toContain('access_denied');
  });

  it('rejects failure, malformed media and oversized streamed responses without retry', async () => {
    for (const response of [
      json({}, 503),
      new Response('<html>error</html>'),
      json({ logged_out: true, extra: 'x'.repeat(65_536) }),
    ]) {
      const fetchImpl = vi.fn<typeof fetch>().mockResolvedValueOnce(response);
      await expect(logoutHostedSession({ baseUrl, fetchImpl }, 'proof')).rejects.toThrow();
      expect(fetchImpl).toHaveBeenCalledTimes(1);
    }
  });

  it('never treats a rejected or unconfirmed legacy-wrapper logout as successful', async () => {
    const client = new NvbesIdentityWeb({
      baseUrl,
      clientId: 'account-web',
      redirectUri: 'https://account.example/callback',
      resource: 'https://account-api.example',
    });
    const fetchImpl = vi
      .fn<typeof fetch>()
      .mockResolvedValueOnce(json({}, 403))
      .mockResolvedValueOnce(json({ logged_out: false }));
    vi.stubGlobal('fetch', fetchImpl);
    await expect(client.logout()).rejects.toThrow('explicit session CSRF');
    expect(fetchImpl).not.toHaveBeenCalled();
    await expect(client.logout('proof')).rejects.toBeInstanceOf(HostedIdentityError);
    await expect(client.logout('proof')).rejects.toThrow('did not confirm');
  });
});
