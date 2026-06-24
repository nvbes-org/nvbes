import { useQuery } from '@tanstack/react-query';
import { Building2, CreditCard, FileText, Globe2, Route, ShieldCheck, Shuffle } from 'lucide-react';
import type { ComponentType, ReactNode } from 'react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { getBillingPlatformCenter } from './internal-admin.api';
import { BillingPlatformActionsPanel } from './internal-admin.billing-platform-actions';
import { LockedState } from './internal-admin.locked-state';
import type {
  AdminCredentials,
  BillingProviderSummary,
  BillingRegionPolicy,
  EinvoicingProfile,
  KycProfile,
  ProviderMigrationRun,
  ProviderRoutingRule,
} from './internal-admin.types';

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

function ProviderList({ rows }: { rows: BillingProviderSummary[] }) {
  return (
    <PlatformList emptyLabel="Aucun provider billing." title="Providers">
      {rows.map((row) => (
        <CompactRow
          badge={`${formatCount(row.account_count)} accounts`}
          key={row.provider}
          label={row.provider}
          meta={row.status}
        />
      ))}
    </PlatformList>
  );
}

function RoutingRuleList({ rows }: { rows: ProviderRoutingRule[] }) {
  return (
    <PlatformList emptyLabel="Aucune routing rule." title="Routing rules">
      {rows.map((row) => (
        <CompactRow
          badge={row.fallback_enabled ? 'fallback' : row.status}
          key={row.id}
          label={`${row.priority} - ${row.provider}`}
          meta={`${row.country ?? 'any country'} / ${row.currency ?? 'any currency'} / ${row.payment_method ?? 'any method'} - ${shortId(row.id)}`}
          tone={row.fallback_enabled ? 'warning' : 'default'}
        />
      ))}
    </PlatformList>
  );
}

function MigrationList({
  onSelectTenant,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  rows: ProviderMigrationRun[];
}) {
  return (
    <PlatformList emptyLabel="Aucune migration provider." title="Provider migrations">
      {rows.map((row) => (
        <LinkedTenantRow
          badge={row.status}
          key={row.id}
          onSelectTenant={() => onSelectTenant(row.tenant_id)}
          subtitle={`${row.tenant_name} - ${row.from_provider} to ${row.to_provider}`}
          title={formatDate(row.updated_at)}
          tone={row.status === 'failed' ? 'danger' : 'warning'}
        />
      ))}
    </PlatformList>
  );
}

function KycList({
  onSelectTenant,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  rows: KycProfile[];
}) {
  return (
    <PlatformList emptyLabel="Aucun profil KYC." title="KYC profiles">
      {rows.map((row) => (
        <LinkedTenantRow
          badge={row.review_status}
          key={row.id}
          onSelectTenant={() => onSelectTenant(row.tenant_id)}
          subtitle={`${row.tenant_name} - ${row.company_domain ?? row.vat_id ?? 'no company metadata'} - ${shortId(row.id)}`}
          title={row.company_name ?? row.tenant_name}
          tone={row.review_status === 'approved' ? 'default' : 'warning'}
        />
      ))}
    </PlatformList>
  );
}

function RegionPolicyList({ rows }: { rows: BillingRegionPolicy[] }) {
  return (
    <PlatformList emptyLabel="Aucune politique region." title="Region policies">
      {rows.map((row) => (
        <CompactRow
          badge={row.tax_evidence_required ? 'tax evidence' : 'no tax evidence'}
          key={row.id}
          label={`${row.country} / ${row.currency}`}
          meta={`${formatCount(row.invoice_retention_years)}y retention - ${row.allowed_payment_methods.join(', ') || 'no methods'}`}
        />
      ))}
    </PlatformList>
  );
}

function EinvoicingList({ rows }: { rows: EinvoicingProfile[] }) {
  return (
    <PlatformList emptyLabel="Aucun profil e-invoicing." title="E-invoicing profiles">
      {rows.map((row) => (
        <CompactRow
          badge={row.status}
          key={row.id}
          label={row.code}
          meta={`${row.country ?? 'global'} - ${row.format} - ${shortId(row.id)}`}
        />
      ))}
    </PlatformList>
  );
}

function CompactRow({
  badge,
  label,
  meta,
  tone = 'default',
}: {
  badge: string;
  label: string;
  meta: string;
  tone?: 'default' | 'warning';
}) {
  return (
    <div className="flex items-center justify-between gap-3 p-3">
      <div className="min-w-0">
        <p className="truncate text-sm font-medium">{label}</p>
        <p className="text-muted-foreground truncate text-xs">{meta}</p>
      </div>
      <Badge variant={tone === 'warning' ? 'outline' : 'secondary'}>{badge}</Badge>
    </div>
  );
}

function LinkedTenantRow({
  badge,
  onSelectTenant,
  subtitle,
  title,
  tone = 'default',
}: {
  badge: string;
  onSelectTenant: () => void;
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
      <Button
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
    </article>
  );
}

function PlatformList({
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

function formatDate(value: string): string {
  return new Intl.DateTimeFormat('fr-FR').format(new Date(value));
}

function shortId(value: string): string {
  return value.slice(0, 8);
}
