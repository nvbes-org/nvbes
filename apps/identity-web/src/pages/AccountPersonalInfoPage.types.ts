import { z } from 'zod';

export const UpdateProfileSchema = z.object({
  user: z.object({
    id: z.string(),
    email: z.string(),
    display_name: z.string(),
    firstname: z.string().nullable().optional(),
    lastname: z.string().nullable().optional(),
    username: z.string().nullable().optional(),
    birthdate: z.string().nullable().optional(),
    region: z.string().nullable().optional(),
    email_verified: z.boolean(),
    mfa_enabled: z.boolean(),
    created_at: z.string(),
  }),
});

export interface UpdateProfileInput {
  firstname: string | null;
  lastname: string | null;
  username: string | null;
  birthdate: string | null;
  region: string | null;
}
