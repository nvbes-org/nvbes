import { useQuery } from '@tanstack/react-query';
import { AlertTriangle, KeyRound, ShieldAlert, ShieldCheck, UserRound, Users } from 'lucide-react';
import type { ComponentType } from 'react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { getSecurityCenter } from './backoffice-service.api';
import { SecurityActionSections } from './backoffice-service.security-actions';
import { LockedState } from './backoffice-service.locked-state';
import type { AdminCredentials, RecentRiskEvent, UserWithoutMfa } from './backoffice-service.types';

export function SecurityCenterPanel({
  credentials,
  disabled,
  onSelectTenant,
  onSelectUser,
}: {
  credentials: AdminCredentials;
  disabled: boolean;
  onSelectTenant: (tenantId: string) => void;
  onSelectUser: (principalId: string) => void;
}) {
  const security = useQuery({
    queryKey: ['security-center'],
    queryFn: () => getSecurityCenter(credentials),
    enabled: !disabled,
  });
  const data = security.data;

  return (
    <section className="border-border bg-card mb-5 rounded-lg border p-4" id="security-center">
      <div className="mb-4 flex flex-col gap-2 md:flex-row md:items-end md:justify-between">
        <div>
          <p className="text-muted-foreground text-xs font-medium tracking-[0.18em] uppercase">
            Security center
          </p>
          <h2 className="text-base font-semibold">Risque, MFA et comptes sensibles</h2>
        </div>
        <Badge variant={(data?.high_risk_events_24h ?? 0) > 0 ? 'destructive' : 'secondary'}>
          {formatCount(data?.high_risk_events_24h)} high risk 24h
        </Badge>
      </div>
      {disabled ? (
        <LockedState label="Connecte un contexte operateur pour charger les signaux securite." />
      ) : null}
      <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-7">
        <SecurityMetric
          icon={ShieldAlert}
          label="Risk 24h"
          tone={(data?.risk_events_24h ?? 0) > 0 ? 'danger' : 'default'}
          value={formatCount(data?.risk_events_24h)}
        />
        <SecurityMetric
          icon={AlertTriangle}
          label="High risk"
          tone={(data?.high_risk_events_24h ?? 0) > 0 ? 'danger' : 'default'}
          value={formatCount(data?.high_risk_events_24h)}
        />
        <SecurityMetric
          icon={ShieldCheck}
          label="No MFA users"
          tone={(data?.active_users_without_mfa ?? 0) > 0 ? 'danger' : 'default'}
          value={formatCount(data?.active_users_without_mfa)}
        />
        <SecurityMetric
          icon={UserRound}
          label="Suspended"
          value={formatCount(data?.suspended_principal_count)}
        />
        <SecurityMetric
          icon={UserRound}
          label="Revoked"
          value={formatCount(data?.revoked_principal_count)}
        />
        <SecurityMetric
          icon={Users}
          label="Unverified"
          tone={(data?.unverified_user_count ?? 0) > 0 ? 'danger' : 'default'}
          value={formatCount(data?.unverified_user_count)}
        />
        <SecurityMetric
          icon={KeyRound}
          label="OAuth consents"
          value={formatCount(data?.active_oauth_consent_count)}
        />
      </div>
      <div className="mt-4 grid gap-3 xl:grid-cols-2">
        <RiskEventList
          rows={data?.recent_risk_events ?? []}
          onSelectTenant={onSelectTenant}
          onSelectUser={onSelectUser}
        />
        <UsersWithoutMfaList
          rows={data?.users_without_mfa ?? []}
          onSelectTenant={onSelectTenant}
          onSelectUser={onSelectUser}
        />
      </div>
      <SecurityActionSections
        credentials={credentials}
        disabled={disabled}
        mfaFactors={data?.active_mfa_factors ?? []}
        oauthConsents={data?.active_oauth_consents ?? []}
        onSelectUser={onSelectUser}
      />
    </section>
  );
}

function RiskEventList({
  onSelectTenant,
  onSelectUser,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  onSelectUser: (principalId: string) => void;
  rows: RecentRiskEvent[];
}) {
  return (
    <div className="rounded-md border">
      <div className="border-b p-3">
        <h3 className="text-sm font-medium">Risk events recents</h3>
      </div>
      <div className="divide-y">
        {rows.length === 0 ? <EmptyRow label="Aucun risk event recent." /> : null}
        {rows.map((row) => (
          <article className="p-3" key={row.id}>
            <div className="mb-2 flex items-center justify-between gap-2">
              <div className="min-w-0">
                <p className="truncate text-sm font-medium">{row.email ?? row.principal_id}</p>
                <p className="text-muted-foreground text-xs">
                  {row.event_type} - score {row.risk_score.toFixed(2)}
                </p>
              </div>
              <Badge variant={row.risk_score >= 0.7 ? 'destructive' : 'outline'}>
                {row.decision}
              </Badge>
            </div>
            <RowActions
              onTenant={() => onSelectTenant(row.tenant_id)}
              onUser={() => onSelectUser(row.principal_id)}
            />
          </article>
        ))}
      </div>
    </div>
  );
}

function UsersWithoutMfaList({
  onSelectTenant,
  onSelectUser,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  onSelectUser: (principalId: string) => void;
  rows: UserWithoutMfa[];
}) {
  return (
    <div className="rounded-md border">
      <div className="border-b p-3">
        <h3 className="text-sm font-medium">Users actifs sans MFA</h3>
      </div>
      <div className="divide-y">
        {rows.length === 0 ? <EmptyRow label="Tous les users actifs ont un MFA actif." /> : null}
        {rows.map((row) => (
          <article className="p-3" key={row.principal_id}>
            <div className="mb-2 min-w-0">
              <p className="truncate text-sm font-medium">{row.email}</p>
              <p className="text-muted-foreground text-xs">
                {row.name} - {row.tenant_name}
              </p>
            </div>
            <RowActions
              onTenant={() => onSelectTenant(row.tenant_id)}
              onUser={() => onSelectUser(row.principal_id)}
            />
          </article>
        ))}
      </div>
    </div>
  );
}

function RowActions({ onTenant, onUser }: { onTenant: () => void; onUser: () => void }) {
  return (
    <div className="flex flex-wrap gap-2">
      <Button
        onClick={() => {
          onUser();
          window.location.hash = 'user-detail';
        }}
        size="sm"
        type="button"
        variant="outline"
      >
        Open user
      </Button>
      <Button
        onClick={() => {
          onTenant();
          window.location.hash = 'tenant-detail';
        }}
        size="sm"
        type="button"
        variant="ghost"
      >
        Tenant
      </Button>
    </div>
  );
}

function EmptyRow({ label }: { label: string }) {
  return <div className="text-muted-foreground p-4 text-sm">{label}</div>;
}

function SecurityMetric({
  icon: Icon,
  label,
  tone = 'default',
  value,
}: {
  icon: ComponentType<{ className?: string }>;
  label: string;
  tone?: 'danger' | 'default';
  value: string;
}) {
  return (
    <div className="bg-muted/30 rounded-md border p-3">
      <div className="text-muted-foreground mb-3 flex items-center justify-between text-xs">
        {label}
        <Icon className={tone === 'danger' ? 'text-destructive size-4' : 'size-4'} />
      </div>
      <div
        className={
          tone === 'danger' ? 'text-destructive text-xl font-semibold' : 'text-xl font-semibold'
        }
      >
        {value}
      </div>
    </div>
  );
}

function formatCount(value: number | undefined): string {
  return typeof value === 'number' ? new Intl.NumberFormat('fr-FR').format(value) : '-';
}
