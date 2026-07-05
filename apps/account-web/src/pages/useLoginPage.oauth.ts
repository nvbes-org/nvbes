import { clientErrorMessage } from '@nvbes/web-runtime';

import {
  authorizeIdentitySession,
  clearPendingOAuthAuthorizeRequest,
  type OAuthAuthorizeRequest,
} from '../identity.oauth';
import { isConsentRequiredError, isInvalidSignatureError } from './LoginPage.errors';

type FinishLoginOptions = {
  email: string;
  password: string;
  sessionToken: string | null;
  oauthRequest: OAuthAuthorizeRequest | null;
  storePasswordCredential: (email: string, password: string, name: string) => Promise<unknown>;
  navigateToAccount: () => void;
  onConsentRequired: (sessionToken: string) => void;
  onInvalidSession: () => void;
};

export async function finishLoginSession({
  email,
  password,
  sessionToken,
  oauthRequest,
  storePasswordCredential,
  navigateToAccount,
  onConsentRequired,
  onInvalidSession,
}: FinishLoginOptions) {
  void storePasswordCredential(email, password, email).catch(() => {});

  if (!oauthRequest) {
    navigateToAccount();
    return;
  }

  if (!sessionToken) {
    throw new Error('Login did not return a session token.');
  }

  try {
    await authorizeIdentitySession(sessionToken, oauthRequest);
    clearPendingOAuthAuthorizeRequest();
  } catch (err) {
    if (isInvalidSignatureError(err)) {
      clearPendingOAuthAuthorizeRequest();
      onInvalidSession();
      return;
    }

    if (!isConsentRequiredError(err)) {
      throw err;
    }
    onConsentRequired(sessionToken);
  }
}

type ApproveConsentOptions = {
  oauthRequest: OAuthAuthorizeRequest | null;
  sessionToken: string | null;
  setError: (value: string | null) => void;
  setCheckingAuth: (value: boolean) => void;
  setSessionToken: (value: string | null) => void;
  setStep: (value: 'identifier') => void;
};

export async function approveConsent({
  oauthRequest,
  sessionToken,
  setError,
  setCheckingAuth,
  setSessionToken,
  setStep,
}: ApproveConsentOptions) {
  if (!oauthRequest) {
    return;
  }

  setError(null);
  setCheckingAuth(true);
  try {
    await authorizeIdentitySession(sessionToken, {
      ...oauthRequest,
      consentAction: 'approve',
    });
    clearPendingOAuthAuthorizeRequest();
  } catch (err) {
    if (isInvalidSignatureError(err)) {
      setError(null);
      setSessionToken(null);
      setStep('identifier');
      setCheckingAuth(false);
      return;
    }

    setError(clientErrorMessage(err, 'Failed to grant consent'));
    setCheckingAuth(false);
  }
}
