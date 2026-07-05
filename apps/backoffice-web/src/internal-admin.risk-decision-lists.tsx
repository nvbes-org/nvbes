import { Building2, FileSearch, UserRound } from 'lucide-react';
import type { ReactNode } from 'react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import type {
  AccessPolicySnapshot,
  BillingRiskScore,
  BillingRiskSignal,
  IdentityRiskDecision,
} from './internal-admin.types';

export function IdentityRiskList({
  onSelectTenant,
  onSelectUser,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  onSelectUser: (principalId: string) => void;
  rows: IdentityRiskDecision[];
}) {
  return (
    <RiskList
      emptyLabel="Aucun risk event identity. Les decisions MFA, blocage et step-up apparaitront ici."
      title="Identity risk decisions"
    >
      {rows.map((row) => (
        <RiskActionRow
          auditTargetId={row.id}
          auditTargetType="identity_risk_decision"
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

export function BillingScoreList({
  onSelectTenant,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  rows: BillingRiskScore[];
}) {
  return (
    <RiskList
      emptyLabel="Aucun billing risk score. Les scores fraude paiement et decisions associées seront listes ici."
      title="Billing risk scores"
    >
      {rows.map((row) => (
        <RiskActionRow
          auditTargetId={row.id}
          auditTargetType="billing_risk_score"
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

export function BillingSignalList({
  onSelectTenant,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  rows: BillingRiskSignal[];
}) {
  return (
    <RiskList
      emptyLabel="Aucun billing risk signal. Les signaux provider, fraude et dispute seront consolides ici."
      title="Billing risk signals"
    >
      {rows.map((row) => (
        <RiskActionRow
          auditTargetId={row.id}
          auditTargetType="billing_risk_signal"
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

export function AccessPolicyList({
  onSelectTenant,
  onSelectWorkspace,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  onSelectWorkspace: (workspaceId: string) => void;
  rows: AccessPolicySnapshot[];
}) {
  return (
    <RiskList
      emptyLabel="Aucune policy access active. Les restrictions tenant/workspace en vigueur seront visibles ici."
      title="Active access policies"
    >
      {rows.map((row) => (
        <RiskActionRow
          auditTargetId={row.id}
          auditTargetType="access_policy"
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
  auditTargetId,
  auditTargetType,
  badge,
  disableTenant = false,
  onSelectTenant,
  onSelectUser,
  onSelectWorkspace,
  subtitle,
  title,
  tone = 'default',
}: {
  auditTargetId: string;
  auditTargetType: string;
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
        <AuditButton targetId={auditTargetId} targetType={auditTargetType} />
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

function AuditButton({ targetId, targetType }: { targetId: string; targetType: string }) {
  return (
    <Button
      onClick={() => {
        window.location.hash = `audit?target_type=${targetType}&q=${encodeURIComponent(targetId)}`;
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

function formatRiskScore(value: number): string {
  return new Intl.NumberFormat('fr-FR', { maximumFractionDigits: 2 }).format(value);
}
