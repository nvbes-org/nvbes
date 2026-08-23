export type UploadFileInput = {
  file: File;
  name: string;
  mimeType: string;
  parentId?: string;
};

export type UploadProgressCallback = (progress: {
  bytesUploaded: number;
  totalBytes: number;
  phase: 'uploading' | 'completing';
}) => void;
