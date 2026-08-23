import { afterEach, describe, expect, it } from 'vite-plus/test';
import {
  drivePathForSession,
  getActiveSession,
  getActiveSessionId,
  saveSessions,
  setActiveSessionId,
  subscribeToSessionChanges,
} from './drive.session.storage';

type StorageListener = (event: StorageEvent) => void;

function installTestWindow(pathname = '/') {
  const storageListeners = new Set<StorageListener>();
  const localStorage = createMemoryStorage();
  const sessionStorage = createMemoryStorage();

  Object.defineProperty(globalThis, 'window', {
    configurable: true,
    value: {
      localStorage,
      sessionStorage,
      location: { pathname },
      addEventListener: (type: string, listener: StorageListener) => {
        if (type === 'storage') {
          storageListeners.add(listener);
        }
      },
      removeEventListener: (type: string, listener: StorageListener) => {
        if (type === 'storage') {
          storageListeners.delete(listener);
        }
      },
      dispatchStorageEvent: (event: Partial<StorageEvent>) => {
        for (const listener of storageListeners) {
          listener(event as StorageEvent);
        }
      },
    },
    writable: true,
  });

  return globalThis.window as unknown as {
    localStorage: Storage;
    dispatchStorageEvent: (event: Partial<StorageEvent>) => void;
  };
}

function createMemoryStorage(): Storage {
  const values = new Map<string, string>();
  return {
    get length() {
      return values.size;
    },
    clear: () => values.clear(),
    getItem: (key: string) => values.get(key) ?? null,
    key: (index: number) => Array.from(values.keys())[index] ?? null,
    removeItem: (key: string) => {
      values.delete(key);
    },
    setItem: (key: string, value: string) => {
      values.set(key, value);
    },
  };
}

afterEach(() => {
  Reflect.deleteProperty(globalThis, 'window');
});

describe('drive session storage synchronization', () => {
  it('notifies subscribers when session storage changes in another tab', () => {
    const windowRef = installTestWindow();
    let notifications = 0;

    const unsubscribe = subscribeToSessionChanges(() => {
      notifications += 1;
    });

    windowRef.dispatchStorageEvent({
      key: 'nvbes_drive_sessions',
      storageArea: window.localStorage,
    });

    expect(notifications).toBe(1);

    unsubscribe();
  });

  it('resolves the active Drive session from the window URL path', () => {
    installTestWindow('/u/user-b/files');
    saveSessions([
      {
        accessToken: 'token-a',
        email: 'a@example.test',
        name: 'Account A',
        userId: 'user-a',
      },
      {
        accessToken: 'token-b',
        email: 'b@example.test',
        name: 'Account B',
        userId: 'user-b',
      },
    ]);

    expect(getActiveSession()?.userId).toBe('user-b');
  });

  it('stores fallback active Drive session in session storage only', () => {
    const windowRef = installTestWindow('/');

    setActiveSessionId('user-a');

    expect(getActiveSessionId()).toBe('user-a');
    expect(windowRef.localStorage.getItem('nvbes_drive_active_user_id')).toBeNull();
  });

  it('builds Drive account paths without stacking user prefixes', () => {
    expect(drivePathForSession('user-b', '/files')).toBe('/u/user-b/files');
    expect(drivePathForSession('user-b', '/u/user-a/files')).toBe('/u/user-b/files');
    expect(drivePathForSession('user-b', '/callback')).toBe('/u/user-b/');
  });
});
