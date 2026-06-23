import { useQuery } from '@tanstack/react-query';
import { AlertTriangle, Inbox, Mail, MailCheck, MailX, RadioTower, ShieldOff } from 'lucide-react';
import type { ComponentType, ReactNode } from 'react';
import { Badge } from '@/components/ui/badge';
import { getCommunicationsCenter } from './internal-admin.api';
import { LockedState } from './internal-admin.locked-state';
import type {
  AdminCredentials,
  BusinessTypeDistribution,
  EmailStatusDistribution,
  RecentEmailEvent,
  RecentEmailFailure,
  RecentEmailSuppression,
} from './internal-admin.types';

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

function StatusDistributionList({ rows }: { rows: EmailStatusDistribution[] }) {
  return (
    <CommsList emptyLabel="Aucun message email." title="Status distribution">
      {rows.map((row) => (
        <CompactRow
          badge={formatCount(row.message_count)}
          key={row.status}
          label={row.status}
          meta="messages"
        />
      ))}
    </CommsList>
  );
}

function BusinessTypeList({ rows }: { rows: BusinessTypeDistribution[] }) {
  return (
    <CommsList emptyLabel="Aucun business type email." title="Business types">
      {rows.map((row) => (
        <CompactRow
          badge={`${formatCount(row.failure_count)} failures`}
          key={row.business_type}
          label={row.business_type}
          meta={`${formatCount(row.message_count)} messages`}
        />
      ))}
    </CommsList>
  );
}

function RecentFailureList({ rows }: { rows: RecentEmailFailure[] }) {
  return (
    <CommsList emptyLabel="Aucun email en echec." title="Recent failures">
      {rows.map((row) => (
        <CompactRow
          badge={row.status}
          key={row.id}
          label={row.recipient_email}
          meta={`${row.business_type} - ${row.provider_email_id ?? 'no provider id'}`}
          tone="danger"
        />
      ))}
    </CommsList>
  );
}

function SuppressionList({ rows }: { rows: RecentEmailSuppression[] }) {
  return (
    <CommsList emptyLabel="Aucune suppression email." title="Suppressions">
      {rows.map((row) => (
        <CompactRow
          badge={row.reason}
          key={row.email}
          label={row.email}
          meta={formatDate(row.suppressed_at)}
          tone="danger"
        />
      ))}
    </CommsList>
  );
}

function UnprocessedEventList({ rows }: { rows: RecentEmailEvent[] }) {
  return (
    <div className="xl:col-span-2">
      <CommsList emptyLabel="Aucun event email non traite." title="Unprocessed webhook events">
        {rows.map((row) => (
          <CompactRow
            badge={row.event_type}
            key={row.id}
            label={row.email}
            meta={`${row.provider_event_id} - ${formatDate(row.created_at)}`}
            tone="warning"
          />
        ))}
      </CommsList>
    </div>
  );
}

function CompactRow({
  badge,
  label,
  meta,
  tone = 'default',
}: {
  badge: string;
  label: string;
  meta: string;
  tone?: 'danger' | 'default' | 'warning';
}) {
  return (
    <div className="flex items-center justify-between gap-3 p-3">
      <div className="min-w-0">
        <p className="truncate text-sm font-medium">{label}</p>
        <p className="text-muted-foreground truncate text-xs">{meta}</p>
      </div>
      <Badge variant={tone === 'danger' ? 'destructive' : 'outline'}>{badge}</Badge>
    </div>
  );
}

function CommsList({
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

function formatDate(value: string): string {
  return new Intl.DateTimeFormat('fr-FR').format(new Date(value));
}
