import { describe, expect, it } from 'vite-plus/test';

import { submitPasswordStep } from '../src/pages/useLoginPage.steps.password';

describe('submitPasswordStep', () => {
  it('redirects an unverified account to email verification', async () => {
    let verificationEmail: string | null = null;
    let displayedError: string | null = null;

    await submitPasswordStep({
      loginStateToken: 'login-state',
      password: 'Sup3rS3cret!',
      email: 'pending@example.test',
      submitPassword: async () => ({
        user: {
          email: 'pending@example.test',
          email_verified: false,
          username: null,
        },
        verification_resend_available_at: null,
      }),
      setError: (value) => {
        displayedError = value;
      },
      setStep: () => undefined,
      setLoginStateToken: () => undefined,
      setAvailableMethods: () => undefined,
      setMfaMethod: () => undefined,
      setSessionToken: () => undefined,
      navigateToVerifyEmail: (email) => {
        verificationEmail = email;
      },
      finishLogin: async () => undefined,
    });

    expect(verificationEmail).toBe('pending@example.test');
    expect(displayedError).toBeNull();
  });
});
