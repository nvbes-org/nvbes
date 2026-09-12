export async function encryptRequestBody(input: {
  body: string;
  keyId: string;
  method: string;
  secret: string;
  url: string;
}): Promise<{ body: ArrayBuffer; headers: Headers }> {
  if (!globalThis.crypto?.subtle) {
    throw new Error('Request E2EE requires Web Crypto');
  }
  if (input.secret.length < 32) {
    throw new Error('Request E2EE secret must be at least 32 characters long');
  }

  const salt: Uint8Array<ArrayBuffer> = crypto.getRandomValues(new Uint8Array(16));
  const nonce: Uint8Array<ArrayBuffer> = crypto.getRandomValues(new Uint8Array(12));
  const key = await deriveRequestEncryptionKey(input.secret, salt);
  const url = new URL(input.url);
  const additionalData = new TextEncoder().encode(`${input.method.toUpperCase()} ${url.pathname}`);
  const body = await crypto.subtle.encrypt(
    {
      name: 'AES-GCM',
      iv: nonce,
      additionalData,
    },
    key,
    new TextEncoder().encode(input.body),
  );

  const headers = new Headers({
    'Content-Type': 'application/octet-stream',
    'X-Nvbes-E2ee': 'aes-256-gcm',
    'X-Nvbes-E2ee-Key-Id': input.keyId,
    'X-Nvbes-E2ee-Nonce': base64Encode(nonce),
    'X-Nvbes-E2ee-Salt': base64Encode(salt),
  });

  return { body, headers };
}

async function deriveRequestEncryptionKey(
  secret: string,
  salt: Uint8Array<ArrayBuffer>,
): Promise<CryptoKey> {
  const material = await crypto.subtle.importKey(
    'raw',
    new TextEncoder().encode(secret),
    'HKDF',
    false,
    ['deriveKey'],
  );

  return crypto.subtle.deriveKey(
    {
      name: 'HKDF',
      hash: 'SHA-256',
      salt,
      info: new TextEncoder().encode('nvbes/request-body-e2ee/v1') as Uint8Array<ArrayBuffer>,
    },
    material,
    { name: 'AES-GCM', length: 256 },
    false,
    ['encrypt'],
  );
}

function base64Encode(bytes: Uint8Array): string {
  let binary = '';
  for (const byte of bytes) {
    binary += String.fromCharCode(byte);
  }
  return btoa(binary);
}
