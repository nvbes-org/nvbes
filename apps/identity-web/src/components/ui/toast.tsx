import type { ReactNode } from 'react';
import {
  useCallback,
  useContext,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from 'react';

import {
  globalToastApi,
  setGlobalToastApi,
  ToastContext,
  type ToastInput,
  type ToastRecord,
} from './toast.shared';
import { ToastViewport } from './toast.viewport';

export type { ToastInput, ToastVariant } from './toast.shared';
export { toast } from './toast.shared';

export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<ToastRecord[]>([]);
  const timersRef = useRef(new Map<string, number>());

  const dismissToast = useCallback((id: string) => {
    const timeoutId = timersRef.current.get(id);
    if (timeoutId !== undefined) {
      window.clearTimeout(timeoutId);
      timersRef.current.delete(id);
    }

    setToasts((current) => current.filter((toastItem) => toastItem.id !== id));
  }, []);

  const pushToast = useCallback(
    (input: ToastInput) => {
      const id = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`;
      const record: ToastRecord = {
        ...input,
        variant: input.variant ?? 'default',
        id,
      };

      setToasts((current) => [...current, record]);

      const durationMs = input.durationMs ?? 5500;
      if (durationMs > 0) {
        const timeoutId = window.setTimeout(() => {
          dismissToast(id);
        }, durationMs);
        timersRef.current.set(id, timeoutId);
      }

      return id;
    },
    [dismissToast],
  );

  useLayoutEffect(() => {
    setGlobalToastApi({ pushToast, dismissToast });
    return () => {
      if (globalToastApi?.pushToast === pushToast) {
        setGlobalToastApi(null);
      }
    };
  }, [dismissToast, pushToast]);

  useEffect(() => {
    return () => {
      timersRef.current.forEach((timeoutId) => {
        window.clearTimeout(timeoutId);
      });
      timersRef.current.clear();
    };
  }, []);

  const value = useMemo(
    () => ({
      pushToast,
      dismissToast,
    }),
    [dismissToast, pushToast],
  );

  return (
    <ToastContext.Provider value={value}>
      {children}
      <ToastViewport toasts={toasts} onDismiss={dismissToast} />
    </ToastContext.Provider>
  );
}

export function useToast() {
  const context = useContext(ToastContext);
  if (!context) {
    throw new Error('useToast must be used within ToastProvider');
  }
  return context;
}
