import {
  identityClient,
  identityRuntimeClient,
  setIdentityClientToken,
} from './drive.session.client';
import { getActiveSession } from './drive.session.storage';
import {
  isJwtExpired,
  isUnauthorizedSessionError,
  refreshAccessToken,
} from './drive.session.tokens';

type SaveSessionFn = (
  userId: string,
  email: string,
  name: string,
  accessToken: string,
  refreshToken?: string,
) => void;
type RemoveSessionFn = (userId: string) => void;

function ensureIdentityClientToken() {
  if (!identityClient.isAuthenticated()) {
    const active = getActiveSession();
    if (active) {
      setIdentityClientToken(active);
    }
  }
}

export function getAccessToken(): string | null {
  ensureIdentityClientToken();

  if (!identityClient.isAuthenticated()) {
    return null;
  }

  try {
    return identityRuntimeClient.token?.accessToken ?? null;
  } catch {
    return null;
  }
}

export async function getValidAccessToken({
  saveSession,
  removeSession,
}: {
  saveSession: SaveSessionFn;
  removeSession: RemoveSessionFn;
}): Promise<string | null> {
  ensureIdentityClientToken();

  if (!(identityClient.isAuthenticated() || identityRuntimeClient.token?.refreshToken)) {
    return null;
  }

  try {
    const active = getActiveSession();
    const token = await identityClient.getAccessToken();
    if (token && isJwtExpired(token)) {
      throw { status: 401, message: 'Access token expired' };
    }

    const currentTokenObj = identityRuntimeClient.token;
    if (active && currentTokenObj && currentTokenObj.accessToken !== active.accessToken) {
      saveSession(
        active.userId,
        active.email,
        active.name,
        currentTokenObj.accessToken,
        currentTokenObj.refreshToken || active.refreshToken,
      );
    }
    return token;
  } catch (error: unknown) {
    console.error('Failed to get valid access token via SDK', error);

    const active = getActiveSession();
    let isSessionRevoked = isUnauthorizedSessionError(error);

    if (active && active.refreshToken) {
      try {
        const refreshed = await refreshAccessToken(identityRuntimeClient, active.refreshToken);
        if (refreshed && refreshed.accessToken) {
          saveSession(
            active.userId,
            active.email,
            active.name,
            refreshed.accessToken,
            refreshed.refreshToken || active.refreshToken,
          );
          return refreshed.accessToken;
        }
      } catch (refreshError: unknown) {
        console.error('Failed to manually refresh token', refreshError);
        if (isUnauthorizedSessionError(refreshError)) {
          isSessionRevoked = true;
        }
      }
    } else {
      isSessionRevoked = true;
    }

    if (isSessionRevoked) {
      const activeSession = getActiveSession();
      if (activeSession) {
        removeSession(activeSession.userId);
      }
    }
    return null;
  }
}
