import { AuthShell } from './auth.shell';
import { useSyncExternalStore } from 'react';
import { Button } from './components/ui/button';
import type { LogoutController } from './logout.controller';

export function LogoutPage({ controller }: { controller: LogoutController }) {
  const { stage } = useSyncExternalStore(controller.subscribe, controller.snapshot);
  return (
    <AuthShell title={stage === 'complete' ? 'Vous êtes déconnecté.' : 'Déconnexion Identity'}>
      {stage === 'confirm' && (
        <>
          <p className="leading-relaxed text-muted-foreground">
            Confirmez la fermeture de votre session Identity dans ce navigateur. Les accès aux
            applications liés à cette session seront révoqués. Vos autres sessions sur d’autres
            appareils restent ouvertes.
          </p>
          <div className="flex flex-wrap gap-3">
            <Button onClick={() => void controller.confirm()}>Confirmer la déconnexion</Button>
            <Button variant="outline" onClick={() => controller.cancel()}>
              Rester connecté
            </Button>
          </div>
        </>
      )}
      {stage === 'loading' && <p role="status">Vérification de la session…</p>}
      {stage === 'leaving' && <p role="status">Fermeture de la session…</p>}
      {stage === 'complete' && (
        <p role="status">
          La session Identity de ce navigateur a été révoquée. Vous pouvez fermer cette page.
        </p>
      )}
      {stage === 'absent' && (
        <p role="status">Aucune session Identity à fermer n’a été trouvée dans ce navigateur.</p>
      )}
      {stage === 'cancelled' && (
        <p role="status">
          La déconnexion a été annulée. Vous pouvez retourner à votre application.
        </p>
      )}
      {stage === 'failed' && (
        <p role="alert">
          La déconnexion n’a pas pu être confirmée. Rechargez cette page pour vérifier la session.
        </p>
      )}
    </AuthShell>
  );
}
