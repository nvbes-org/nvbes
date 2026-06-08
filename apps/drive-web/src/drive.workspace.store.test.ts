import { describe, expect, it } from 'vite-plus/test';
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
