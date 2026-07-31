import { AccountAuthenticationError, AccountHttpError } from '@nvbes/account-client';
import { useEffect } from 'react';
import { clearAccountAccessToken } from './account.oauth.access-token';

export function useAccountAuthenticationRecovery(error: unknown): void {
  useEffect(() => {
    if (
      error instanceof AccountAuthenticationError ||
      (error instanceof AccountHttpError && error.status === 401)
    ) {
      clearAccountAccessToken();
    }
  }, [error]);
}
