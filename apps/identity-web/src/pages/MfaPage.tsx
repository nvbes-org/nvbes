import { Alert, AlertDescription } from '@/components/ui/alert';
import { Separator } from '@/components/ui/separator';
import { MfaPageAddFactorCard } from './MfaPageAddFactorCard';
import { MfaPageFactorList } from './MfaPageFactorList';
import { MfaPageEmptyState, MfaPageHeader, MfaPageSkeleton } from './MfaPage.layout';
import { MfaPageStepUpDialog } from './MfaPageStepUpDialog';
import { useMfaPage } from './useMfaPage';

export default function MfaPage() {
  const {
    factors,
    loading,
    error,
    removingId,
    showStepUp,
    stepUpMethod,
    stepUpPassword,
    stepUpTotpCode,
    stepUpRecoveryCode,
    stepUpLoading,
    stepUpError,
    hasTotp,
    hasWebAuthn,
    hasRecovery,
    navigateBack,
    navigateTo,
    handleRemove,
    handleStepUp,
    cancelStepUp,
    setStepUpMethod,
    setStepUpPassword,
    setStepUpTotpCode,
    setStepUpRecoveryCode,
    resetStepUpMethod,
  } = useMfaPage();

  if (loading) return <MfaPageSkeleton />;

  return (
    <div className="flex flex-col gap-6 animate-fade-slide-up [animation-delay:0ms]">
      <MfaPageHeader onBack={navigateBack} />

      {error && (
        <Alert variant="destructive">
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}

      {factors.length === 0 ? <MfaPageEmptyState /> : null}

      <MfaPageFactorList factors={factors} removingId={removingId} onRemove={handleRemove} />

      <Separator className="my-2" />

      <MfaPageAddFactorCard hasRecovery={hasRecovery} onNavigate={navigateTo} />

      <MfaPageStepUpDialog
        open={showStepUp}
        stepUpMethod={stepUpMethod}
        hasTotp={hasTotp}
        hasWebAuthn={hasWebAuthn}
        hasRecovery={hasRecovery}
        stepUpPassword={stepUpPassword}
        stepUpTotpCode={stepUpTotpCode}
        stepUpRecoveryCode={stepUpRecoveryCode}
        stepUpLoading={stepUpLoading}
        stepUpError={stepUpError}
        onOpenChange={(open) => {
          if (!open) cancelStepUp();
        }}
        onMethodSelect={setStepUpMethod}
        onPasswordChange={setStepUpPassword}
        onTotpCodeChange={setStepUpTotpCode}
        onRecoveryCodeChange={setStepUpRecoveryCode}
        onBack={() => {
          resetStepUpMethod();
        }}
        onSubmit={handleStepUp}
      />
    </div>
  );
}
