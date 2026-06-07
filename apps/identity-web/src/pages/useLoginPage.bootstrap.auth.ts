import { identityClient } from '@nvbes/identity-client';
import { clientErrorMessage } from '@nvbes/web-runtime';
import {
  isConsentRequiredError,
  isInvalidSignatureError,
  isMissingLoginSessionError,
} from './LoginPage.errors';
import { syncTrackingConsent } from '../tracking-consent';
import type { UseLoginPageBootstrapOptions } from './useLoginPage.bootstrap.types';

export async function bootstrapLoginAuth({
  authorizeCurrentOAuth,
  hasOAuthRequest,
  locationSearchStr,
  navigateToAccount,
  setCheckingAuth,
  setConnectedAccounts,
  setError,
  setStep,
}: Pick<
  UseLoginPageBootstrapOptions,
  | 'authorizeCurrentOAuth'
  | 'hasOAuthRequest'
  | 'locationSearchStr'
  | 'navigateToAccount'
  | 'setCheckingAuth'
  | 'setConnectedAccounts'
  | 'setError'
  | 'setStep'
>) {
  try {
    const accountsList = await identityClient.listAccounts().catch(() => []);
    setConnectedAccounts(accountsList);

    const searchParams = new URLSearchParams(locationSearchStr);
    const hasAuthUserQuery = searchParams.has('authuser');

    if (hasAuthUserQuery) {
      await identityClient.getMe();
      await syncTrackingConsent();
      if (hasOAuthRequest) {
        await authorizeCurrentOAuth();
        return;
      }
      navigateToAccount();
      return;
    }

    if (accountsList.length > 0) {
      setStep('chooser');
      setCheckingAuth(false);
      return;
    }

    await identityClient.getMe();
    await syncTrackingConsent();
    if (hasOAuthRequest) {
      await authorizeCurrentOAuth();
      return;
    }
    navigateToAccount();
  } catch (err) {
    if (isConsentRequiredError(err)) {
      setStep('consent');
    } else if (isInvalidSignatureError(err)) {
      setError(null);
      setStep('identifier');
    } else if (isMissingLoginSessionError(err)) {
      setError(null);
    } else {
      setError(clientErrorMessage(err, "Impossible de finaliser l'autorisation OAuth"));
    }
    setCheckingAuth(false);
  }
}
