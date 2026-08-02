import { useMutation } from '@tanstack/react-query';
import { useState, type FormEvent } from 'react';
import { resetAccountPassword } from '@/account.identity';
import { clearPasswordResetToken, readPasswordResetToken } from '@/account.password-reset-token';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { PublicAccountPage } from './AccountForgotPasswordPage';

export default function AccountResetPasswordPage() {
  const token = readPasswordResetToken();
  const [password, setPassword] = useState('');
  const [confirmation, setConfirmation] = useState('');
  const mutation = useMutation({
    mutationFn: () => resetAccountPassword(token, password),
    onSuccess: clearPasswordResetToken,
  });
  const submit = (event: FormEvent) => {
    event.preventDefault();
    if (token && password.length >= 12 && password === confirmation) mutation.mutate();
  };
  return (
    <PublicAccountPage title="Réinitialiser le mot de passe">
      {!token ? (
        <Alert variant="destructive">
          <AlertDescription>Le lien de réinitialisation est incomplet.</AlertDescription>
        </Alert>
      ) : mutation.isSuccess ? (
        <Alert>
          <AlertDescription>
            Le mot de passe a été réinitialisé. Vous pouvez vous reconnecter.
          </AlertDescription>
        </Alert>
      ) : (
        <form className="space-y-4" onSubmit={submit}>
          <div className="space-y-2">
            <Label htmlFor="reset-password">Nouveau mot de passe</Label>
            <Input
              id="reset-password"
              type="password"
              autoComplete="new-password"
              value={password}
              onChange={(event) => setPassword(event.target.value)}
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="reset-confirmation">Confirmation</Label>
            <Input
              id="reset-confirmation"
              type="password"
              autoComplete="new-password"
              value={confirmation}
              onChange={(event) => setConfirmation(event.target.value)}
            />
          </div>
          {confirmation && password !== confirmation ? (
            <p className="text-sm text-destructive">Les mots de passe ne correspondent pas.</p>
          ) : null}
          <Button
            className="w-full"
            type="submit"
            disabled={password.length < 12 || password !== confirmation || mutation.isPending}
          >
            Réinitialiser
          </Button>
        </form>
      )}
      {mutation.error ? (
        <Alert variant="destructive">
          <AlertDescription>{mutation.error.message}</AlertDescription>
        </Alert>
      ) : null}
    </PublicAccountPage>
  );
}
