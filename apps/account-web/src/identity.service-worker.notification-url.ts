export const defaultNotificationPath = '/account/0/notifications';

export function notificationUrl(value: unknown, origin: string): string {
  const expectedOrigin = new URL(origin).origin;
  const fallbackUrl = new URL(defaultNotificationPath, expectedOrigin).href;
  if (typeof value !== 'string' || value.trim().length === 0) {
    return fallbackUrl;
  }

  try {
    const candidate = new URL(value, expectedOrigin);
    const isHttp = candidate.protocol === 'https:' || candidate.protocol === 'http:';
    return isHttp && candidate.origin === expectedOrigin ? candidate.href : fallbackUrl;
  } catch {
    return fallbackUrl;
  }
}
