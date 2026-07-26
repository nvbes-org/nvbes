import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  AccountPasswordErrorMessage,
  AccountPasswordSuccessMessage,
} from '@/pages/AccountPasswordPage.messages';
import type { AccountPasswordPageModel } from '@/pages/AccountPasswordPage.types';
import { PasswordStrengthMeter } from '@/pages/RegisterPage.password';

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
>) {
  return (
    <form onSubmit={handleSubmit} className="flex flex-col gap-5">
      <div className="flex flex-col gap-2">
        <Label htmlFor="password-username">Compte</Label>
        <Input
          id="password-username"
          name="username"
          type="email"
          value={accountEmail}
          readOnly
          autoComplete="username"
        />
      </div>

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
          minLength={8}
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
        />
      </div>

      {error ? <AccountPasswordErrorMessage message={error} /> : null}
      {success ? (
        <AccountPasswordSuccessMessage message="Votre mot de passe a ete modifie avec succes." />
      ) : null}

      <Button type="submit" disabled={mutation.isPending} className="w-full">
        {mutation.isPending ? 'Modification...' : 'Modifier le mot de passe'}
      </Button>
    </form>
  );
}
