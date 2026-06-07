import StepUpForm from '@/components/StepUpForm';
import { TotpConfirmCard, TotpSetupCard, TotpSetupSuccessCard } from './TotpSetupPage.shared';
import { useTotpSetupPage } from './useTotpSetupPage';

export default function TotpSetupPage() {
  const {
    step,
    label,
    secretBase32,
    totpCode,
    error,
    loading,
    qrUrl,
    navigateBack,
    onStepUpSuccess,
    onLabelChange,
    onTotpCodeChange,
    onSetup,
    onConfirm,
    onCopySecret,
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
        qrUrl={qrUrl}
        secretBase32={secretBase32}
        totpCode={totpCode}
        error={error}
        loading={loading}
        onCopySecret={onCopySecret}
        onTotpCodeChange={onTotpCodeChange}
        onCancel={navigateBack}
        onSubmit={onConfirm}
      />
    );
  }

  return <TotpSetupSuccessCard onBack={navigateBack} />;
}
