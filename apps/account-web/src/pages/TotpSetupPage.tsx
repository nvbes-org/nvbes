import StepUpModal from '@/components/StepUpModal';
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

  const isStepUp = step === 'stepup';

  return (
    <>
      <StepUpModal
        open={isStepUp}
        onSuccess={onStepUpSuccess}
        onCancel={navigateBack}
        description="Pour configurer un code d'authentification, veuillez confirmer votre identité."
      />

      {step === 'confirm' ? (
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
      ) : (
        <TotpSetupCard
          label={label}
          error={error}
          loading={loading || isStepUp}
          onLabelChange={onLabelChange}
          onCancel={navigateBack}
          onSubmit={onSetup}
        />
      )}
    </>
  );
}
