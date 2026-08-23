import {
  getAccessToken as resolveAccessToken,
  getValidAccessToken as resolveValidAccessToken,
} from './drive.session.access';
import {
  type AccountSession,
  identityClient,
  setIdentityClientToken,
} from './drive.session.client';
import {
  clearActiveSessionId,
  clearStoredSessions,
  getActiveSession,
  getActiveSessionId,
  getSessions,
  notifySessionChanges,
  saveSessions,
  setActiveSessionId,
} from './drive.session.storage';
export { subscribeToSessionChanges, getSessions } from './drive.session.storage';
export { drivePathForSession } from './drive.session.storage';
export { identityClient } from './drive.session.client';

export function saveSession(
  userId: string,
  email: string,
  name: string,
  accessToken: string,
  refreshToken?: string,
): void {
  const sessions = getSessions();
  const existingIndex = sessions.findIndex((session) => session.userId === userId);
  const newSession: AccountSession = { userId, email, name, accessToken, refreshToken };

  if (existingIndex > -1) {
    sessions[existingIndex] = newSession;
  } else {
    sessions.push(newSession);
  }

  saveSessions(sessions);
  setActiveSessionId(userId);

  setIdentityClientToken(newSession);
  notifySessionChanges();
}

export function setActiveSession(userId: string): void {
  const sessions = getSessions();
  const session = sessions.find((entry) => entry.userId === userId);
  if (session) {
    setActiveSessionId(userId);
    setIdentityClientToken(session);
    notifySessionChanges();
  }
}

export function removeSession(userId: string): void {
  let sessions = getSessions();
  sessions = sessions.filter((session) => session.userId !== userId);
  saveSessions(sessions);

  const activeId = getActiveSessionId();
  if (activeId === userId) {
    if (sessions.length > 0) {
      setActiveSessionId(sessions[0].userId);
      setIdentityClientToken(sessions[0]);
    } else {
      clearActiveSessionId();
      identityClient.logout();
    }
  }
  notifySessionChanges();
}

export function getAccessToken(): string | null {
  return resolveAccessToken();
}

export async function getValidAccessToken(): Promise<string | null> {
  return resolveValidAccessToken({
    saveSession,
    removeSession,
  });
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
    clearStoredSessions();
    identityClient.logout();
    notifySessionChanges();
  }
}
