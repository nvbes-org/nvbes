import { useCallback, useMemo } from 'react';

export type FilePickerOptions = {
  multiple?: boolean;
  types?: FilePickerType[];
  excludeAcceptAllOption?: boolean;
};

export type SaveFilePickerOptions = {
  suggestedName?: string;
  types?: FilePickerType[];
};

export type FilePickerType = {
  description: string;
  accept: Record<string, string[]>;
};

export type FileWithHandle = {
  file: File;
  handle: FileSystemFileHandle;
  name: string;
  relativePath: string;
};

export function useFileSystemAccess() {
  const supported = useMemo(
    () =>
      typeof window !== 'undefined' &&
      'showOpenFilePicker' in window &&
      'showDirectoryPicker' in window,
    [],
  );

  const openFiles = useCallback(
    async (options?: FilePickerOptions): Promise<FileWithHandle[]> => {
      if (!supported) {
        throw new Error('File System Access API is not supported in this browser.');
      }

      const handles = await window.showOpenFilePicker({
        multiple: options?.multiple ?? true,
        types: options?.types,
        excludeAcceptAllOption: options?.excludeAcceptAllOption ?? false,
      });

      const results: FileWithHandle[] = [];
      for (const handle of handles) {
        const file = await handle.getFile();
        results.push({
          file,
          handle,
          name: handle.name,
          relativePath: handle.name,
        });
      }
      return results;
    },
    [supported],
  );

  const openDirectory = useCallback(async (): Promise<FileWithHandle[]> => {
    if (!supported) {
      throw new Error('File System Access API is not supported in this browser.');
    }

    const handle = await window.showDirectoryPicker();
    return walkDirectory(handle, '');
  }, [supported]);

  const readFile = useCallback(async (handle: FileSystemFileHandle): Promise<File> => {
    return handle.getFile();
  }, []);

  const saveFile = useCallback(
    async (blob: Blob, options?: SaveFilePickerOptions): Promise<void> => {
      if (!supported) {
        throw new Error('File System Access API is not supported in this browser.');
      }

      const handle = await window.showSaveFilePicker({
        suggestedName: options?.suggestedName,
        types: options?.types,
      });

      const writable = await handle.createWritable();
      await writable.write(blob);
      await writable.close();
    },
    [supported],
  );

  return { supported, openFiles, openDirectory, readFile, saveFile };
}

async function walkDirectory(
  dirHandle: FileSystemDirectoryHandle,
  path: string,
): Promise<FileWithHandle[]> {
  const results: FileWithHandle[] = [];
  const prefix = path ? `${path}/` : '';

  for await (const [name, entry] of dirHandle.entries()) {
    if (entry.kind === 'file') {
      const fileHandle = entry as FileSystemFileHandle;
      const file = await fileHandle.getFile();
      results.push({
        file,
        handle: fileHandle,
        name,
        relativePath: `${prefix}${name}`,
      });
    } else if (entry.kind === 'directory') {
      const subResults = await walkDirectory(
        entry as FileSystemDirectoryHandle,
        `${prefix}${name}`,
      );
      results.push(...subResults);
    }
  }
  return results;
}
