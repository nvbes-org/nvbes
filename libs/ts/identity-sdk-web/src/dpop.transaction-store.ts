import { computeJwkThumbprint, type DpopMainKeyPair } from './dpop';

export interface DpopTransactionStore {
  save(id: string, key: DpopMainKeyPair, expiresAt: number): Promise<void>;
  load(id: string): Promise<DpopMainKeyPair | null>;
  remove(id: string): Promise<void>;
}

/** Stores structured-cloned CryptoKeys, never JWK private material. No memory fallback. */
export class IndexedDbDpopTransactionStore implements DpopTransactionStore {
  constructor(private readonly databaseName = 'nvbes.identity.dpop.transactions') {}

  private open(): Promise<IDBDatabase> {
    return new Promise((resolve, reject) => {
      const request = indexedDB.open(this.databaseName, 1);
      request.onupgradeneeded = () => request.result.createObjectStore('keys');
      request.onerror = () => reject(new Error('DPoP key database unavailable'));
      request.onblocked = () => reject(new Error('DPoP key database blocked'));
      request.onsuccess = () => resolve(request.result);
    });
  }

  async save(id: string, key: DpopMainKeyPair, expiresAt: number): Promise<void> {
    if (
      key.keyPair.privateKey.extractable ||
      !Number.isFinite(expiresAt) ||
      expiresAt <= Date.now()
    ) {
      throw new Error('Invalid pending DPoP key');
    }
    const db = await this.open();
    try {
      await new Promise<void>((resolve, reject) => {
        const tx = db.transaction('keys', 'readwrite');
        const store = tx.objectStore('keys');
        const scan = store.openCursor();
        let retained = 0;
        scan.onsuccess = () => {
          const cursor = scan.result;
          if (cursor) {
            const value: unknown = cursor.value;
            if (
              !isRecord(value) ||
              typeof value.expiresAt !== 'number' ||
              value.expiresAt <= Date.now()
            ) {
              cursor.delete();
            } else {
              retained++;
            }
            cursor.continue();
          } else if (retained >= 32) {
            tx.abort();
          } else {
            store.add({ key, expiresAt }, id);
          }
        };
        tx.oncomplete = () => resolve();
        tx.onabort = () => reject(new Error('DPoP key storage failed or reached capacity'));
        tx.onerror = () => reject(new Error('DPoP key storage failed'));
      });
    } finally {
      db.close();
    }
  }

  async load(id: string): Promise<DpopMainKeyPair | null> {
    const db = await this.open();
    let value: unknown;
    try {
      value = await new Promise<unknown>((resolve, reject) => {
        const request = db.transaction('keys', 'readonly').objectStore('keys').get(id);
        request.onsuccess = () => resolve(request.result);
        request.onerror = () => reject(new Error('DPoP key read failed'));
      });
    } finally {
      db.close();
    }
    if (value === undefined) return null;
    if (!isRecord(value) || typeof value.expiresAt !== 'number' || value.expiresAt <= Date.now()) {
      await this.remove(id);
      return null;
    }
    const key = value.key;
    if (
      !isRecord(key) ||
      key.usingWorker !== false ||
      !isRecord(key.keyPair) ||
      !(key.keyPair.privateKey instanceof CryptoKey) ||
      key.keyPair.privateKey.extractable ||
      !(key.keyPair.publicKey instanceof CryptoKey)
    ) {
      throw new Error('Invalid stored DPoP key');
    }
    const publicJwk = await crypto.subtle.exportKey('jwk', key.keyPair.publicKey);
    const jkt = await computeJwkThumbprint(publicJwk);
    if (key.jkt !== jkt) throw new Error('Stored DPoP thumbprint mismatch');
    return {
      keyPair: { privateKey: key.keyPair.privateKey, publicKey: key.keyPair.publicKey },
      publicJwk,
      jkt,
      usingWorker: false,
    };
  }

  async remove(id: string): Promise<void> {
    const db = await this.open();
    try {
      await new Promise<void>((resolve, reject) => {
        const tx = db.transaction('keys', 'readwrite');
        tx.objectStore('keys').delete(id);
        tx.oncomplete = () => resolve();
        tx.onabort = () => reject(new Error('DPoP key deletion failed'));
        tx.onerror = () => reject(new Error('DPoP key deletion failed'));
      });
    } finally {
      db.close();
    }
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}
