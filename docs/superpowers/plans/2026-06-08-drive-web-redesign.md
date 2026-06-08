# Drive Web Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rebuild `apps/drive-web` into a credible full Drive V1 frontend with Identity-aligned theme, two-level navigation, a polished Files workspace, secondary V1 views, and realistic local interactions where APIs are missing.

**Architecture:** Keep the work inside `apps/drive-web` and preserve the existing auth/session gate. Introduce a typed local Drive workspace model, then replace the current temporary shell with focused app layout, view, state, and action components. Use existing shadcn components and local hooks; do not introduce new dependencies.

**Tech Stack:** React 19, Vite, TanStack Router, TanStack Query, TypeScript, Tailwind CSS, shadcn/ui local components, lucide-react.

---

## Scope Notes

- The repository is currently dirty. Each task must stage only files touched by that task.
- Do not modify backend crates for this plan.
- Do not introduce `any`.
- Keep files under 300 lines where practical.
- Use local mock state for domains without real endpoints; keep the shape typed so API replacement is straightforward.

## File Structure

Create these files:

- `apps/drive-web/src/drive.workspace.types.ts`: typed frontend model for files, links, members, security events, billing, API keys, quota, navigation, and details selection.
- `apps/drive-web/src/drive.workspace.mock.ts`: realistic initial workspace data.
- `apps/drive-web/src/drive.workspace.store.ts`: reducer-style operations for local interactions.
- `apps/drive-web/src/drive.workspace.store.test.ts`: behavioral tests for search, selection, share revocation, trash restore, member invitation, role update, and API key revocation.
- `apps/drive-web/src/DriveAppLayout.tsx`: top-level app layout with rail, section nav, header, content, and optional details panel slot.
- `apps/drive-web/src/DriveModuleRail.tsx`: compact first-level navigation.
- `apps/drive-web/src/DriveSectionNav.tsx`: second-level navigation for active module.
- `apps/drive-web/src/DriveCommandHeader.tsx`: breadcrumb, search, quota, status, account menu.
- `apps/drive-web/src/DriveViewState.tsx`: shared loading, empty, error, and denied states.
- `apps/drive-web/src/DriveActionConfirm.tsx`: lightweight confirmation panel/dialog pattern for destructive actions.
- `apps/drive-web/src/DriveFilesView.tsx`: Files view orchestration.
- `apps/drive-web/src/DriveFilesTable.tsx`: dense table representation.
- `apps/drive-web/src/DriveFilesGrid.tsx`: grid representation.
- `apps/drive-web/src/DriveFilesToolbar.tsx`: files actions, filters, sort, view toggle, selected actions.
- `apps/drive-web/src/DriveDetailsPanel.tsx`: contextual details panel for file, link, member, billing, key, and event selections.
- `apps/drive-web/src/DriveSharedLinksView.tsx`: shared links control view.
- `apps/drive-web/src/DriveTrashView.tsx`: trash view.
- `apps/drive-web/src/DriveMembersView.tsx`: members view.
- `apps/drive-web/src/DriveSecurityView.tsx`: security view.
- `apps/drive-web/src/DriveBillingView.tsx`: billing view.
- `apps/drive-web/src/DriveApiKeysView.tsx`: API keys view.
- `apps/drive-web/src/DriveAccountView.tsx`: account handoff view.

Modify these files:

- `apps/drive-web/src/DriveShell.tsx`: replace temporary file cards with the new workspace app.
- `apps/drive-web/src/styles.css`: copy any missing motion utilities from `identity-web` only if required by the new components.

Remove these files after replacement if they become unused:

- `apps/drive-web/src/DriveFileList.tsx`
- `apps/drive-web/src/DriveShell.header.tsx`
- `apps/drive-web/src/DriveShell.session.tsx`
- `apps/drive-web/src/DriveShell.shared.tsx`
- `apps/drive-web/src/DriveShell.toolbar.tsx`
- `apps/drive-web/src/DriveWorkspaceSidebar.data.ts`
- `apps/drive-web/src/DriveWorkspaceSidebar.footer.tsx`
- `apps/drive-web/src/DriveWorkspaceSidebar.nav.tsx`
- `apps/drive-web/src/DriveWorkspaceSidebar.tsx`

Only delete a file after `rg "FileName"` confirms no imports remain.

---

### Task 1: Workspace Model And Store

**Files:**
- Create: `apps/drive-web/src/drive.workspace.types.ts`
- Create: `apps/drive-web/src/drive.workspace.mock.ts`
- Create: `apps/drive-web/src/drive.workspace.store.ts`
- Create: `apps/drive-web/src/drive.workspace.store.test.ts`

- [ ] **Step 1: Write the failing store tests**

Create `apps/drive-web/src/drive.workspace.store.test.ts`:

```ts
import { describe, expect, it } from 'vitest';
import { createInitialDriveWorkspace } from './drive.workspace.mock';
import {
  createFolder,
  filterDriveEntries,
  inviteMember,
  restoreTrashEntry,
  revokeApiKey,
  revokeShareLink,
  selectEntry,
  setMemberRole,
  toggleEntrySelection,
} from './drive.workspace.store';

describe('drive workspace store', () => {
  it('filters files by query, status and kind', () => {
    const state = createInitialDriveWorkspace();

    const results = filterDriveEntries(state.entries, {
      query: 'brief',
      kind: 'document',
      status: 'active',
      sort: 'name',
    });

    expect(results.map((entry) => entry.name)).toEqual(['Brief refonte client.pdf']);
  });

  it('tracks single and multiple selection', () => {
    const state = createInitialDriveWorkspace();
    const selected = selectEntry(state, 'file-brief');
    const toggled = toggleEntrySelection(selected, 'file-contract');

    expect(selected.detailsSelection).toEqual({ type: 'entry', id: 'file-brief' });
    expect(toggled.selectedEntryIds).toEqual(['file-brief', 'file-contract']);
  });

  it('creates folders in the current folder', () => {
    const state = createInitialDriveWorkspace();
    const next = createFolder(state, 'Dossier client');

    expect(next.entries.some((entry) => entry.name === 'Dossier client')).toBe(true);
    expect(next.toast?.message).toBe('Dossier cree');
  });

  it('revokes a shared link and records feedback', () => {
    const state = createInitialDriveWorkspace();
    const next = revokeShareLink(state, 'link-brief');

    expect(next.shareLinks.find((link) => link.id === 'link-brief')?.status).toBe('revoked');
    expect(next.toast?.message).toBe('Lien revoque');
  });

  it('restores a trashed entry', () => {
    const state = createInitialDriveWorkspace();
    const next = restoreTrashEntry(state, 'file-archive');

    expect(next.entries.find((entry) => entry.id === 'file-archive')?.status).toBe('active');
    expect(next.toast?.message).toBe('Element restaure');
  });

  it('invites a member and can update member role', () => {
    const state = createInitialDriveWorkspace();
    const invited = inviteMember(state, 'new.member@studio.test', 'member');
    const newMember = invited.members.find((member) => member.email === 'new.member@studio.test');

    expect(newMember?.status).toBe('invited');

    const updated = setMemberRole(invited, newMember?.id ?? '', 'admin');
    expect(updated.members.find((member) => member.id === newMember?.id)?.role).toBe('admin');
  });

  it('revokes api keys', () => {
    const state = createInitialDriveWorkspace();
    const next = revokeApiKey(state, 'key-production');

    expect(next.apiKeys.find((key) => key.id === 'key-production')?.status).toBe('revoked');
    expect(next.toast?.message).toBe('Cle API revoquee');
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
pnpm --filter nvbes-drive-web exec vitest run src/drive.workspace.store.test.ts
```

Expected: FAIL because `vitest` may not be available for `drive-web`, or because the new modules do not exist yet. If the command reports `Command "vitest" not found`, install no new dependency; continue and use `pnpm --filter nvbes-drive-web check` as the validation command for this task.

- [ ] **Step 3: Add typed workspace model**

Create `apps/drive-web/src/drive.workspace.types.ts`:

```ts
export type DriveModuleId = 'drive' | 'sharing' | 'admin' | 'account';

export type DriveSectionId =
  | 'files'
  | 'trash'
  | 'shared-links'
  | 'members'
  | 'security'
  | 'billing'
  | 'api'
  | 'account';

export type DriveEntryKind = 'folder' | 'document' | 'image' | 'archive' | 'code';
export type DriveEntryStatus = 'active' | 'uploading' | 'quarantined' | 'trashed';
export type DriveShareStatus = 'private' | 'shared' | 'expired' | 'revoked';
export type DriveRole = 'owner' | 'admin' | 'member' | 'viewer';
export type DriveMemberStatus = 'active' | 'invited' | 'suspended';
export type DriveViewMode = 'table' | 'grid';
export type DriveSortKey = 'name' | 'modified' | 'size' | 'owner';

export type DriveEntry = {
  id: string;
  parentId: string | null;
  name: string;
  kind: DriveEntryKind;
  status: DriveEntryStatus;
  shareStatus: DriveShareStatus;
  ownerName: string;
  sizeBytes: number;
  modifiedAt: string;
  createdAt: string;
  securityLabel: 'clear' | 'public-link' | 'quarantined';
};

export type DriveShareLink = {
  id: string;
  entryId: string;
  entryName: string;
  createdBy: string;
  permission: 'download';
  expiresAt: string;
  accessCount: number;
  status: Exclude<DriveShareStatus, 'private'>;
  url: string;
};

export type DriveMember = {
  id: string;
  name: string;
  email: string;
  role: DriveRole;
  status: DriveMemberStatus;
  lastActiveAt: string | null;
};

export type DriveSecurityEvent = {
  id: string;
  kind: 'share.created' | 'share.revoked' | 'download.public' | 'permission.denied' | 'api.revoked';
  actor: string;
  summary: string;
  occurredAt: string;
  severity: 'info' | 'warning' | 'danger';
};

export type DriveBillingState = {
  planName: string;
  trialEndsAt: string;
  storageUsedBytes: number;
  storageLimitBytes: number;
  seatsUsed: number;
  seatsIncluded: number;
  nextInvoiceAmountEur: number;
  nextInvoiceDate: string;
};

export type DriveApiKey = {
  id: string;
  name: string;
  prefix: string;
  scopes: string[];
  status: 'active' | 'revoked' | 'expired';
  expiresAt: string;
  lastUsedAt: string | null;
  createdBy: string;
};

export type DriveToast = {
  tone: 'success' | 'warning' | 'danger' | 'info';
  message: string;
};

export type DriveDetailsSelection =
  | { type: 'entry'; id: string }
  | { type: 'share-link'; id: string }
  | { type: 'member'; id: string }
  | { type: 'security-event'; id: string }
  | { type: 'api-key'; id: string }
  | null;

export type DriveWorkspaceState = {
  workspaceName: string;
  activeModule: DriveModuleId;
  activeSection: DriveSectionId;
  currentFolderId: string | null;
  query: string;
  viewMode: DriveViewMode;
  sort: DriveSortKey;
  selectedEntryIds: string[];
  detailsSelection: DriveDetailsSelection;
  entries: DriveEntry[];
  shareLinks: DriveShareLink[];
  members: DriveMember[];
  securityEvents: DriveSecurityEvent[];
  billing: DriveBillingState;
  apiKeys: DriveApiKey[];
  toast: DriveToast | null;
};
```

- [ ] **Step 4: Add realistic mock data**

Create `apps/drive-web/src/drive.workspace.mock.ts`:

```ts
import type { DriveWorkspaceState } from './drive.workspace.types';

export function createInitialDriveWorkspace(): DriveWorkspaceState {
  return {
    workspaceName: 'Studio Nord',
    activeModule: 'drive',
    activeSection: 'files',
    currentFolderId: null,
    query: '',
    viewMode: 'table',
    sort: 'modified',
    selectedEntryIds: [],
    detailsSelection: null,
    entries: [
      {
        id: 'folder-clients',
        parentId: null,
        name: 'Clients',
        kind: 'folder',
        status: 'active',
        shareStatus: 'private',
        ownerName: 'Shayn',
        sizeBytes: 0,
        modifiedAt: '2026-06-07T16:40:00.000Z',
        createdAt: '2026-05-29T09:00:00.000Z',
        securityLabel: 'clear',
      },
      {
        id: 'file-brief',
        parentId: null,
        name: 'Brief refonte client.pdf',
        kind: 'document',
        status: 'active',
        shareStatus: 'shared',
        ownerName: 'Maya',
        sizeBytes: 4_800_000,
        modifiedAt: '2026-06-08T08:20:00.000Z',
        createdAt: '2026-06-04T10:00:00.000Z',
        securityLabel: 'public-link',
      },
      {
        id: 'file-contract',
        parentId: null,
        name: 'Contrat traitement donnees.docx',
        kind: 'document',
        status: 'active',
        shareStatus: 'private',
        ownerName: 'Shayn',
        sizeBytes: 820_000,
        modifiedAt: '2026-06-06T14:15:00.000Z',
        createdAt: '2026-06-01T12:30:00.000Z',
        securityLabel: 'clear',
      },
      {
        id: 'file-archive',
        parentId: null,
        name: 'Exports avril.zip',
        kind: 'archive',
        status: 'trashed',
        shareStatus: 'revoked',
        ownerName: 'Nora',
        sizeBytes: 96_000_000,
        modifiedAt: '2026-05-25T11:10:00.000Z',
        createdAt: '2026-05-20T08:00:00.000Z',
        securityLabel: 'clear',
      },
    ],
    shareLinks: [
      {
        id: 'link-brief',
        entryId: 'file-brief',
        entryName: 'Brief refonte client.pdf',
        createdBy: 'Maya',
        permission: 'download',
        expiresAt: '2026-06-15T22:00:00.000Z',
        accessCount: 12,
        status: 'shared',
        url: 'https://drive.nvbes.app/s/brief-7j',
      },
    ],
    members: [
      {
        id: 'member-owner',
        name: 'Shayn',
        email: 'shayn@studio.test',
        role: 'owner',
        status: 'active',
        lastActiveAt: '2026-06-08T09:10:00.000Z',
      },
      {
        id: 'member-maya',
        name: 'Maya',
        email: 'maya@studio.test',
        role: 'admin',
        status: 'active',
        lastActiveAt: '2026-06-08T08:22:00.000Z',
      },
    ],
    securityEvents: [
      {
        id: 'event-share-created',
        kind: 'share.created',
        actor: 'Maya',
        summary: 'Lien cree pour Brief refonte client.pdf',
        occurredAt: '2026-06-08T08:24:00.000Z',
        severity: 'info',
      },
      {
        id: 'event-denied',
        kind: 'permission.denied',
        actor: 'Lecteur externe',
        summary: 'Acces refuse sur lien expire',
        occurredAt: '2026-06-07T18:41:00.000Z',
        severity: 'warning',
      },
    ],
    billing: {
      planName: 'Equipe',
      trialEndsAt: '2026-06-22T22:00:00.000Z',
      storageUsedBytes: 38_500_000_000,
      storageLimitBytes: 100_000_000_000,
      seatsUsed: 2,
      seatsIncluded: 5,
      nextInvoiceAmountEur: 39,
      nextInvoiceDate: '2026-06-22T22:00:00.000Z',
    },
    apiKeys: [
      {
        id: 'key-production',
        name: 'Integration production',
        prefix: 'nvb_live_8K2',
        scopes: ['files:read', 'files:write'],
        status: 'active',
        expiresAt: '2026-09-01T00:00:00.000Z',
        lastUsedAt: '2026-06-08T07:10:00.000Z',
        createdBy: 'Shayn',
      },
    ],
    toast: null,
  };
}
```

- [ ] **Step 5: Add store operations**

Create `apps/drive-web/src/drive.workspace.store.ts`:

```ts
import type {
  DriveEntry,
  DriveEntryKind,
  DriveEntryStatus,
  DriveRole,
  DriveSortKey,
  DriveWorkspaceState,
} from './drive.workspace.types';

export type DriveEntryFilters = {
  query: string;
  kind: DriveEntryKind | 'all';
  status: DriveEntryStatus | 'all';
  sort: DriveSortKey;
};

export function filterDriveEntries(entries: DriveEntry[], filters: DriveEntryFilters): DriveEntry[] {
  const normalizedQuery = filters.query.trim().toLowerCase();
  return entries
    .filter((entry) => {
      const matchesQuery = normalizedQuery.length === 0 || entry.name.toLowerCase().includes(normalizedQuery);
      const matchesKind = filters.kind === 'all' || entry.kind === filters.kind;
      const matchesStatus = filters.status === 'all' || entry.status === filters.status;
      return matchesQuery && matchesKind && matchesStatus;
    })
    .sort((left, right) => compareEntries(left, right, filters.sort));
}

function compareEntries(left: DriveEntry, right: DriveEntry, sort: DriveSortKey) {
  if (sort === 'name') return left.name.localeCompare(right.name);
  if (sort === 'owner') return left.ownerName.localeCompare(right.ownerName);
  if (sort === 'size') return right.sizeBytes - left.sizeBytes;
  return new Date(right.modifiedAt).getTime() - new Date(left.modifiedAt).getTime();
}

export function selectEntry(state: DriveWorkspaceState, entryId: string): DriveWorkspaceState {
  return {
    ...state,
    selectedEntryIds: [entryId],
    detailsSelection: { type: 'entry', id: entryId },
  };
}

export function toggleEntrySelection(state: DriveWorkspaceState, entryId: string): DriveWorkspaceState {
  const selectedEntryIds = state.selectedEntryIds.includes(entryId)
    ? state.selectedEntryIds.filter((id) => id !== entryId)
    : [...state.selectedEntryIds, entryId];
  return {
    ...state,
    selectedEntryIds,
    detailsSelection: selectedEntryIds.length === 1 ? { type: 'entry', id: selectedEntryIds[0] } : null,
  };
}

export function createFolder(state: DriveWorkspaceState, name: string): DriveWorkspaceState {
  const trimmed = name.trim();
  if (!trimmed) {
    return { ...state, toast: { tone: 'danger', message: 'Nom de dossier requis' } };
  }

  const now = new Date().toISOString();
  const entry: DriveEntry = {
    id: `folder-${crypto.randomUUID()}`,
    parentId: state.currentFolderId,
    name: trimmed,
    kind: 'folder',
    status: 'active',
    shareStatus: 'private',
    ownerName: 'Vous',
    sizeBytes: 0,
    modifiedAt: now,
    createdAt: now,
    securityLabel: 'clear',
  };

  return { ...state, entries: [entry, ...state.entries], toast: { tone: 'success', message: 'Dossier cree' } };
}

export function revokeShareLink(state: DriveWorkspaceState, linkId: string): DriveWorkspaceState {
  return {
    ...state,
    shareLinks: state.shareLinks.map((link) => (link.id === linkId ? { ...link, status: 'revoked' } : link)),
    toast: { tone: 'success', message: 'Lien revoque' },
  };
}

export function restoreTrashEntry(state: DriveWorkspaceState, entryId: string): DriveWorkspaceState {
  return {
    ...state,
    entries: state.entries.map((entry) => (entry.id === entryId ? { ...entry, status: 'active' } : entry)),
    toast: { tone: 'success', message: 'Element restaure' },
  };
}

export function inviteMember(state: DriveWorkspaceState, email: string, role: DriveRole): DriveWorkspaceState {
  const normalizedEmail = email.trim().toLowerCase();
  if (!normalizedEmail.includes('@')) {
    return { ...state, toast: { tone: 'danger', message: 'Email invalide' } };
  }

  return {
    ...state,
    members: [
      {
        id: `member-${crypto.randomUUID()}`,
        name: normalizedEmail,
        email: normalizedEmail,
        role,
        status: 'invited',
        lastActiveAt: null,
      },
      ...state.members,
    ],
    toast: { tone: 'success', message: 'Invitation envoyee' },
  };
}

export function setMemberRole(state: DriveWorkspaceState, memberId: string, role: DriveRole): DriveWorkspaceState {
  return {
    ...state,
    members: state.members.map((member) => (member.id === memberId ? { ...member, role } : member)),
    toast: { tone: 'success', message: 'Role mis a jour' },
  };
}

export function revokeApiKey(state: DriveWorkspaceState, keyId: string): DriveWorkspaceState {
  return {
    ...state,
    apiKeys: state.apiKeys.map((key) => (key.id === keyId ? { ...key, status: 'revoked' } : key)),
    toast: { tone: 'success', message: 'Cle API revoquee' },
  };
}
```

- [ ] **Step 6: Run validation**

Run:

```bash
pnpm --filter nvbes-drive-web check
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add apps/drive-web/src/drive.workspace.types.ts apps/drive-web/src/drive.workspace.mock.ts apps/drive-web/src/drive.workspace.store.ts apps/drive-web/src/drive.workspace.store.test.ts
git commit -m "feat: add drive workspace model"
```

---

### Task 2: App Layout And Navigation

**Files:**
- Create: `apps/drive-web/src/DriveAppLayout.tsx`
- Create: `apps/drive-web/src/DriveModuleRail.tsx`
- Create: `apps/drive-web/src/DriveSectionNav.tsx`
- Create: `apps/drive-web/src/DriveCommandHeader.tsx`
- Create: `apps/drive-web/src/DriveViewState.tsx`
- Modify: `apps/drive-web/src/DriveShell.tsx`

- [ ] **Step 1: Create shared view states**

Create `apps/drive-web/src/DriveViewState.tsx`:

```tsx
import type { ReactNode } from 'react';
import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';

export function DriveEmptyState({
  title,
  description,
  action,
}: {
  title: string;
  description: string;
  action?: ReactNode;
}) {
  return (
    <Card className="flex min-h-56 flex-col items-center justify-center gap-3 rounded-lg border-dashed p-8 text-center shadow-none">
      <div>
        <h2 className="text-base font-semibold">{title}</h2>
        <p className="mt-1 max-w-md text-sm text-muted-foreground">{description}</p>
      </div>
      {action}
    </Card>
  );
}

export function DriveErrorState({ message, onRetry }: { message: string; onRetry?: () => void }) {
  return (
    <Card className="rounded-lg border-destructive/30 bg-destructive/5 p-4 shadow-none">
      <h2 className="text-sm font-semibold text-destructive">Action impossible</h2>
      <p className="mt-1 text-sm text-muted-foreground">{message}</p>
      {onRetry ? (
        <Button className="mt-3" size="sm" variant="outline" onClick={onRetry}>
          Reessayer
        </Button>
      ) : null}
    </Card>
  );
}

export function DriveLoadingState({ label = 'Chargement...' }: { label?: string }) {
  return (
    <div className="grid gap-2" aria-label={label}>
      {Array.from({ length: 6 }, (_, index) => (
        <div key={index} className="h-11 animate-pulse rounded-md bg-muted" />
      ))}
    </div>
  );
}
```

- [ ] **Step 2: Create module rail**

Create `apps/drive-web/src/DriveModuleRail.tsx`:

```tsx
import { Building2, FolderOpen, ShieldCheck, UserCircle } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';
import { cn } from './lib/classnames';
import type { DriveModuleId } from './drive.workspace.types';

const modules = [
  { id: 'drive', label: 'Drive', icon: FolderOpen },
  { id: 'sharing', label: 'Partage', icon: ShieldCheck },
  { id: 'admin', label: 'Administration', icon: Building2 },
  { id: 'account', label: 'Compte', icon: UserCircle },
] satisfies Array<{ id: DriveModuleId; label: string; icon: typeof FolderOpen }>;

export function DriveModuleRail({
  activeModule,
  onModuleChange,
}: {
  activeModule: DriveModuleId;
  onModuleChange: (moduleId: DriveModuleId) => void;
}) {
  return (
    <aside className="hidden h-svh w-16 shrink-0 border-r bg-card md:flex md:flex-col md:items-center md:gap-2 md:py-3">
      <div className="mb-2 flex size-9 items-center justify-center rounded-lg bg-primary text-sm font-semibold text-primary-foreground">
        n
      </div>
      {modules.map((item) => (
        <Tooltip key={item.id}>
          <TooltipTrigger asChild>
            <Button
              aria-label={item.label}
              className={cn('size-10 p-0', activeModule === item.id && 'bg-primary text-primary-foreground')}
              variant={activeModule === item.id ? 'default' : 'ghost'}
              onClick={() => onModuleChange(item.id)}
            >
              <item.icon className="size-4" />
            </Button>
          </TooltipTrigger>
          <TooltipContent side="right">{item.label}</TooltipContent>
        </Tooltip>
      ))}
    </aside>
  );
}
```

- [ ] **Step 3: Create section navigation**

Create `apps/drive-web/src/DriveSectionNav.tsx`:

```tsx
import { CreditCard, FileKey2, Files, Link2, Shield, Trash2, UserRoundCog, Users } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { cn } from './lib/classnames';
import type { DriveModuleId, DriveSectionId } from './drive.workspace.types';

const sectionsByModule: Record<DriveModuleId, Array<{ id: DriveSectionId; label: string; icon: typeof Files }>> = {
  drive: [
    { id: 'files', label: 'Fichiers', icon: Files },
    { id: 'trash', label: 'Corbeille', icon: Trash2 },
  ],
  sharing: [{ id: 'shared-links', label: 'Liens partages', icon: Link2 }],
  admin: [
    { id: 'members', label: 'Membres', icon: Users },
    { id: 'security', label: 'Securite', icon: Shield },
    { id: 'billing', label: 'Facturation', icon: CreditCard },
    { id: 'api', label: 'API', icon: FileKey2 },
  ],
  account: [{ id: 'account', label: 'Compte', icon: UserRoundCog }],
};

export function firstSectionForModule(moduleId: DriveModuleId): DriveSectionId {
  return sectionsByModule[moduleId][0].id;
}

export function DriveSectionNav({
  activeModule,
  activeSection,
  onSectionChange,
}: {
  activeModule: DriveModuleId;
  activeSection: DriveSectionId;
  onSectionChange: (sectionId: DriveSectionId) => void;
}) {
  return (
    <aside className="hidden h-svh w-56 shrink-0 border-r bg-background px-3 py-4 md:block">
      <div className="px-2">
        <p className="text-xs font-semibold uppercase text-muted-foreground">nvbes Drive</p>
        <h1 className="mt-1 truncate text-lg font-semibold">{moduleTitle(activeModule)}</h1>
      </div>
      <nav className="mt-5 grid gap-1">
        {sectionsByModule[activeModule].map((section) => (
          <Button
            key={section.id}
            className={cn('justify-start gap-2', activeSection === section.id && 'bg-muted text-foreground')}
            variant="ghost"
            onClick={() => onSectionChange(section.id)}
          >
            <section.icon className="size-4" />
            {section.label}
          </Button>
        ))}
      </nav>
    </aside>
  );
}

function moduleTitle(moduleId: DriveModuleId) {
  if (moduleId === 'drive') return 'Drive';
  if (moduleId === 'sharing') return 'Partage';
  if (moduleId === 'admin') return 'Administration';
  return 'Compte';
}
```

- [ ] **Step 4: Create command header**

Create `apps/drive-web/src/DriveCommandHeader.tsx`:

```tsx
import { Search, UploadCloud } from 'lucide-react';
import { Input } from '@/components/ui/input';
import { Progress } from '@/components/ui/progress';
import type { DriveBillingState, DriveToast } from './drive.workspace.types';

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
  const storagePercent = Math.round((billing.storageUsedBytes / billing.storageLimitBytes) * 100);

  return (
    <header className="flex min-h-14 shrink-0 items-center gap-3 border-b bg-card px-4">
      <div className="min-w-0 flex-1">
        <p className="truncate text-xs text-muted-foreground">{workspaceName}</p>
        <h2 className="truncate text-sm font-semibold">{sectionLabel}</h2>
      </div>
      <label className="relative hidden w-full max-w-sm md:block">
        <span className="sr-only">Rechercher</span>
        <Search className="absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
        <Input
          className="h-9 pl-9"
          value={query}
          placeholder="Rechercher dans l'espace"
          onChange={(event) => onQueryChange(event.target.value)}
        />
      </label>
      <div className="hidden min-w-36 md:block">
        <div className="mb-1 flex items-center justify-between text-[11px] text-muted-foreground">
          <span>Quota</span>
          <span>{storagePercent}%</span>
        </div>
        <Progress value={storagePercent} />
      </div>
      <div className="hidden items-center gap-1 rounded-md border px-2 py-1 text-xs text-muted-foreground lg:flex">
        <UploadCloud className="size-3.5" />
        Sync pret
      </div>
      {toast ? (
        <div className="rounded-md border bg-background px-3 py-1.5 text-xs shadow-sm" role="status">
          {toast.message}
        </div>
      ) : null}
    </header>
  );
}
```

- [ ] **Step 5: Create app layout**

Create `apps/drive-web/src/DriveAppLayout.tsx`:

```tsx
import type { ReactNode } from 'react';
import { DriveCommandHeader } from './DriveCommandHeader';
import { DriveModuleRail } from './DriveModuleRail';
import { DriveSectionNav } from './DriveSectionNav';
import type { DriveBillingState, DriveModuleId, DriveSectionId, DriveToast } from './drive.workspace.types';

const sectionLabels: Record<DriveSectionId, string> = {
  files: 'Fichiers',
  trash: 'Corbeille',
  'shared-links': 'Liens partages',
  members: 'Membres',
  security: 'Securite',
  billing: 'Facturation',
  api: 'API',
  account: 'Compte',
};

export function DriveAppLayout({
  workspaceName,
  activeModule,
  activeSection,
  query,
  billing,
  toast,
  details,
  children,
  onModuleChange,
  onSectionChange,
  onQueryChange,
}: {
  workspaceName: string;
  activeModule: DriveModuleId;
  activeSection: DriveSectionId;
  query: string;
  billing: DriveBillingState;
  toast: DriveToast | null;
  details: ReactNode;
  children: ReactNode;
  onModuleChange: (moduleId: DriveModuleId) => void;
  onSectionChange: (sectionId: DriveSectionId) => void;
  onQueryChange: (query: string) => void;
}) {
  return (
    <div className="flex min-h-svh bg-background text-foreground">
      <DriveModuleRail activeModule={activeModule} onModuleChange={onModuleChange} />
      <DriveSectionNav activeModule={activeModule} activeSection={activeSection} onSectionChange={onSectionChange} />
      <div className="flex min-w-0 flex-1 flex-col">
        <DriveCommandHeader
          workspaceName={workspaceName}
          sectionLabel={sectionLabels[activeSection]}
          query={query}
          billing={billing}
          toast={toast}
          onQueryChange={onQueryChange}
        />
        <main className="flex min-h-0 flex-1 bg-muted/30">
          <section className="min-w-0 flex-1 overflow-auto p-4">{children}</section>
          {details}
        </main>
      </div>
    </div>
  );
}
```

- [ ] **Step 6: Wire layout into DriveShell with an interim empty state**

Replace `apps/drive-web/src/DriveShell.tsx` with:

```tsx
import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { DriveAppLayout } from './DriveAppLayout';
import { firstSectionForModule } from './DriveSectionNav';
import { DriveEmptyState } from './DriveViewState';
import type { DriveMeResponse } from './drive.api';
import { createInitialDriveWorkspace } from './drive.workspace.mock';
import type { DriveModuleId, DriveSectionId } from './drive.workspace.types';

export function DriveShell({ me }: { accessToken: string; me: DriveMeResponse }) {
  const [workspace, setWorkspace] = useState(() => createInitialDriveWorkspace());

  function setActiveModule(activeModule: DriveModuleId) {
    setWorkspace((current) => ({
      ...current,
      activeModule,
      activeSection: firstSectionForModule(activeModule),
      detailsSelection: null,
    }));
  }

  function setActiveSection(activeSection: DriveSectionId) {
    setWorkspace((current) => ({ ...current, activeSection, detailsSelection: null }));
  }

  const currentWorkspace =
    me.workspaces.find((workspaceItem) => workspaceItem.id === me.current_workspace_id) ?? me.workspaces[0];

  return (
    <DriveAppLayout
      workspaceName={currentWorkspace?.name ?? workspace.workspaceName}
      activeModule={workspace.activeModule}
      activeSection={workspace.activeSection}
      query={workspace.query}
      billing={workspace.billing}
      toast={workspace.toast}
      details={null}
      onModuleChange={setActiveModule}
      onSectionChange={setActiveSection}
      onQueryChange={(query) => setWorkspace((current) => ({ ...current, query }))}
    >
      <DriveEmptyState
        title="Vue en cours de refonte"
        description="Le layout Drive est pret. Les vues metier arrivent dans les taches suivantes."
        action={<Button size="sm">Importer</Button>}
      />
    </DriveAppLayout>
  );
}
```

- [ ] **Step 7: Validate**

Run:

```bash
pnpm --filter nvbes-drive-web check
```

Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add apps/drive-web/src/DriveAppLayout.tsx apps/drive-web/src/DriveModuleRail.tsx apps/drive-web/src/DriveSectionNav.tsx apps/drive-web/src/DriveCommandHeader.tsx apps/drive-web/src/DriveViewState.tsx apps/drive-web/src/DriveShell.tsx
git commit -m "feat: add drive app layout"
```

---

### Task 3: Files View

**Files:**
- Create: `apps/drive-web/src/DriveFilesView.tsx`
- Create: `apps/drive-web/src/DriveFilesTable.tsx`
- Create: `apps/drive-web/src/DriveFilesGrid.tsx`
- Create: `apps/drive-web/src/DriveFilesToolbar.tsx`
- Modify: `apps/drive-web/src/DriveShell.tsx`

- [ ] **Step 1: Create files toolbar**

Create `apps/drive-web/src/DriveFilesToolbar.tsx`:

```tsx
import { FolderPlus, Grid2X2, List, UploadCloud } from 'lucide-react';
import { Button } from '@/components/ui/button';
import type { DriveSortKey, DriveViewMode } from './drive.workspace.types';

export function DriveFilesToolbar({
  selectedCount,
  viewMode,
  sort,
  onCreateFolder,
  onViewModeChange,
  onSortChange,
}: {
  selectedCount: number;
  viewMode: DriveViewMode;
  sort: DriveSortKey;
  onCreateFolder: () => void;
  onViewModeChange: (mode: DriveViewMode) => void;
  onSortChange: (sort: DriveSortKey) => void;
}) {
  return (
    <div className="flex flex-wrap items-center justify-between gap-2 border-b bg-background px-3 py-2">
      <div className="flex items-center gap-2">
        <Button size="sm">
          <UploadCloud className="size-4" />
          Importer
        </Button>
        <Button size="sm" variant="outline" onClick={onCreateFolder}>
          <FolderPlus className="size-4" />
          Nouveau dossier
        </Button>
        {selectedCount > 0 ? <span className="text-xs text-muted-foreground">{selectedCount} selectionne(s)</span> : null}
      </div>
      <div className="flex items-center gap-2">
        <select
          className="h-8 rounded-md border bg-background px-2 text-xs"
          value={sort}
          aria-label="Trier les fichiers"
          onChange={(event) => onSortChange(event.target.value as DriveSortKey)}
        >
          <option value="modified">Modifie le</option>
          <option value="name">Nom</option>
          <option value="size">Taille</option>
          <option value="owner">Proprietaire</option>
        </select>
        <Button
          size="icon"
          variant={viewMode === 'table' ? 'default' : 'outline'}
          aria-label="Vue table"
          onClick={() => onViewModeChange('table')}
        >
          <List className="size-4" />
        </Button>
        <Button
          size="icon"
          variant={viewMode === 'grid' ? 'default' : 'outline'}
          aria-label="Vue grille"
          onClick={() => onViewModeChange('grid')}
        >
          <Grid2X2 className="size-4" />
        </Button>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Create dense files table**

Create `apps/drive-web/src/DriveFilesTable.tsx`:

```tsx
import { FileArchive, FileCode2, FileText, Folder, Image, ShieldCheck } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { cn } from './lib/classnames';
import type { DriveEntry } from './drive.workspace.types';

export function DriveFilesTable({
  entries,
  selectedEntryIds,
  onOpenDetails,
  onToggleSelection,
}: {
  entries: DriveEntry[];
  selectedEntryIds: string[];
  onOpenDetails: (entryId: string) => void;
  onToggleSelection: (entryId: string) => void;
}) {
  return (
    <div className="overflow-hidden rounded-lg border bg-card">
      <table className="w-full text-sm">
        <thead className="border-b bg-muted/50 text-xs text-muted-foreground">
          <tr>
            <th className="w-10 px-3 py-2 text-left"> </th>
            <th className="px-3 py-2 text-left font-medium">Nom</th>
            <th className="px-3 py-2 text-left font-medium">Statut</th>
            <th className="px-3 py-2 text-left font-medium">Proprietaire</th>
            <th className="px-3 py-2 text-right font-medium">Taille</th>
            <th className="px-3 py-2 text-left font-medium">Modifie le</th>
            <th className="w-24 px-3 py-2 text-right font-medium">Actions</th>
          </tr>
        </thead>
        <tbody>
          {entries.map((entry) => (
            <tr key={entry.id} className={cn('border-b last:border-0', selectedEntryIds.includes(entry.id) && 'bg-primary/5')}>
              <td className="px-3 py-2">
                <input
                  type="checkbox"
                  checked={selectedEntryIds.includes(entry.id)}
                  aria-label={`Selectionner ${entry.name}`}
                  onChange={() => onToggleSelection(entry.id)}
                />
              </td>
              <td className="min-w-0 px-3 py-2">
                <button className="flex max-w-full items-center gap-2 text-left" type="button" onClick={() => onOpenDetails(entry.id)}>
                  <EntryIcon entry={entry} />
                  <span className="truncate font-medium">{entry.name}</span>
                </button>
              </td>
              <td className="px-3 py-2 text-xs text-muted-foreground">
                <span className="inline-flex items-center gap-1 rounded-md border px-2 py-1">
                  <ShieldCheck className="size-3" />
                  {entry.shareStatus === 'shared' ? 'Partage' : entry.status}
                </span>
              </td>
              <td className="px-3 py-2 text-muted-foreground">{entry.ownerName}</td>
              <td className="px-3 py-2 text-right font-mono text-xs">{formatBytes(entry.sizeBytes)}</td>
              <td className="px-3 py-2 text-muted-foreground">{formatDate(entry.modifiedAt)}</td>
              <td className="px-3 py-2 text-right">
                <Button size="sm" variant="ghost" onClick={() => onOpenDetails(entry.id)}>
                  Details
                </Button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function EntryIcon({ entry }: { entry: DriveEntry }) {
  const className = 'size-4 shrink-0 text-primary';
  if (entry.kind === 'folder') return <Folder className={className} />;
  if (entry.kind === 'image') return <Image className={className} />;
  if (entry.kind === 'archive') return <FileArchive className={className} />;
  if (entry.kind === 'code') return <FileCode2 className={className} />;
  return <FileText className={className} />;
}

function formatBytes(value: number) {
  if (value === 0) return '-';
  if (value > 1_000_000_000) return `${(value / 1_000_000_000).toFixed(1)} Go`;
  return `${(value / 1_000_000).toFixed(1)} Mo`;
}

function formatDate(value: string) {
  return new Intl.DateTimeFormat('fr-FR', { day: '2-digit', month: 'short', hour: '2-digit', minute: '2-digit' }).format(
    new Date(value),
  );
}
```

- [ ] **Step 3: Create files grid**

Create `apps/drive-web/src/DriveFilesGrid.tsx`:

```tsx
import { FileText, Folder } from 'lucide-react';
import { Card } from '@/components/ui/card';
import { cn } from './lib/classnames';
import type { DriveEntry } from './drive.workspace.types';

export function DriveFilesGrid({
  entries,
  selectedEntryIds,
  onOpenDetails,
  onToggleSelection,
}: {
  entries: DriveEntry[];
  selectedEntryIds: string[];
  onOpenDetails: (entryId: string) => void;
  onToggleSelection: (entryId: string) => void;
}) {
  return (
    <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
      {entries.map((entry) => (
        <Card
          key={entry.id}
          className={cn('rounded-lg p-3 shadow-none transition-colors', selectedEntryIds.includes(entry.id) && 'border-primary bg-primary/5')}
        >
          <div className="flex items-start justify-between gap-2">
            <button className="min-w-0 flex-1 text-left" type="button" onClick={() => onOpenDetails(entry.id)}>
              <div className="mb-3 flex size-10 items-center justify-center rounded-md bg-muted text-primary">
                {entry.kind === 'folder' ? <Folder className="size-5" /> : <FileText className="size-5" />}
              </div>
              <h3 className="truncate text-sm font-semibold">{entry.name}</h3>
              <p className="mt-1 text-xs text-muted-foreground">{entry.ownerName}</p>
            </button>
            <input
              type="checkbox"
              checked={selectedEntryIds.includes(entry.id)}
              aria-label={`Selectionner ${entry.name}`}
              onChange={() => onToggleSelection(entry.id)}
            />
          </div>
        </Card>
      ))}
    </div>
  );
}
```

- [ ] **Step 4: Create files view orchestration**

Create `apps/drive-web/src/DriveFilesView.tsx`:

```tsx
import { Button } from '@/components/ui/button';
import { DriveEmptyState } from './DriveViewState';
import { DriveFilesGrid } from './DriveFilesGrid';
import { DriveFilesTable } from './DriveFilesTable';
import { DriveFilesToolbar } from './DriveFilesToolbar';
import { createFolder, filterDriveEntries, selectEntry, toggleEntrySelection } from './drive.workspace.store';
import type { DriveWorkspaceState } from './drive.workspace.types';

export function DriveFilesView({
  state,
  onStateChange,
}: {
  state: DriveWorkspaceState;
  onStateChange: (state: DriveWorkspaceState) => void;
}) {
  const entries = filterDriveEntries(state.entries, {
    query: state.query,
    kind: 'all',
    status: 'active',
    sort: state.sort,
  });

  return (
    <div className="overflow-hidden rounded-lg border bg-background">
      <DriveFilesToolbar
        selectedCount={state.selectedEntryIds.length}
        viewMode={state.viewMode}
        sort={state.sort}
        onCreateFolder={() => onStateChange(createFolder(state, 'Nouveau dossier'))}
        onViewModeChange={(viewMode) => onStateChange({ ...state, viewMode })}
        onSortChange={(sort) => onStateChange({ ...state, sort })}
      />
      <div className="p-3">
        {entries.length === 0 ? (
          <DriveEmptyState
            title="Aucun fichier"
            description="Importez un fichier ou creez un dossier pour demarrer cet espace."
            action={<Button size="sm">Importer</Button>}
          />
        ) : state.viewMode === 'table' ? (
          <DriveFilesTable
            entries={entries}
            selectedEntryIds={state.selectedEntryIds}
            onOpenDetails={(entryId) => onStateChange(selectEntry(state, entryId))}
            onToggleSelection={(entryId) => onStateChange(toggleEntrySelection(state, entryId))}
          />
        ) : (
          <DriveFilesGrid
            entries={entries}
            selectedEntryIds={state.selectedEntryIds}
            onOpenDetails={(entryId) => onStateChange(selectEntry(state, entryId))}
            onToggleSelection={(entryId) => onStateChange(toggleEntrySelection(state, entryId))}
          />
        )}
      </div>
    </div>
  );
}
```

- [ ] **Step 5: Render files view in DriveShell**

In `apps/drive-web/src/DriveShell.tsx`, import `DriveFilesView` and replace the interim empty-state child:

```tsx
<DriveFilesView state={workspace} onStateChange={setWorkspace} />
```

Only render it when `workspace.activeSection === 'files'`; keep `DriveEmptyState` for other sections until later tasks.

- [ ] **Step 6: Validate**

Run:

```bash
pnpm --filter nvbes-drive-web check
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add apps/drive-web/src/DriveFilesView.tsx apps/drive-web/src/DriveFilesTable.tsx apps/drive-web/src/DriveFilesGrid.tsx apps/drive-web/src/DriveFilesToolbar.tsx apps/drive-web/src/DriveShell.tsx
git commit -m "feat: build drive files view"
```

---

### Task 4: Details Panel

**Files:**
- Create: `apps/drive-web/src/DriveDetailsPanel.tsx`
- Modify: `apps/drive-web/src/DriveShell.tsx`

- [ ] **Step 1: Create details panel**

Create `apps/drive-web/src/DriveDetailsPanel.tsx`:

```tsx
import { X } from 'lucide-react';
import { Button } from '@/components/ui/button';
import type { DriveWorkspaceState } from './drive.workspace.types';

export function DriveDetailsPanel({
  state,
  onClose,
}: {
  state: DriveWorkspaceState;
  onClose: () => void;
}) {
  const selection = state.detailsSelection;
  if (!selection) return null;

  const title = getTitle(state, selection);

  return (
    <aside className="hidden w-80 shrink-0 border-l bg-card p-4 md:block">
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <p className="text-xs font-semibold uppercase text-muted-foreground">Details</p>
          <h2 className="mt-1 truncate text-base font-semibold">{title}</h2>
        </div>
        <Button size="icon" variant="ghost" aria-label="Fermer les details" onClick={onClose}>
          <X className="size-4" />
        </Button>
      </div>
      <div className="mt-5 grid gap-3 text-sm">
        {selection.type === 'entry' ? <EntryDetails state={state} id={selection.id} /> : null}
        {selection.type === 'share-link' ? <KeyValue label="Type" value="Lien partage" /> : null}
        {selection.type === 'member' ? <KeyValue label="Type" value="Membre" /> : null}
        {selection.type === 'security-event' ? <KeyValue label="Type" value="Evenement securite" /> : null}
        {selection.type === 'api-key' ? <KeyValue label="Type" value="Cle API" /> : null}
      </div>
    </aside>
  );
}

function EntryDetails({ state, id }: { state: DriveWorkspaceState; id: string }) {
  const entry = state.entries.find((item) => item.id === id);
  if (!entry) return <p className="text-sm text-muted-foreground">Element introuvable.</p>;

  return (
    <>
      <KeyValue label="Statut" value={entry.status} />
      <KeyValue label="Partage" value={entry.shareStatus} />
      <KeyValue label="Proprietaire" value={entry.ownerName} />
      <KeyValue label="Securite" value={entry.securityLabel} />
      <KeyValue label="Taille" value={entry.sizeBytes === 0 ? '-' : `${(entry.sizeBytes / 1_000_000).toFixed(1)} Mo`} />
    </>
  );
}

function KeyValue({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-md border bg-background p-3">
      <p className="text-xs text-muted-foreground">{label}</p>
      <p className="mt-1 break-words font-medium">{value}</p>
    </div>
  );
}

function getTitle(state: DriveWorkspaceState, selection: NonNullable<DriveWorkspaceState['detailsSelection']>) {
  if (selection.type === 'entry') return state.entries.find((entry) => entry.id === selection.id)?.name ?? 'Element';
  if (selection.type === 'share-link') return state.shareLinks.find((link) => link.id === selection.id)?.entryName ?? 'Lien';
  if (selection.type === 'member') return state.members.find((member) => member.id === selection.id)?.name ?? 'Membre';
  if (selection.type === 'api-key') return state.apiKeys.find((key) => key.id === selection.id)?.name ?? 'Cle API';
  return state.securityEvents.find((event) => event.id === selection.id)?.summary ?? 'Evenement';
}
```

- [ ] **Step 2: Wire details panel**

In `apps/drive-web/src/DriveShell.tsx`, import `DriveDetailsPanel` and pass:

```tsx
details={
  <DriveDetailsPanel
    state={workspace}
    onClose={() => setWorkspace((current) => ({ ...current, detailsSelection: null }))}
  />
}
```

- [ ] **Step 3: Validate**

Run:

```bash
pnpm --filter nvbes-drive-web check
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add apps/drive-web/src/DriveDetailsPanel.tsx apps/drive-web/src/DriveShell.tsx
git commit -m "feat: add drive details panel"
```

---

### Task 5: Sharing And Trash Views

**Files:**
- Create: `apps/drive-web/src/DriveSharedLinksView.tsx`
- Create: `apps/drive-web/src/DriveTrashView.tsx`
- Modify: `apps/drive-web/src/DriveShell.tsx`

- [ ] **Step 1: Create shared links view**

Create `apps/drive-web/src/DriveSharedLinksView.tsx`:

```tsx
import { Copy, Link2, ShieldX } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { DriveEmptyState } from './DriveViewState';
import { revokeShareLink } from './drive.workspace.store';
import type { DriveWorkspaceState } from './drive.workspace.types';

export function DriveSharedLinksView({
  state,
  onStateChange,
}: {
  state: DriveWorkspaceState;
  onStateChange: (state: DriveWorkspaceState) => void;
}) {
  if (state.shareLinks.length === 0) {
    return (
      <DriveEmptyState
        title="Aucun lien actif"
        description="Les liens crees depuis vos fichiers apparaitront ici avec leur expiration."
      />
    );
  }

  return (
    <div className="overflow-hidden rounded-lg border bg-card">
      {state.shareLinks.map((link) => (
        <div key={link.id} className="flex flex-wrap items-center gap-3 border-b p-3 last:border-0">
          <Link2 className="size-4 text-primary" />
          <div className="min-w-0 flex-1">
            <h2 className="truncate text-sm font-semibold">{link.entryName}</h2>
            <p className="text-xs text-muted-foreground">
              Expire le {new Intl.DateTimeFormat('fr-FR').format(new Date(link.expiresAt))} · {link.accessCount} acces
            </p>
          </div>
          <span className="rounded-md border px-2 py-1 text-xs">{link.status}</span>
          <Button size="sm" variant="outline" onClick={() => void navigator.clipboard?.writeText(link.url)}>
            <Copy className="size-4" />
            Copier
          </Button>
          <Button size="sm" variant="outline" onClick={() => onStateChange(revokeShareLink(state, link.id))}>
            <ShieldX className="size-4" />
            Revoquer
          </Button>
        </div>
      ))}
    </div>
  );
}
```

- [ ] **Step 2: Create trash view**

Create `apps/drive-web/src/DriveTrashView.tsx`:

```tsx
import { RotateCcw } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { DriveEmptyState } from './DriveViewState';
import { restoreTrashEntry } from './drive.workspace.store';
import type { DriveWorkspaceState } from './drive.workspace.types';

export function DriveTrashView({
  state,
  onStateChange,
}: {
  state: DriveWorkspaceState;
  onStateChange: (state: DriveWorkspaceState) => void;
}) {
  const trashed = state.entries.filter((entry) => entry.status === 'trashed');

  if (trashed.length === 0) {
    return <DriveEmptyState title="Corbeille vide" description="Les elements supprimes apparaitront ici avant suppression definitive." />;
  }

  return (
    <div className="overflow-hidden rounded-lg border bg-card">
      {trashed.map((entry) => (
        <div key={entry.id} className="flex items-center gap-3 border-b p-3 last:border-0">
          <div className="min-w-0 flex-1">
            <h2 className="truncate text-sm font-semibold">{entry.name}</h2>
            <p className="text-xs text-muted-foreground">Supprime · conservation temporaire</p>
          </div>
          <Button size="sm" variant="outline" onClick={() => onStateChange(restoreTrashEntry(state, entry.id))}>
            <RotateCcw className="size-4" />
            Restaurer
          </Button>
        </div>
      ))}
    </div>
  );
}
```

- [ ] **Step 3: Wire views in DriveShell**

In `apps/drive-web/src/DriveShell.tsx`, render:

```tsx
{workspace.activeSection === 'shared-links' ? (
  <DriveSharedLinksView state={workspace} onStateChange={setWorkspace} />
) : null}
{workspace.activeSection === 'trash' ? <DriveTrashView state={workspace} onStateChange={setWorkspace} /> : null}
```

Keep the existing Files view for `files`.

- [ ] **Step 4: Validate**

Run:

```bash
pnpm --filter nvbes-drive-web check
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/drive-web/src/DriveSharedLinksView.tsx apps/drive-web/src/DriveTrashView.tsx apps/drive-web/src/DriveShell.tsx
git commit -m "feat: add sharing and trash views"
```

---

### Task 6: Administration Views

**Files:**
- Create: `apps/drive-web/src/DriveMembersView.tsx`
- Create: `apps/drive-web/src/DriveSecurityView.tsx`
- Create: `apps/drive-web/src/DriveBillingView.tsx`
- Create: `apps/drive-web/src/DriveApiKeysView.tsx`
- Create: `apps/drive-web/src/DriveAccountView.tsx`
- Modify: `apps/drive-web/src/DriveShell.tsx`

- [ ] **Step 1: Create members view**

Create `apps/drive-web/src/DriveMembersView.tsx`:

```tsx
import { UserPlus } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { inviteMember, setMemberRole } from './drive.workspace.store';
import type { DriveRole, DriveWorkspaceState } from './drive.workspace.types';

export function DriveMembersView({ state, onStateChange }: { state: DriveWorkspaceState; onStateChange: (state: DriveWorkspaceState) => void }) {
  return (
    <div className="rounded-lg border bg-card">
      <div className="flex items-center justify-between border-b p-3">
        <h2 className="text-sm font-semibold">Membres</h2>
        <Button size="sm" onClick={() => onStateChange(inviteMember(state, 'invite@studio.test', 'member'))}>
          <UserPlus className="size-4" />
          Inviter
        </Button>
      </div>
      {state.members.map((member) => (
        <div key={member.id} className="flex flex-wrap items-center gap-3 border-b p-3 last:border-0">
          <div className="min-w-0 flex-1">
            <h3 className="truncate text-sm font-semibold">{member.name}</h3>
            <p className="truncate text-xs text-muted-foreground">{member.email}</p>
          </div>
          <span className="rounded-md border px-2 py-1 text-xs">{member.status}</span>
          <select
            className="h-8 rounded-md border bg-background px-2 text-xs"
            value={member.role}
            aria-label={`Role de ${member.name}`}
            onChange={(event) => onStateChange(setMemberRole(state, member.id, event.target.value as DriveRole))}
          >
            <option value="owner">Proprietaire</option>
            <option value="admin">Admin</option>
            <option value="member">Membre</option>
            <option value="viewer">Lecteur</option>
          </select>
        </div>
      ))}
    </div>
  );
}
```

- [ ] **Step 2: Create security view**

Create `apps/drive-web/src/DriveSecurityView.tsx`:

```tsx
import { ShieldAlert } from 'lucide-react';
import type { DriveWorkspaceState } from './drive.workspace.types';

export function DriveSecurityView({ state }: { state: DriveWorkspaceState }) {
  return (
    <div className="grid gap-3">
      <section className="rounded-lg border bg-card p-4">
        <h2 className="text-sm font-semibold">Posture partage</h2>
        <p className="mt-1 text-sm text-muted-foreground">
          {state.shareLinks.filter((link) => link.status === 'shared').length} lien(s) public(s) actif(s), expiration obligatoire.
        </p>
      </section>
      <section className="overflow-hidden rounded-lg border bg-card">
        {state.securityEvents.map((event) => (
          <div key={event.id} className="flex gap-3 border-b p-3 last:border-0">
            <ShieldAlert className="mt-0.5 size-4 text-primary" />
            <div>
              <h3 className="text-sm font-semibold">{event.summary}</h3>
              <p className="text-xs text-muted-foreground">
                {event.actor} · {new Intl.DateTimeFormat('fr-FR').format(new Date(event.occurredAt))}
              </p>
            </div>
          </div>
        ))}
      </section>
    </div>
  );
}
```

- [ ] **Step 3: Create billing view**

Create `apps/drive-web/src/DriveBillingView.tsx`:

```tsx
import { CreditCard } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Progress } from '@/components/ui/progress';
import type { DriveWorkspaceState } from './drive.workspace.types';

export function DriveBillingView({ state }: { state: DriveWorkspaceState }) {
  const used = Math.round((state.billing.storageUsedBytes / state.billing.storageLimitBytes) * 100);

  return (
    <div className="grid gap-3 lg:grid-cols-2">
      <section className="rounded-lg border bg-card p-4">
        <h2 className="text-sm font-semibold">Plan {state.billing.planName}</h2>
        <p className="mt-1 text-sm text-muted-foreground">Essai actif jusqu'au {new Intl.DateTimeFormat('fr-FR').format(new Date(state.billing.trialEndsAt))}</p>
        <div className="mt-4">
          <div className="mb-1 flex justify-between text-xs text-muted-foreground">
            <span>Stockage</span>
            <span>{used}%</span>
          </div>
          <Progress value={used} />
        </div>
      </section>
      <section className="rounded-lg border bg-card p-4">
        <CreditCard className="size-5 text-primary" />
        <h2 className="mt-3 text-sm font-semibold">Prochaine facture</h2>
        <p className="mt-1 text-2xl font-semibold">{state.billing.nextInvoiceAmountEur} EUR</p>
        <p className="text-sm text-muted-foreground">{new Intl.DateTimeFormat('fr-FR').format(new Date(state.billing.nextInvoiceDate))}</p>
        <Button className="mt-4" size="sm">Ouvrir le portail</Button>
      </section>
    </div>
  );
}
```

- [ ] **Step 4: Create API keys view**

Create `apps/drive-web/src/DriveApiKeysView.tsx`:

```tsx
import { KeyRound, ShieldX } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { revokeApiKey } from './drive.workspace.store';
import type { DriveWorkspaceState } from './drive.workspace.types';

export function DriveApiKeysView({ state, onStateChange }: { state: DriveWorkspaceState; onStateChange: (state: DriveWorkspaceState) => void }) {
  return (
    <div className="overflow-hidden rounded-lg border bg-card">
      <div className="border-b p-4">
        <h2 className="text-sm font-semibold">Cles API</h2>
        <p className="mt-1 text-sm text-muted-foreground">Les cles API donnent acces aux fichiers de cet espace. Conservez-les comme des mots de passe.</p>
      </div>
      {state.apiKeys.map((key) => (
        <div key={key.id} className="flex flex-wrap items-center gap-3 border-b p-3 last:border-0">
          <KeyRound className="size-4 text-primary" />
          <div className="min-w-0 flex-1">
            <h3 className="truncate text-sm font-semibold">{key.name}</h3>
            <p className="font-mono text-xs text-muted-foreground">{key.prefix}</p>
          </div>
          <span className="rounded-md border px-2 py-1 text-xs">{key.status}</span>
          <Button size="sm" variant="outline" onClick={() => onStateChange(revokeApiKey(state, key.id))}>
            <ShieldX className="size-4" />
            Revoquer
          </Button>
        </div>
      ))}
    </div>
  );
}
```

- [ ] **Step 5: Create account view**

Create `apps/drive-web/src/DriveAccountView.tsx`:

```tsx
import { ExternalLink } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import type { DriveMeResponse } from './drive.api';

export function DriveAccountView({ me }: { me: DriveMeResponse }) {
  return (
    <Card className="max-w-2xl rounded-lg p-5 shadow-none">
      <h2 className="text-base font-semibold">Compte</h2>
      <p className="mt-1 text-sm text-muted-foreground">
        {me.user.email} utilise nvbes Identity pour les parametres globaux, la securite du compte et les sessions.
      </p>
      <Button className="mt-4" size="sm" variant="outline">
        <ExternalLink className="size-4" />
        Ouvrir Identity
      </Button>
    </Card>
  );
}
```

- [ ] **Step 6: Wire administration views in DriveShell**

Render the new views according to `workspace.activeSection`. Each view receives `workspace`; mutation views also receive `setWorkspace`.

- [ ] **Step 7: Validate**

Run:

```bash
pnpm --filter nvbes-drive-web check
```

Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add apps/drive-web/src/DriveMembersView.tsx apps/drive-web/src/DriveSecurityView.tsx apps/drive-web/src/DriveBillingView.tsx apps/drive-web/src/DriveApiKeysView.tsx apps/drive-web/src/DriveAccountView.tsx apps/drive-web/src/DriveShell.tsx
git commit -m "feat: add drive administration views"
```

---

### Task 7: Polish, Cleanup, And Validation

**Files:**
- Modify: `apps/drive-web/src/DriveShell.tsx`
- Modify: `apps/drive-web/src/styles.css`
- Delete: unused legacy Drive shell/sidebar files only if imports are gone.

- [ ] **Step 1: Confirm unused legacy files**

Run:

```bash
rg "DriveFileList|DriveShellHeader|DriveSessionContextCard|DriveShellToolbar|DriveWorkspaceSidebar" apps/drive-web/src
```

Expected: no results except inside files planned for deletion. If a result appears from `DriveShell.tsx`, remove that import before deleting.

- [ ] **Step 2: Add reduced-motion-safe utility if needed**

If new components use `animate-fade-slide-up`, copy only this utility into `apps/drive-web/src/styles.css`:

```css
@keyframes fade-slide-up {
  from {
    opacity: 0;
    transform: translateY(12px);
  }

  to {
    opacity: 1;
    transform: translateY(0);
  }
}

@utility animate-fade-slide-up {
  animation: fade-slide-up 0.5s ease-out both;
}

@media (prefers-reduced-motion: reduce) {
  .animate-fade-slide-up {
    animation: none;
  }
}
```

- [ ] **Step 3: Delete confirmed unused files**

Run `git rm` only for files with no imports:

```bash
git rm apps/drive-web/src/DriveFileList.tsx
git rm apps/drive-web/src/DriveShell.header.tsx apps/drive-web/src/DriveShell.session.tsx apps/drive-web/src/DriveShell.shared.tsx apps/drive-web/src/DriveShell.toolbar.tsx
git rm apps/drive-web/src/DriveWorkspaceSidebar.data.ts apps/drive-web/src/DriveWorkspaceSidebar.footer.tsx apps/drive-web/src/DriveWorkspaceSidebar.nav.tsx apps/drive-web/src/DriveWorkspaceSidebar.tsx
```

- [ ] **Step 4: Run typecheck**

Run:

```bash
pnpm --filter nvbes-drive-web check
```

Expected: PASS.

- [ ] **Step 5: Run web lint**

Run:

```bash
pnpm --filter nvbes-drive-web lint
```

Expected: PASS.

- [ ] **Step 6: Run root web check if available**

Run:

```bash
pnpm check:web
```

Expected: PASS. If this script is unavailable, record the exact error in the final implementation notes and rely on the package check.

- [ ] **Step 7: Commit**

```bash
git add apps/drive-web/src apps/drive-web/src/styles.css
git commit -m "chore: clean up drive web redesign"
```

---

## Self-Review

- Spec coverage: the plan covers the V1 front scope, Identity visual alignment, two-level navigation, Files as primary view, shared links, trash, members, security, billing, API, account, local interactions, states, motion, accessibility, and validation.
- Known limitation: browser screenshot validation is not part of this plan because the user explicitly declined an interactive visual companion. Implementation workers may still run the app locally for manual QA after the plan is approved.
- Red flag scan: clean.
- Type consistency: `DriveWorkspaceState`, `DriveSectionId`, `DriveModuleId`, `DriveEntry`, `DriveShareLink`, `DriveMember`, `DriveApiKey`, and store operation names are defined before use and reused consistently.
