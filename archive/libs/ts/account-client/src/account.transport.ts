import { z } from 'zod';
import {
  AccountAuthenticationError,
  AccountDtoValidationError,
  AccountHttpError,
} from './account.errors';

export type GetAccessToken = () => string | null | Promise<string | null>;

export type AccountTransportOptions = {
  baseUrl?: string;
  fetchImpl?: typeof fetch;
  getAccessToken: GetAccessToken;
};

export type AccountRequestOptions = {
  signal?: AbortSignal;
};

type JsonRequestOptions = AccountRequestOptions & {
  body?: unknown;
  method: 'DELETE' | 'GET' | 'PATCH' | 'POST' | 'PUT';
};

const ErrorEnvelopeSchema = z.object({
  error: z
    .object({
      message: z.string().optional(),
      request_id: z.string().optional(),
    })
    .optional(),
});

export class AccountTransport {
  private readonly baseUrl: string;
  private readonly fetchImpl: typeof fetch;
  private readonly getAccessToken: GetAccessToken;

  constructor(options: AccountTransportOptions) {
    this.baseUrl = options.baseUrl ?? globalThis.location?.origin ?? 'http://localhost';
    this.fetchImpl = options.fetchImpl ?? fetch.bind(globalThis);
    this.getAccessToken = options.getAccessToken;
  }

  async request<T>(path: string, schema: z.ZodType<T>, options: JsonRequestOptions): Promise<T> {
    const response = await this.fetchImpl(this.resolveUrl(path), {
      body: serializeBody(options.body),
      credentials: 'omit',
      headers: await this.headers(options.body !== undefined),
      method: options.method,
      signal: options.signal,
    });
    const body = await readResponseBody(response);
    assertResponseOk(response, body);

    const parsed = schema.safeParse(body);
    if (!parsed.success) {
      throw new AccountDtoValidationError(parsed.error);
    }
    return parsed.data;
  }

  async requestBlob(path: string, options: AccountRequestOptions = {}): Promise<Blob> {
    const response = await this.fetchImpl(this.resolveUrl(path), {
      credentials: 'omit',
      headers: await this.headers(false),
      method: 'GET',
      signal: options.signal,
    });
    if (!response.ok) {
      const body = await readResponseBody(response);
      assertResponseOk(response, body);
    }
    return response.blob();
  }

  private async headers(hasBody: boolean): Promise<Headers> {
    const token = (await this.getAccessToken())?.trim();
    if (!token) {
      throw new AccountAuthenticationError();
    }

    const headers = new Headers({
      Accept: 'application/json',
      Authorization: `Bearer ${token}`,
    });
    if (hasBody) {
      headers.set('Content-Type', 'application/json');
    }
    return headers;
  }

  private resolveUrl(path: string): string {
    const base = new URL(this.baseUrl);
    const normalizedBase = new URL(base);
    normalizedBase.pathname = `${base.pathname.replace(/\/+$/u, '')}/`;
    return new URL(path.replace(/^\/+/u, ''), normalizedBase).toString();
  }
}

function serializeBody(body: unknown): BodyInit | undefined {
  return body === undefined ? undefined : JSON.stringify(body);
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

function assertResponseOk(response: Response, body: unknown): asserts response is Response {
  if (response.ok) {
    return;
  }
  const envelope = ErrorEnvelopeSchema.safeParse(body);
  const message =
    envelope.success && envelope.data.error?.message
      ? envelope.data.error.message
      : `${response.status} ${response.statusText}`;
  const requestId =
    envelope.success && envelope.data.error?.request_id
      ? envelope.data.error.request_id
      : (response.headers.get('x-request-id') ?? undefined);
  throw new AccountHttpError(message, response, body, requestId);
}
