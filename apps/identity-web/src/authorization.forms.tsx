import type { FormEvent } from 'react';
import { Fingerprint, ArrowRight } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Field, FieldGroup, FieldLabel, FieldSeparator } from '@/components/ui/field';
import { Input } from '@/components/ui/input';
import type { AuthorizationController } from './authorization.controller';
import type { AuthorizationState } from './authorization.state';
import { RecoveryRequest } from './recovery.request';

export function AuthenticationForms({
  controller,
  state,
}: {
  controller: AuthorizationController;
  state: AuthorizationState;
}) {
  function password(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const form = event.currentTarget;
    const data = new FormData(form);
    const email = data.get('email');
    const secret = data.get('password');
    form.reset();
    if (typeof email === 'string' && typeof secret === 'string')
      void controller.password(email, secret);
  }
  function totp(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const form = event.currentTarget;
    const code = new FormData(form).get('code');
    form.reset();
    if (typeof code === 'string') void controller.totp(code);
  }
  const login = state.stage === 'login';
  return (
    <div className="flex flex-col gap-6">
      <Button
        size="lg"
        variant="outline"
        disabled={state.busy}
        onClick={() => void controller.passkey()}
      >
        <Fingerprint data-icon="inline-start" />
        {login ? 'Se connecter avec une passkey' : 'Vérifier avec une passkey'}
      </Button>
      {login ? (
        <form onSubmit={password}>
          <FieldGroup>
            <FieldSeparator>ou avec votre mot de passe</FieldSeparator>
            <Field data-disabled={state.busy}>
              <FieldLabel htmlFor="email">Adresse email</FieldLabel>
              <Input
                id="email"
                name="email"
                type="email"
                autoComplete="username"
                maxLength={320}
                required
                disabled={state.busy}
              />
            </Field>
            <Field data-disabled={state.busy}>
              <FieldLabel htmlFor="password">Mot de passe</FieldLabel>
              <Input
                id="password"
                name="password"
                type="password"
                autoComplete="current-password"
                maxLength={1024}
                required
                disabled={state.busy}
              />
            </Field>
            <Button size="lg" type="submit" disabled={state.busy}>
              Continuer <ArrowRight data-icon="inline-end" />
            </Button>
          </FieldGroup>
        </form>
      ) : state.stage === 'security-step-up' ||
        state.authentication?.minimumAuthentication === 'recent_mfa' ? (
        <form onSubmit={totp}>
          <FieldGroup>
            <FieldSeparator>ou avec votre application d’authentification</FieldSeparator>
            <Field data-disabled={state.busy}>
              <FieldLabel htmlFor="code">Code à 6 chiffres</FieldLabel>
              <Input
                id="code"
                name="code"
                inputMode="numeric"
                autoComplete="one-time-code"
                pattern="[0-9]{6}"
                maxLength={6}
                required
                disabled={state.busy}
              />
            </Field>
            <Button size="lg" type="submit" disabled={state.busy}>
              Vérifier le code
            </Button>
          </FieldGroup>
        </form>
      ) : (
        <p className="text-sm leading-relaxed text-muted-foreground">
          Cette application demande une passkey ou une clé de sécurité avec vérification
          utilisateur.
        </p>
      )}
      {state.stage === 'step-up' && <RecoveryRequest controller={controller} busy={state.busy} />}
      {state.stage === 'security-step-up' && (
        <Button
          variant="ghost"
          disabled={state.busy}
          onClick={() => void controller.cancelSecurityStepUp()}
        >
          Revenir aux accès demandés
        </Button>
      )}
    </div>
  );
}
