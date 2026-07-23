export function isPasswordExpiredError(err: unknown): boolean {
  return apiErrorCode(err) === 'password_expired';
}

export function isPrimaryEmailVerificationRequiredError(err: unknown): boolean {
  return apiErrorCode(err) === 'email_mfa_requires_verified_email';
}

export function isConsentRequiredError(err: unknown): boolean {
  return apiErrorCode(err) === 'consent_required';
}

export function isMissingLoginSessionError(err: unknown): boolean {
  if (httpStatus(err) !== 401) return false;
  return ['invalid_token', 'missing_token'].includes(apiErrorCode(err) ?? '');
}

export function isInvalidSignatureError(err: unknown): boolean {
  if (httpStatus(err) !== 401) return false;
  return (
    apiErrorCode(err) === 'token_validation_failed' || apiErrorMessage(err) === 'InvalidSignature'
  );
}

function apiErrorCode(err: unknown): string | undefined {
  const body = errorBody(err);
  if (typeof body !== 'object' || body === null) return undefined;
  const error = (body as Record<string, unknown>).error;
  if (typeof error !== 'object' || error === null) return undefined;
  const code = (error as Record<string, unknown>).code;
  return typeof code === 'string' ? code : undefined;
}

function apiErrorMessage(err: unknown): string | undefined {
  const body = errorBody(err);
  if (typeof body !== 'object' || body === null) return undefined;
  const error = (body as Record<string, unknown>).error;
  if (typeof error !== 'object' || error === null) return undefined;
  const message = (error as Record<string, unknown>).message;
  return typeof message === 'string' ? message : undefined;
}

function errorBody(err: unknown): unknown {
  if (typeof err !== 'object' || err === null) return undefined;
  return (err as { body?: unknown }).body;
}

function httpStatus(err: unknown): number | undefined {
  if (typeof err !== 'object' || err === null) return undefined;
  const status = (err as { status?: unknown }).status;
  return typeof status === 'number' ? status : undefined;
}
