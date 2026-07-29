import { Alert, AlertDescription } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { MfaPageAddFactorCard, MfaPageRecoveryCodesCard } from './MfaPageAddFactorCard';
import { MfaPageFactorList } from './MfaPageFactorList';
import { MfaPageEmptyState, MfaPageHeader, MfaPageSkeleton } from './MfaPage.layout';
import { MfaPageStepUpDialog } from './MfaPageStepUpDialog';
import { useMfaPage } from './useMfaPage';

export default function MfaPage() {
  const {
    factors,
    loading,
    error,
    hasMore,
    loadMore,
    loadingMore,
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
    canUsePassword,
    recoveryCreatedAt,
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

  const visibleFactors = factors.filter(
    (factor) => factor.factor_type !== 'email' && factor.factor_type !== 'recovery_code',
  );

  return (
    <div className="flex flex-col gap-6 animate-fade-slide-up [animation-delay:0ms]">
      <MfaPageHeader onBack={navigateBack} />

      {error && (
        <Alert variant="destructive">
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}

      {visibleFactors.length === 0 && !hasRecovery ? <MfaPageEmptyState /> : null}

      <MfaPageFactorList factors={visibleFactors} removingId={removingId} onRemove={handleRemove} />

      {hasMore && (
        <Button
          type="button"
          variant="outline"
          disabled={loadingMore}
          onClick={() => void loadMore()}
        >
          {loadingMore ? 'Chargement…' : 'Charger plus de facteurs'}
        </Button>
      )}

      <div className="max-w-2xl">
        <h1 className="text-2xl font-heading font-semibold">Ajouter une nouvelle méthode</h1>
      </div>

      <MfaPageAddFactorCard onNavigate={navigateTo} />

      <div className="max-w-2xl">
        <h1 className="text-2xl font-heading font-semibold">Codes de récupération</h1>
        <p className="text-sm text-muted-foreground mt-1">
          {hasRecovery
            ? 'Gardez vos codes dans un endroit sûr et régénérez-les si nécessaire.'
            : 'Générez des codes de secours pour récupérer votre accès.'}
        </p>
      </div>

      <MfaPageRecoveryCodesCard
        hasRecovery={hasRecovery}
        recoveryCreatedAt={recoveryCreatedAt}
        onNavigate={navigateTo}
      />

      <MfaPageStepUpDialog
        open={showStepUp}
        stepUpMethod={stepUpMethod}
        hasTotp={hasTotp}
        hasWebAuthn={hasWebAuthn}
        hasRecovery={hasRecovery}
        canUsePassword={canUsePassword}
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
