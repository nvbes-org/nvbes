import { createRequestHeaders } from '@nvbes/http-client';
import { z } from 'zod';

const SwitchWorkspaceResultSchema = z.object({
  stepped_up: z.boolean(),
  workspace: z.object({
    id: z.string(),
    name: z.string(),
    workspace_type: z.string(),
    role: z.string(),
    trial_ends_at: z.string().datetime().nullable(),
  }),
  session: z.object({
    id: z.string(),
    current: z.boolean(),
  }),
});

function identityApiBaseUrl(): string {
  return (import.meta.env.VITE_IDENTITY_API_BASE_URL || 'http://localhost:8080').replace(
    /\/+$/u,
    '',
  );
}

export async function switchDriveWorkspace(accessToken: string, workspaceId: string): Promise<void> {
  const response = await fetch(`${identityApiBaseUrl()}/auth/workspaces/${workspaceId}/switch`, {
    method: 'POST',
    headers: createRequestHeaders('POST', {
      Authorization: `Bearer ${accessToken}`,
      'Content-Type': 'application/json',
    }),
    body: JSON.stringify({
      webauthn_response: {},
    }),
  });

  if (!response.ok) {
    let message = 'Impossible de changer de workspace.';

    try {
      const body = (await response.json()) as {
        error?: { message?: string };
        message?: string;
      };
      message = body.error?.message ?? body.message ?? message;
    } catch {
      // Ignore parse errors and keep the fallback message.
    }

    throw new Error(message);
  }

  SwitchWorkspaceResultSchema.parse(await response.json());
}
