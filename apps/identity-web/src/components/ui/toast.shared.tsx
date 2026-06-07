import { createContext } from 'react';

export type ToastVariant = 'default' | 'destructive';

export type ToastInput = {
  title: string;
  description?: string;
  variant?: ToastVariant;
  durationMs?: number;
};

export type ToastRecord = ToastInput & { id: string };

export type ToastContextValue = {
  pushToast: (toast: ToastInput) => string;
  dismissToast: (id: string) => void;
};

export const ToastContext = createContext<ToastContextValue | null>(null);

export let globalToastApi: ToastContextValue | null = null;

export function setGlobalToastApi(value: ToastContextValue | null) {
  globalToastApi = value;
}

export function toast(input: ToastInput): string | null {
  return globalToastApi?.pushToast(input) ?? null;
}
