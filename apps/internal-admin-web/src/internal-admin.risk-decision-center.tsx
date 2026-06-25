import { useQuery } from '@tanstack/react-query';
import { AlertTriangle, Gauge, ShieldAlert } from 'lucide-react';
import type { ComponentType } from 'react';
import { Badge } from '@/components/ui/badge';
import { getRiskDecisionCenter } from './internal-admin.api';
import { LockedState } from './internal-admin.locked-state';
import { RiskDecisionActionsPanel } from './internal-admin.risk-decision-actions';
import {
  AccessPolicyList,
  BillingScoreList,
  BillingSignalList,
  IdentityRiskList,
} from './internal-admin.risk-decision-lists';
import type { AdminCredentials } from './internal-admin.types';

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
      <RiskDecisionActionsPanel credentials={credentials} disabled={disabled} />
    </section>
  );
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
