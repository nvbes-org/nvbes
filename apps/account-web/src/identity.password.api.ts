import { z } from 'zod';
import { identityHttpClient } from './identity.http';

const ForgotPasswordResultSchema = z.object({
  success: z.boolean(),
  requires_admin_approval: z.boolean(),
  available_at: z.string().nullable().optional(),
});

const ResetPasswordResultSchema = z.object({
  success: z.boolean(),
});

const EmptySchema = z.undefined();

export interface ChangePasswordInput {
  current_password: string;
  new_password: string;
}

export type ForgotPasswordResult = z.infer<typeof ForgotPasswordResultSchema>;
export type ResetPasswordResult = z.infer<typeof ResetPasswordResultSchema>;

export function forgotPassword(email: string): Promise<ForgotPasswordResult> {
  return identityHttpClient.post('/auth/password/forgot', ForgotPasswordResultSchema, {
    email,
  });
}

export function resetPassword(token: string, newPassword: string): Promise<ResetPasswordResult> {
  return identityHttpClient.post('/auth/password/reset', ResetPasswordResultSchema, {
    token,
    new_password: newPassword,
  });
}

export function changePassword(input: ChangePasswordInput): Promise<void> {
  return identityHttpClient.post('/auth/password/change', EmptySchema, {
    current_password: input.current_password,
    new_password: input.new_password,
  });
}
