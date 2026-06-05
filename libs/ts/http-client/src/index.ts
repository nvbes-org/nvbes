import { z } from 'zod';

const ApiErrorEnvelopeSchema = z.object({
  error: z
    .object({
      code: z.string().optional(),
      message: z.string().optional(),
      request_id: z.string().optional(),
    })
    .optional(),
});

export type HttpClientOptions = {
  baseUrl?: string;
  credentials?: RequestCredentials;
  headers?: HeadersInit;
  fetchImpl?: typeof fetch;
  requestE2ee?: RequestE2eeOptions;
};

export type HttpRequestOptions = Omit<RequestInit, 'body' | 'headers'> & {
  body?: BodyInit | unknown;
  headers?: HeadersInit;
  requestE2ee?: RequestE2eeOptions | false;
};

export type RequestE2eeOptions = {
  keyId: string;
  secret: string;
};

export class HttpError extends Error {
  readonly status: number;
  readonly statusText: string;
  readonly body: unknown;
  readonly requestId: string | undefined;

  constructor(message: string, response: Response, body: unknown, requestId?: string) {
    super(message);
    this.name = 'HttpError';
    this.status = response.status;
    this.statusText = response.statusText;
    this.body = body;
    this.requestId = requestId;
  }
}

export class DtoValidationError extends Error {
  readonly cause: z.ZodError;

  constructor(cause: z.ZodError) {
    super('Response DTO validation failed');
    this.name = 'DtoValidationError';
    this.cause = cause;
  }
}

export class HttpClient {
  private readonly baseUrl: string;
  private readonly credentials?: RequestCredentials;
  private readonly headers?: HeadersInit;
  private readonly fetchImpl: typeof fetch;
  private readonly requestE2ee?: RequestE2eeOptions;

  constructor(options: HttpClientOptions = {}) {
    this.baseUrl = options.baseUrl ?? globalThis.location?.origin ?? 'http://localhost';
    this.credentials = options.credentials;
    this.headers = options.headers;
    this.fetchImpl = options.fetchImpl ?? fetch.bind(globalThis);
    this.requestE2ee = options.requestE2ee;
  }

  async get<T>(path: string, schema: z.ZodType<T>, options?: HttpRequestOptions): Promise<T> {
    return this.request(path, schema, { ...options, method: 'GET' });
  }

  async post<T>(
    path: string,
    schema: z.ZodType<T>,
    body?: unknown,
    options?: HttpRequestOptions,
  ): Promise<T> {
    return this.request(path, schema, { ...options, method: 'POST', body });
  }

  async delete<T>(path: string, schema: z.ZodType<T>, options?: HttpRequestOptions): Promise<T> {
    return this.request(path, schema, { ...options, method: 'DELETE' });
  }

  async request<T>(
    path: string,
    schema: z.ZodType<T>,
    options: HttpRequestOptions = {},
  ): Promise<T> {
    const url = this.resolveUrl(path);
    const response = await this.fetchImpl(url, await this.buildInit(url, options));
    const body = await readResponseBody(response);

    if (!response.ok) {
      const requestId = readRequestId(response, body);
      throw new HttpError(readErrorMessage(response, body), response, body, requestId);
    }

    const parsed = schema.safeParse(body);
    if (!parsed.success) {
      throw new DtoValidationError(parsed.error);
    }

    return parsed.data;
  }

  private async buildInit(url: string, options: HttpRequestOptions): Promise<RequestInit> {
    const headers = new Headers(this.headers);
    mergeHeaders(headers, options.headers);

    const credentials = options.credentials ?? this.credentials;
    const method = options.method ?? 'GET';
    if (credentials && MUTATING_METHODS.has(method.toUpperCase()) && !headers.has('X-CSRF-Token')) {
      const csrfToken = readCsrfToken();
      if (csrfToken) {
        headers.set('X-CSRF-Token', csrfToken);
      }
    }

    let body: BodyInit | undefined;
    if (options.body instanceof FormData || options.body instanceof URLSearchParams) {
      body = options.body;
    } else if (typeof options.body === 'string' || options.body instanceof Blob) {
      body = options.body;
    } else if (options.body !== undefined) {
      headers.set('Content-Type', headers.get('Content-Type') ?? 'application/json');
      body = JSON.stringify(options.body);
    }

    const requestE2ee =
      options.requestE2ee === false ? undefined : (options.requestE2ee ?? this.requestE2ee);
    if (requestE2ee && body !== undefined && typeof body === 'string') {
      const encrypted = await encryptRequestBody({
        body,
        keyId: requestE2ee.keyId,
        method,
        secret: requestE2ee.secret,
        url,
      });
      encrypted.headers.forEach((value, key) => headers.set(key, value));
      body = encrypted.body;
    }

    return {
      ...options,
      credentials: options.credentials ?? this.credentials,
      headers,
      body,
    };
  }

  private resolveUrl(path: string): string {
    const url = new URL(path, this.baseUrl);
    if (typeof window !== 'undefined' && window.location) {
      const pageParams = new URLSearchParams(window.location.search);
      const authuser = pageParams.get('authuser');
      if (authuser) {
        url.searchParams.set('authuser', authuser);
      }
    }
    return url.toString();
  }
}

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

export function createHttpClient(options?: HttpClientOptions): HttpClient {
  return new HttpClient(options);
}

const MUTATING_METHODS = new Set(['POST', 'PUT', 'PATCH', 'DELETE']);

function readCsrfToken(): string | undefined {
  if (typeof document === 'undefined') {
    return undefined;
  }
  const match = document.cookie.match(/(?:^|;\s*)csrf_token=([^;]*)/);
  return match?.[1] || undefined;
}

function mergeHeaders(target: Headers, source?: HeadersInit): void {
  if (!source) {
    return;
  }
  new Headers(source).forEach((value, key) => {
    target.set(key, value);
  });
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

async function readResponseBody(response: Response): Promise<unknown> {
  if (response.status === 204) {
    return undefined;
  }

  const text = await response.text();
  if (!text) {
    return undefined;
  }

  try {
    return JSON.parse(text) as unknown;
  } catch {
    return text;
  }
}

function readErrorMessage(response: Response, body: unknown): string {
  const envelope = ApiErrorEnvelopeSchema.safeParse(body);
  return envelope.success && envelope.data.error?.message
    ? envelope.data.error.message
    : `${response.status} ${response.statusText}`;
}

function readRequestId(response: Response, body: unknown): string | undefined {
  const envelope = ApiErrorEnvelopeSchema.safeParse(body);
  if (envelope.success && envelope.data.error?.request_id) {
    return envelope.data.error.request_id;
  }
  return response.headers.get('x-request-id') ?? undefined;
}
