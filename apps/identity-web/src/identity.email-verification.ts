import { z } from 'zod';
import { identityHttpClient } from './identity.http';

export type ResendVerificationResponse = {
  success: boolean;
  email_verified: boolean;
  verification_resend_available_at: string | null;
};

export type VerifyEmailResponse = {
  success: boolean;
};

const ResendVerificationResponseSchema = z.object({
  success: z.boolean(),
  email_verified: z.boolean(),
  verification_resend_available_at: z.string().nullable(),
});

const VerifyEmailResponseSchema = z.object({
  success: z.boolean(),
});

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
  return identityHttpClient.post('/auth/verify-email', VerifyEmailResponseSchema, { token });
}
