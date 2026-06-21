type DeveloperOAuthState = {
  state: string;
  codeVerifier: string;
  returnTo: string;
};

const OAUTH_STORAGE_KEY = 'nvbes_developer_oauth_state';

export function readDeveloperOauthState(): DeveloperOAuthState | null {
  const raw = sessionStorage.getItem(OAUTH_STORAGE_KEY);
  if (!raw) {
    return null;
  }

  try {
    return JSON.parse(raw) as DeveloperOAuthState;
  } catch {
    clearDeveloperOauthState();
    return null;
  }
}

export function clearDeveloperOauthState(): void {
  sessionStorage.removeItem(OAUTH_STORAGE_KEY);
}

export async function buildDeveloperAuthorizationUrl(returnTo: string): Promise<string> {
  const { codeVerifier, codeChallenge } = await createPkceChallenge();
  const state = createRandomValue(24);
  sessionStorage.setItem(OAUTH_STORAGE_KEY, JSON.stringify({ state, codeVerifier, returnTo }));

  const { developerIdentityClient } = await import('./developer.oauth.client');
  return developerIdentityClient.getAuthorizationUrl(
    'openid profile email offline_access',
    state,
    codeChallenge,
    'S256',
  );
}

async function createPkceChallenge(): Promise<{ codeVerifier: string; codeChallenge: string }> {
  const codeVerifier = createRandomValue(48);
  const digest = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(codeVerifier));

  return {
    codeVerifier,
    codeChallenge: base64UrlEncode(new Uint8Array(digest)),
  };
}

function createRandomValue(length: number): string {
  return base64UrlEncode(crypto.getRandomValues(new Uint8Array(length)));
}

function base64UrlEncode(bytes: Uint8Array): string {
  let binary = '';
  for (const byte of bytes) {
    binary += String.fromCharCode(byte);
  }

  return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/u, '');
}
