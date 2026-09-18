import { afterEach, beforeEach, expect, it, vi } from 'vite-plus/test';
import { queueMutation } from './service-worker';

type Transaction = {
  oncomplete: (() => void) | null;
  onerror: (() => void) | null;
  onabort: (() => void) | null;
  error: Error | null;
  objectStore: ReturnType<typeof vi.fn>;
};
let tx: Transaction;
let add: ReturnType<typeof vi.fn>;
let db: {
  close: ReturnType<typeof vi.fn>;
  transaction: ReturnType<typeof vi.fn>;
  createObjectStore: ReturnType<typeof vi.fn>;
};
let request: {
  result: typeof db;
  error: Error | null;
  onsuccess: (() => void) | null;
  onerror: (() => void) | null;
  onupgradeneeded: (() => void) | null;
};
let open: ReturnType<typeof vi.fn>;
beforeEach(() => {
  vi.stubGlobal('navigator', {});
  vi.spyOn(Date, 'now').mockReturnValue(1234);
  add = vi.fn();
  tx = {
    oncomplete: null,
    onerror: null,
    onabort: null,
    error: null,
    objectStore: vi.fn(() => ({ add })),
  };
  db = { close: vi.fn(), transaction: vi.fn(() => tx), createObjectStore: vi.fn() };
  request = { result: db, error: null, onsuccess: null, onerror: null, onupgradeneeded: null };
  open = vi.fn(() => request);
  vi.stubGlobal('indexedDB', { open });
});
afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});
async function opened() {
  request.onsuccess?.();
  await Promise.resolve();
  await Promise.resolve();
}
it('creates the store and commits the mutation before registering background sync', async () => {
  const register = vi.fn().mockResolvedValue(undefined);
  vi.stubGlobal('navigator', { serviceWorker: { ready: Promise.resolve({ sync: { register } }) } });
  const pending = queueMutation('profile', { display: 'Synthetic' });
  expect(open).toHaveBeenCalledExactlyOnceWith('nvbes-offline', 1);
  request.onupgradeneeded?.();
  expect(db.createObjectStore).toHaveBeenCalledExactlyOnceWith('mutations', {
    keyPath: 'id',
    autoIncrement: true,
  });
  await opened();
  expect(db.transaction).toHaveBeenCalledExactlyOnceWith('mutations', 'readwrite');
  expect(tx.objectStore).toHaveBeenCalledExactlyOnceWith('mutations');
  expect(add).toHaveBeenCalledExactlyOnceWith({
    tag: 'profile',
    payload: { display: 'Synthetic' },
    timestamp: 1234,
  });
  expect(register).not.toHaveBeenCalled();
  expect(db.close).not.toHaveBeenCalled();
  tx.oncomplete?.();
  await pending;
  expect(db.close).toHaveBeenCalledTimes(1);
  expect(register).toHaveBeenCalledExactlyOnceWith('profile');
  expect(db.close.mock.invocationCallOrder[0]).toBeLessThan(register.mock.invocationCallOrder[0]);
});
it.each(['abort', 'error', 'add', 'transaction'])(
  'closes the database and rejects %s failures',
  async (cause) => {
    const failure = new Error('storage failed');
    if (cause === 'add')
      add.mockImplementation(() => {
        throw failure;
      });
    if (cause === 'transaction')
      db.transaction.mockImplementation(() => {
        throw failure;
      });
    const result = expect(queueMutation('profile', {})).rejects.toThrow(
      cause === 'abort' ? 'aborted' : 'storage failed',
    );
    await opened();
    if (cause === 'abort') tx.onabort?.();
    if (cause === 'error') {
      tx.error = failure;
      tx.onerror?.();
    }
    await result;
    expect(db.close).toHaveBeenCalledTimes(1);
  },
);
it('propagates database-open errors without creating a transaction', async () => {
  const result = expect(queueMutation('profile', {})).rejects.toThrow('open failed');
  request.error = new Error('open failed');
  request.onerror?.();
  await result;
  expect(db.transaction).not.toHaveBeenCalled();
});
it.each(['missing-worker', 'missing-sync', 'denied-sync'])(
  'keeps committed mutations when sync is %s',
  async (cause) => {
    if (cause !== 'missing-worker')
      vi.stubGlobal('navigator', {
        serviceWorker: {
          ready: Promise.resolve(
            cause === 'missing-sync'
              ? {}
              : { sync: { register: vi.fn().mockRejectedValue(new Error('denied')) } },
          ),
        },
      });
    const result = queueMutation('profile', {});
    await opened();
    tx.oncomplete?.();
    await expect(result).resolves.toBeUndefined();
    expect(db.close).toHaveBeenCalledTimes(1);
  },
);
