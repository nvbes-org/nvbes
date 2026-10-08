import { useEffect, useState, useSyncExternalStore, type FormEvent } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Field, FieldLabel } from '@/components/ui/field';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { FactorsController, factorsGateway } from './factors.controller';
import type { AuthorizationController } from './authorization.controller';

export function FactorsPanel({
  authorization,
  csrf,
  expiresAt,
}: {
  authorization: AuthorizationController;
  csrf: string;
  expiresAt: string;
}) {
  const [controller] = useState(
    () =>
      new FactorsController(factorsGateway(location.origin), csrf, expiresAt, (revoked) =>
        authorization.closeFactors(revoked),
      ),
  );
  const state = useSyncExternalStore(controller.subscribe, controller.snapshot);
  const [removal, setRemoval] = useState<{
    kind: 'passkey' | 'totp';
    id: string;
    label: string;
  } | null>(null);
  useEffect(() => {
    void controller.start();
    const expire = () => controller.expire();
    const dispose = () => controller.dispose();
    const timer = window.setTimeout(expire, Math.max(0, Date.parse(expiresAt) - Date.now()));
    document.addEventListener('visibilitychange', expire);
    window.addEventListener('pagehide', dispose);
    return () => {
      clearTimeout(timer);
      document.removeEventListener('visibilitychange', expire);
      window.removeEventListener('pagehide', dispose);
      controller.dispose();
    };
  }, [controller, expiresAt]);
  function rename(event: FormEvent<HTMLFormElement>, id: string) {
    event.preventDefault();
    const label = new FormData(event.currentTarget).get('label');
    if (typeof label === 'string') void controller.rename(id, label);
  }
  const last = state.passkeys.length + state.totp.length <= 1;
  return (
    <div className="flex flex-col gap-6" aria-busy={state.busy}>
      {state.error && (
        <Alert variant="destructive">
          <AlertDescription>{state.error}</AlertDescription>
        </Alert>
      )}
      {removal ? (
        <>
          <h2 className="font-medium">Supprimer « {removal.label} » ?</h2>
          <p className="text-sm text-muted-foreground">
            {removal.kind === 'totp'
              ? 'Toutes vos sessions seront déconnectées.'
              : 'Les sessions ayant utilisé cette passkey seront déconnectées.'}{' '}
            Cette connexion sera interrompue. Vous devrez revenir à votre application et vous
            reconnecter avec une méthode conservée.
          </p>
          <Button
            variant="destructive"
            disabled={state.busy}
            onClick={() => void controller.revoke(removal.kind, removal.id)}
          >
            Confirmer la suppression
          </Button>
          <Button variant="ghost" disabled={state.busy} onClick={() => setRemoval(null)}>
            Conserver ce facteur
          </Button>
        </>
      ) : (
        <>
          <h2 className="font-medium">Vos méthodes de sécurité</h2>
          {state.passkeys.map((factor) => (
            <section
              key={factor.id}
              aria-label={factor.label}
              className="flex flex-col gap-3 border-b pb-4"
            >
              <form onSubmit={(event) => rename(event, factor.id)} className="flex flex-col gap-3">
                <Field>
                  <FieldLabel htmlFor={factor.id}>Nom de la passkey</FieldLabel>
                  <Input
                    id={factor.id}
                    name="label"
                    defaultValue={factor.label}
                    maxLength={128}
                    required
                    disabled={state.busy}
                  />
                </Field>
                <Button type="submit" variant="outline" disabled={state.busy}>
                  Renommer
                </Button>
              </form>
              <Button
                variant="ghost"
                disabled={state.busy || last}
                onClick={() => setRemoval({ kind: 'passkey', id: factor.id, label: factor.label })}
              >
                Supprimer cette passkey
              </Button>
            </section>
          ))}
          {state.totp.map((factor) => (
            <section
              key={factor.id}
              aria-label="Application d’authentification"
              className="flex flex-col gap-3 border-b pb-4"
            >
              <p className="text-sm">Application d’authentification (TOTP)</p>
              <Button
                variant="ghost"
                disabled={state.busy || last}
                onClick={() =>
                  setRemoval({
                    kind: 'totp',
                    id: factor.id,
                    label: 'Application d’authentification',
                  })
                }
              >
                Supprimer TOTP
              </Button>
            </section>
          ))}
          {state.loaded && last && (
            <p className="text-sm text-muted-foreground">
              Ajoutez une autre méthode avant de supprimer la dernière.
            </p>
          )}
        </>
      )}
      <Button
        variant="outline"
        disabled={state.busy}
        onClick={() => void authorization.endFactors()}
      >
        Revenir aux accès demandés
      </Button>
      <p role="status" className="text-sm text-muted-foreground">
        {state.busy ? 'Vérification en cours…' : (state.notice ?? '')}
      </p>
    </div>
  );
}
