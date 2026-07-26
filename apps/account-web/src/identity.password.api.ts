import { z } from 'zod';
import { identityHttpClient } from './identity.http';

const ForgotPasswordResultSchema = z.object({
  success: z.boolean(),
});

const ResetPasswordResultSchema = z.object({
  success: z.boolean(),
});

const ChangePasswordResultSchema = z.object({
  success: z.boolean(),
});

export interface ChangePasswordInput {
  new_password: string;
}

export type ForgotPasswordResult = z.infer<typeof ForgotPasswordResultSchema>;
export type ResetPasswordResult = z.infer<typeof ResetPasswordResultSchema>;
export type ChangePasswordResult = z.infer<typeof ChangePasswordResultSchema>;

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

export function changePassword(input: ChangePasswordInput): Promise<ChangePasswordResult> {
  return identityHttpClient.post('/auth/password/change', ChangePasswordResultSchema, {
    new_password: input.new_password,
  });
}
