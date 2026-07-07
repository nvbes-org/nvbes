import { useQuery } from '@tanstack/react-query';
import {
  Building2,
  Fingerprint,
  IdCard,
  KeyRound,
  ShieldAlert,
  ShieldCheck,
  UserRound,
  UsersRound,
} from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { getIdentityGovernanceCenter } from './backoffice-service.api';
import { IdentityGovernanceActionLists } from './backoffice-service.identity-governance-actions';
import {
  GovernanceList,
  GovernanceMetric,
  LinkedTenantRow,
} from './backoffice-service.identity-governance-list';
import { LockedState } from './backoffice-service.locked-state';
import { OperatorGrantsPanel } from './backoffice-service.operator-grants';
import type {
  AdminCredentials,
  OverdueAccessReview,
  ScimConnector,
  SsoProvider,
  UnverifiedDomain,
} from './backoffice-service.types';

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
      <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-8">
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
        <GovernanceMetric
          icon={ShieldCheck}
          label="Operator grants"
          tone={(data?.revoked_operator_grant_count ?? 0) > 0 ? 'warning' : 'default'}
          value={formatCount(data?.active_operator_grant_count)}
        />
      </div>
      <div className="mt-4 grid gap-3 xl:grid-cols-2">
        <OperatorGrantsPanel
          credentials={credentials}
          disabled={disabled}
          grants={data?.operator_grants ?? []}
          roleDistribution={data?.operator_role_distribution ?? []}
        />
        <DomainList onSelectTenant={onSelectTenant} rows={data?.unverified_domains ?? []} />
        <SsoList onSelectTenant={onSelectTenant} rows={data?.sso_providers ?? []} />
        <ScimList onSelectTenant={onSelectTenant} rows={data?.scim_connectors ?? []} />
        <AccessReviewList
          onSelectTenant={onSelectTenant}
          rows={data?.overdue_access_reviews ?? []}
        />
        <IdentityGovernanceActionLists
          breakGlassAccounts={data?.break_glass_accounts ?? []}
          credentials={credentials}
          disabled={disabled}
          onSelectTenant={onSelectTenant}
          onSelectUser={onSelectUser}
          recoveryRequests={data?.pending_recovery_requests ?? []}
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

function formatCount(value: number | undefined): string {
  return typeof value === 'number' ? new Intl.NumberFormat('fr-FR').format(value) : '-';
}

function formatDate(value: string): string {
  return new Intl.DateTimeFormat('fr-FR').format(new Date(value));
}
