import { useMutation } from '@tanstack/react-query';
import { CheckCircle2, Download, Loader2 } from 'lucide-react';
import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { downloadBillingExport } from './backoffice-service.api';
import { LockedState } from './backoffice-service.locked-state';
import type { AdminCredentials, ExportType } from './backoffice-service.types';

const exportOptions: Array<{ description: string; label: string; type: ExportType }> = [
  { description: 'Factures, montants, taxes et statuts.', label: 'Invoices', type: 'invoices' },
  { description: 'Paiements provider et statuts de capture.', label: 'Payments', type: 'payments' },
  { description: 'Base de controle TVA et taxes facturees.', label: 'Tax', type: 'tax' },
  { description: 'Ecritures comptables billing.', label: 'Ledger', type: 'ledger' },
  { description: 'Comptes billing et contacts finance.', label: 'Customers', type: 'customers' },
  {
    description: 'Abonnements, periodes et statuts.',
    label: 'Subscriptions',
    type: 'subscriptions',
  },
];

export function BillingExports({
  credentials,
  disabled,
}: {
  credentials: AdminCredentials;
  disabled: boolean;
}) {
  const [lastDownload, setLastDownload] = useState<string | null>(null);
  const mutation = useMutation({
    mutationFn: (exportType: ExportType) => downloadBillingExport(credentials, exportType),
    onSuccess: ({ blob, filename }) => {
      const url = URL.createObjectURL(blob);
      const anchor = document.createElement('a');
      anchor.href = url;
      anchor.download = filename;
      anchor.click();
      URL.revokeObjectURL(url);
      setLastDownload(filename);
    },
  });

  return (
    <section className="border-border bg-card rounded-lg border p-4" id="exports">
      <div className="mb-4">
        <h2 className="text-sm font-semibold">Exports finance</h2>
        <p className="text-muted-foreground text-xs">
          Telechargement CSV inline depuis backoffice-service-api.
        </p>
      </div>
      <div className="grid gap-2">
        {exportOptions.map((option) => (
          <Button
            className="h-auto justify-between gap-3 px-3 py-2 text-left"
            disabled={disabled || mutation.isPending}
            key={option.type}
            onClick={() => mutation.mutate(option.type)}
            type="button"
            variant="outline"
          >
            <span className="min-w-0">
              <span className="block text-sm font-medium">{option.label}</span>
              <span className="text-muted-foreground block truncate text-xs">
                {option.description}
              </span>
            </span>
            {mutation.isPending ? (
              <Loader2 className="size-4 shrink-0 animate-spin" />
            ) : (
              <Download className="size-4 shrink-0" />
            )}
          </Button>
        ))}
      </div>
      {disabled ? (
        <div className="mt-3">
          <LockedState label="Exports verrouilles tant que le contexte operateur est incomplet." />
        </div>
      ) : null}
      {mutation.error ? (
        <p className="border-destructive/20 bg-destructive/5 text-destructive mt-3 rounded-md border p-3 text-xs">
          {mutation.error instanceof Error ? mutation.error.message : 'Export failed'}
        </p>
      ) : null}
      {lastDownload ? (
        <div className="border-border bg-muted/40 text-muted-foreground mt-3 flex items-center gap-2 rounded-md border p-3 text-xs">
          <CheckCircle2 className="text-primary size-4" />
          Dernier export telecharge: <span className="font-mono">{lastDownload}</span>
        </div>
      ) : null}
    </section>
  );
}
