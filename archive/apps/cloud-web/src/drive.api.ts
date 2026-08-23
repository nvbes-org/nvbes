import { createHttpClient } from '@nvbes/http-client';
import { createVerifiedFetch } from '@nvbes/web-runtime';
import { z } from 'zod';

const DriveUserViewSchema = z.object({
  id: z.string(),
  email: z.string(),
  display_name: z.string(),
  email_verified_at: z.string().nullable(),
  status: z.string(),
});

const DriveWorkspaceViewSchema = z.object({
  id: z.string(),
  name: z.string(),
  workspace_type: z.string(),
  role: z.string(),
  trial_ends_at: z.string().nullable(),
});

const DriveMeResponseSchema = z.object({
  user: DriveUserViewSchema,
  workspaces: z.array(DriveWorkspaceViewSchema),
  current_session_id: z.string(),
  current_tenant_id: z.string().nullable(),
  current_organization_id: z.string().nullable(),
  current_workspace_id: z.string().nullable(),
});

const EmptyResponseSchema = z.undefined();

export type DriveUserView = z.infer<typeof DriveUserViewSchema>;
export type DriveWorkspaceView = z.infer<typeof DriveWorkspaceViewSchema>;
export type DriveMeResponse = z.infer<typeof DriveMeResponseSchema>;

function cloudServiceBaseUrl(): string {
  const configuredBaseUrl = import.meta.env.VITE_CLOUD_SERVICE_BASE_URL || '/api';
  if (configuredBaseUrl.startsWith('/')) {
    return `${window.location.origin}${configuredBaseUrl}`;
  }

  return configuredBaseUrl;
}

import { getValidAccessToken } from './drive.session';

export async function driveClient(accessToken?: string) {
  const token = accessToken || (await getValidAccessToken());
  const baseUrl = cloudServiceBaseUrl();
  const headers: Record<string, string> = {};
  if (token) {
    headers['Authorization'] = `Bearer ${token}`;
  }
  return createHttpClient({
    baseUrl,
    fetchImpl: createVerifiedFetch({
      allowedOrigins: [baseUrl],
      credentials: 'include',
    }),
    headers,
  });
}

export async function fetchDriveMe(accessToken: string): Promise<DriveMeResponse> {
  const client = await driveClient(accessToken);
  return client.get('/auth/me', DriveMeResponseSchema);
}

export async function logoutDrive(accessToken: string): Promise<void> {
  const client = await driveClient(accessToken);
  await client.post('/auth/logout', EmptyResponseSchema);
}
