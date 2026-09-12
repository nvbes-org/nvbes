import { z } from 'zod';

export type SafeStorageKind = 'local' | 'session';

export interface SafeStorage {
  readonly available: boolean;
  getItem(key: string): string | null;
  keys(): string[];
  setItem(key: string, value: string): boolean;
  removeItem(key: string): boolean;
  getJson<T>(key: string, schema: z.ZodType<T>): T | null;
  setJson(key: string, value: unknown): boolean;
}

export function getSafeLocalStorage(): SafeStorage {
  return createSafeStorage('local');
}

export function getSafeSessionStorage(): SafeStorage {
  return createSafeStorage('session');
}

export function createSafeStorage(kind: SafeStorageKind): SafeStorage {
  const storage = readBrowserStorage(kind);
  if (!storage) {
    return unavailableStorage;
  }

  return {
    available: true,
    getItem(key) {
      try {
        return storage.getItem(key);
      } catch {
        return null;
      }
    },
    keys() {
      try {
        return Object.keys(storage);
      } catch {
        return [];
      }
    },
    setItem(key, value) {
      try {
        storage.setItem(key, value);
        return true;
      } catch {
        return false;
      }
    },
    removeItem(key) {
      try {
        storage.removeItem(key);
        return true;
      } catch {
        return false;
      }
    },
    getJson<T>(key: string, schema: z.ZodType<T>): T | null {
      const value = this.getItem(key);
      if (!value) {
        return null;
      }

      try {
        const parsed = JSON.parse(value) as unknown;
        const result = schema.safeParse(parsed);
        return result.success ? result.data : null;
      } catch {
        return null;
      }
    },
    setJson(key, value) {
      try {
        return this.setItem(key, JSON.stringify(value));
      } catch {
        return false;
      }
    },
  };
}

const unavailableStorage: SafeStorage = {
  available: false,
  getItem: () => null,
  keys: () => [],
  setItem: () => false,
  removeItem: () => false,
  getJson: () => null,
  setJson: () => false,
};

function readBrowserStorage(kind: SafeStorageKind): Storage | null {
  if (typeof window === 'undefined') {
    return null;
  }

  try {
    const storage = kind === 'local' ? window.localStorage : window.sessionStorage;
    if (!storage || typeof storage.getItem !== 'function') {
      return null;
    }
    return storage;
  } catch {
    return null;
  }
}
