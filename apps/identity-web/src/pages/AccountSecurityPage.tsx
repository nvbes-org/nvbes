import { identityClient } from '@nvbes/identity-client';
import { listMfaFactors } from '@nvbes/identity-sdk-web';
import { useMutation, useQueryClient, useSuspenseQuery } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { ChevronRight, Key, Monitor, ShieldCheck, ShieldOff, KeyRound } from 'lucide-react';
import { useTransition } from 'react';
import { z } from 'zod';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Separator } from '@/components/ui/separator';
import { identityHttpClient } from '../identity.http';

const SecurityPreferencesSchema = z.object({
  theme: z.string(),
  language: z.string(),
  skip_password: z.boolean().default(false),
});

interface SecurityOverview {
  mfa_enabled: boolean;
  email_verified: boolean;
  session_count: number;
  has_webauthn: boolean;
  skip_password: boolean;
  raw_prefs: {
    theme: string;
    language: string;
    skip_password: boolean;
  } | null;
}

function SecurityRow({
  icon: Icon,
  iconColor,
  label,
  description,
  badgeLabel,
  badgeVariant,
  onAction,
  disabled,
}: {
  icon: React.ComponentType<{ className?: string }>;
  iconColor?: string;
  label: string;
  description: string;
  badgeLabel: string;
  badgeVariant: 'default' | 'secondary';
  onAction: () => void;
  disabled?: boolean;
}) {
  return (
    <div className="flex items-center justify-between gap-3">
      <div className="flex items-center gap-3 min-w-0">
        <div className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-muted">
          <Icon className={iconColor ?? 'size-4 text-muted-foreground'} />
        </div>
        <div className="flex flex-col min-w-0">
          <span className="text-sm font-medium">{label}</span>
          <span className="text-xs text-muted-foreground truncate">{description}</span>
        </div>
      </div>
      <div className="flex items-center gap-2 shrink-0">
        <Badge variant={badgeVariant} className="shrink-0">
          {badgeLabel}
        </Badge>
        <Button variant="ghost" size="icon-sm" onClick={onAction} disabled={disabled}>
          <ChevronRight className="size-4" />
        </Button>
      </div>
    </div>
  );
}

export default function AccountSecurityPage() {
  const navigate = useNavigate();
  const [isPending, startTransition] = useTransition();
  const queryClient = useQueryClient();

  const { data: overview } = useSuspenseQuery({
    queryKey: ['account', 'security-overview'],
    queryFn: async ({ signal }) => {
      const [meResult, sessionsResult, factorsResult, prefsResult] = await Promise.allSettled([
        identityClient.getMe({ signal }),
        identityClient.listSessions({ signal }),
        listMfaFactors(''),
        identityHttpClient.request('/auth/me/preferences', SecurityPreferencesSchema, { signal }),
      ]);

      const me = meResult.status === 'fulfilled' ? meResult.value : null;
      const sessions = sessionsResult.status === 'fulfilled' ? sessionsResult.value : [];
      const factors = factorsResult.status === 'fulfilled' ? factorsResult.value.factors : [];
      const prefs = prefsResult.status === 'fulfilled' ? prefsResult.value : null;

      return {
        mfa_enabled: me?.user.mfa_enabled ?? false,
        email_verified: me?.user.email_verified ?? false,
        session_count: sessions.length,
        has_webauthn: factors.some((f) => f.factor_type === 'webauthn'),
        skip_password: prefs?.skip_password ?? false,
        raw_prefs: prefs
          ? {
              theme: prefs.theme,
              language: prefs.language,
              skip_password: prefs.skip_password ?? false,
            }
          : null,
      } satisfies SecurityOverview;
    },
    staleTime: 0,
    gcTime: 30 * 60 * 1000,
    refetchOnWindowFocus: true,
  });

  const mutation = useMutation({
    mutationFn: async (newSkipPassword: boolean) => {
      if (!overview.raw_prefs) return;
      await identityHttpClient.request('/auth/me/preferences', SecurityPreferencesSchema, {
        method: 'PUT',
        body: {
          ...overview.raw_prefs,
          skip_password: newSkipPassword,
        },
      });
    },
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ['account', 'security-overview'] });
    },
  });

  return (
    <div className="flex flex-col gap-6 animate-fade-slide-up [animation-delay:0ms]">
      <div>
        <h1 className="text-xl font-heading font-semibold">Securite</h1>
        <p className="text-sm text-muted-foreground mt-1">
          Gerer l&apos;authentification et la securite de votre compte.
        </p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Authentification multi-facteurs</CardTitle>
          <CardDescription>
            Ajoutez une couche de securite supplementaire a votre compte.
          </CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-3">
          <SecurityRow
            icon={overview.mfa_enabled ? ShieldCheck : ShieldOff}
            iconColor={
              overview.mfa_enabled ? 'size-4 text-primary' : 'size-4 text-muted-foreground'
            }
            label="Double authentification (MFA)"
            description={overview.mfa_enabled ? 'Configurer et activee' : 'Non configuree'}
            badgeLabel={overview.mfa_enabled ? 'Active' : 'Inactive'}
            badgeVariant={overview.mfa_enabled ? 'default' : 'secondary'}
            onAction={() => startTransition(() => void navigate({ to: '/account/mfa' }))}
            disabled={isPending}
          />

          <Separator />

          <SecurityRow
            icon={Key}
            label="Mot de passe"
            description="Modifier votre mot de passe"
            badgeLabel="Defini"
            badgeVariant="default"
            onAction={() =>
              startTransition(() => void navigate({ to: '/account/security/password' }))
            }
            disabled={isPending}
          />

          <Separator />

          <SecurityRow
            icon={Monitor}
            label="Sessions actives"
            description={`${overview.session_count} session(s) ouverte(s)`}
            badgeLabel={overview.session_count.toString()}
            badgeVariant="default"
            onAction={() => startTransition(() => void navigate({ to: '/account/sessions' }))}
            disabled={isPending}
          />
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Options de connexion</CardTitle>
          <CardDescription>Personnalisez la facon dont vous vous authentifiez.</CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-4">
          <div className="flex items-start justify-between gap-3">
            <div className="flex items-start gap-3 min-w-0">
              <div className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-muted">
                <KeyRound className="size-4 text-muted-foreground" />
              </div>
              <div className="flex flex-col min-w-0">
                <span className="text-sm font-medium">Passer le mot de passe si possible</span>
                <span className="text-xs text-muted-foreground leading-relaxed mt-0.5">
                  Permet de se connecter directement avec une passkey ou une cle de securite sans
                  saisir de mot de passe.
                </span>
                {!overview.has_webauthn && (
                  <span className="text-xs text-amber-500 font-medium mt-1">
                    Ajoutez d&apos;abord une passkey ou une cle de securite dans la double
                    authentification pour activer cette option.
                  </span>
                )}
              </div>
            </div>
            <div className="flex items-center shrink-0">
              <input
                id="skip-password-toggle"
                type="checkbox"
                className="size-4 rounded border-border text-primary focus:ring-ring cursor-pointer disabled:cursor-not-allowed"
                disabled={!overview.has_webauthn || mutation.isPending}
                checked={overview.skip_password}
                onChange={(e) => mutation.mutate(e.target.checked)}
              />
            </div>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Verification de l&apos;email</CardTitle>
          <CardDescription>
            Une adresse email verifiee est requise pour acceder aux fonctionnalites avancees.
          </CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-3">
          <div className="flex items-center justify-between gap-3">
            <div className="flex items-center gap-3 min-w-0">
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
    </div>
  );
}
