export type DeveloperSession = {
  accessToken: string;
  refreshToken?: string;
  scope: string;
  tokenType: string;
};

const STORAGE_SESSION_KEY = 'nvbes_developer_session';

export function getDeveloperSession(): DeveloperSession | null {
  const storage = browserLocalStorage();
  const raw = storage?.getItem(STORAGE_SESSION_KEY);
  if (!raw) {
    return null;
  }

  try {
    return JSON.parse(raw) as DeveloperSession;
  } catch {
    clearDeveloperSession();
    return null;
  }
}

export function saveDeveloperSession(session: DeveloperSession): void {
  browserLocalStorage()?.setItem(STORAGE_SESSION_KEY, JSON.stringify(session));
}

export function clearDeveloperSession(): void {
  browserLocalStorage()?.removeItem(STORAGE_SESSION_KEY);
}

export function getDeveloperAccessToken(): string | null {
  return getDeveloperSession()?.accessToken ?? null;
}

function browserLocalStorage(): Storage | null {
  if (typeof localStorage === 'undefined' || typeof localStorage.getItem !== 'function') {
    return null;
  }

  return localStorage;
}
