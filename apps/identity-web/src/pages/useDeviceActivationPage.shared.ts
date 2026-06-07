export function csrfTokenFromCookie() {
  const match =
    typeof document !== 'undefined' ? document.cookie.match(/(?:^|;\s*)csrf_token=([^;]*)/) : null;
  return match?.[1];
}

export function getDeviceActionEndpoint(action: 'approve' | 'deny') {
  return action === 'approve' ? '/oauth/device/approve' : '/oauth/device/deny';
}
