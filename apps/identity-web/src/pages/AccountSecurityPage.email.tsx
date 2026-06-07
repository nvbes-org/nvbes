import { ShieldCheck } from 'lucide-react';

import { Badge } from '@/components/ui/badge';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import type { SecurityOverview } from './useAccountSecurityPage';

export function SecurityEmailVerificationCard({ overview }: { overview: SecurityOverview }) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>Verification de l&apos;email</CardTitle>
        <CardDescription>
          Une adresse email verifiee est requise pour acceder aux fonctionnalites avancees.
        </CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-3">
        <div className="flex items-center justify-between gap-3">
          <div className="flex min-w-0 items-center gap-3">
            <div className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-muted">
              <ShieldCheck
                className={
                  overview.email_verified ? 'size-4 text-primary' : 'size-4 text-muted-foreground'
                }
              />
            </div>
            <div className="flex flex-col">
              <span className="text-sm font-medium">Statut de verification</span>
              <span className="text-xs text-muted-foreground">
                {overview.email_verified ? 'Votre email est verifie.' : 'Email non verifie.'}
              </span>
            </div>
          </div>
          <Badge variant={overview.email_verified ? 'default' : 'secondary'}>
            {overview.email_verified ? 'Verifie' : 'En attente'}
          </Badge>
        </div>
      </CardContent>
    </Card>
  );
}
