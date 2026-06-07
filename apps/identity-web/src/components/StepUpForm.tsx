import {
  StepUpActions,
  StepUpError,
  StepUpFormCard,
  StepUpFormHeader,
  StepUpMethodFields,
  StepUpMethodSelect,
} from './StepUpForm.shared';
import { useStepUpForm } from './useStepUpForm';

interface StepUpFormProps {
  onSuccess: () => void;
  onCancel: () => void;
  description?: string;
}

export default function StepUpForm({ onSuccess, onCancel, description }: StepUpFormProps) {
  const {
    error,
    handleSubmit,
    handleWebAuthnClick,
    hasRecovery,
    hasTotp,
    hasWebAuthn,
    loading,
    method,
    password,
    recoveryCode,
    setMethod,
    setPassword,
    setRecoveryCode,
    setTotpCode,
    totpCode,
  } = useStepUpForm({ onSuccess });

  return (
    <StepUpFormCard onSubmit={handleSubmit}>
      <StepUpFormHeader description={description} />
      <StepUpMethodSelect
        method={method}
        hasWebAuthn={hasWebAuthn}
        hasTotp={hasTotp}
        hasRecovery={hasRecovery}
        onMethodChange={setMethod}
      />
      <StepUpMethodFields
        method={method}
        password={password}
        totpCode={totpCode}
        recoveryCode={recoveryCode}
        loading={loading}
        onPasswordChange={setPassword}
        onTotpCodeChange={setTotpCode}
        onRecoveryCodeChange={setRecoveryCode}
        onWebAuthnClick={handleWebAuthnClick}
      />
      {error && <StepUpError error={error} />}
      <StepUpActions method={method} loading={loading} onCancel={onCancel} />
    </StepUpFormCard>
  );
}
