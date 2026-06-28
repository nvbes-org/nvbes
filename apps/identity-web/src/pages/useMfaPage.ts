import { useNavigate } from '@tanstack/react-router';
import { useMfaFactors } from './useMfaPage.factors';
import { useMfaStepUp } from './useMfaPage.step-up';

export function useMfaPage() {
  const navigate = useNavigate();
  const { factors, loading, error, setFactors } = useMfaFactors();
  const stepUp = useMfaStepUp({ setFactors });

  return {
    factors,
    loading,
    error,
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
    navigateBack: () => void navigate({ to: '/account/security' }),
    navigateTo: (path: string) => void navigate({ to: path }),
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
