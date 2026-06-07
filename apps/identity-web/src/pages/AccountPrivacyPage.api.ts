import { z } from 'zod';

import { identityHttpClient } from '../identity.http';

const emptySchema = z.undefined();

export function exportAccountData(): Promise<void> {
  return identityHttpClient.post('/auth/me/export', emptySchema, {});
}

export function deleteAccount(): Promise<void> {
  return identityHttpClient.post('/auth/me/delete', emptySchema, {});
}
