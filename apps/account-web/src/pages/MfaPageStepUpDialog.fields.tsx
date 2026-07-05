import { Input } from '@/components/ui/input';
import type { MfaPageStepUpDialogProps } from './MfaPageStepUpDialog.types';

export function MfaPageStepUpFormFields({
  stepUpMethod,
  stepUpPassword,
  stepUpTotpCode,
  stepUpRecoveryCode,
  onPasswordChange,
  onTotpCodeChange,
  onRecoveryCodeChange,
}: Pick<
  MfaPageStepUpDialogProps,
  | 'stepUpMethod'
  | 'stepUpPassword'
  | 'stepUpTotpCode'
  | 'stepUpRecoveryCode'
  | 'onPasswordChange'
  | 'onTotpCodeChange'
  | 'onRecoveryCodeChange'
>) {
  if (stepUpMethod === 'password') {
    return (
      <Input
        type="password"
        placeholder="Mot de passe"
        value={stepUpPassword}
        onChange={(event) => onPasswordChange(event.target.value)}
        required
        autoFocus
      />
    );
  }

  if (stepUpMethod === 'totp') {
    return (
      <Input
        type="text"
        inputMode="numeric"
        autoComplete="one-time-code"
        placeholder="000000"
        maxLength={6}
        value={stepUpTotpCode}
        onChange={(event) => onTotpCodeChange(event.target.value)}
        required
        autoFocus
      />
    );
  }

  if (stepUpMethod === 'webauthn') {
    return (
      <p className="text-sm text-muted-foreground">
        Cliquez sur Confirmer pour utiliser votre biométrie ou clé de sécurité.
      </p>
    );
  }

  return (
    <Input
      type="text"
      placeholder="XXXX-XXXX-XXXX"
      value={stepUpRecoveryCode}
      onChange={(event) => onRecoveryCodeChange(event.target.value)}
      required
      autoFocus
    />
  );
}
