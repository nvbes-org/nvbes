import { Button } from '@/components/ui/button';
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
  hasEmail,
  hasWebAuthn,
  hasRecovery,
  availableCount,
  totpCode,
  emailCode,
  recoveryCode,
  onTotpCodeChange,
  onEmailCodeChange,
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
          hasEmail={hasEmail}
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
          emailCode={emailCode}
          recoveryCode={recoveryCode}
          onTotpCodeChange={onTotpCodeChange}
          onEmailCodeChange={onEmailCodeChange}
          onRecoveryCodeChange={onRecoveryCodeChange}
          onMfaSubmit={onMfaSubmit}
          onBackToMethodSelect={onBackToMethodSelect}
        />
      )}

      <Separator />

      <Button
        type="button"
        variant="ghost"
        className="text-muted-foreground hover:text-foreground"
        onClick={onResetToIdentifier}
      >
        Pas votre compte ? Revenir à l&apos;identification
      </Button>
    </div>
  );
}
