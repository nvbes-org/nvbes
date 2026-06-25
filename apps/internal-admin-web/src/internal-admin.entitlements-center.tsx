import { useQuery } from '@tanstack/react-query';
import { AlertTriangle, Gauge, PackageCheck, ScrollText, Sparkles } from 'lucide-react';
import type { ComponentType } from 'react';
import { Badge } from '@/components/ui/badge';
import { getEntitlementsCenter } from './internal-admin.api';
import { EntitlementsActionsPanel } from './internal-admin.entitlements-actions';
import {
  ExpiringEntitlementList,
  OverQuotaList,
  PlanList,
  UnpublishedChangeList,
} from './internal-admin.entitlements-lists';
import { LockedState } from './internal-admin.locked-state';
import type { AdminCredentials } from './internal-admin.types';

export function EntitlementsCenterPanel({
  credentials,
  disabled,
  onSelectTenant,
  onSelectWorkspace,
}: {
  credentials: AdminCredentials;
  disabled: boolean;
  onSelectTenant: (tenantId: string) => void;
  onSelectWorkspace: (workspaceId: string) => void;
}) {
  const entitlements = useQuery({
    queryKey: ['entitlements-center'],
    queryFn: () => getEntitlementsCenter(credentials),
    enabled: !disabled,
  });
  const data = entitlements.data;

  return (
    <section className="border-border bg-card mb-5 rounded-lg border p-4" id="entitlements-center">
      <div className="mb-4 flex flex-col gap-2 md:flex-row md:items-end md:justify-between">
        <div>
          <p className="text-muted-foreground text-xs font-medium tracking-[0.18em] uppercase">
            Entitlements center
          </p>
          <h2 className="text-base font-semibold">Plans, features, quotas et droits produit</h2>
        </div>
        <Badge variant={(data?.over_quota_balance_count ?? 0) > 0 ? 'destructive' : 'secondary'}>
          {formatCount(data?.over_quota_balance_count)} over quota
        </Badge>
      </div>
      {disabled ? (
        <LockedState label="Connecte un contexte operateur pour charger les signaux entitlements." />
      ) : null}
      <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-7">
        <EntitlementMetric
          icon={PackageCheck}
          label="Active plans"
          value={formatCount(data?.active_plan_count)}
        />
        <EntitlementMetric
          icon={Sparkles}
          label="Features"
          value={formatCount(data?.active_feature_count)}
        />
        <EntitlementMetric
          icon={Gauge}
          label="Quota defs"
          value={formatCount(data?.quota_definition_count)}
        />
        <EntitlementMetric
          icon={PackageCheck}
          label="Active rights"
          value={formatCount(data?.active_entitlement_count)}
        />
        <EntitlementMetric
          icon={AlertTriangle}
          label="Over quota"
          tone={(data?.over_quota_balance_count ?? 0) > 0 ? 'danger' : 'default'}
          value={formatCount(data?.over_quota_balance_count)}
        />
        <EntitlementMetric
          icon={ScrollText}
          label="Unpublished"
          tone={(data?.unpublished_change_count ?? 0) > 0 ? 'warning' : 'default'}
          value={formatCount(data?.unpublished_change_count)}
        />
        <EntitlementMetric
          icon={Sparkles}
          label="Trials active"
          value={formatCount(data?.active_trial_grant_count)}
        />
      </div>
      <EntitlementsActionsPanel credentials={credentials} disabled={disabled} />
      <div className="mt-4 grid gap-3 xl:grid-cols-2">
        <PlanList rows={data?.active_plans ?? []} />
        <OverQuotaList
          onSelectTenant={onSelectTenant}
          onSelectWorkspace={onSelectWorkspace}
          rows={data?.over_quota_balances ?? []}
        />
        <ExpiringEntitlementList
          onSelectTenant={onSelectTenant}
          onSelectWorkspace={onSelectWorkspace}
          rows={data?.expiring_entitlements ?? []}
        />
        <UnpublishedChangeList
          onSelectTenant={onSelectTenant}
          rows={data?.unpublished_changes ?? []}
        />
      </div>
    </section>
  );
}

function EntitlementMetric({
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
