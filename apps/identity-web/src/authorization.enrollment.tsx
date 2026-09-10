import { useEffect, type FormEvent } from 'react';
import { Fingerprint } from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  Field,
  FieldDescription,
  FieldGroup,
  FieldLabel,
  FieldSeparator,
} from '@/components/ui/field';
import { Input } from '@/components/ui/input';
import type { AuthorizationController } from './authorization.controller';
import type { AuthorizationState } from './authorization.state';

export function EnrollmentForm({
  controller,
  state,
}: {
  controller: AuthorizationController;
  state: AuthorizationState;
}) {
  const enrollment = state.totpEnrollment;
  useEffect(() => {
    if (!enrollment) return;
    const expire = () => controller.expireTotp();
    const timer = window.setTimeout(
      expire,
      Math.max(0, Date.parse(enrollment.expiresAt) - Date.now()),
    );
    document.addEventListener('visibilitychange', expire);
    return () => {
      window.clearTimeout(timer);
      document.removeEventListener('visibilitychange', expire);
    };
  }, [controller, enrollment]);

  function register(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const label = new FormData(event.currentTarget).get('label');
    if (typeof label === 'string') void controller.registerPasskey(label);
  }
  function confirm(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const form = event.currentTarget;
    const code = new FormData(form).get('code');
    form.reset();
    if (typeof code === 'string') void controller.confirmTotp(code);
  }

  return (
    <div className="flex flex-col gap-6">
      {enrollment ? (
        <form onSubmit={confirm}>
          <FieldGroup>
            <p className="text-sm leading-relaxed text-muted-foreground">
              Dans votre application d’authentification, ajoutez un compte nvbes avec une clé de
              configuration, puis saisissez le code affiché.
            </p>
            <Field>
              <FieldLabel htmlFor="setup-key">Clé de configuration</FieldLabel>
              <Input
                id="setup-key"
                value={enrollment.secretBase32}
                readOnly
                autoComplete="off"
                spellCheck={false}
              />
              <FieldDescription>
                Code basé sur le temps : 30 secondes, 6 chiffres. Gardez cette clé privée.
              </FieldDescription>
            </Field>
            <Field data-disabled={state.busy}>
              <FieldLabel htmlFor="enrollment-code">Code de confirmation</FieldLabel>
              <Input
                id="enrollment-code"
                name="code"
                inputMode="numeric"
                autoComplete="one-time-code"
                pattern="[0-9]{6}"
                maxLength={6}
                required
                disabled={state.busy}
              />
            </Field>
            <Button type="submit" size="lg" disabled={state.busy}>
              Confirmer l’application
            </Button>
            <Button
              type="button"
              variant="ghost"
              disabled={state.busy}
              onClick={() => void controller.cancelEnrollment()}
            >
              Autre méthode
            </Button>
          </FieldGroup>
        </form>
      ) : (
        <>
          <p className="text-sm leading-relaxed text-muted-foreground">
            Ajoutez une méthode de sécurité. Une passkey utilise le déverrouillage de votre appareil
            ou une clé de sécurité compatible.
          </p>
          <form onSubmit={register}>
            <FieldGroup>
              <Field data-disabled={state.busy}>
                <FieldLabel htmlFor="passkey-label">Nom de la passkey</FieldLabel>
                <Input
                  id="passkey-label"
                  name="label"
                  defaultValue="Ma passkey"
                  maxLength={128}
                  required
                  disabled={state.busy}
                />
              </Field>
              <Button size="lg" type="submit" disabled={state.busy}>
                <Fingerprint data-icon="inline-start" />
                Créer une passkey
              </Button>
              <FieldDescription>
                Confirmez sa création, puis son utilisation dans les fenêtres de votre navigateur.
              </FieldDescription>
            </FieldGroup>
          </form>
          {!state.hasTotp &&
            !(
              state.firstEnrollmentAvailable &&
              state.authentication?.minimumAuthentication === 'recent_webauthn'
            ) && (
              <>
                <FieldSeparator>ou</FieldSeparator>
                <Button
                  size="lg"
                  variant="outline"
                  disabled={state.busy}
                  onClick={() => void controller.startTotp()}
                >
                  Configurer une application d’authentification
                </Button>
              </>
            )}
          {!state.authentication?.needsStepUp && (
            <Button
              variant="ghost"
              disabled={state.busy}
              onClick={() => void controller.cancelEnrollment()}
            >
              Revenir aux accès demandés
            </Button>
          )}
        </>
      )}
    </div>
  );
}
