import type { EnterpriseTrustCenterResponse } from '@nvbes/identity-client';
import { useQuery } from '@tanstack/react-query';
import { AlertCircle, Building2, Globe2, KeyRound } from 'lucide-react';
import { Alert, AlertDescription, AlertTitle } from '../components/ui/alert';
import { Badge } from '../components/ui/badge';
import { Card, CardContent, CardHeader, CardTitle } from '../components/ui/card';
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from '../components/ui/empty';
import { Skeleton } from '../components/ui/skeleton';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '../components/ui/table';
import { enterpriseTrustCenterQueryOptions } from '../enterprise.queries';

type TrustDomain = EnterpriseTrustCenterResponse['verified_domains'][number];
type TrustProvider = EnterpriseTrustCenterResponse['sso']['providers'][number];

export function SettingsPage() {
  const trustQuery = useQuery(enterpriseTrustCenterQueryOptions());
  const trustCenter = trustQuery.data;

  return (
    <div className="flex flex-col gap-6">
      <header className="flex flex-col gap-3 md:flex-row md:items-end md:justify-between">
        <div>
          <div className="flex items-center gap-2">
            <h1 className="text-2xl font-heading font-semibold">Settings</h1>
            <Badge variant="outline" className="rounded-md">
              Trust
            </Badge>
          </div>
          <p className="mt-2 max-w-2xl text-sm leading-6 text-muted-foreground">
            Tenant identity settings for domain verification, SSO readiness, and security tier.
          </p>
        </div>
        {trustCenter ? (
          <Badge variant={trustCenter.sso.enabled ? 'default' : 'secondary'} className="rounded-md">
            {trustCenter.sso.enabled ? 'SSO configured' : 'SSO pending'}
          </Badge>
        ) : null}
      </header>

      {trustQuery.error ? <SettingsErrorAlert error={trustQuery.error} /> : null}

      {trustQuery.isPending ? (
        <SettingsLoadingState />
      ) : trustCenter ? (
        <>
          <section className="grid gap-3 md:grid-cols-3">
            <SettingMetric
              icon={Building2}
              label="Tenant"
              value={trustCenter.tenant.name}
              detail={trustCenter.tenant.security_tier}
            />
            <SettingMetric
              icon={Globe2}
              label="Verified domains"
              value={`${verifiedDomainCount(trustCenter.verified_domains)}`}
              detail={`${trustCenter.verified_domains.length} registered`}
            />
            <SettingMetric
              icon={KeyRound}
              label="SSO providers"
              value={`${trustCenter.sso.active_providers}`}
              detail={`${trustCenter.sso.required_domains} required domains`}
            />
          </section>

          <section className="grid gap-4 xl:grid-cols-2">
            <DomainsCard domains={trustCenter.verified_domains} />
            <SsoCard providers={trustCenter.sso.providers} />
          </section>
        </>
      ) : null}
    </div>
  );
}

function SettingMetric({
  icon: Icon,
  label,
  value,
  detail,
}: {
  icon: typeof Building2;
  label: string;
  value: string;
  detail: string;
}) {
  return (
    <Card className="min-h-28 rounded-lg" size="sm">
      <CardContent>
        <div className="flex items-center justify-between gap-3">
          <p className="truncate text-xs font-medium text-muted-foreground">{label}</p>
          <Icon className="size-4 shrink-0 text-muted-foreground" />
        </div>
        <p className="mt-4 truncate text-xl font-heading font-semibold">{value}</p>
        <p className="mt-1 truncate text-xs text-muted-foreground">{detail}</p>
      </CardContent>
    </Card>
  );
}

function DomainsCard({ domains }: { domains: TrustDomain[] }) {
  return (
    <Card className="rounded-lg" size="sm">
      <CardHeader className="border-b border-border">
        <CardTitle>Domains</CardTitle>
      </CardHeader>
      <CardContent>
        {domains.length > 0 ? (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Domain</TableHead>
                <TableHead>Status</TableHead>
                <TableHead>SSO</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {domains.map((domain) => (
                <TableRow key={domain.id}>
                  <TableCell className="font-medium">{domain.domain}</TableCell>
                  <TableCell>
                    <Badge
                      variant={domain.verified ? 'default' : 'secondary'}
                      className="rounded-md"
                    >
                      {domain.verified ? 'Verified' : 'Pending'}
                    </Badge>
                  </TableCell>
                  <TableCell>{domain.sso_required ? 'Required' : 'Optional'}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        ) : (
          <EmptyState
            title="No domains"
            description="Add and verify a tenant domain before enforcing SSO."
          />
        )}
      </CardContent>
    </Card>
  );
}

function SsoCard({ providers }: { providers: TrustProvider[] }) {
  return (
    <Card className="rounded-lg" size="sm">
      <CardHeader className="border-b border-border">
        <CardTitle>SSO providers</CardTitle>
      </CardHeader>
      <CardContent>
        {providers.length > 0 ? (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Name</TableHead>
                <TableHead>Type</TableHead>
                <TableHead>Status</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {providers.map((provider) => (
                <TableRow key={provider.id}>
                  <TableCell className="font-medium">{provider.name}</TableCell>
                  <TableCell>{provider.provider_family}</TableCell>
                  <TableCell>
                    <Badge
                      variant={provider.status === 'active' ? 'default' : 'secondary'}
                      className="rounded-md"
                    >
                      {provider.status}
                    </Badge>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        ) : (
          <EmptyState
            title="No SSO provider"
            description="Configure a SAML or OIDC provider to centralize enterprise login."
          />
        )}
      </CardContent>
    </Card>
  );
}

function EmptyState({ title, description }: { title: string; description: string }) {
  return (
    <Empty className="min-h-48 border bg-muted/20">
      <EmptyMedia variant="icon">
        <AlertCircle className="size-4" />
      </EmptyMedia>
      <EmptyHeader>
        <EmptyTitle>{title}</EmptyTitle>
        <EmptyDescription>{description}</EmptyDescription>
      </EmptyHeader>
    </Empty>
  );
}

function SettingsLoadingState() {
  return (
    <div className="grid gap-4 xl:grid-cols-2">
      <Skeleton className="h-64 w-full rounded-lg" />
      <Skeleton className="h-64 w-full rounded-lg" />
    </div>
  );
}

function SettingsErrorAlert({ error }: { error: unknown }) {
  return (
    <Alert variant="destructive">
      <AlertCircle className="size-4" />
      <AlertTitle>Settings unavailable</AlertTitle>
      <AlertDescription>
        {error instanceof Error ? error.message : 'Unable to load tenant trust settings.'}
      </AlertDescription>
    </Alert>
  );
}

function verifiedDomainCount(domains: TrustDomain[]): number {
  return domains.filter((domain) => domain.verified).length;
}
