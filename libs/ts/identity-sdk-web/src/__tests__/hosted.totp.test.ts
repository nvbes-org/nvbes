import { afterEach, beforeEach, describe, expect, it, vi } from 'vite-plus/test';
import {
  startHostedTotpEnrollment,
  confirmHostedTotpEnrollment,
  stepUpHostedTotp,
} from '../hosted.totp';
import { NvbesIdentityWeb } from '../identity-web.client';

const baseUrl = 'https://identity.example';
const id = '12345678-1234-4234-9234-123456789012';
const secret = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567';
const expires = '2026-09-10T12:00:00Z';
const enrollment = {
  factor_id: id,
  secret_base32: secret,
  expires_at: expires,
  provisioning_uri: `otpauth://totp/nvbes:${id}?secret=${secret}&issuer=nvbes&algorithm=SHA1&digits=6&period=30`,
};
const json = (value: unknown, status = 200) =>
  new Response(JSON.stringify(value), {
    status,
    headers: { 'content-type': 'application/json' },
  });
beforeEach(() => vi.stubGlobal('location', { origin: baseUrl }));
afterEach(() => vi.unstubAllGlobals());

describe('hosted TOTP', () => {
  it('uses active routes through the public client with session CSRF and no persistence', async () => {
    const fetchImpl = vi
      .fn<typeof fetch>()
      .mockResolvedValueOnce(json({ ...enrollment, private_metadata: 'excluded' }))
      .mockResolvedValueOnce(json({ enrolled: true, step_up: true, expires_at: expires }))
      .mockResolvedValueOnce(json({ step_up: true, expires_at: expires }));
    vi.stubGlobal('fetch', fetchImpl);
    const client = new NvbesIdentityWeb({
      baseUrl,
      clientId: 'account',
      redirectUri: 'https://account.example/callback',
      resource: 'https://api.example',
    });
    expect(await client.startTotpEnrollment('csrf')).toEqual({
      factorId: id,
      secretBase32: secret,
      provisioningUri: enrollment.provisioning_uri,
      expiresAt: expires,
    });
    expect(await client.confirmTotpEnrollment('csrf', id, '012345')).toBe(expires);
    expect(await client.stepUpTotp('csrf', '123456')).toBe(expires);
    expect(fetchImpl.mock.calls.map(([url]) => url)).toEqual([
      `${baseUrl}/oauth/session/totp/enrollment/start`,
      `${baseUrl}/oauth/session/totp/enrollment/confirm`,
      `${baseUrl}/oauth/session/step-up/totp`,
    ]);
    expect(
      fetchImpl.mock.calls.map(([, init]) => {
        if (typeof init?.body !== 'string') throw new Error('Expected JSON request body.');
        return JSON.parse(init.body);
      }),
    ).toEqual([{}, { factor_id: id, code: '012345' }, { code: '123456' }]);
    for (const [, init] of fetchImpl.mock.calls) {
      expect(init).toMatchObject({
        method: 'POST',
        credentials: 'same-origin',
        cache: 'no-store',
        redirect: 'error',
      });
      expect(new Headers(init?.headers).get('x-csrf-token')).toBe('csrf');
      expect(new Headers(init?.headers).has('authorization')).toBe(false);
    }
  });

  it('rejects malformed provisioning without leaking sensitive response data', async () => {
    for (const payload of [
      null,
      [],
      {},
      { ...enrollment, factor_id: '../bad' },
      { ...enrollment, expires_at: 'tomorrow' },
      { ...enrollment, secret_base32: secret.toLowerCase() },
      {
        ...enrollment,
        provisioning_uri: enrollment.provisioning_uri.replace(secret, 'A'.repeat(32)),
      },
      { ...enrollment, provisioning_uri: enrollment.provisioning_uri.replace('SHA1', 'SHA256') },
      { ...enrollment, provisioning_uri: `${enrollment.provisioning_uri}&secret=${secret}` },
      { ...enrollment, provisioning_uri: `https://attacker.example/${secret}` },
    ]) {
      const fetchImpl = vi.fn<typeof fetch>().mockResolvedValueOnce(json(payload));
      const error = await startHostedTotpEnrollment({ baseUrl, fetchImpl }, 'csrf').catch(
        (cause: unknown) => cause,
      );
      expect(error).toBeInstanceOf(Error);
      expect(String(error)).not.toContain(secret);
    }
  });

  it('refuses invalid input, absent CSRF and other browser origins before sending', async () => {
    const fetchImpl = vi.fn<typeof fetch>();
    const config = { baseUrl, fetchImpl };
    for (const code of ['', '12345', '1234567', '１２３４５６', '123 45', '12345\n']) {
      await expect(confirmHostedTotpEnrollment(config, 'csrf', id, code)).rejects.toThrow();
      await expect(stepUpHostedTotp(config, 'csrf', code)).rejects.toThrow();
    }
    await expect(
      confirmHostedTotpEnrollment(config, 'csrf', 'invalid', '123456'),
    ).rejects.toThrow();
    await expect(startHostedTotpEnrollment(config, '')).rejects.toThrow();
    vi.stubGlobal('location', { origin: 'https://account.example' });
    await expect(startHostedTotpEnrollment(config, 'csrf')).rejects.toThrow();
    expect(fetchImpl).not.toHaveBeenCalled();
  });

  it('requires explicit enrollment and step-up acknowledgement plus an expiry', async () => {
    const fetchImpl = vi.fn<typeof fetch>();
    const config = { baseUrl, fetchImpl };
    for (const payload of [
      [],
      {},
      { enrolled: true, step_up: false, expires_at: expires },
      { enrolled: 'true', step_up: true, expires_at: expires },
      { enrolled: true, step_up: true, expires_at: 'bad' },
    ]) {
      fetchImpl.mockResolvedValueOnce(json(payload));
      await expect(confirmHostedTotpEnrollment(config, 'csrf', id, '123456')).rejects.toThrow();
    }
    for (const payload of [{ step_up: 'true', expires_at: expires }, { step_up: true }]) {
      fetchImpl.mockResolvedValueOnce(json(payload));
      await expect(stepUpHostedTotp(config, 'csrf', '123456')).rejects.toThrow();
    }
  });

  it('preserves quota, replay and service failures without retry or response-body leakage', async () => {
    for (const status of [400, 403, 429, 503]) {
      const fetchImpl = vi.fn<typeof fetch>().mockResolvedValueOnce(json({ secret }, status));
      await expect(
        stepUpHostedTotp({ baseUrl, fetchImpl }, 'csrf', '123456'),
      ).rejects.toMatchObject({ name: 'HostedIdentityError', status });
      expect(fetchImpl).toHaveBeenCalledTimes(1);
    }
  });
});
