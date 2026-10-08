import { afterEach, beforeEach, describe, expect, it, vi } from 'vite-plus/test';
import { IndexedDbDpopTransactionStore } from '../dpop.transaction-store';
import type { DpopMainKeyPair } from '../dpop';

describe('IndexedDbDpopTransactionStore', () => {
  let store: IndexedDbDpopTransactionStore;
  const inMemoryDb = new Map<string, unknown>();

  beforeEach(() => {
    inMemoryDb.clear();
    store = new IndexedDbDpopTransactionStore('test.dpop.transactions');

    const fakeIndexedDb = {
      open: () => {
        interface MockRequest {
          result?: unknown;
          onsuccess?: () => void;
        }
        const req: MockRequest = {};
        setTimeout(() => {
          req.result = {
            createObjectStore: () => {},
            close: () => {},
            transaction: () => ({
              objectStore: () => ({
                openCursor: () => {
                  const cursorReq: MockRequest = {};
                  setTimeout(() => {
                    cursorReq.result = null; // no existing records to clean up
                    cursorReq.onsuccess?.();
                  }, 0);
                  return cursorReq;
                },
                add: (val: unknown, key: string) => {
                  inMemoryDb.set(key, val);
                },
                get: (key: string) => {
                  const getReq: MockRequest = {};
                  setTimeout(() => {
                    getReq.result = inMemoryDb.get(key);
                    getReq.onsuccess?.();
                  }, 0);
                  return getReq;
                },
                delete: (key: string) => {
                  inMemoryDb.delete(key);
                },
              }),
              oncomplete: null as (() => void) | null,
            }),
          };
          // Simulate tx oncomplete
          const db = req.result as {
            transaction: (...args: unknown[]) => { oncomplete?: () => void };
          };
          const origTx = db.transaction;
          db.transaction = (...args: unknown[]) => {
            const tx = origTx(...args);
            setTimeout(() => {
              tx.oncomplete?.();
            }, 5);
            return tx;
          };
          req.onsuccess?.();
        }, 0);
        return req;
      },
    };

    vi.stubGlobal('indexedDB', fakeIndexedDb);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('rejects saving invalid or expired DPoP keys', async () => {
    const invalidKey = {
      keyPair: {
        privateKey: { extractable: true }, // extractable keys are forbidden
        publicKey: {},
      },
    } as unknown as DpopMainKeyPair;

    await expect(store.save('tx1', invalidKey, Date.now() + 1000)).rejects.toThrow(
      'Invalid pending DPoP key',
    );

    const expiredKey = {
      keyPair: {
        privateKey: { extractable: false },
        publicKey: {},
      },
    } as unknown as DpopMainKeyPair;

    await expect(store.save('tx1', expiredKey, Date.now() - 1000)).rejects.toThrow(
      'Invalid pending DPoP key',
    );
  });

  it('removes stored transaction keys', async () => {
    inMemoryDb.set('tx_delete', { val: 'test' });
    await store.remove('tx_delete');
    expect(inMemoryDb.has('tx_delete')).toBe(false);
  });

  it('returns null on load if key not found', async () => {
    const loaded = await store.load('nonexistent');
    expect(loaded).toBeNull();
  });
});
