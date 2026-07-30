import { FeedbackAlert } from '@/components/FeedbackAlert';
import { NewPasswordFields } from '@/components/NewPasswordFields';
import { Button } from '@/components/ui/button';
import { Field, FieldLabel } from '@/components/ui/field';
import { Input } from '@/components/ui/input';
import type { ResetPasswordPageModel } from './ResetPasswordPage.types';

export function ResetPasswordForm({
  tokenFromLink,
  token,
  password,
  confirmPassword,
  error,
  isPending,
  setToken,
  setPassword,
  setConfirmPassword,
  handleSubmit,
}: Pick<
  ResetPasswordPageModel,
  | 'tokenFromLink'
  | 'token'
  | 'password'
  | 'confirmPassword'
  | 'error'
  | 'isPending'
  | 'setToken'
  | 'setPassword'
  | 'setConfirmPassword'
  | 'handleSubmit'
>) {
  return (
    <form onSubmit={handleSubmit} className="flex flex-col gap-5">
      {!tokenFromLink && (
        <Field>
          <FieldLabel htmlFor="reset-token">Code de reinitialisation</FieldLabel>
          <Input
            id="reset-token"
            type="text"
            placeholder="Collez le code reçu par email"
            value={token}
            onChange={(event) => setToken(event.target.value)}
            required
            autoFocus
          />
        </Field>
      )}
      <NewPasswordFields
        password={password}
        confirmation={confirmPassword}
        passwordId="reset-password"
        confirmationId="reset-confirm"
        confirmationLabel="Confirmer le mot de passe"
        passwordAutoFocus={Boolean(tokenFromLink)}
        onPasswordChange={setPassword}
        onConfirmationChange={setConfirmPassword}
      />
      {error ? <FeedbackAlert tone="error">{error}</FeedbackAlert> : null}
      <Button type="submit" disabled={isPending} className="w-full" size="lg">
        {isPending ? 'Reinitialisation...' : 'Reinitialiser le mot de passe'}
      </Button>
    </form>
  );
}
