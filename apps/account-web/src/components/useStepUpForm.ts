import { completeWebAuthnStepUp, listMfaFactors, stepUp } from '@nvbes/identity-sdk-web';
import type { MfaFactorView } from '@nvbes/identity-sdk-core/src/types';
import { useCallback, useEffect, useState, type SubmitEvent } from 'react';
import { z } from 'zod';

import { identityHttpClient } from '../identity.http';

const PreferencesSchema = z.object({
  theme: z.string(),
  language: z.string(),
  skip_password: z.boolean().default(false),
});

export type StepUpMethod = 'password' | 'webauthn' | 'totp' | 'recovery';
export type WebAuthnStatus = 'idle' | 'prompting' | 'error' | 'success';

export function useStepUpForm({
  open = true,
  onSuccess,
}: {
  open?: boolean;
  onSuccess: () => void;
}) {
  const [method, setMethodState] = useState<StepUpMethod>('password');
  const [factors, setFactors] = useState<MfaFactorView[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [password, setPassword] = useState('');
  const [totpCode, setTotpCode] = useState('');
  const [recoveryCode, setRecoveryCode] = useState('');
  const [webauthnStatus, setWebauthnStatus] = useState<WebAuthnStatus>('idle');

  const hasTotp = factors.some((factor) => factor.factor_type === 'totp');
  const hasWebAuthn = factors.some((factor) => factor.factor_type === 'webauthn');
  const hasRecovery = factors.some((factor) => factor.factor_type === 'recovery');

  const setMethod = useCallback((newMethod: StepUpMethod) => {
    setMethodState(newMethod);
    setError(null);
    if (newMethod === 'webauthn') {
      setWebauthnStatus('idle');
    }
  }, []);

  useEffect(() => {
    if (!open) {
      setWebauthnStatus('idle');
      setError(null);
      return;
    }

    let active = true;

    async function load() {
      try {
        const [factorsRes, prefsRes] = await Promise.all([
          listMfaFactors(''),
          identityHttpClient.request('/auth/me/preferences', PreferencesSchema),
        ]);

        if (!active) {
          return;
        }

        setFactors(factorsRes.factors);
        const canSkip =
          (prefsRes.skip_password ?? false) &&
          factorsRes.factors.some((factor) => factor.factor_type === 'webauthn');
        if (canSkip) {
          setMethod('webauthn');
        }
      } catch (err) {
        console.error('Failed to load step-up pre-requisites:', err);
      }
    }

    void load();

    return () => {
      active = false;
    };
  }, [open, setMethod]);

  const handleWebAuthnClick = useCallback(async () => {
    setWebauthnStatus('prompting');
    setLoading(true);
    setError(null);
    try {
      await completeWebAuthnStepUp('');
      setWebauthnStatus('success');
      onSuccess();
    } catch (err) {
      setWebauthnStatus('error');
      let errMsg = 'Authentification WebAuthn échouée';
      if (err instanceof Error) {
        const lowerMsg = err.message.toLowerCase();
        if (
          err.name === 'NotAllowedError' ||
          lowerMsg.includes('cancel') ||
          lowerMsg.includes('annul')
        ) {
          errMsg = "L'authentification a été annulée.";
        } else if (
          err.name === 'TimeoutError' ||
          lowerMsg.includes('timeout') ||
          lowerMsg.includes('expir')
        ) {
          errMsg = "Le délai d'attente pour la clé de sécurité a expiré.";
        } else {
          errMsg = err.message;
        }
      }
      setError(errMsg);
    } finally {
      setLoading(false);
    }
  }, [onSuccess]);

  useEffect(() => {
    if (open && method === 'webauthn' && webauthnStatus === 'idle' && !loading) {
      void handleWebAuthnClick();
    }
  }, [open, method, webauthnStatus, loading, handleWebAuthnClick]);

  const submitStepUp = async () => {
    if (method === 'password') {
      await stepUp('', { password });
      return;
    }
    if (method === 'totp') {
      await stepUp('', { totpCode });
      return;
    }
    if (method === 'recovery') {
      await stepUp('', { recoveryCode });
      return;
    }
    await handleWebAuthnClick();
  };

  const handleSubmit = async (event: SubmitEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (method === 'webauthn') {
      await handleWebAuthnClick();
      return;
    }
    setLoading(true);
    setError(null);
    try {
      await submitStepUp();
      onSuccess();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Vérification échouée');
    } finally {
      setLoading(false);
    }
  };

  return {
    error,
    handleSubmit,
    handleWebAuthnClick,
    hasRecovery,
    hasTotp,
    hasWebAuthn,
    loading,
    method,
    password,
    recoveryCode,
    setMethod,
    setPassword,
    setRecoveryCode,
    setTotpCode,
    totpCode,
    webauthnStatus,
  };
}
