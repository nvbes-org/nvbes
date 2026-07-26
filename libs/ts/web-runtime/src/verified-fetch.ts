import { z } from 'zod';

import { readVerifiedFetchCsrfToken } from './verified-fetch.csrf';

export type VerifiedFetchInput = RequestInfo | URL;

export interface VerifiedFetchOptions {
  allowedOrigins?: readonly string[];
  csrfCookieName?: string;
  csrfHeaderName?: string;
  credentials?: RequestCredentials;
  fetchImpl?: typeof fetch;
  headerName?: string;
  headerValue?: string;
  sameOrigin?: string | URL;
}

export type VerifiedFetchInit = RequestInit &
  VerifiedFetchOptions & {
    skipCsrf?: boolean;
  };

export class VerifiedFetchError extends Error {
  readonly kind: 'cross-origin' | 'http' | 'dto';
  readonly status?: number;
  readonly body?: unknown;
  readonly url?: string;
  readonly cause: unknown;

  constructor(input: {
    kind: 'cross-origin' | 'http' | 'dto';
    message: string;
    body?: unknown;
    cause?: unknown;
    status?: number;
    url?: string;
  }) {
    super(input.message);
    this.name = 'VerifiedFetchError';
    this.kind = input.kind;
    this.status = input.status;
    this.body = input.body;
    this.url = input.url;
    this.cause = input.cause;
  }
}

export function createVerifiedFetch(options: VerifiedFetchOptions = {}): typeof fetch {
  return (input, init) =>
    verifiedFetch(input, {
      ...init,
      allowedOrigins: options.allowedOrigins,
      csrfCookieName: options.csrfCookieName,
      csrfHeaderName: options.csrfHeaderName,
      credentials: init?.credentials ?? options.credentials,
      fetchImpl: options.fetchImpl,
      headerName: options.headerName,
      headerValue: options.headerValue,
      sameOrigin: options.sameOrigin,
    });
}

export async function verifiedFetch(
  input: VerifiedFetchInput,
  init: VerifiedFetchInit = {},
): Promise<Response> {
  const url = resolveVerifiedUrl(input, init.sameOrigin);
  assertAllowedOrigin(url, init);

  const headers = new Headers(readRequestHeaders(input));
  mergeHeaders(headers, init.headers);
  headers.set(init.headerName ?? 'Nvbes-Verified-Fetch', init.headerValue ?? '1');

  const method = readRequestMethod(input, init);
  const credentials = init.credentials ?? readRequestCredentials(input) ?? 'include';
  if (shouldAttachAjaxHeader(method) && !headers.has('X-Requested-With')) {
    headers.set('X-Requested-With', 'XMLHttpRequest');
  }
  if (!init.skipCsrf && shouldAttachCsrf(method, credentials) && !headers.has(csrfHeader(init))) {
    const csrfToken = readVerifiedFetchCsrfToken(init);
    if (csrfToken) {
      headers.set(csrfHeader(init), csrfToken);
    }
  }

  const {
    allowedOrigins,
    csrfCookieName,
    csrfHeaderName,
    fetchImpl,
    headerName,
    headerValue,
    sameOrigin,
    skipCsrf,
    ...requestInit
  } = init;
  void allowedOrigins;
  void csrfCookieName;
  void csrfHeaderName;
  void headerName;
  void headerValue;
  void sameOrigin;
  void skipCsrf;

  return (fetchImpl ?? fetch.bind(globalThis))(input, {
    ...requestInit,
    credentials,
    headers,
    method,
  });
}

export async function verifiedFetchJson<T>(
  input: VerifiedFetchInput,
  schema: z.ZodType<T>,
  init: VerifiedFetchInit = {},
): Promise<T> {
  const response = await verifiedFetch(input, init);
  const body = await readResponseBody(response);

  if (!response.ok) {
    throw new VerifiedFetchError({
      body,
      kind: 'http',
      message: readHttpErrorMessage(response, body),
      status: response.status,
      url: response.url,
    });
  }

  const parsed = schema.safeParse(body);
  if (!parsed.success) {
    throw new VerifiedFetchError({
      body,
      cause: parsed.error,
      kind: 'dto',
      message: 'Verified fetch response validation failed',
      url: response.url,
    });
  }

  return parsed.data;
}

function resolveVerifiedUrl(input: VerifiedFetchInput, sameOrigin?: string | URL): URL {
  const base = sameOrigin ? new URL(sameOrigin) : currentOriginUrl();
  if (typeof input === 'string') {
    return new URL(input, base);
  }
  if (input instanceof URL) {
    return new URL(input.toString(), base);
  }
  return new URL(input.url, base);
}

function currentOriginUrl(): URL {
  const origin = globalThis.location?.origin ?? 'http://localhost';
  return new URL(origin);
}

function assertAllowedOrigin(url: URL, init: VerifiedFetchInit): void {
  const sameOrigin = init.sameOrigin
    ? new URL(init.sameOrigin, currentOriginUrl()).origin
    : currentOriginUrl().origin;
  const allowed = new Set([sameOrigin]);
  for (const origin of init.allowedOrigins ?? []) {
    allowed.add(new URL(origin, currentOriginUrl()).origin);
  }

  if (!allowed.has(url.origin)) {
    throw new VerifiedFetchError({
      kind: 'cross-origin',
      message: `Verified fetch refused cross-origin request to ${url.origin}`,
      url: url.toString(),
    });
  }
}

function readRequestHeaders(input: VerifiedFetchInput): HeadersInit | undefined {
  if (typeof Request !== 'undefined' && input instanceof Request) {
    return input.headers;
  }
  return undefined;
}

function readRequestCredentials(input: VerifiedFetchInput): RequestCredentials | undefined {
  if (typeof Request !== 'undefined' && input instanceof Request) {
    return input.credentials;
  }
  return undefined;
}

function readRequestMethod(input: VerifiedFetchInput, init: RequestInit): string {
  if (init.method) {
    return init.method;
  }
  if (typeof Request !== 'undefined' && input instanceof Request) {
    return input.method;
  }
  return 'GET';
}

function mergeHeaders(target: Headers, source?: HeadersInit): void {
  if (!source) {
    return;
  }
  new Headers(source).forEach((value, key) => {
    target.set(key, value);
  });
}

const MUTATING_METHODS = new Set(['POST', 'PUT', 'PATCH', 'DELETE']);

function shouldAttachAjaxHeader(method: string): boolean {
  return MUTATING_METHODS.has(method.toUpperCase());
}

function shouldAttachCsrf(method: string, credentials: RequestCredentials): boolean {
  return credentials !== 'omit' && MUTATING_METHODS.has(method.toUpperCase());
}

function csrfHeader(init: VerifiedFetchInit): string {
  return init.csrfHeaderName ?? 'X-CSRF-Token';
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

function readHttpErrorMessage(response: Response, body: unknown): string {
  if (isErrorEnvelope(body)) {
    return body.error.message ?? `${response.status} ${response.statusText}`;
  }
  return `${response.status} ${response.statusText}`;
}

function isErrorEnvelope(value: unknown): value is { error: { message?: string } } {
  return (
    typeof value === 'object' &&
    value !== null &&
    'error' in value &&
    typeof value.error === 'object' &&
    value.error !== null
  );
}
