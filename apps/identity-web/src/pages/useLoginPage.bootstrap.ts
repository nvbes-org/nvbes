import { useEffect, useRef } from 'react';
import { bootstrapLoginAuth } from './useLoginPage.bootstrap.auth';
import { autoFillStoredPassword } from './useLoginPage.bootstrap.autofill';
import { mountLoginDecoyLinks } from './useLoginPage.bootstrap.decoy';
import type {
  LoginPageDecoyRef,
  UseLoginPageBootstrapOptions,
} from './useLoginPage.bootstrap.types';

export function useLoginPageBootstrap({
  checkingAuth,
  locationSearchStr,
  hasOAuthRequest,
  setCheckingAuth,
  setConnectedAccounts,
  setStep,
  setEmail,
  setPassword,
  setError,
  navigateToAccount,
  authorizeCurrentOAuth,
}: UseLoginPageBootstrapOptions) {
  const decoyRef = useRef<LoginPageDecoyRef['current']>(null);

  useEffect(() => {
    void bootstrapLoginAuth({
      authorizeCurrentOAuth,
      hasOAuthRequest,
      locationSearchStr,
      navigateToAccount,
      setCheckingAuth,
      setConnectedAccounts,
      setError,
      setStep,
    });
  }, [
    authorizeCurrentOAuth,
    hasOAuthRequest,
    locationSearchStr,
    navigateToAccount,
    setCheckingAuth,
    setConnectedAccounts,
    setError,
    setStep,
  ]);

  useEffect(() => {
    if (checkingAuth) {
      return;
    }

    void autoFillStoredPassword({ setEmail, setPassword });
  }, [checkingAuth, setEmail, setPassword]);

  useEffect(() => {
    return mountLoginDecoyLinks(decoyRef, 'decoy-links-container');
  }, []);

  return decoyRef;
}
