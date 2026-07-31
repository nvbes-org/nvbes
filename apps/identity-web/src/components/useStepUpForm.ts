import {
  completeWebAuthnStepUp,
  listMfaFactors,
  requestEmailStepUpCode,
  stepUp,
  type StepUpPurpose,
} from '@nvbes/identity-sdk-web';
import type { MfaFactorView } from '@nvbes/identity-sdk-core/src/types';
import { useCallback, useEffect, useState, type SubmitEvent } from 'react';
import { z } from 'zod';

import { identityHttpClient } from '../identity.http';

const PreferencesSchema = z.object({
  theme: z.string(),
  language: z.string(),
  skip_password: z.boolean().default(false),
});

export type StepUpMethod = 'password' | 'webauthn' | 'totp' | 'recovery' | 'email';
export type WebAuthnStatus = 'idle' | 'prompting' | 'error' | 'success';

export function useStepUpForm({
  open = true,
  onSuccess,
  purpose,
}: {
  open?: boolean;
  onSuccess: () => void;
  purpose?: StepUpPurpose;
}) {
  const [method, setMethodState] = useState<StepUpMethod>('password');
  const [factors, setFactors] = useState<MfaFactorView[]>([]);
  const [mfaEnabled, setMfaEnabled] = useState(false);
  const [prerequisitesLoaded, setPrerequisitesLoaded] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [password, setPassword] = useState('');
  const [totpCode, setTotpCode] = useState('');
  const [recoveryCode, setRecoveryCode] = useState('');
  const [emailCode, setEmailCode] = useState('');
  const [emailChallengeId, setEmailChallengeId] = useState<string | null>(null);
  const [emailCodeSent, setEmailCodeSent] = useState(false);
  const [webauthnStatus, setWebauthnStatus] = useState<WebAuthnStatus>('idle');

  const hasTotp = factors.some((factor) => factor.factor_type === 'totp');
  const hasWebAuthn = factors.some((factor) => factor.factor_type === 'webauthn');
  const hasRecovery = factors.some((factor) => factor.factor_type === 'recovery');
  const allowEmail = purpose === 'password_change';
  const canUsePassword = !mfaEnabled;

  const setMethod = useCallback((newMethod: StepUpMethod) => {
    setMethodState(newMethod);
    setError(null);
    if (newMethod === 'webauthn') {
      setWebauthnStatus('idle');
    }
  }, []);

  useEffect(() => {
    if (!open) {
      setMethodState('password');
      setPrerequisitesLoaded(false);
      setWebauthnStatus('idle');
      setError(null);
      setEmailCode('');
      setEmailChallengeId(null);
      setEmailCodeSent(false);
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
        setMfaEnabled(factorsRes.mfa_enabled);
        setPrerequisitesLoaded(true);
        if (factorsRes.mfa_enabled) {
          if (factorsRes.factors.some((factor) => factor.factor_type === 'webauthn')) {
            setMethod('webauthn');
          } else if (factorsRes.factors.some((factor) => factor.factor_type === 'totp')) {
            setMethod('totp');
          } else if (factorsRes.factors.some((factor) => factor.factor_type === 'recovery')) {
            setMethod('recovery');
          } else if (purpose === 'password_change') {
            setMethod('email');
          }
          return;
        }
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
  }, [open, purpose, setMethod]);

  const sendEmailCode = useCallback(async () => {
    if (purpose !== 'password_change') {
      return;
    }
    setLoading(true);
    setError(null);
    try {
      const result = await requestEmailStepUpCode('', purpose);
      setEmailChallengeId(result.challenge_id);
      setEmailCodeSent(true);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Impossible d'envoyer le code");
    } finally {
      setLoading(false);
    }
  }, [purpose]);

  const handleWebAuthnClick = useCallback(async () => {
    setWebauthnStatus('prompting');
    setLoading(true);
    setError(null);
    try {
      await completeWebAuthnStepUp('', undefined, undefined, purpose);
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
  }, [onSuccess, purpose]);

  useEffect(() => {
    if (open && method === 'webauthn' && webauthnStatus === 'idle' && !loading) {
      void handleWebAuthnClick();
    }
  }, [open, method, webauthnStatus, loading, handleWebAuthnClick]);

  const submitStepUp = async () => {
    if (method === 'password') {
      await stepUp('', { password, purpose });
      return;
    }
    if (method === 'totp') {
      await stepUp('', { totpCode, purpose });
      return;
    }
    if (method === 'recovery') {
      await stepUp('', { recoveryCode, purpose });
      return;
    }
    if (method === 'email') {
      if (!emailChallengeId) {
        throw new Error("Envoyez d'abord un code de vérification.");
      }
      await stepUp('', { purpose, emailCode, emailChallengeId });
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
    allowEmail,
    canUsePassword,
    emailCode,
    emailCodeSent,
    handleSubmit,
    handleWebAuthnClick,
    hasRecovery,
    hasTotp,
    hasWebAuthn,
    loading,
    method,
    password,
    prerequisitesLoaded,
    recoveryCode,
    sendEmailCode,
    setEmailCode,
    setMethod,
    setPassword,
    setRecoveryCode,
    setTotpCode,
    totpCode,
    webauthnStatus,
  };
}
