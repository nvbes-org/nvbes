import { z } from 'zod';
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
export declare class HttpError extends Error {
    readonly status: number;
    readonly statusText: string;
    readonly body: unknown;
    readonly recovery: HttpRecovery | undefined;
    readonly requestId: string | undefined;
    constructor(message: string, response: Response, body: unknown, requestId?: string);
}
export declare class DtoValidationError extends Error {
    readonly cause: z.ZodError;
    constructor(cause: z.ZodError);
}
export declare class HttpClient {
    private readonly baseUrl;
    private readonly credentials?;
    private readonly headers?;
    private readonly fetchImpl;
    private readonly idempotencyKey?;
    private readonly requestE2ee?;
    constructor(options?: HttpClientOptions);
    get<T>(path: string, schema: z.ZodType<T>, options?: HttpRequestOptions): Promise<T>;
    post<T>(path: string, schema: z.ZodType<T>, body?: unknown, options?: HttpRequestOptions): Promise<T>;
    delete<T>(path: string, schema: z.ZodType<T>, options?: HttpRequestOptions): Promise<T>;
    request<T>(path: string, schema: z.ZodType<T>, options?: HttpRequestOptions): Promise<T>;
    private buildInit;
    private resolveUrl;
}
export declare function resolveRequestUrl(path: string, baseUrl: string): URL;
export declare function encryptRequestBody(input: {
    body: string;
    keyId: string;
    method: string;
    secret: string;
    url: string;
}): Promise<{
    body: ArrayBuffer;
    headers: Headers;
}>;
export declare function createHttpClient(options?: HttpClientOptions): HttpClient;
export declare function createRequestHeaders(method: string, headers?: HeadersInit, idempotencyKey?: string | false): Headers;
export declare function applyAjaxRequestHeader(headers: Headers, method: string): void;
export declare function applyIdempotencyKey(headers: Headers, method: string, idempotencyKey?: string | false): void;
export declare function createIdempotencyKey(): string;
