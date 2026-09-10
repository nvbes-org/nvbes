import { useState, type FormEvent } from 'react';
import { MailIcon } from 'lucide-react';
import { Button } from './components/ui/button';
import { Field, FieldLabel } from './components/ui/field';
import { Input } from './components/ui/input';
import type { AuthorizationController } from './authorization.controller';

export function PasswordLogin({
  controller,
  busy,
  onStep,
}: {
  controller: AuthorizationController;
  busy: boolean;
  onStep: (step: 'identifier' | 'password') => void;
}) {
  const [email, setEmail] = useState('');
  const [identified, setIdentified] = useState(false);
  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!identified) {
      setIdentified(true);
      onStep('password');
      return;
    }
    const form = event.currentTarget;
    const secret = new FormData(form).get('password');
    form.reset();
    setIdentified(false);
    onStep('identifier');
    if (typeof secret === 'string') void controller.password(email, secret);
  }
  return (
    <form onSubmit={submit} className="flex flex-col gap-5">
      {identified ? (
        <>
          <div className="flex items-center gap-2 rounded-xl border p-3">
            <MailIcon className="size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
            <span className="truncate text-sm font-medium">{email}</span>
          </div>
          <input type="hidden" name="username" autoComplete="username" value={email} />
          <Field>
            <FieldLabel htmlFor="password">Mot de passe</FieldLabel>
            <Input
              key="password"
              id="password"
              name="password"
              type="password"
              placeholder="••••••••"
              autoComplete="current-password"
              spellCheck={false}
              maxLength={1024}
              required
              disabled={busy}
              autoFocus
              className="h-11 rounded-xl px-3.5"
            />
          </Field>
          <div className="flex justify-end">
            <a
              href="/password-recovery"
              className="text-xs text-muted-foreground transition-colors hover:text-foreground"
            >
              Mot de passe oublié ?
            </a>
          </div>
        </>
      ) : (
        <Field>
          <FieldLabel htmlFor="email">Adresse email</FieldLabel>
          <Input
            key="email"
            id="email"
            name="email"
            type="email"
            placeholder="vous@exemple.fr"
            autoComplete="username webauthn"
            value={email}
            onChange={(event) => setEmail(event.target.value)}
            maxLength={320}
            required
            disabled={busy}
            autoFocus
            className="h-11 rounded-xl px-3.5"
          />
        </Field>
      )}
      <div className="flex justify-end gap-2">
        {identified && (
          <Button
            type="button"
            variant="outline"
            disabled={busy}
            className="h-11 rounded-full px-5"
            onClick={() => {
              setIdentified(false);
              onStep('identifier');
            }}
          >
            Retour
          </Button>
        )}
        <Button type="submit" disabled={busy} className="h-11 rounded-full px-6" size="lg">
          {identified ? 'Se connecter' : 'Continuer'}
        </Button>
      </div>
    </form>
  );
}
