import { ExpirationPlugin } from 'workbox-expiration';
import { precacheAndRoute } from 'workbox-precaching';
import { NavigationRoute, registerRoute } from 'workbox-routing';
import { CacheFirst, NetworkOnly, StaleWhileRevalidate } from 'workbox-strategies';

import { captureDriveServiceWorkerException } from './drive.sw.sentry';

type ServiceWorkerFetchEvent = { request: Request };
type PrecacheManifestEntry = string | { url: string; revision?: string | null };
type DriveWorkerGlobal = typeof globalThis & { __WB_MANIFEST: PrecacheManifestEntry[] };

function imageStrategy(isSlowConnection: boolean) {
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

export function registerDriveServiceWorkerRoutes(isSlowConnection: boolean) {
  precacheAndRoute((self as unknown as DriveWorkerGlobal).__WB_MANIFEST);

  registerRoute(
    /\.(?:woff2?|ttf|otf)$/,
    new CacheFirst({
      cacheName: 'fonts',
      plugins: [new ExpirationPlugin({ maxEntries: 10, maxAgeSeconds: 30 * 24 * 60 * 60 })],
    }),
  );

  registerRoute(/\.(?:png|jpg|jpeg|svg|gif|webp|ico)$/, imageStrategy(isSlowConnection));

  registerRoute(/\/(?:auth|oauth|billing)\//, new NetworkOnly());

  registerRoute(
    new NavigationRoute(
      async ({ event }) => {
        const fetchEvent = event as unknown as ServiceWorkerFetchEvent;
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
}
