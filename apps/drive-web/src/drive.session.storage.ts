import type { AccountSession } from './drive.session.client';

const STORAGE_SESSIONS_KEY = 'nvbes_drive_sessions';
const STORAGE_ACTIVE_ID_KEY = 'nvbes_drive_active_user_id';
const SESSION_STORAGE_KEYS = new Set([STORAGE_SESSIONS_KEY, STORAGE_ACTIVE_ID_KEY]);

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
  try {
    const raw = localStorage.getItem(STORAGE_SESSIONS_KEY);
    return raw ? JSON.parse(raw) : [];
  } catch {
    return [];
  }
}

export function saveSessions(sessions: AccountSession[]): void {
  localStorage.setItem(STORAGE_SESSIONS_KEY, JSON.stringify(sessions));
}

export function getActiveSessionId(): string | null {
  return localStorage.getItem(STORAGE_ACTIVE_ID_KEY);
}

export function setActiveSessionId(userId: string): void {
  localStorage.setItem(STORAGE_ACTIVE_ID_KEY, userId);
}

export function clearActiveSessionId(): void {
  localStorage.removeItem(STORAGE_ACTIVE_ID_KEY);
}

export function clearStoredSessions(): void {
  localStorage.removeItem(STORAGE_ACTIVE_ID_KEY);
  localStorage.removeItem(STORAGE_SESSIONS_KEY);
}

export function getActiveSession(): AccountSession | null {
  const sessions = getSessions();
  const activeId = getActiveSessionId();
  if (!activeId) {
    return sessions[0] || null;
  }
  return sessions.find((session) => session.userId === activeId) || sessions[0] || null;
}
