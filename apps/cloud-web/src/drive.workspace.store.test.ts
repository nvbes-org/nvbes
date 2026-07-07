import { describe, expect, it } from 'vite-plus/test';
import { createInitialDriveWorkspace } from './drive.workspace.mock';
import {
  createFolder,
  filterDriveEntries,
  restoreTrashEntry,
  revokeShareLink,
  selectEntry,
  selectEntryFromSecondaryAction,
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

  it('keeps existing selection when using the secondary action', () => {
    const state = createInitialDriveWorkspace();
    const selected = selectEntry(state, 'file-brief');
    const extended = selectEntryFromSecondaryAction(selected, 'file-contract');
    const preserved = selectEntryFromSecondaryAction(extended, 'file-contract');

    expect(extended.selectedEntryIds).toEqual(['file-brief', 'file-contract']);
    expect(extended.detailsSelection).toEqual({ type: 'entry', id: 'file-brief' });
    expect(preserved).toBe(extended);
  });

  it('creates folders in the current folder', () => {
    const state = createInitialDriveWorkspace();
    const next = createFolder(state, 'Dossier client');

    expect(next.entries.some((entry) => entry.name === 'Dossier client')).toBe(true);
  });

  it('revokes a shared link and records feedback', () => {
    const state = createInitialDriveWorkspace();
    const next = revokeShareLink(state, 'link-brief');

    expect(next.shareLinks.find((link) => link.id === 'link-brief')?.status).toBe('revoked');
  });

  it('restores a trashed entry', () => {
    const state = createInitialDriveWorkspace();
    const next = restoreTrashEntry(state, 'file-archive');

    expect(next.entries.find((entry) => entry.id === 'file-archive')?.status).toBe('active');
  });
});
