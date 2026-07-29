import { useNavigate } from '@tanstack/react-router';
import { useAuthuser } from '@/hooks/useAuthuser';
import { accountPathForAuthuser } from '@/identity.authuser';
import { useMfaFactors } from './useMfaPage.factors';
import { useMfaStepUp } from './useMfaPage.step-up';

export function useMfaPage() {
  const authuser = useAuthuser();
  const navigate = useNavigate();
  const { factors, loading, error, setFactors, hasMore, loadMore, loadingMore } = useMfaFactors();
  const stepUp = useMfaStepUp({ setFactors });

  return {
    factors,
    loading,
    error,
    hasMore,
    loadMore,
    loadingMore,
    removingId: stepUp.removingId,
    showStepUp: stepUp.showStepUp,
    stepUpMethod: stepUp.stepUpMethod,
    stepUpPassword: stepUp.stepUpPassword,
    stepUpTotpCode: stepUp.stepUpTotpCode,
    stepUpRecoveryCode: stepUp.stepUpRecoveryCode,
    stepUpLoading: stepUp.stepUpLoading,
    stepUpError: stepUp.stepUpError,
    hasTotp: factors.some((factor) => factor.factor_type === 'totp'),
    hasWebAuthn: factors.some((factor) => factor.factor_type === 'webauthn'),
    hasRecovery: factors.some(
      (factor) => factor.factor_type === 'recovery' || factor.factor_type === 'recovery_code',
    ),
    canUsePassword: !factors.some((factor) => factor.factor_type !== 'email'),
    recoveryCreatedAt: factors.find(
      (factor) => factor.factor_type === 'recovery' || factor.factor_type === 'recovery_code',
    )?.created_at,
    navigateBack: () =>
      void navigate({
        to: '/account/$accountIndex/security',
        params: { accountIndex: authuser },
      }),
    navigateTo: (path: string) => {
      window.location.assign(accountPathForAuthuser(authuser, path));
    },
    handleRemove: stepUp.handleRemove,
    handleStepUp: stepUp.handleStepUp,
    cancelStepUp: stepUp.cancelStepUp,
    setStepUpMethod: stepUp.setStepUpMethod,
    setStepUpPassword: stepUp.setStepUpPassword,
    setStepUpTotpCode: stepUp.setStepUpTotpCode,
    setStepUpRecoveryCode: stepUp.setStepUpRecoveryCode,
    resetStepUpMethod: stepUp.resetStepUpMethod,
  };
}
