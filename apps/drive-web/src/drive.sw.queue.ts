import { createRequestHeaders } from '@nvbes/http-client';

import { captureDriveServiceWorkerException } from './drive.sw.sentry';

async function openMutationsDb(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const req = indexedDB.open('nvbes-offline', 1);
    req.onupgradeneeded = () => {
      req.result.createObjectStore('mutations', {
        keyPath: 'id',
        autoIncrement: true,
      });
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

export async function replayQueue(tag: string) {
  const records = await readMutations(tag);
  for (const record of records) {
    try {
      const method = record.init?.method ?? 'GET';
      const headers = createRequestHeaders(method, record.init?.headers);
      await fetch(record.url, {
        ...record.init,
        headers,
      });
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
