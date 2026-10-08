import { afterEach, beforeEach, describe, expect, it, vi } from 'vite-plus/test';
import { loginHostedSecurityKey } from '../hosted.security-key';
import { getWebAuthnCredential } from '../webauthn.credentials';
vi.mock('../webauthn.credentials', () => ({ getWebAuthnCredential: vi.fn() }));

const baseUrl = 'https://identity.example';
const interaction = {
  interaction: 'request',
  csrfToken: 'first',
  sessionCsrfToken: null,
  needsLogin: true,
  clientId: 'account',
  scope: 'openid account:read',
};
const credentials = { email: 'synthetic@example.test', password: 'synthetic-test-password' };
const login = { interaction: 'request', csrf_token: 'rotated', session_csrf_token: 'session' };
const options = {
  ceremony_id: 'ceremony',
  options: {
    publicKey: {
      challenge: 'AQID',
      rpId: 'identity.example',
      userVerification: 'required',
      allowCredentials: [{ id: 'BAUG', type: 'public-key' }],
    },
  },
};
const json = (value: unknown, status = 200) =>
  new Response(JSON.stringify(value), {
    status,
    headers: { 'content-type': 'application/json' },
  });
beforeEach(() => vi.stubGlobal('location', { origin: baseUrl, hostname: 'identity.example' }));
afterEach(() => {
  vi.unstubAllGlobals();
  vi.resetAllMocks();
});

describe('non-discoverable security key login', () => {
  it('authenticates the password before retrieving credential IDs and returns only after step-up', async () => {
    vi.mocked(getWebAuthnCredential).mockResolvedValueOnce({
      id: 'BAUG',
      rawId: new Uint8Array([4, 5, 6]).buffer,
      type: 'public-key',
      response: { clientDataJSON: new Uint8Array([1]).buffer },
    } as PublicKeyCredential);
    const fetchImpl = vi
      .fn<typeof fetch>()
      .mockResolvedValueOnce(json(login))
      .mockResolvedValueOnce(json(options))
      .mockResolvedValueOnce(json({ step_up: true, expires_at: '2030-01-01T00:00:00Z' }));
    const signal = new AbortController().signal;
    const result = await loginHostedSecurityKey({ baseUrl, fetchImpl }, interaction, credentials, {
      signal,
    });
    expect(result.interaction.csrfToken).toBe('rotated');
    expect(result.stepUpExpiresAt).toBe('2030-01-01T00:00:00Z');
    expect(fetchImpl.mock.calls.map(([url]) => url)).toEqual([
      `${baseUrl}/oauth/authorize/login`,
      `${baseUrl}/oauth/session/webauthn/step-up/options`,
      `${baseUrl}/oauth/session/webauthn/step-up/finish`,
    ]);
    expect(
      fetchImpl.mock.calls.map(([, init]) => new Headers(init?.headers).get('x-csrf-token')),
    ).toEqual(['first', 'session', 'session']);
    expect(vi.mocked(getWebAuthnCredential).mock.calls[0]?.[1]?.signal).toBe(signal);
  });

  it('never requests credentials or logs out an unrelated session after password refusal', async () => {
    const fetchImpl = vi
      .fn<typeof fetch>()
      .mockResolvedValueOnce(json({ error: 'invalid_request' }, 400));
    await expect(
      loginHostedSecurityKey({ baseUrl, fetchImpl }, interaction, credentials),
    ).rejects.toMatchObject({ status: 400 });
    expect(fetchImpl).toHaveBeenCalledTimes(1);
    expect(getWebAuthnCredential).not.toHaveBeenCalled();
  });

  it('revokes the password session on native cancellation without submitting an assertion', async () => {
    vi.mocked(getWebAuthnCredential).mockRejectedValueOnce(
      new DOMException('Cancelled', 'NotAllowedError'),
    );
    const fetchImpl = vi
      .fn<typeof fetch>()
      .mockResolvedValueOnce(json(login))
      .mockResolvedValueOnce(json(options))
      .mockResolvedValueOnce(json({ logged_out: true }));
    await expect(
      loginHostedSecurityKey({ baseUrl, fetchImpl }, interaction, credentials),
    ).rejects.toMatchObject({
      name: 'HostedSecurityKeyLoginError',
      sessionRevocationConfirmed: true,
    });
    expect(fetchImpl.mock.calls.map(([url]) => url)).toEqual([
      `${baseUrl}/oauth/authorize/login`,
      `${baseUrl}/oauth/session/webauthn/step-up/options`,
      `${baseUrl}/oauth/logout`,
    ]);
    expect(new Headers(fetchImpl.mock.calls[2]?.[1]?.headers).get('x-csrf-token')).toBe('session');
  });

  it('does not claim logout succeeded when cleanup fails and never retries', async () => {
    const fetchImpl = vi
      .fn<typeof fetch>()
      .mockResolvedValueOnce(json(login))
      .mockResolvedValueOnce(json({}, 503))
      .mockResolvedValueOnce(json({}, 503));
    await expect(
      loginHostedSecurityKey({ baseUrl, fetchImpl }, interaction, credentials),
    ).rejects.toMatchObject({
      name: 'HostedSecurityKeyLoginError',
      sessionRevocationConfirmed: false,
    });
    expect(fetchImpl).toHaveBeenCalledTimes(3);
    expect(getWebAuthnCredential).not.toHaveBeenCalled();
  });

  it.each([400, 200])(
    'revokes the session on refusal or expired step-up (HTTP %s)',
    async (status) => {
      vi.mocked(getWebAuthnCredential).mockResolvedValueOnce({
        id: 'BAUG',
        rawId: new Uint8Array([4, 5, 6]).buffer,
        type: 'public-key',
        response: { clientDataJSON: new Uint8Array([1]).buffer },
      } as PublicKeyCredential);
      const fetchImpl = vi
        .fn<typeof fetch>()
        .mockResolvedValueOnce(json(login))
        .mockResolvedValueOnce(json(options))
        .mockResolvedValueOnce(json({ step_up: true, expires_at: '2000-01-01T00:00:00Z' }, status))
        .mockResolvedValueOnce(json({ logged_out: true }));
      await expect(
        loginHostedSecurityKey({ baseUrl, fetchImpl }, interaction, credentials),
      ).rejects.toMatchObject({ sessionRevocationConfirmed: true });
      expect(fetchImpl).toHaveBeenCalledTimes(4);
    },
  );
});
