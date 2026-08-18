let capturedPasswordResetToken = '';

export function captureAndStripPasswordResetToken(): void {
  if (typeof window === 'undefined' || window.location.pathname !== '/reset-password') return;
  const url = new URL(window.location.href);
  capturedPasswordResetToken = url.searchParams.get('token') ?? '';
  if (!url.searchParams.has('token')) return;
  url.searchParams.delete('token');
  window.history.replaceState(window.history.state, '', `${url.pathname}${url.search}${url.hash}`);
}

export function readPasswordResetToken(): string {
  return capturedPasswordResetToken;
}

export function clearPasswordResetToken(): void {
  capturedPasswordResetToken = '';
}
