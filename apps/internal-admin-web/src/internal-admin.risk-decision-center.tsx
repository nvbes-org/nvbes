import { useQuery } from '@tanstack/react-query';
import { AlertTriangle, Building2, Gauge, ShieldAlert, UserRound } from 'lucide-react';
import type { ComponentType, ReactNode } from 'react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { getRiskDecisionCenter } from './internal-admin.api';
import { LockedState } from './internal-admin.locked-state';
import type {
  AccessPolicySnapshot,
  AdminCredentials,
  BillingRiskScore,
  BillingRiskSignal,
  IdentityRiskDecision,
} from './internal-admin.types';

export function RiskDecisionCenterPanel({
  credentials,
  disabled,
  onSelectTenant,
  onSelectUser,
  onSelectWorkspace,
}: {
  credentials: AdminCredentials;
  disabled: boolean;
  onSelectTenant: (tenantId: string) => void;
  onSelectUser: (principalId: string) => void;
  onSelectWorkspace: (workspaceId: string) => void;
}) {
  const risk = useQuery({
    queryKey: ['risk-decision-center'],
    queryFn: () => getRiskDecisionCenter(credentials),
    enabled: !disabled,
  });
  const data = risk.data;

  return (
    <section className="border-border bg-card mb-5 rounded-lg border p-4" id="risk-decision-center">
      <div className="mb-4 flex flex-col gap-2 md:flex-row md:items-end md:justify-between">
        <div>
          <p className="text-muted-foreground text-xs font-medium tracking-[0.18em] uppercase">
            Risk decision
          </p>
          <h2 className="text-base font-semibold">Fraude, decisions et restrictions d acces</h2>
        </div>
        <Badge
          variant={
            (data?.high_identity_risk_event_count_24h ?? 0) > 0 ? 'destructive' : 'secondary'
          }
        >
          {formatCount(data?.high_identity_risk_event_count_24h)} high identity risk 24h
        </Badge>
      </div>
      {disabled ? (
        <LockedState label="Connecte un contexte operateur pour charger les decisions risk." />
      ) : null}
      <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-6">
        <RiskMetric
          icon={ShieldAlert}
          label="Identity risk 24h"
          value={formatCount(data?.identity_risk_event_count_24h)}
        />
        <RiskMetric
          icon={AlertTriangle}
          label="High identity"
          tone={(data?.high_identity_risk_event_count_24h ?? 0) > 0 ? 'danger' : 'default'}
          value={formatCount(data?.high_identity_risk_event_count_24h)}
        />
        <RiskMetric
          icon={Gauge}
          label="Billing signals"
          value={formatCount(data?.billing_risk_signal_count_24h)}
        />
        <RiskMetric
          icon={AlertTriangle}
          label="High scores"
          tone={(data?.high_billing_risk_score_count ?? 0) > 0 ? 'danger' : 'default'}
          value={formatCount(data?.high_billing_risk_score_count)}
        />
        <RiskMetric
          icon={ShieldAlert}
          label="Active policies"
          tone={(data?.active_access_policy_count ?? 0) > 0 ? 'warning' : 'default'}
          value={formatCount(data?.active_access_policy_count)}
        />
        <RiskMetric
          icon={ShieldAlert}
          label="Policies 24h"
          value={formatCount(data?.access_policy_count_24h)}
        />
      </div>
      <div className="mt-4 grid gap-3 xl:grid-cols-2">
        <IdentityRiskList
          onSelectTenant={onSelectTenant}
          onSelectUser={onSelectUser}
          rows={data?.recent_identity_risks ?? []}
        />
        <BillingScoreList onSelectTenant={onSelectTenant} rows={data?.billing_risk_scores ?? []} />
        <BillingSignalList
          onSelectTenant={onSelectTenant}
          rows={data?.billing_risk_signals ?? []}
        />
        <AccessPolicyList
          onSelectTenant={onSelectTenant}
          onSelectWorkspace={onSelectWorkspace}
          rows={data?.active_access_policies ?? []}
        />
      </div>
    </section>
  );
}

function IdentityRiskList({
  onSelectTenant,
  onSelectUser,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  onSelectUser: (principalId: string) => void;
  rows: IdentityRiskDecision[];
}) {
  return (
    <RiskList emptyLabel="Aucun risk event identity." title="Identity risk decisions">
      {rows.map((row) => (
        <RiskActionRow
          badge={formatRiskScore(row.risk_score)}
          key={row.id}
          onSelectTenant={() => onSelectTenant(row.tenant_id)}
          onSelectUser={() => onSelectUser(row.principal_id)}
          subtitle={`${row.tenant_name} - ${row.decision} - ${formatDate(row.created_at)}`}
          title={row.email ?? row.event_type}
          tone={row.risk_score >= 0.7 ? 'danger' : 'warning'}
        />
      ))}
    </RiskList>
  );
}

function BillingScoreList({
  onSelectTenant,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  rows: BillingRiskScore[];
}) {
  return (
    <RiskList emptyLabel="Aucun billing risk score." title="Billing risk scores">
      {rows.map((row) => (
        <RiskActionRow
          badge={formatCount(row.score)}
          key={row.id}
          onSelectTenant={() => onSelectTenant(row.tenant_id)}
          subtitle={`${row.tenant_name} - ${formatDate(row.created_at)}`}
          title={row.decision}
          tone={row.score >= 70 ? 'danger' : 'warning'}
        />
      ))}
    </RiskList>
  );
}

function BillingSignalList({
  onSelectTenant,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  rows: BillingRiskSignal[];
}) {
  return (
    <RiskList emptyLabel="Aucun billing risk signal." title="Billing risk signals">
      {rows.map((row) => (
        <RiskActionRow
          badge={formatDate(row.occurred_at)}
          disableTenant={!row.tenant_id}
          key={row.id}
          onSelectTenant={() => {
            if (row.tenant_id) onSelectTenant(row.tenant_id);
          }}
          subtitle={row.tenant_name ?? 'global signal'}
          title={row.signal_type}
        />
      ))}
    </RiskList>
  );
}

function AccessPolicyList({
  onSelectTenant,
  onSelectWorkspace,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  onSelectWorkspace: (workspaceId: string) => void;
  rows: AccessPolicySnapshot[];
}) {
  return (
    <RiskList emptyLabel="Aucune policy access active." title="Active access policies">
      {rows.map((row) => (
        <RiskActionRow
          badge={row.policy_state}
          key={row.id}
          onSelectTenant={() => onSelectTenant(row.tenant_id)}
          onSelectWorkspace={
            row.workspace_id ? () => onSelectWorkspace(row.workspace_id ?? '') : undefined
          }
          subtitle={`${row.tenant_name} - ${row.workspace_name ?? 'tenant scoped'} - ${row.reason}`}
          title={formatDate(row.effective_from)}
          tone="warning"
        />
      ))}
    </RiskList>
  );
}

function RiskActionRow({
  badge,
  disableTenant = false,
  onSelectTenant,
  onSelectUser,
  onSelectWorkspace,
  subtitle,
  title,
  tone = 'default',
}: {
  badge: string;
  disableTenant?: boolean;
  onSelectTenant: () => void;
  onSelectUser?: () => void;
  onSelectWorkspace?: () => void;
  subtitle: string;
  title: string;
  tone?: 'danger' | 'default' | 'warning';
}) {
  return (
    <article className="p-3">
      <div className="mb-2 flex items-start justify-between gap-2">
        <div className="min-w-0">
          <p className="truncate text-sm font-medium">{title}</p>
          <p className="text-muted-foreground truncate text-xs">{subtitle}</p>
        </div>
        <Badge variant={tone === 'danger' ? 'destructive' : 'outline'}>{badge}</Badge>
      </div>
      <div className="flex flex-wrap gap-2">
        <Button
          disabled={disableTenant}
          onClick={() => {
            onSelectTenant();
            window.location.hash = 'tenant-detail';
          }}
          size="sm"
          type="button"
          variant="ghost"
        >
          <Building2 className="size-4" />
          Tenant
        </Button>
        {onSelectUser ? (
          <Button
            onClick={() => {
              onSelectUser();
              window.location.hash = 'user-detail';
            }}
            size="sm"
            type="button"
            variant="outline"
          >
            <UserRound className="size-4" />
            User
          </Button>
        ) : null}
        {onSelectWorkspace ? (
          <Button
            onClick={() => {
              onSelectWorkspace();
              window.location.hash = 'workspace-detail';
            }}
            size="sm"
            type="button"
            variant="outline"
          >
            Workspace
          </Button>
        ) : null}
      </div>
    </article>
  );
}

function RiskList({
  children,
  emptyLabel,
  title,
}: {
  children: ReactNode[];
  emptyLabel: string;
  title: string;
}) {
  return (
    <div className="rounded-md border">
      <div className="border-b p-3">
        <h3 className="text-sm font-medium">{title}</h3>
      </div>
      <div className="divide-y">
        {children.length === 0 ? <EmptyRow label={emptyLabel} /> : children}
      </div>
    </div>
  );
}

function EmptyRow({ label }: { label: string }) {
  return <div className="text-muted-foreground p-4 text-sm">{label}</div>;
}

function RiskMetric({
  icon: Icon,
  label,
  tone = 'default',
  value,
}: {
  icon: ComponentType<{ className?: string }>;
  label: string;
  tone?: 'danger' | 'default' | 'warning';
  value: string;
}) {
  const iconClass =
    tone === 'danger'
      ? 'text-destructive size-4'
      : tone === 'warning'
        ? 'text-amber-500 size-4'
        : 'size-4';

  return (
    <div className="bg-muted/30 rounded-md border p-3">
      <div className="text-muted-foreground mb-3 flex items-center justify-between text-xs">
        {label}
        <Icon className={iconClass} />
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

function formatDate(value: string): string {
  return new Intl.DateTimeFormat('fr-FR').format(new Date(value));
}

function formatRiskScore(value: number): string {
  return new Intl.NumberFormat('fr-FR', { maximumFractionDigits: 2 }).format(value);
}
