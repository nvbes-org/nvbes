import { describe, expect, it, vi } from 'vite-plus/test';
import { BrowserLogoutTransaction } from '../oauth.logout';

function setup() {
  const values = new Map<string, string>();
  const storage: Storage = {
    get length() {
      return values.size;
    },
    clear: () => values.clear(),
    getItem: (key) => values.get(key) ?? null,
    setItem: (key, value) => {
      values.set(key, value);
    },
    removeItem: (key) => {
      values.delete(key);
    },
    key: (index) => [...values.keys()][index] ?? null,
  };
  const transaction = new BrowserLogoutTransaction(storage);
  const submission = transaction.create({
    baseUrl: 'https://identity.example',
    clientId: 'account-web',
    idToken: 'private.id.token',
    redirectUri: 'https://account.example/oauth/logout/callback',
  });
  return {
    storage,
    transaction,
    submission,
    callback: () =>
      new URL(`https://account.example/oauth/logout/callback?state=${submission.fields.state}`),
  };
}
describe('RP logout transaction', () => {
  it('stores only a bounded state transaction and consumes it once', () => {
    const { storage, transaction, submission, callback } = setup();
    expect(storage.getItem('nvbes.oauth.logout')).not.toContain('private.id.token');
    expect(submission.action).toBe('https://identity.example/oauth/end-session');
    expect(submission.fields.id_token_hint).toBe('private.id.token');
    transaction.consume(callback());
    expect(storage.length).toBe(0);
    expect(() => transaction.consume(callback())).toThrow();
  });
  it('refuses wrong or duplicate state, destination changes and expired callbacks', () => {
    for (const change of [
      (url: URL) => url.searchParams.set('state', 'wrong'),
      (url: URL) => url.searchParams.append('state', url.searchParams.get('state')!),
      (url: URL) => {
        url.hostname = 'attacker.example';
      },
      (url: URL) => {
        url.pathname = '/other';
      },
      (url: URL) => {
        url.hash = 'unexpected';
      },
      (url: URL) => url.searchParams.append('error', 'fake'),
    ]) {
      const { transaction, callback, storage } = setup();
      const url = callback();
      change(url);
      expect(() => transaction.consume(url)).toThrow();
      expect(storage.length).toBe(0);
    }
    vi.useFakeTimers();
    try {
      const { transaction, callback } = setup();
      vi.advanceTimersByTime(300_000);
      expect(() => transaction.consume(callback())).toThrow();
    } finally {
      vi.useRealTimers();
    }
  });
});
