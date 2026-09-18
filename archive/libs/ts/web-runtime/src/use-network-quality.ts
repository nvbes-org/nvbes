import * as React from 'react';

export type EffectiveType = 'slow-2g' | '2g' | '3g' | '4g' | '5g' | 'unknown';

export interface NetworkQuality {
  effectiveType: EffectiveType;
  isSlowConnection: boolean;
  saveData: boolean;
  downlink: number | undefined;
  rtt: number | undefined;
}

interface ConnectionInfo {
  readonly effectiveType: string;
  readonly downlink: number;
  readonly rtt: number;
  readonly saveData: boolean;
  addEventListener: (type: string, listener: EventListenerOrEventListenerObject) => void;
  removeEventListener: (type: string, listener: EventListenerOrEventListenerObject) => void;
}

const SLOW_TYPES = new Set(['slow-2g', '2g', '3g']);

function getConnection(): ConnectionInfo | null {
  const nav = navigator as Navigator & { connection?: ConnectionInfo };
  return nav.connection ?? null;
}

let cachedNetworkQuality: NetworkQuality | undefined;

function buildNetworkQuality(): NetworkQuality {
  const conn = getConnection();
  if (!conn) {
    return {
      effectiveType: 'unknown',
      isSlowConnection: false,
      saveData: false,
      downlink: undefined,
      rtt: undefined,
    };
  }

  const effectiveType = (['slow-2g', '2g', '3g', '4g'] as const).includes(
    conn.effectiveType as 'slow-2g' | '2g' | '3g' | '4g',
  )
    ? (conn.effectiveType as 'slow-2g' | '2g' | '3g' | '4g')
    : 'unknown';

  return {
    effectiveType,
    isSlowConnection: SLOW_TYPES.has(conn.effectiveType),
    saveData: conn.saveData ?? false,
    downlink: conn.downlink,
    rtt: conn.rtt,
  };
}

function snapshotsEqual(a: NetworkQuality, b: NetworkQuality): boolean {
  return (
    a.effectiveType === b.effectiveType &&
    a.isSlowConnection === b.isSlowConnection &&
    a.saveData === b.saveData &&
    a.downlink === b.downlink &&
    a.rtt === b.rtt
  );
}

function getNetworkQuality(): NetworkQuality {
  const next = buildNetworkQuality();
  if (cachedNetworkQuality && snapshotsEqual(cachedNetworkQuality, next)) {
    return cachedNetworkQuality;
  }
  cachedNetworkQuality = next;
  return next;
}

function subscribeToNetworkChanges(onStoreChange: () => void): () => void {
  const conn = getConnection();
  if (!conn) {
    return () => {};
  }

  conn.addEventListener('change', onStoreChange);
  return () => conn.removeEventListener('change', onStoreChange);
}

export function useNetworkQuality(): NetworkQuality {
  return React.useSyncExternalStore(
    subscribeToNetworkChanges,
    getNetworkQuality,
    getNetworkQuality,
  );
}
