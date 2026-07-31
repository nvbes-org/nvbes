import type { z } from 'zod';

export class AccountAuthenticationError extends Error {
  readonly code = 'missing_access_token';

  constructor() {
    super('An Account OAuth access token is required');
    this.name = 'AccountAuthenticationError';
  }
}

export class AccountHttpError extends Error {
  readonly status: number;
  readonly statusText: string;
  readonly body: unknown;
  readonly requestId: string | undefined;

  constructor(message: string, response: Response, body: unknown, requestId?: string) {
    super(message);
    this.name = 'AccountHttpError';
    this.status = response.status;
    this.statusText = response.statusText;
    this.body = body;
    this.requestId = requestId;
  }
}

export class AccountDtoValidationError extends Error {
  readonly cause: z.ZodError;

  constructor(cause: z.ZodError) {
    super('Account response DTO validation failed');
    this.name = 'AccountDtoValidationError';
    this.cause = cause;
  }
}
