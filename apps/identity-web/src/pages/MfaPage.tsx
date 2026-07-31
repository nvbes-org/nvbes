import { FeedbackAlert } from '@/components/FeedbackAlert';
import { Button } from '@/components/ui/button';
import { IdentityPage, IdentityPageHeader } from '@/components/IdentityPage';
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
    <IdentityPage>
      <MfaPageHeader onBack={navigateBack} />

      {error && <FeedbackAlert tone="error">{error}</FeedbackAlert>}

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

      <IdentityPageHeader
        as="h2"
        size="subsection"
        title="Ajouter une nouvelle méthode"
        contentClassName="max-w-2xl"
      />

      <MfaPageAddFactorCard onNavigate={navigateTo} />

      <IdentityPageHeader
        as="h2"
        size="subsection"
        title="Codes de récupération"
        description={
          hasRecovery
            ? 'Gardez vos codes dans un endroit sûr et régénérez-les si nécessaire.'
            : 'Générez des codes de secours pour récupérer votre accès.'
        }
        contentClassName="max-w-2xl"
      />

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
    </IdentityPage>
  );
}
