import type { HttpRequestContextHeadersProvider } from './http.types';

const MUTATING_METHODS = new Set(['POST', 'PUT', 'PATCH', 'DELETE']);
const IDEMPOTENCY_KEY_METHODS = new Set(['POST', 'PUT', 'PATCH']);

let requestContextHeadersProvider: HttpRequestContextHeadersProvider | undefined;

export function configureHttpRequestContextHeaders(
  provider?: HttpRequestContextHeadersProvider,
): void {
  requestContextHeadersProvider = provider;
}

export async function requestContextHeaders(): Promise<HeadersInit | undefined> {
  return requestContextHeadersProvider?.();
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

export function sameOrigin(url: string, baseUrl: string): boolean {
  return new URL(url).origin === new URL(baseUrl).origin;
}

export function stripCrossOriginRequestContext(headers: Headers): void {
  headers.delete('Authorization');
  headers.delete('Proxy-Authorization');
  headers.delete('Cookie');
  headers.delete('DPoP');
  headers.delete('X-Auth-User');
  headers.delete('X-CSRF-Token');
  headers.delete('Idempotency-Key');
  headers.delete('X-Requested-With');
}

export function currentAuthuser(): string | undefined {
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

export function readCsrfToken(authuser?: string): string | undefined {
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

export function mergeHeaders(target: Headers, source?: HeadersInit): void {
  if (!source) {
    return;
  }
  new Headers(source).forEach((value, key) => {
    target.set(key, value);
  });
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

  if (headers.has('Idempotency-Key') || idempotencyKey === false) {
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

export function isMutatingMethod(method: string): boolean {
  return MUTATING_METHODS.has(method.toUpperCase());
}

function isAbsoluteUrl(value: string): boolean {
  return /^[a-z][a-z\d+\-.]*:\/\//iu.test(value);
}

function isAuthuser(value: string | null | undefined): value is string {
  return typeof value === 'string' && /^\d{1,3}$/u.test(value);
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
