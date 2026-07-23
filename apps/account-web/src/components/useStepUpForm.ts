import { completeWebAuthnStepUp, listMfaFactors, stepUp } from '@nvbes/identity-sdk-web';
import type { MfaFactorView } from '@nvbes/identity-sdk-core/src/types';
import { useEffect, useState, type SubmitEvent } from 'react';
import { z } from 'zod';

import { identityHttpClient } from '../identity.http';

const PreferencesSchema = z.object({
  theme: z.string(),
  language: z.string(),
  skip_password: z.boolean().default(false),
});

export type StepUpMethod = 'password' | 'webauthn' | 'totp' | 'recovery';

export function useStepUpForm({ onSuccess }: { onSuccess: () => void }) {
  const [method, setMethod] = useState<StepUpMethod>('password');
  const [factors, setFactors] = useState<MfaFactorView[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [password, setPassword] = useState('');
  const [totpCode, setTotpCode] = useState('');
  const [recoveryCode, setRecoveryCode] = useState('');

  const hasTotp = factors.some((factor) => factor.factor_type === 'totp');
  const hasWebAuthn = factors.some((factor) => factor.factor_type === 'webauthn');
  const hasRecovery = factors.some((factor) => factor.factor_type === 'recovery');

  useEffect(() => {
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
  }, []);

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
    await completeWebAuthnStepUp('');
  };

  const handleSubmit = async (event: SubmitEvent<HTMLFormElement>) => {
    event.preventDefault();
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

  const handleWebAuthnClick = async () => {
    setLoading(true);
    setError(null);
    try {
      await completeWebAuthnStepUp('');
      onSuccess();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Authentification WebAuthn échouée');
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
  };
}
