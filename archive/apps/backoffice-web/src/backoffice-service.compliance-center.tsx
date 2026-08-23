import { useQuery } from '@tanstack/react-query';
import { FileCheck2, MailWarning, ShieldCheck, UsersRound } from 'lucide-react';
import type { ComponentType } from 'react';
import { Badge } from '@/components/ui/badge';
import { getComplianceCenter } from './backoffice-service.api';
import { ComplianceActionsPanel } from './backoffice-service.compliance-actions';
import { RevokedConsentList, SuppressedEmailList } from './backoffice-service.compliance-lists';
import { LockedState } from './backoffice-service.locked-state';
import type { AdminCredentials } from './backoffice-service.types';

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
      <ComplianceActionsPanel credentials={credentials} disabled={disabled} />
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
