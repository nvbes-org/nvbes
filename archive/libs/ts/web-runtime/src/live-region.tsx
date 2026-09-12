import { createContext, useCallback, useContext, useMemo, useState, type ReactNode } from 'react';

export interface LiveRegionApi {
  announce(message: string): void;
}

const LiveRegionContext = createContext<LiveRegionApi | null>(null);

export function LiveRegionProvider({ children }: { children: ReactNode }) {
  const [message, setMessage] = useState('');
  const announce = useCallback((nextMessage: string) => {
    setMessage('');
    window.setTimeout(() => setMessage(nextMessage), 0);
  }, []);
  const value = useMemo(() => ({ announce }), [announce]);

  return (
    <LiveRegionContext.Provider value={value}>
      {children}
      <LiveRegionMessage message={message} />
    </LiveRegionContext.Provider>
  );
}

export function useLiveRegion(): LiveRegionApi {
  return useContext(LiveRegionContext) ?? fallbackLiveRegion;
}

export function LiveRegionMessage({ message }: { message: string }) {
  return (
    <div aria-live="polite" aria-atomic="true" className="sr-only">
      {message}
    </div>
  );
}

const fallbackLiveRegion: LiveRegionApi = {
  announce: () => undefined,
};
