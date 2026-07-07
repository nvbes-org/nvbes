import type { EnterpriseTrustCenterResponse } from '@nvbes/identity-client';
import {
  BadgeCheck,
  CircleAlert,
  CircleCheck,
  FileSearch,
  Server,
  ShieldCheck,
} from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';

type TrustCenter = EnterpriseTrustCenterResponse;

function formatDate(value: string | null | undefined) {
  if (!value) return 'Non renseigne';
  return new Date(value).toLocaleDateString('fr-FR', {
    day: '2-digit',
    month: 'short',
    year: 'numeric',
  });
}

function StatusBadge({ active, label }: { active: boolean; label: string }) {
  return (
    <Badge variant={active ? 'default' : 'secondary'} className="w-fit gap-1.5">
      {active ? <CircleCheck className="size-3.5" /> : <CircleAlert className="size-3.5" />}
      {label}
    </Badge>
  );
}

function MetricTile({ label, value }: { label: string; value: string | number }) {
  return (
    <div className="rounded-lg border border-border bg-muted/30 px-3 py-2">
      <p className="text-xs text-muted-foreground">{label}</p>
      <p className="mt-1 text-lg font-heading font-semibold">{value}</p>
    </div>
  );
}

export function TrustCenterHero({ trustCenter }: { trustCenter: TrustCenter }) {
  const coverage =
    trustCenter.mfa.active_members === 0
      ? 0
      : Math.round((trustCenter.mfa.members_with_mfa / trustCenter.mfa.active_members) * 100);

  return (
    <Card className="overflow-hidden border-border bg-card">
      <CardHeader className="gap-4">
        <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
          <div>
            <CardTitle className="text-2xl font-heading">Trust Center</CardTitle>
            <CardDescription>
              Page generee pour {trustCenter.tenant.name} le {formatDate(trustCenter.generated_at)}.
            </CardDescription>
          </div>
          <StatusBadge
            active={trustCenter.tenant.status === 'active'}
            label={trustCenter.tenant.status}
          />
        </div>
      </CardHeader>
      <CardContent className="grid gap-3 sm:grid-cols-3">
        <MetricTile label="Couverture MFA" value={`${coverage}%`} />
        <MetricTile
          label="Domaines verifies"
          value={trustCenter.verified_domains.filter((domain) => domain.verified).length}
        />
        <MetricTile label="Regions actives" value={trustCenter.hosting_regions.length} />
      </CardContent>
    </Card>
  );
}

export function TrustCenterSecurityGrid({ trustCenter }: { trustCenter: TrustCenter }) {
  return (
    <div className="grid gap-4 md:grid-cols-2">
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between gap-3">
            <CardTitle className="flex items-center gap-2 text-base">
              <ShieldCheck className="size-4" />
              MFA
            </CardTitle>
            <StatusBadge
              active={trustCenter.mfa.enabled}
              label={trustCenter.mfa.enabled ? 'Active' : 'Inactive'}
            />
          </div>
          <CardDescription>
            {trustCenter.mfa.members_with_mfa} membre(s) sur {trustCenter.mfa.active_members} avec
            MFA.
          </CardDescription>
        </CardHeader>
        <CardContent className="grid gap-3 sm:grid-cols-2">
          <MetricTile label="Facteurs actifs" value={trustCenter.mfa.active_factors} />
          <MetricTile label="Biométrie" value={trustCenter.mfa.passkey_factors} />
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <div className="flex items-center justify-between gap-3">
            <CardTitle className="flex items-center gap-2 text-base">
              <BadgeCheck className="size-4" />
              SSO
            </CardTitle>
            <StatusBadge
              active={trustCenter.sso.enabled}
              label={trustCenter.sso.enabled ? 'Configure' : 'Non configure'}
            />
          </div>
          <CardDescription>
            {trustCenter.sso.active_providers} fournisseur(s), {trustCenter.sso.required_domains}{' '}
            domaine(s) forces.
          </CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-2">
          {trustCenter.sso.providers.length === 0 ? (
            <p className="text-sm text-muted-foreground">Aucun fournisseur SSO configure.</p>
          ) : (
            trustCenter.sso.providers.map((provider) => (
              <div
                key={provider.id}
                className="flex items-center justify-between gap-3 rounded-lg border px-3 py-2"
              >
                <div className="min-w-0">
                  <p className="truncate text-sm font-medium">{provider.name}</p>
                  <p className="text-xs text-muted-foreground">
                    {provider.provider_family} / {provider.provider_type}
                  </p>
                </div>
                <Badge variant="secondary">{provider.status}</Badge>
              </div>
            ))
          )}
        </CardContent>
      </Card>
    </div>
  );
}

export function TrustCenterOperations({ trustCenter }: { trustCenter: TrustCenter }) {
  return (
    <div className="grid gap-4 md:grid-cols-2">
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2 text-base">
            <FileSearch className="size-4" />
            Audit
          </CardTitle>
          <CardDescription>
            Journal append-only, {trustCenter.audit.recent_events.length} evenement(s) recents.
          </CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-2">
          {trustCenter.audit.recent_events.slice(0, 5).map((event) => (
            <div key={event.id} className="flex items-center justify-between gap-3 text-sm">
              <span className="truncate">{event.event_type}</span>
              <span className="shrink-0 text-xs text-muted-foreground">
                {formatDate(event.created_at)}
              </span>
            </div>
          ))}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2 text-base">
            <Server className="size-4" />
            Hebergement
          </CardTitle>
          <CardDescription>Regions et juridictions des workspaces du tenant.</CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-2">
          {trustCenter.hosting_regions.map((region) => (
            <div
              key={`${region.data_region}-${region.legal_jurisdiction}`}
              className="flex items-center justify-between gap-3 rounded-lg border px-3 py-2"
            >
              <div>
                <p className="text-sm font-medium">{region.data_region.toUpperCase()}</p>
                <p className="text-xs text-muted-foreground">{region.legal_jurisdiction}</p>
              </div>
              <Badge variant="secondary">{region.workspace_count} workspace(s)</Badge>
            </div>
          ))}
        </CardContent>
      </Card>
    </div>
  );
}

export function TrustCenterLegal({ trustCenter }: { trustCenter: TrustCenter }) {
  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-base">DPA & sous-traitants</CardTitle>
        <CardDescription>
          {trustCenter.dpa.name} version {trustCenter.dpa.version}, statut {trustCenter.dpa.status}.
        </CardDescription>
      </CardHeader>
      <CardContent className="grid gap-3 md:grid-cols-2">
        {trustCenter.subprocessors.map((processor) => (
          <div key={processor.name} className="rounded-lg border px-3 py-3">
            <div className="flex items-start justify-between gap-3">
              <div>
                <p className="text-sm font-medium">{processor.name}</p>
                <p className="mt-1 text-xs text-muted-foreground">{processor.service}</p>
              </div>
              <Badge variant={processor.transfer_outside_eea ? 'secondary' : 'outline'}>
                {processor.transfer_outside_eea ? 'Transfert' : 'EEE'}
              </Badge>
            </div>
            <p className="mt-3 text-xs text-muted-foreground">
              {processor.location} · {processor.transfer_safeguard}
            </p>
          </div>
        ))}
      </CardContent>
    </Card>
  );
}
