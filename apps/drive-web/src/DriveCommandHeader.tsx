import type { ReactNode } from 'react';
import { Search } from 'lucide-react';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';

export function DriveCommandHeader({
  workspaceName,
  sectionLabel,
  query,
  actions,
  workspaceSwitcher,
  onQueryChange,
}: {
  workspaceName: string;
  sectionLabel: string;
  query: string;
  actions?: ReactNode;
  workspaceSwitcher?: ReactNode;
  onQueryChange: (query: string) => void;
}) {
  return (
    <header className="border-b border-border/70 bg-background/90 px-3 py-2.5 backdrop-blur md:px-4 md:py-2">
      <div className="grid gap-3 xl:grid-cols-[minmax(12rem,0.8fr)_minmax(16rem,1fr)_auto] xl:items-center">
        <div className="min-w-0">
          {workspaceSwitcher ?? (
            <p className="text-[0.7rem] font-medium uppercase tracking-[0.18em] text-muted-foreground">
              {workspaceName}
            </p>
          )}
          <h1 className="truncate text-xl font-semibold tracking-tight md:text-[1.35rem]">
            {sectionLabel}
          </h1>
        </div>

        <Label className="relative block" htmlFor="drive-command-search">
          <span className="sr-only">Rechercher dans Drive</span>
          <Search className="pointer-events-none absolute left-3 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground" />
          <Input
            id="drive-command-search"
            value={query}
            onChange={(event) => onQueryChange(event.target.value)}
            placeholder="Rechercher fichiers, membres, liens..."
            className="h-9 rounded-xl bg-muted/50 pl-9 text-sm"
          />
        </Label>

        {actions ? <div className="justify-self-end xl:justify-self-auto">{actions}</div> : null}
      </div>
    </header>
  );
}
