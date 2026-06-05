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

type DpopMainKeyPair = {
  keyPair: CryptoKeyPair;
  publicJwk: JsonWebKey;
  jkt: string;
  usingWorker: false;
};

type DpopKeyPair = DpopWorkerKeyPair | DpopMainKeyPair;

let cryptoWorker: DpopWorkerCrypto | null = null;
let cachedKeyPair: DpopKeyPair | null = null;
let currentNonce: string | null = null;

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

async function computeJwkThumbprint(jwk: JsonWebKey): Promise<string> {
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

  const keyPair = await crypto.subtle.generateKey(
    {
      name: 'ECDSA',
      namedCurve: 'P-256',
    },
    true,
    ['sign'],
  );

  const publicJwk = await crypto.subtle.exportKey('jwk', keyPair.publicKey);
  const jkt = await computeJwkThumbprint(publicJwk);

  cachedKeyPair = { keyPair, publicJwk, jkt, usingWorker: false };
  return cachedKeyPair;
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

export function setDpopNonce(nonce: string): void {
  currentNonce = nonce;
}

export function getDpopNonce(): string | null {
  return currentNonce;
}

export function extractNonceFromResponse(headers: Headers): string | null {
  const nonce = headers.get(D_POP_NONCE_HEADER.toLowerCase());
  if (nonce) {
    currentNonce = nonce;
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

  const payload: Record<string, unknown> = {
    jti,
    htm: method.toUpperCase(),
    htu: url,
    iat,
  };

  if (accessToken) {
    const ath = await sha256Base64url(accessToken);
    payload.ath = ath;
  }

  if (currentNonce) {
    payload.nonce = currentNonce;
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
  options: RequestInit & { dpop?: boolean; accessToken?: string } = {},
): Promise<Response> {
  const { dpop = true, accessToken, ...fetchOptions } = options;

  if (!dpop) {
    return fetch(url, fetchOptions);
  }

  try {
    const keyPair = await ensureDpopKeyPair();
    const method = (fetchOptions.method ?? 'GET').toUpperCase();
    const proof = await createDpopProof(keyPair, method, url, accessToken);

    const headers = new Headers(fetchOptions.headers ?? {});
    headers.set(D_POP_HEADER, proof);

    if (accessToken) {
      headers.set('Authorization', `DPoP ${accessToken}`);
    }

    const response = await fetch(url, {
      ...fetchOptions,
      headers,
    });

    extractNonceFromResponse(response.headers);

    return response;
  } catch (error) {
    console.error('DPoP fetch failed:', error);
    return fetch(url, fetchOptions);
  }
}

export function isDpopSupported(): boolean {
  return (
    typeof crypto !== 'undefined' &&
    typeof crypto.subtle !== 'undefined' &&
    typeof crypto.subtle.generateKey === 'function'
  );
}
