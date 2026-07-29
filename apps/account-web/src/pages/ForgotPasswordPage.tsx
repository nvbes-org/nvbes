import { useMutation } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { ArrowLeft } from 'lucide-react';
import { useState } from 'react';
import { Alert, AlertDescription } from '@/components/ui/alert';
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
    <Alert variant="destructive">
      <AlertDescription>{message}</AlertDescription>
    </Alert>
  );
}

function SuccessMessage({ message }: { message: string }) {
  return (
    <Alert>
      <AlertDescription>{message}</AlertDescription>
    </Alert>
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

  const handleSubmit = async (e: React.SubmitEvent<HTMLFormElement>) => {
    e.preventDefault();
    setError(null);
    mutation.mutate(email);
  };

  return (
    <div className="flex min-h-screen items-center justify-center bg-muted/30 p-4 sm:p-8">
      <div className="w-full max-w-sm animate-fade-slide-up [animation-delay:150ms]">
        <Button
          type="button"
          variant="ghost"
          className="mb-6 px-0 text-muted-foreground hover:text-foreground"
          onClick={() => navigate({ to: '/login' })}
        >
          <ArrowLeft data-icon="inline-start" />
          Retour a la connexion
        </Button>

        <Card>
          <CardHeader>
            <CardTitle role="heading" aria-level={1}>
              Mot de passe oublie
            </CardTitle>
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
