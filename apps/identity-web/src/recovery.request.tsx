import type { FormEvent } from 'react';
import { Button } from '@/components/ui/button';
import { Field, FieldGroup, FieldLabel, FieldDescription } from '@/components/ui/field';
import { Input } from '@/components/ui/input';
import type { AuthorizationController } from './authorization.controller';

export function RecoveryRequest({
  controller,
  busy,
}: {
  controller: AuthorizationController;
  busy: boolean;
}) {
  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const form = event.currentTarget;
    const code = new FormData(form).get('recovery-code');
    form.reset();
    if (typeof code === 'string') void controller.recover(code.trim());
  }
  return (
    <details className="text-sm">
      <summary className="cursor-pointer py-2">Vous avez perdu vos facteurs ?</summary>
      <form onSubmit={submit} className="pt-4">
        <FieldGroup>
          <Field>
            <FieldLabel htmlFor="recovery-code">Code de secours</FieldLabel>
            <Input
              id="recovery-code"
              name="recovery-code"
              type="password"
              autoComplete="off"
              pattern="nvr1_[A-Za-z0-9_\-]{43}"
              maxLength={48}
              required
              disabled={busy}
            />
            <FieldDescription>
              Utiliser ce code déconnecte toutes vos sessions et interrompt cette connexion. Vous
              devrez créer une nouvelle passkey, puis revenir à votre application pour vous
              reconnecter.
            </FieldDescription>
          </Field>
          <Button type="submit" disabled={busy}>
            Utiliser ce code de secours
          </Button>
        </FieldGroup>
      </form>
    </details>
  );
}
