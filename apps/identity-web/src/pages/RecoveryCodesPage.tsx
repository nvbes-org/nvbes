import { useNavigate } from '@tanstack/react-router';
import { useState } from 'react';
import { downloadRecoveryCodes, generateCodesWithPassword } from './RecoveryCodesPage.actions';
import { RecoveryCodesListStep } from './RecoveryCodesPage.list';
import { RecoveryCodesPasswordStep } from './RecoveryCodesPage.password';
import { RecoveryCodesStepUp } from './RecoveryCodesPage.stepup';

export default function RecoveryCodesPage() {
  const navigate = useNavigate();

  const [step, setStep] = useState<'stepup' | 'password' | 'codes'>('stepup');
  const [password, setPassword] = useState('');
  const [codes, setCodes] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

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

  if (step === 'stepup') {
    return (
      <RecoveryCodesStepUp
        onSuccess={() => setStep('password')}
        onCancel={() => void navigate({ to: '/account/security' })}
      />
    );
  }

  if (step === 'password') {
    return (
      <RecoveryCodesPasswordStep
        error={error}
        loading={loading}
        password={password}
        onCancel={() => void navigate({ to: '/account/security' })}
        onPasswordChange={setPassword}
        onSubmit={handleGenerate}
      />
    );
  }

  return (
    <RecoveryCodesListStep
      codes={codes}
      onBack={() => void navigate({ to: '/account/security' })}
      onDownload={download}
    />
  );
}
