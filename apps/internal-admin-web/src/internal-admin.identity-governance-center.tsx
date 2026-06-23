import { useQuery } from '@tanstack/react-query';
import {
  Building2,
  Fingerprint,
  IdCard,
  KeyRound,
  ShieldAlert,
  UserRound,
  UsersRound,
} from 'lucide-react';
import type { ComponentType, ReactNode } from 'react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { getIdentityGovernanceCenter } from './internal-admin.api';
import { LockedState } from './internal-admin.locked-state';
import type {
  AdminCredentials,
  BreakGlassAccount,
  OverdueAccessReview,
  PendingRecoveryRequest,
  ScimConnector,
  SsoProvider,
  UnverifiedDomain,
} from './internal-admin.types';

export function IdentityGovernanceCenterPanel({
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
  const governance = useQuery({
    queryKey: ['identity-governance-center'],
    queryFn: () => getIdentityGovernanceCenter(credentials),
    enabled: !disabled,
  });
  const data = governance.data;

  return (
    <section
      className="border-border bg-card mb-5 rounded-lg border p-4"
      id="identity-governance-center"
    >
      <div className="mb-4 flex flex-col gap-2 md:flex-row md:items-end md:justify-between">
        <div>
          <p className="text-muted-foreground text-xs font-medium tracking-[0.18em] uppercase">
            Identity governance
          </p>
          <h2 className="text-base font-semibold">SSO, SCIM, access reviews et break-glass</h2>
        </div>
        <Badge variant={(data?.overdue_access_review_count ?? 0) > 0 ? 'destructive' : 'secondary'}>
          {formatCount(data?.overdue_access_review_count)} reviews overdue
        </Badge>
      </div>
      {disabled ? (
        <LockedState label="Connecte un contexte operateur pour charger les signaux governance." />
      ) : null}
      <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-7">
        <GovernanceMetric
          icon={Fingerprint}
          label="Active IdP"
          value={formatCount(data?.active_idp_count)}
        />
        <GovernanceMetric
          icon={Building2}
          label="Unverified domains"
          tone={(data?.unverified_domain_count ?? 0) > 0 ? 'warning' : 'default'}
          value={formatCount(data?.unverified_domain_count)}
        />
        <GovernanceMetric
          icon={UsersRound}
          label="SCIM connectors"
          value={formatCount(data?.active_scim_connector_count)}
        />
        <GovernanceMetric
          icon={ShieldAlert}
          label="Overdue reviews"
          tone={(data?.overdue_access_review_count ?? 0) > 0 ? 'danger' : 'default'}
          value={formatCount(data?.overdue_access_review_count)}
        />
        <GovernanceMetric
          icon={IdCard}
          label="Pending items"
          tone={(data?.pending_review_item_count ?? 0) > 0 ? 'warning' : 'default'}
          value={formatCount(data?.pending_review_item_count)}
        />
        <GovernanceMetric
          icon={KeyRound}
          label="Break-glass"
          tone={(data?.active_break_glass_count ?? 0) > 0 ? 'danger' : 'default'}
          value={formatCount(data?.active_break_glass_count)}
        />
        <GovernanceMetric
          icon={UserRound}
          label="Recovery pending"
          tone={(data?.pending_recovery_count ?? 0) > 0 ? 'warning' : 'default'}
          value={formatCount(data?.pending_recovery_count)}
        />
      </div>
      <div className="mt-4 grid gap-3 xl:grid-cols-2">
        <DomainList onSelectTenant={onSelectTenant} rows={data?.unverified_domains ?? []} />
        <SsoList onSelectTenant={onSelectTenant} rows={data?.sso_providers ?? []} />
        <ScimList onSelectTenant={onSelectTenant} rows={data?.scim_connectors ?? []} />
        <AccessReviewList
          onSelectTenant={onSelectTenant}
          rows={data?.overdue_access_reviews ?? []}
        />
        <BreakGlassList
          onSelectTenant={onSelectTenant}
          onSelectUser={onSelectUser}
          rows={data?.break_glass_accounts ?? []}
        />
        <RecoveryList
          onSelectTenant={onSelectTenant}
          onSelectUser={onSelectUser}
          rows={data?.pending_recovery_requests ?? []}
        />
      </div>
    </section>
  );
}

function DomainList({
  onSelectTenant,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  rows: UnverifiedDomain[];
}) {
  return (
    <GovernanceList emptyLabel="Aucun domaine non verifie." title="Unverified domains">
      {rows.map((row) => (
        <LinkedTenantRow
          badge={
            row.verification_expires_at ? formatDate(row.verification_expires_at) : 'no expiry'
          }
          key={`${row.tenant_id}:${row.domain}`}
          onSelectTenant={() => onSelectTenant(row.tenant_id)}
          subtitle={row.tenant_name}
          title={row.domain}
          tone="warning"
        />
      ))}
    </GovernanceList>
  );
}

function SsoList({
  onSelectTenant,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  rows: SsoProvider[];
}) {
  return (
    <GovernanceList emptyLabel="Aucun fournisseur SSO." title="SSO providers">
      {rows.map((row) => (
        <LinkedTenantRow
          badge={row.status}
          key={row.id}
          onSelectTenant={() => onSelectTenant(row.tenant_id)}
          subtitle={`${row.tenant_name} - ${row.provider_type} - signed ${row.require_signed_assertions ? 'required' : 'off'}`}
          title={row.name}
        />
      ))}
    </GovernanceList>
  );
}

function ScimList({
  onSelectTenant,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  rows: ScimConnector[];
}) {
  return (
    <GovernanceList emptyLabel="Aucun connecteur SCIM." title="SCIM connectors">
      {rows.map((row) => (
        <LinkedTenantRow
          badge={row.status}
          key={row.id}
          onSelectTenant={() => onSelectTenant(row.tenant_id)}
          subtitle={`${row.tenant_name} - ${row.base_url ?? 'no base url'}`}
          title={row.provider}
        />
      ))}
    </GovernanceList>
  );
}

function AccessReviewList({
  onSelectTenant,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  rows: OverdueAccessReview[];
}) {
  return (
    <GovernanceList emptyLabel="Aucune review en retard." title="Overdue access reviews">
      {rows.map((row) => (
        <LinkedTenantRow
          badge={`${formatCount(row.pending_item_count)} pending`}
          key={row.id}
          onSelectTenant={() => onSelectTenant(row.tenant_id)}
          subtitle={`${row.tenant_name} - due ${formatDate(row.due_at)}`}
          title={row.name}
          tone="danger"
        />
      ))}
    </GovernanceList>
  );
}

function BreakGlassList({
  onSelectTenant,
  onSelectUser,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  onSelectUser: (principalId: string) => void;
  rows: BreakGlassAccount[];
}) {
  return (
    <GovernanceList emptyLabel="Aucun compte break-glass actif." title="Break-glass accounts">
      {rows.map((row) => (
        <LinkedTenantRow
          badge={row.last_used_at ? `used ${formatDate(row.last_used_at)}` : 'never used'}
          key={`${row.tenant_id}:${row.principal_id}`}
          onSelectTenant={() => onSelectTenant(row.tenant_id)}
          onSelectUser={() => onSelectUser(row.principal_id)}
          subtitle={`${row.tenant_name} - ${row.procedure_reference}`}
          title={row.reason}
          tone="danger"
        />
      ))}
    </GovernanceList>
  );
}

function RecoveryList({
  onSelectTenant,
  onSelectUser,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  onSelectUser: (principalId: string) => void;
  rows: PendingRecoveryRequest[];
}) {
  return (
    <GovernanceList emptyLabel="Aucune recovery request pending." title="Password recovery queue">
      {rows.map((row) => (
        <LinkedTenantRow
          badge={row.status}
          key={row.id}
          onSelectTenant={() => onSelectTenant(row.tenant_id)}
          onSelectUser={() => onSelectUser(row.principal_id)}
          subtitle={`${row.tenant_name} - available ${formatDate(row.available_at)}`}
          title={row.email}
          tone="warning"
        />
      ))}
    </GovernanceList>
  );
}

function LinkedTenantRow({
  badge,
  onSelectTenant,
  onSelectUser,
  subtitle,
  title,
  tone = 'default',
}: {
  badge: string;
  onSelectTenant: () => void;
  onSelectUser?: () => void;
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
      </div>
    </article>
  );
}

function GovernanceList({
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

function GovernanceMetric({
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
