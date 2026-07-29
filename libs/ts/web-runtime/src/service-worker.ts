import {
  createTrustedServiceWorkerScriptUrl,
  type NvbesTrustedScriptUrl,
} from "./trusted-types";

let readyResolve: (() => void) | null = null;
const readyPromise = new Promise<void>((resolve) => {
  readyResolve = resolve;
});

function onWindowLoad(handler: () => void | Promise<void>) {
  if (document.readyState === 'complete') {
    void handler();
  } else {
    window.addEventListener(
      'load',
      () => {
        void handler();
      },
      { once: true },
    );
  }
}

export function registerServiceWorker() {
  if (import.meta.env.DEV) return;
  if (!('serviceWorker' in navigator)) return;

  onWindowLoad(async () => {
    try {
      const serviceWorker = navigator.serviceWorker as Omit<ServiceWorkerContainer, 'register'> & {
        register(
          scriptURL: string | NvbesTrustedScriptUrl,
          options?: RegistrationOptions,
        ): Promise<ServiceWorkerRegistration>;
      };
      const registration = await serviceWorker.register(
        createTrustedServiceWorkerScriptUrl('/sw.js'),
      );
      if (readyResolve) {
        readyResolve();
      }
      registration.addEventListener('updatefound', () => {
        const installing = registration.installing;
        if (!installing) return;
        installing.addEventListener('statechange', () => {
          if (installing.state === 'installed' && navigator.serviceWorker.controller) {
            if (import.meta.env.DEV) {
              console.debug('[SW] Nouvelle version installee, rechargez pour appliquer.');
            }
          }
        });
      });
    } catch (error) {
      console.error('[SW] Service worker registration failed:', error);
    }
  });
}

// ---------------------------------------------------------------------------
// Install prompt (beforeinstallprompt)
// ---------------------------------------------------------------------------

type InstallPromptCallback = (prompt: () => Promise<void>) => void;

let deferredPrompt: (() => Promise<void>) | null = null;
const installPromptListeners = new Set<InstallPromptCallback>();

function handleBeforeInstallPrompt(event: Event) {
  if (installPromptListeners.size === 0) {
    return;
  }

  event.preventDefault();
  deferredPrompt = async () => {
    const ev = event as Event & {
      prompt: () => Promise<void>;
      userChoice: Promise<{ outcome: 'accepted' | 'dismissed' }>;
    };
    await ev.prompt();
    const choice = await ev.userChoice;
    if (choice.outcome === 'accepted') {
      deferredPrompt = null;
    }
  };
  for (const listener of installPromptListeners) {
    listener(deferredPrompt);
  }
}

if (typeof window !== 'undefined') {
  window.addEventListener('beforeinstallprompt', handleBeforeInstallPrompt);
}

export function onInstallReady(callback: InstallPromptCallback) {
  installPromptListeners.add(callback);
  if (deferredPrompt) {
    callback(deferredPrompt);
  }
  return () => {
    installPromptListeners.delete(callback);
  };
}

export function getInstallPrompt(): (() => Promise<void>) | null {
  return deferredPrompt;
}

export type WebAppDisplayMode =
  | 'browser'
  | 'standalone'
  | 'minimal-ui'
  | 'fullscreen'
  | 'window-controls-overlay';

type StandaloneNavigator = Navigator & { standalone?: boolean };
type BadgingNavigator = Navigator & {
  setAppBadge?: (contents?: number) => Promise<void>;
  clearAppBadge?: () => Promise<void>;
};

function matchesDisplayMode(mode: Exclude<WebAppDisplayMode, 'browser'>): boolean {
  return window.matchMedia?.(`(display-mode: ${mode})`).matches ?? false;
}

export function getWebAppDisplayMode(): WebAppDisplayMode {
  if (matchesDisplayMode('window-controls-overlay')) return 'window-controls-overlay';
  if (matchesDisplayMode('fullscreen')) return 'fullscreen';
  if (matchesDisplayMode('standalone')) return 'standalone';
  if (matchesDisplayMode('minimal-ui')) return 'minimal-ui';

  const navigatorWithStandalone = navigator as StandaloneNavigator;
  return navigatorWithStandalone.standalone ? 'standalone' : 'browser';
}

export function isInstalledWebApp(): boolean {
  return getWebAppDisplayMode() !== 'browser';
}

export function supportsAppBadging(): boolean {
  const badgingNavigator = navigator as BadgingNavigator;
  return (
    typeof badgingNavigator.setAppBadge === 'function' &&
    typeof badgingNavigator.clearAppBadge === 'function'
  );
}

export async function setAppBadge(count?: number): Promise<boolean> {
  const badgingNavigator = navigator as BadgingNavigator;
  if (typeof badgingNavigator.setAppBadge !== 'function') return false;

  try {
    await badgingNavigator.setAppBadge(count);
    return true;
  } catch {
    return false;
  }
}

export async function clearAppBadge(): Promise<boolean> {
  const badgingNavigator = navigator as BadgingNavigator;
  if (typeof badgingNavigator.clearAppBadge !== 'function') return false;

  try {
    await badgingNavigator.clearAppBadge();
    return true;
  } catch {
    return false;
  }
}

export type WebPushSupport =
  | { supported: true; permission: NotificationPermission }
  | {
      supported: false;
      permission: NotificationPermission | 'unsupported';
      reason: 'missing-notifications' | 'missing-service-worker' | 'missing-push-manager';
    };

export function getWebPushSupport(): WebPushSupport {
  if (!('Notification' in window)) {
    return { supported: false, permission: 'unsupported', reason: 'missing-notifications' };
  }

  if (!('serviceWorker' in navigator)) {
    return {
      supported: false,
      permission: Notification.permission,
      reason: 'missing-service-worker',
    };
  }

  if (!('PushManager' in window)) {
    return {
      supported: false,
      permission: Notification.permission,
      reason: 'missing-push-manager',
    };
  }

  return { supported: true, permission: Notification.permission };
}

export async function requestWebPushPermission(): Promise<NotificationPermission | 'unsupported'> {
  const support = getWebPushSupport();
  if (!support.supported) return support.permission;
  if (support.permission !== 'default') return support.permission;

  return Notification.requestPermission();
}

// ---------------------------------------------------------------------------
// Bidirectional communication with Service Worker
// ---------------------------------------------------------------------------

export async function sendToSw<T = unknown>(
  type: string,
  data?: Record<string, unknown>,
): Promise<T> {
  const sw = await getSwReady();
  if (!sw?.active) throw new Error('Service worker not active');

  return new Promise((resolve, reject) => {
    const channel = new MessageChannel();
    const id = crypto.randomUUID();

    channel.port1.onmessage = (event: MessageEvent<{ id: string; result?: T; error?: string }>) => {
      if (event.data?.id !== id) return;
      channel.port1.close();
      if (event.data.error) {
        reject(new Error(event.data.error));
      } else {
        resolve(event.data.result as T);
      }
    };

    sw.active!.postMessage({ type, id, ...data }, [channel.port2]);
  });
}

// ---------------------------------------------------------------------------
// Periodic Background Sync
// ---------------------------------------------------------------------------

export async function registerPeriodicSync(
  tag: string,
  minIntervalMs: number = 24 * 60 * 60 * 1000,
) {
  if (!('serviceWorker' in navigator)) return false;
  const sw = await navigator.serviceWorker.ready;

  if ('periodicSync' in sw) {
    try {
      await (
        sw as ServiceWorkerRegistration & {
          periodicSync: {
            register: (tag: string, options: { minInterval: number }) => Promise<void>;
          };
        }
      ).periodicSync.register(tag, { minInterval: minIntervalMs });
      return true;
    } catch {
      return false;
    }
  }
  return false;
}

// ---------------------------------------------------------------------------
// Background Fetch
// ---------------------------------------------------------------------------

export async function startBackgroundFetch(
  id: string,
  requests: RequestInfo[],
  title: string,
): Promise<boolean> {
  if (!('serviceWorker' in navigator)) return false;
  const sw = await navigator.serviceWorker.ready;

  if ('backgroundFetch' in sw) {
    try {
      await (
        sw as ServiceWorkerRegistration & {
          backgroundFetch: {
            fetch: (
              id: string,
              requests: RequestInfo[],
              options: { title: string },
            ) => Promise<void>;
          };
        }
      ).backgroundFetch.fetch(id, requests, { title });
      return true;
    } catch {
      return false;
    }
  }
  return false;
}

type BgFetchEvent = { type: string; id: string; results?: { url: string; size: number }[] };

export function onBackgroundFetchEvent(callback: (event: BgFetchEvent) => void): () => void {
  const handler = (event: MessageEvent<BgFetchEvent>) => {
    if (event.data?.type?.startsWith('backgroundfetch')) {
      callback(event.data);
    }
  };
  navigator.serviceWorker.addEventListener('message', handler);
  return () => navigator.serviceWorker.removeEventListener('message', handler);
}

export async function getSwReady(): Promise<ServiceWorkerRegistration | null> {
  if (!('serviceWorker' in navigator)) return null;
  await readyPromise;
  return navigator.serviceWorker.ready;
}

export async function registerBackgroundSync(
  tag: string,
  _handler: (event: Event) => Promise<void>,
) {
  if (!('serviceWorker' in navigator)) return;
  const sw = await navigator.serviceWorker.ready;
  const active = sw.active;
  if (!active) return;

  const channel = new MessageChannel();
  navigator.serviceWorker.addEventListener('message', (event) => {
    if (event.data?.type === 'sync' && event.data?.tag === tag) {
      void _handler(event);
    }
  });

  active.postMessage({ type: 'register-sync', tag }, [channel.port2]);
}

export async function queueMutation(tag: string, payload: unknown) {
  const db = await openMutationStore();
  const tx = db.transaction('mutations', 'readwrite');
  tx.objectStore('mutations').add({
    tag,
    payload,
    timestamp: Date.now(),
  });
  await new Promise<void>((resolve, reject) => {
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(tx.error);
  });

  if (!('serviceWorker' in navigator)) {
    return;
  }

  const sw = await navigator.serviceWorker.ready;
  const registration = sw as ServiceWorkerRegistration & {
    sync?: { register(tag: string): Promise<void> };
  };

  if (registration.sync) {
    try {
      await registration.sync.register(tag);
    } catch {
      // Background sync not available, will retry on next online event
    }
  }
}

async function openMutationStore(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const req = indexedDB.open('nvbes-offline', 1);
    req.onupgradeneeded = () => {
      req.result.createObjectStore('mutations', { keyPath: 'id', autoIncrement: true });
    };
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  });
}

// ---------------------------------------------------------------------------
// Network Quality
// ---------------------------------------------------------------------------

export async function sendNetworkQualityToSw(quality: {
  effectiveType: string;
  isSlowConnection: boolean;
  saveData: boolean;
  downlink: number | undefined;
  rtt: number | undefined;
}): Promise<void> {
  if (!('serviceWorker' in navigator)) return;
  try {
    const sw = await navigator.serviceWorker.ready;
    sw.active?.postMessage({
      type: 'NETWORK_QUALITY',
      ...quality,
    });
  } catch {
    // SW not available
  }
}

export async function migrateServiceWorkers(): Promise<number> {
  if (!('serviceWorker' in navigator)) return 0;
  const registrations = await navigator.serviceWorker.getRegistrations();
  let unregistered = 0;
  for (const reg of registrations) {
    const scriptUrl = reg.active?.scriptURL || reg.installing?.scriptURL || reg.waiting?.scriptURL;
    if (scriptUrl && scriptUrl.includes('/sw.js')) continue;
    await reg.unregister();
    unregistered++;
  }
  return unregistered;
}
