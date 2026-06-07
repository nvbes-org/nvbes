import { Effect } from 'effect';
import type { DriveMeResponse } from './drive.api';
import { fetchDriveMe, logoutDrive } from './drive.api';
import { clearDriveSession, saveSession, identityClient } from './drive.session';
import { buildAuthorizationUrl, clearOauthStorage, readOauthStorage } from './drive.workflow.oauth';

export interface CompleteDriveCallbackResult {
  accessToken: string;
  tokenType: string;
  me: DriveMeResponse;
}

export function startDriveLoginWorkflow() {
  return Effect.tryPromise({
    try: async () => {
      window.location.assign(await buildAuthorizationUrl('login'));
    },
    catch: (error: unknown) => error,
  });
}

export function startDriveRegisterWorkflow() {
  return Effect.tryPromise({
    try: async () => {
      window.location.assign(await buildAuthorizationUrl('register'));
    },
    catch: (error: unknown) => error,
  });
}

export function completeDriveCallbackWorkflow(searchParams: URLSearchParams) {
  return Effect.tryPromise({
    try: async (): Promise<CompleteDriveCallbackResult> => {
      const code = searchParams.get('code');
      const state = searchParams.get('state');

      if (!code || !state) {
        throw new Error('Missing OAuth callback parameters.');
      }

      const stored = readOauthStorage();
      if (!stored || stored.state !== state) {
        clearOauthStorage();
        throw new Error('OAuth state mismatch.');
      }

      const token = await identityClient.exchangeCode(code, stored.codeVerifier);
      const me = await fetchDriveMe(token.accessToken);
      saveSession(
        me.user.id,
        me.user.email,
        me.user.display_name,
        token.accessToken,
        token.refreshToken,
      );
      clearOauthStorage();

      return {
        accessToken: token.accessToken,
        tokenType: token.tokenType,
        me,
      };
    },
    catch: (error: unknown) => error,
  });
}

export function fetchDriveMeWorkflow(accessToken: string) {
  return Effect.tryPromise({
    try: () => fetchDriveMe(accessToken),
    catch: (error: unknown) => error,
  });
}

export function logoutDriveWorkflow(accessToken: string | null) {
  return Effect.tryPromise({
    try: async () => {
      clearDriveSession();
      if (accessToken) {
        await logoutDrive(accessToken).catch(() => undefined);
      }
    },
    catch: (error: unknown) => error,
  });
}
