import { HttpError } from '@nvbes/http-client';

export function isStepUpRequiredError(error: unknown): boolean {
  if (!error) return false;

  if (error instanceof HttpError) {
    const code = readErrorCodeFromPayload(error.body);
    if (code === 'step_up_required') return true;
  }

  if (typeof error === 'object' && error !== null && 'body' in error) {
    const code = readErrorCodeFromPayload((error as { body?: unknown }).body);
    if (code === 'step_up_required') return true;
  }

  if (error instanceof Error) {
    const msg = error.message;
    if (
      msg.includes('step_up_required') ||
      msg.includes('Please verify again before continuing.') ||
      msg.includes('Confirmez votre identité pour continuer.')
    ) {
      return true;
    }
  }

  return false;
}

function readErrorCodeFromPayload(body: unknown): string | null {
  if (
    typeof body === 'object' &&
    body !== null &&
    'error' in body &&
    typeof (body as { error?: unknown }).error === 'object' &&
    (body as { error?: unknown }).error !== null
  ) {
    return (body as { error: { code?: string } }).error.code ?? null;
  }
  return null;
}
