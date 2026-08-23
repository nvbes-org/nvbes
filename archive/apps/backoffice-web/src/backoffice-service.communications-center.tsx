import { useQuery } from '@tanstack/react-query';
import { AlertTriangle, Inbox, Mail, MailCheck, MailX, RadioTower, ShieldOff } from 'lucide-react';
import type { ComponentType } from 'react';
import { Badge } from '@/components/ui/badge';
import { getCommunicationsCenter } from './backoffice-service.api';
import { CommunicationsActionsPanel } from './backoffice-service.communications-actions';
import {
  BusinessTypeList,
  RecentFailureList,
  StatusDistributionList,
  SuppressionList,
  UnprocessedEventList,
} from './backoffice-service.communications-lists';
import { LockedState } from './backoffice-service.locked-state';
import type { AdminCredentials } from './backoffice-service.types';

export function CommunicationsCenterPanel({
  credentials,
  disabled,
}: {
  credentials: AdminCredentials;
  disabled: boolean;
}) {
  const communications = useQuery({
    queryKey: ['communications-center'],
    queryFn: () => getCommunicationsCenter(credentials),
    enabled: !disabled,
  });
  const data = communications.data;

  return (
    <section
      className="border-border bg-card mb-5 rounded-lg border p-4"
      id="communications-center"
    >
      <div className="mb-4 flex flex-col gap-2 md:flex-row md:items-end md:justify-between">
        <div>
          <p className="text-muted-foreground text-xs font-medium tracking-[0.18em] uppercase">
            Communications center
          </p>
          <h2 className="text-base font-semibold">Emails transactionnels et delivrabilite</h2>
        </div>
        <Badge variant={(data?.failed_message_count_24h ?? 0) > 0 ? 'destructive' : 'secondary'}>
          {formatCount(data?.failed_message_count_24h)} failures 24h
        </Badge>
      </div>
      {disabled ? (
        <LockedState label="Connecte un contexte operateur pour charger les signaux communications." />
      ) : null}
      <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-7">
        <CommsMetric
          icon={Inbox}
          label="Queued"
          tone={(data?.queued_message_count ?? 0) > 0 ? 'warning' : 'default'}
          value={formatCount(data?.queued_message_count)}
        />
        <CommsMetric
          icon={Mail}
          label="Sent 24h"
          value={formatCount(data?.sent_message_count_24h)}
        />
        <CommsMetric
          icon={MailCheck}
          label="Delivered 24h"
          value={formatCount(data?.delivered_message_count_24h)}
        />
        <CommsMetric
          icon={MailX}
          label="Failed 24h"
          tone={(data?.failed_message_count_24h ?? 0) > 0 ? 'danger' : 'default'}
          value={formatCount(data?.failed_message_count_24h)}
        />
        <CommsMetric
          icon={ShieldOff}
          label="Suppressed"
          tone={(data?.suppressed_email_count ?? 0) > 0 ? 'danger' : 'default'}
          value={formatCount(data?.suppressed_email_count)}
        />
        <CommsMetric
          icon={RadioTower}
          label="Webhook events"
          value={formatCount(data?.webhook_event_count_24h)}
        />
        <CommsMetric
          icon={AlertTriangle}
          label="Unprocessed"
          tone={(data?.unprocessed_event_count ?? 0) > 0 ? 'warning' : 'default'}
          value={formatCount(data?.unprocessed_event_count)}
        />
      </div>
      <CommunicationsActionsPanel credentials={credentials} disabled={disabled} />
      <div className="mt-4 grid gap-3 xl:grid-cols-2">
        <StatusDistributionList rows={data?.status_distribution ?? []} />
        <BusinessTypeList rows={data?.business_type_distribution ?? []} />
        <RecentFailureList rows={data?.recent_failures ?? []} />
        <SuppressionList rows={data?.recent_suppressions ?? []} />
        <UnprocessedEventList rows={data?.recent_unprocessed_events ?? []} />
      </div>
    </section>
  );
}

function CommsMetric({
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
