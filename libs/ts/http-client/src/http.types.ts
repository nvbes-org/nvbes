export type HttpRecovery = 'reauthenticate';

export type RequestE2eeOptions = {
  keyId: string;
  secret: string;
};

export type HttpClientOptions = {
  baseUrl?: string;
  credentials?: RequestCredentials;
  headers?: HeadersInit;
  fetchImpl?: typeof fetch;
  idempotencyKey?: string | false;
  requestE2ee?: RequestE2eeOptions;
};

export type HttpRequestContextHeadersProvider = () =>
  | HeadersInit
  | Promise<HeadersInit | undefined>
  | undefined;

export type HttpRequestOptions = Omit<RequestInit, 'body' | 'headers'> & {
  body?: unknown;
  headers?: HeadersInit;
  idempotencyKey?: string | false;
  requestE2ee?: RequestE2eeOptions | false;
};
