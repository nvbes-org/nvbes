import type { FormEvent } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Spinner } from '@/components/ui/spinner';
import { LoginPageError } from './LoginPage.layout';

export function LoginPageIdentifierForm({
  email,
  error,
  loading,
  onEmailChange,
  onSubmit,
}: {
  email: string;
  error: string | null;
  loading: boolean;
  onEmailChange: (value: string) => void;
  onSubmit: (event: FormEvent<HTMLFormElement>) => void;
}) {
  return (
    <form onSubmit={onSubmit} className="flex flex-col gap-5">
      <div className="flex flex-col gap-2">
        <Label htmlFor="login-email">Email</Label>
        <Input
          id="login-email"
          type="email"
          placeholder="vous@exemple.fr"
          value={email}
          onChange={(event) => onEmailChange(event.target.value)}
          required
          autoComplete="email"
          autoFocus
        />
      </div>
      {error && <LoginPageError message={error} />}
      <div className="flex flex-col gap-2">
        <Button
          type="submit"
          disabled={loading}
          className="w-full gap-2"
          size="lg"
          aria-busy={loading}
        >
          {loading && <Spinner className="size-4" />}
          <span>{loading ? 'Préparation...' : 'Continuer'}</span>
        </Button>
      </div>
    </form>
  );
}
