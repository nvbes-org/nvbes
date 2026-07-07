import StepUpForm from '@/components/StepUpForm';
import { TotpConfirmCard, TotpSetupCard } from './TotpSetupPage.shared';
import { useTotpSetupPage } from './useTotpSetupPage';

export default function TotpSetupPage() {
  const {
    step,
    label,
    secretBase32,
    totpCode,
    error,
    loading,
    qrData,
    navigateBack,
    onStepUpSuccess,
    onLabelChange,
    onTotpCodeChange,
    onSetup,
    onConfirm,
  } = useTotpSetupPage();

  if (step === 'stepup') {
    return (
      <div className="flex min-h-screen items-center justify-center">
        <StepUpForm
          onSuccess={onStepUpSuccess}
          onCancel={navigateBack}
          description="Pour configurer un code d'authentification, veuillez confirmer votre identité."
        />
      </div>
    );
  }

  if (step === 'setup') {
    return (
      <TotpSetupCard
        label={label}
        error={error}
        loading={loading}
        onLabelChange={onLabelChange}
        onCancel={navigateBack}
        onSubmit={onSetup}
      />
    );
  }

  if (step === 'confirm') {
    return (
      <TotpConfirmCard
        qrData={qrData}
        secretBase32={secretBase32}
        totpCode={totpCode}
        error={error}
        loading={loading}
        onTotpCodeChange={onTotpCodeChange}
        onCancel={navigateBack}
        onSubmit={onConfirm}
      />
    );
  }

  return null;
}
