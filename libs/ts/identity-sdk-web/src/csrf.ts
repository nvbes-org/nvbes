export function readScopedCsrfToken(authuser = readCurrentAuthuser()): string | undefined {
  if (typeof document === 'undefined') {
    return undefined;
  }

  const cookies = parseDocumentCookies(document.cookie);
  const names =
    authuser && authuser !== '0'
      ? [`csrf_token_${authuser}`, `__Host-csrf_token_${authuser}`]
      : ['csrf_token', '__Host-csrf_token'];

  for (const name of names) {
    const value = cookies.get(name);
    if (value) {
      return value;
    }
  }

  return undefined;
}

export function readCurrentAuthuser(): string | undefined {
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

function parseDocumentCookies(cookieHeader: string): Map<string, string> {
  const cookies = new Map<string, string>();
  for (const cookie of cookieHeader.split(';')) {
    const [name, ...valueParts] = cookie.trim().split('=');
    if (!name || valueParts.length === 0) {
      continue;
    }
    cookies.set(name, valueParts.join('='));
  }
  return cookies;
}
