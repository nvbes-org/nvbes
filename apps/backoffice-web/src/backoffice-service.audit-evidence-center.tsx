import { useQuery } from '@tanstack/react-query';
import {
  AlertTriangle,
  Building2,
  CircleDot,
  FileCheck2,
  KeyRound,
  ShieldCheck,
  UserRound,
} from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { getAuditEvidenceCenter } from './backoffice-service.api';
import { AuditEvidenceAlerts } from './backoffice-service.audit-evidence-alerts';
import {
  EvidenceList,
  EvidenceMetric,
  HashChain,
  SigningKeyList,
  formatCount,
  formatDate,
  shortHash,
} from './backoffice-service.audit-evidence-widgets';
import { LockedState } from './backoffice-service.locked-state';
import type {
  AdminCredentials,
  AuditEvidenceEvent,
  AuditHashAnomaly,
} from './backoffice-service.types';

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
      <AuditEvidenceAlerts alerts={data?.alerts ?? []} runtimeAlerts={data?.runtime_alerts ?? []} />
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
          icon={ShieldCheck}
          label="Linked"
          value={formatCount(data?.linked_hash_count)}
        />
        <EvidenceMetric
          icon={CircleDot}
          label="Chain heads"
          tone={(data?.chain_head_count ?? 0) > 1 ? 'warning' : 'default'}
          value={formatCount(data?.chain_head_count)}
        />
        <EvidenceMetric
          icon={AlertTriangle}
          label="Anomalies"
          tone={(data?.hash_anomaly_count ?? 0) > 0 ? 'danger' : 'default'}
          value={formatCount(data?.hash_anomaly_count)}
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
          <HashChain
            eventHash={row.event_hash}
            previousEventHash={row.previous_event_hash}
            status={row.hash_chain_status}
          />
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
                {row.tenant_name} - hash {shortHash(row.event_hash)}
              </p>
            </div>
            <Badge variant="destructive">{formatDate(row.created_at)}</Badge>
          </div>
          <HashChain eventHash={row.event_hash} previousEventHash={row.previous_event_hash} />
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
