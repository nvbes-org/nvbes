import { useEffect, useSyncExternalStore, type FormEvent } from 'react';
import { Button } from '@/components/ui/button';
import { Field, FieldGroup, FieldLabel } from '@/components/ui/field';
import { Input } from '@/components/ui/input';
import { Alert, AlertDescription } from '@/components/ui/alert';
import type { RecoveryController } from './recovery.controller';

const titles = {
  loading: 'Préparation de la récupération',
  ready: 'Remplacez votre facteur perdu.',
  complete: 'Votre nouvelle passkey est prête.',
  cancelled: 'Récupération annulée.',
  closed: 'Récupération interrompue.',
};

export function RecoveryPage({ controller }: { controller: RecoveryController }) {
  const state = useSyncExternalStore(controller.subscribe, controller.snapshot);
  useEffect(() => {
    if (!state.expiresAt) return;
    const expire = () => controller.expire();
    const timer = window.setTimeout(expire, Math.max(0, Date.parse(state.expiresAt) - Date.now()));
    document.addEventListener('visibilitychange', expire);
    return () => {
      window.clearTimeout(timer);
      document.removeEventListener('visibilitychange', expire);
    };
  }, [controller, state.expiresAt]);
  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const label = new FormData(event.currentTarget).get('label');
    if (typeof label === 'string') void controller.complete(label);
  }
  return (
    <main
      className="mx-auto flex min-h-svh max-w-xl flex-col justify-center gap-6 px-6 py-12"
      aria-busy={state.busy}
    >
      <span className="text-2xl font-semibold tracking-tighter">nvbes.</span>
      <h1 className="font-heading text-4xl tracking-tight">{titles[state.stage]}</h1>
      {state.error && (
        <Alert variant="destructive">
          <AlertDescription>{state.error}</AlertDescription>
        </Alert>
      )}
      {state.stage === 'ready' ? (
        <>
          <p className="text-sm leading-relaxed text-muted-foreground">
            Le code de secours a été consommé et vos sessions ont été déconnectées. Créez une
            passkey de remplacement. La confirmation révoquera vos anciens facteurs et les codes de
            secours restants. Une nouvelle connexion sera nécessaire.
          </p>
          <form onSubmit={submit}>
            <FieldGroup>
              <Field>
                <FieldLabel htmlFor="replacement-label">Nom de la nouvelle passkey</FieldLabel>
                <Input
                  id="replacement-label"
                  name="label"
                  defaultValue="Ma nouvelle passkey"
                  maxLength={128}
                  required
                  disabled={state.busy}
                />
              </Field>
              <Button size="lg" type="submit" disabled={state.busy}>
                Créer la passkey de remplacement
              </Button>
            </FieldGroup>
          </form>
          <Button variant="ghost" disabled={state.busy} onClick={() => void controller.cancel()}>
            Annuler la récupération
          </Button>
        </>
      ) : (
        state.stage !== 'loading' && (
          <p className="leading-relaxed text-muted-foreground">
            {state.stage === 'complete'
              ? 'Revenez à votre application et démarrez une nouvelle connexion avec cette passkey. Pensez ensuite à générer de nouveaux codes de secours.'
              : 'Revenez à votre application pour démarrer une nouvelle connexion. Un code déjà utilisé ne peut pas être réutilisé ; annuler ne restaure pas les sessions déconnectées.'}
          </p>
        )
      )}
      <p role="status" aria-live="polite" className="text-sm text-muted-foreground">
        {state.busy ? 'Vérification en cours…' : ''}
      </p>
    </main>
  );
}
