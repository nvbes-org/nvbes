const REFRESH_LOCK = 'nvbes-token-refresh';

export async function acquireTokenRefreshLock(): Promise<boolean> {
  if (typeof navigator === 'undefined' || !('locks' in navigator)) {
    return true;
  }
  try {
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 5000);

    await navigator.locks.request(REFRESH_LOCK, { signal: controller.signal }, async () => {
      clearTimeout(timeout);
    });
    return true;
  } catch {
    return false;
  }
}

export async function withTokenRefreshLock(
  refresh: () => Promise<void>,
  onNewTokenReady: () => void,
) {
  if (typeof navigator === 'undefined' || !('locks' in navigator)) {
    await refresh();
    return;
  }

  const isOwner = await acquireTokenRefreshLock();

  if (isOwner) {
    try {
      await navigator.locks.request(
        REFRESH_LOCK,
        { steal: false, ifAvailable: false },
        async () => {
          await refresh();
          onNewTokenReady();
        },
      );
    } catch {
      // Lock was stolen or aborted — another tab handled it
    }
  } else {
    // Another tab owns the lock, wait for the signal
    await new Promise<void>((resolve) => {
      navigator.locks.request(REFRESH_LOCK, { ifAvailable: false }, async () => {
        resolve();
      });
    });
  }
}
