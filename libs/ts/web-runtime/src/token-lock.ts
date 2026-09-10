const REFRESH_LOCK = 'nvbes-token-refresh';

export async function acquireTokenRefreshLock(): Promise<boolean> {
  // Availability probe only: the lock is released before this promise resolves.
  // Use withTokenRefreshLock to protect a refresh operation.
  if (typeof navigator === 'undefined' || !('locks' in navigator)) {
    return true;
  }
  try {
    await underTokenRefreshLock(async () => undefined);
    return true;
  } catch {
    return false;
  }
}

export async function withTokenRefreshLock(
  refresh: () => Promise<void>,
  onNewTokenReady: () => void,
) {
  const execute = async () => {
    await refresh();
    onNewTokenReady();
  };
  if (typeof navigator === 'undefined' || !('locks' in navigator)) {
    await execute();
    return;
  }
  await underTokenRefreshLock(execute);
}

async function underTokenRefreshLock(operation: () => Promise<void>): Promise<void> {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 5000);
  try {
    await navigator.locks.request(REFRESH_LOCK, { signal: controller.signal }, async () => {
      // The deadline bounds acquisition, not an operation already holding the lock.
      clearTimeout(timeout);
      await operation();
    });
  } finally {
    clearTimeout(timeout);
  }
}
