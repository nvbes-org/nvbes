import { useCallback, useEffect, useState } from 'react';

export type StorageInfo = {
  supported: boolean;
  usage: number;
  quota: number;
  usageRatio: number;
  persisted: boolean | null;
  isPersisting: boolean;
  formattedUsage: string;
  formattedQuota: string;
  requestPersistence: () => Promise<boolean>;
  refresh: () => Promise<void>;
};

function isStorageManagerSupported(): boolean {
  return typeof navigator !== 'undefined' && 'storage' in navigator;
}

const UNITS = ['octets', 'Ko', 'Mo', 'Go', 'To'] as const;

function formatBytes(bytes: number): string {
  if (bytes === 0) return `0 ${UNITS[0]}`;
  const base = 1024;
  const exponent = Math.min(Math.floor(Math.log(bytes) / Math.log(base)), UNITS.length - 1);
  const value = bytes / base ** exponent;
  return `${exponent === 0 ? value : value.toFixed(1)} ${UNITS[exponent]}`;
}

export function useStorageManager(): StorageInfo {
  const [usage, setUsage] = useState(0);
  const [quota, setQuota] = useState(0);
  const [persisted, setPersisted] = useState<boolean | null>(null);
  const [isPersisting, setIsPersisting] = useState(false);

  const refresh = useCallback(async () => {
    if (!isStorageManagerSupported()) return;

    try {
      const estimate = await navigator.storage.estimate();
      setUsage(estimate.usage ?? 0);
      setQuota(estimate.quota ?? 0);
    } catch {
      // Storage estimate unavailable
    }

    try {
      const isPersisted = await navigator.storage.persisted();
      setPersisted(isPersisted);
    } catch {
      // persisted() unavailable
    }
  }, []);

  const requestPersistence = useCallback(async (): Promise<boolean> => {
    if (!isStorageManagerSupported()) return false;

    setIsPersisting(true);
    try {
      const granted = await navigator.storage.persist();
      setPersisted(granted);
      return granted;
    } catch {
      return false;
    } finally {
      setIsPersisting(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const supported = isStorageManagerSupported();
  const usageRatio = quota > 0 ? Math.min((usage / quota) * 100, 100) : 0;

  return {
    supported,
    usage,
    quota,
    usageRatio,
    persisted,
    isPersisting,
    formattedUsage: formatBytes(usage),
    formattedQuota: formatBytes(quota),
    requestPersistence,
    refresh,
  };
}
