import { getSafeLocalStorage } from '@nvbes/web-runtime';
import { z } from 'zod';
import type { AccountSession } from './drive.session.client';

const STORAGE_SESSIONS_KEY = 'nvbes_drive_sessions';
const STORAGE_ACTIVE_ID_KEY = 'nvbes_drive_active_user_id';
const SESSION_STORAGE_KEYS = new Set([STORAGE_SESSIONS_KEY, STORAGE_ACTIVE_ID_KEY]);
const AccountSessionSchema = z.object({
  accessToken: z.string(),
  email: z.string(),
  name: z.string(),
  refreshToken: z.string().optional(),
  userId: z.string(),
});
const AccountSessionsSchema = z.array(AccountSessionSchema);

const listeners = new Set<() => void>();
let storageListenerInstalled = false;

function handleStorageEvent(event: StorageEvent): void {
  if (event.storageArea !== window.localStorage) {
    return;
  }

  if (event.key !== null && !SESSION_STORAGE_KEYS.has(event.key)) {
    return;
  }

  notifySessionChanges();
}

function installStorageListener(): void {
  if (storageListenerInstalled || typeof window === 'undefined') {
    return;
  }

  window.addEventListener('storage', handleStorageEvent);
  storageListenerInstalled = true;
}

function removeStorageListener(): void {
  if (!storageListenerInstalled || typeof window === 'undefined') {
    return;
  }

  window.removeEventListener('storage', handleStorageEvent);
  storageListenerInstalled = false;
}

export function subscribeToSessionChanges(listener: () => void): () => void {
  installStorageListener();
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
    if (listeners.size === 0) {
      removeStorageListener();
    }
  };
}

export function notifySessionChanges(): void {
  for (const listener of listeners) {
    try {
      listener();
    } catch (error) {
      console.error('Session listener error:', error);
    }
  }
}

export function getSessions(): AccountSession[] {
  return getSafeLocalStorage().getJson(STORAGE_SESSIONS_KEY, AccountSessionsSchema) ?? [];
}

export function saveSessions(sessions: AccountSession[]): void {
  getSafeLocalStorage().setJson(STORAGE_SESSIONS_KEY, sessions);
}

export function getActiveSessionId(): string | null {
  return getSafeLocalStorage().getItem(STORAGE_ACTIVE_ID_KEY);
}

export function setActiveSessionId(userId: string): void {
  getSafeLocalStorage().setItem(STORAGE_ACTIVE_ID_KEY, userId);
}

export function clearActiveSessionId(): void {
  getSafeLocalStorage().removeItem(STORAGE_ACTIVE_ID_KEY);
}

export function clearStoredSessions(): void {
  const storage = getSafeLocalStorage();
  storage.removeItem(STORAGE_ACTIVE_ID_KEY);
  storage.removeItem(STORAGE_SESSIONS_KEY);
}

export function getActiveSession(): AccountSession | null {
  const sessions = getSessions();
  const activeId = getActiveSessionId();
  if (!activeId) {
    return sessions[0] || null;
  }
  return sessions.find((session) => session.userId === activeId) || sessions[0] || null;
}
