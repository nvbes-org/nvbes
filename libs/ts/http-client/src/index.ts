import { z } from 'zod';

const ApiErrorEnvelopeSchema = z.object({
  error: z
    .object({
      code: z.string().optional(),
      message: z.string().optional(),
      recovery: z.enum(['reauthenticate']).optional(),
      request_id: z.string().optional(),
    })
    .optional(),
});

export type HttpRecovery = 'reauthenticate';

export type HttpClientOptions = {
  baseUrl?: string;
  credentials?: RequestCredentials;
  headers?: HeadersInit;
  fetchImpl?: typeof fetch;
  idempotencyKey?: string | false;
  requestE2ee?: RequestE2eeOptions;
};

export type HttpRequestOptions = Omit<RequestInit, 'body' | 'headers'> & {
  body?: unknown;
  headers?: HeadersInit;
  idempotencyKey?: string | false;
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
  readonly recovery: HttpRecovery | undefined;
  readonly requestId: string | undefined;

  constructor(message: string, response: Response, body: unknown, requestId?: string) {
    super(message);
    this.name = 'HttpError';
    this.status = response.status;
    this.statusText = response.statusText;
    this.body = body;
    this.recovery = readErrorRecovery(body);
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
  private readonly idempotencyKey?: string | false;
  private readonly requestE2ee?: RequestE2eeOptions;

  constructor(options: HttpClientOptions = {}) {
    this.baseUrl = options.baseUrl ?? globalThis.location?.origin ?? 'http://localhost';
    this.credentials = options.credentials;
    this.headers = options.headers;
    this.fetchImpl = options.fetchImpl ?? fetch.bind(globalThis);
    this.idempotencyKey = options.idempotencyKey;
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
      console.error('DTO validation failed', parsed.error.issues, body);
      throw new DtoValidationError(parsed.error);
    }

    return parsed.data;
  }

  private async buildInit(url: string, options: HttpRequestOptions): Promise<RequestInit> {
    const headers = new Headers(this.headers);
    mergeHeaders(headers, options.headers);

    const credentials = options.credentials ?? this.credentials;
    const method = options.method ?? 'GET';
    const authuser = currentAuthuser();
    if (authuser && !headers.has('X-Auth-User')) {
      headers.set('X-Auth-User', authuser);
    }
    applyAjaxRequestHeader(headers, method);
    applyIdempotencyKey(headers, method, options.idempotencyKey ?? this.idempotencyKey);
    if (credentials && MUTATING_METHODS.has(method.toUpperCase()) && !headers.has('X-CSRF-Token')) {
      const csrfToken = readCsrfToken(authuser);
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
      encrypted.headers.forEach((value, key) => {
        headers.set(key, value);
      });
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
    const url = resolveRequestUrl(path, this.baseUrl);
    const authuser = currentAuthuser();
    if (authuser) {
      url.searchParams.set('authuser', authuser);
    }
    return url.toString();
  }
}

export function resolveRequestUrl(path: string, baseUrl: string): URL {
  if (isAbsoluteUrl(path)) {
    return new URL(path);
  }

  const base = new URL(baseUrl);
  if (!path.startsWith('/') || base.pathname === '/') {
    return new URL(path, base);
  }

  const normalizedBase = new URL(base);
  normalizedBase.pathname = `${base.pathname.replace(/\/+$/u, '')}/`;
  return new URL(path.replace(/^\/+/u, ''), normalizedBase);
}

function isAbsoluteUrl(value: string): boolean {
  return /^[a-z][a-z\d+\-.]*:\/\//iu.test(value);
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

export function createRequestHeaders(
  method: string,
  headers?: HeadersInit,
  idempotencyKey?: string | false,
): Headers {
  const normalized = new Headers(headers);
  applyAjaxRequestHeader(normalized, method);
  applyIdempotencyKey(normalized, method, idempotencyKey);
  return normalized;
}

export function applyAjaxRequestHeader(headers: Headers, method: string): void {
  if (!MUTATING_METHODS.has(method.toUpperCase()) || headers.has('X-Requested-With')) {
    return;
  }

  headers.set('X-Requested-With', 'XMLHttpRequest');
}

export function applyIdempotencyKey(
  headers: Headers,
  method: string,
  idempotencyKey?: string | false,
): void {
  if (!IDEMPOTENCY_KEY_METHODS.has(method.toUpperCase())) {
    return;
  }

  if (headers.has('Idempotency-Key')) {
    return;
  }

  if (idempotencyKey === false) {
    return;
  }

  headers.set(
    'Idempotency-Key',
    typeof idempotencyKey === 'string' ? idempotencyKey : createIdempotencyKey(),
  );
}

export function createIdempotencyKey(): string {
  if (typeof crypto.randomUUID === 'function') {
    return crypto.randomUUID();
  }

  const bytes = crypto.getRandomValues(new Uint8Array(16));
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, '0')).join('');
}

const MUTATING_METHODS = new Set(['POST', 'PUT', 'PATCH', 'DELETE']);
const IDEMPOTENCY_KEY_METHODS = new Set(['POST', 'PUT', 'PATCH']);

function currentAuthuser(): string | undefined {
  if (typeof window === 'undefined' || !window.location) {
    return undefined;
  }

  const accountPathMatch = window.location.pathname.match(/^\/account\/([^/]+)(?:\/|$)/u);
  const fromAccountPath = accountPathMatch?.[1] ? decodeURIComponent(accountPathMatch[1]) : null;
  if (isAuthuser(fromAccountPath)) {
    return fromAccountPath;
  }

  const fromSearch = new URLSearchParams(window.location.search).get('authuser');
  if (isAuthuser(fromSearch)) {
    return fromSearch;
  }

  const pathMatch = window.location.pathname.match(/^\/u\/([^/]+)(?:\/|$)/u);
  const fromPath = pathMatch?.[1] ? decodeURIComponent(pathMatch[1]) : null;
  if (isAuthuser(fromPath)) {
    return fromPath;
  }

  return undefined;
}

function isAuthuser(value: string | null | undefined): value is string {
  return typeof value === 'string' && /^\d{1,3}$/u.test(value);
}

function readCsrfToken(authuser?: string): string | undefined {
  if (typeof document === 'undefined') {
    return undefined;
  }
  const cookies = parseDocumentCookies(document.cookie);
  const names =
    authuser && authuser !== '0'
      ? [`csrf_token_${authuser}`, `__Host-csrf_token_${authuser}`]
      : ['csrf_token', '__Host-csrf_token'];

  for (const name of names) {
    const value = cookies.get(name);
    if (value) {
      return value;
    }
  }

  return undefined;
}

function parseDocumentCookies(cookieHeader: string): Map<string, string> {
  const cookies = new Map<string, string>();
  for (const cookie of cookieHeader.split(';')) {
    const [name, ...valueParts] = cookie.trim().split('=');
    if (!name || valueParts.length === 0) {
      continue;
    }
    cookies.set(name, valueParts.join('='));
  }
  return cookies;
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

function readErrorRecovery(body: unknown): HttpRecovery | undefined {
  const envelope = ApiErrorEnvelopeSchema.safeParse(body);
  return envelope.success ? envelope.data.error?.recovery : undefined;
}
