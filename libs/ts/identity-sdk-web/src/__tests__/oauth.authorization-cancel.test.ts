import { expect, it, vi } from 'vite-plus/test';
import { discardAuthorizationRequest } from '../oauth.authorization-cancel';
import { MemoryStorage } from '../storage';

it('clears abandoned state and removes only its DPoP key while preserving a newer request', async () => {
  const storage = new MemoryStorage();
  const transaction = {
    state: 'old',
    codeVerifier: 'verifier',
    nonce: null,
    createdAt: 0,
    returnTo: '/',
    dpop: {
      keyId: 'old-key',
      jkt: 'jkt',
      issuer: 'https://identity.example',
      clientId: 'client',
      redirectUri: 'https://client.example/callback',
    },
  };
  storage.saveTransaction(transaction);
  let done = () => {};
  const remove = vi.fn().mockReturnValue(
    new Promise<void>((resolve) => {
      done = resolve;
    }),
  );
  const dpopStore = { remove, save: vi.fn(), load: vi.fn() };
  const pending = discardAuthorizationRequest({ storage, dpopStore });
  expect(storage.getTransaction()).toBeNull();
  storage.saveTransaction({ ...transaction, state: 'new' });
  done();
  await pending;
  expect(remove).toHaveBeenCalledExactlyOnceWith('old-key');
  expect(storage.getTransaction()?.state).toBe('new');
});

it('leaves no usable transaction after a failed key deletion and reports the failure', async () => {
  const storage = new MemoryStorage();
  storage.saveTransaction({
    state: 'old',
    codeVerifier: 'verifier',
    nonce: null,
    createdAt: 0,
    returnTo: '/',
    dpop: {
      keyId: 'key',
      jkt: 'jkt',
      issuer: 'issuer',
      clientId: 'client',
      redirectUri: 'redirect',
    },
  });
  const dpopStore = {
    remove: vi.fn().mockRejectedValue(new Error('Unavailable')),
    save: vi.fn(),
    load: vi.fn(),
  };
  await expect(discardAuthorizationRequest({ storage, dpopStore })).rejects.toThrow('Unavailable');
  expect(storage.getTransaction()).toBeNull();
});
