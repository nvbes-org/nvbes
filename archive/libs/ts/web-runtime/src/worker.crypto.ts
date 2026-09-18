import { defineWorker } from './worker';

const keyStore = new Map<number, CryptoKeyPair>();

let nextKeyId = 1;

async function encrypt(
  plaintext: ArrayBuffer,
  secret: ArrayBuffer,
): Promise<{
  encrypted: ArrayBuffer;
  iv: number[];
  salt: number[];
}> {
  const salt = crypto.getRandomValues(new Uint8Array(16));
  const iv = crypto.getRandomValues(new Uint8Array(12));

  const keyMaterial = await crypto.subtle.importKey('raw', secret, 'HKDF', false, ['deriveKey']);

  const encKey = await crypto.subtle.deriveKey(
    {
      name: 'HKDF',
      hash: 'SHA-256',
      salt,
      info: new TextEncoder().encode('nvbes-e2ee-request'),
    },
    keyMaterial,
    { name: 'AES-GCM', length: 256 },
    false,
    ['encrypt'],
  );

  const additionalData = new TextEncoder().encode('nvbes-e2ee-v1');
  const encrypted = await crypto.subtle.encrypt(
    { name: 'AES-GCM', iv, additionalData },
    encKey,
    plaintext,
  );

  return {
    encrypted,
    iv: Array.from(iv),
    salt: Array.from(salt),
  };
}

async function generateKeyPair(): Promise<{
  publicJwk: JsonWebKey;
  keyHandleId: number;
}> {
  const keyPair = await crypto.subtle.generateKey({ name: 'ECDSA', namedCurve: 'P-256' }, true, [
    'sign',
    'verify',
  ]);

  const publicJwk = await crypto.subtle.exportKey('jwk', keyPair.publicKey);
  const id = nextKeyId++;
  keyStore.set(id, keyPair);

  return { publicJwk, keyHandleId: id };
}

async function sign(keyHandleId: number, data: ArrayBuffer): Promise<ArrayBuffer> {
  const keyPair = keyStore.get(keyHandleId);
  if (!keyPair) throw new Error(`Key handle not found: ${keyHandleId}`);

  return crypto.subtle.sign({ name: 'ECDSA', hash: { name: 'SHA-256' } }, keyPair.privateKey, data);
}

defineWorker({ encrypt, generateKeyPair, sign });
