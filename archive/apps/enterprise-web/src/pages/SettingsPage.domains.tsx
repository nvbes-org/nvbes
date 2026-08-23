import type { EnterpriseTrustCenterResponse } from '@nvbes/identity-client';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { Globe2, ShieldCheck } from 'lucide-react';
import { useState } from 'react';
import { enterpriseClient } from '../enterprise.api';
import { enterpriseQueryKeys } from '../enterprise.queries';
import { Badge } from '../components/ui/badge';
import { Button } from '../components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '../components/ui/card';
import { Checkbox } from '../components/ui/checkbox';
import { Input } from '../components/ui/input';
import { Label } from '../components/ui/label';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '../components/ui/table';
import { EmptyState } from './SettingsPage.empty';

type TrustDomain = EnterpriseTrustCenterResponse['verified_domains'][number];
type TrustProvider = EnterpriseTrustCenterResponse['sso']['providers'][number];

type DomainAction = {
  domainId: string;
  ssoRequired: boolean;
};

export function DomainsCard({
  tenantId,
  domains,
  providers,
}: {
  tenantId: string;
  domains: TrustDomain[];
  providers: TrustProvider[];
}) {
  const queryClient = useQueryClient();
  const activeProvider = providers.find((provider) => provider.status === 'active');
  const [domainName, setDomainName] = useState('');
  const [createdToken, setCreatedToken] = useState<string | null>(null);
  const [verificationTokens, setVerificationTokens] = useState<Record<string, string>>({});

  const refreshTrustCenter = () =>
    queryClient.invalidateQueries({ queryKey: enterpriseQueryKeys.trustCenter });

  const createMutation = useMutation({
    mutationFn: (domain: string) =>
      enterpriseClient.createTenantDomain(tenantId, {
        domain,
        sso_required: false,
      }),
    onSuccess: (response) => {
      setDomainName('');
      setCreatedToken(response.verification_token ?? null);
      return refreshTrustCenter();
    },
  });

  const verifyMutation = useMutation({
    mutationFn: ({ domainId, token }: { domainId: string; token: string }) =>
      enterpriseClient.verifyTenantDomain(tenantId, domainId, { token }),
    onSuccess: (_response, variables) => {
      setVerificationTokens((current) => {
        const next = { ...current };
        delete next[variables.domainId];
        return next;
      });
      return refreshTrustCenter();
    },
  });

  const ssoMutation = useMutation({
    mutationFn: ({ domainId, ssoRequired }: DomainAction) =>
      enterpriseClient.updateTenantDomain(tenantId, domainId, {
        sso_required: ssoRequired,
        sso_provider_id: ssoRequired ? activeProvider?.id : null,
      }),
    onSuccess: () => refreshTrustCenter(),
  });

  return (
    <Card className="rounded-lg" size="sm">
      <CardHeader className="border-b border-border">
        <CardTitle>Domains</CardTitle>
      </CardHeader>
      <CardContent className="flex flex-col gap-4">
        <form
          className="grid gap-3 md:grid-cols-[1fr_auto]"
          onSubmit={(event) => {
            event.preventDefault();
            const domain = domainName.trim();
            if (domain) {
              createMutation.mutate(domain);
            }
          }}
        >
          <div className="grid gap-2">
            <Label htmlFor="tenant-domain">Domain</Label>
            <Input
              id="tenant-domain"
              placeholder="example.com"
              value={domainName}
              onChange={(event) => setDomainName(event.target.value)}
            />
          </div>
          <Button type="submit" className="self-end" disabled={!domainName.trim()}>
            <Globe2 className="size-4" />
            Add
          </Button>
        </form>

        {createdToken ? (
          <div className="rounded-md border border-border bg-muted/30 p-3 text-sm">
            <p className="font-medium">Verification token</p>
            <code className="mt-2 block break-all rounded bg-background px-2 py-1 text-xs">
              {createdToken}
            </code>
          </div>
        ) : null}

        {createMutation.error ? <MutationError error={createMutation.error} /> : null}

        {domains.length > 0 ? (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Domain</TableHead>
                <TableHead>Status</TableHead>
                <TableHead>SSO</TableHead>
                <TableHead className="w-48">Verify</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {domains.map((domain) => (
                <DomainRow
                  key={domain.id}
                  domain={domain}
                  activeProviderId={activeProvider?.id ?? null}
                  token={verificationTokens[domain.id] ?? ''}
                  pendingVerify={verifyMutation.isPending}
                  pendingSso={ssoMutation.isPending}
                  onTokenChange={(token) =>
                    setVerificationTokens((current) => ({ ...current, [domain.id]: token }))
                  }
                  onVerify={(token) => verifyMutation.mutate({ domainId: domain.id, token })}
                  onSsoChange={(ssoRequired) =>
                    ssoMutation.mutate({ domainId: domain.id, ssoRequired })
                  }
                />
              ))}
            </TableBody>
          </Table>
        ) : (
          <EmptyState
            title="No domains"
            description="Add and verify a tenant domain before enforcing SSO."
          />
        )}

        {verifyMutation.error ? <MutationError error={verifyMutation.error} /> : null}
        {ssoMutation.error ? <MutationError error={ssoMutation.error} /> : null}
      </CardContent>
    </Card>
  );
}

function DomainRow({
  domain,
  activeProviderId,
  token,
  pendingVerify,
  pendingSso,
  onTokenChange,
  onVerify,
  onSsoChange,
}: {
  domain: TrustDomain;
  activeProviderId: string | null;
  token: string;
  pendingVerify: boolean;
  pendingSso: boolean;
  onTokenChange: (token: string) => void;
  onVerify: (token: string) => void;
  onSsoChange: (ssoRequired: boolean) => void;
}) {
  const canRequireSso = domain.sso_required || activeProviderId !== null;

  return (
    <TableRow>
      <TableCell className="font-medium">{domain.domain}</TableCell>
      <TableCell>
        <Badge variant={domain.verified ? 'default' : 'secondary'} className="rounded-md">
          {domain.verified ? 'Verified' : 'Pending'}
        </Badge>
      </TableCell>
      <TableCell>
        <label className="flex items-center gap-2 text-sm">
          <Checkbox
            checked={domain.sso_required}
            disabled={pendingSso || !canRequireSso}
            onCheckedChange={(checked) => onSsoChange(checked === true)}
          />
          <span>{domain.sso_required ? 'Required' : 'Optional'}</span>
        </label>
      </TableCell>
      <TableCell>
        {domain.verified ? (
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            <ShieldCheck className="size-4" />
            Done
          </div>
        ) : (
          <form
            className="flex gap-2"
            onSubmit={(event) => {
              event.preventDefault();
              if (token.trim()) {
                onVerify(token.trim());
              }
            }}
          >
            <Input
              aria-label={`Verification token for ${domain.domain}`}
              value={token}
              onChange={(event) => onTokenChange(event.target.value)}
            />
            <Button type="submit" size="sm" disabled={pendingVerify || !token.trim()}>
              Verify
            </Button>
          </form>
        )}
      </TableCell>
    </TableRow>
  );
}

function MutationError({ error }: { error: unknown }) {
  return (
    <p className="text-sm text-destructive">
      {error instanceof Error ? error.message : 'The domain action failed.'}
    </p>
  );
}
