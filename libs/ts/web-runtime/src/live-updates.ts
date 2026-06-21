import { useEffect, useState } from 'react';

export type BrowserVisibilityState = 'visible' | 'hidden';

export function getBrowserVisibilityState(): BrowserVisibilityState {
  if (typeof document === 'undefined') {
    return 'visible';
  }
  return document.visibilityState === 'hidden' ? 'hidden' : 'visible';
}

export function useBrowserVisibilityState(): BrowserVisibilityState {
  const [visibility, setVisibility] = useState(getBrowserVisibilityState);

  useEffect(() => {
    if (typeof document === 'undefined') {
      return undefined;
    }

    const updateVisibility = () => setVisibility(getBrowserVisibilityState());
    document.addEventListener('visibilitychange', updateVisibility);
    return () => document.removeEventListener('visibilitychange', updateVisibility);
  }, []);

  return visibility;
}

export function useVisibilityAwareInterval(
  visibleIntervalMs: number,
  hiddenIntervalMs: number | false = false,
): number | false {
  return useBrowserVisibilityState() === 'visible' ? visibleIntervalMs : hiddenIntervalMs;
}
