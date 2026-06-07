export function resetMfaStepUpState({
  setStepUpMethod,
  setStepUpPassword,
  setStepUpTotpCode,
  setStepUpRecoveryCode,
  setStepUpError,
}: {
  setStepUpMethod: (value: null) => void;
  setStepUpPassword: (value: string) => void;
  setStepUpTotpCode: (value: string) => void;
  setStepUpRecoveryCode: (value: string) => void;
  setStepUpError: (value: string | null) => void;
}) {
  setStepUpMethod(null);
  setStepUpPassword('');
  setStepUpTotpCode('');
  setStepUpRecoveryCode('');
  setStepUpError(null);
}
