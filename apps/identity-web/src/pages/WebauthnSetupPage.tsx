import StepUpForm from '@/components/StepUpForm';
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

  if (step === 'stepup') {
    return (
      <div className="flex min-h-screen items-center justify-center">
        <StepUpForm
          onSuccess={onStepUpSuccess}
          onCancel={navigateBack}
          description={copy.stepUpText}
        />
      </div>
    );
  }

  if (step === 'register') {
    return (
      <WebauthnSetupRegisterCard
        title={copy.title}
        label={label}
        labelPlaceholder={copy.labelPlaceholder}
        supportMessage={support?.message ?? null}
        supported={supported}
        showPlatformWarning={showPlatformWarning}
        error={error}
        loading={loading}
        buttonText={copy.buttonText}
        onLabelChange={onLabelChange}
        onCancel={navigateBack}
        onSubmit={onRegister}
      />
    );
  }

  return null;
}
