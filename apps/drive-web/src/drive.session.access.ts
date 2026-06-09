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

let refreshInFlight: Promise<string | null> | null = null;

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

  const active = getActiveSession();
  const currentTokenObj = identityRuntimeClient.token;
  const token = currentTokenObj?.accessToken ?? null;
  if (token && !isJwtExpired(token)) {
    return token;
  }

  if (!active?.refreshToken) {
    removeActiveSession(removeSession);
    return null;
  }

  if (!refreshInFlight) {
    refreshInFlight = refreshAndPersistAccessToken(active, saveSession, removeSession).finally(
      () => {
        refreshInFlight = null;
      },
    );
  }

  return refreshInFlight;
}

async function refreshAndPersistAccessToken(
  active: NonNullable<ReturnType<typeof getActiveSession>>,
  saveSession: SaveSessionFn,
  removeSession: RemoveSessionFn,
): Promise<string | null> {
  const refreshToken = active.refreshToken;
  if (!refreshToken) {
    removeActiveSession(removeSession);
    return null;
  }

  try {
    const refreshed = await refreshAccessToken(identityRuntimeClient, refreshToken);
    if (refreshed.accessToken) {
      saveSession(
        active.userId,
        active.email,
        active.name,
        refreshed.accessToken,
        refreshed.refreshToken || refreshToken,
      );
      return refreshed.accessToken;
    }
  } catch (refreshError: unknown) {
    console.error('Failed to refresh Drive token', refreshError);
    if (isUnauthorizedSessionError(refreshError)) {
      const current = getActiveSession();
      if (current?.userId === active.userId) {
        removeSession(active.userId);
      }
    }
  }
  return null;
}

function removeActiveSession(removeSession: RemoveSessionFn): void {
  const activeSession = getActiveSession();
  if (activeSession) {
    removeSession(activeSession.userId);
  }
}
