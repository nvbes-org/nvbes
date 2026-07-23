import { z } from 'zod';

import { prepareProfileAvatar } from '../account.avatar-upload';
import { uploadProfileAvatarToPresignedUrl } from '../account.avatar-upload.transport';
import { identityHttpClient } from '../identity.http';
import { type UpdateProfileInput, UpdateProfileSchema } from './AccountPersonalInfoPage.shared';

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

export async function uploadProfileAvatar(file: File) {
  const uploadFile = await prepareProfileAvatar(file);

  return identityHttpClient
    .request<{ upload_url: string; object_key: string }>(
      '/auth/me/avatar',
      z.object({ upload_url: z.string().url(), object_key: z.string() }),
      {
        method: 'POST',
        body: { content_type: uploadFile.type, size_bytes: uploadFile.size },
      },
    )
    .then(async ({ upload_url }) => {
      await uploadProfileAvatarToPresignedUrl(upload_url, uploadFile);
    });
}

export function deleteProfileAvatar() {
  return identityHttpClient.delete('/auth/me/avatar', z.object({ success: z.boolean() }));
}
