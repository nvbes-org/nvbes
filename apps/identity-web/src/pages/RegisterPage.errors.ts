function apiErrorCode(error: unknown): string | null {
  if (typeof error !== 'object' || error === null || !('body' in error)) {
    return null;
  }

  const body = error.body;
  if (typeof body !== 'object' || body === null || !('error' in body)) {
    return null;
  }

  const envelopeError = body.error;
  if (typeof envelopeError !== 'object' || envelopeError === null || !('code' in envelopeError)) {
    return null;
  }

  return typeof envelopeError.code === 'string' ? envelopeError.code : null;
}

export function isEmailAlreadyExistsError(error: unknown): boolean {
  return apiErrorCode(error) === 'email_already_exists';
}

export function isUsernameTakenError(error: unknown): boolean {
  return apiErrorCode(error) === 'username_taken';
}
