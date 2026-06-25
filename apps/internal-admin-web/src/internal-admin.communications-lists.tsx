import { FileSearch } from 'lucide-react';
import type { ReactNode } from 'react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import type {
  BusinessTypeDistribution,
  EmailStatusDistribution,
  RecentEmailEvent,
  RecentEmailFailure,
  RecentEmailSuppression,
} from './internal-admin.types';

export function StatusDistributionList({ rows }: { rows: EmailStatusDistribution[] }) {
  return (
    <CommsList
      emptyLabel="Aucun message email. Les volumes par statut de delivery apparaitront ici."
      title="Status distribution"
    >
      {rows.map((row) => (
        <CompactRow
          auditTargetId={row.status}
          auditTargetType="email_status"
          badge={formatCount(row.message_count)}
          key={row.status}
          label={row.status}
          meta="messages"
        />
      ))}
    </CommsList>
  );
}

export function BusinessTypeList({ rows }: { rows: BusinessTypeDistribution[] }) {
  return (
    <CommsList
      emptyLabel="Aucun business type email. Les templates transactionnels et leurs echecs seront listes ici."
      title="Business types"
    >
      {rows.map((row) => (
        <CompactRow
          auditTargetId={row.business_type}
          auditTargetType="email_business_type"
          badge={`${formatCount(row.failure_count)} failures`}
          key={row.business_type}
          label={row.business_type}
          meta={`${formatCount(row.message_count)} messages`}
        />
      ))}
    </CommsList>
  );
}

export function RecentFailureList({ rows }: { rows: RecentEmailFailure[] }) {
  return (
    <CommsList
      emptyLabel="Aucun email en echec. Les erreurs provider et messages a rejouer seront visibles ici."
      title="Recent failures"
    >
      {rows.map((row) => (
        <CompactRow
          auditTargetId={row.id}
          auditTargetType="email_message"
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

export function SuppressionList({ rows }: { rows: RecentEmailSuppression[] }) {
  return (
    <CommsList
      emptyLabel="Aucune suppression email. Les bounces, blocks et opt-outs seront listes ici."
      title="Suppressions"
    >
      {rows.map((row) => (
        <CompactRow
          auditTargetId={row.email}
          auditTargetType="email_suppression"
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

export function UnprocessedEventList({ rows }: { rows: RecentEmailEvent[] }) {
  return (
    <div className="xl:col-span-2">
      <CommsList
        emptyLabel="Aucun event email non traite. Les webhooks provider en attente de reconciliation apparaitront ici."
        title="Unprocessed webhook events"
      >
        {rows.map((row) => (
          <CompactRow
            auditTargetId={row.id}
            auditTargetType="email_provider_event"
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
  auditTargetId,
  auditTargetType,
  badge,
  label,
  meta,
  tone = 'default',
}: {
  auditTargetId: string;
  auditTargetType: string;
  badge: string;
  label: string;
  meta: string;
  tone?: 'danger' | 'default' | 'warning';
}) {
  return (
    <article className="p-3">
      <div className="mb-2 flex items-center justify-between gap-3">
        <div className="min-w-0">
          <p className="truncate text-sm font-medium">{label}</p>
          <p className="text-muted-foreground truncate text-xs">{meta}</p>
        </div>
        <Badge variant={tone === 'danger' ? 'destructive' : 'outline'}>{badge}</Badge>
      </div>
      <AuditButton targetId={auditTargetId} targetType={auditTargetType} />
    </article>
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
