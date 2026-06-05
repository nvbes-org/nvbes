import { BadgeCheck, Shield, ShieldAlert, ShieldCheck, UserCircle } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Separator } from '@/components/ui/separator';
import { Skeleton } from '@/components/ui/skeleton';
import { useAccountContext } from '@/hooks/useAccountContext';

function StandingSkeleton() {
  return (
    <div className="flex flex-col gap-6">
      <div>
        <Skeleton className="h-6 w-40" />
        <Skeleton className="h-4 w-72 mt-1" />
      </div>
      <Card>
        <CardHeader>
          <Skeleton className="h-5 w-32" />
          <Skeleton className="h-4 w-48" />
        </CardHeader>
        <CardContent className="flex flex-col gap-3">
          {Array.from({ length: 4 }).map((_, i) => (
            <Skeleton key={i} className="h-12 w-full rounded-lg" />
          ))}
        </CardContent>
      </Card>
    </div>
  );
}

export default function AccountStandingPage() {
  const { me } = useAccountContext();

  if (!me) return <StandingSkeleton />;

  const user = me.user;

  return (
    <div className="flex flex-col gap-6 animate-fade-slide-up [animation-delay:0ms]">
      <div>
        <h1 className="text-xl font-heading font-semibold">Statut du compte</h1>
        <p className="text-sm text-muted-foreground mt-1">
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
          <div className="flex items-center justify-between gap-3 py-1">
            <div className="flex items-center gap-3 min-w-0">
              <div className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-muted">
                <UserCircle className="size-4 text-muted-foreground" />
              </div>
              <div className="flex flex-col min-w-0">
                <span className="text-sm font-medium">Email verifie</span>
                <span className="text-xs text-muted-foreground">{user.email}</span>
              </div>
            </div>
            <Badge variant={user.email_verified ? 'default' : 'outline'}>
              {user.email_verified ? 'Verifie' : 'Non verifie'}
            </Badge>
          </div>

          <Separator className="my-1" />

          <div className="flex items-center justify-between gap-3 py-1">
            <div className="flex items-center gap-3 min-w-0">
              <div className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-muted">
                <Shield className="size-4 text-muted-foreground" />
              </div>
              <div className="flex flex-col min-w-0">
                <span className="text-sm font-medium">MFA activee</span>
                <span className="text-xs text-muted-foreground">
                  Authentification multi-facteurs
                </span>
              </div>
            </div>
            <Badge variant={user.mfa_enabled ? 'default' : 'outline'}>
              {user.mfa_enabled ? 'Active' : 'Non active'}
            </Badge>
          </div>

          <Separator className="my-1" />

          <div className="flex items-center justify-between gap-3 py-1">
            <div className="flex items-center gap-3 min-w-0">
              <div className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-muted">
                <BadgeCheck className="size-4 text-muted-foreground" />
              </div>
              <div className="flex flex-col min-w-0">
                <span className="text-sm font-medium">Membre depuis</span>
                <span className="text-xs text-muted-foreground">
                  {new Date(user.created_at).toLocaleDateString('fr-FR', {
                    year: 'numeric',
                    month: 'long',
                    day: 'numeric',
                  })}
                </span>
              </div>
            </div>
            <Badge variant="secondary">
              {new Date(user.created_at).toLocaleDateString('fr-FR', {
                year: 'numeric',
                month: 'short',
              })}
            </Badge>
          </div>

          <Separator className="my-1" />

          <div className="flex items-center justify-between gap-3 py-1">
            <div className="flex items-center gap-3 min-w-0">
              <div className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-muted">
                <ShieldCheck className="size-4 text-muted-foreground" />
              </div>
              <div className="flex flex-col min-w-0">
                <span className="text-sm font-medium">Region</span>
                <span className="text-xs text-muted-foreground">
                  Region de stockage des donnees
                </span>
              </div>
            </div>
            <Badge variant="secondary">{user.region ?? 'Non definie'}</Badge>
          </div>
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
          <div className="flex items-center justify-between gap-3 py-1">
            <div className="flex items-center gap-3 min-w-0">
              <div className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-muted">
                {user.mfa_enabled ? (
                  <ShieldCheck className="size-4 text-emerald-500" />
                ) : (
                  <ShieldAlert className="size-4 text-amber-500" />
                )}
              </div>
              <div className="flex flex-col min-w-0">
                <span className="text-sm font-medium">Authentification multi-facteurs</span>
                <span className="text-xs text-muted-foreground">
                  {user.mfa_enabled
                    ? 'Votre compte est protege par MFA.'
                    : 'Activez MFA pour securiser votre compte.'}
                </span>
              </div>
            </div>
            <Badge variant={user.mfa_enabled ? 'default' : 'outline'}>
              {user.mfa_enabled ? 'Protege' : 'Recommande'}
            </Badge>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
