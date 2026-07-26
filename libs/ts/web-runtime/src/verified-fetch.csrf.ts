export function readVerifiedFetchCsrfToken(input: { csrfCookieName?: string }): string | undefined {
  if (input.csrfCookieName) {
    return readCookie(input.csrfCookieName);
  }

  const authuser = readCurrentAuthuser();
  const names =
    authuser && authuser !== '0'
      ? [
          `csrf_token_${authuser}`,
          `__Host-csrf_token_${authuser}`,
          'csrf_token',
          '__Host-csrf_token',
        ]
      : ['csrf_token', '__Host-csrf_token'];

  for (const name of names) {
    const token = readCookie(name);
    if (token) {
      return token;
    }
  }

  return undefined;
}

function readCurrentAuthuser(): string | undefined {
  if (typeof window === 'undefined' || !window.location) {
    return undefined;
  }

  const fromSearch = new URLSearchParams(window.location.search).get('authuser');
  if (isAuthuser(fromSearch)) {
    return fromSearch;
  }

  const fromPath = window.location.pathname.match(/^\/u\/([^/]+)(?:\/|$)/u)?.[1];
  return isAuthuser(fromPath) ? fromPath : undefined;
}

function isAuthuser(value: string | null | undefined): value is string {
  return typeof value === 'string' && /^\d{1,3}$/u.test(value);
}

function readCookie(name: string): string | undefined {
  if (typeof document === 'undefined') {
    return undefined;
  }
  const escapedName = name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const match = document.cookie.match(new RegExp(`(?:^|;\\s*)${escapedName}=([^;]*)`));
  return match?.[1] ? decodeURIComponent(match[1]) : undefined;
}
