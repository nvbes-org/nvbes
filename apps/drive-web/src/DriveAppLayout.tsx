import type { ReactNode } from 'react';
import { DriveCommandHeader } from './DriveCommandHeader';
import { DriveMobileModuleSwitcher, DriveModuleRail } from './DriveModuleRail';
import type { DriveBillingState } from './drive.workspace.types';
import type { DriveModuleId } from './drive.workspace.types';

export function DriveAppLayout({
  activeModule,
  workspaceName,
  sectionLabel,
  query,
  billing,
  topbarActions,
  workspaceSwitcher,
  details,
  children,
  onModuleChange,
  onQueryChange,
}: {
  activeModule: DriveModuleId;
  workspaceName: string;
  sectionLabel: string;
  query: string;
  billing: DriveBillingState;
  topbarActions?: ReactNode;
  workspaceSwitcher?: ReactNode;
  details?: ReactNode;
  children: ReactNode;
  onModuleChange: (moduleId: DriveModuleId) => void;
  onQueryChange: (query: string) => void;
}) {
  return (
    <div className="flex h-dvh flex-col bg-muted/30 text-foreground">
      <div className="flex min-h-0 flex-1">
        <DriveModuleRail
          activeModule={activeModule}
          billing={billing}
          onModuleChange={onModuleChange}
        />
        <div className="flex min-w-0 flex-1 flex-col">
          <DriveCommandHeader
            workspaceName={workspaceName}
            sectionLabel={sectionLabel}
            query={query}
            actions={topbarActions}
            workspaceSwitcher={workspaceSwitcher}
            onQueryChange={onQueryChange}
          />
          <DriveMobileModuleSwitcher activeModule={activeModule} onModuleChange={onModuleChange} />
          <div className="flex min-h-0 flex-1 flex-col md:flex-row">
            <main className="min-w-0 flex-1 p-4 md:p-2">{children}</main>
            {details ? (
              <aside className="max-h-full overflow-y-auto border-t border-border/70 bg-background/80 p-4 md:w-96 md:border-l md:border-t-0">
                {details}
              </aside>
            ) : null}
          </div>
        </div>
      </div>
    </div>
  );
}
