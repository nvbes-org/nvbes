import type { FileWithHandle } from './hooks/use-file-system-access';
import type { UploadFileInput } from './drive.native-fs.service';

function guessMimeType(fileName: string): string {
  const ext = fileName.split('.').pop()?.toLowerCase() ?? '';
  const mimeTypes: Record<string, string> = {
    jpg: 'image/jpeg',
    jpeg: 'image/jpeg',
    png: 'image/png',
    gif: 'image/gif',
    webp: 'image/webp',
    svg: 'image/svg+xml',
    pdf: 'application/pdf',
    doc: 'application/msword',
    docx: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
    xls: 'application/vnd.ms-excel',
    xlsx: 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
    zip: 'application/zip',
    tar: 'application/x-tar',
    gz: 'application/gzip',
    mp4: 'video/mp4',
    mp3: 'audio/mpeg',
    json: 'application/json',
    html: 'text/html',
    css: 'text/css',
    js: 'text/javascript',
    ts: 'text/typescript',
    md: 'text/markdown',
    txt: 'text/plain',
    csv: 'text/csv',
  };
  return mimeTypes[ext] ?? 'application/octet-stream';
}

export function toUploadFileInputs(files: FileWithHandle[], parentId?: string): UploadFileInput[] {
  return files.map((entry) => ({
    file: entry.file,
    name: entry.relativePath,
    mimeType: entry.file.type || guessMimeType(entry.name),
    parentId,
  }));
}

export function toDroppedUploadFileInputs(
  files: Iterable<File>,
  parentId?: string,
): UploadFileInput[] {
  return Array.from(files, (file) => ({
    file,
    name: file.name,
    mimeType: file.type || guessMimeType(file.name),
    parentId,
  }));
}
