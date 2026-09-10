import { afterEach, beforeEach, describe, expect, it, vi } from 'vite-plus/test';
import { registerHostedPasskey, loginHostedPasskey, stepUpHostedPasskey } from '../hosted.webauthn';
import { hostedCreationOptions, hostedRequestOptions } from '../hosted.webauthn.options';
import { createWebAuthnCredential, getWebAuthnCredential } from '../webauthn.credentials';
import { record } from '../hosted.transport';

vi.mock('../webauthn.credentials', () => ({
  createWebAuthnCredential: vi.fn(),
  getWebAuthnCredential: vi.fn(),
}));

const baseUrl = 'https://identity.example';
const request = { challenge: 'AQID', rpId: 'identity.example', userVerification: 'required' };
const creation = {
  challenge: 'AQID',
  rp: { id: 'identity.example', name: 'Identity' },
  user: { id: 'BAUG', name: 'user', displayName: 'User' },
  pubKeyCredParams: [{ type: 'public-key', alg: -7 }],
  authenticatorSelection: { residentKey: 'preferred', userVerification: 'required' },
};
const interaction = {
  interaction: 'transaction',
  csrfToken: 'interaction-proof',
  sessionCsrfToken: null,
  needsLogin: true,
  clientId: 'account-web',
  scope: 'openid account:read',
};
const json = (value: unknown, status = 200) =>
  new Response(JSON.stringify(value), {
    status,
    headers: { 'content-type': 'application/json' },
  });

function requestBody(init: RequestInit | undefined): Record<string, unknown> {
  if (typeof init?.body !== 'string') throw new Error('Expected JSON request body');
  return record(JSON.parse(init.body));
}
// Only the native authenticator is substituted: conversion and HTTP transport run unchanged.
const credential = {
  id: 'AQID',
  type: 'public-key',
  rawId: new Uint8Array([1, 2, 3]).buffer,
  response: { clientDataJSON: new Uint8Array([4, 5, 6]).buffer },
} as PublicKeyCredential;

beforeEach(() => {
  vi.stubGlobal('location', { origin: baseUrl, hostname: 'identity.example' });
  vi.mocked(createWebAuthnCredential).mockResolvedValue(credential);
  vi.mocked(getWebAuthnCredential).mockResolvedValue(credential);
});
afterEach(() => {
  vi.unstubAllGlobals();
  vi.resetAllMocks();
});

describe('hosted WebAuthn', () => {
  it('decodes challenges and credential descriptors while preserving required UV', () => {
    const parsed = hostedCreationOptions({
      publicKey: {
        ...creation,
        excludeCredentials: [{ type: 'public-key', id: 'BwgJ', transports: ['usb'] }],
      },
    });
    expect(new Uint8Array(parsed.challenge as ArrayBuffer)).toEqual(new Uint8Array([1, 2, 3]));
    expect(new Uint8Array(parsed.user.id as ArrayBuffer)).toEqual(new Uint8Array([4, 5, 6]));
    expect(parsed.authenticatorSelection?.userVerification).toBe('required');
    expect(new Uint8Array(parsed.excludeCredentials![0]!.id as ArrayBuffer)).toEqual(
      new Uint8Array([7, 8, 9]),
    );
    expect(hostedRequestOptions({ publicKey: request }).rpId).toBe('identity.example');
  });

  it('rejects malformed options, another RP and weakened user verification', () => {
    for (const publicKey of [
      null,
      {},
      { ...creation, rp: { id: 'other.example', name: 'Other' } },
      { ...creation, authenticatorSelection: { userVerification: 'preferred' } },
      { ...creation, pubKeyCredParams: [{ type: 'password', alg: -7 }] },
      { ...creation, challenge: '' },
      { ...creation, excludeCredentials: [null] },
      { ...creation, authenticatorSelection: { userVerification: 'required', residentKey: [] } },
    ])
      expect(() => hostedCreationOptions({ publicKey })).toThrow();
    for (const publicKey of [
      { ...request, rpId: 'other.example' },
      { ...request, userVerification: 'preferred' },
      { ...request, allowCredentials: [{}] },
      { ...request, challenge: '?' },
    ])
      expect(() => hostedRequestOptions({ publicKey })).toThrow();
  });

  it('registers with session CSRF and the exact ceremony returned by Identity', async () => {
    const fetchImpl = vi
      .fn<typeof fetch>()
      .mockResolvedValueOnce(json({ ceremony_id: 'ceremony', options: { publicKey: creation } }))
      .mockResolvedValueOnce(json({ credential_id: 'registered-id' }));
    const signal = new AbortController().signal;
    expect(
      await registerHostedPasskey({ baseUrl, fetchImpl }, 'session-proof', 'Key', { signal }),
    ).toBe('registered-id');
    expect(vi.mocked(createWebAuthnCredential).mock.calls[0]?.[1]?.signal).toBe(signal);
    expect(
      fetchImpl.mock.calls.map(([url]) => (url instanceof Request ? url.url : url.toString())),
    ).toEqual([
      `${baseUrl}/oauth/session/webauthn/registration/options`,
      `${baseUrl}/oauth/session/webauthn/registration/finish`,
    ]);
    for (const [, init] of fetchImpl.mock.calls) {
      expect(new Headers(init?.headers).get('x-csrf-token')).toBe('session-proof');
      expect(init?.credentials).toBe('same-origin');
    }
    expect(requestBody(fetchImpl.mock.calls[1]?.[1])).toEqual({
      ceremony_id: 'ceremony',
      label: 'Key',
      credential: {
        id: 'AQID',
        rawId: 'AQID',
        type: 'public-key',
        response: { clientDataJSON: 'BAUG' },
      },
    });
  });

  it('rotates interaction/session proofs only after a matching passkey login', async () => {
    const fetchImpl = vi
      .fn<typeof fetch>()
      .mockResolvedValueOnce(
        json({ ceremony_id: 'login-ceremony', options: { publicKey: request } }),
      )
      .mockResolvedValueOnce(
        json({ interaction: 'transaction', csrf_token: 'rotated', session_csrf_token: 'session' }),
      );
    expect(await loginHostedPasskey({ baseUrl, fetchImpl }, interaction)).toEqual({
      ...interaction,
      needsLogin: false,
      csrfToken: 'rotated',
      sessionCsrfToken: 'session',
    });
    expect(interaction.needsLogin).toBe(true);
    for (const [, init] of fetchImpl.mock.calls) {
      expect(new Headers(init?.headers).get('x-csrf-token')).toBe('interaction-proof');
      expect(requestBody(init).interaction).toBe('transaction');
    }
    fetchImpl
      .mockResolvedValueOnce(json({ ceremony_id: 'next', options: { publicKey: request } }))
      .mockResolvedValueOnce(json({ interaction: 'other' }));
    await expect(loginHostedPasskey({ baseUrl, fetchImpl }, interaction)).rejects.toThrow(
      'mismatch',
    );
  });

  it('requires an explicit successful step-up result and valid expiry', async () => {
    for (const result of [
      { step_up: false, expires_at: '2030-01-01T00:00:00Z' },
      { step_up: true, expires_at: 'invalid' },
    ]) {
      const fetchImpl = vi
        .fn<typeof fetch>()
        .mockResolvedValueOnce(json({ ceremony_id: 'step-up', options: { publicKey: request } }))
        .mockResolvedValueOnce(json(result));
      await expect(stepUpHostedPasskey({ baseUrl, fetchImpl }, 'session')).rejects.toThrow(
        'did not confirm',
      );
    }
    const fetchImpl = vi
      .fn<typeof fetch>()
      .mockResolvedValueOnce(json({ ceremony_id: 'step-up', options: { publicKey: request } }))
      .mockResolvedValueOnce(json({ step_up: true, expires_at: '2030-01-01T00:00:00Z' }));
    expect(await stepUpHostedPasskey({ baseUrl, fetchImpl }, 'session')).toBe(
      '2030-01-01T00:00:00Z',
    );
  });

  it('does not finish or retry a cancelled native ceremony', async () => {
    const fetchImpl = vi
      .fn<typeof fetch>()
      .mockResolvedValueOnce(json({ ceremony_id: 'login', options: { publicKey: request } }));
    vi.mocked(getWebAuthnCredential).mockRejectedValueOnce(
      new DOMException('Cancelled', 'NotAllowedError'),
    );
    await expect(loginHostedPasskey({ baseUrl, fetchImpl }, interaction)).rejects.toThrow(
      'Cancelled',
    );
    expect(fetchImpl).toHaveBeenCalledTimes(1);
  });

  it('refuses cross-origin enrollment and invalid labels without opening an authenticator', async () => {
    const fetchImpl = vi.fn<typeof fetch>();
    for (const label of ['', 'x'.repeat(129), 'key\u0085'])
      await expect(
        registerHostedPasskey({ baseUrl, fetchImpl }, 'session', label),
      ).rejects.toThrow();
    vi.stubGlobal('location', { origin: 'https://account.example' });
    await expect(registerHostedPasskey({ baseUrl, fetchImpl }, 'session', 'Key')).rejects.toThrow(
      'Identity browser origin',
    );
    expect(fetchImpl).not.toHaveBeenCalled();
    expect(createWebAuthnCredential).not.toHaveBeenCalled();
  });
});
