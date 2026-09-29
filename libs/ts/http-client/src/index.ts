import { z } from 'zod';

import {
  DtoValidationError,
  HttpError,
  readErrorMessage,
  readRequestId,
  readResponseBody,
} from './http.errors';
import { encryptRequestBody } from './http.request-e2ee';
import {
  applyAjaxRequestHeader,
  applyIdempotencyKey,
  currentAuthuser,
  isMutatingMethod,
  mergeHeaders,
  readCsrfToken,
  requestContextHeaders,
  resolveRequestUrl,
  sameOrigin,
  stripCrossOriginRequestContext,
} from './http.request-context';
import type { HttpClientOptions, HttpRequestOptions, RequestE2eeOptions } from './http.types';

export { DtoValidationError, HttpError } from './http.errors';
export { encryptRequestBody } from './http.request-e2ee';
export {
  applyAjaxRequestHeader,
  applyIdempotencyKey,
  configureHttpRequestContextHeaders,
  createIdempotencyKey,
  createRequestHeaders,
  resolveRequestUrl,
} from './http.request-context';
export type {
  HttpClientOptions,
  HttpRecovery,
  HttpRequestContextHeadersProvider,
  HttpRequestOptions,
  RequestE2eeOptions,
} from './http.types';

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
      console.error('DTO validation failed', parsed.error.issues);
      throw new DtoValidationError(parsed.error);
    }

    return parsed.data;
  }

  private async buildInit(url: string, options: HttpRequestOptions): Promise<RequestInit> {
    const isTrustedOrigin = sameOrigin(url, this.baseUrl);
    const headers = new Headers(this.headers);
    if (isTrustedOrigin) {
      mergeHeaders(headers, await requestContextHeaders());
    }
    mergeHeaders(headers, options.headers);

    const method = options.method ?? 'GET';
    const credentials = isTrustedOrigin ? (options.credentials ?? this.credentials) : 'omit';
    if (isTrustedOrigin) {
      const authuser = currentAuthuser();
      if (authuser && !headers.has('X-Auth-User')) {
        headers.set('X-Auth-User', authuser);
      }
      applyAjaxRequestHeader(headers, method);
      applyIdempotencyKey(headers, method, options.idempotencyKey ?? this.idempotencyKey);
      if (credentials !== 'omit' && isMutatingMethod(method) && !headers.has('X-CSRF-Token')) {
        const csrfToken = readCsrfToken(authuser);
        if (csrfToken) {
          headers.set('X-CSRF-Token', csrfToken);
        }
      }
    } else {
      stripCrossOriginRequestContext(headers);
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
      ...toRequestInit(options),
      credentials,
      headers,
      body,
    };
  }

  private resolveUrl(path: string): string {
    const url = resolveRequestUrl(path, this.baseUrl);
    if (sameOrigin(url.toString(), this.baseUrl)) {
      const authuser = currentAuthuser();
      if (authuser) {
        url.searchParams.set('authuser', authuser);
      }
    } else {
      url.searchParams.delete('authuser');
    }
    return url.toString();
  }
}

export function createHttpClient(options?: HttpClientOptions): HttpClient {
  return new HttpClient(options);
}

function toRequestInit(options: HttpRequestOptions): RequestInit {
  const init: RequestInit & Pick<HttpRequestOptions, 'idempotencyKey' | 'requestE2ee'> = {
    ...options,
    body: undefined,
  };
  Reflect.deleteProperty(init, 'idempotencyKey');
  Reflect.deleteProperty(init, 'requestE2ee');
  return init;
}
