import { beforeEach, describe, expect, it } from 'vite-plus/test';
import { memoryStorage, type OAuthTransaction } from '../storage';

const transaction: OAuthTransaction = {
  state: 'test-state',
  codeVerifier: 'test-verifier',
  nonce: 'test-nonce',
  createdAt: 123,
  returnTo: '/security',
};

describe('MemoryStorage', () => {
  beforeEach(() => {
    memoryStorage.clearTransaction();
  });

  it('saves and retrieves an OAuth transaction', () => {
    memoryStorage.saveTransaction(transaction);

    expect(memoryStorage.getTransaction()).toEqual(transaction);
  });

  it('returns copies that cannot mutate the stored transaction', () => {
    memoryStorage.saveTransaction(transaction);
    const stored = memoryStorage.getTransaction();
    if (!stored) {
      throw new Error('Expected a stored transaction.');
    }

    stored.state = 'mutated';

    expect(memoryStorage.getTransaction()?.state).toBe('test-state');
  });

  it('clears an OAuth transaction', () => {
    memoryStorage.saveTransaction(transaction);
    memoryStorage.clearTransaction();

    expect(memoryStorage.getTransaction()).toBeNull();
  });
});
