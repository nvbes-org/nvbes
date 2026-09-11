// @vitest-environment happy-dom
import { afterEach, beforeEach, expect, it, vi } from 'vite-plus/test';
import { z } from 'zod';
import { createSafeStorage, getSafeLocalStorage, getSafeSessionStorage } from './safe-storage';

function memoryStorage(): Storage {
  const entries: Record<string, string> = {};
  return Object.defineProperties(entries, {
    getItem: { value: (key: string) => entries[key] ?? null },
    setItem: {
      value: (key: string, value: string) => {
        entries[key] = value;
      },
    },
    removeItem: {
      value: (key: string) => {
        delete entries[key];
      },
    },
    length: { get: () => Object.keys(entries).length },
  }) as unknown as Storage;
}
beforeEach(() => {
  const localStorage = memoryStorage();
  const sessionStorage = memoryStorage();
  vi.stubGlobal('window', { localStorage, sessionStorage });
  vi.stubGlobal('localStorage', localStorage);
  vi.stubGlobal('sessionStorage', sessionStorage);
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});
it.each(['local', 'session'] as const)('round trips %s values and validates JSON', (kind) => {
  const store = kind === 'local' ? getSafeLocalStorage() : getSafeSessionStorage();
  expect(store.available).toBe(true);
  expect(store.getItem('missing')).toBeNull();
  expect(store.getJson('missing', z.string())).toBeNull();
  expect(store.setItem('one', 'value')).toBe(true);
  expect(store.getItem('one')).toBe('value');
  expect(store.keys()).toContain('one');
  expect(store.setJson('json', { enabled: true })).toBe(true);
  expect(store.getJson('json', z.object({ enabled: z.boolean() }))).toEqual({ enabled: true });
  expect(store.getJson('json', z.string())).toBeNull();
  store.setItem('json', '{');
  expect(store.getJson('json', z.string())).toBeNull();
  expect(store.removeItem('one')).toBe(true);
  expect(store.getItem('one')).toBeNull();
  const circular: Record<string, unknown> = {};
  circular.self = circular;
  expect(store.setJson('json', circular)).toBe(false);
  expect(kind === 'local' ? sessionStorage.length : localStorage.length).toBe(0);
});
it.each([undefined, {}, { getItem: 1 }])(
  'degrades honestly when storage is absent: %j',
  (storage) => {
    vi.stubGlobal('window', { localStorage: storage });
    const store = createSafeStorage('local');
    expect(store.available).toBe(false);
    expect(store.getItem('x')).toBeNull();
    expect(store.keys()).toEqual([]);
    expect(store.setItem('x', 'y')).toBe(false);
    expect(store.removeItem('x')).toBe(false);
    expect(store.getJson('x', z.string())).toBeNull();
    expect(store.setJson('x', {})).toBe(false);
  },
);
it('contains security, quota and serialization failures', () => {
  const denied = () => {
    throw new Error('denied');
  };
  vi.stubGlobal('window', {
    get localStorage() {
      return denied();
    },
  });
  expect(getSafeLocalStorage().available).toBe(false);
  vi.stubGlobal('window', undefined);
  expect(getSafeSessionStorage().available).toBe(false);
  vi.stubGlobal('window', {
    localStorage: new Proxy(
      { getItem: denied, setItem: denied, removeItem: denied },
      { ownKeys: denied },
    ),
  });
  const store = getSafeLocalStorage();
  expect(store.available).toBe(true);
  expect(store.getItem('x')).toBeNull();
  expect(store.keys()).toEqual([]);
  expect(store.setItem('x', 'y')).toBe(false);
  expect(store.removeItem('x')).toBe(false);
});
