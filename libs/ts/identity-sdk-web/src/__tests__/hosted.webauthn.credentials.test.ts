import { afterEach, beforeEach, describe, expect, it, vi } from 'vite-plus/test';
import {
  listHostedPasskeys,
  renameHostedPasskey,
  revokeHostedPasskey,
} from '../hosted.webauthn.credentials';
import { HostedIdentityError } from '../hosted.transport';

const baseUrl = 'https://identity.example';
const id = '12345678-1234-4234-9234-123456789012';
const row = { id, label: 'My key', created_at: '2026-09-10T10:00:00Z', last_used_at: null };
const json = (value: unknown, status = 200) =>
  new Response(JSON.stringify(value), {
    status,
    headers: { 'content-type': 'application/json' },
  });
beforeEach(() => vi.stubGlobal('location', { origin: baseUrl }));
afterEach(() => vi.unstubAllGlobals());

describe('hosted passkey management', () => {
  it('maps only owned metadata and protects even the list with session CSRF', async () => {
    const fetchImpl = vi
      .fn<typeof fetch>()
      .mockResolvedValueOnce(json([{ ...row, secret: 'not-exposed' }]))
      .mockResolvedValueOnce(json({ renamed: true }))
      .mockResolvedValueOnce(json({ revoked: true }));
    const config = { baseUrl, fetchImpl };
    expect(await listHostedPasskeys(config, 'session-proof')).toEqual([
      { id, label: row.label, createdAt: row.created_at, lastUsedAt: null },
    ]);
    await renameHostedPasskey(config, 'session-proof', id, 'Renamed');
    await revokeHostedPasskey(config, 'session-proof', id);
    expect(
      fetchImpl.mock.calls.map(([url]) => (url instanceof Request ? url.url : url.toString())),
    ).toEqual([
      `${baseUrl}/oauth/session/webauthn/credentials/list`,
      `${baseUrl}/oauth/session/webauthn/credentials/rename`,
      `${baseUrl}/oauth/session/webauthn/credentials/revoke`,
    ]);
    for (const [, init] of fetchImpl.mock.calls) {
      expect(init).toMatchObject({
        method: 'POST',
        credentials: 'same-origin',
        redirect: 'error',
        cache: 'no-store',
      });
      expect(new Headers(init?.headers).get('x-csrf-token')).toBe('session-proof');
    }
    expect(fetchImpl.mock.calls[1]?.[1]?.body).toBe(
      JSON.stringify({ credential_id: id, label: 'Renamed' }),
    );
    expect(fetchImpl.mock.calls[2]?.[1]?.body).toBe(JSON.stringify({ credential_id: id }));
  });

  it('accepts an empty list and validates all entries before exposing the list', async () => {
    const fetchImpl = vi.fn<typeof fetch>().mockResolvedValueOnce(json([]));
    expect(await listHostedPasskeys({ baseUrl, fetchImpl }, 'proof')).toEqual([]);
    for (const payload of [
      {},
      [row, row],
      Array.from({ length: 11 }, () => row),
      [null],
      [{ ...row, id: '../other' }],
      [{ ...row, created_at: 'bad' }],
      [{ ...row, last_used_at: undefined }],
      [{ ...row, label: 'x'.repeat(129) }],
    ]) {
      fetchImpl.mockResolvedValueOnce(json(payload));
      await expect(listHostedPasskeys({ baseUrl, fetchImpl }, 'proof')).rejects.toThrow();
    }
  });

  it('preserves last-factor conflicts and session failures without retry or success', async () => {
    for (const status of [409, 401, 403, 503]) {
      const fetchImpl = vi
        .fn<typeof fetch>()
        .mockResolvedValueOnce(json({ error: 'last_strong_factor' }, status));
      await expect(revokeHostedPasskey({ baseUrl, fetchImpl }, 'proof', id)).rejects.toMatchObject({
        name: 'HostedIdentityError',
        status,
      });
      expect(fetchImpl).toHaveBeenCalledTimes(1);
    }
  });

  it('requires positive mutation acknowledgement', async () => {
    const fetchImpl = vi
      .fn<typeof fetch>()
      .mockResolvedValueOnce(json({ renamed: false }))
      .mockResolvedValueOnce(json({ revoked: 'true' }));
    await expect(renameHostedPasskey({ baseUrl, fetchImpl }, 'proof', id, 'Key')).rejects.toThrow(
      'did not confirm',
    );
    await expect(revokeHostedPasskey({ baseUrl, fetchImpl }, 'proof', id)).rejects.toThrow(
      'did not confirm',
    );
  });

  it('rejects invalid identifiers, labels and cross-origin requests before sending', async () => {
    const fetchImpl = vi.fn<typeof fetch>();
    await expect(revokeHostedPasskey({ baseUrl, fetchImpl }, 'proof', 'bad')).rejects.toThrow();
    await expect(
      renameHostedPasskey({ baseUrl, fetchImpl }, 'proof', id, 'key\u0085'),
    ).rejects.toThrow();
    vi.stubGlobal('location', { origin: 'https://account.example' });
    await expect(listHostedPasskeys({ baseUrl, fetchImpl }, 'proof')).rejects.toThrow(
      'Identity browser origin',
    );
    expect(fetchImpl).not.toHaveBeenCalled();
  });

  it('retains the shared transport response-size and HTTP failure boundaries', async () => {
    const fetchImpl = vi
      .fn<typeof fetch>()
      .mockResolvedValueOnce(json([{ ...row, label: 'x'.repeat(65_536) }]))
      .mockResolvedValueOnce(json({}, 401));
    await expect(listHostedPasskeys({ baseUrl, fetchImpl }, 'proof')).rejects.toThrow('too large');
    await expect(listHostedPasskeys({ baseUrl, fetchImpl }, 'proof')).rejects.toBeInstanceOf(
      HostedIdentityError,
    );
  });
});
