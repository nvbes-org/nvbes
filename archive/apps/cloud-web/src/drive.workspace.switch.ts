import { createRequestHeaders } from '@nvbes/http-client';
import { verifiedFetch } from '@nvbes/web-runtime';
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

function accountServiceBaseUrl(): string {
  return (import.meta.env.VITE_ACCOUNT_SERVICE_BASE_URL || 'http://localhost:8080').replace(
    /\/+$/u,
    '',
  );
}

export async function switchDriveWorkspace(
  accessToken: string,
  workspaceId: string,
): Promise<void> {
  const baseUrl = accountServiceBaseUrl();
  const response = await verifiedFetch(`${baseUrl}/auth/workspaces/${workspaceId}/switch`, {
    allowedOrigins: [baseUrl],
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
