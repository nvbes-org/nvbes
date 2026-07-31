/// <reference lib="webworker" />

type PrecacheManifestEntry = string | { url: string; revision?: string | null };
type AccountWorkerGlobal = ServiceWorkerGlobalScope & {
  __WB_MANIFEST: PrecacheManifestEntry[];
};

const worker = self as unknown as AccountWorkerGlobal;
const precacheManifest = (self as unknown as AccountWorkerGlobal).__WB_MANIFEST;
void precacheManifest;

worker.addEventListener('install', (event) => {
  event.waitUntil(worker.skipWaiting());
});

worker.addEventListener('activate', (event) => {
  event.waitUntil(worker.clients.claim());
});

worker.addEventListener('fetch', (event) => {
  if (event.request.mode === 'navigate') {
    event.respondWith(fetch(event.request));
  }
});
