import { useQuery } from '@tanstack/react-query';
import { FileCheck2, MailWarning, ShieldCheck, UserRound, UsersRound } from 'lucide-react';
import type { ComponentType } from 'react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { getComplianceCenter } from './internal-admin.api';
import { LockedState } from './internal-admin.locked-state';
import type {
  AdminCredentials,
  RecentRevokedConsent,
  RecentSuppressedEmail,
} from './internal-admin.types';

export function ComplianceCenterPanel({
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
  const compliance = useQuery({
    queryKey: ['compliance-center'],
    queryFn: () => getComplianceCenter(credentials),
    enabled: !disabled,
  });
  const data = compliance.data;

  return (
    <section className="border-border bg-card mb-5 rounded-lg border p-4" id="compliance-center">
      <div className="mb-4 flex flex-col gap-2 md:flex-row md:items-end md:justify-between">
        <div>
          <p className="text-muted-foreground text-xs font-medium tracking-[0.18em] uppercase">
            Compliance center
          </p>
          <h2 className="text-base font-semibold">Consentements, emails et verification</h2>
        </div>
        <Badge
          variant={(data?.email_delivery_failure_count_24h ?? 0) > 0 ? 'destructive' : 'secondary'}
        >
          {formatCount(data?.email_delivery_failure_count_24h)} email failures 24h
        </Badge>
      </div>
      {disabled ? (
        <LockedState label="Connecte un contexte operateur pour charger les signaux compliance." />
      ) : null}
      <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-6">
        <ComplianceMetric
          icon={FileCheck2}
          label="Active consents"
          value={formatCount(data?.active_consent_count)}
        />
        <ComplianceMetric
          icon={FileCheck2}
          label="Revoked 30d"
          tone={(data?.revoked_consent_count_30d ?? 0) > 0 ? 'danger' : 'default'}
          value={formatCount(data?.revoked_consent_count_30d)}
        />
        <ComplianceMetric
          icon={MailWarning}
          label="Suppressed"
          tone={(data?.suppressed_email_count ?? 0) > 0 ? 'danger' : 'default'}
          value={formatCount(data?.suppressed_email_count)}
        />
        <ComplianceMetric
          icon={MailWarning}
          label="Bounces 24h"
          tone={(data?.email_bounce_count_24h ?? 0) > 0 ? 'danger' : 'default'}
          value={formatCount(data?.email_bounce_count_24h)}
        />
        <ComplianceMetric
          icon={ShieldCheck}
          label="Delivery failures"
          tone={(data?.email_delivery_failure_count_24h ?? 0) > 0 ? 'danger' : 'default'}
          value={formatCount(data?.email_delivery_failure_count_24h)}
        />
        <ComplianceMetric
          icon={UsersRound}
          label="Unverified users"
          tone={(data?.unverified_user_count ?? 0) > 0 ? 'danger' : 'default'}
          value={formatCount(data?.unverified_user_count)}
        />
      </div>
      <div className="mt-4 grid gap-3 xl:grid-cols-2">
        <RevokedConsentList
          rows={data?.recent_revoked_consents ?? []}
          onSelectTenant={onSelectTenant}
          onSelectUser={onSelectUser}
        />
        <SuppressedEmailList
          rows={data?.recent_suppressed_emails ?? []}
          onSelectTenant={onSelectTenant}
          onSelectUser={onSelectUser}
        />
      </div>
    </section>
  );
}

function RevokedConsentList({
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
        {rows.length === 0 ? <EmptyRow label="Aucun consentement revoque recent." /> : null}
        {rows.map((row) => (
          <article className="p-3" key={row.id}>
            <div className="mb-2 flex items-center justify-between gap-2">
              <div className="min-w-0">
                <p className="truncate text-sm font-medium">{row.email ?? row.principal_id}</p>
                <p className="text-muted-foreground text-xs">
                  {row.consent_type} - {row.document_version}
                </p>
              </div>
              <Badge variant="outline">{row.tenant_name}</Badge>
            </div>
            <LinkedRowActions
              onTenant={() => onSelectTenant(row.tenant_id)}
              onUser={() => onSelectUser(row.principal_id)}
            />
          </article>
        ))}
      </div>
    </div>
  );
}

function SuppressedEmailList({
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
        {rows.length === 0 ? <EmptyRow label="Aucun email supprime." /> : null}
        {rows.map((row) => (
          <article className="p-3" key={`${row.email}:${row.suppressed_at}`}>
            <div className="mb-2 flex items-center justify-between gap-2">
              <div className="min-w-0">
                <p className="truncate text-sm font-medium">{row.email}</p>
                <p className="text-muted-foreground text-xs">{row.reason}</p>
              </div>
              <Badge variant="outline">{row.tenant_name ?? 'unmatched'}</Badge>
            </div>
            <LinkedRowActions
              disabled={!row.principal_id || !row.tenant_id}
              onTenant={() => {
                if (row.tenant_id) onSelectTenant(row.tenant_id);
              }}
              onUser={() => {
                if (row.principal_id) onSelectUser(row.principal_id);
              }}
            />
          </article>
        ))}
      </div>
    </div>
  );
}

function LinkedRowActions({
  disabled = false,
  onTenant,
  onUser,
}: {
  disabled?: boolean;
  onTenant: () => void;
  onUser: () => void;
}) {
  return (
    <div className="flex flex-wrap gap-2">
      <Button
        disabled={disabled}
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
        disabled={disabled}
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
    </div>
  );
}

function EmptyRow({ label }: { label: string }) {
  return <div className="text-muted-foreground p-4 text-sm">{label}</div>;
}

function ComplianceMetric({
  icon: Icon,
  label,
  tone = 'default',
  value,
}: {
  icon: ComponentType<{ className?: string }>;
  label: string;
  tone?: 'danger' | 'default';
  value: string;
}) {
  return (
    <div className="bg-muted/30 rounded-md border p-3">
      <div className="text-muted-foreground mb-3 flex items-center justify-between text-xs">
        {label}
        <Icon className={tone === 'danger' ? 'text-destructive size-4' : 'size-4'} />
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
