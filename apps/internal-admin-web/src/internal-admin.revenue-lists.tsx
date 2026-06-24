import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import type { RecentDispute, RecentDunningCase } from './internal-admin.types';

export function DunningCaseList({
  onSelectTenant,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  rows: RecentDunningCase[];
}) {
  return (
    <div className="rounded-md border">
      <div className="border-b p-3">
        <h3 className="text-sm font-medium">Dunning cases</h3>
      </div>
      <div className="divide-y">
        {rows.length === 0 ? <EmptyRow label="Aucun dunning case ouvert." /> : null}
        {rows.map((row) => (
          <article className="p-3" key={row.id}>
            <div className="mb-2 flex items-center justify-between gap-2">
              <div className="min-w-0">
                <p className="truncate text-sm font-medium">{row.tenant_name}</p>
                <p className="text-muted-foreground text-xs">
                  {row.policy_state} - {formatDate(row.opened_at)} - {shortId(row.id)}
                </p>
              </div>
              <Badge variant="destructive">{row.status}</Badge>
            </div>
            <TenantButton tenantId={row.tenant_id} onSelectTenant={onSelectTenant} />
          </article>
        ))}
      </div>
    </div>
  );
}

export function DisputeList({
  onSelectTenant,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  rows: RecentDispute[];
}) {
  return (
    <div className="rounded-md border">
      <div className="border-b p-3">
        <h3 className="text-sm font-medium">Disputes</h3>
      </div>
      <div className="divide-y">
        {rows.length === 0 ? <EmptyRow label="Aucune dispute recente." /> : null}
        {rows.map((row) => (
          <article className="p-3" key={row.id}>
            <div className="mb-2 flex items-center justify-between gap-2">
              <div className="min-w-0">
                <p className="truncate text-sm font-medium">{row.tenant_name}</p>
                <p className="text-muted-foreground text-xs">
                  {row.status} - {formatDate(row.created_at)} - {shortId(row.id)}
                </p>
              </div>
              <Badge variant="destructive">{formatMoney(row.amount_minor, row.currency)}</Badge>
            </div>
            <TenantButton tenantId={row.tenant_id} onSelectTenant={onSelectTenant} />
          </article>
        ))}
      </div>
    </div>
  );
}

function TenantButton({
  onSelectTenant,
  tenantId,
}: {
  onSelectTenant: (tenantId: string) => void;
  tenantId: string;
}) {
  return (
    <Button
      onClick={() => {
        onSelectTenant(tenantId);
        window.location.hash = 'tenant-detail';
      }}
      size="sm"
      type="button"
      variant="outline"
    >
      Open tenant
    </Button>
  );
}

function EmptyRow({ label }: { label: string }) {
  return <div className="text-muted-foreground p-4 text-sm">{label}</div>;
}

function formatMoney(amountMinor: number, currency: string): string {
  return new Intl.NumberFormat('fr-FR', {
    currency,
    style: 'currency',
  }).format(amountMinor / 100);
}

function formatDate(value: string | null): string {
  if (!value) return '-';
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString('fr-FR', {
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    month: '2-digit',
  });
}

function shortId(value: string): string {
  return value.slice(0, 8);
}
