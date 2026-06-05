import { createHttpClient } from '@nvbes/http-client';
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

function driveApiBaseUrl(): string {
  return import.meta.env.VITE_DRIVE_API_BASE_URL || 'http://localhost:4000';
}

import { getValidAccessToken } from './drive.session';

export async function driveClient(accessToken?: string) {
  const token = accessToken || (await getValidAccessToken());
  const headers: Record<string, string> = {};
  if (token) {
    headers['Authorization'] = `Bearer ${token}`;
  }
  return createHttpClient({
    baseUrl: driveApiBaseUrl(),
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
