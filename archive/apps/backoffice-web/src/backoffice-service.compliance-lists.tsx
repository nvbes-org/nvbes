import { FileSearch, UserRound } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import type { RecentRevokedConsent, RecentSuppressedEmail } from './backoffice-service.types';

export function RevokedConsentList({
  onSelectTenant,
  onSelectUser,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  onSelectUser: (principalId: string) => void;
  rows: RecentRevokedConsent[];
}) {
  return (
    <div className="rounded-md border">
      <div className="border-b p-3">
        <h3 className="text-sm font-medium">Consentements revoques</h3>
      </div>
      <div className="divide-y">
        {rows.length === 0 ? (
          <EmptyRow label="Aucun consentement revoque recent. Les retraits GDPR et changements de version documentaire apparaitront ici." />
        ) : null}
        {rows.map((row) => (
          <article className="p-3" key={row.id}>
            <RowHeader
              badge={row.tenant_name}
              subtitle={`${row.consent_type} - ${row.document_version}`}
              title={row.email ?? row.principal_id}
            />
            <LinkedRowActions
              auditTargetId={row.id}
              auditTargetType="compliance_consent"
              onTenant={() => onSelectTenant(row.tenant_id)}
              onUser={() => onSelectUser(row.principal_id)}
              tenantDisabled={false}
              userDisabled={false}
            />
          </article>
        ))}
      </div>
    </div>
  );
}

export function SuppressedEmailList({
  onSelectTenant,
  onSelectUser,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  onSelectUser: (principalId: string) => void;
  rows: RecentSuppressedEmail[];
}) {
  return (
    <div className="rounded-md border">
      <div className="border-b p-3">
        <h3 className="text-sm font-medium">Emails supprimes</h3>
      </div>
      <div className="divide-y">
        {rows.length === 0 ? (
          <EmptyRow label="Aucun email supprime. Les suppressions, bounces et exclusions marketing seront listees ici." />
        ) : null}
        {rows.map((row) => (
          <article className="p-3" key={`${row.email}:${row.suppressed_at}`}>
            <RowHeader
              badge={row.tenant_name ?? 'unmatched'}
              subtitle={row.reason}
              title={row.email}
            />
            <LinkedRowActions
              auditTargetId={row.email}
              auditTargetType="email_suppression"
              onTenant={() => {
                if (row.tenant_id) onSelectTenant(row.tenant_id);
              }}
              onUser={() => {
                if (row.principal_id) onSelectUser(row.principal_id);
              }}
              tenantDisabled={!row.tenant_id}
              userDisabled={!row.principal_id}
            />
          </article>
        ))}
      </div>
    </div>
  );
}

function RowHeader({ badge, subtitle, title }: { badge: string; subtitle: string; title: string }) {
  return (
    <div className="mb-2 flex items-center justify-between gap-2">
      <div className="min-w-0">
        <p className="truncate text-sm font-medium">{title}</p>
        <p className="text-muted-foreground text-xs">{subtitle}</p>
      </div>
      <Badge variant="outline">{badge}</Badge>
    </div>
  );
}

function LinkedRowActions({
  auditTargetId,
  auditTargetType,
  onTenant,
  onUser,
  tenantDisabled,
  userDisabled,
}: {
  auditTargetId: string;
  auditTargetType: string;
  onTenant: () => void;
  onUser: () => void;
  tenantDisabled: boolean;
  userDisabled: boolean;
}) {
  return (
    <div className="flex flex-wrap gap-2">
      <Button
        disabled={userDisabled}
        onClick={() => {
          onUser();
          window.location.hash = 'user-detail';
        }}
        size="sm"
        type="button"
        variant="outline"
      >
        <UserRound className="size-4" />
        Open user
      </Button>
      <Button
        disabled={tenantDisabled}
        onClick={() => {
          onTenant();
          window.location.hash = 'tenant-detail';
        }}
        size="sm"
        type="button"
        variant="ghost"
      >
        Tenant
      </Button>
      <AuditButton targetId={auditTargetId} targetType={auditTargetType} />
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
