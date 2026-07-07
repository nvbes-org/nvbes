import { z } from 'zod';
import { driveClient } from './drive.api';

const RequiredHeaderSchema = z.object({
  name: z.string(),
  value: z.string(),
});

const SignedUploadUrlSchema = z.object({
  url: z.string(),
  method: z.string(),
  expires_at: z.string(),
  required_headers: z.array(RequiredHeaderSchema),
});

const UploadObjectViewSchema = z.object({
  id: z.string(),
  workspace_id: z.string(),
  parent_id: z.string().nullable(),
  name: z.string(),
  object_type: z.string(),
  status: z.string(),
  scan_status: z.string(),
  size_bytes: z.number(),
  mime_type: z.string().nullable(),
  checksum: z.string().nullable(),
  created_by: z.string(),
  created_by_principal_id: z.string(),
  created_at: z.string(),
  updated_at: z.string(),
});

const CreateUploadResponseSchema = z.object({
  upload_id: z.string(),
  storage_object: UploadObjectViewSchema,
  upload_url: SignedUploadUrlSchema,
  tus_url: z.string(),
  expires_at: z.string(),
});

const CompleteUploadResponseSchema = z.object({
  upload_id: z.string(),
  storage_object: UploadObjectViewSchema,
  activated_at: z.string(),
});

const CancelUploadResponseSchema = z.object({
  upload_id: z.string(),
  storage_object_id: z.string(),
  cancelled_at: z.string(),
});

export type SignedUploadUrl = z.infer<typeof SignedUploadUrlSchema>;
export type CreateUploadResponse = z.infer<typeof CreateUploadResponseSchema>;
export type CompleteUploadResponse = z.infer<typeof CompleteUploadResponseSchema>;
export type CancelUploadResponse = z.infer<typeof CancelUploadResponseSchema>;

export async function createUpload(
  workspaceId: string,
  input: {
    parentId?: string;
    name: string;
    mimeType: string;
    expectedSizeBytes: number;
    expectedChecksum?: string;
  },
): Promise<CreateUploadResponse> {
  const client = await driveClient();
  return client.post(`/workspaces/${workspaceId}/uploads`, CreateUploadResponseSchema, {
    parentId: input.parentId ?? null,
    name: input.name,
    mimeType: input.mimeType,
    expectedSizeBytes: input.expectedSizeBytes,
    expectedChecksum: input.expectedChecksum ?? null,
  });
}

export async function completeUpload(
  workspaceId: string,
  uploadId: string,
  input: { sizeBytes: number; checksum?: string },
): Promise<CompleteUploadResponse> {
  const client = await driveClient();
  return client.post(
    `/workspaces/${workspaceId}/uploads/${uploadId}/complete`,
    CompleteUploadResponseSchema,
    {
      sizeBytes: input.sizeBytes,
      checksum: input.checksum ?? null,
    },
  );
}

export async function cancelUpload(
  workspaceId: string,
  uploadId: string,
): Promise<CancelUploadResponse> {
  const client = await driveClient();
  return client.post(
    `/workspaces/${workspaceId}/uploads/${uploadId}/cancel`,
    CancelUploadResponseSchema,
  );
}
