import { useMutation } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
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
import { forgotPassword } from '../identity.password.api';

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

export default function ForgotPasswordPage() {
  const navigate = useNavigate();
  const [email, setEmail] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState(false);

  const mutation = useMutation({
    mutationFn: (email: string) => forgotPassword(email),
    onSuccess: () => setSuccess(true),
    onError: () => setError('Une erreur est survenue. Veuillez reessayer.'),
  });

  const handleSubmit = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    setError(null);
    mutation.mutate(email);
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
            <CardTitle>Mot de passe oublie</CardTitle>
            <CardDescription>
              Entrez votre adresse email pour recevoir un lien de reinitialisation.
            </CardDescription>
          </CardHeader>
          <CardContent>
            {success ? (
              <div className="flex flex-col gap-5">
                <SuccessMessage
                  message={`Si un compte existe avec l'adresse ${email}, vous recevrez un email avec les instructions pour reinitialiser votre mot de passe.`}
                />
                <Button
                  variant="outline"
                  className="w-full"
                  onClick={() => navigate({ to: '/login' })}
                >
                  Retour a la connexion
                </Button>
              </div>
            ) : (
              <form onSubmit={handleSubmit} className="flex flex-col gap-5">
                <div className="flex flex-col gap-2">
                  <Label htmlFor="forgot-email">Email</Label>
                  <Input
                    id="forgot-email"
                    type="email"
                    placeholder="vous@exemple.fr"
                    value={email}
                    onChange={(e: React.ChangeEvent<HTMLInputElement>) => setEmail(e.target.value)}
                    required
                    autoComplete="email"
                    autoFocus
                  />
                </div>
                {error && <ErrorMessage message={error} />}
                <Button type="submit" disabled={mutation.isPending} className="w-full" size="lg">
                  {mutation.isPending ? 'Envoi...' : 'Envoyer le lien'}
                </Button>
              </form>
            )}
          </CardContent>
          <CardFooter className="justify-center">
            <p className="text-center text-sm text-muted-foreground">
              <a href="/register" className="font-medium text-primary hover:underline">
                Creer un compte
              </a>
            </p>
          </CardFooter>
        </Card>
      </div>
    </div>
  );
}
