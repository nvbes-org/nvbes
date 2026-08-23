import { AccountHttpError } from '@nvbes/account-client';
import { useMutation, useQuery } from '@tanstack/react-query';
import { useAccountAuthenticationRecovery } from '@/account.authentication';
import { accountClient } from '@/account.client';
import { accountQueryKeys } from '@/account.queries';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';

export function AccountPrivacyExportActions() {
  const status = useQuery({
    queryKey: accountQueryKeys.export,
    queryFn: ({ signal }) => accountClient.getLatestDataExport({ signal }),
    refetchInterval: (query) =>
      query.state.data && ['completed', 'failed', 'expired'].includes(query.state.data.status)
        ? false
        : 1_000,
    retry: (failureCount, error) =>
      !(error instanceof AccountHttpError && error.status === 404) && failureCount < 2,
  });
  const request = useMutation({
    mutationFn: () => accountClient.requestDataExport(),
    onSuccess: () => status.refetch(),
  });
  const download = useMutation({
    mutationFn: async (exportId: string) => {
      const blob = await accountClient.downloadDataExport(exportId);
      downloadBlob(blob, `nvbes-account-export-${exportId}.json`);
    },
  });
  const noExport = status.error instanceof AccountHttpError && status.error.status === 404;
  useAccountAuthenticationRecovery(
    request.error ?? (noExport ? null : status.error) ?? download.error,
  );

  return (
    <div className="space-y-4">
      <div className="flex flex-col justify-between gap-3 sm:flex-row sm:items-center">
        <div>
          <h2 className="text-base font-semibold">Exporter mes données</h2>
          <p className="mt-1 text-sm text-muted-foreground">
            Prépare un document réunissant vos données Account, Cloud, Billing et Identity, sans
            secrets d’authentification.
          </p>
        </div>
        <Button
          type="button"
          variant="outline"
          disabled={
            request.isPending ||
            status.data?.status === 'pending' ||
            status.data?.status === 'processing' ||
            status.data?.status === 'completed'
          }
          onClick={() => request.mutate()}
        >
          {request.isPending
            ? 'Demande en cours…'
            : status.data?.status === 'completed'
              ? 'Export prêt'
              : request.isSuccess
                ? 'Préparation…'
                : 'Demander un export'}
        </Button>
      </div>

      {status.data ? (
        <div className="flex flex-wrap items-center justify-between gap-3 border-y border-border py-3 text-sm">
          <span className="text-muted-foreground">
            {status.data.participants.filter((item) => item.status === 'completed').length}/
            {status.data.participants.length} produits collectés · état {status.data.status}
          </span>
          {status.data.status === 'completed' ? (
            <Button
              type="button"
              variant="outline"
              disabled={download.isPending}
              onClick={() => download.mutate(status.data.export_id)}
            >
              {download.isPending ? 'Téléchargement…' : 'Télécharger le JSON'}
            </Button>
          ) : null}
        </div>
      ) : null}

      {request.error ? (
        <Alert variant="destructive">
          <AlertTitle>Export impossible</AlertTitle>
          <AlertDescription>{request.error.message}</AlertDescription>
        </Alert>
      ) : null}
      {status.data?.status === 'failed' || (!noExport && status.error) || download.error ? (
        <Alert variant="destructive">
          <AlertTitle>Export interrompu</AlertTitle>
          <AlertDescription>
            {status.data?.last_error ?? status.error?.message ?? download.error?.message}
          </AlertDescription>
        </Alert>
      ) : null}
    </div>
  );
}

function downloadBlob(blob: Blob, filename: string) {
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement('a');
  anchor.href = url;
  anchor.download = filename;
  anchor.click();
  window.setTimeout(() => URL.revokeObjectURL(url), 0);
}
