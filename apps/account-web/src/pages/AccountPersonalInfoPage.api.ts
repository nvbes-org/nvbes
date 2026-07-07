import type { z } from 'zod';

import { identityHttpClient } from '../identity.http';
import { UpdateProfileSchema, type UpdateProfileInput } from './AccountPersonalInfoPage.shared';

export type UpdateProfileResponse = z.infer<typeof UpdateProfileSchema>;

export function updateProfile(input: UpdateProfileInput) {
  return identityHttpClient.request('/auth/me', UpdateProfileSchema, {
    method: 'PATCH',
    body: {
      firstname: input.firstname || undefined,
      lastname: input.lastname || undefined,
      username: input.username || undefined,
      birthdate: input.birthdate || undefined,
      region: input.region || undefined,
    },
  });
}
