import { z } from 'zod';

import type { HttpRecovery } from './http.types';

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

export async function readResponseBody(response: Response): Promise<unknown> {
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

export function readErrorMessage(response: Response, body: unknown): string {
  const envelope = ApiErrorEnvelopeSchema.safeParse(body);
  return envelope.success && envelope.data.error?.message
    ? envelope.data.error.message
    : `${response.status} ${response.statusText}`;
}

export function readRequestId(response: Response, body: unknown): string | undefined {
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
