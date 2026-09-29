import { createLocalJWKSet, jwtVerify, type JWK } from 'jose';

export interface VerifiedIdentity {
  issuer: string;
  subject: string;
  clientId: string;
  nonce: string;
  authTime: number;
}

export async function verifyIdToken(input: {
  token: string | null;
  accessToken: string;
  issuer: string;
  clientId: string;
  nonce: string | null;
  fetchImpl?: typeof fetch;
  now?: () => number;
}): Promise<VerifiedIdentity> {
  if (!input.token || input.token.length > 16_384 || !input.nonce) {
    throw new Error('OIDC ID token and transaction nonce are required.');
  }
  const url = new URL(input.issuer);
  const local =
    url.protocol === 'http:' && ['localhost', '127.0.0.1', '[::1]'].includes(url.hostname);
  if (
    (url.protocol !== 'https:' && !local) ||
    url.username ||
    url.password ||
    url.pathname !== '/' ||
    url.search ||
    url.hash
  )
    throw new Error('Invalid OIDC issuer origin.');
  const issuer = url.origin;
  const response = await (input.fetchImpl ?? fetch)(`${issuer}/oauth/jwks`, {
    credentials: 'omit',
    redirect: 'error',
    cache: 'no-store',
    signal: AbortSignal.timeout(10_000),
    headers: { Accept: 'application/json' },
  });
  if (!response.ok || !response.body) throw new Error('OIDC keys unavailable.');
  const reader = response.body.getReader();
  const chunks: Uint8Array[] = [];
  let size = 0;
  try {
    for (;;) {
      const chunk = await reader.read();
      if (chunk.done) break;
      size += chunk.value.length;
      if (size > 65_536) throw new Error('OIDC keys response too large.');
      chunks.push(chunk.value);
    }
  } finally {
    await reader.cancel();
  }
  const bytes = new Uint8Array(size);
  let offset = 0;
  for (const chunk of chunks) {
    bytes.set(chunk, offset);
    offset += chunk.length;
  }
  const jwks: unknown = JSON.parse(new TextDecoder().decode(bytes));
  if (!record(jwks) || !Array.isArray(jwks.keys) || jwks.keys.length > 32)
    throw new Error('Invalid OIDC keys.');
  const keys: JWK[] = jwks.keys.map((key: unknown) => {
    if (
      !record(key) ||
      key.kty !== 'RSA' ||
      key.alg !== 'RS256' ||
      key.use !== 'sig' ||
      typeof key.kid !== 'string' ||
      typeof key.n !== 'string' ||
      typeof key.e !== 'string'
    ) {
      throw new Error('Unsupported OIDC signing key.');
    }
    return { kty: 'RSA', alg: 'RS256', use: 'sig', kid: key.kid, n: key.n, e: key.e };
  });
  const now = new Date((input.now ?? Date.now)());
  const { payload, protectedHeader } = await jwtVerify(input.token, createLocalJWKSet({ keys }), {
    issuer,
    audience: input.clientId,
    algorithms: ['RS256'],
    typ: 'JWT',
    requiredClaims: ['sub', 'iat', 'exp', 'auth_time', 'nonce', 'at_hash'],
    currentDate: now,
  });
  const seconds = Math.floor(now.getTime() / 1000);
  if (
    !protectedHeader.kid ||
    payload.aud !== input.clientId ||
    (payload.azp !== undefined && payload.azp !== input.clientId) ||
    payload.nonce !== input.nonce ||
    typeof payload.sub !== 'string' ||
    !payload.sub ||
    typeof payload.iat !== 'number' ||
    !Number.isSafeInteger(payload.iat) ||
    payload.iat > seconds + 30 ||
    typeof payload.exp !== 'number' ||
    !Number.isSafeInteger(payload.exp) ||
    payload.exp <= payload.iat ||
    typeof payload.auth_time !== 'number' ||
    !Number.isSafeInteger(payload.auth_time) ||
    payload.auth_time < 0 ||
    payload.auth_time > payload.iat
  )
    throw new Error('Invalid OIDC identity claims.');
  const digest = new Uint8Array(
    await crypto.subtle.digest('SHA-256', new TextEncoder().encode(input.accessToken)),
  );
  const atHash = btoa(String.fromCharCode(...digest.slice(0, 16)))
    .replace(/\+/g, '-')
    .replace(/\//g, '_')
    .replace(/=+$/, '');
  if (payload.at_hash !== atHash) throw new Error('OIDC access token binding failed.');
  return {
    issuer,
    subject: payload.sub,
    clientId: input.clientId,
    nonce: input.nonce,
    authTime: payload.auth_time,
  };
}

function record(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
