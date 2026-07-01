export function readAuthuser(search = window.location.search): string {
  return new URLSearchParams(search).get('authuser') ?? '0';
}

export function authuserSearch(authuser: string): { authuser: string } | undefined {
  return authuser === '0' ? undefined : { authuser };
}
