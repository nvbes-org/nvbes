import { useNavigate } from '@tanstack/react-router';
import { useState } from 'react';
import { downloadRecoveryCodes, generateCodes } from './RecoveryCodesPage.actions';
import { RecoveryCodesListStep } from './RecoveryCodesPage.list';
import { RecoveryCodesStepUp } from './RecoveryCodesPage.stepup';

export default function RecoveryCodesPage() {
  const navigate = useNavigate();
  const navigateBack = () => void navigate({ to: '/mfa' });

  const [step, setStep] = useState<'stepup' | 'codes'>('stepup');
  const [codes, setCodes] = useState<string[]>([]);

  const handleStepUpSuccess = async () => {
    const result = await generateCodes();
    setCodes(result.codes ?? []);
    setStep('codes');
  };

  const download = () => {
    downloadRecoveryCodes(codes);
  };

  return (
    <>
      <RecoveryCodesStepUp
        open={step === 'stepup'}
        onSuccess={handleStepUpSuccess}
        onCancel={navigateBack}
      />
      <RecoveryCodesListStep codes={codes} onBack={navigateBack} onDownload={download} />
    </>
  );
}
