import { clientErrorMessage } from '@nvbes/web-runtime';

import type { MfaMethod } from './LoginPage.mfa';
import {
  isInvalidSignatureError,
  isPasswordExpiredError,
  isPrimaryEmailVerificationRequiredError,
} from './LoginPage.errors';
import { normalizeMfaMethods } from './LoginPage.mfa';
import { preferredMfaMethod, requestsMfa } from './useLoginPage.shared';
import type { PasswordMutateAsync } from './useLoginPage.steps.shared';

type SubmitPasswordStepOptions = {
  loginStateToken: string | null;
  password: string;
  email: string;
  submitPassword: PasswordMutateAsync;
  setError: (value: string | null) => void;
  setStep: (value: 'identifier' | 'mfa') => void;
  setLoginStateToken: (value: string | null) => void;
  setAvailableMethods: (value: MfaMethod[]) => void;
  setMfaMethod: (value: MfaMethod | null) => void;
  setSessionToken: (value: string | null) => void;
  navigateToVerifyEmail: (
    email: string,
    resendAvailableAt: string | null,
    accountName: string | null,
  ) => void;
  navigateToForgotPassword: (email: string) => void;
  finishLogin: (sessionToken: string | null) => Promise<void>;
};

export async function submitPasswordStep({
  loginStateToken,
  password,
  email,
  submitPassword,
  setError,
  setStep,
  setLoginStateToken,
  setAvailableMethods,
  setMfaMethod,
  setSessionToken,
  navigateToVerifyEmail,
  navigateToForgotPassword,
  finishLogin,
}: SubmitPasswordStepOptions) {
  if (!loginStateToken) {
    setError('Session de connexion expirée. Recommencez.');
    setStep('identifier');
    return;
  }

  setError(null);
  try {
    const result = await submitPassword({
      stateToken: loginStateToken,
      password,
    });

    if (requestsMfa(result.next_step, result.available_methods)) {
      if (result.state_token) {
        setLoginStateToken(result.state_token);
      }
      setAvailableMethods(normalizeMfaMethods(result.available_methods));
      setMfaMethod(preferredMfaMethod(result.available_methods));
      setStep('mfa');
      return;
    }

    if (result.user?.email_verified === false) {
      navigateToVerifyEmail(
        result.user?.email ?? email,
        result.verification_resend_available_at ?? null,
        result.user?.username?.trim() || null,
      );
      return;
    }

    if (result.session_token) {
      setSessionToken(result.session_token);
    }
    await finishLogin(result.session_token ?? null);
  } catch (err) {
    if (isPrimaryEmailVerificationRequiredError(err)) {
      navigateToVerifyEmail(email, null, null);
      return;
    }

    if (isInvalidSignatureError(err)) {
      setLoginStateToken(null);
      setSessionToken(null);
      setAvailableMethods([]);
      setMfaMethod(null);
      setStep('identifier');
      return;
    }

    if (isPasswordExpiredError(err)) {
      navigateToForgotPassword(email);
      return;
    }
    setError(clientErrorMessage(err, 'Login failed'));
  }
}
