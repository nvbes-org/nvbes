import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  AccountPasswordErrorMessage,
  AccountPasswordSuccessMessage,
} from '@/pages/AccountPasswordPage.messages';
import type { AccountPasswordPageModel } from '@/pages/AccountPasswordPage.types';
import { PasswordStrengthMeter } from '@/pages/RegisterPage.password';
import { MAX_PASSWORD_LENGTH, MIN_PASSWORD_LENGTH } from '../identity.password.policy';

export function AccountPasswordForm({
  accountEmail,
  confirmPassword,
  error,
  handleSubmit,
  mutation,
  newPassword,
  setConfirmPassword,
  setNewPassword,
  success,
  requestResetEmail,
  resetEmailMutation,
  resetEmailResendSeconds,
  resetEmailSent,
}: Pick<
  AccountPasswordPageModel,
  | 'accountEmail'
  | 'confirmPassword'
  | 'error'
  | 'handleSubmit'
  | 'mutation'
  | 'newPassword'
  | 'setConfirmPassword'
  | 'setNewPassword'
  | 'success'
  | 'requestResetEmail'
  | 'resetEmailMutation'
  | 'resetEmailResendSeconds'
  | 'resetEmailSent'
>) {
  return (
    <form onSubmit={handleSubmit} className="flex flex-col gap-5">
      <input name="username" type="hidden" value={accountEmail} autoComplete="username" readOnly />

      <div className="flex flex-col gap-2">
        <Label htmlFor="new-password">Nouveau mot de passe</Label>
        <Input
          id="new-password"
          name="new-password"
          type="password"
          placeholder="••••••••"
          value={newPassword}
          onChange={(event: React.ChangeEvent<HTMLInputElement>) =>
            setNewPassword(event.target.value)
          }
          required
          autoComplete="new-password"
          minLength={MIN_PASSWORD_LENGTH}
          maxLength={MAX_PASSWORD_LENGTH}
          spellCheck={false}
          autoFocus
        />
        <PasswordStrengthMeter password={newPassword} />
      </div>

      <div className="flex flex-col gap-2">
        <Label htmlFor="confirm-password">Confirmer le nouveau mot de passe</Label>
        <Input
          id="confirm-password"
          name="confirm-password"
          type="password"
          placeholder="••••••••"
          value={confirmPassword}
          onChange={(event: React.ChangeEvent<HTMLInputElement>) =>
            setConfirmPassword(event.target.value)
          }
          required
          autoComplete="new-password"
          minLength={MIN_PASSWORD_LENGTH}
          maxLength={MAX_PASSWORD_LENGTH}
          spellCheck={false}
        />
      </div>

      {error ? <AccountPasswordErrorMessage message={error} /> : null}
      {success ? (
        <AccountPasswordSuccessMessage message="Votre mot de passe a ete modifie avec succes." />
      ) : null}

      <Button type="submit" disabled={mutation.isPending} className="w-full">
        {mutation.isPending ? 'Modification...' : 'Modifier le mot de passe'}
      </Button>

      <div className="border-t pt-4 text-center">
        <Button
          type="button"
          variant="link"
          className="h-auto p-0"
          disabled={resetEmailMutation.isPending || resetEmailResendSeconds > 0}
          onClick={requestResetEmail}
        >
          <span aria-live="polite">
            {resetEmailMutation.isPending
              ? 'Envoi en cours...'
              : resetEmailResendSeconds > 0
                ? `Renvoyer dans ${formatCountdown(resetEmailResendSeconds)}`
                : resetEmailSent
                  ? "Renvoyer l'email de réinitialisation"
                  : 'Réinitialiser le mot de passe via email'}
          </span>
        </Button>
      </div>
    </form>
  );
}

function formatCountdown(seconds: number): string {
  const minutes = Math.floor(seconds / 60);
  const remainingSeconds = seconds % 60;
  return `${minutes}:${remainingSeconds.toString().padStart(2, '0')}`;
}
