# Drive Workspace Switcher Implementation Plan

> **Status: inactive future plan.** Cloud/Drive has not been selected as the
> first product. See the
> [active direction](../../product/nvbes-product-strategy.md).

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a workspace switcher dropdown in the Drive header allowing users to switch between workspaces (and implicitly tenants) without logging out.

**Architecture:** New `DriveWorkspaceSwitcher` component replaces the plain workspace name in `DriveCommandHeader`. On selection, calls the existing `switchDriveWorkspace()` function, then invalidates the TanStack Query `me` cache to re-render with the new workspace context.

**Tech Stack:** React, TanStack Query, shadcn/ui, existing `switchDriveWorkspace()` from `drive.workspace.switch.ts`

---

## File Structure

| File | Action | Responsibility |
|------|--------|---------------|
| `apps/cloud-web/src/DriveWorkspaceSwitcher.tsx` | Create | Dropdown component listing workspaces, handles switch action |
| `apps/cloud-web/src/DriveCommandHeader.tsx` | Modify | Accept optional `workspaceSwitcher` prop, render instead of plain text |
| `apps/cloud-web/src/DriveShell.tsx` | Modify | Instantiate `DriveWorkspaceSwitcher`, pass to header |

---

### Task 1: Create DriveWorkspaceSwitcher component

**Files:**
- Create: `apps/cloud-web/src/DriveWorkspaceSwitcher.tsx`
- Uses: `apps/cloud-web/src/drive.workspace.switch.ts`
- Uses: `apps/cloud-web/src/drive.queries.ts`
- Uses: `apps/cloud-web/src/drive.api.ts`

- [ ] **Step 1: Write DriveWorkspaceSwitcher.tsx**

```tsx
import { useCallback, useRef, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { ChevronDown } from 'lucide-react';
import { driveQueryKeys } from './drive.queries';
import { switchDriveWorkspace } from './drive.workspace.switch';
import type { DriveWorkspaceView } from './drive.api';

function useDriveWorkspaceSwitcher(accessToken: string) {
  const queryClient = useQueryClient();
  const [switchingId, setSwitchingId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const handleSwitch = useCallback(
    async (workspaceId: string) => {
      setError(null);
      setSwitchingId(workspaceId);
      try {
        await switchDriveWorkspace(accessToken, workspaceId);
        queryClient.invalidateQueries({ queryKey: driveQueryKeys.me(accessToken) });
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Erreur inattendue');
      } finally {
        setSwitchingId(null);
      }
    },
    [accessToken, queryClient],
  );

  return { switchingId, error, handleSwitch };
}

export function DriveWorkspaceSwitcher({
  accessToken,
  workspaceName,
  workspaceId,
  workspaces,
}: {
  accessToken: string;
  workspaceName: string;
  workspaceId: string | null;
  workspaces: DriveWorkspaceView[];
}) {
  const [open, setOpen] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);
  const { switchingId, error, handleSwitch } = useDriveWorkspaceSwitcher(accessToken);

  return (
    <div ref={menuRef} className="relative">
      <button
        type="button"
        onClick={() => setOpen((prev) => !prev)}
        className="group flex items-center gap-1 text-[0.7rem] font-medium uppercase tracking-[0.18em] text-muted-foreground hover:text-foreground transition-colors"
      >
        {workspaceName}
        <ChevronDown
          className={`size-3 transition-transform ${open ? 'rotate-180' : ''}`}
        />
      </button>

      {open && (
        <>
          <div
            className="fixed inset-0 z-40"
            onClick={() => setOpen(false)}
          />
          <div className="absolute left-0 top-full z-50 mt-1 w-56 rounded-lg border border-border/70 bg-popover p-1 shadow-lg">
            {error && (
              <p className="px-2 py-1.5 text-xs text-destructive">{error}</p>
            )}
            {workspaces.map((ws) => {
              const isActive = ws.id === workspaceId;
              const isSwitching = switchingId === ws.id;
              return (
                <button
                  key={ws.id}
                  type="button"
                  disabled={isActive || isSwitching}
                  onClick={() => {
                    if (!isActive && !isSwitching) {
                      void handleSwitch(ws.id);
                    }
                  }}
                  className={`flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-sm transition-colors ${
                    isActive
                      ? 'bg-accent font-medium text-accent-foreground'
                      : 'text-popover-foreground hover:bg-accent/50'
                  } disabled:cursor-not-allowed`}
                >
                  <span className="flex-1 truncate">{ws.name}</span>
                  {isActive ? (
                    <span className="text-xs text-muted-foreground">Actif</span>
                  ) : isSwitching ? (
                    <span className="size-3 animate-spin rounded-full border-2 border-foreground border-t-transparent" />
                  ) : null}
                </button>
              );
            })}
          </div>
        </>
      )}
    </div>
  );
}
```

---

### Task 2: Modify DriveCommandHeader to accept workspaceSwitcher prop

**Files:**
- Modify: `apps/cloud-web/src/DriveCommandHeader.tsx`

- [ ] **Step 1: Update props and rendering**

Add `workspaceSwitcher` as an optional prop. When provided, render it instead of the plain workspace name `<p>`.

```tsx
import type { ReactNode } from 'react';
import { Search } from 'lucide-react';
import { Input } from '@/components/ui/input';

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

        <label className="relative block">
          <span className="sr-only">Rechercher dans Drive</span>
          <Search className="pointer-events-none absolute left-3 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground" />
          <Input
            value={query}
            onChange={(event) => onQueryChange(event.target.value)}
            placeholder="Rechercher fichiers, membres, liens..."
            className="h-9 rounded-xl bg-muted/50 pl-9 text-sm"
          />
        </label>

        {actions ? <div className="justify-self-end xl:justify-self-auto">{actions}</div> : null}
      </div>
    </header>
  );
}
```

---

### Task 3: Modify DriveShell to instantiate and pass the switcher

**Files:**
- Modify: `apps/cloud-web/src/DriveShell.tsx`

- [ ] **Step 1: Add DriveWorkspaceSwitcher import and instantiation**

```tsx
import { useState } from 'react';
import { DriveAccountMenu } from './DriveAccountMenu';
import { DriveAppLayout } from './DriveAppLayout';
import { DriveDetailsPanel } from './DriveDetailsPanel';
import { DriveFilesView } from './DriveFilesView';
import { DriveSharedLinksView } from './DriveSharedLinksView';
import { DriveTrashView } from './DriveTrashView';
import { DriveWorkspaceSwitcher } from './DriveWorkspaceSwitcher';
import type { DriveMeResponse } from './drive.api';
import { createInitialDriveWorkspace } from './drive.workspace.mock';
import type { DriveWorkspaceState } from './drive.workspace.types';

const MOCK_WORKSPACE_NAME = 'Workspace personnel';

function labelForModule(moduleId: DriveWorkspaceState['activeModuleId']): string {
  if (moduleId === 'sharing') return 'Liens partages';
  if (moduleId === 'trash') return 'Corbeille';
  return 'Fichiers';
}

export function DriveShell({ accessToken, me }: { accessToken: string; me: DriveMeResponse }) {
  const [workspace, setWorkspace] = useState<DriveWorkspaceState>(() => createInitialDriveWorkspace());
  const currentWorkspace =
    me.workspaces.find((workspace) => workspace.id === me.current_workspace_id) ?? me.workspaces[0];
  const workspaceName = currentWorkspace?.name ?? MOCK_WORKSPACE_NAME;

  function handleModuleChange(moduleId: DriveWorkspaceState['activeModuleId']) {
    setWorkspace((current) => ({ ...current, activeModuleId: moduleId }));
  }

  return (
    <DriveAppLayout
      activeModule={workspace.activeModuleId}
      workspaceName={workspaceName}
      sectionLabel={labelForModule(workspace.activeModuleId)}
      query={workspace.query}
      billing={workspace.billing}
      topbarActions={<DriveAccountMenu accessToken={accessToken} user={me.user} />}
      workspaceSwitcher={
        <DriveWorkspaceSwitcher
          accessToken={accessToken}
          workspaceName={workspaceName}
          workspaceId={me.current_workspace_id}
          workspaces={me.workspaces}
        />
      }
      details={
        workspace.detailsSelection ? (
          <DriveDetailsPanel
            selection={workspace.detailsSelection}
            members={workspace.members}
            onStateChange={setWorkspace}
          />
        ) : null
      }
      onModuleChange={handleModuleChange}
      onQueryChange={(query) => setWorkspace((current) => ({ ...current, query }))}
    >
      {workspace.activeModuleId === 'trash' ? (
        <DriveTrashView state={workspace} onStateChange={setWorkspace} />
      ) : workspace.activeModuleId === 'sharing' ? (
        <DriveSharedLinksView state={workspace} onStateChange={setWorkspace} />
      ) : (
        <DriveFilesView state={workspace} onStateChange={setWorkspace} />
      )}
    </DriveAppLayout>
  );
}
```

- [ ] **Step 2: Update DriveAppLayout to pass workspaceSwitcher through**

```tsx
// In DriveAppLayout.tsx, update the props to include workspaceSwitcher
// and pass it to DriveCommandHeader

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
    <div className="min-h-svh bg-muted/30 text-foreground">
      <div className="flex min-h-svh">
        <DriveModuleRail activeModule={activeModule} billing={billing} onModuleChange={onModuleChange} />
        <div className="flex min-w-0 flex-1 flex-col">
          <DriveCommandHeader
            workspaceName={workspaceName}
            sectionLabel={sectionLabel}
            query={query}
            actions={topbarActions}
            workspaceSwitcher={workspaceSwitcher}
            onQueryChange={onQueryChange}
          />
          // ... rest unchanged
```

- [ ] **Step 3: Verify compilation**

Run: `pnpm check:web` or at minimum check `apps/cloud-web` for TypeScript errors

---

## Verification

- [ ] Run `pnpm lint:web` to confirm no lint errors
- [ ] Run `pnpm check:web` to confirm no type errors
- [ ] Verify the switcher renders correct workspace list in the header
- [ ] Verify switching workspace invalidates the me query and re-renders
