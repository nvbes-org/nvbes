import { normalizeClientError } from '@nvbes/web-runtime';
import { HttpError } from '@nvbes/http-client';
import type { DriveMeResponse } from './drive.api';
import { fetchDriveMe, logoutDrive } from './drive.api';
import {
  clearDriveSession,
  getValidAccessToken,
  identityClient,
  saveSession,
} from './drive.session';
import { buildAuthorizationUrl, clearOauthStorage, readOauthStorage } from './drive.workflow.oauth';

export interface CompleteDriveCallbackResult {
  accessToken: string;
  tokenType: string;
  me: DriveMeResponse;
}

const meRequests = new Map<string, Promise<DriveMeResponse>>();

export async function startDriveLogin(): Promise<void> {
  window.location.assign(await buildAuthorizationUrl('login'));
}

export async function startDriveRegister(): Promise<void> {
  window.location.assign(await buildAuthorizationUrl('register'));
}

export async function completeDriveCallback(
  searchParams: URLSearchParams,
): Promise<CompleteDriveCallbackResult> {
  try {
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
  } catch (error) {
    throw normalizeClientError(error);
  }
}

export async function fetchDriveMeWithRefresh(accessToken: string): Promise<DriveMeResponse> {
  const inFlight = meRequests.get(accessToken);
  if (inFlight) {
    return inFlight;
  }

  const request = fetchDriveMeWithRefreshOnce(accessToken).finally(() => {
    meRequests.delete(accessToken);
  });
  meRequests.set(accessToken, request);
  return request;
}

async function fetchDriveMeWithRefreshOnce(accessToken: string): Promise<DriveMeResponse> {
  try {
    return await fetchDriveMe(accessToken);
  } catch (error: unknown) {
    if (!isInvalidTokenError(error)) {
      throw normalizeClientError(error);
    }

    const refreshedToken = await getValidAccessToken();
    if (!refreshedToken) {
      clearDriveSession();
      throw normalizeClientError(error);
    }

    return fetchDriveMe(refreshedToken);
  }
}

export async function logoutDriveSession(accessToken: string | null): Promise<void> {
  try {
    clearDriveSession();
    if (accessToken) {
      await logoutDrive(accessToken).catch(() => undefined);
    }
  } catch (error) {
    throw normalizeClientError(error);
  }
}

function isInvalidTokenError(error: unknown): boolean {
  if (!(error instanceof HttpError) || error.status !== 401) {
    return false;
  }

  const body = error.body;
  if (!body || typeof body !== 'object' || !('error' in body)) {
    return false;
  }

  const envelope = body.error;
  return (
    !!envelope &&
    typeof envelope === 'object' &&
    'code' in envelope &&
    envelope.code === 'invalid_token'
  );
}
