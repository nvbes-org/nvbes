import { Progress } from '@/components/ui/progress';
import { SidebarFooter } from '@/components/ui/sidebar';
import type { useStorageManager } from '@/hooks/use-storage-manager';

export function DriveWorkspaceSidebarFooter({
  workspaceName,
  storage,
}: {
  workspaceName: string;
  storage: ReturnType<typeof useStorageManager>;
}) {
  return (
    <SidebarFooter className="border-t p-3">
      <div className="flex flex-col gap-2">
        <div className="text-sm font-medium">{workspaceName}</div>
        <div className="flex items-center justify-between gap-4 text-xs">
          <span className="text-muted-foreground">Stockage</span>
          <span className="font-semibold">
            {storage.supported
              ? `${storage.formattedUsage} / ${storage.formattedQuota}`
              : 'Indisponible'}
          </span>
        </div>
        {storage.supported && <Progress value={storage.usageRatio} aria-label="Stockage utilise" />}
        {storage.supported && storage.persisted === false && (
          <button
            type="button"
            onClick={() => void storage.requestPersistence()}
            disabled={storage.isPersisting}
            className="mt-1 rounded-md bg-muted px-2 py-1 text-[10px] font-medium text-muted-foreground transition-colors hover:bg-muted/80 disabled:opacity-50"
          >
            {storage.isPersisting ? 'Verrouillage...' : 'Activer le stockage persistant'}
          </button>
        )}
        {storage.supported && storage.persisted === true && (
          <span className="mt-1 text-[10px] font-medium text-emerald-500">
            Stockage persistant actif
          </span>
        )}
      </div>
    </SidebarFooter>
  );
}
