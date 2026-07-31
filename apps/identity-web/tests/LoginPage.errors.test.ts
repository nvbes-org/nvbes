import { describe, expect, it } from 'vite-plus/test';

import { isUnauthorizedError } from '../src/pages/LoginPage.errors';

describe('isUnauthorizedError', () => {
  it('recognizes every unauthorized response, including a missing session', () => {
    expect(
      isUnauthorizedError({
        status: 401,
        body: { error: { code: 'session_not_found', message: 'Session not found.' } },
      }),
    ).toBe(true);
  });

  it('does not suppress errors with another HTTP status', () => {
    expect(isUnauthorizedError({ status: 403 })).toBe(false);
  });
});
