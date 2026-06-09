import { Building2, FolderKanban, Share2, Star, Trash2, Users } from 'lucide-react';
import type { LucideIcon } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Progress } from '@/components/ui/progress';
import type { DriveBillingState } from './drive.workspace.types';
import { cn } from './lib/classnames';
import type { DriveModuleId } from './drive.workspace.types';

type DriveModule = {
  id: DriveModuleId;
  label: string;
  icon: LucideIcon;
};

const DRIVE_MODULES: DriveModule[] = [
  { id: 'drive', label: 'Drive', icon: FolderKanban },
  { id: 'sharing', label: 'Partage', icon: Share2 },
  { id: 'shared-with-me', label: 'Partages avec moi', icon: Users },
  { id: 'starred', label: 'Suivis', icon: Star },
  { id: 'trash', label: 'Corbeille', icon: Trash2 },
];

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

export function DriveModuleRail({
  activeModule,
  billing,
  onModuleChange,
}: {
  activeModule: DriveModuleId;
  billing: DriveBillingState;
  onModuleChange: (moduleId: DriveModuleId) => void;
}) {
  const quotaPercent = storagePercent(billing);

  return (
    <aside className="hidden w-64 border-r border-border/70 bg-background text-foreground md:flex md:flex-col">
      <div className="flex h-16 items-center gap-3 border-b border-border/70 px-4">
        <div className="grid size-10 place-items-center rounded-2xl bg-primary/10 text-primary">
          <Building2 className="size-4" aria-hidden="true" />
        </div>
        <div className="min-w-0">
          <p className="text-[0.7rem] font-semibold uppercase tracking-[0.18em] text-muted-foreground">
            Workspace
          </p>
          <h2 className="truncate text-sm font-semibold">Drive</h2>
        </div>
      </div>

      <nav className="flex flex-1 flex-col gap-1 px-3 py-3" aria-label="Modules Drive">
        {DRIVE_MODULES.map((module) => {
          const Icon = module.icon;
          const isActive = module.id === activeModule;

          return (
            <Button
              key={module.id}
              type="button"
              variant={isActive ? 'secondary' : 'ghost'}
              aria-current={isActive ? 'page' : undefined}
              className={cn(
                'h-auto justify-start gap-2.5 rounded-lg px-3 py-2.5 text-left text-sm font-medium text-muted-foreground hover:bg-muted hover:text-foreground',
                isActive && 'bg-muted text-foreground shadow-sm',
              )}
              onClick={() => onModuleChange(module.id)}
            >
              <Icon className="size-4 shrink-0" aria-hidden="true" />
              <span className="truncate">{module.label}</span>
            </Button>
          );
        })}
      </nav>

      <div className="border-t border-border/70">
        <div className="px-3 py-3">
          <div className="mb-2 flex items-center justify-between gap-3 text-xs">
            <span className="inline-flex items-center gap-1.5 font-medium uppercase tracking-[0.18em] text-muted-foreground">
              Quota
            </span>
            <span className="text-right text-muted-foreground">
              {formatBytes(billing.storageUsedBytes)} / {formatBytes(billing.storageLimitBytes)}
            </span>
          </div>
          <Progress value={quotaPercent} />
        </div>
      </div>
    </aside>
  );
}

export function DriveMobileModuleSwitcher({
  activeModule,
  onModuleChange,
}: {
  activeModule: DriveModuleId;
  onModuleChange: (moduleId: DriveModuleId) => void;
}) {
  return (
    <nav
      className="flex gap-2 overflow-x-auto border-b border-border/70 bg-background/95 p-3 md:hidden"
      aria-label="Modules Drive"
    >
      {DRIVE_MODULES.map((module) => {
        const Icon = module.icon;
        const isActive = module.id === activeModule;

        return (
          <Button
            key={module.id}
            type="button"
            variant={isActive ? 'secondary' : 'outline'}
            className={cn('shrink-0 rounded-2xl', isActive && 'bg-muted text-foreground shadow-sm')}
            aria-current={isActive ? 'page' : undefined}
            onClick={() => onModuleChange(module.id)}
          >
            <Icon className="size-4" aria-hidden="true" />
            {module.label}
          </Button>
        );
      })}
    </nav>
  );
}
