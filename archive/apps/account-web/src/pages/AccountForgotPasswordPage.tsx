import { useMutation } from '@tanstack/react-query';
import { Link } from '@tanstack/react-router';
import { useState, type FormEvent } from 'react';
import { requestAccountPasswordReset } from '@/account.identity';
import { ACCOUNT_WEB_PATHS } from '@/account.routes';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';

export default function AccountForgotPasswordPage() {
  const [email, setEmail] = useState('');
  const mutation = useMutation({ mutationFn: () => requestAccountPasswordReset(email) });
  const submit = (event: FormEvent) => {
    event.preventDefault();
    if (email) mutation.mutate();
  };
  return (
    <PublicAccountPage title="Mot de passe oublié">
      {mutation.isSuccess ? (
        <Alert>
          <AlertDescription>
            Si ce compte existe, un lien de réinitialisation a été envoyé.
          </AlertDescription>
        </Alert>
      ) : (
        <form className="space-y-4" onSubmit={submit}>
          <div className="space-y-2">
            <Label htmlFor="reset-email">Adresse e-mail</Label>
            <Input
              id="reset-email"
              type="email"
              autoComplete="email"
              value={email}
              onChange={(event) => setEmail(event.target.value)}
            />
          </div>
          <Button className="w-full" type="submit" disabled={!email || mutation.isPending}>
            Envoyer le lien
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

export function PublicAccountPage({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
  return (
    <main className="flex min-h-screen items-center justify-center bg-background px-6">
      <section className="w-full max-w-md space-y-6">
        <header>
          <p className="text-sm font-semibold text-primary">nvbes Account</p>
          <h1 className="mt-2 text-2xl font-semibold">{title}</h1>
        </header>
        {children}
        <Link className="text-sm text-primary hover:underline" to={ACCOUNT_WEB_PATHS.profile}>
          Retour à la connexion
        </Link>
      </section>
    </main>
  );
}
