import type {
  DriveEntry,
  DriveEntryKind,
  DriveEntryStatus,
  DriveSortKey,
  DriveWorkspaceState,
} from './drive.workspace.types';

const WORKSPACE_EVENT_TIME = '2026-06-08T10:00:00.000Z';

export type DriveEntryFilters = {
  query?: string;
  kind?: DriveEntryKind;
  status?: DriveEntryStatus;
  sort?: DriveSortKey;
};

export function filterDriveEntries(
  entries: DriveEntry[],
  filters: DriveEntryFilters,
): DriveEntry[] {
  const normalizedQuery = filters.query?.trim().toLocaleLowerCase() ?? '';

  return entries
    .filter((entry) => {
      const matchesQuery =
        normalizedQuery.length === 0 || entry.name.toLocaleLowerCase().includes(normalizedQuery);
      const matchesKind = filters.kind === undefined || entry.kind === filters.kind;
      const matchesStatus = filters.status === undefined || entry.status === filters.status;

      return matchesQuery && matchesKind && matchesStatus;
    })
    .sort((left, right) => compareDriveEntries(left, right, filters.sort ?? 'updated'));
}

export function selectEntry(state: DriveWorkspaceState, entryId: string): DriveWorkspaceState {
  return {
    ...state,
    selectedEntryIds: [entryId],
    detailsSelection: { type: 'entry', id: entryId },
  };
}

export function toggleEntrySelection(
  state: DriveWorkspaceState,
  entryId: string,
): DriveWorkspaceState {
  const isSelected = state.selectedEntryIds.includes(entryId);

  return {
    ...state,
    selectedEntryIds: isSelected
      ? state.selectedEntryIds.filter((selectedId) => selectedId !== entryId)
      : [...state.selectedEntryIds, entryId],
    detailsSelection: { type: 'entry', id: entryId },
  };
}

export function selectEntryFromSecondaryAction(
  state: DriveWorkspaceState,
  entryId: string,
): DriveWorkspaceState {
  if (state.selectedEntryIds.includes(entryId)) {
    return state;
  }

  if (state.selectedEntryIds.length === 0) {
    return selectEntry(state, entryId);
  }

  return {
    ...state,
    selectedEntryIds: [...state.selectedEntryIds, entryId],
  };
}

export function createFolder(state: DriveWorkspaceState, name: string): DriveWorkspaceState {
  const folderName = name.trim();
  const folderId = `folder-${slugifyFolderName(folderName)}-${state.entries.length + 1}`;

  return {
    ...state,
    entries: [
      ...state.entries,
      {
        id: folderId,
        parentId: state.currentFolderId,
        name: folderName,
        kind: 'folder',
        status: 'active',
        shareStatus: 'private',
        sizeBytes: 0,
        ownerId: 'member-owner',
        createdAt: WORKSPACE_EVENT_TIME,
        updatedAt: WORKSPACE_EVENT_TIME,
        starred: false,
        sharedWithCount: 0,
      },
    ],
  };
}

export function revokeShareLink(state: DriveWorkspaceState, linkId: string): DriveWorkspaceState {
  return {
    ...state,
    shareLinks: state.shareLinks.map((link) =>
      link.id === linkId ? { ...link, status: 'revoked' } : link,
    ),
  };
}

export function restoreTrashEntry(
  state: DriveWorkspaceState,
  entryId: string,
): DriveWorkspaceState {
  return {
    ...state,
    entries: state.entries.map((entry) =>
      entry.id === entryId
        ? {
            ...entry,
            status: 'active',
            updatedAt: WORKSPACE_EVENT_TIME,
          }
        : entry,
    ),
  };
}

function compareDriveEntries(left: DriveEntry, right: DriveEntry, sort: DriveSortKey): number {
  if (sort === 'name') {
    return left.name.localeCompare(right.name, 'fr');
  }

  if (sort === 'size') {
    return right.sizeBytes - left.sizeBytes;
  }

  if (sort === 'kind') {
    return left.kind.localeCompare(right.kind, 'fr') || left.name.localeCompare(right.name, 'fr');
  }

  return right.updatedAt.localeCompare(left.updatedAt);
}

function slugifyFolderName(name: string): string {
  const slug = name
    .toLocaleLowerCase()
    .replaceAll(/[^a-z0-9]+/g, '-')
    .replaceAll(/^-|-$/g, '');

  return slug.length > 0 ? slug : 'nouveau-dossier';
}
