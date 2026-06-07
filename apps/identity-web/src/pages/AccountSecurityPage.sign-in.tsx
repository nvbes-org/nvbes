import { KeyRound } from 'lucide-react';

import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import type { SecurityOverview } from './useAccountSecurityPage';

export function SecuritySignInOptionsCard({
  overview,
  pending,
  onSkipPasswordChange,
}: {
  overview: SecurityOverview;
  pending: boolean;
  onSkipPasswordChange: (checked: boolean) => void;
}) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>Options de connexion</CardTitle>
        <CardDescription>Personnalisez la facon dont vous vous authentifiez.</CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-4">
        <div className="flex items-start justify-between gap-3">
          <div className="flex min-w-0 items-start gap-3">
            <div className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-muted">
              <KeyRound className="size-4 text-muted-foreground" />
            </div>
            <div className="flex min-w-0 flex-col">
              <span className="text-sm font-medium">Passer le mot de passe si possible</span>
              <span className="mt-0.5 text-xs leading-relaxed text-muted-foreground">
                Permet de se connecter directement avec une passkey ou une cle de securite sans
                saisir de mot de passe.
              </span>
              {!overview.has_webauthn ? (
                <span className="mt-1 text-xs font-medium text-amber-500">
                  Ajoutez d&apos;abord une passkey ou une cle de securite dans la double
                  authentification pour activer cette option.
                </span>
              ) : null}
            </div>
          </div>
          <div className="flex shrink-0 items-center">
            <input
              id="skip-password-toggle"
              type="checkbox"
              className="size-4 cursor-pointer rounded border-border text-primary focus:ring-ring disabled:cursor-not-allowed"
              disabled={!overview.has_webauthn || pending}
              checked={overview.skip_password}
              onChange={(event) => onSkipPasswordChange(event.target.checked)}
            />
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
