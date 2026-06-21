import { HttpError } from '@nvbes/http-client';

export type SessionStaleReason = 'unauthorized' | 'csrf' | 'login-timeout';

export interface SessionStaleResult {
  stale: boolean;
  reason: SessionStaleReason | null;
  status: number | null;
}

export function detectSessionStale(error: unknown): SessionStaleResult {
  const status = readErrorStatus(error);
  if (status === 401) {
    return { reason: 'unauthorized', stale: true, status };
  }
  if (status === 419) {
    return { reason: 'csrf', stale: true, status };
  }
  if (status === 440) {
    return { reason: 'login-timeout', stale: true, status };
  }
  return { reason: null, stale: false, status };
}

export function isSessionStaleError(error: unknown): boolean {
  return detectSessionStale(error).stale;
}

function readErrorStatus(error: unknown): number | null {
  if (error instanceof HttpError) {
    return error.status;
  }

  if (typeof error !== 'object' || error === null || !('status' in error)) {
    return null;
  }

  const status = error.status;
  return typeof status === 'number' ? status : null;
}
