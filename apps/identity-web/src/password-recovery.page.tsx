import { useSyncExternalStore, type FormEvent } from 'react';
import { AuthShell } from './auth.shell';
import { Button } from './components/ui/button';
import { Field, FieldLabel } from './components/ui/field';
import { Input } from './components/ui/input';
import { Alert, AlertDescription } from './components/ui/alert';
import type { PasswordRecoveryController } from './password-recovery.controller';

export function PasswordRecoveryPage({ controller }: { controller: PasswordRecoveryController }) {
  const state = useSyncExternalStore(controller.subscribe, controller.snapshot);
  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const form = event.currentTarget;
    const data = new FormData(form);
    form.reset();
    if (state.stage === 'request') {
      const email = data.get('email');
      if (typeof email === 'string') void controller.request(email);
    } else {
      const password = data.get('password');
      const confirmation = data.get('confirmation');
      if (typeof password === 'string' && typeof confirmation === 'string')
        void controller.reset(password, confirmation);
    }
  }
  return (
    <AuthShell
      title={
        state.stage === 'complete'
          ? 'Mot de passe modifié'
          : state.stage === 'reset'
            ? 'Nouveau mot de passe'
            : 'Mot de passe oublié'
      }
      description="Retrouvez l’accès à votre compte nvbes."
    >
      {state.error && (
        <Alert variant="destructive">
          <AlertDescription>{state.error}</AlertDescription>
        </Alert>
      )}
      {(state.stage === 'request' || state.stage === 'reset') && (
        <form onSubmit={submit} className="flex flex-col gap-5" aria-busy={state.busy}>
          {state.stage === 'request' ? (
            <Field>
              <FieldLabel htmlFor="recovery-email">Adresse email</FieldLabel>
              <Input
                id="recovery-email"
                name="email"
                type="email"
                autoComplete="email"
                maxLength={320}
                required
                disabled={state.busy}
                className="h-11 rounded-xl px-3.5"
              />
            </Field>
          ) : (
            <>
              <Field>
                <FieldLabel htmlFor="new-password">Nouveau mot de passe</FieldLabel>
                <Input
                  id="new-password"
                  name="password"
                  type="password"
                  autoComplete="new-password"
                  minLength={12}
                  maxLength={1024}
                  required
                  disabled={state.busy}
                  className="h-11 rounded-xl px-3.5"
                />
              </Field>
              <Field>
                <FieldLabel htmlFor="confirm-password">Confirmer le mot de passe</FieldLabel>
                <Input
                  id="confirm-password"
                  name="confirmation"
                  type="password"
                  autoComplete="new-password"
                  minLength={12}
                  maxLength={1024}
                  required
                  disabled={state.busy}
                  className="h-11 rounded-xl px-3.5"
                />
              </Field>
              <p className="text-sm text-muted-foreground">
                Choisissez un nouveau mot de passe d’au moins 12 caractères. Vos sessions seront
                fermées. Vous devrez ensuite vous reconnecter à votre application.
              </p>
            </>
          )}
          <Button type="submit" disabled={state.busy} className="h-11 self-end rounded-full px-6">
            {state.stage === 'request' ? 'Envoyer le lien' : 'Changer le mot de passe'}
          </Button>
        </form>
      )}
      {state.stage === 'sent' && (
        <p role="status">
          Si cette adresse correspond à un compte éligible, un email de récupération sera envoyé. Le
          lien expire après 15 minutes.
        </p>
      )}
      {state.stage === 'complete' && (
        <p role="status">
          Revenez à votre application et reconnectez-vous avec votre nouveau mot de passe. Les
          anciens liens de récupération ne sont plus utilisables.
        </p>
      )}
      {state.stage === 'closed' && !state.error && (
        <p role="status">Cette page a été fermée. Revenez à votre application.</p>
      )}
      {(state.stage === 'loading' || state.busy) && <p role="status">Vérification en cours…</p>}
    </AuthShell>
  );
}
