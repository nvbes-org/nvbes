import { z } from 'zod';
import { identityHttpClient } from '../identity.http';

export type RecoveryReviewView = {
  request_id: string;
  principal_id: string;
  email: string;
  status: string;
  available_at: string;
  approved_at: string | null;
  review_available_at: string | null;
  secondary_approved_at: string | null;
  created_at: string;
  updated_at: string;
};

const RecoveryReviewViewSchema = z.object({
  request_id: z.string(),
  principal_id: z.string(),
  email: z.string(),
  status: z.string(),
  available_at: z.string(),
  approved_at: z.string().nullable(),
  review_available_at: z.string().nullable(),
  secondary_approved_at: z.string().nullable(),
  created_at: z.string(),
  updated_at: z.string(),
});

const RecoveryReviewsResponseSchema = z.object({
  workspace_id: z.string(),
  reviews: z.array(RecoveryReviewViewSchema),
});

export async function listRecoveryReviews(
  workspaceId: string,
  signal?: AbortSignal,
): Promise<RecoveryReviewView[]> {
  const data = await identityHttpClient.get(
    `/workspaces/${workspaceId}/recovery-reviews`,
    RecoveryReviewsResponseSchema,
    {
      signal,
    },
  );

  return data.reviews || [];
}
