import { useLocation, useNavigate } from '@tanstack/react-router';
import { authuserSearch, readAuthuser } from '@/identity.authuser';
import { useMfaFactors } from './useMfaPage.factors';
import { useMfaStepUp } from './useMfaPage.step-up';

export function useMfaPage() {
  const location = useLocation();
  const authuser = readAuthuser(location.searchStr);
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
    navigateBack: () =>
      void navigate({ to: '/account/security', search: authuserSearch(authuser) }),
    navigateTo: (path: string) => void navigate({ to: path, search: authuserSearch(authuser) }),
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
