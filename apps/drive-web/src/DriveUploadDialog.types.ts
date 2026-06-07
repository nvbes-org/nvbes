import type { UploadProgress, UploadSummary } from './drive.uploads.types';

export type DriveUploadDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  uploads: UploadProgress[];
  summary: UploadSummary;
  isUploading: boolean;
  onCancel: () => void;
  onClear: () => void;
  onUploadFiles: () => void;
  onUploadFolder: () => void;
};
