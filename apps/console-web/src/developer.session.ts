import { HttpError } from '@nvbes/http-client';
import { z } from 'zod';
import { identityHttpClient } from './identity.http';

const LogoutResponseSchema = z.undefined();

export function buildDeveloperLoginUrl(returnTo: string = window.location.href): string {
  const url = new URL('/login', identityWebBaseUrl());
  url.searchParams.set('return_to', returnTo);
  return url.toString();
}

export function redirectToDeveloperLogin(returnTo?: string): void {
  void import('./developer.auth').then(({ startDeveloperLogin }) => startDeveloperLogin(returnTo));
}

export async function logoutDeveloperSession(): Promise<void> {
  const { clearDeveloperAuth } = await import('./developer.auth');
  clearDeveloperAuth();
  await identityHttpClient.post('/auth/logout', LogoutResponseSchema);
}

export function isDeveloperAuthError(error: unknown): boolean {
  return error instanceof HttpError && (error.status === 401 || error.status === 403);
}

function identityWebBaseUrl(): string {
  return (import.meta.env.VITE_IDENTITY_WEB_BASE_URL || 'http://localhost:3001').replace(
    /\/+$/u,
    '',
  );
}
