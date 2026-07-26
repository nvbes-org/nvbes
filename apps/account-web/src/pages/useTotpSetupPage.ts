import { confirmTotp, setupTotp } from '@nvbes/identity-sdk-web';
import { useLocation, useNavigate } from '@tanstack/react-router';
import { useState } from 'react';
import { readAuthuser } from '@/identity.authuser';

import { isStepUpRequiredError } from '@/identity.step-up';

export function useTotpSetupPage() {
  const location = useLocation();
  const authuser = readAuthuser(location.searchStr, location.pathname);
  const navigate = useNavigate();
  const [step, setStep] = useState<'stepup' | 'setup' | 'confirm'>('stepup');
  const [label, setLabel] = useState('');
  const [factorId, setFactorId] = useState('');
  const [secretBase32, setSecretBase32] = useState('');
  const [provisioningUri, setProvisioningUri] = useState('');
  const [totpCode, setTotpCode] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const handleSetup = async () => {
    setLoading(true);
    setError(null);

    try {
      const result = await setupTotp('', label || undefined);
      setFactorId(result.factor.id);
      setSecretBase32(result.secret_base32);
      setProvisioningUri(result.provisioning_uri);
      setStep('confirm');
    } catch (err) {
      if (isStepUpRequiredError(err)) {
        setStep('stepup');
      } else {
        const message = err instanceof Error ? err.message : 'TOTP setup failed';
        setError(message);
      }
    } finally {
      setLoading(false);
    }
  };

  const handleConfirm = async (event: React.SubmitEvent<HTMLFormElement>) => {
    event.preventDefault();
    setLoading(true);
    setError(null);

    try {
      await confirmTotp('', factorId, totpCode);
      void navigate({
        to: '/account/$accountIndex/mfa/recovery-codes',
        params: { accountIndex: authuser },
      });
    } catch (err) {
      setError(err instanceof Error ? err.message : 'TOTP confirmation failed');
    } finally {
      setLoading(false);
    }
  };

  return {
    step,
    label,
    secretBase32,
    totpCode,
    error,
    loading,
    qrData: provisioningUri || null,
    navigateBack: () =>
      void navigate({
        to: '/account/$accountIndex/security',
        params: { accountIndex: authuser },
      }),
    onStepUpSuccess: () => setStep('setup'),
    onLabelChange: setLabel,
    onTotpCodeChange: setTotpCode,
    onSetup: handleSetup,
    onConfirm: handleConfirm,
  };
}
