import { MailIcon } from 'lucide-react';
import type { SubmitEvent } from 'react';
import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Spinner } from '@/components/ui/spinner';
import { LoginPageError } from './LoginPage.layout';

export function LoginPagePasswordForm({
  email,
  password,
  error,
  loading,
  onPasswordChange,
  onSubmit,
  onBack,
}: {
  email: string;
  password: string;
  error: string | null;
  loading: boolean;
  onPasswordChange: (value: string) => void;
  onSubmit: (event: SubmitEvent<HTMLFormElement>) => void;
  onBack: () => void;
}) {
  return (
    <form onSubmit={onSubmit} className="flex flex-col gap-5">
      <Card className="flex flex-row items-center gap-2 p-3">
        <MailIcon className="size-4 shrink-0 text-muted-foreground" />
        <span className="truncate text-sm font-medium">{email}</span>
      </Card>
      <div className="flex flex-col gap-2">
        <Label htmlFor="login-password">Mot de passe</Label>
        <Input
          type="email"
          name="username"
          autoComplete="username"
          value={email}
          readOnly
          tabIndex={-1}
          aria-hidden="true"
          className="sr-only"
        />
        <Input
          id="login-password"
          name="password"
          type="password"
          placeholder="••••••••"
          value={password}
          onChange={(event) => onPasswordChange(event.target.value)}
          required
          autoComplete="current-password"
          spellCheck={false}
          autoFocus
        />
      </div>
      <div className="flex justify-end">
        <a
          href="/forgot-password"
          className="text-xs text-muted-foreground transition-colors hover:text-foreground"
        >
          Mot de passe oublie ?
        </a>
      </div>
      {error && <LoginPageError message={error} />}
      <div className="flex gap-2">
        <Button
          type="button"
          variant="outline"
          className="flex-1"
          onClick={onBack}
          disabled={loading}
        >
          Retour
        </Button>
        <Button type="submit" disabled={loading} className="flex-1 gap-2" aria-busy={loading}>
          {loading && <Spinner className="size-4" />}
          <span>{loading ? 'Connexion...' : 'Se connecter'}</span>
        </Button>
      </div>
    </form>
  );
}
