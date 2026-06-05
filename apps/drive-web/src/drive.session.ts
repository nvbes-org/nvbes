import { NvbesIdentity } from '@nvbes/identity-sdk';
import {
  type IdentityRuntimeClient,
  isJwtExpired,
  isUnauthorizedSessionError,
  refreshAccessToken,
} from './drive.session.tokens';

const driveOrigin = window.location.origin;
const identityWebBaseUrl = (
  import.meta.env.VITE_IDENTITY_WEB_BASE_URL || 'http://localhost:3001'
).replace(/\/+$/u, '');
const identityApiBaseUrl = (
  import.meta.env.VITE_IDENTITY_API_BASE_URL || 'http://localhost:8080'
).replace(/\/+$/u, '');
const clientId = import.meta.env.VITE_IDENTITY_CLIENT_ID;

if (!clientId) {
  throw new Error('VITE_IDENTITY_CLIENT_ID is required.');
}

export const identityClient = new NvbesIdentity({
  clientId,
  redirectUri: `${driveOrigin}/callback`,
  authorizationUrl: `${identityWebBaseUrl}/login`,
  tokenUrl: `${identityApiBaseUrl}/oauth/token`,
  userInfoUrl: `${identityApiBaseUrl}/oauth/userinfo`,
});

export interface AccountSession {
  userId: string;
  email: string;
  name: string;
  accessToken: string;
  refreshToken?: string;
}

const identityRuntimeClient = identityClient as unknown as IdentityRuntimeClient;

const STORAGE_SESSIONS_KEY = 'nvbes_drive_sessions';
const STORAGE_ACTIVE_ID_KEY = 'nvbes_drive_active_user_id';

const listeners = new Set<() => void>();

export function subscribeToSessionChanges(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

function notifySessionChanges(): void {
  for (const listener of listeners) {
    try {
      listener();
    } catch (e) {
      console.error('Session listener error:', e);
    }
  }
}

export function getSessions(): AccountSession[] {
  try {
    const raw = localStorage.getItem(STORAGE_SESSIONS_KEY);
    return raw ? JSON.parse(raw) : [];
  } catch {
    return [];
  }
}

export function getActiveSession(): AccountSession | null {
  const sessions = getSessions();
  const activeId = localStorage.getItem(STORAGE_ACTIVE_ID_KEY);
  if (!activeId) {
    return sessions[0] || null;
  }
  return sessions.find((s) => s.userId === activeId) || sessions[0] || null;
}

function setIdentityClientToken(session: AccountSession): void {
  identityRuntimeClient.token = {
    accessToken: session.accessToken,
    tokenType: 'Bearer',
    expiresIn: 3600,
    refreshToken: session.refreshToken,
    scope: 'openid profile email offline_access drive:read drive:write',
  };
}

export function saveSession(
  userId: string,
  email: string,
  name: string,
  accessToken: string,
  refreshToken?: string,
): void {
  const sessions = getSessions();
  const existingIndex = sessions.findIndex((s) => s.userId === userId);
  const newSession: AccountSession = { userId, email, name, accessToken, refreshToken };

  if (existingIndex > -1) {
    sessions[existingIndex] = newSession;
  } else {
    sessions.push(newSession);
  }

  localStorage.setItem(STORAGE_SESSIONS_KEY, JSON.stringify(sessions));
  localStorage.setItem(STORAGE_ACTIVE_ID_KEY, userId);

  setIdentityClientToken(newSession);
  notifySessionChanges();
}

export function setActiveSession(userId: string): void {
  const sessions = getSessions();
  const session = sessions.find((s) => s.userId === userId);
  if (session) {
    localStorage.setItem(STORAGE_ACTIVE_ID_KEY, userId);
    setIdentityClientToken(session);
    notifySessionChanges();
  }
}

export function removeSession(userId: string): void {
  let sessions = getSessions();
  sessions = sessions.filter((s) => s.userId !== userId);
  localStorage.setItem(STORAGE_SESSIONS_KEY, JSON.stringify(sessions));

  const activeId = localStorage.getItem(STORAGE_ACTIVE_ID_KEY);
  if (activeId === userId) {
    if (sessions.length > 0) {
      localStorage.setItem(STORAGE_ACTIVE_ID_KEY, sessions[0].userId);
      setIdentityClientToken(sessions[0]);
    } else {
      localStorage.removeItem(STORAGE_ACTIVE_ID_KEY);
      identityClient.logout();
    }
  }
  notifySessionChanges();
}

export function getAccessToken(): string | null {
  if (!identityClient.isAuthenticated()) {
    const active = getActiveSession();
    if (active) {
      setIdentityClientToken(active);
    }
  }

  if (identityClient.isAuthenticated()) {
    try {
      return identityRuntimeClient.token?.accessToken ?? null;
    } catch {
      return null;
    }
  }
  return null;
}

export async function getValidAccessToken(): Promise<string | null> {
  if (!identityClient.isAuthenticated()) {
    const active = getActiveSession();
    if (active) {
      setIdentityClientToken(active);
    }
  }

  if (identityClient.isAuthenticated() || identityRuntimeClient.token?.refreshToken) {
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
    } catch (e: unknown) {
      console.error('Failed to get valid access token via SDK', e);

      const active = getActiveSession();
      let isSessionRevoked = isUnauthorizedSessionError(e);

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
        } catch (refreshErr: unknown) {
          console.error('Failed to manually refresh token', refreshErr);
          if (isUnauthorizedSessionError(refreshErr)) {
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
  return null;
}

export function setAccessToken(token: string | null): void {
  if (!token) {
    clearDriveSession();
  }
}

export function clearDriveSession(): void {
  const active = getActiveSession();
  if (active) {
    removeSession(active.userId);
  } else {
    localStorage.removeItem(STORAGE_ACTIVE_ID_KEY);
    localStorage.removeItem(STORAGE_SESSIONS_KEY);
    identityClient.logout();
    notifySessionChanges();
  }
}
