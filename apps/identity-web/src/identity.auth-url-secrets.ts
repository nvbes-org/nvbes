const SENSITIVE_AUTH_ROUTES = [
  '/reset-password',
  '/verify',
  '/verify-email',
  '/verify-result',
] as const;

type SensitiveAuthRoute = (typeof SENSITIVE_AUTH_ROUTES)[number];

const capturedTokens = new Map<SensitiveAuthRoute, string>();

export function captureAndStripSensitiveAuthUrlToken(): void {
  if (typeof window === 'undefined') {
    return;
  }

  const url = new URL(window.location.href);
  const route = sensitiveAuthRoute(url.pathname);
  if (!route || !url.searchParams.has('token')) {
    return;
  }

  const token = url.searchParams.get('token');
  if (token) {
    capturedTokens.set(route, token);
  }

  url.searchParams.delete('token');
  window.history.replaceState(window.history.state, '', `${url.pathname}${url.search}${url.hash}`);
}

export function readCapturedAuthUrlToken(route: SensitiveAuthRoute): string {
  return capturedTokens.get(route) ?? '';
}

export function clearCapturedAuthUrlToken(route: SensitiveAuthRoute): void {
  capturedTokens.delete(route);
}

function sensitiveAuthRoute(pathname: string): SensitiveAuthRoute | null {
  return SENSITIVE_AUTH_ROUTES.find((route) => route === pathname) ?? null;
}
