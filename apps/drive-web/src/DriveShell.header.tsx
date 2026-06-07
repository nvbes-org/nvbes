import { Bell, FileText, HelpCircle } from 'lucide-react';
import type { ReactNode } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';
import { DriveAccountMenu } from './DriveAccountMenu';
import type { DriveMeResponse } from './drive.api';

function IconButton({ label, children }: { label: string; children: ReactNode }) {
  return (
    <Tooltip>
      <TooltipTrigger
        render={
          <Button variant="outline" aria-label={label} className="size-9 rounded-full p-0">
            {children}
          </Button>
        }
      />
      <TooltipContent>{label}</TooltipContent>
    </Tooltip>
  );
}

export function DriveShellHeader({
  accessToken,
  user,
  search,
  deferredSearch,
  onSearchChange,
}: {
  accessToken: string;
  user: DriveMeResponse['user'];
  search: string;
  deferredSearch: string;
  onSearchChange: (value: string) => void;
}) {
  return (
    <header className="flex h-14 items-center justify-between border-b px-4">
      <div className="relative w-full max-w-[400px]">
        <FileText className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground" />
        <Input
          className="h-9 bg-background pl-10 text-sm shadow-sm"
          aria-label="Rechercher dans Mes fichiers"
          placeholder="Rechercher dans Mes fichiers (appuyez sur /)"
          value={search}
          onChange={(event) => onSearchChange(event.target.value)}
        />
        {deferredSearch !== search ? (
          <span className="absolute inset-y-0 right-3 flex items-center text-xs text-muted-foreground">
            Filtrage...
          </span>
        ) : null}
      </div>
      <div className="flex items-center gap-2">
        <IconButton label="Aide">
          <HelpCircle />
        </IconButton>
        <IconButton label="Notifications">
          <Bell />
        </IconButton>

        <DriveAccountMenu accessToken={accessToken} user={user} />
      </div>
    </header>
  );
}
