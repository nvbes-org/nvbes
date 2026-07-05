import type { ComponentType, ReactNode } from 'react';
import { Badge } from '@/components/ui/badge';
import type { SigningKeySummary } from './internal-admin.types';

export function HashChain({
  eventHash,
  previousEventHash,
  status,
}: {
  eventHash: string | null;
  previousEventHash: string | null;
  status?: string;
}) {
  const variant = status === 'hash_anomaly' ? 'destructive' : 'outline';
  return (
    <div className="text-muted-foreground mt-2 flex flex-wrap items-center gap-2 font-mono text-[11px]">
      {status ? <Badge variant={variant}>{status}</Badge> : null}
      <span className="rounded-md border px-2 py-1">prev:{shortHash(previousEventHash)}</span>
      <span className="rounded-md border px-2 py-1">hash:{shortHash(eventHash)}</span>
    </div>
  );
}

export function SigningKeyList({ rows }: { rows: SigningKeySummary[] }) {
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

export function EvidenceList({
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

export function EmptyRow({ label }: { label: string }) {
  return <div className="text-muted-foreground p-4 text-sm">{label}</div>;
}

export function EvidenceMetric({
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

export function formatCount(value: number | undefined): string {
  return typeof value === 'number' ? new Intl.NumberFormat('fr-FR').format(value) : '-';
}

export function formatDate(value: string): string {
  return new Intl.DateTimeFormat('fr-FR').format(new Date(value));
}

export function shortHash(value: string | null): string {
  if (!value) return 'missing';
  return value.length > 12 ? value.slice(0, 12) : value;
}
