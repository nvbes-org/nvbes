import { createWorker } from './worker';

type CryptoWorkerMethods = {
  encrypt: (
    plaintext: ArrayBuffer,
    secret: ArrayBuffer,
  ) => Promise<{
    encrypted: ArrayBuffer;
    iv: number[];
    salt: number[];
  }>;
  generateKeyPair: () => Promise<{
    publicJwk: JsonWebKey;
    keyHandleId: number;
  }>;
  sign: (keyHandleId: number, data: ArrayBuffer) => Promise<ArrayBuffer>;
};

let instance: ReturnType<typeof createWorker<CryptoWorkerMethods>> | null = null;

function getWorker() {
  if (!instance) {
    instance = createWorker<CryptoWorkerMethods>(
      () => new Worker(new URL('./worker.crypto.ts', import.meta.url), { type: 'module' }),
    );
  }
  return instance;
}

export const cryptoWorker = {
  encrypt(plaintext: ArrayBuffer, secret: ArrayBuffer) {
    return getWorker().encrypt(plaintext, secret);
  },

  generateKeyPair() {
    return getWorker().generateKeyPair();
  },

  sign(keyHandleId: number, data: ArrayBuffer) {
    return getWorker().sign(keyHandleId, data);
  },
};
