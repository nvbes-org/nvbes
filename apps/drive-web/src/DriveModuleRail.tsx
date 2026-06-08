import { Building2, FolderKanban, Share2, Shield, UserCircle2 } from 'lucide-react';
import type { LucideIcon } from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import type { DriveModuleId } from './DriveSectionNav';
import { cn } from './lib/classnames';

type DriveModule = {
  id: DriveModuleId;
  label: string;
  icon: LucideIcon;
};

const DRIVE_MODULES: DriveModule[] = [
  { id: 'drive', label: 'Drive', icon: FolderKanban },
  { id: 'sharing', label: 'Partage', icon: Share2 },
  { id: 'admin', label: 'Administration', icon: Shield },
  { id: 'account', label: 'Compte', icon: UserCircle2 },
];

export function DriveModuleRail({
  activeModule,
  onModuleChange,
}: {
  activeModule: DriveModuleId;
  onModuleChange: (moduleId: DriveModuleId) => void;
}) {
  return (
    <TooltipProvider>
      <aside className="hidden w-16 border-r border-border/70 bg-foreground text-background md:flex md:flex-col md:items-center">
        <div className="flex h-16 items-center justify-center">
          <div className="grid size-9 place-items-center rounded-2xl bg-background/12">
            <Building2 className="size-4" aria-hidden="true" />
          </div>
        </div>
        <nav className="flex flex-1 flex-col items-center gap-2 px-2 py-3" aria-label="Modules Drive">
          {DRIVE_MODULES.map((module) => {
            const Icon = module.icon;
            const isActive = module.id === activeModule;

            return (
              <Tooltip key={module.id}>
                <TooltipTrigger asChild>
                  <Button
                    type="button"
                    size="icon-lg"
                    variant="ghost"
                    aria-label={module.label}
                    aria-current={isActive ? 'page' : undefined}
                    className={cn(
                      'rounded-2xl text-background/70 hover:bg-background/10 hover:text-background',
                      isActive && 'bg-background text-foreground shadow-sm hover:bg-background hover:text-foreground',
                    )}
                    onClick={() => onModuleChange(module.id)}
                  >
                    <Icon className="size-4" aria-hidden="true" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent side="right">{module.label}</TooltipContent>
              </Tooltip>
            );
          })}
        </nav>
      </aside>
    </TooltipProvider>
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
