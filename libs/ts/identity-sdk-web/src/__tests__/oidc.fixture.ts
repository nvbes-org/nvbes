import { exportJWK, generateKeyPair, SignJWT, type JWTPayload } from 'jose';

const pair = generateKeyPair('RS256');
export async function jwksResponse(): Promise<Response> {
  return new Response(
    JSON.stringify({
      keys: [
        { ...(await exportJWK((await pair).publicKey)), kid: 'test-key', alg: 'RS256', use: 'sig' },
      ],
    }),
  );
}
export async function idToken(
  nonce: string,
  accessToken: string,
  claims: JWTPayload = {},
  header = {},
): Promise<string> {
  const now = Math.floor(Date.now() / 1000);
  const hash = new Uint8Array(
    await crypto.subtle.digest('SHA-256', new TextEncoder().encode(accessToken)),
  );
  const atHash = btoa(String.fromCharCode(...hash.slice(0, 16)))
    .replace(/\+/g, '-')
    .replace(/\//g, '_')
    .replace(/=+$/, '');
  return new SignJWT({
    iss: 'https://identity.example',
    aud: 'account-web',
    sub: 'principal',
    nonce,
    iat: now,
    exp: now + 300,
    auth_time: now,
    at_hash: atHash,
    ...claims,
  })
    .setProtectedHeader({ alg: 'RS256', typ: 'JWT', kid: 'test-key', ...header })
    .sign((await pair).privateKey);
}
