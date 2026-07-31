import { Link } from '@tanstack/react-router';
import { MailIcon } from 'lucide-react';
import type { SubmitEvent } from 'react';
import { FeedbackAlert } from '@/components/FeedbackAlert';
import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import { Field, FieldLabel } from '@/components/ui/field';
import { Input } from '@/components/ui/input';
import { Spinner } from '@/components/ui/spinner';

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
      <Field>
        <FieldLabel htmlFor="login-password">Mot de passe</FieldLabel>
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
          className="h-11 rounded-xl px-3.5"
        />
      </Field>
      <div className="flex justify-end">
        <Link
          to="/forgot-password"
          className="text-xs text-muted-foreground transition-colors hover:text-foreground"
        >
          Mot de passe oublie ?
        </Link>
      </div>
      {error && <FeedbackAlert tone="error">{error}</FeedbackAlert>}
      <div className="flex justify-end gap-2">
        <Button
          type="button"
          variant="outline"
          className="h-11 rounded-full px-5"
          onClick={onBack}
          disabled={loading}
        >
          Retour
        </Button>
        <Button
          type="submit"
          disabled={loading}
          className="h-11 rounded-full px-5"
          aria-busy={loading}
        >
          {loading && <Spinner className="size-4" />}
          <span>{loading ? 'Connexion...' : 'Se connecter'}</span>
        </Button>
      </div>
    </form>
  );
}
