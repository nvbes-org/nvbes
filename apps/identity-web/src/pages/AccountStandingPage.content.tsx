import { BadgeCheck, Shield, ShieldAlert, ShieldCheck, UserCircle } from 'lucide-react';

import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Separator } from '@/components/ui/separator';
import type { AccountMe } from '@/lib/account-context';
import { formatStandingDate, StandingRow } from './AccountStandingPage.row';

export function AccountStandingContent({ me }: { me: AccountMe }) {
  const user = me.user;

  return (
    <div className="flex animate-fade-slide-up flex-col gap-6 [animation-delay:0ms]">
      <div>
        <h1 className="font-heading text-xl font-semibold">Statut du compte</h1>
        <p className="mt-1 text-sm text-muted-foreground">
          Suivi du statut, de la verification et de la reputation du compte.
        </p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Verification du compte</CardTitle>
          <CardDescription>
            Statut de verification de votre identite et de votre compte.
          </CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-1">
          <StandingRow
            title="Email verifie"
            subtitle={user.email}
            badgeLabel={user.email_verified ? 'Verifie' : 'Non verifie'}
            badgeVariant={user.email_verified ? 'default' : 'outline'}
            icon={UserCircle}
          />

          <Separator className="my-1" />

          <StandingRow
            title="MFA activee"
            subtitle="Authentification multi-facteurs"
            badgeLabel={user.mfa_enabled ? 'Active' : 'Non active'}
            badgeVariant={user.mfa_enabled ? 'default' : 'outline'}
            icon={Shield}
          />

          <Separator className="my-1" />

          <StandingRow
            title="Membre depuis"
            subtitle={formatStandingDate(user.created_at, {
              year: 'numeric',
              month: 'long',
              day: 'numeric',
            })}
            badgeLabel={formatStandingDate(user.created_at, {
              year: 'numeric',
              month: 'short',
            })}
            badgeVariant="secondary"
            icon={BadgeCheck}
          />

          <Separator className="my-1" />

          <StandingRow
            title="Region"
            subtitle="Region de stockage des donnees"
            badgeLabel={user.region ?? 'Non definie'}
            badgeVariant="secondary"
            icon={ShieldCheck}
          />
        </CardContent>
      </Card>

      <Card className="animate-fade-slide-up [animation-delay:100ms]">
        <CardHeader>
          <CardTitle>Securite du compte</CardTitle>
          <CardDescription>
            Recommandations pour maintenir votre compte en bon etat.
          </CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-1">
          <StandingRow
            title="Authentification multi-facteurs"
            subtitle={
              user.mfa_enabled
                ? 'Votre compte est protege par MFA.'
                : 'Activez MFA pour securiser votre compte.'
            }
            badgeLabel={user.mfa_enabled ? 'Protege' : 'Recommande'}
            badgeVariant={user.mfa_enabled ? 'default' : 'outline'}
            icon={user.mfa_enabled ? ShieldCheck : ShieldAlert}
            iconClassName={user.mfa_enabled ? 'size-4 text-emerald-500' : 'size-4 text-amber-500'}
          />
        </CardContent>
      </Card>
    </div>
  );
}
