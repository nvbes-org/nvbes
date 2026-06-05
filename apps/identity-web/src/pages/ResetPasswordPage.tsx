import { useMutation } from '@tanstack/react-query';
import { useLocation, useNavigate } from '@tanstack/react-router';
import { ArrowLeft } from 'lucide-react';
import { useState } from 'react';
import { Button } from '@/components/ui/button';
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { resetPassword } from '../identity.password.api';

function ErrorMessage({ message }: { message: string }) {
  return (
    <div className="rounded-lg border border-destructive/30 bg-destructive/5 px-3 py-2 text-sm text-destructive">
      {message}
    </div>
  );
}

function SuccessMessage({ message }: { message: string }) {
  return (
    <div className="rounded-lg border border-emerald-500/30 bg-emerald-500/5 px-3 py-2 text-sm text-emerald-600">
      {message}
    </div>
  );
}

export default function ResetPasswordPage() {
  const navigate = useNavigate();
  const location = useLocation();
  const searchParams = new URLSearchParams(location.searchStr);
  const tokenFromUrl = searchParams.get('token') ?? '';

  const [token, setToken] = useState(tokenFromUrl);
  const [password, setPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState(false);

  const mutation = useMutation({
    mutationFn: () => resetPassword(token, password),
    onSuccess: () => setSuccess(true),
    onError: () => setError('Le lien est invalide ou a expire. Veuillez recommencer.'),
  });

  const handleSubmit = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    setError(null);

    if (password.length < 8) {
      setError('Le mot de passe doit contenir au moins 8 caracteres.');
      return;
    }
    if (password !== confirmPassword) {
      setError('Les mots de passe ne correspondent pas.');
      return;
    }

    mutation.mutate();
  };

  return (
    <div className="flex min-h-screen items-center justify-center bg-muted/30 p-4 sm:p-8">
      <div className="w-full max-w-sm animate-fade-slide-up [animation-delay:150ms]">
        <button
          type="button"
          className="mb-6 flex items-center gap-1.5 text-sm text-muted-foreground hover:text-foreground transition-colors"
          onClick={() => navigate({ to: '/login' })}
        >
          <ArrowLeft className="size-3.5" />
          Retour a la connexion
        </button>

        <Card>
          <CardHeader>
            <CardTitle>Nouveau mot de passe</CardTitle>
            <CardDescription>Choisissez un nouveau mot de passe pour votre compte.</CardDescription>
          </CardHeader>
          <CardContent>
            {success ? (
              <div className="flex flex-col gap-5">
                <SuccessMessage message="Votre mot de passe a ete reinitialise avec succes." />
                <Button className="w-full" onClick={() => navigate({ to: '/login' })}>
                  Se connecter
                </Button>
              </div>
            ) : (
              <form onSubmit={handleSubmit} className="flex flex-col gap-5">
                {!tokenFromUrl && (
                  <div className="flex flex-col gap-2">
                    <Label htmlFor="reset-token">Code de reinitialisation</Label>
                    <Input
                      id="reset-token"
                      type="text"
                      placeholder="Collez le code reçu par email"
                      value={token}
                      onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
                        setToken(e.target.value)
                      }
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
                    onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
                      setPassword(e.target.value)
                    }
                    required
                    autoComplete="new-password"
                    autoFocus={!!tokenFromUrl}
                  />
                </div>
                <div className="flex flex-col gap-2">
                  <Label htmlFor="reset-confirm">Confirmer le mot de passe</Label>
                  <Input
                    id="reset-confirm"
                    type="password"
                    placeholder="••••••••"
                    value={confirmPassword}
                    onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
                      setConfirmPassword(e.target.value)
                    }
                    required
                    autoComplete="new-password"
                  />
                </div>
                {error && <ErrorMessage message={error} />}
                <Button type="submit" disabled={mutation.isPending} className="w-full" size="lg">
                  {mutation.isPending ? 'Reinitialisation...' : 'Reinitialiser le mot de passe'}
                </Button>
              </form>
            )}
          </CardContent>
          <CardFooter className="justify-center">
            <p className="text-center text-sm text-muted-foreground">
              <a href="/forgot-password" className="font-medium text-primary hover:underline">
                Renvoyer le lien
              </a>
            </p>
          </CardFooter>
        </Card>
      </div>
    </div>
  );
}
