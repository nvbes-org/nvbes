import { useQuery } from '@tanstack/react-query';
import { CreditCard, FileText, Globe2, Route, ShieldCheck, Shuffle } from 'lucide-react';
import type { ComponentType } from 'react';
import { Badge } from '@/components/ui/badge';
import { getBillingPlatformCenter } from './backoffice-service.api';
import { BillingPlatformActionsPanel } from './backoffice-service.billing-platform-actions';
import {
  EinvoicingList,
  KycList,
  MigrationList,
  ProviderList,
  RegionPolicyList,
  RoutingRuleList,
} from './backoffice-service.billing-platform-lists';
import { LockedState } from './backoffice-service.locked-state';
import type { AdminCredentials } from './backoffice-service.types';

export function BillingPlatformCenterPanel({
  credentials,
  disabled,
  onSelectTenant,
}: {
  credentials: AdminCredentials;
  disabled: boolean;
  onSelectTenant: (tenantId: string) => void;
}) {
  const platform = useQuery({
    queryKey: ['billing-platform-center'],
    queryFn: () => getBillingPlatformCenter(credentials),
    enabled: !disabled,
  });
  const data = platform.data;

  return (
    <section
      className="border-border bg-card mb-5 rounded-lg border p-4"
      id="billing-platform-center"
    >
      <div className="mb-4 flex flex-col gap-2 md:flex-row md:items-end md:justify-between">
        <div>
          <p className="text-muted-foreground text-xs font-medium tracking-[0.18em] uppercase">
            Billing platform
          </p>
          <h2 className="text-base font-semibold">Providers, KYC, routing et e-invoicing</h2>
        </div>
        <Badge variant={(data?.planned_migration_count ?? 0) > 0 ? 'destructive' : 'secondary'}>
          {formatCount(data?.planned_migration_count)} migrations
        </Badge>
      </div>
      {disabled ? (
        <LockedState label="Connecte un contexte operateur pour charger les signaux billing platform." />
      ) : null}
      <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-8">
        <PlatformMetric
          icon={CreditCard}
          label="Providers"
          value={formatCount(data?.active_provider_count)}
        />
        <PlatformMetric
          icon={CreditCard}
          label="Accounts"
          value={formatCount(data?.active_provider_account_count)}
        />
        <PlatformMetric
          icon={Route}
          label="Routing rules"
          value={formatCount(data?.active_routing_rule_count)}
        />
        <PlatformMetric
          icon={Shuffle}
          label="Fallback rules"
          tone={(data?.fallback_routing_rule_count ?? 0) > 0 ? 'warning' : 'default'}
          value={formatCount(data?.fallback_routing_rule_count)}
        />
        <PlatformMetric
          icon={Shuffle}
          label="Migrations"
          tone={(data?.planned_migration_count ?? 0) > 0 ? 'danger' : 'default'}
          value={formatCount(data?.planned_migration_count)}
        />
        <PlatformMetric
          icon={ShieldCheck}
          label="KYC pending"
          tone={(data?.pending_kyc_profile_count ?? 0) > 0 ? 'warning' : 'default'}
          value={formatCount(data?.pending_kyc_profile_count)}
        />
        <PlatformMetric
          icon={Globe2}
          label="Region policies"
          value={formatCount(data?.region_policy_count)}
        />
        <PlatformMetric
          icon={FileText}
          label="E-invoicing"
          value={formatCount(data?.active_einvoicing_profile_count)}
        />
      </div>
      <div className="mt-4 grid gap-3 xl:grid-cols-2">
        <ProviderList rows={data?.providers ?? []} />
        <RoutingRuleList rows={data?.routing_rules ?? []} />
        <MigrationList onSelectTenant={onSelectTenant} rows={data?.provider_migrations ?? []} />
        <KycList onSelectTenant={onSelectTenant} rows={data?.kyc_profiles ?? []} />
        <RegionPolicyList rows={data?.region_policies ?? []} />
        <EinvoicingList rows={data?.einvoicing_profiles ?? []} />
      </div>
      <BillingPlatformActionsPanel credentials={credentials} disabled={disabled} />
    </section>
  );
}

function PlatformMetric({
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
