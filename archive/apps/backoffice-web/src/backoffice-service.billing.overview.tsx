import { useQuery } from '@tanstack/react-query';
import { AlertTriangle, Clock3, CreditCard, FileText, Gauge, ReceiptText } from 'lucide-react';
import type { ComponentType } from 'react';
import { getBillingOverview } from './backoffice-service.api';
import type { AdminCredentials } from './backoffice-service.types';

export function BillingOverviewCards({
  credentials,
  disabled,
  readiness,
}: {
  credentials: AdminCredentials;
  disabled: boolean;
  readiness: number;
}) {
  const overview = useQuery({
    queryKey: ['billing-overview', credentials.workspaceId],
    queryFn: () => getBillingOverview(credentials),
    enabled: !disabled,
  });
  const data = overview.data;

  return (
    <section className="grid grid-cols-2 gap-3">
      <MetricCard icon={Gauge} label="Contexte" value={`${readiness}%`} />
      <MetricCard
        icon={ReceiptText}
        label="Invoices open"
        value={formatCount(data?.open_invoice_count)}
      />
      <MetricCard
        icon={Clock3}
        label="Overdue"
        tone={(data?.overdue_invoice_count ?? 0) > 0 ? 'danger' : 'default'}
        value={formatCount(data?.overdue_invoice_count)}
      />
      <MetricCard
        icon={AlertTriangle}
        label="Provider errors"
        tone={(data?.failed_provider_event_count ?? 0) > 0 ? 'danger' : 'default'}
        value={formatCount(data?.failed_provider_event_count)}
      />
      <MetricCard
        icon={FileText}
        label="Open total"
        value={formatMoneyMinor(data?.open_invoice_total_minor)}
      />
      <MetricCard
        icon={CreditCard}
        label="Captured 30d"
        value={formatMoneyMinor(data?.captured_payment_total_minor_30d)}
      />
    </section>
  );
}

function MetricCard({
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
    <div className="border-border bg-card rounded-lg border p-3">
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

function formatMoneyMinor(value: number | undefined): string {
  if (typeof value !== 'number') return '-';
  return new Intl.NumberFormat('fr-FR', {
    currency: 'EUR',
    maximumFractionDigits: 0,
    style: 'currency',
  }).format(value / 100);
}
