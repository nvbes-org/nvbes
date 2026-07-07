import { ExternalLink, Link, Unlink } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Separator } from '@/components/ui/separator';
import { Skeleton } from '@/components/ui/skeleton';
import type { LinkedIdentity } from './AccountSocialPage.api';

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

export function SocialSkeleton() {
  return (
    <div className="flex flex-col gap-6">
      <div>
        <Skeleton className="h-6 w-40" />
        <Skeleton className="mt-1 h-4 w-72" />
      </div>
      <Card>
        <CardHeader>
          <Skeleton className="h-5 w-40" />
          <Skeleton className="h-4 w-56" />
        </CardHeader>
        <CardContent className="flex flex-col gap-3">
          {Array.from({ length: 2 }).map((_, index) => (
            <Skeleton key={index} className="h-12 w-full rounded-lg" />
          ))}
        </CardContent>
      </Card>
    </div>
  );
}

export function SocialLinkedIdentitiesCard({
  identities,
  hasTenant,
  unlinking,
  onUnlink,
}: {
  identities: LinkedIdentity[];
  hasTenant: boolean;
  unlinking: string | null;
  onUnlink: (identity: LinkedIdentity) => void;
}) {
  return (
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
              {!hasTenant ? (
                <p className="mt-1 text-xs text-muted-foreground">Aucune organisation active.</p>
              ) : null}
            </div>
          </div>
        ) : (
          identities.map((identity, index) => {
            const providerLabel = providerLabels[identity.provider_type] ?? identity.provider_type;
            const subject = identity.principal_display_name ?? identity.subject;

            return (
              <div key={identity.id}>
                {index > 0 ? <Separator className="my-1" /> : null}
                <div className="flex items-center justify-between gap-3 py-1">
                  <div className="flex min-w-0 items-center gap-3">
                    <div className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-muted">
                      <ExternalLink className="size-4 text-muted-foreground" />
                    </div>
                    <div className="flex min-w-0 flex-col">
                      <span className="text-sm font-medium">{providerLabel}</span>
                      <span className="truncate text-xs text-muted-foreground">{subject}</span>
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
                      onClick={() => onUnlink(identity)}
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
  );
}
