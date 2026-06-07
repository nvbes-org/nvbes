import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Separator } from '@/components/ui/separator';
import {
  AccountPasswordErrorMessage,
  AccountPasswordSuccessMessage,
} from '@/pages/AccountPasswordPage.messages';
import type { AccountPasswordPageModel } from '@/pages/AccountPasswordPage.types';

export function AccountPasswordForm({
  confirmPassword,
  currentPassword,
  error,
  handleSubmit,
  mutation,
  newPassword,
  setConfirmPassword,
  setCurrentPassword,
  setNewPassword,
  success,
}: Pick<
  AccountPasswordPageModel,
  | 'confirmPassword'
  | 'currentPassword'
  | 'error'
  | 'handleSubmit'
  | 'mutation'
  | 'newPassword'
  | 'setConfirmPassword'
  | 'setCurrentPassword'
  | 'setNewPassword'
  | 'success'
>) {
  return (
    <form onSubmit={handleSubmit} className="flex flex-col gap-5">
      <div className="flex flex-col gap-2">
        <Label htmlFor="current-password">Mot de passe actuel</Label>
        <Input
          id="current-password"
          type="password"
          placeholder="••••••••"
          value={currentPassword}
          onChange={(event: React.ChangeEvent<HTMLInputElement>) =>
            setCurrentPassword(event.target.value)
          }
          required
          autoComplete="current-password"
          autoFocus
        />
      </div>

      <Separator />

      <div className="flex flex-col gap-2">
        <Label htmlFor="new-password">Nouveau mot de passe</Label>
        <Input
          id="new-password"
          type="password"
          placeholder="••••••••"
          value={newPassword}
          onChange={(event: React.ChangeEvent<HTMLInputElement>) =>
            setNewPassword(event.target.value)
          }
          required
          autoComplete="new-password"
          minLength={8}
        />
        <p className="text-xs text-muted-foreground">
          Minimum 8 caracteres. Incluez des majuscules, minuscules, chiffres et symboles.
        </p>
      </div>

      <div className="flex flex-col gap-2">
        <Label htmlFor="confirm-password">Confirmer le nouveau mot de passe</Label>
        <Input
          id="confirm-password"
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
