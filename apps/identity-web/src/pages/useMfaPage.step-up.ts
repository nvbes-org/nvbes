import { removeMfaFactor } from '@nvbes/identity-sdk-web';
import { useCallback, useState } from 'react';
import { StepUpMethod } from './MfaPage.shared';
import {
  getStepUpErrorMessage,
  removeFactorAfterStepUp,
  runStepUpVerification,
  type SetFactors,
} from './useMfaPage.step-up.helpers';
import { resetMfaStepUpState } from './useMfaPage.step-up.state';

import { isStepUpRequiredError } from '@/identity.step-up';

export function useMfaStepUp({ setFactors }: { setFactors: SetFactors }) {
  const [removingId, setRemovingId] = useState<string | null>(null);
  const [showStepUp, setShowStepUp] = useState(false);
  const [stepUpMethod, setStepUpMethod] = useState<StepUpMethod | null>(null);
  const [stepUpPassword, setStepUpPassword] = useState('');
  const [stepUpTotpCode, setStepUpTotpCode] = useState('');
  const [stepUpRecoveryCode, setStepUpRecoveryCode] = useState('');
  const [stepUpLoading, setStepUpLoading] = useState(false);
  const [stepUpError, setStepUpError] = useState<string | null>(null);

  const resetStepUpState = useCallback(() => {
    resetMfaStepUpState({
      setStepUpMethod,
      setStepUpPassword,
      setStepUpTotpCode,
      setStepUpRecoveryCode,
      setStepUpError,
    });
  }, []);

  const handleRemove = useCallback(
    async (factorId: string) => {
      setRemovingId(factorId);
      setStepUpError(null);

      try {
        await removeMfaFactor('', factorId);
        setFactors((prev) => prev.filter((factor) => factor.id !== factorId));
        setRemovingId(null);
      } catch (err) {
        if (isStepUpRequiredError(err)) {
          setShowStepUp(true);
          return;
        }

        const message = err instanceof Error ? err.message : 'Failed to remove factor';
        setStepUpError(message);
        setRemovingId(null);
      }
    },
    [setFactors],
  );

  const handleStepUp = useCallback(
    async (event: React.SubmitEvent<HTMLFormElement>) => {
      event.preventDefault();
      setStepUpLoading(true);
      setStepUpError(null);

      try {
        await runStepUpVerification({
          stepUpMethod,
          stepUpPassword,
          stepUpTotpCode,
          stepUpRecoveryCode,
        });

        setShowStepUp(false);
        resetStepUpState();

        if (removingId) {
          await removeFactorAfterStepUp(removingId, setFactors);
          setRemovingId(null);
        }
      } catch (err) {
        setStepUpError(getStepUpErrorMessage(err));
      } finally {
        setStepUpLoading(false);
      }
    },
    [
      removingId,
      resetStepUpState,
      setFactors,
      stepUpMethod,
      stepUpPassword,
      stepUpRecoveryCode,
      stepUpTotpCode,
    ],
  );

  const cancelStepUp = useCallback(() => {
    setShowStepUp(false);
    resetStepUpState();
    setRemovingId(null);
  }, [resetStepUpState]);

  return {
    removingId,
    showStepUp,
    stepUpMethod,
    stepUpPassword,
    stepUpTotpCode,
    stepUpRecoveryCode,
    stepUpLoading,
    stepUpError,
    handleRemove,
    handleStepUp,
    cancelStepUp,
    setStepUpMethod,
    setStepUpPassword,
    setStepUpTotpCode,
    setStepUpRecoveryCode,
    resetStepUpMethod: () => {
      setStepUpMethod(null);
      setStepUpError(null);
    },
  };
}
