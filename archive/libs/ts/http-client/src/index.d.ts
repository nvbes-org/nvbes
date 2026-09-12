import { z } from 'zod';
import type { HttpClientOptions, HttpRequestOptions } from './http.types';
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
export declare class HttpClient {
  private readonly baseUrl;
  private readonly credentials?;
  private readonly headers?;
  private readonly fetchImpl;
  private readonly idempotencyKey?;
  private readonly requestE2ee?;
  constructor(options?: HttpClientOptions);
  get<T>(path: string, schema: z.ZodType<T>, options?: HttpRequestOptions): Promise<T>;
  post<T>(
    path: string,
    schema: z.ZodType<T>,
    body?: unknown,
    options?: HttpRequestOptions,
  ): Promise<T>;
  delete<T>(path: string, schema: z.ZodType<T>, options?: HttpRequestOptions): Promise<T>;
  request<T>(path: string, schema: z.ZodType<T>, options?: HttpRequestOptions): Promise<T>;
  private buildInit;
  private resolveUrl;
}
export declare function createHttpClient(options?: HttpClientOptions): HttpClient;
