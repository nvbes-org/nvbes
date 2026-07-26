import { describe, expect, it } from 'vite-plus/test';

import { submitPasswordStep } from '../src/pages/useLoginPage.steps.password';

describe('submitPasswordStep', () => {
  it('redirects an unverified account to email verification when email MFA cannot start', async () => {
    let verificationEmail: string | null = null;
    let displayedError: string | null = null;

    await submitPasswordStep({
      loginStateToken: 'login-state',
      password: 'Sup3rS3cret!',
      email: 'pending@example.test',
      submitPassword: async () => {
        throw {
          status: 403,
          body: {
            error: {
              code: 'email_mfa_requires_verified_email',
              message: 'Email MFA requires a verified primary email.',
            },
          },
        };
      },
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
      navigateToForgotPassword: () => undefined,
      finishLogin: async () => undefined,
    });

    expect(verificationEmail).toBe('pending@example.test');
    expect(displayedError).toBeNull();
  });
});
