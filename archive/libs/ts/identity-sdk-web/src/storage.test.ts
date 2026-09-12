import { afterEach, describe, expect, it, vi } from 'vite-plus/test';
import {
  BrowserSessionStorage,
  defaultWebStorage,
  MemoryStorage,
  type OAuthTransaction,
} from './storage';

const transaction: OAuthTransaction = {
  state: 'state',
  codeVerifier: 'verifier',
  nonce: 'nonce',
  createdAt: 100,
  returnTo: '/account',
};

function browserStorage(): Storage {
  const values = new Map<string, string>();
  return {
    get length() {
      return values.size;
    },
    clear: () => values.clear(),
    getItem: (key) => values.get(key) ?? null,
    key: (index) => [...values.keys()][index] ?? null,
    removeItem: (key) => {
      values.delete(key);
    },
    setItem: (key, value) => {
      values.set(key, value);
    },
  };
}

afterEach(() => vi.unstubAllGlobals());

describe('one-time OAuth transaction storage', () => {
  it('isolates memory transactions from caller mutation and other instances', () => {
    const storage = new MemoryStorage();
    expect(storage.getTransaction()).toBeNull();
    const input = { ...transaction };
    storage.saveTransaction(input);
    input.state = 'changed';
    expect(storage.getTransaction()).toEqual(transaction);
    const retrieved = storage.getTransaction();
    if (!retrieved) throw new Error('Missing transaction');
    retrieved.codeVerifier = 'changed';
    expect(storage.getTransaction()).toEqual(transaction);
    expect(new MemoryStorage().getTransaction()).toBeNull();
    storage.clearTransaction();
    expect(storage.getTransaction()).toBeNull();
  });

  it.each(['custom.transaction', undefined])('persists and clears only its own key (%s)', (key) => {
    const backend = browserStorage();
    backend.setItem('unrelated', 'keep');
    const storage = new BrowserSessionStorage(backend, key);
    expect(storage.getTransaction()).toBeNull();
    storage.saveTransaction(transaction);
    expect(backend.getItem(key ?? 'nvbes.identity.oauth.transaction')).toBe(
      JSON.stringify(transaction),
    );
    expect(storage.getTransaction()).toEqual(transaction);
    storage.saveTransaction({ ...transaction, nonce: null });
    expect(storage.getTransaction()?.nonce).toBeNull();
    storage.clearTransaction();
    expect(storage.getTransaction()).toBeNull();
    expect(backend.getItem('unrelated')).toBe('keep');
  });

  it.each([
    null,
    [],
    1,
    'text',
    {},
    { ...transaction, state: '' },
    { ...transaction, state: 1 },
    { ...transaction, codeVerifier: '' },
    { ...transaction, codeVerifier: false },
    { ...transaction, nonce: 1 },
    { ...transaction, createdAt: '100' },
    { ...transaction, returnTo: 1 },
    { ...transaction, returnTo: 'https://evil.example' },
    { ...transaction, returnTo: '//evil.example' },
  ])('removes invalid stored transactions (%j)', (invalid) => {
    const backend = browserStorage();
    backend.setItem('transaction', JSON.stringify(invalid));
    const storage = new BrowserSessionStorage(backend, 'transaction');
    expect(storage.getTransaction()).toBeNull();
    expect(backend.getItem('transaction')).toBeNull();
  });

  it.each([
    '{',
    '{"state":"state","codeVerifier":"verifier","nonce":null,"createdAt":1e400,"returnTo":"/"}',
  ])('rejects corrupt JSON and nonfinite timestamps', (raw) => {
    const backend = browserStorage();
    backend.setItem('transaction', raw);
    expect(new BrowserSessionStorage(backend, 'transaction').getTransaction()).toBeNull();
    expect(backend.length).toBe(0);
  });

  it('chooses session storage only after a successful writable probe', () => {
    const backend = browserStorage();
    vi.stubGlobal('window', { sessionStorage: backend });
    const storage = defaultWebStorage();
    expect(storage).toBeInstanceOf(BrowserSessionStorage);
    expect(backend.length).toBe(0);
    storage.saveTransaction(transaction);
    expect(defaultWebStorage().getTransaction()).toEqual(transaction);
  });

  it('falls back to isolated memory when the browser denies storage', () => {
    vi.stubGlobal('window', {
      get sessionStorage() {
        throw new Error('denied');
      },
    });
    const storage = defaultWebStorage();
    expect(storage).toBeInstanceOf(MemoryStorage);
    storage.saveTransaction(transaction);
    expect(defaultWebStorage().getTransaction()).toBeNull();
    vi.stubGlobal('window', undefined);
    expect(defaultWebStorage()).toBeInstanceOf(MemoryStorage);
  });
});
