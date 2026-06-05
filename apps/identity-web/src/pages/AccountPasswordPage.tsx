import { useMutation, useQueryClient } from '@tanstack/react-query';
import { Key } from 'lucide-react';
import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Separator } from '@/components/ui/separator';
import { type ChangePasswordInput, changePassword } from '../identity.password.api';

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

export default function AccountPasswordPage() {
  const queryClient = useQueryClient();
  const [currentPassword, setCurrentPassword] = useState('');
  const [newPassword, setNewPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState(false);

  const mutation = useMutation({
    mutationFn: (input: ChangePasswordInput) => changePassword(input),
    onSuccess: () => {
      setSuccess(true);
      setCurrentPassword('');
      setNewPassword('');
      setConfirmPassword('');
      queryClient.invalidateQueries({ queryKey: ['identity', 'account'] });
    },
    onError: () => {
      setError('Le mot de passe actuel est incorrect ou le nouveau mot de passe est invalide.');
    },
  });

  const handleSubmit = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    setError(null);
    setSuccess(false);

    if (newPassword.length < 8) {
      setError('Le nouveau mot de passe doit contenir au moins 8 caracteres.');
      return;
    }
    if (newPassword !== confirmPassword) {
      setError('Les mots de passe ne correspondent pas.');
      return;
    }
    if (currentPassword === newPassword) {
      setError("Le nouveau mot de passe doit etre different de l'actuel.");
      return;
    }

    mutation.mutate({ current_password: currentPassword, new_password: newPassword });
  };

  return (
    <div className="flex flex-col gap-6 animate-fade-slide-up [animation-delay:0ms]">
      <div>
        <h1 className="text-xl font-heading font-semibold">Mot de passe</h1>
        <p className="text-sm text-muted-foreground mt-1">Modifier votre mot de passe.</p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Changer le mot de passe</CardTitle>
          <CardDescription>
            Utilisez un mot de passe fort et unique que vous n&apos;utilisez pas ailleurs.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleSubmit} className="flex flex-col gap-5">
            <div className="flex flex-col gap-2">
              <Label htmlFor="current-password">Mot de passe actuel</Label>
              <Input
                id="current-password"
                type="password"
                placeholder="••••••••"
                value={currentPassword}
                onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
                  setCurrentPassword(e.target.value)
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
                onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
                  setNewPassword(e.target.value)
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
                onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
                  setConfirmPassword(e.target.value)
                }
                required
                autoComplete="new-password"
              />
            </div>

            {error && <ErrorMessage message={error} />}
            {success && <SuccessMessage message="Votre mot de passe a ete modifie avec succes." />}

            <Button type="submit" disabled={mutation.isPending} className="w-full">
              {mutation.isPending ? 'Modification...' : 'Modifier le mot de passe'}
            </Button>
          </form>
        </CardContent>
      </Card>

      <Card className="animate-fade-slide-up [animation-delay:100ms]">
        <CardHeader>
          <CardTitle>Mot de passe oublie</CardTitle>
          <CardDescription>
            Si vous ne vous souvenez plus de votre mot de passe actuel, utilisez le lien de
            recuperation.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <Button variant="outline" className="w-full justify-between" asChild>
            <a href="/forgot-password">
              Reinitialiser via email
              <Key className="size-4" data-icon="inline-end" />
            </a>
          </Button>
        </CardContent>
      </Card>
    </div>
  );
}
