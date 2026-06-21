import { getSafeLocalStorage } from '@nvbes/web-runtime';
import { z } from 'zod';

export type DeveloperSession = {
  accessToken: string;
  refreshToken?: string;
  scope: string;
  tokenType: string;
};

const STORAGE_SESSION_KEY = 'nvbes_developer_session';
const DeveloperSessionSchema = z.object({
  accessToken: z.string(),
  refreshToken: z.string().optional(),
  scope: z.string(),
  tokenType: z.string(),
});

export function getDeveloperSession(): DeveloperSession | null {
  const session = getSafeLocalStorage().getJson(STORAGE_SESSION_KEY, DeveloperSessionSchema);
  if (!session) {
    clearDeveloperSession();
  }
  return session;
}

export function saveDeveloperSession(session: DeveloperSession): void {
  getSafeLocalStorage().setJson(STORAGE_SESSION_KEY, session);
}

export function clearDeveloperSession(): void {
  getSafeLocalStorage().removeItem(STORAGE_SESSION_KEY);
}

export function getDeveloperAccessToken(): string | null {
  return getDeveloperSession()?.accessToken ?? null;
}
