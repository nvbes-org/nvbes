import { KeyRound } from 'lucide-react';

import { Card, CardContent } from '@/components/ui/card';
import {
  Item,
  ItemActions,
  ItemContent,
  ItemDescription,
  ItemMedia,
  ItemTitle,
} from '@/components/ui/item';
import { Switch } from '@/components/ui/switch';
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
      <CardContent className="flex flex-col gap-4">
        <Item className="items-start px-0 py-0">
          <ItemMedia variant="icon" className="size-8 rounded-lg bg-muted">
            <KeyRound className="size-4 text-muted-foreground" />
          </ItemMedia>
          <ItemContent className="min-w-0 gap-0.5">
            <ItemTitle>Passer le mot de passe si possible</ItemTitle>
            <ItemDescription className="text-xs leading-relaxed">
              Permet de se connecter directement avec une passkey sans saisir de mot de passe.
            </ItemDescription>
            {!overview.has_passkey ? (
              <p className="mt-1 text-xs font-medium text-amber-500">
                Ajoutez d&apos;abord une passkey dans la double authentification pour activer cette
                option.
              </p>
            ) : null}
          </ItemContent>
          <ItemActions className="shrink-0">
            <Switch
              id="skip-password-toggle"
              aria-label="Passer le mot de passe si possible"
              disabled={!overview.has_passkey || pending}
              checked={overview.skip_password}
              onCheckedChange={onSkipPasswordChange}
            />
          </ItemActions>
        </Item>
      </CardContent>
    </Card>
  );
}
