import { Building2, FileSearch } from 'lucide-react';
import type { ReactNode } from 'react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import type {
  BillingProviderSummary,
  BillingRegionPolicy,
  EinvoicingProfile,
  KycProfile,
  ProviderMigrationRun,
  ProviderRoutingRule,
} from './backoffice-service.types';

export function ProviderList({ rows }: { rows: BillingProviderSummary[] }) {
  return (
    <PlatformList
      emptyLabel="Aucun provider billing configure. Les comptes actifs et anomalies provider apparaitront ici."
      title="Providers"
    >
      {rows.map((row) => (
        <CompactRow
          badge={`${formatCount(row.account_count)} accounts`}
          key={row.provider}
          label={row.provider}
          meta={row.status}
          targetId={row.provider}
          targetType="billing_provider"
        />
      ))}
    </PlatformList>
  );
}

export function RoutingRuleList({ rows }: { rows: ProviderRoutingRule[] }) {
  return (
    <PlatformList
      emptyLabel="Aucune routing rule active. Les priorites provider, fallback et devises seront listees ici."
      title="Routing rules"
    >
      {rows.map((row) => (
        <CompactRow
          badge={row.fallback_enabled ? 'fallback' : row.status}
          key={row.id}
          label={`${row.priority} - ${row.provider}`}
          meta={`${row.country ?? 'any country'} / ${row.currency ?? 'any currency'} / ${row.payment_method ?? 'any method'} - ${shortId(row.id)}`}
          targetId={row.id}
          targetType="billing_provider_routing_rule"
          tone={row.fallback_enabled ? 'warning' : 'default'}
        />
      ))}
    </PlatformList>
  );
}

export function MigrationList({
  onSelectTenant,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  rows: ProviderMigrationRun[];
}) {
  return (
    <PlatformList
      emptyLabel="Aucune migration provider planifiee. Les bascules PSP et reprises interrompues seront visibles ici."
      title="Provider migrations"
    >
      {rows.map((row) => (
        <LinkedTenantRow
          badge={row.status}
          key={row.id}
          onSelectTenant={() => onSelectTenant(row.tenant_id)}
          subtitle={`${row.tenant_name} - ${row.from_provider} to ${row.to_provider}`}
          targetId={row.id}
          targetType="billing_provider_migration"
          title={formatDate(row.updated_at)}
          tone={row.status === 'failed' ? 'danger' : 'warning'}
        />
      ))}
    </PlatformList>
  );
}

export function KycList({
  onSelectTenant,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  rows: KycProfile[];
}) {
  return (
    <PlatformList
      emptyLabel="Aucun profil KYC en revue. Les dossiers bloquants et preuves d'entreprise seront listes ici."
      title="KYC profiles"
    >
      {rows.map((row) => (
        <LinkedTenantRow
          badge={row.review_status}
          key={row.id}
          onSelectTenant={() => onSelectTenant(row.tenant_id)}
          subtitle={`${row.tenant_name} - ${row.company_domain ?? row.vat_id ?? 'no company metadata'} - ${shortId(row.id)}`}
          targetId={row.id}
          targetType="billing_kyc_profile"
          title={row.company_name ?? row.tenant_name}
          tone={row.review_status === 'approved' ? 'default' : 'warning'}
        />
      ))}
    </PlatformList>
  );
}

export function RegionPolicyList({ rows }: { rows: BillingRegionPolicy[] }) {
  return (
    <PlatformList
      emptyLabel="Aucune politique region billing. Les exigences taxe, retention et moyens de paiement seront listees ici."
      title="Region policies"
    >
      {rows.map((row) => (
        <CompactRow
          badge={row.tax_evidence_required ? 'tax evidence' : 'no tax evidence'}
          key={row.id}
          label={`${row.country} / ${row.currency}`}
          meta={`${formatCount(row.invoice_retention_years)}y retention - ${row.allowed_payment_methods.join(', ') || 'no methods'}`}
          targetId={row.id}
          targetType="billing_region_policy"
        />
      ))}
    </PlatformList>
  );
}

export function EinvoicingList({ rows }: { rows: EinvoicingProfile[] }) {
  return (
    <PlatformList
      emptyLabel="Aucun profil e-invoicing actif. Les formats par pays et leurs statuts apparaitront ici."
      title="E-invoicing profiles"
    >
      {rows.map((row) => (
        <CompactRow
          badge={row.status}
          key={row.id}
          label={row.code}
          meta={`${row.country ?? 'global'} - ${row.format} - ${shortId(row.id)}`}
          targetId={row.id}
          targetType="billing_einvoicing_profile"
        />
      ))}
    </PlatformList>
  );
}

function CompactRow({
  badge,
  label,
  meta,
  targetId,
  targetType,
  tone = 'default',
}: {
  badge: string;
  label: string;
  meta: string;
  targetId: string;
  targetType: string;
  tone?: 'default' | 'warning';
}) {
  return (
    <article className="p-3">
      <div className="mb-2 flex items-center justify-between gap-3">
        <div className="min-w-0">
          <p className="truncate text-sm font-medium">{label}</p>
          <p className="text-muted-foreground truncate text-xs">{meta}</p>
        </div>
        <Badge variant={tone === 'warning' ? 'outline' : 'secondary'}>{badge}</Badge>
      </div>
      <AuditButton targetId={targetId} targetType={targetType} />
    </article>
  );
}

function LinkedTenantRow({
  badge,
  onSelectTenant,
  subtitle,
  targetId,
  targetType,
  title,
  tone = 'default',
}: {
  badge: string;
  onSelectTenant: () => void;
  subtitle: string;
  targetId: string;
  targetType: string;
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
          onClick={() => {
            onSelectTenant();
            window.location.hash = 'tenant-detail';
          }}
          size="sm"
          type="button"
          variant="outline"
        >
          <Building2 className="size-4" />
          Open tenant
        </Button>
        <AuditButton targetId={targetId} targetType={targetType} />
      </div>
    </article>
  );
}

function AuditButton({ targetId, targetType }: { targetId: string; targetType: string }) {
  return (
    <Button
      onClick={() => {
        window.location.hash = `audit?target_type=${targetType}&q=${targetId}`;
      }}
      size="sm"
      type="button"
      variant="ghost"
    >
      <FileSearch className="size-4" />
      Audit
    </Button>
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
  return (
    <div className="text-muted-foreground bg-muted/20 m-3 rounded-md border p-3 text-sm">
      {label}
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
