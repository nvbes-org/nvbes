import { useEffect } from 'react';
import { Button } from '@/components/ui/button';
import type { AuthorizationController } from './authorization.controller';
import { managementExpiry, type AuthorizationState } from './authorization.state';

export function RecoveryCodes({
  controller,
  state,
}: {
  controller: AuthorizationController;
  state: AuthorizationState;
}) {
  const expiry = managementExpiry(state);
  useEffect(() => {
    const expire = () => controller.expireRecoveryCodes();
    const timer = window.setTimeout(expire, Math.max(0, Date.parse(expiry ?? '') - Date.now()));
    document.addEventListener('visibilitychange', expire);
    return () => {
      window.clearTimeout(timer);
      document.removeEventListener('visibilitychange', expire);
    };
  }, [controller, expiry]);
  return (
    <>
      <p className="text-sm leading-relaxed text-muted-foreground">
        Conservez ces codes dans un endroit sûr, par exemple votre gestionnaire de mots de passe.
        Chacun est utilisable une seule fois après connexion par mot de passe si vous perdez vos
        facteurs. Ils ne seront plus affichés une fois cet écran fermé. Les anciens codes ont été
        remplacés.
      </p>
      <ol aria-label="Codes de secours" className="flex flex-col gap-3 text-xs">
        {state.recoveryCodes?.map((code) => (
          <li key={code}>
            <code className="break-all select-all">{code}</code>
          </li>
        ))}
      </ol>
      <Button
        size="lg"
        disabled={state.busy}
        onClick={() => void controller.dismissRecoveryCodes()}
      >
        J’ai conservé mes codes
      </Button>
    </>
  );
}
