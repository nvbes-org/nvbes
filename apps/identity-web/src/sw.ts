/// <reference lib="webworker" />

type PrecacheManifestEntry = string | { url: string; revision?: string | null };
type IdentityWorkerGlobal = ServiceWorkerGlobalScope & {
  __WB_MANIFEST: PrecacheManifestEntry[];
};
type PushPayload = {
  title?: unknown;
  body?: unknown;
  url?: unknown;
  tag?: unknown;
};

const worker = self as unknown as IdentityWorkerGlobal;
const precacheManifest = (self as unknown as IdentityWorkerGlobal).__WB_MANIFEST;
void precacheManifest;

const defaultNotificationUrl = '/account/notifications';
const defaultNotificationTitle = 'nvbes Identity';

function readPushPayload(event: PushEvent): PushPayload {
  try {
    const payload = event.data?.json();
    return typeof payload === 'object' && payload !== null ? (payload as PushPayload) : {};
  } catch {
    return {};
  }
}

function textValue(value: unknown): string | undefined {
  return typeof value === 'string' && value.trim().length > 0 ? value : undefined;
}

function notificationUrl(value: unknown): string {
  const rawUrl = textValue(value) ?? defaultNotificationUrl;
  return new URL(rawUrl, worker.location.origin).href;
}

worker.addEventListener('install', (event) => {
  event.waitUntil(worker.skipWaiting());
});

worker.addEventListener('activate', (event) => {
  event.waitUntil(worker.clients.claim());
});

worker.addEventListener('fetch', (event) => {
  if (event.request.mode !== 'navigate') return;
  event.respondWith(fetch(event.request));
});

worker.addEventListener('push', (event) => {
  const payload = readPushPayload(event);
  const title = textValue(payload.title) ?? defaultNotificationTitle;

  event.waitUntil(
    worker.registration.showNotification(title, {
      body: textValue(payload.body),
      data: { url: notificationUrl(payload.url) },
      icon: '/icon-192.png',
      badge: '/icon-180.png',
      tag: textValue(payload.tag),
    }),
  );
});

worker.addEventListener('notificationclick', (event) => {
  event.notification.close();
  const url = notificationUrl((event.notification.data as PushPayload | undefined)?.url);

  event.waitUntil(
    (async () => {
      const clients = await worker.clients.matchAll({
        includeUncontrolled: true,
        type: 'window',
      });

      for (const client of clients) {
        const windowClient = client as WindowClient;
        if (windowClient.url === url) {
          await windowClient.focus();
          return;
        }
      }

      await worker.clients.openWindow(url);
    })(),
  );
});
