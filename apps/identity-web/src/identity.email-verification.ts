import { z } from 'zod';
import { identityHttpClient } from './identity.http';

export type ResendVerificationResponse = {
  success: boolean;
  email_verified: boolean;
  verification_resend_available_at: string | null;
};

export type VerifyEmailResponse = {
  success: boolean;
  user?: {
    email?: string;
  };
};

const ResendVerificationResponseSchema = z.object({
  success: z.boolean(),
  email_verified: z.boolean(),
  verification_resend_available_at: z.string().nullable(),
});

const VerifyEmailResponseSchema = z.object({
  success: z.boolean(),
  user: z
    .object({
      email: z.string().optional(),
    })
    .passthrough()
    .optional(),
});

const VERIFY_EMAIL_RESULT_TTL_MS = 5 * 60 * 1000;
const verifyEmailTokenRequests = new Map<string, Promise<VerifyEmailResponse>>();

async function sha256Hex(value: string): Promise<string> {
  const digest = await globalThis.crypto.subtle.digest('SHA-256', new TextEncoder().encode(value));

  return Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, '0')).join('');
}

async function verifyEmailIdempotencyKey(token: string): Promise<string> {
  return `verify-email:${await sha256Hex(token)}`;
}

export async function resendVerificationEmail(email: string): Promise<ResendVerificationResponse> {
  return identityHttpClient.post('/auth/verify-email/resend', ResendVerificationResponseSchema, {
    email,
  });
}

export async function changeVerificationEmail(
  currentEmail: string,
  email: string,
): Promise<ResendVerificationResponse> {
  return identityHttpClient.post('/auth/verify-email/change', ResendVerificationResponseSchema, {
    current_email: currentEmail,
    email,
  });
}

export async function verifyEmailToken(token: string): Promise<VerifyEmailResponse> {
  const existing = verifyEmailTokenRequests.get(token);
  if (existing) {
    return existing;
  }

  const request = verifyEmailIdempotencyKey(token)
    .then((idempotencyKey) =>
      identityHttpClient.post(
        '/auth/verify-email',
        VerifyEmailResponseSchema,
        { token },
        { idempotencyKey },
      ),
    )
    .then((result) => {
      globalThis.setTimeout(() => {
        verifyEmailTokenRequests.delete(token);
      }, VERIFY_EMAIL_RESULT_TTL_MS);

      return result;
    })
    .catch((error: unknown) => {
      verifyEmailTokenRequests.delete(token);
      throw error;
    });

  verifyEmailTokenRequests.set(token, request);
  return request;
}
