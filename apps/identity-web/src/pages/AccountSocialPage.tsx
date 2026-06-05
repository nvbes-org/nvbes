import { useQuery, useQueryClient } from '@tanstack/react-query';
import { ExternalLink, Link, Unlink } from 'lucide-react';
import { useState } from 'react';
import { z } from 'zod';
import { accountQueryKeys } from '@/account.queries';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Separator } from '@/components/ui/separator';
import { Skeleton } from '@/components/ui/skeleton';
import { useAccountContext } from '@/hooks/useAccountContext';
import { identityHttpClient } from '../identity.http';

const LinkedIdentitySchema = z.object({
  id: z.string(),
  provider_type: z.string(),
  provider_id: z.string(),
  subject: z.string(),
  created_at: z.string().optional(),
  principal_display_name: z.string().optional(),
});

const LinkedIdentitiesResponseSchema = z.object({
  identities: z.array(LinkedIdentitySchema),
});

type LinkedIdentity = z.infer<typeof LinkedIdentitySchema>;

function listLinkedIdentities(tenantId: string, signal?: AbortSignal): Promise<LinkedIdentity[]> {
  return identityHttpClient
    .get(`/tenants/${tenantId}/linked-identities`, LinkedIdentitiesResponseSchema, { signal })
    .then((r) => r.identities)
    .catch(() => []);
}

function unlinkIdentity(tenantId: string, identityId: string): Promise<void> {
  const emptySchema = z.undefined();
  return identityHttpClient.delete(
    `/tenants/${tenantId}/linked-identities/${identityId}`,
    emptySchema,
  );
}

const providerLabels: Record<string, string> = {
  google: 'Google',
  github: 'GitHub',
  microsoft: 'Microsoft',
  apple: 'Apple',
  oidc: 'OpenID Connect',
  saml: 'SAML',
  linkedin: 'LinkedIn',
  facebook: 'Facebook',
  twitter: 'Twitter',
};

function SocialSkeleton() {
  return (
    <div className="flex flex-col gap-6">
      <div>
        <Skeleton className="h-6 w-40" />
        <Skeleton className="h-4 w-72 mt-1" />
      </div>
      <Card>
        <CardHeader>
          <Skeleton className="h-5 w-40" />
          <Skeleton className="h-4 w-56" />
        </CardHeader>
        <CardContent className="flex flex-col gap-3">
          {Array.from({ length: 2 }).map((_, i) => (
            <Skeleton key={i} className="h-12 w-full rounded-lg" />
          ))}
        </CardContent>
      </Card>
    </div>
  );
}

export default function AccountSocialPage() {
  const [unlinking, setUnlinking] = useState<string | null>(null);
  const queryClient = useQueryClient();
  const { me, loading: accountLoading } = useAccountContext();
  const tenantId = me?.current_tenant_id ?? null;

  const { data: identities = [], isPending } = useQuery({
    queryKey: [...accountQueryKeys.all, 'linked-identities', tenantId] as const,
    queryFn: ({ signal }) =>
      tenantId ? listLinkedIdentities(tenantId, signal) : Promise.resolve([]),
    enabled: Boolean(tenantId),
    staleTime: 5 * 60 * 1000,
    gcTime: 30 * 60 * 1000,
    refetchOnWindowFocus: false,
  });

  const handleUnlink = async (identity: LinkedIdentity) => {
    if (!tenantId) return;
    const queryKey = [...accountQueryKeys.all, 'linked-identities', tenantId] as const;
    const previous = queryClient.getQueryData<LinkedIdentity[]>(queryKey) ?? [];
    queryClient.setQueryData<LinkedIdentity[]>(
      queryKey,
      previous.filter((i) => i.id !== identity.id),
    );
    setUnlinking(identity.id);
    try {
      await unlinkIdentity(tenantId, identity.id);
    } catch {
      queryClient.setQueryData(queryKey, previous);
    } finally {
      setUnlinking(null);
    }
  };

  if (accountLoading || (tenantId && isPending)) return <SocialSkeleton />;

  return (
    <div className="flex flex-col gap-6 animate-fade-slide-up [animation-delay:0ms]">
      <div>
        <h1 className="text-xl font-heading font-semibold">Contenu & social</h1>
        <p className="text-sm text-muted-foreground mt-1">
          Gerer vos profils sociaux et identites liees.
        </p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Identites liees</CardTitle>
          <CardDescription>
            Comptes externes lies a votre identite nvbes. La gestion des identites liees est
            administree au niveau de votre organisation.
          </CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-1">
          {identities.length === 0 ? (
            <div className="flex flex-col items-center gap-3 py-8">
              <Link className="size-8 text-muted-foreground" />
              <div className="text-center">
                <p className="text-sm text-muted-foreground">
                  Aucune identite externe liee a votre compte.
                </p>
                {!me?.current_tenant_id && (
                  <p className="text-xs text-muted-foreground mt-1">Aucune organisation active.</p>
                )}
              </div>
            </div>
          ) : (
            identities.map((identity, index) => {
              const providerLabel =
                providerLabels[identity.provider_type] ?? identity.provider_type;
              const subject = identity.principal_display_name ?? identity.subject;

              return (
                <div key={identity.id}>
                  {index > 0 && <Separator className="my-1" />}
                  <div className="flex items-center justify-between gap-3 py-1">
                    <div className="flex items-center gap-3 min-w-0">
                      <div className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-muted">
                        <ExternalLink className="size-4 text-muted-foreground" />
                      </div>
                      <div className="flex flex-col min-w-0">
                        <span className="text-sm font-medium">{providerLabel}</span>
                        <span className="text-xs text-muted-foreground truncate">{subject}</span>
                      </div>
                    </div>
                    <div className="flex items-center gap-2">
                      <Badge variant="secondary" className="shrink-0">
                        {identity.provider_type}
                      </Badge>
                      <Button
                        variant="ghost"
                        size="sm"
                        className="h-8 w-8 p-0 text-muted-foreground hover:text-destructive"
                        onClick={() => handleUnlink(identity)}
                        disabled={unlinking === identity.id}
                        aria-label={`Delier ${providerLabel}`}
                      >
                        <Unlink className="size-3.5" />
                      </Button>
                    </div>
                  </div>
                </div>
              );
            })
          )}
        </CardContent>
      </Card>
    </div>
  );
}
