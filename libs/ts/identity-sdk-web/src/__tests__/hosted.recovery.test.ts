import { afterEach, beforeEach, describe, expect, it, vi } from 'vite-plus/test';
import {
  generateHostedRecoveryCodes,
  redeemHostedRecoveryCode,
  completeHostedMfaRecovery,
} from '../hosted.recovery';
import { NvbesIdentityWeb } from '../identity-web.client';
import { createWebAuthnCredential } from '../webauthn.credentials';

vi.mock('../webauthn.credentials', () => ({ createWebAuthnCredential: vi.fn() }));
const baseUrl = 'https://identity.example';
const id = '12345678-1234-4234-9234-123456789012';
const codes = Array.from({ length: 10 }, (_, i) => `nvr1_${String(i).repeat(43)}`);
const recovery = { csrfToken: 'A'.repeat(43), expiresAt: '2030-01-01T00:05:00Z' };
const creation = {
  challenge: 'AQID',
  rp: { id: 'identity.example', name: 'Identity' },
  user: { id: 'BAUG', name: 'user', displayName: 'User' },
  pubKeyCredParams: [{ type: 'public-key', alg: -7 }],
  authenticatorSelection: { residentKey: 'preferred', userVerification: 'required' },
};
const options = { ceremony_id: id, options: { publicKey: creation } };
const complete = { recovered: true, credential_id: id, must_reauthenticate: true };
const json = (value: unknown, status = 200) =>
  new Response(JSON.stringify(value), {
    status,
    headers: { 'content-type': 'application/json' },
  });

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] });
  vi.setSystemTime(new Date('2030-01-01T00:00:00Z'));
  vi.stubGlobal('location', { origin: baseUrl, hostname: 'identity.example' });
  vi.mocked(createWebAuthnCredential).mockResolvedValue({
    id: 'AQID',
    type: 'public-key',
    rawId: new Uint8Array([1, 2, 3]).buffer,
    response: { clientDataJSON: new Uint8Array([4, 5, 6]).buffer },
  } as PublicKeyCredential);
});
afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
  vi.resetAllMocks();
});

describe('hosted MFA recovery', () => {
  it('uses distinct proofs through the client and requires reauthentication without persisting secrets', async () => {
    const fetchImpl = vi
      .fn<typeof fetch>()
      .mockResolvedValueOnce(json({ codes }))
      .mockResolvedValueOnce(
        json({
          recovery: true,
          csrf_token: recovery.csrfToken,
          expires_at: recovery.expiresAt,
          token: 'must-not-escape',
        }),
      )
      .mockResolvedValueOnce(json(options))
      .mockResolvedValueOnce(json(complete));
    vi.stubGlobal('fetch', fetchImpl);
    const client = new NvbesIdentityWeb({
      baseUrl,
      clientId: 'account',
      redirectUri: 'https://account.example/callback',
      resource: 'https://api.example',
    });
    const clear = vi.spyOn(client, 'clearAuthorizationTransaction');
    expect(await client.generateRecoveryCodes('session-proof')).toEqual(codes);
    expect(await client.redeemRecoveryCode('session-proof', codes[0]!)).toEqual(recovery);
    expect(clear).toHaveBeenCalledTimes(1);
    const signal = new AbortController().signal;
    expect(await client.completeMfaRecovery(recovery, 'Replacement', { signal })).toEqual({
      credentialId: id,
      mustReauthenticate: true,
    });
    expect(vi.mocked(createWebAuthnCredential).mock.calls[0]?.[1]?.signal).toBe(signal);
    expect(fetchImpl.mock.calls.map(([url]) => url)).toEqual([
      `${baseUrl}/oauth/session/recovery/codes/generate`,
      `${baseUrl}/oauth/session/recovery/redeem`,
      `${baseUrl}/oauth/recovery/registration/options`,
      `${baseUrl}/oauth/recovery/registration/finish`,
    ]);
    expect(
      fetchImpl.mock.calls.map(([, init]) => new Headers(init?.headers).get('x-csrf-token')),
    ).toEqual(['session-proof', 'session-proof', recovery.csrfToken, recovery.csrfToken]);
    for (const [, init] of fetchImpl.mock.calls) {
      expect(init).toMatchObject({
        method: 'POST',
        credentials: 'same-origin',
        redirect: 'error',
        cache: 'no-store',
      });
      expect(new Headers(init?.headers).has('authorization')).toBe(false);
    }
    const body = fetchImpl.mock.calls[3]?.[1]?.body;
    if (typeof body !== 'string') throw new Error('Expected JSON request body.');
    expect(JSON.parse(body)).toMatchObject({
      ceremony_id: id,
      label: 'Replacement',
      credential: { id: 'AQID', rawId: 'AQID' },
    });
  });

  it('rejects incomplete or duplicate code batches without leaking them in errors', async () => {
    for (const values of [
      null,
      [],
      codes.slice(1),
      [...codes, codes[0]],
      Array(10).fill(codes[0]),
      [...codes.slice(1), 'invalid-secret'],
    ]) {
      const fetchImpl = vi.fn<typeof fetch>().mockResolvedValueOnce(json({ codes: values }));
      const error = await generateHostedRecoveryCodes({ baseUrl, fetchImpl }, 'csrf').catch(
        (cause: unknown) => cause,
      );
      expect(error).toBeInstanceOf(Error);
      expect(String(error)).not.toContain(codes[0]);
      expect(String(error)).not.toContain('invalid-secret');
    }
  });

  it('rejects foreign origins, invalid input and expired sessions before sending or opening WebAuthn', async () => {
    const fetchImpl = vi.fn<typeof fetch>();
    const config = { baseUrl, fetchImpl };
    for (const code of ['', 'old-code', `${codes[0]}\n`, 'nvr1_' + 'é'.repeat(43)])
      await expect(redeemHostedRecoveryCode(config, 'csrf', code)).rejects.toThrow();
    for (const value of [
      { ...recovery, csrfToken: '' },
      { ...recovery, expiresAt: '2030-01-01T00:00:00Z' },
    ])
      await expect(completeHostedMfaRecovery(config, value, 'Key')).rejects.toThrow();
    await expect(completeHostedMfaRecovery(config, recovery, '')).rejects.toThrow();
    await expect(generateHostedRecoveryCodes(config, '')).rejects.toThrow();
    vi.stubGlobal('location', { origin: 'https://account.example' });
    await expect(generateHostedRecoveryCodes(config, 'csrf')).rejects.toThrow();
    await expect(redeemHostedRecoveryCode(config, 'csrf', codes[0]!)).rejects.toThrow();
    await expect(completeHostedMfaRecovery(config, recovery, 'Key')).rejects.toThrow();
    expect(fetchImpl).not.toHaveBeenCalled();
    expect(createWebAuthnCredential).not.toHaveBeenCalled();
  });

  it('requires explicit recovery confirmation, a distinct proof and a future expiry', async () => {
    for (const payload of [
      null,
      [],
      {},
      { recovery: 'true' },
      { recovery: true, csrf_token: 'bad', expires_at: recovery.expiresAt },
      { recovery: true, csrf_token: recovery.csrfToken, expires_at: 'yesterday' },
    ]) {
      const fetchImpl = vi.fn<typeof fetch>().mockResolvedValueOnce(json(payload));
      await expect(
        redeemHostedRecoveryCode({ baseUrl, fetchImpl }, 'csrf', codes[0]!),
      ).rejects.toThrow();
    }
    for (const result of [
      { ...complete, recovered: false },
      { ...complete, must_reauthenticate: false },
      { ...complete, credential_id: 'bad' },
    ]) {
      const fetchImpl = vi
        .fn<typeof fetch>()
        .mockResolvedValueOnce(json(options))
        .mockResolvedValueOnce(json(result));
      await expect(
        completeHostedMfaRecovery({ baseUrl, fetchImpl }, recovery, 'Key'),
      ).rejects.toThrow();
    }
  });

  it('does not finish after cancellation, expiry during the prompt or hostile RP options', async () => {
    for (const mode of ['cancel', 'expire', 'hostile']) {
      const payload =
        mode === 'hostile'
          ? {
              ...options,
              options: { publicKey: { ...creation, rp: { id: 'other.example', name: 'Other' } } },
            }
          : options;
      const fetchImpl = vi.fn<typeof fetch>().mockResolvedValueOnce(json(payload));
      if (mode === 'cancel')
        vi.mocked(createWebAuthnCredential).mockRejectedValueOnce(
          new DOMException('Cancelled', 'NotAllowedError'),
        );
      if (mode === 'expire')
        vi.mocked(createWebAuthnCredential).mockImplementationOnce(async () => {
          vi.setSystemTime(new Date(recovery.expiresAt));
          return {} as PublicKeyCredential;
        });
      await expect(
        completeHostedMfaRecovery({ baseUrl, fetchImpl }, recovery, 'Key'),
      ).rejects.toThrow();
      expect(fetchImpl).toHaveBeenCalledTimes(1);
      vi.setSystemTime(new Date('2030-01-01T00:00:00Z'));
    }
  });

  it('propagates refusal, quota and store failures without retry or secret leakage', async () => {
    for (const status of [400, 403, 429, 503]) {
      const fetchImpl = vi
        .fn<typeof fetch>()
        .mockResolvedValueOnce(json({ code: codes[0] }, status));
      const error = await redeemHostedRecoveryCode({ baseUrl, fetchImpl }, 'csrf', codes[0]!).catch(
        (cause: unknown) => cause,
      );
      expect(error).toMatchObject({ name: 'HostedIdentityError', status });
      expect(String(error)).not.toContain(codes[0]);
      expect(fetchImpl).toHaveBeenCalledTimes(1);
    }
  });
});
