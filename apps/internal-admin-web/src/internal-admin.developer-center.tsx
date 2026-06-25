import { useQuery } from '@tanstack/react-query';
import { AlertTriangle, Code2, KeyRound, RadioTower, ShieldAlert, Store } from 'lucide-react';
import type { ComponentType } from 'react';
import { Badge } from '@/components/ui/badge';
import { getDeveloperCenter } from './internal-admin.api';
import { DeveloperActionsPanel } from './internal-admin.developer-actions';
import {
  ExpiringSecretList,
  HealthIssueList,
  MarketplaceList,
  RiskyScopeList,
  WebhookFailureList,
} from './internal-admin.developer-lists';
import { LockedState } from './internal-admin.locked-state';
import type { AdminCredentials } from './internal-admin.types';

export function DeveloperCenterPanel({
  credentials,
  disabled,
  onSelectTenant,
}: {
  credentials: AdminCredentials;
  disabled: boolean;
  onSelectTenant: (tenantId: string) => void;
}) {
  const developer = useQuery({
    queryKey: ['developer-center'],
    queryFn: () => getDeveloperCenter(credentials),
    enabled: !disabled,
  });
  const data = developer.data;

  return (
    <section className="border-border bg-card mb-5 rounded-lg border p-4" id="developer-center">
      <div className="mb-4 flex flex-col gap-2 md:flex-row md:items-end md:justify-between">
        <div>
          <p className="text-muted-foreground text-xs font-medium tracking-[0.18em] uppercase">
            Developer center
          </p>
          <h2 className="text-base font-semibold">Apps, webhooks, scopes et secrets</h2>
        </div>
        <Badge
          variant={(data?.failed_webhook_delivery_count_24h ?? 0) > 0 ? 'destructive' : 'secondary'}
        >
          {formatCount(data?.failed_webhook_delivery_count_24h)} webhook failures 24h
        </Badge>
      </div>
      {disabled ? (
        <LockedState label="Connecte un contexte operateur pour charger les signaux developer." />
      ) : null}
      <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-7">
        <DeveloperMetric
          icon={Code2}
          label="Active clients"
          value={formatCount(data?.active_client_count)}
        />
        <DeveloperMetric
          icon={Store}
          label="Marketplace pending"
          tone={(data?.pending_marketplace_app_count ?? 0) > 0 ? 'warning' : 'default'}
          value={formatCount(data?.pending_marketplace_app_count)}
        />
        <DeveloperMetric
          icon={RadioTower}
          label="Webhook failures"
          tone={(data?.failed_webhook_delivery_count_24h ?? 0) > 0 ? 'danger' : 'default'}
          value={formatCount(data?.failed_webhook_delivery_count_24h)}
        />
        <DeveloperMetric
          icon={RadioTower}
          label="Active endpoints"
          value={formatCount(data?.active_webhook_endpoint_count)}
        />
        <DeveloperMetric
          icon={KeyRound}
          label="Secrets expiring"
          tone={(data?.expiring_secret_count ?? 0) > 0 ? 'danger' : 'default'}
          value={formatCount(data?.expiring_secret_count)}
        />
        <DeveloperMetric
          icon={ShieldAlert}
          label="Restricted scopes"
          tone={(data?.restricted_scope_count ?? 0) > 0 ? 'danger' : 'default'}
          value={formatCount(data?.restricted_scope_count)}
        />
        <DeveloperMetric
          icon={AlertTriangle}
          label="Health issues"
          tone={(data?.failing_health_check_count ?? 0) > 0 ? 'warning' : 'default'}
          value={formatCount(data?.failing_health_check_count)}
        />
      </div>
      <DeveloperActionsPanel credentials={credentials} disabled={disabled} />
      <div className="mt-4 grid gap-3 xl:grid-cols-2">
        <MarketplaceList
          onSelectTenant={onSelectTenant}
          rows={data?.pending_marketplace_apps ?? []}
        />
        <WebhookFailureList onSelectTenant={onSelectTenant} rows={data?.webhook_failures ?? []} />
        <ExpiringSecretList onSelectTenant={onSelectTenant} rows={data?.expiring_secrets ?? []} />
        <RiskyScopeList rows={data?.risky_scopes ?? []} />
        <HealthIssueList onSelectTenant={onSelectTenant} rows={data?.health_issues ?? []} />
      </div>
    </section>
  );
}

function DeveloperMetric({
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
