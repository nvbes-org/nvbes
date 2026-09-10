import { describe, expect, it, vi } from 'vite-plus/test';
import { generateBrowserDpopKeyPair } from '../dpop';
import { OAuthSession } from '../oauth.session';

async function fixture() {
  const dpopKey = await generateBrowserDpopKeyPair();
  const fetchImpl = vi.fn<typeof fetch>();
  const initial = {
    accessToken: 'access-1',
    tokenType: 'DPoP',
    expiresIn: 300,
    refreshToken: 'refresh-1',
    idToken: null,
    scope: 'openid account:read',
    returnTo: '/',
    dpopKey,
  };
  return {
    fetchImpl,
    initial,
    session: new OAuthSession(
      { baseUrl: 'https://identity.example', clientId: 'account-web', fetchImpl },
      initial,
    ),
  };
}
function response(refresh = 'refresh-2', overrides: Record<string, unknown> = {}): Response {
  return new Response(
    JSON.stringify({
      access_token: 'access-2',
      token_type: 'DPoP',
      expires_in: 300,
      refresh_token: refresh,
      scope: 'openid account:read',
      ...overrides,
    }),
  );
}

describe('OAuth session refresh', () => {
  it('coalesces concurrent refreshes and uses the new secret and same key on the next rotation', async () => {
    const { fetchImpl, session, initial } = await fixture();
    fetchImpl.mockResolvedValueOnce(response()).mockResolvedValueOnce(response('refresh-3'));
    const first = session.refresh();
    expect(session.refresh()).toBe(first);
    expect((await first).refreshToken).toBe('refresh-2');
    await session.refresh();
    expect(fetchImpl).toHaveBeenCalledTimes(2);
    const proofs = fetchImpl.mock.calls.map(([url, init]) => {
      expect(url).toBe('https://identity.example/oauth/token');
      expect(init).toMatchObject({ credentials: 'omit', redirect: 'error', cache: 'no-store' });
      const proof = new Headers(init?.headers).get('DPoP')!;
      return proof.split('.').map((part) => part.replace(/-/g, '+').replace(/_/g, '/'));
    });
    expect(JSON.parse(atob(proofs[0][0])).jwk).toEqual(JSON.parse(atob(proofs[1][0])).jwk);
    expect(JSON.parse(atob(proofs[0][1])).jti).not.toBe(JSON.parse(atob(proofs[1][1])).jti);
    const body = fetchImpl.mock.calls[1][1]?.body;
    if (!(body instanceof URLSearchParams)) throw new Error('Expected OAuth form body');
    expect(body.get('refresh_token')).toBe('refresh-2');
    expect(session.snapshot()?.dpopKey).toBe(initial.dpopKey);
  });

  it('discards ambiguous state after transport failure without retry', async () => {
    const { fetchImpl, session } = await fixture();
    fetchImpl.mockRejectedValueOnce(new Error('response lost'));
    await expect(session.refresh()).rejects.toThrow('response lost');
    expect(session.snapshot()).toBeNull();
    await expect(session.refresh()).rejects.toThrow('reauthentication');
    expect(fetchImpl).toHaveBeenCalledTimes(1);
  });

  it('refuses downgrade, scope escalation, missing rotation and malformed lifetime', async () => {
    for (const overrides of [
      { token_type: 'Bearer' },
      { scope: 'openid account:write' },
      { refresh_token: null },
      { refresh_token: 'refresh-1' },
      { expires_in: 1.5 },
    ]) {
      const { fetchImpl, session } = await fixture();
      fetchImpl.mockResolvedValueOnce(response('refresh-2', overrides));
      await expect(session.refresh()).rejects.toThrow();
      expect(session.snapshot()).toBeNull();
    }
  });

  it('cannot resurrect a session cleared while the refresh request is pending', async () => {
    const { fetchImpl, session } = await fixture();
    let complete!: (response: Response) => void;
    let entered!: () => void;
    const started = new Promise<void>((resolve) => {
      entered = resolve;
    });
    fetchImpl.mockImplementationOnce(() => {
      entered();
      return new Promise((resolve) => {
        complete = resolve;
      });
    });
    const pending = session.refresh();
    await started;
    session.clear();
    complete(response());
    await expect(pending).rejects.toThrow('cleared');
    expect(session.snapshot()).toBeNull();
  });
});
