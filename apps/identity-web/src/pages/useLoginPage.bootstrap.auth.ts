import { identityClient } from '@nvbes/identity-client';
import { clientErrorMessage } from '@nvbes/web-runtime';
import {
  isConsentRequiredError,
  isInvalidSignatureError,
  isUnauthorizedError,
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
  setEmail,
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
  | 'setEmail'
>) {
  try {
    const accountsList = await identityClient.listAccounts().catch(() => []);
    setConnectedAccounts(accountsList);

    const searchParams = new URLSearchParams(locationSearchStr);
    const authuser = searchParams.get('authuser');
    const hasAuthUserQuery = authuser !== null;

    if (hasAuthUserQuery) {
      const selectedAccount = accountsList.find((account) => account.authuser === authuser);
      if (selectedAccount?.status === 'expired') {
        setEmail(selectedAccount.user.email);
        setError(selectedAccount.message ?? 'Session expirée, veuillez vous reconnecter.');
        setStep('identifier');
        setCheckingAuth(false);
        return;
      }

      await identityClient.getMe();
      await syncTrackingConsent({ sourceOfTruth: 'backend' });
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
    } else if (isUnauthorizedError(err)) {
      setError(null);
    } else {
      setError(clientErrorMessage(err, "Impossible de finaliser l'autorisation OAuth"));
    }
    setCheckingAuth(false);
  }
}
