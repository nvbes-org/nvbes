import StepUpModal from '@/components/StepUpModal';
import { type WebauthnSetupKind, WebauthnSetupRegisterCard } from './WebauthnSetupPage.shared';
import { useWebauthnSetupPage } from './useWebauthnSetupPage';

export default function WebauthnSetupPage({ kind = 'security_key' }: { kind?: WebauthnSetupKind }) {
  const {
    copy,
    step,
    label,
    error,
    loading,
    support,
    showPlatformWarning,
    navigateBack,
    onStepUpSuccess,
    onLabelChange,
    onRegister,
    supported,
  } = useWebauthnSetupPage(kind);

  const isStepUp = step === 'stepup';

  return (
    <>
      <StepUpModal
        open={isStepUp}
        onSuccess={onStepUpSuccess}
        onCancel={navigateBack}
        description={copy.stepUpText}
      />

      <WebauthnSetupRegisterCard
        title={copy.title}
        label={label}
        labelPlaceholder={copy.labelPlaceholder}
        supportMessage={support?.message ?? null}
        supported={supported}
        showPlatformWarning={showPlatformWarning}
        error={error}
        loading={loading || isStepUp}
        buttonText={copy.buttonText}
        onLabelChange={onLabelChange}
        onCancel={navigateBack}
        onSubmit={onRegister}
      />
    </>
  );
}
