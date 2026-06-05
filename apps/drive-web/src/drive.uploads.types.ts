export type UploadProgress = {
  fileName: string;
  fileSize: number;
  bytesUploaded: number;
  status: 'pending' | 'uploading' | 'completing' | 'done' | 'error';
  error?: string;
  uploadId?: string;
};

export type UploadItem = {
  file: File;
  name: string;
  mimeType: string;
  parentId?: string;
};

export type UploadSummary = {
  totalFiles: number;
  totalBytes: number;
  completedFiles: number;
  failedFiles: number;
};
