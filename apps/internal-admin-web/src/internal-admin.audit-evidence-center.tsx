import { useQuery } from '@tanstack/react-query';
import {
  AlertTriangle,
  Building2,
  FileCheck2,
  KeyRound,
  ShieldCheck,
  UserRound,
} from 'lucide-react';
import type { ComponentType, ReactNode } from 'react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { getAuditEvidenceCenter } from './internal-admin.api';
import { LockedState } from './internal-admin.locked-state';
import type {
  AdminCredentials,
  AuditEvidenceEvent,
  AuditHashAnomaly,
  SigningKeySummary,
} from './internal-admin.types';

export function AuditEvidenceCenterPanel({
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
  const evidence = useQuery({
    queryKey: ['audit-evidence-center'],
    queryFn: () => getAuditEvidenceCenter(credentials),
    enabled: !disabled,
  });
  const data = evidence.data;
  const hashIssueCount = (data?.missing_hash_count ?? 0) + (data?.backfilled_hash_count ?? 0);

  return (
    <section
      className="border-border bg-card mb-5 rounded-lg border p-4"
      id="audit-evidence-center"
    >
      <div className="mb-4 flex flex-col gap-2 md:flex-row md:items-end md:justify-between">
        <div>
          <p className="text-muted-foreground text-xs font-medium tracking-[0.18em] uppercase">
            Audit evidence
          </p>
          <h2 className="text-base font-semibold">Audit immutable, actions sensibles et cles</h2>
        </div>
        <Badge variant={hashIssueCount > 0 ? 'destructive' : 'secondary'}>
          {formatCount(hashIssueCount)} hash issues
        </Badge>
      </div>
      {disabled ? (
        <LockedState label="Connecte un contexte operateur pour charger les preuves audit." />
      ) : null}
      <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-8">
        <EvidenceMetric
          icon={FileCheck2}
          label="Audit 24h"
          value={formatCount(data?.audit_events_24h)}
        />
        <EvidenceMetric
          icon={UserRound}
          label="Actorless 24h"
          tone={(data?.actorless_event_count_24h ?? 0) > 0 ? 'warning' : 'default'}
          value={formatCount(data?.actorless_event_count_24h)}
        />
        <EvidenceMetric
          icon={AlertTriangle}
          label="Sensitive 24h"
          tone={(data?.sensitive_action_count_24h ?? 0) > 0 ? 'warning' : 'default'}
          value={formatCount(data?.sensitive_action_count_24h)}
        />
        <EvidenceMetric
          icon={ShieldCheck}
          label="Missing hash"
          tone={(data?.missing_hash_count ?? 0) > 0 ? 'danger' : 'default'}
          value={formatCount(data?.missing_hash_count)}
        />
        <EvidenceMetric
          icon={ShieldCheck}
          label="Backfilled"
          tone={(data?.backfilled_hash_count ?? 0) > 0 ? 'danger' : 'default'}
          value={formatCount(data?.backfilled_hash_count)}
        />
        <EvidenceMetric
          icon={KeyRound}
          label="Active keys"
          value={formatCount(data?.active_signing_key_count)}
        />
        <EvidenceMetric
          icon={KeyRound}
          label="Deprecated"
          tone={(data?.deprecated_signing_key_count ?? 0) > 0 ? 'warning' : 'default'}
          value={formatCount(data?.deprecated_signing_key_count)}
        />
        <EvidenceMetric
          icon={KeyRound}
          label="Revoked"
          tone={(data?.revoked_signing_key_count ?? 0) > 0 ? 'danger' : 'default'}
          value={formatCount(data?.revoked_signing_key_count)}
        />
      </div>
      <div className="mt-4 grid gap-3 xl:grid-cols-2">
        <AuditEventList
          onSelectTenant={onSelectTenant}
          onSelectUser={onSelectUser}
          rows={data?.recent_audit_events ?? []}
          title="Recent audit events"
        />
        <AuditEventList
          emptyLabel="Aucun evenement sans acteur."
          onSelectTenant={onSelectTenant}
          onSelectUser={onSelectUser}
          rows={data?.actorless_events ?? []}
          title="Actorless events"
        />
        <HashAnomalyList onSelectTenant={onSelectTenant} rows={data?.hash_anomalies ?? []} />
        <SigningKeyList rows={data?.signing_keys ?? []} />
      </div>
    </section>
  );
}

function AuditEventList({
  emptyLabel = 'Aucun evenement audit.',
  onSelectTenant,
  onSelectUser,
  rows,
  title,
}: {
  emptyLabel?: string;
  onSelectTenant: (tenantId: string) => void;
  onSelectUser: (principalId: string) => void;
  rows: AuditEvidenceEvent[];
  title: string;
}) {
  return (
    <EvidenceList emptyLabel={emptyLabel} title={title}>
      {rows.map((row) => (
        <article className="p-3" key={row.id}>
          <div className="mb-2 flex items-start justify-between gap-2">
            <div className="min-w-0">
              <p className="truncate text-sm font-medium">{row.action}</p>
              <p className="text-muted-foreground truncate text-xs">
                {row.tenant_name} - {row.target_type} - {row.ip ?? 'no ip'}
              </p>
            </div>
            <Badge variant="outline">{formatDate(row.created_at)}</Badge>
          </div>
          <div className="flex flex-wrap gap-2">
            <Button
              onClick={() => {
                onSelectTenant(row.tenant_id);
                window.location.hash = 'tenant-detail';
              }}
              size="sm"
              type="button"
              variant="ghost"
            >
              <Building2 className="size-4" />
              Tenant
            </Button>
            <Button
              disabled={!row.actor_principal_id}
              onClick={() => {
                if (row.actor_principal_id) onSelectUser(row.actor_principal_id);
                window.location.hash = 'user-detail';
              }}
              size="sm"
              type="button"
              variant="outline"
            >
              <UserRound className="size-4" />
              {row.actor_email ?? 'Actor'}
            </Button>
          </div>
        </article>
      ))}
    </EvidenceList>
  );
}

function HashAnomalyList({
  onSelectTenant,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  rows: AuditHashAnomaly[];
}) {
  return (
    <EvidenceList emptyLabel="Aucune anomalie hash." title="Hash anomalies">
      {rows.map((row) => (
        <article className="p-3" key={row.id}>
          <div className="mb-2 flex items-start justify-between gap-2">
            <div className="min-w-0">
              <p className="truncate text-sm font-medium">{row.action}</p>
              <p className="text-muted-foreground truncate text-xs">
                {row.tenant_name} - hash {row.event_hash || 'missing'}
              </p>
            </div>
            <Badge variant="destructive">{formatDate(row.created_at)}</Badge>
          </div>
          <Button
            onClick={() => {
              onSelectTenant(row.tenant_id);
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
      ))}
    </EvidenceList>
  );
}

function SigningKeyList({ rows }: { rows: SigningKeySummary[] }) {
  return (
    <EvidenceList emptyLabel="Aucune cle de signature." title="Signing keys">
      {rows.map((row) => (
        <div className="flex items-center justify-between gap-3 p-3" key={row.kid}>
          <div className="min-w-0">
            <p className="truncate text-sm font-medium">{row.kid}</p>
            <p className="text-muted-foreground truncate text-xs">
              {row.algorithm} - {row.kms_key_id ?? 'local key'} - activated{' '}
              {formatDate(row.activated_at)}
            </p>
          </div>
          <Badge variant={row.status === 'revoked' ? 'destructive' : 'outline'}>{row.status}</Badge>
        </div>
      ))}
    </EvidenceList>
  );
}

function EvidenceList({
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

function EvidenceMetric({
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
