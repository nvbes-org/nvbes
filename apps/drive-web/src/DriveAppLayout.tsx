import type { ReactNode } from 'react';
import { DriveCommandHeader } from './DriveCommandHeader';
import { DriveMobileModuleSwitcher, DriveModuleRail } from './DriveModuleRail';
import { DriveSectionNav } from './DriveSectionNav';
import type { DriveModuleId, DriveSectionId } from './DriveSectionNav';
import type { DriveBillingState, DriveToast } from './drive.workspace.types';

export function DriveAppLayout({
  activeModule,
  activeSection,
  workspaceName,
  sectionLabel,
  query,
  billing,
  toast,
  details,
  children,
  onModuleChange,
  onSectionChange,
  onQueryChange,
}: {
  activeModule: DriveModuleId;
  activeSection: DriveSectionId;
  workspaceName: string;
  sectionLabel: string;
  query: string;
  billing: DriveBillingState;
  toast: DriveToast | null;
  details?: ReactNode;
  children: ReactNode;
  onModuleChange: (moduleId: DriveModuleId) => void;
  onSectionChange: (sectionId: DriveSectionId) => void;
  onQueryChange: (query: string) => void;
}) {
  return (
    <div className="min-h-svh bg-muted/30 text-foreground">
      <div className="flex min-h-svh">
        <DriveModuleRail activeModule={activeModule} onModuleChange={onModuleChange} />
        <div className="flex min-w-0 flex-1 flex-col">
          <DriveCommandHeader
            workspaceName={workspaceName}
            sectionLabel={sectionLabel}
            query={query}
            billing={billing}
            toast={toast}
            onQueryChange={onQueryChange}
          />
          <DriveMobileModuleSwitcher activeModule={activeModule} onModuleChange={onModuleChange} />
          <div className="flex min-h-0 flex-1 flex-col md:flex-row">
            <DriveSectionNav
              activeModule={activeModule}
              activeSection={activeSection}
              onSectionChange={onSectionChange}
            />
            <main className="min-w-0 flex-1 p-4 md:p-6">{children}</main>
            {details ? (
              <aside className="border-t border-border/70 bg-background/80 p-4 md:w-80 md:border-l md:border-t-0">
                {details}
              </aside>
            ) : null}
          </div>
        </div>
      </div>
    </div>
  );
}
