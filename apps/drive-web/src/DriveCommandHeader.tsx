import { Bell, CheckCircle2, Database, Search } from 'lucide-react';
import { Input } from '@/components/ui/input';
import { Progress } from '@/components/ui/progress';
import type { DriveBillingState, DriveToast } from './drive.workspace.types';
import { cn } from './lib/classnames';

function formatBytes(bytes: number): string {
  const gib = bytes / 1_073_741_824;
  if (gib >= 100) {
    return `${Math.round(gib)} Go`;
  }

  return `${gib.toFixed(1)} Go`;
}

function storagePercent(billing: DriveBillingState): number {
  if (billing.storageLimitBytes <= 0) {
    return 0;
  }

  return Math.min(100, Math.round((billing.storageUsedBytes / billing.storageLimitBytes) * 100));
}

export function DriveCommandHeader({
  workspaceName,
  sectionLabel,
  query,
  billing,
  toast,
  onQueryChange,
}: {
  workspaceName: string;
  sectionLabel: string;
  query: string;
  billing: DriveBillingState;
  toast: DriveToast | null;
  onQueryChange: (query: string) => void;
}) {
  const quotaPercent = storagePercent(billing);

  return (
    <header className="border-b border-border/70 bg-background/90 px-4 py-3 backdrop-blur md:px-6">
      <div className="grid gap-4 xl:grid-cols-[minmax(14rem,0.8fr)_minmax(18rem,1fr)_minmax(16rem,0.8fr)] xl:items-center">
        <div className="min-w-0">
          <p className="text-xs font-medium uppercase tracking-[0.18em] text-muted-foreground">{workspaceName}</p>
          <h1 className="truncate text-2xl font-semibold tracking-tight">{sectionLabel}</h1>
        </div>

        <label className="relative block">
          <span className="sr-only">Rechercher dans Drive</span>
          <Search className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            value={query}
            onChange={(event) => onQueryChange(event.target.value)}
            placeholder="Rechercher fichiers, membres, liens..."
            className="h-10 rounded-2xl bg-muted/50 pl-9"
          />
        </label>

        <div className="grid gap-3 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-center xl:grid-cols-1">
          <div className="rounded-2xl border bg-card px-3 py-2">
            <div className="mb-2 flex items-center justify-between gap-3 text-xs">
              <span className="inline-flex items-center gap-1.5 font-medium">
                <Database className="size-3.5" aria-hidden="true" />
                Quota
              </span>
              <span className="text-muted-foreground">
                {formatBytes(billing.storageUsedBytes)} / {formatBytes(billing.storageLimitBytes)}
              </span>
            </div>
            <Progress value={quotaPercent} />
          </div>

          <div className="flex flex-wrap items-center gap-2 text-xs">
            <span className="inline-flex items-center gap-1.5 rounded-full bg-emerald-500/10 px-2.5 py-1 font-medium text-emerald-700">
              <CheckCircle2 className="size-3.5" aria-hidden="true" />
              Synchronise
            </span>
            {toast ? (
              <span
                className={cn(
                  'inline-flex items-center gap-1.5 rounded-full px-2.5 py-1 font-medium',
                  toast.tone === 'error' && 'bg-destructive/10 text-destructive',
                  toast.tone === 'success' && 'bg-emerald-500/10 text-emerald-700',
                  toast.tone === 'info' && 'bg-blue-500/10 text-blue-700',
                )}
              >
                <Bell className="size-3.5" aria-hidden="true" />
                {toast.message}
              </span>
            ) : null}
          </div>
        </div>
      </div>
    </header>
  );
}
