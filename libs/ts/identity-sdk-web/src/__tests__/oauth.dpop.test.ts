import { describe, expect, it, vi } from 'vite-plus/test';
import { type DpopMainKeyPair } from '../dpop';
import { type DpopTransactionStore } from '../dpop.transaction-store';
import { createAuthorizationRequest } from '../oauth.authorization-request';
import { exchangeAuthorizationCode } from '../oauth.authorization-code';
import { MemoryStorage } from '../storage';
import { idToken, jwksResponse } from './oidc.fixture';

class TestKeyStore implements DpopTransactionStore {
  keys = new Map<string, DpopMainKeyPair>();
  async save(id: string, key: DpopMainKeyPair): Promise<void> {
    this.keys.set(id, key);
  }
  async load(id: string): Promise<DpopMainKeyPair | null> {
    return this.keys.get(id) ?? null;
  }
  async remove(id: string): Promise<void> {
    this.keys.delete(id);
  }
}

const par = () =>
  new Response(
    JSON.stringify({ request_uri: 'urn:ietf:params:oauth:request_uri:test', expires_in: 300 }),
    { status: 201 },
  );
const tokens = (tokenType: string, identityToken?: string) =>
  new Response(
    JSON.stringify({
      access_token: 'token',
      id_token: identityToken,
      token_type: tokenType,
      expires_in: 300,
      scope: 'openid account:read',
    }),
  );
function setup() {
  return {
    baseUrl: 'https://identity.example',
    clientId: 'account-web',
    redirectUri: 'https://account.example/callback',
    resource: 'https://account-api.example',
    storage: new MemoryStorage(),
    dpopStore: new TestKeyStore(),
    fetchImpl: vi.fn<typeof fetch>().mockImplementation(jwksResponse),
  };
}
function decode(part: string): Record<string, unknown> {
  return JSON.parse(atob(part.replace(/-/g, '+').replace(/_/g, '/')));
}

describe('OAuth DPoP transaction binding', () => {
  it('does not overwrite or clear a pending transaction when a second authorization starts', async () => {
    const config = setup();
    config.fetchImpl.mockResolvedValueOnce(par());
    await createAuthorizationRequest(config, { scope: 'openid account:read' });
    const pending = config.storage.getTransaction();
    await expect(
      createAuthorizationRequest(config, { scope: 'openid account:read' }),
    ).rejects.toThrow('already pending');
    expect(config.storage.getTransaction()).toEqual(pending);
    expect(config.dpopStore.keys.size).toBe(1);
    expect(config.fetchImpl).toHaveBeenCalledTimes(1);
  });
  it('uses the same private key at PAR and token, with distinct proofs and no bearer downgrade', async () => {
    const config = setup();
    config.fetchImpl.mockResolvedValueOnce(par());
    const request = await createAuthorizationRequest(config, { scope: 'openid account:read' });
    const binding = config.storage.getTransaction()?.dpop;
    expect(binding).toBeDefined();
    const parProof = new Headers(config.fetchImpl.mock.calls[0]?.[1]?.headers).get('DPoP')!;
    config.fetchImpl.mockResolvedValueOnce(tokens('Bearer'));
    await expect(
      exchangeAuthorizationCode(config, { state: request.state, code: 'code' }),
    ).rejects.toThrow('binding');
    expect(config.storage.getTransaction()).not.toBeNull();
    config.fetchImpl.mockResolvedValueOnce(
      tokens('DPoP', await idToken(config.storage.getTransaction()!.nonce!, 'token')),
    );
    const result = await exchangeAuthorizationCode(config, { state: request.state, code: 'code' });
    const tokenProof = new Headers(config.fetchImpl.mock.calls[2]?.[1]?.headers).get('DPoP')!;
    expect(decode(parProof.split('.')[0]).jwk).toEqual(decode(tokenProof.split('.')[0]).jwk);
    expect(decode(parProof.split('.')[1]).jti).not.toEqual(decode(tokenProof.split('.')[1]).jti);
    expect(decode(tokenProof.split('.')[1]).htu).toBe('https://identity.example/oauth/token');
    expect(result.dpopKey?.jkt).toBe(binding?.jkt);
    expect(config.dpopStore.keys.size).toBe(0);
    expect(config.storage.getTransaction()).toBeNull();
  });

  it('refuses missing keys or changed context before token transport', async () => {
    const config = setup();
    config.fetchImpl.mockResolvedValueOnce(par());
    const request = await createAuthorizationRequest(config, { scope: 'openid account:read' });
    for (const changed of [
      { clientId: 'other' },
      { baseUrl: 'https://other.example' },
      { redirectUri: 'https://other.example/callback' },
    ]) {
      await expect(
        exchangeAuthorizationCode(
          { ...config, ...changed },
          { state: request.state, code: 'code' },
        ),
      ).rejects.toThrow('context');
    }
    config.dpopStore.keys.clear();
    await expect(
      exchangeAuthorizationCode(config, { state: request.state, code: 'code' }),
    ).rejects.toThrow('key');
    expect(config.fetchImpl).toHaveBeenCalledTimes(1);
  });

  it('removes pending keys and transaction after PAR fails', async () => {
    const config = setup();
    config.fetchImpl.mockRejectedValueOnce(new Error('network'));
    await expect(
      createAuthorizationRequest(config, { scope: 'openid account:read' }),
    ).rejects.toThrow('network');
    expect(config.dpopStore.keys.size).toBe(0);
    expect(config.storage.getTransaction()).toBeNull();
  });
});
