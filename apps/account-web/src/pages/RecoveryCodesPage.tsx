import { listMfaFactors } from '@nvbes/identity-sdk-web';
import { useLocation, useNavigate } from '@tanstack/react-router';
import { useEffect, useState } from 'react';
import { authuserSearch, readAuthuser } from '@/identity.authuser';
import { downloadRecoveryCodes, generateCodesWithPassword } from './RecoveryCodesPage.actions';
import { RecoveryCodesListStep } from './RecoveryCodesPage.list';
import { RecoveryCodesPasswordStep } from './RecoveryCodesPage.password';
import { RecoveryCodesStepUp } from './RecoveryCodesPage.stepup';

export default function RecoveryCodesPage() {
  const location = useLocation();
  const authuser = readAuthuser(location.searchStr);
  const navigate = useNavigate();
  const navigateBack = () =>
    void navigate({ to: '/account/mfa', search: authuserSearch(authuser) });

  const [step, setStep] = useState<'loading' | 'stepup' | 'password' | 'codes'>('loading');
  const [password, setPassword] = useState('');
  const [codes, setCodes] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    let cancelled = false;

    listMfaFactors('')
      .then((result) => {
        if (cancelled) {
          return;
        }

        const hasRecovery = result.factors.some(
          (factor) => factor.factor_type === 'recovery' || factor.factor_type === 'recovery_code',
        );
        setStep(hasRecovery ? 'stepup' : 'password');
      })
      .catch(() => {
        if (!cancelled) {
          setStep('stepup');
        }
      });

    return () => {
      cancelled = true;
    };
  }, []);

  const handleGenerate = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    setLoading(true);
    setError(null);
    try {
      const result = await generateCodesWithPassword(password);
      setCodes(result.codes ?? []);
      setStep('codes');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to generate recovery codes');
    } finally {
      setLoading(false);
    }
  };

  const download = () => {
    downloadRecoveryCodes(codes);
  };

  if (step === 'loading') {
    return null;
  }

  if (step === 'stepup') {
    return <RecoveryCodesStepUp onSuccess={() => setStep('password')} onCancel={navigateBack} />;
  }

  if (step === 'password') {
    return (
      <RecoveryCodesPasswordStep
        error={error}
        loading={loading}
        password={password}
        onCancel={navigateBack}
        onPasswordChange={setPassword}
        onSubmit={handleGenerate}
      />
    );
  }

  return <RecoveryCodesListStep codes={codes} onBack={navigateBack} onDownload={download} />;
}
