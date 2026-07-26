import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { PasswordStrengthMeter } from './RegisterPage.password';
import { ResetPasswordErrorMessage } from './ResetPasswordPage.messages';
import type { ResetPasswordPageModel } from './ResetPasswordPage.types';

export function ResetPasswordForm({
  tokenFromUrl,
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
  | 'tokenFromUrl'
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
      {!tokenFromUrl && (
        <div className="flex flex-col gap-2">
          <Label htmlFor="reset-token">Code de reinitialisation</Label>
          <Input
            id="reset-token"
            type="text"
            placeholder="Collez le code reçu par email"
            value={token}
            onChange={(event) => setToken(event.target.value)}
            required
            autoFocus
          />
        </div>
      )}
      <div className="flex flex-col gap-2">
        <Label htmlFor="reset-password">Nouveau mot de passe</Label>
        <Input
          id="reset-password"
          type="password"
          placeholder="••••••••"
          value={password}
          onChange={(event) => setPassword(event.target.value)}
          required
          autoComplete="new-password"
          autoFocus={!!tokenFromUrl}
        />
        <PasswordStrengthMeter password={password} />
      </div>
      <div className="flex flex-col gap-2">
        <Label htmlFor="reset-confirm">Confirmer le mot de passe</Label>
        <Input
          id="reset-confirm"
          type="password"
          placeholder="••••••••"
          value={confirmPassword}
          onChange={(event) => setConfirmPassword(event.target.value)}
          required
          autoComplete="new-password"
        />
      </div>
      {error && <ResetPasswordErrorMessage message={error} />}
      <Button type="submit" disabled={isPending} className="w-full" size="lg">
        {isPending ? 'Reinitialisation...' : 'Reinitialiser le mot de passe'}
      </Button>
    </form>
  );
}
