import { afterEach, beforeEach, describe, expect, it, vi } from 'vite-plus/test';
import { resumeHostedMfaRecovery, cancelHostedMfaRecovery } from '../hosted.recovery';
import { NvbesIdentityWeb } from '../identity-web.client';
const baseUrl = 'https://identity.example';
const recovery = { csrfToken: 'A'.repeat(43), expiresAt: '2030-01-01T00:05:00Z' };
const json = (body: unknown, status = 200) =>
  new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' },
  });
beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] });
  vi.setSystemTime(new Date('2030-01-01T00:00:00Z'));
  vi.stubGlobal('location', { origin: baseUrl });
});
afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
});
describe('recovery lifecycle', () => {
  it('bootstraps without a saved proof and cancels with the returned proof through the client', async () => {
    const fetchImpl = vi
      .fn<typeof fetch>()
      .mockResolvedValueOnce(
        json({ recovery: true, csrf_token: recovery.csrfToken, expires_at: recovery.expiresAt }),
      )
      .mockResolvedValueOnce(json({ cancelled: true, must_reauthenticate: true }));
    vi.stubGlobal('fetch', fetchImpl);
    const client = new NvbesIdentityWeb({
      baseUrl,
      clientId: 'account',
      redirectUri: 'https://account.example/callback',
      resource: 'https://api.example',
    });
    expect(await client.resumeMfaRecovery()).toEqual(recovery);
    await client.cancelMfaRecovery(recovery);
    expect(fetchImpl.mock.calls.map(([url]) => url)).toEqual([
      `${baseUrl}/oauth/recovery/resume`,
      `${baseUrl}/oauth/recovery/cancel`,
    ]);
    const first = new Headers(fetchImpl.mock.calls[0]?.[1]?.headers).get('x-csrf-token');
    expect(first).toMatch(/^[A-Za-z0-9_-]{43}$/);
    expect(first).not.toBe(recovery.csrfToken);
    expect(new Headers(fetchImpl.mock.calls[1]?.[1]?.headers).get('x-csrf-token')).toBe(
      recovery.csrfToken,
    );
  });
  it('refuses other origins and expired cancellation proofs before sending', async () => {
    const fetchImpl = vi.fn<typeof fetch>();
    await expect(
      cancelHostedMfaRecovery(
        { baseUrl, fetchImpl },
        { ...recovery, expiresAt: '2029-01-01T00:00:00Z' },
      ),
    ).rejects.toThrow();
    vi.stubGlobal('location', { origin: 'https://account.example' });
    await expect(resumeHostedMfaRecovery({ baseUrl, fetchImpl })).rejects.toThrow();
    await expect(cancelHostedMfaRecovery({ baseUrl, fetchImpl }, recovery)).rejects.toThrow();
    expect(fetchImpl).not.toHaveBeenCalled();
  });
  it('requires explicit acknowledgement and refuses stale bootstrap state', async () => {
    const fetchImpl = vi.fn<typeof fetch>();
    for (const body of [
      null,
      {},
      { recovery: true, csrf_token: recovery.csrfToken, expires_at: '2029-01-01T00:00:00Z' },
    ]) {
      fetchImpl.mockResolvedValueOnce(json(body));
      await expect(resumeHostedMfaRecovery({ baseUrl, fetchImpl })).rejects.toThrow();
    }
    for (const body of [{ cancelled: true }, { cancelled: true, must_reauthenticate: false }]) {
      fetchImpl.mockResolvedValueOnce(json(body));
      await expect(cancelHostedMfaRecovery({ baseUrl, fetchImpl }, recovery)).rejects.toThrow();
    }
  });
  it('does not retry an unknown cancellation result or return an inactive recovery session', async () => {
    for (const status of [400, 403, 429, 503]) {
      const fetchImpl = vi.fn<typeof fetch>().mockResolvedValueOnce(json({}, status));
      await expect(resumeHostedMfaRecovery({ baseUrl, fetchImpl })).rejects.toMatchObject({
        status,
      });
      expect(fetchImpl).toHaveBeenCalledTimes(1);
      fetchImpl.mockResolvedValueOnce(json({}, status));
      await expect(cancelHostedMfaRecovery({ baseUrl, fetchImpl }, recovery)).rejects.toMatchObject(
        { status },
      );
      expect(fetchImpl).toHaveBeenCalledTimes(2);
    }
  });
});
