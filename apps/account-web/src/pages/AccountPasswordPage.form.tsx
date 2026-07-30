import { FeedbackAlert } from '@/components/FeedbackAlert';
import { NewPasswordFields } from '@/components/NewPasswordFields';
import { Button } from '@/components/ui/button';
import type { AccountPasswordPageModel } from '@/pages/AccountPasswordPage.types';

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

      <NewPasswordFields
        password={newPassword}
        confirmation={confirmPassword}
        passwordId="new-password"
        confirmationId="confirm-password"
        passwordAutoFocus
        onPasswordChange={setNewPassword}
        onConfirmationChange={setConfirmPassword}
      />

      {error ? <FeedbackAlert tone="error">{error}</FeedbackAlert> : null}
      {success ? (
        <FeedbackAlert>Votre mot de passe a ete modifie avec succes.</FeedbackAlert>
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
