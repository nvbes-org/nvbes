import { describe, expect, it } from 'vite-plus/test';
import { toDroppedUploadFileInputs } from './drive.uploads.prepare';

describe('drive uploads drop', () => {
  it('converts dropped files into upload inputs for the current folder', () => {
    const brief = new File(['brief'], 'brief.pdf', { type: 'application/pdf' });
    const notes = new File(['notes'], 'notes.txt');

    const inputs = toDroppedUploadFileInputs([brief, notes], 'folder-clients');

    expect(inputs).toEqual([
      {
        file: brief,
        name: 'brief.pdf',
        mimeType: 'application/pdf',
        parentId: 'folder-clients',
      },
      {
        file: notes,
        name: 'notes.txt',
        mimeType: 'text/plain',
        parentId: 'folder-clients',
      },
    ]);
  });
});
