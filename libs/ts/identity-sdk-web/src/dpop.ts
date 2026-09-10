import { createRequestHeaders } from '@nvbes/http-client';

const D_POP_NONCE_HEADER = 'DPoP-Nonce';
const D_POP_HEADER = 'DPoP';

interface DpopWorkerCrypto {
  generateKeyPair(): Promise<{ publicJwk: JsonWebKey; handle: number }>;
  sign(handle: number, data: string): Promise<ArrayBuffer>;
}

type DpopWorkerKeyPair = {
  handle: number;
  publicJwk: JsonWebKey;
  jkt: string;
  usingWorker: true;
};

export type DpopMainKeyPair = {
  keyPair: CryptoKeyPair;
  publicJwk: JsonWebKey;
  jkt: string;
  usingWorker: false;
};

export type DpopKeyPair = DpopWorkerKeyPair | DpopMainKeyPair;

let cryptoWorker: DpopWorkerCrypto | null = null;
let cachedKeyPair: DpopKeyPair | null = null;
const nonces = new Map<string, string>();

export function configureDpopCryptoWorker(worker: DpopWorkerCrypto): void {
  cryptoWorker = worker;
}

function base64urlEncode(input: ArrayBuffer | Uint8Array): string {
  const bytes = input instanceof Uint8Array ? input : new Uint8Array(input);
  let binary = '';
  for (let i = 0; i < bytes.length; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

async function sha256(input: string): Promise<ArrayBuffer> {
  const encoder = new TextEncoder();
  const data = encoder.encode(input);
  return crypto.subtle.digest('SHA-256', data);
}

async function sha256Base64url(input: string): Promise<string> {
  const hash = await sha256(input);
  return base64urlEncode(hash);
}

export async function computeJwkThumbprint(jwk: JsonWebKey): Promise<string> {
  const canonical = {
    crv: jwk.crv,
    kty: jwk.kty,
    x: jwk.x,
    y: jwk.y,
  };
  const json = JSON.stringify(canonical);
  return sha256Base64url(json);
}

export async function generateDpopKeyPair(): Promise<DpopKeyPair> {
  if (cryptoWorker) {
    const { publicJwk, handle } = await cryptoWorker.generateKeyPair();
    const jkt = await computeJwkThumbprint(publicJwk);

    cachedKeyPair = { handle, publicJwk, jkt, usingWorker: true };
    return cachedKeyPair;
  }

  cachedKeyPair = await generateBrowserDpopKeyPair();
  return cachedKeyPair;
}

/** Independent key for one OAuth transaction; never changes the legacy global cache. */
export async function generateBrowserDpopKeyPair(): Promise<DpopMainKeyPair> {
  const keyPair = await crypto.subtle.generateKey(
    {
      name: 'ECDSA',
      namedCurve: 'P-256',
    },
    false,
    ['sign'],
  );

  const publicJwk = await crypto.subtle.exportKey('jwk', keyPair.publicKey);
  const jkt = await computeJwkThumbprint(publicJwk);

  return { keyPair, publicJwk, jkt, usingWorker: false };
}

export function getCachedKeyPair(): DpopKeyPair | null {
  return cachedKeyPair;
}

export function getCachedJkt(): string | null {
  return cachedKeyPair?.jkt ?? null;
}

export function getCachedPublicJwk(): JsonWebKey | null {
  return cachedKeyPair?.publicJwk ?? null;
}

export async function ensureDpopKeyPair(): Promise<DpopKeyPair> {
  if (cachedKeyPair) return cachedKeyPair;
  return generateDpopKeyPair();
}

export function setDpopNonce(nonce: string, serverUrl: string): void {
  const origin = new URL(serverUrl).origin;
  if (!nonce) {
    nonces.delete(origin);
    return;
  }
  if (nonce.length > 1024) throw new Error('DPoP nonce is too large');
  nonces.delete(origin);
  if (nonces.size >= 32) {
    const oldest = nonces.keys().next().value;
    if (oldest !== undefined) nonces.delete(oldest);
  }
  nonces.set(origin, nonce);
}

export function getDpopNonce(serverUrl: string): string | null {
  return nonces.get(new URL(serverUrl).origin) ?? null;
}

export function extractNonceFromResponse(headers: Headers, serverUrl: string): string | null {
  const nonce = headers.get(D_POP_NONCE_HEADER.toLowerCase());
  if (nonce) {
    setDpopNonce(nonce, serverUrl);
  }
  return nonce;
}

export async function createDpopProof(
  keyPair: DpopKeyPair,
  method: string,
  url: string,
  accessToken?: string,
): Promise<string> {
  const jti = crypto.randomUUID();
  const iat = Math.floor(Date.now() / 1000);
  const target = new URL(url);
  target.search = '';
  target.hash = '';

  const payload: Record<string, unknown> = {
    jti,
    htm: method.toUpperCase(),
    htu: target.href,
    iat,
  };

  if (accessToken) {
    const ath = await sha256Base64url(accessToken);
    payload.ath = ath;
  }

  const nonce = getDpopNonce(url);
  if (nonce) {
    payload.nonce = nonce;
  }

  const publicJwk: Record<string, unknown> = {
    kty: keyPair.publicJwk.kty,
    crv: keyPair.publicJwk.crv,
    x: keyPair.publicJwk.x,
    y: keyPair.publicJwk.y,
  };

  const header = {
    typ: 'dpop+jwt',
    alg: 'ES256',
    jwk: publicJwk,
  };

  const headerB64 = base64urlEncode(new TextEncoder().encode(JSON.stringify(header)));
  const payloadB64 = base64urlEncode(new TextEncoder().encode(JSON.stringify(payload)));
  const signingInput = `${headerB64}.${payloadB64}`;

  let signature: ArrayBuffer;
  if (keyPair.usingWorker && cryptoWorker) {
    signature = await cryptoWorker.sign(keyPair.handle, signingInput);
  } else if (!keyPair.usingWorker) {
    signature = await crypto.subtle.sign(
      { name: 'ECDSA', hash: { name: 'SHA-256' } },
      keyPair.keyPair.privateKey,
      new TextEncoder().encode(signingInput),
    );
  } else {
    throw new Error('DPoP crypto worker configured but unavailable for signing');
  }

  const signatureB64 = base64urlEncode(signature);

  return `${signingInput}.${signatureB64}`;
}

export async function dpopFetch(
  url: string,
  options: RequestInit & { dpop?: boolean; accessToken?: string; key?: DpopKeyPair } = {},
): Promise<Response> {
  const { dpop = true, accessToken, key, ...fetchOptions } = options;
  const method = (fetchOptions.method ?? 'GET').toUpperCase();
  const headers = createRequestHeaders(method, fetchOptions.headers);

  if (!dpop) {
    return fetch(url, {
      ...fetchOptions,
      headers,
    });
  }

  const keyPair = key ?? (await ensureDpopKeyPair());
  const proof = await createDpopProof(keyPair, method, url, accessToken);
  headers.set(D_POP_HEADER, proof);

  if (accessToken) {
    headers.set('Authorization', `DPoP ${accessToken}`);
  }

  const response = await fetch(url, {
    ...fetchOptions,
    redirect: 'error',
    credentials: 'omit',
    headers,
  });

  extractNonceFromResponse(response.headers, url);
  return response;
}

export function isDpopSupported(): boolean {
  return (
    typeof crypto !== 'undefined' &&
    typeof crypto.subtle !== 'undefined' &&
    typeof crypto.subtle.generateKey === 'function'
  );
}
