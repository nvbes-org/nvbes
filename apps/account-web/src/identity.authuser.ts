export function readAuthuser(search = currentSearch(), pathname = currentPathname()): string {
  const fromAccountPath = pathname.match(/^\/account\/([^/]+)(?:\/|$)/u)?.[1];
  if (isAuthuser(fromAccountPath)) {
    return fromAccountPath;
  }

  const fromSearch = new URLSearchParams(search).get('authuser');
  if (isAuthuser(fromSearch)) {
    return fromSearch;
  }

  const fromPath = pathname.match(/^\/u\/([^/]+)(?:\/|$)/u)?.[1];
  return isAuthuser(fromPath) ? fromPath : '0';
}

export function authuserSearch(authuser: string): { authuser?: string } {
  return authuser === '0' ? {} : { authuser };
}

export function accountHrefForAuthuser(
  authuser: string,
  pathname = currentPathname(),
  search = currentSearch(),
): string {
  const params = new URLSearchParams(search);
  params.delete('authuser');
  params.delete('from');

  const query = params.toString();
  const path = accountPathForAuthuser(authuser, pathname);
  return query ? `${path}?${query}` : path;
}

export function accountPathForAuthuser(authuser: string, pathname = '/account'): string {
  const suffix = accountPathSuffix(pathname);
  return `/account/${encodeURIComponent(authuser)}${suffix}`;
}

function accountPathSuffix(pathname: string): string {
  const indexedPath = pathname.match(/^\/account\/\d{1,3}(\/.*)?$/u);
  if (indexedPath) {
    return indexedPath[1] ?? '';
  }

  const legacyPrefixedPath = pathname.match(/^\/u\/\d{1,3}\/account(\/.*)?$/u)?.[1];
  if (legacyPrefixedPath) {
    return legacyPrefixedPath;
  }

  return pathname.match(/^\/account(\/.*)?$/u)?.[1] ?? '';
}

function isAuthuser(value: string | null | undefined): value is string {
  return typeof value === 'string' && /^\d{1,3}$/u.test(value);
}

function currentSearch(): string {
  return typeof window === 'undefined' ? '' : window.location.search;
}

function currentPathname(): string {
  return typeof window === 'undefined' ? '/' : window.location.pathname;
}
