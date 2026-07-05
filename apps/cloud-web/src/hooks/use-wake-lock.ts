import { useCallback, useEffect, useRef } from 'react';

type WakeLockState =
  | { type: 'unsupported' }
  | { type: 'inactive' }
  | { type: 'acquiring' }
  | { type: 'active' }
  | { type: 'releasing' };

const WAKE_LOCK_TYPE = 'screen' as const;

function isSupported(): boolean {
  return typeof navigator !== 'undefined' && 'wakeLock' in navigator;
}

export function useWakeLock() {
  const sentinelRef = useRef<WakeLockSentinel | null>(null);
  const acquiredRef = useRef(false);
  const stateRef = useRef<WakeLockState>({ type: isSupported() ? 'inactive' : 'unsupported' });

  const acquire = useCallback(async () => {
    if (!isSupported()) return;
    if (acquiredRef.current) return;

    acquiredRef.current = true;
    stateRef.current = { type: 'acquiring' };

    try {
      const sentinel = await navigator.wakeLock.request(WAKE_LOCK_TYPE);
      sentinelRef.current = sentinel;
      stateRef.current = { type: 'active' };

      sentinel.addEventListener('release', () => {
        acquiredRef.current = false;
        sentinelRef.current = null;
        stateRef.current = { type: 'inactive' };
      });
    } catch {
      acquiredRef.current = false;
      stateRef.current = { type: 'inactive' };
    }
  }, []);

  const release = useCallback(async () => {
    if (!sentinelRef.current) return;

    stateRef.current = { type: 'releasing' };
    acquiredRef.current = false;

    try {
      await sentinelRef.current.release();
    } catch {
      // Best-effort release
    }
    sentinelRef.current = null;
    stateRef.current = { type: 'inactive' };
  }, []);

  useEffect(() => {
    if (!isSupported()) return;

    async function onVisibilityChange() {
      if (document.visibilityState === 'visible' && acquiredRef.current) {
        try {
          const sentinel = await navigator.wakeLock.request(WAKE_LOCK_TYPE);
          sentinelRef.current = sentinel;
          stateRef.current = { type: 'active' };

          sentinel.addEventListener('release', () => {
            if (sentinelRef.current === sentinel) {
              acquiredRef.current = false;
              sentinelRef.current = null;
              stateRef.current = { type: 'inactive' };
            }
          });
        } catch {
          // Re-acquisition failed; uploads continue without wake lock
        }
      }
    }

    document.addEventListener('visibilitychange', onVisibilityChange);
    return () => {
      document.removeEventListener('visibilitychange', onVisibilityChange);
    };
  }, []);

  useEffect(() => {
    return () => {
      if (sentinelRef.current) {
        sentinelRef.current.release().catch(() => {});
        sentinelRef.current = null;
        acquiredRef.current = false;
      }
    };
  }, []);

  return { acquire, release };
}
