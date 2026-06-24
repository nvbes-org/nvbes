import { useQuery } from '@tanstack/react-query';
import { AlertTriangle, CreditCard, ReceiptText, Repeat2, WalletCards } from 'lucide-react';
import type { ComponentType } from 'react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { getRevenueCenter } from './internal-admin.api';
import { LockedState } from './internal-admin.locked-state';
import { RevenueActionsPanel } from './internal-admin.revenue-actions';
import { DisputeList, DunningCaseList } from './internal-admin.revenue-lists';
import type {
  AdminCredentials,
  MoneyTotal,
  RecentCapturedPayment,
  RecentOverdueInvoice,
} from './internal-admin.types';

export function RevenueCenterPanel({
  credentials,
  disabled,
  onSelectTenant,
}: {
  credentials: AdminCredentials;
  disabled: boolean;
  onSelectTenant: (tenantId: string) => void;
}) {
  const revenue = useQuery({
    queryKey: ['revenue-center'],
    queryFn: () => getRevenueCenter(credentials),
    enabled: !disabled,
  });
  const data = revenue.data;

  return (
    <section className="border-border bg-card mb-5 rounded-lg border p-4" id="revenue-center">
      <div className="mb-4 flex flex-col gap-2 md:flex-row md:items-end md:justify-between">
        <div>
          <p className="text-muted-foreground text-xs font-medium tracking-[0.18em] uppercase">
            Revenue center
          </p>
          <h2 className="text-base font-semibold">Cash, invoices et recouvrement</h2>
        </div>
        <Badge variant={(data?.open_dunning_case_count ?? 0) > 0 ? 'destructive' : 'secondary'}>
          {formatCount(data?.open_dunning_case_count)} dunning open
        </Badge>
      </div>
      {disabled ? (
        <LockedState label="Connecte un contexte operateur pour charger les signaux revenue." />
      ) : null}
      <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-5">
        <MoneyMetric icon={CreditCard} label="Captured 30d" totals={data?.captured_payments_30d} />
        <MoneyMetric icon={ReceiptText} label="Open invoices" totals={data?.open_invoices} />
        <MoneyMetric
          icon={AlertTriangle}
          label="Overdue"
          tone={(data?.overdue_invoices.length ?? 0) > 0 ? 'danger' : 'default'}
          totals={data?.overdue_invoices}
        />
        <MoneyMetric
          icon={Repeat2}
          label="Refunds 30d"
          tone={(data?.refunds_30d.length ?? 0) > 0 ? 'danger' : 'default'}
          totals={data?.refunds_30d}
        />
        <MoneyMetric
          icon={AlertTriangle}
          label="Disputes 30d"
          tone={(data?.disputes_30d.length ?? 0) > 0 ? 'danger' : 'default'}
          totals={data?.disputes_30d}
        />
      </div>
      <div className="mt-3 grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
        <CountMetric
          icon={WalletCards}
          label="Active subs"
          value={formatCount(data?.active_subscription_count)}
        />
        <CountMetric
          icon={WalletCards}
          label="Trialing subs"
          value={formatCount(data?.trialing_subscription_count)}
        />
        <CountMetric
          icon={AlertTriangle}
          label="Dunning"
          tone={(data?.open_dunning_case_count ?? 0) > 0 ? 'danger' : 'default'}
          value={formatCount(data?.open_dunning_case_count)}
        />
        <CountMetric
          icon={AlertTriangle}
          label="Recon diffs"
          tone={(data?.unresolved_reconciliation_difference_count ?? 0) > 0 ? 'danger' : 'default'}
          value={formatCount(data?.unresolved_reconciliation_difference_count)}
        />
      </div>
      <div className="mt-4 grid gap-3 xl:grid-cols-2">
        <OverdueInvoiceList
          rows={data?.recent_overdue_invoices ?? []}
          onSelectTenant={onSelectTenant}
        />
        <DunningCaseList rows={data?.recent_dunning_cases ?? []} onSelectTenant={onSelectTenant} />
        <DisputeList rows={data?.recent_disputes ?? []} onSelectTenant={onSelectTenant} />
        <CapturedPaymentList
          rows={data?.recent_captured_payments ?? []}
          onSelectTenant={onSelectTenant}
        />
      </div>
      <RevenueActionsPanel credentials={credentials} disabled={disabled} />
    </section>
  );
}

function OverdueInvoiceList({
  onSelectTenant,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  rows: RecentOverdueInvoice[];
}) {
  return (
    <div className="rounded-md border">
      <div className="border-b p-3">
        <h3 className="text-sm font-medium">Invoices overdue</h3>
      </div>
      <div className="divide-y">
        {rows.length === 0 ? <EmptyRow label="Aucune invoice overdue." /> : null}
        {rows.map((row) => (
          <article className="p-3" key={row.id}>
            <div className="mb-2 flex items-center justify-between gap-2">
              <div className="min-w-0">
                <p className="truncate text-sm font-medium">{row.invoice_number ?? row.id}</p>
                <p className="text-muted-foreground text-xs">
                  {row.tenant_name} - due {formatDate(row.due_at)} - {shortId(row.id)}
                </p>
              </div>
              <Badge variant="destructive">{formatMoney(row.total_minor, row.currency)}</Badge>
            </div>
            <TenantButton tenantId={row.tenant_id} onSelectTenant={onSelectTenant} />
          </article>
        ))}
      </div>
    </div>
  );
}

function CapturedPaymentList({
  onSelectTenant,
  rows,
}: {
  onSelectTenant: (tenantId: string) => void;
  rows: RecentCapturedPayment[];
}) {
  return (
    <div className="rounded-md border">
      <div className="border-b p-3">
        <h3 className="text-sm font-medium">Paiements captures</h3>
      </div>
      <div className="divide-y">
        {rows.length === 0 ? <EmptyRow label="Aucun paiement capture." /> : null}
        {rows.map((row) => (
          <article className="p-3" key={row.id}>
            <div className="mb-2 flex items-center justify-between gap-2">
              <div className="min-w-0">
                <p className="truncate text-sm font-medium">{row.tenant_name}</p>
                <p className="text-muted-foreground text-xs">{formatDate(row.created_at)}</p>
              </div>
              <Badge variant="secondary">{formatMoney(row.amount_minor, row.currency)}</Badge>
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

function MoneyMetric({
  icon: Icon,
  label,
  tone = 'default',
  totals,
}: {
  icon: ComponentType<{ className?: string }>;
  label: string;
  tone?: 'danger' | 'default';
  totals: MoneyTotal[] | undefined;
}) {
  return (
    <div className="bg-muted/30 rounded-md border p-3">
      <div className="text-muted-foreground mb-3 flex items-center justify-between text-xs">
        {label}
        <Icon className={tone === 'danger' ? 'text-destructive size-4' : 'size-4'} />
      </div>
      <div className={tone === 'danger' ? 'text-destructive space-y-1' : 'space-y-1'}>
        {totals && totals.length > 0 ? (
          totals.map((total) => (
            <div className="text-lg font-semibold" key={total.currency}>
              {formatMoney(total.amount_minor, total.currency)}
              <span className="text-muted-foreground ml-2 text-xs font-normal">
                {formatCount(total.object_count)}
              </span>
            </div>
          ))
        ) : (
          <div className="text-lg font-semibold">-</div>
        )}
      </div>
    </div>
  );
}

function CountMetric({
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

function EmptyRow({ label }: { label: string }) {
  return <div className="text-muted-foreground p-4 text-sm">{label}</div>;
}

function formatCount(value: number | undefined): string {
  return typeof value === 'number' ? new Intl.NumberFormat('fr-FR').format(value) : '-';
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
