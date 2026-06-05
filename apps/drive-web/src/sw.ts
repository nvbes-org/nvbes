/// <reference lib="webworker" />

import { clientsClaim } from 'workbox-core';
import { ExpirationPlugin } from 'workbox-expiration';
import { precacheAndRoute } from 'workbox-precaching';
import { NavigationRoute, registerRoute } from 'workbox-routing';
import { CacheFirst, NetworkOnly, StaleWhileRevalidate } from 'workbox-strategies';
import {
  captureDriveServiceWorkerException,
  setDriveServiceWorkerSentryConsent,
} from './drive.sw.sentry';

declare const self: ServiceWorkerGlobalScope;

let isSlowConnection = false;

interface PeriodicSyncEvent extends ExtendableEvent {
  readonly tag: string;
}

interface BackgroundFetchRecord {
  readonly request: Request;
  readonly responseReady: Promise<Response>;
}

interface BackgroundFetchRegistration {
  readonly id: string;
  matchAll(): Promise<BackgroundFetchRecord[]>;
}

interface BackgroundFetchEvent extends ExtendableEvent {
  readonly registration: BackgroundFetchRegistration;
  updateUI(options: { title: string }): Promise<void>;
}

self.addEventListener('error', (event) => {
  captureDriveServiceWorkerException(event.error ?? event.message, 'global.error', {
    filename: event.filename,
    lineno: event.lineno,
    colno: event.colno,
  });
});

self.addEventListener('unhandledrejection', (event) => {
  captureDriveServiceWorkerException(event.reason, 'global.unhandledrejection');
});

clientsClaim();
precacheAndRoute(self.__WB_MANIFEST);

registerRoute(
  /\.(?:woff2?|ttf|otf)$/,
  new CacheFirst({
    cacheName: 'fonts',
    plugins: [new ExpirationPlugin({ maxEntries: 10, maxAgeSeconds: 30 * 24 * 60 * 60 })],
  }),
);

function imageStrategy() {
  return isSlowConnection
    ? new CacheFirst({
        cacheName: 'images',
        plugins: [new ExpirationPlugin({ maxEntries: 60, maxAgeSeconds: 7 * 24 * 60 * 60 })],
      })
    : new StaleWhileRevalidate({
        cacheName: 'images',
        plugins: [new ExpirationPlugin({ maxEntries: 60, maxAgeSeconds: 7 * 24 * 60 * 60 })],
      });
}

registerRoute(/\.(?:png|jpg|jpeg|svg|gif|webp|ico)$/, imageStrategy());

registerRoute(/\/(?:auth|oauth|billing)\//, new NetworkOnly());

registerRoute(
  new NavigationRoute(
    async ({ event }) => {
      const fetchEvent = event as FetchEvent;
      try {
        return await fetch(fetchEvent.request);
      } catch (error) {
        captureDriveServiceWorkerException(error, 'navigation.fetch', {
          url: fetchEvent.request.url,
          method: fetchEvent.request.method,
        });
        return caches.match('/offline.html') as Promise<Response>;
      }
    },
    {
      denylist: [/^\/api\//, /^\/auth\//, /^\/oauth\//],
    },
  ),
);

// ---------------------------------------------------------------------------
// Bidirectional message channel
// ---------------------------------------------------------------------------

self.addEventListener('message', (event: ExtendableMessageEvent) => {
  const { type, id } = event.data || {};

  if (type === 'SKIP_WAITING') {
    void self.skipWaiting();
    return;
  }

  if (type === 'NETWORK_QUALITY') {
    isSlowConnection = event.data.isSlowConnection === true;
    return;
  }

  if (type === 'SENTRY_CONSENT_UPDATED') {
    setDriveServiceWorkerSentryConsent(event.data?.sentryAccepted === true);
    return;
  }

  const port = event.ports?.[0];
  if (!port) return;

  switch (type) {
    case 'PING': {
      port.postMessage({ id, result: 'pong' });
      return;
    }
    case 'SYNC_NOW': {
      if (isSlowConnection) {
        port.postMessage({ id, error: 'Sync deferred: slow connection' });
        return;
      }
      const tag = event.data?.tag;
      if (tag) {
        event.waitUntil(replayQueue(tag).finally(() => port.postMessage({ id, result: 'ok' })));
      } else {
        port.postMessage({ id, error: 'Missing tag' });
      }
      return;
    }
    case 'REGISTER_PERIODIC_SYNC': {
      port.postMessage({ id, result: 'ok' });
      return;
    }
    default:
      port.postMessage({ id, error: `Unknown message type: ${type}` });
  }
});

// ---------------------------------------------------------------------------
// Periodic Background Sync
// ---------------------------------------------------------------------------

self.addEventListener('periodicsync', (event) => {
  const syncEvent = event as PeriodicSyncEvent;
  syncEvent.waitUntil(replayQueue(syncEvent.tag));
});

// ---------------------------------------------------------------------------
// Background Fetch
// ---------------------------------------------------------------------------

self.addEventListener('backgroundfetchsuccess', (event) => {
  const fetchEvent = event as BackgroundFetchEvent;
  fetchEvent.waitUntil(
    (async () => {
      const records = await fetchEvent.registration.matchAll();
      const results = [];
      for (const record of records) {
        results.push({
          url: record.request.url,
          size: record.responseReady.then((r) => r.headers.get('content-length')),
        });
      }
      await fetchEvent.updateUI({ title: 'Telechargement termine' });

      const allClients = await self.clients.matchAll();
      for (const client of allClients) {
        client.postMessage({
          type: 'backgroundfetchsuccess',
          id: fetchEvent.registration.id,
          results,
        });
      }
    })(),
  );
});

self.addEventListener('backgroundfetchfail', (event) => {
  const fetchEvent = event as BackgroundFetchEvent;
  fetchEvent.waitUntil(
    (async () => {
      await fetchEvent.updateUI({ title: 'Telechargement echoue' });

      const allClients = await self.clients.matchAll();
      for (const client of allClients) {
        client.postMessage({
          type: 'backgroundfetchfail',
          id: fetchEvent.registration.id,
        });
      }
    })(),
  );
});

self.addEventListener('backgroundfetchclick', (event) => {
  const fetchEvent = event as BackgroundFetchEvent;
  fetchEvent.waitUntil(self.clients.openWindow('/downloads'));
});

// ---------------------------------------------------------------------------
// Queue replay (IndexedDB)
// ---------------------------------------------------------------------------

async function replayQueue(tag: string) {
  const records = await readMutations(tag);
  for (const record of records) {
    try {
      await fetch(record.url, record.init);
    } catch (error) {
      captureDriveServiceWorkerException(error, 'queue.replay', {
        tag,
        url: record.url,
      });
      // Will retry on next sync
    }
  }
  await clearMutations(tag);
}

function openMutationsDb(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const req = indexedDB.open('nvbes-offline', 1);
    req.onupgradeneeded = () => {
      req.result.createObjectStore('mutations', { keyPath: 'id', autoIncrement: true });
    };
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  });
}

async function readMutations(
  tag: string,
): Promise<{ id: number; url: string; init: RequestInit }[]> {
  const db = await openMutationsDb();
  return new Promise((resolve, reject) => {
    const tx = db.transaction('mutations', 'readonly');
    const store = tx.objectStore('mutations');
    const req = store.getAll();
    req.onsuccess = () => resolve(req.result.filter((r: { tag: string }) => r.tag === tag));
    req.onerror = () => reject(req.error);
  });
}

async function clearMutations(tag: string) {
  const db = await openMutationsDb();
  const tx = db.transaction('mutations', 'readwrite');
  const store = tx.objectStore('mutations');
  const req = store.getAll();
  req.onsuccess = () => {
    for (const record of req.result) {
      if (record.tag === tag) store.delete(record.id);
    }
  };
  await new Promise<void>((resolve) => {
    tx.oncomplete = () => resolve();
  });
}
