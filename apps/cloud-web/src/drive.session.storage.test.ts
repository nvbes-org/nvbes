import { afterEach, describe, expect, it } from 'vite-plus/test';
import { subscribeToSessionChanges } from './drive.session.storage';

type StorageListener = (event: StorageEvent) => void;

function installTestWindow() {
  const storageListeners = new Set<StorageListener>();

  Object.defineProperty(globalThis, 'window', {
    configurable: true,
    value: {
      localStorage: {},
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
    dispatchStorageEvent: (event: Partial<StorageEvent>) => void;
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
});
