import { afterEach, beforeEach, describe, expect, it, vi } from 'vite-plus/test';
import { listHostedTotpFactors, revokeHostedTotpFactor } from '../hosted.totp.management';
import { NvbesIdentityWeb } from '../identity-web.client';
const baseUrl = 'https://identity.example';
const id = '12345678-1234-4234-9234-123456789012';
const row = { id, created_at: '2026-09-10T12:00:00Z' };
const json = (value: unknown, status = 200) =>
  new Response(JSON.stringify(value), { status, headers: { 'content-type': 'application/json' } });
beforeEach(() => vi.stubGlobal('location', { origin: baseUrl }));
afterEach(() => vi.unstubAllGlobals());
describe('hosted TOTP management', () => {
  it('maps only public metadata and confirms all-session revocation through the client', async () => {
    const fetchImpl = vi
      .fn<typeof fetch>()
      .mockResolvedValueOnce(json([{ ...row, secret_base32: 'excluded' }]))
      .mockResolvedValueOnce(json({ revoked: true, sessions_revoked: true }));
    vi.stubGlobal('fetch', fetchImpl);
    const client = new NvbesIdentityWeb({
      baseUrl,
      clientId: 'account',
      redirectUri: 'https://account.example/callback',
      resource: 'https://api.example',
    });
    expect(await client.listTotpFactors('csrf')).toEqual([{ id, createdAt: row.created_at }]);
    await client.revokeTotpFactor('csrf', id);
    expect(fetchImpl.mock.calls[0]?.[0]).toBe(`${baseUrl}/oauth/session/totp/factors/list`);
    expect(fetchImpl.mock.calls[1]?.[0]).toBe(`${baseUrl}/oauth/session/totp/factors/revoke`);
    expect(fetchImpl.mock.calls[1]?.[1]?.body).toBe(JSON.stringify({ factor_id: id }));
    for (const [, init] of fetchImpl.mock.calls) {
      expect(init).toMatchObject({
        method: 'POST',
        credentials: 'same-origin',
        cache: 'no-store',
        redirect: 'error',
      });
      expect(new Headers(init?.headers).get('x-csrf-token')).toBe('csrf');
    }
  });
  it('accepts an empty list and rejects malformed metadata or multiple factors', async () => {
    const fetchImpl = vi.fn<typeof fetch>().mockResolvedValueOnce(json([]));
    expect(await listHostedTotpFactors({ baseUrl, fetchImpl }, 'csrf')).toEqual([]);
    for (const payload of [
      {},
      [null],
      [row, row],
      [{ ...row, id: 'invalid' }],
      [{ ...row, created_at: 'invalid' }],
    ]) {
      fetchImpl.mockResolvedValueOnce(json(payload));
      await expect(listHostedTotpFactors({ baseUrl, fetchImpl }, 'csrf')).rejects.toThrow();
    }
  });
  it('requires both mutation acknowledgements and preserves last-factor conflicts without retry', async () => {
    const fetchImpl = vi.fn<typeof fetch>();
    for (const payload of [
      { revoked: true },
      { sessions_revoked: true },
      { revoked: true, sessions_revoked: 'true' },
    ]) {
      fetchImpl.mockResolvedValueOnce(json(payload));
      await expect(revokeHostedTotpFactor({ baseUrl, fetchImpl }, 'csrf', id)).rejects.toThrow(
        'did not confirm',
      );
    }
    fetchImpl.mockResolvedValueOnce(json({ error: 'last_strong_factor' }, 409));
    await expect(revokeHostedTotpFactor({ baseUrl, fetchImpl }, 'csrf', id)).rejects.toMatchObject({
      status: 409,
    });
    expect(fetchImpl).toHaveBeenCalledTimes(4);
  });
  it('rejects invalid identifiers, CSRF and cross-origin callers before fetch', async () => {
    const fetchImpl = vi.fn<typeof fetch>();
    await expect(
      revokeHostedTotpFactor({ baseUrl, fetchImpl }, 'csrf', '../other'),
    ).rejects.toThrow();
    await expect(listHostedTotpFactors({ baseUrl, fetchImpl }, '')).rejects.toThrow();
    vi.stubGlobal('location', { origin: 'https://account.example' });
    await expect(listHostedTotpFactors({ baseUrl, fetchImpl }, 'csrf')).rejects.toThrow();
    expect(fetchImpl).not.toHaveBeenCalled();
  });
});
