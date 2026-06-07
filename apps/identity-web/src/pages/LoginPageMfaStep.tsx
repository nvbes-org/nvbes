import { Separator } from '@/components/ui/separator';
import {
  LoginPageMfaMethodChoices,
  LoginPageMfaMethodForm,
  type LoginPageMfaStepProps,
} from './LoginPageMfaStep.shared';

export function LoginPageMfaStep({
  error,
  loading,
  mfaMethod,
  hasTotp,
  hasWebAuthn,
  hasRecovery,
  availableCount,
  totpCode,
  recoveryCode,
  onTotpCodeChange,
  onRecoveryCodeChange,
  onMfaMethodSelect,
  onMfaSubmit,
  onBackToMethodSelect,
  onResetToIdentifier,
}: LoginPageMfaStepProps) {
  return (
    <div className="flex flex-col gap-5">
      {!mfaMethod ? (
        <LoginPageMfaMethodChoices
          hasTotp={hasTotp}
          hasWebAuthn={hasWebAuthn}
          hasRecovery={hasRecovery}
          availableCount={availableCount}
          onMfaMethodSelect={onMfaMethodSelect}
        />
      ) : (
        <LoginPageMfaMethodForm
          error={error}
          loading={loading}
          mfaMethod={mfaMethod}
          totpCode={totpCode}
          recoveryCode={recoveryCode}
          onTotpCodeChange={onTotpCodeChange}
          onRecoveryCodeChange={onRecoveryCodeChange}
          onMfaSubmit={onMfaSubmit}
          onBackToMethodSelect={onBackToMethodSelect}
        />
      )}

      <Separator />

      <button
        type="button"
        className="text-center text-sm text-muted-foreground transition-colors hover:text-foreground"
        onClick={onResetToIdentifier}
      >
        Pas votre compte ? Revenir à l&apos;identification
      </button>
    </div>
  );
}
