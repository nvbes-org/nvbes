import type { SubmitEvent } from 'react';
import { FeedbackAlert } from '@/components/FeedbackAlert';
import { Button } from '@/components/ui/button';
import { Field, FieldLabel } from '@/components/ui/field';
import { Input } from '@/components/ui/input';
import { Spinner } from '@/components/ui/spinner';

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
  onSubmit: (event: SubmitEvent<HTMLFormElement>) => void;
}) {
  return (
    <form onSubmit={onSubmit} className="flex flex-col gap-5">
      <Field>
        <FieldLabel htmlFor="login-email">Email</FieldLabel>
        <Input
          id="login-email"
          type="email"
          placeholder="vous@exemple.fr"
          value={email}
          onChange={(event) => onEmailChange(event.target.value)}
          required
          autoComplete="username webauthn"
          autoFocus
          className="h-11 rounded-xl px-3.5"
        />
      </Field>
      {error && <FeedbackAlert tone="error">{error}</FeedbackAlert>}
      <div className="flex flex-col gap-2">
        <Button
          type="submit"
          disabled={loading}
          className="h-11 self-end rounded-full px-6"
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
