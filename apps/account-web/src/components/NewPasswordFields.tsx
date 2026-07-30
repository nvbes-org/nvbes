import { Field, FieldLabel } from '@/components/ui/field';
import { Input } from '@/components/ui/input';
import { MAX_PASSWORD_LENGTH, MIN_PASSWORD_LENGTH } from '@/identity.password.policy';
import { PasswordStrengthMeter } from './PasswordStrengthMeter';

export function NewPasswordFields({
  password,
  confirmation,
  passwordId,
  confirmationId,
  passwordLabel = 'Nouveau mot de passe',
  confirmationLabel = 'Confirmer le nouveau mot de passe',
  passwordAutoFocus = false,
  onPasswordChange,
  onConfirmationChange,
}: {
  password: string;
  confirmation: string;
  passwordId: string;
  confirmationId: string;
  passwordLabel?: string;
  confirmationLabel?: string;
  passwordAutoFocus?: boolean;
  onPasswordChange: (value: string) => void;
  onConfirmationChange: (value: string) => void;
}) {
  return (
    <>
      <Field>
        <FieldLabel htmlFor={passwordId}>{passwordLabel}</FieldLabel>
        <Input
          id={passwordId}
          name="new-password"
          type="password"
          placeholder="••••••••"
          value={password}
          onChange={(event) => onPasswordChange(event.target.value)}
          required
          autoComplete="new-password"
          minLength={MIN_PASSWORD_LENGTH}
          maxLength={MAX_PASSWORD_LENGTH}
          spellCheck={false}
          autoFocus={passwordAutoFocus}
        />
        <PasswordStrengthMeter password={password} />
      </Field>
      <Field>
        <FieldLabel htmlFor={confirmationId}>{confirmationLabel}</FieldLabel>
        <Input
          id={confirmationId}
          name="confirm-password"
          type="password"
          placeholder="••••••••"
          value={confirmation}
          onChange={(event) => onConfirmationChange(event.target.value)}
          required
          autoComplete="new-password"
          minLength={MIN_PASSWORD_LENGTH}
          maxLength={MAX_PASSWORD_LENGTH}
          spellCheck={false}
        />
      </Field>
    </>
  );
}
