import { HttpError } from '@nvbes/http-client';

export type SessionStaleReason = 'reauthenticate';

export interface SessionStaleResult {
  stale: boolean;
  reason: SessionStaleReason | null;
  status: number | null;
}

export function detectSessionStale(error: unknown): SessionStaleResult {
  const status = readErrorStatus(error);
  if (error instanceof HttpError && error.recovery === 'reauthenticate') {
    return { reason: 'reauthenticate', stale: true, status };
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
